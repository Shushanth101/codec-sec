use std::collections::HashMap;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Serialize;
use uuid::Uuid;
use crate::{
    errors::AppError,
    models::{
        CreateSubmissionRequest, CreateSubmissionResponse, Submission, SubmissionResponse,
        SubmissionStatus,
    },
    AppState,
};

#[derive(Debug, Serialize)]
pub struct SynchronousExecuteResponse {
    pub stdout: String,
    pub stderr: String,
    pub compile_output: String,
    pub exit_code: i32,
    pub status: String,
    pub time_ms: u64,
    pub memory_kb: u64,
}

pub async fn create_submission(
    State(state): State<AppState>,
    Json(payload): Json<CreateSubmissionRequest>,
) -> Result<impl IntoResponse, AppError> {
    // 1. Validate request
    if payload.source.len() > state.config.max_source_size {
        return Err(AppError::InvalidRequest(format!(
            "Source code size exceeds maximum limit of {} bytes",
            state.config.max_source_size
        )));
    }

    if state.registry.get(&payload.language).is_none() {
        return Err(AppError::InvalidRequest(format!(
            "Unsupported language: {}",
            payload.language
        )));
    }

    let time_limit = payload.time_limit_ms.unwrap_or(state.config.default_time_limit_ms);
    let memory_limit = payload.memory_limit_kb.unwrap_or(state.config.default_memory_limit_kb);

    // 2. Generate unique submission ID
    let submission_id = Uuid::new_v4().to_string();

    let submission = Submission {
        id: submission_id.clone(),
        language: payload.language,
        source: payload.source,
        stdin: payload.stdin,
        time_limit_ms: time_limit,
        memory_limit_kb: memory_limit,
        status: SubmissionStatus::Queued,
        stdout: None,
        stderr: None,
        compile_output: None,
        exit_code: None,
        time_ms: None,
        memory_kb: None,
        created_at: chrono::Utc::now().to_rfc3339(),
    };

    // 3. Save to store
    state.store.set(&submission_id, submission).await?;

    // 4. Enqueue job
    state.queue.enqueue(submission_id.clone()).await?;

    Ok((
        StatusCode::ACCEPTED,
        Json(CreateSubmissionResponse {
            id: submission_id,
            status: SubmissionStatus::Queued.as_str().to_string(),
        }),
    ))
}

pub async fn get_submission(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let sub = state.store.get(&id).await?;
    match sub {
        Some(submission) => {
            let res = SubmissionResponse {
                id: submission.id,
                language: submission.language,
                status: submission.status.as_str().to_string(),
                stdout: submission.stdout,
                stderr: submission.stderr,
                compile_output: submission.compile_output,
                exit_code: submission.exit_code,
                time_ms: submission.time_ms,
                memory_kb: submission.memory_kb,
            };
            Ok(Json(res))
        }
        None => Err(AppError::SubmissionNotFound(id)),
    }
}

pub async fn execute(
    Query(params): Query<HashMap<String, String>>,
    State(state): State<AppState>,
    Json(payload): Json<CreateSubmissionRequest>,
) -> Result<impl IntoResponse, AppError> {
    let wait = params.get("wait").map(|v| v == "true").unwrap_or(false);

    if !wait {
        // Behaves exactly like POST /submissions if wait is false/missing
        return create_submission(State(state), Json(payload)).await;
    }

    // Synchronous execution using wait=true
    if payload.source.len() > state.config.max_source_size {
        return Err(AppError::InvalidRequest(format!(
            "Source code size exceeds maximum limit of {} bytes",
            state.config.max_source_size
        )));
    }

    if state.registry.get(&payload.language).is_none() {
        return Err(AppError::InvalidRequest(format!(
            "Unsupported language: {}",
            payload.language
        )));
    }

    let time_limit = payload.time_limit_ms.unwrap_or(state.config.default_time_limit_ms);
    let memory_limit = payload.memory_limit_kb.unwrap_or(state.config.default_memory_limit_kb);

    let submission_id = Uuid::new_v4().to_string();

    let submission = Submission {
        id: submission_id.clone(),
        language: payload.language,
        source: payload.source,
        stdin: payload.stdin,
        time_limit_ms: time_limit,
        memory_limit_kb: memory_limit,
        status: SubmissionStatus::Queued,
        stdout: None,
        stderr: None,
        compile_output: None,
        exit_code: None,
        time_ms: None,
        memory_kb: None,
        created_at: chrono::Utc::now().to_rfc3339(),
    };

    // Setup oneshot channel for waiting
    let (tx, rx) = tokio::sync::oneshot::channel();
    {
        let mut waiters = state.workers_ctx.waiters.lock().await;
        waiters.insert(submission_id.clone(), tx);
    }

    // Save and Enqueue
    state.store.set(&submission_id, submission).await?;
    state.queue.enqueue(submission_id.clone()).await?;

    // Wait for the result with timeout (time_limit_ms + compile_overhead (10s) + grace_period (5s))
    let total_timeout = time_limit + 15000;
    let wait_result = tokio::time::timeout(
        tokio::time::Duration::from_millis(total_timeout),
        rx,
    ).await;

    match wait_result {
        Ok(Ok(final_sub)) => {
            let res = SynchronousExecuteResponse {
                stdout: final_sub.stdout.unwrap_or_default(),
                stderr: final_sub.stderr.unwrap_or_default(),
                compile_output: final_sub.compile_output.unwrap_or_default(),
                exit_code: final_sub.exit_code.unwrap_or(0),
                status: final_sub.status.as_str().to_string(),
                time_ms: final_sub.time_ms.unwrap_or(0),
                memory_kb: final_sub.memory_kb.unwrap_or(0),
            };
            Ok((StatusCode::OK, Json(res)))
        }
        Ok(Err(_)) => {
            // Oneshot sender dropped without sending (worker panic or pool shutdown)
            Err(AppError::Internal("Worker task dropped before sending result".to_string()))
        }
        Err(_) => {
            // Timeout expired. Clean up the waiter map
            let mut waiters = state.workers_ctx.waiters.lock().await;
            waiters.remove(&submission_id);
            
            Err(AppError::Internal("Execution timed out waiting for queue processing".to_string()))
        }
    }
}
