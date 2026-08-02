use std::path::Path;
use tokio::fs;
use crate::errors::AppError;
use crate::models::SubmissionStatus;
use crate::sandbox::manager::Sandbox;
use crate::sandbox::isolate::{self, IsolateRunArgs};

pub struct ExecuteResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub status: SubmissionStatus,
    pub time_ms: u64,
    pub memory_kb: u64,
}

pub struct Executor;

impl Executor {
    pub async fn execute(
        sandbox: &Sandbox,
        _source_or_exec_name: &str,
        stdin: &str,
        time_limit_ms: u64,
        memory_limit_kb: u64,
        stack_limit_kb: u64,
        exec_cmd: &[String],
        extra_dirs: &[String],
        language: &str,
    ) -> Result<ExecuteResult, AppError> {
        // 1. Write stdin to stdin.txt inside sandbox box
        let stdin_path = sandbox.box_path.join("stdin.txt");
        fs::write(&stdin_path, stdin)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to write stdin to sandbox: {}", e)))?;

        // 2. Clear stdout, stderr, and meta files if any exist
        let stdout_path = sandbox.box_path.join("stdout.txt");
        let stderr_path = sandbox.box_path.join("stderr.txt");
        let meta_path = sandbox.box_path.join("meta.txt");

        let _ = fs::remove_file(&stdout_path).await;
        let _ = fs::remove_file(&stderr_path).await;
        let _ = fs::remove_file(&meta_path).await;

        // 3. Run execution inside sandbox
        let meta_file_str = meta_path.to_string_lossy().to_string();

        let limit_as = if language == "c" || language == "cpp" || language == "rust" {
            Some(memory_limit_kb)
        } else {
            None
        };

        let _status = isolate::run(
            sandbox.box_id,
            IsolateRunArgs {
                cmd: exec_cmd,
                time_limit_ms: Some(time_limit_ms),
                memory_limit_kb: Some(memory_limit_kb),
                memory_limit_as_kb: limit_as,
                stack_limit_kb: Some(stack_limit_kb),
                fsize_limit_kb: Some(1024), // Strict 1MB output/file write limit
                stdin_file: Some("stdin.txt"),
                stdout_file: Some("stdout.txt"),
                stderr_file: Some("stderr.txt"),
                meta_file: Some(&meta_file_str),
                use_cg: true, // Enable cgroups for execution timing and memory limits
                extra_dirs,
            },
        ).await?;

        // 4. Read stdout and stderr with limit (100KB max to avoid memory bloat)
        let stdout = Self::read_limited_file(&stdout_path, 102400).await;
        let stderr = Self::read_limited_file(&stderr_path, 102400).await;

        // 5. Parse meta file
        let mut time_ms = 0;
        let mut memory_kb = 0;
        let mut exit_code = 0;
        let mut status = SubmissionStatus::Accepted;

        if meta_path.exists() {
            let meta_content = fs::read_to_string(&meta_path).await.unwrap_or_default();
            let mut isolate_status: Option<String> = None;
            let mut exitsig: Option<i32> = None;
            let mut max_rss: u64 = 0;
            let mut cg_mem: Option<u64> = None;

            for line in meta_content.lines() {
                if let Some((key, val)) = line.split_once(':') {
                    match key {
                        "time" => {
                            if let Ok(secs) = val.parse::<f64>() {
                                time_ms = (secs * 1000.0) as u64;
                            }
                        }
                        "max-rss" => {
                            if let Ok(kb) = val.parse::<u64>() {
                                max_rss = kb;
                            }
                        }
                        "cg-mem" => {
                            if let Ok(kb) = val.parse::<u64>() {
                                cg_mem = Some(kb);
                            }
                        }
                        "exitcode" => {
                            if let Ok(code) = val.parse::<i32>() {
                                exit_code = code;
                            }
                        }
                        "exitsig" => {
                            if let Ok(sig) = val.parse::<i32>() {
                                exitsig = Some(sig);
                            }
                        }
                        "status" => {
                            isolate_status = Some(val.to_string());
                        }
                        _ => {}
                    }
                }
            }

            // Prefer cgroup memory over max-rss if available
            memory_kb = cg_mem.unwrap_or(max_rss);

            // Determine final status based on isolate's reports
            if let Some(status_str) = isolate_status {
                match status_str.as_str() {
                    "TO" => {
                        status = SubmissionStatus::TimeLimitExceeded;
                    }
                    "SG" => {
                        // Check if killed by OOM (typically SIGKILL (9) or SIGSEGV (11) and near memory limit)
                        let is_oom = (exitsig == Some(9) || exitsig == Some(11))
                            && memory_kb >= (memory_limit_kb.saturating_sub(4096)); // close to limit
                        if is_oom {
                            status = SubmissionStatus::MemoryLimitExceeded;
                        } else {
                            status = SubmissionStatus::RuntimeError;
                        }
                    }
                    "RE" => {
                        status = SubmissionStatus::RuntimeError;
                    }
                    _ => {
                        status = SubmissionStatus::InternalError;
                    }
                }
            } else if exit_code != 0 {
                status = SubmissionStatus::RuntimeError;
            }
        } else {
            // Missing meta file means execution failed to start or write metrics
            status = SubmissionStatus::InternalError;
        }

        Ok(ExecuteResult {
            stdout,
            stderr,
            exit_code,
            status,
            time_ms,
            memory_kb,
        })
    }

    async fn read_limited_file(path: &Path, limit: usize) -> String {
        if !path.exists() {
            return String::new();
        }
        match fs::metadata(path).await {
            Ok(meta) => {
                let size = meta.len() as usize;
                if size > limit {
                    match fs::read(path).await {
                        Ok(bytes) => {
                            let truncated = String::from_utf8_lossy(&bytes[..limit]).to_string();
                            format!("{}... [Output Truncated]", truncated)
                        }
                        Err(_) => String::new(),
                    }
                } else {
                    fs::read_to_string(path).await.unwrap_or_default()
                }
            }
            Err(_) => String::new(),
        }
    }
}
