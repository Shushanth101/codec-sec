use std::path::Path;
use tokio::fs;
use crate::errors::AppError;
use crate::sandbox::manager::Sandbox;
use crate::sandbox::isolate::{self, IsolateRunArgs};

pub struct CompileResult {
    pub success: bool,
    pub compile_output: String,
}

pub struct Compiler;

impl Compiler {
    pub async fn compile(
        sandbox: &Sandbox,
        source_code: &str,
        source_file_name: &str,
        compile_cmd: &[String],
        extra_dirs: &[String],
    ) -> Result<CompileResult, AppError> {
        // 1. Write source file to sandbox box directory
        let source_path = sandbox.box_path.join(source_file_name);
        fs::write(&source_path, source_code)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to write source file to sandbox: {}", e)))?;

        // 2. Clear previous compile outputs if any
        let compile_out_path = sandbox.box_path.join("compile_out.txt");
        let compile_err_path = sandbox.box_path.join("compile_err.txt");
        let _ = fs::remove_file(&compile_out_path).await;
        let _ = fs::remove_file(&compile_err_path).await;

        // 3. Run compilation inside sandbox
        let status = isolate::run(
            sandbox.box_id,
            IsolateRunArgs {
                cmd: compile_cmd,
                time_limit_ms: Some(10000), // 10 seconds limit for compiling
                memory_limit_kb: Some(1048576), // 1GB limit for compiling
                stdin_file: None,
                stdout_file: Some("compile_out.txt"),
                stderr_file: Some("compile_err.txt"),
                meta_file: None,
                use_cg: true, // Compiling uses cgroups for stable memory constraints
                extra_dirs,
            },
        ).await?;

        // 4. Read compile output from the stdout/stderr files
        let compile_out = if compile_out_path.exists() {
            fs::read_to_string(&compile_out_path).await.unwrap_or_default()
        } else {
            String::new()
        };

        let compile_err = if compile_err_path.exists() {
            fs::read_to_string(&compile_err_path).await.unwrap_or_default()
        } else {
            String::new()
        };

        // Combine stdout and stderr for compilation output
        let mut compile_output = String::new();
        if !compile_out.trim().is_empty() {
            compile_output.push_str(&compile_out);
        }
        if !compile_err.trim().is_empty() {
            if !compile_output.is_empty() {
                compile_output.push('\n');
            }
            compile_output.push_str(&compile_err);
        }

        let success = status.success();

        Ok(CompileResult {
            success,
            compile_output,
        })
    }
}
