mod api;
mod compiler;
mod config;
mod errors;
mod executor;
mod models;
mod queue;
mod runtime;
mod sandbox;
mod store;
mod workers;

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use axum::{
    routing::{get, post},
    Router,
};
use tokio::sync::Mutex;
use crate::config::Config;
use crate::runtime::registry::RuntimeRegistry;
use crate::sandbox::manager::SandboxManager;
use crate::queue::{InMemoryQueue, JobQueue};
use crate::store::{InMemoryStore, SubmissionStore};
use crate::workers::{WorkerContext, start_workers};

#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub registry: Arc<RuntimeRegistry>,
    pub queue: Arc<dyn JobQueue>,
    pub store: Arc<dyn SubmissionStore>,
    pub workers_ctx: Arc<WorkerContext>,
}

async fn compile_malloc_wrapper() {
    let code = r#"
void* __real_malloc(unsigned long size);
void* __real_calloc(unsigned long num, unsigned long size);
void* __real_realloc(void* ptr, unsigned long size);

void* __wrap_malloc(unsigned long size) {
    void* p = __real_malloc(size);
    if (size > 0 && p == 0) {
        __builtin_trap();
    }
    return p;
}

void* __wrap_calloc(unsigned long num, unsigned long size) {
    void* p = __real_calloc(num, size);
    if (num > 0 && size > 0 && p == 0) {
        __builtin_trap();
    }
    return p;
}

void* __wrap_realloc(void* ptr, unsigned long size) {
    void* p = __real_realloc(ptr, size);
    if (size > 0 && p == 0) {
        __builtin_trap();
    }
    return p;
}
"#;
    let _ = tokio::fs::write("/app/malloc_wrapper.c", code).await;
    let _ = tokio::process::Command::new("gcc")
        .args(&["-O3", "-c", "/app/malloc_wrapper.c", "-o", "/app/malloc_wrapper.o"])
        .status()
        .await;
}

#[tokio::main]
async fn main() {
    // Compile malloc wrapper on startup
    compile_malloc_wrapper().await;

    // 1. Load config
    let config = Config::from_env();

    // 2. Initialize tracing subscriber
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(&config.log_level)),
        )
        .init();

    tracing::info!("Starting CodecSec server...");
    tracing::info!("Config loaded: {:?}", config);

    // 3. Initialize Shared Components
    let registry = Arc::new(RuntimeRegistry::new());
    let sandbox_manager = SandboxManager::new(config.max_concurrent_sandboxes);
    let queue = Arc::new(InMemoryQueue::new(1000));
    let store = Arc::new(InMemoryStore::new());
    let waiters = Arc::new(Mutex::new(HashMap::new()));

    let workers_ctx = Arc::new(WorkerContext {
        registry: registry.clone(),
        sandbox_manager,
        queue: queue.clone() as Arc<dyn JobQueue>,
        store: store.clone() as Arc<dyn SubmissionStore>,
        waiters,
    });

    // 4. Start Background Workers
    start_workers(workers_ctx.clone(), config.worker_threads);
    tracing::info!("Started {} background worker threads", config.worker_threads);

    // 5. Build router and Axum app state
    let state = AppState {
        config: config.clone(),
        registry,
        queue: queue as Arc<dyn JobQueue>,
        store: store as Arc<dyn SubmissionStore>,
        workers_ctx,
    };

    let app = Router::new()
        .route("/runtimes", get(api::runtimes::get_runtimes))
        .route("/submissions", post(api::submissions::create_submission))
        .route("/submissions/:id", get(api::submissions::get_submission))
        .route("/execute", post(api::submissions::execute))
        .layer(axum::extract::DefaultBodyLimit::max(15 * 1024 * 1024))
        .with_state(state);

    // 6. Start server
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    tracing::info!("Listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
