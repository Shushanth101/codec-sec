use crate::runtime::runtime::Runtime;

pub struct RustRuntime;

impl RustRuntime {
    pub fn new() -> Self {
        Self
    }
}

impl Runtime for RustRuntime {
    fn id(&self) -> &str {
        "rust"
    }

    fn name(&self) -> &str {
        "Rust"
    }

    fn extension(&self) -> &str {
        "rs"
    }

    fn compiled(&self) -> bool {
        true
    }

    fn compile_command(&self, source_file: &str, output_file: &str) -> Option<Vec<String>> {
        Some(vec![
            "/usr/bin/rustc".to_string(),
            "-O".to_string(),
            source_file.to_string(),
            "-o".to_string(),
            output_file.to_string(),
        ])
    }

    fn execute_command(&self, source_or_exec_file: &str) -> Vec<String> {
        vec![source_or_exec_file.to_string()]
    }
}
