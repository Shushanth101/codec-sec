use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Submission {0} not found")]
    SubmissionNotFound(String),

    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    #[error("Sandbox error: {0}")]
    SandboxError(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_type, message) = match &self {
            AppError::SubmissionNotFound(id) => (
                StatusCode::NOT_FOUND,
                "SUBMISSION_NOT_FOUND",
                format!("Submission {} not found", id),
            ),
            AppError::InvalidRequest(msg) => (
                StatusCode::BAD_REQUEST,
                "INVALID_REQUEST",
                msg.clone(),
            ),
            AppError::SandboxError(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "SANDBOX_ERROR",
                msg.clone(),
            ),
            AppError::Internal(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "INTERNAL_ERROR",
                msg.clone(),
            ),
        };

        let body = Json(json!({
            "error": error_type,
            "message": message,
        }));

        (status, body).into_response()
    }
}

// Implement conversion from anyhow::Error to AppError for easy error propagation with `?`
impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        AppError::Internal(err.to_string())
    }
}

impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        AppError::Internal(format!("I/O Error: {}", err))
    }
}
