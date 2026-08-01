use std::collections::HashMap;
use tokio::sync::RwLock;
use async_trait::async_trait;
use crate::errors::AppError;
use crate::models::{Submission, SubmissionStatus};

#[async_trait]
pub trait SubmissionStore: Send + Sync {
    async fn get(&self, id: &str) -> Result<Option<Submission>, AppError>;
    async fn set(&self, id: &str, submission: Submission) -> Result<(), AppError>;
    async fn update_status(&self, id: &str, status: SubmissionStatus) -> Result<(), AppError>;
    async fn update_result(
        &self,
        id: &str,
        status: SubmissionStatus,
        stdout: Option<String>,
        stderr: Option<String>,
        compile_output: Option<String>,
        exit_code: Option<i32>,
        time_ms: Option<u64>,
        memory_kb: Option<u64>,
    ) -> Result<(), AppError>;
}

pub struct InMemoryStore {
    submissions: RwLock<HashMap<String, Submission>>,
}

impl InMemoryStore {
    pub fn new() -> Self {
        Self {
            submissions: RwLock::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl SubmissionStore for InMemoryStore {
    async fn get(&self, id: &str) -> Result<Option<Submission>, AppError> {
        let read_guard = self.submissions.read().await;
        Ok(read_guard.get(id).cloned())
    }

    async fn set(&self, id: &str, submission: Submission) -> Result<(), AppError> {
        let mut write_guard = self.submissions.write().await;
        write_guard.insert(id.to_string(), submission);
        Ok(())
    }

    async fn update_status(&self, id: &str, status: SubmissionStatus) -> Result<(), AppError> {
        let mut write_guard = self.submissions.write().await;
        if let Some(sub) = write_guard.get_mut(id) {
            sub.status = status;
            Ok(())
        } else {
            Err(AppError::SubmissionNotFound(id.to_string()))
        }
    }

    async fn update_result(
        &self,
        id: &str,
        status: SubmissionStatus,
        stdout: Option<String>,
        stderr: Option<String>,
        compile_output: Option<String>,
        exit_code: Option<i32>,
        time_ms: Option<u64>,
        memory_kb: Option<u64>,
    ) -> Result<(), AppError> {
        let mut write_guard = self.submissions.write().await;
        if let Some(sub) = write_guard.get_mut(id) {
            sub.status = status;
            sub.stdout = stdout;
            sub.stderr = stderr;
            sub.compile_output = compile_output;
            sub.exit_code = exit_code;
            sub.time_ms = time_ms;
            sub.memory_kb = memory_kb;
            Ok(())
        } else {
            Err(AppError::SubmissionNotFound(id.to_string()))
        }
    }
}
