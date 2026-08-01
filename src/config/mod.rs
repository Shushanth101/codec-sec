use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub port: u16,
    pub log_level: String,
    pub default_memory_limit_kb: u64,
    pub default_time_limit_ms: u64,
    pub max_source_size: usize,
    pub max_concurrent_sandboxes: u32,
    pub worker_threads: usize,
}

impl Config {
    pub fn from_env() -> Self {
        let port = env::var("PORT")
            .ok()
            .and_then(|val| val.parse().ok())
            .unwrap_or(54054);

        let log_level = env::var("LOG_LEVEL")
            .unwrap_or_else(|_| "info".to_string());

        let default_memory_limit_kb = env::var("DEFAULT_MEMORY_LIMIT_KB")
            .ok()
            .and_then(|val| val.parse().ok())
            .unwrap_or(262144); // 256MB

        let default_time_limit_ms = env::var("DEFAULT_TIME_LIMIT_MS")
            .ok()
            .and_then(|val| val.parse().ok())
            .unwrap_or(2000); // 2 seconds

        let max_source_size = env::var("MAX_SOURCE_SIZE")
            .ok()
            .and_then(|val| val.parse().ok())
            .unwrap_or(1048576); // 1MB

        let max_concurrent_sandboxes = env::var("MAX_CONCURRENT_SANDBOXES")
            .ok()
            .and_then(|val| val.parse().ok())
            .unwrap_or(20);

        let worker_threads = env::var("WORKER_THREADS")
            .ok()
            .and_then(|val| val.parse().ok())
            .unwrap_or(4);

        Self {
            port,
            log_level,
            default_memory_limit_kb,
            default_time_limit_ms,
            max_source_size,
            max_concurrent_sandboxes,
            worker_threads,
        }
    }
}
