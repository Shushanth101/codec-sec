use std::collections::HashMap;
use std::sync::Arc;
use tokio::fs;
use tokio::sync::{oneshot, Mutex};
use crate::models::{Submission, SubmissionStatus};
use crate::runtime::registry::RuntimeRegistry;
use crate::sandbox::manager::SandboxManager;
use crate::queue::JobQueue;
use crate::store::SubmissionStore;
use crate::compiler::Compiler;
use crate::executor::Executor;

pub struct WorkerContext {
    pub registry: Arc<RuntimeRegistry>,
    pub sandbox_manager: SandboxManager,
    pub queue: Arc<dyn JobQueue>,
    pub store: Arc<dyn SubmissionStore>,
    pub waiters: Arc<Mutex<HashMap<String, oneshot::Sender<Submission>>>>,
}

pub fn start_workers(context: Arc<WorkerContext>, num_workers: usize) {
    for i in 0..num_workers {
        let ctx = context.clone();
        tokio::spawn(async move {
            tracing::info!("Worker {} started", i);
            loop {
                if let Err(e) = run_worker_loop(&ctx).await {
                    tracing::error!("Worker {} encountered error: {:?}", i, e);
                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                }
            }
        });
    }
}

async fn run_worker_loop(ctx: &WorkerContext) -> Result<(), crate::errors::AppError> {
    let sub_id = ctx.queue.dequeue().await?;
    tracing::info!("Dequeued submission {}", sub_id);

    let sub_opt = ctx.store.get(&sub_id).await?;
    let submission = match sub_opt {
        Some(s) => s,
        None => {
            tracing::error!("Submission {} not found in store", sub_id);
            return Ok(());
        }
    };

    let runtime = match ctx.registry.get(&submission.language) {
        Some(r) => r,
        None => {
            let msg = format!("Unsupported language: {}", submission.language);
            ctx.store.update_result(
                &sub_id,
                SubmissionStatus::InternalError,
                None,
                Some(msg),
                None,
                None,
                None,
                None,
            ).await?;
            notify_waiter(ctx, &sub_id).await;
            return Ok(());
        }
    };

    // Transition to Compiling / Running depending on whether it needs compilation
    let initial_status = if runtime.compiled() {
        SubmissionStatus::Compiling
    } else {
        SubmissionStatus::Running
    };

    ctx.store.update_status(&sub_id, initial_status).await?;

    // Acquire sandbox
    let sandbox = match ctx.sandbox_manager.acquire().await {
        Ok(s) => s,
        Err(e) => {
            let err_msg = format!("Failed to acquire sandbox: {}", e);
            tracing::error!("{}", err_msg);
            ctx.store.update_result(
                &sub_id,
                SubmissionStatus::InternalError,
                None,
                Some(err_msg),
                None,
                None,
                None,
                None,
            ).await?;
            notify_waiter(ctx, &sub_id).await;
            return Ok(());
        }
    };

    // Prepare filenames
    let source_file_name = if submission.language == "java" {
        "Main.java".to_string()
    } else {
        format!("source.{}", runtime.extension())
    };

    let exec_file_name = if submission.language == "java" {
        "".to_string()
    } else if runtime.compiled() {
        "program".to_string()
    } else {
        source_file_name.clone()
    };

    let mut extra_dirs = Vec::new();

    if submission.language == "rust" {
        extra_dirs.push("/usr/local/cargo=/usr/local/cargo".to_string());
        extra_dirs.push("/usr/local/rustup=/usr/local/rustup".to_string());
    } else if submission.language == "java" {
        extra_dirs.push("/usr/lib/jvm=/usr/lib/jvm".to_string());
        extra_dirs.push("/etc/java-17-openjdk=/etc/java-17-openjdk".to_string());
    }

    let mut compile_extra_dirs = extra_dirs.clone();
    compile_extra_dirs.push("/etc/alternatives=/etc/alternatives".to_string());

    let mut compile_output: Option<String> = None;

    // Compile if compiled language
    if runtime.compiled() {
        let compile_cmd = match runtime.compile_command(&source_file_name, &exec_file_name) {
            Some(cmd) => cmd,
            None => {
                ctx.store.update_result(
                    &sub_id,
                    SubmissionStatus::InternalError,
                    None,
                    Some("Compile command not generated".to_string()),
                    None,
                    None,
                    None,
                    None,
                ).await?;
                notify_waiter(ctx, &sub_id).await;
                return Ok(());
            }
        };

        let compile_start = std::time::Instant::now();
        let compile_res = Compiler::compile(
            &sandbox,
            &submission.source,
            &source_file_name,
            &compile_cmd,
            &compile_extra_dirs,
        ).await;
        let compile_duration = compile_start.elapsed();

        match compile_res {
            Ok(res) => {
                compile_output = Some(res.compile_output.clone());
                tracing::info!("Submission {} compiled in {:?}", sub_id, compile_duration);
                if !res.success {
                    ctx.store.update_result(
                        &sub_id,
                        SubmissionStatus::CompileError,
                        None,
                        None,
                        compile_output,
                        None,
                        None,
                        None,
                    ).await?;
                    notify_waiter(ctx, &sub_id).await;
                    return Ok(());
                }
            }
            Err(e) => {
                let err_msg = format!("Compilation failed: {}", e);
                ctx.store.update_result(
                    &sub_id,
                    SubmissionStatus::InternalError,
                    None,
                    Some(err_msg),
                    None,
                    None,
                    None,
                    None,
                ).await?;
                notify_waiter(ctx, &sub_id).await;
                return Ok(());
            }
        }
    } else {
        // For interpreted languages, write source file directly to sandbox
        let source_path = sandbox.box_path.join(&source_file_name);
        if let Err(e) = fs::write(&source_path, &submission.source).await {
            let err_msg = format!("Failed to write source file to sandbox: {}", e);
            ctx.store.update_result(
                &sub_id,
                SubmissionStatus::InternalError,
                None,
                Some(err_msg),
                None,
                None,
                None,
                None,
            ).await?;
            notify_waiter(ctx, &sub_id).await;
            return Ok(());
        }
    }

    // Update status to Running
    ctx.store.update_status(&sub_id, SubmissionStatus::Running).await?;

    // Execute
    let exec_cmd = runtime.execute_command(&exec_file_name);
    let exec_start = std::time::Instant::now();
    let exec_res = Executor::execute(
        &sandbox,
        &exec_file_name,
        submission.stdin.as_deref().unwrap_or(""),
        submission.time_limit_ms,
        submission.memory_limit_kb,
        submission.stack_limit_kb,
        &exec_cmd,
        &extra_dirs,
        &submission.language,
    ).await;
    let exec_duration = exec_start.elapsed();

    match exec_res {
        Ok(res) => {
            tracing::info!(
                "Submission {} executed in {:?}, status: {:?}, time: {}ms, memory: {}KB, exit_code: {}",
                sub_id,
                exec_duration,
                res.status,
                res.time_ms,
                res.memory_kb,
                res.exit_code
            );

            ctx.store.update_result(
                &sub_id,
                res.status,
                Some(res.stdout),
                Some(res.stderr),
                compile_output,
                Some(res.exit_code),
                Some(res.time_ms),
                Some(res.memory_kb),
            ).await?;
        }
        Err(e) => {
            let err_msg = format!("Execution failed: {}", e);
            ctx.store.update_result(
                &sub_id,
                SubmissionStatus::InternalError,
                None,
                Some(err_msg),
                compile_output,
                None,
                None,
                None,
            ).await?;
        }
    }

    notify_waiter(ctx, &sub_id).await;
    Ok(())
}

async fn notify_waiter(ctx: &WorkerContext, sub_id: &str) {
    let mut waiters_guard = ctx.waiters.lock().await;
    if let Some(tx) = waiters_guard.remove(sub_id) {
        if let Ok(Some(submission)) = ctx.store.get(sub_id).await {
            let _ = tx.send(submission);
        }
    }
}
