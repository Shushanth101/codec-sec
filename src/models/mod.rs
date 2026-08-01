use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SubmissionStatus {
    Queued,
    Compiling,
    Running,
    Accepted,
    CompileError,
    RuntimeError,
    TimeLimitExceeded,
    MemoryLimitExceeded,
    InternalError,
}

impl SubmissionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Queued => "Queued",
            Self::Compiling => "Compiling",
            Self::Running => "Running",
            Self::Accepted => "Accepted",
            Self::CompileError => "Compile Error",
            Self::RuntimeError => "Runtime Error",
            Self::TimeLimitExceeded => "Time Limit Exceeded",
            Self::MemoryLimitExceeded => "Memory Limit Exceeded",
            Self::InternalError => "Internal Error",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Submission {
    pub id: String,
    pub language: String,
    pub source: String,
    pub stdin: Option<String>,
    pub time_limit_ms: u64,
    pub memory_limit_kb: u64,
    pub status: SubmissionStatus,
    pub stdout: Option<String>,
    pub stderr: Option<String>,
    pub compile_output: Option<String>,
    pub exit_code: Option<i32>,
    pub time_ms: Option<u64>,
    pub memory_kb: Option<u64>,
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateSubmissionRequest {
    pub language: String,
    pub source: String,
    pub stdin: Option<String>,
    pub time_limit_ms: Option<i64>,
    pub memory_limit_kb: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CreateSubmissionResponse {
    pub id: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SubmissionResponse {
    pub id: String,
    pub language: String,
    pub status: String,
    pub stdout: Option<String>,
    pub stderr: Option<String>,
    pub compile_output: Option<String>,
    pub exit_code: Option<i32>,
    pub time_ms: Option<u64>,
    pub memory_kb: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RuntimeInfo {
    pub id: String,
    pub name: String,
    pub compiled: bool,
}
