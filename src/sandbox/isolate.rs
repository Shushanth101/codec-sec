use std::path::PathBuf;
use tokio::process::Command;
use crate::errors::AppError;

pub struct IsolateRunArgs<'a> {
    pub cmd: &'a [String],
    pub time_limit_ms: Option<u64>,
    pub memory_limit_kb: Option<u64>,
    pub stdin_file: Option<&'a str>,
    pub stdout_file: Option<&'a str>,
    pub stderr_file: Option<&'a str>,
    pub meta_file: Option<&'a str>,
    pub use_cg: bool,
}

pub async fn init(box_id: u32) -> Result<PathBuf, AppError> {
    let output = Command::new("isolate")
        .arg(format!("--box-id={}", box_id))
        .arg("--init")
        .output()
        .await
        .map_err(|e| AppError::SandboxError(format!("Failed to execute isolate --init: {}", e)))?;

    if !output.status.success() {
        return Err(AppError::SandboxError(format!(
            "isolate --init failed (code {:?}): {}",
            output.status.code(),
            String::from_utf8_lossy(&output.stderr)
        )));
    }

    let stdout_str = String::from_utf8_lossy(&output.stdout);
    let path_str = stdout_str.trim();
    if path_str.is_empty() {
        return Err(AppError::SandboxError("isolate --init returned empty path".to_string()));
    }

    Ok(PathBuf::from(path_str))
}

pub async fn cleanup(box_id: u32) -> Result<(), AppError> {
    let output = Command::new("isolate")
        .arg(format!("--box-id={}", box_id))
        .arg("--cleanup")
        .output()
        .await
        .map_err(|e| AppError::SandboxError(format!("Failed to execute isolate --cleanup: {}", e)))?;

    if !output.status.success() {
        return Err(AppError::SandboxError(format!(
            "isolate --cleanup failed (code {:?}): {}",
            output.status.code(),
            String::from_utf8_lossy(&output.stderr)
        )));
    }

    Ok(())
}

pub async fn run(box_id: u32, args: IsolateRunArgs<'_>) -> Result<std::process::ExitStatus, AppError> {
    let mut cmd = Command::new("isolate");
    cmd.arg(format!("--box-id={}", box_id));

    if args.use_cg {
        cmd.arg("--cg");
        if let Some(mem_limit) = args.memory_limit_kb {
            cmd.arg(format!("--cg-mem={}", mem_limit));
        }
    } else {
        if let Some(mem_limit) = args.memory_limit_kb {
            cmd.arg(format!("--mem={}", mem_limit));
        }
    }

    if let Some(time_limit) = args.time_limit_ms {
        let limit_secs = time_limit as f64 / 1000.0;
        cmd.arg(format!("--time={}", limit_secs));
        // Add wall-time slightly larger than CPU time
        cmd.arg(format!("--wall-time={}", limit_secs * 2.0 + 1.0));
    }

    if let Some(stdin) = args.stdin_file {
        cmd.arg(format!("--stdin={}", stdin));
    }

    if let Some(stdout) = args.stdout_file {
        cmd.arg(format!("--stdout={}", stdout));
    }

    if let Some(stderr) = args.stderr_file {
        cmd.arg(format!("--stderr={}", stderr));
    }

    if let Some(meta) = args.meta_file {
        cmd.arg(format!("--meta={}", meta));
    }

    cmd.arg("--run");
    cmd.arg("--");
    for arg in args.cmd {
        cmd.arg(arg);
    }

    let mut child = cmd.spawn().map_err(|e| {
        AppError::SandboxError(format!("Failed to spawn isolate process: {}", e))
    })?;

    let status = child.wait().await.map_err(|e| {
        AppError::SandboxError(format!("Failed to wait for isolate process: {}", e))
    })?;

    Ok(status)
}
