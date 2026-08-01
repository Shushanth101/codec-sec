use crate::runtime::runtime::Runtime;

pub struct CRuntime;

impl CRuntime {
    pub fn new() -> Self {
        Self
    }
}

impl Runtime for CRuntime {
    fn id(&self) -> &str {
        "c"
    }

    fn name(&self) -> &str {
        "C"
    }

    fn extension(&self) -> &str {
        "c"
    }

    fn compiled(&self) -> bool {
        true
    }

    fn compile_command(&self, source_file: &str, output_file: &str) -> Option<Vec<String>> {
        Some(vec![
            "/usr/bin/gcc".to_string(),
            "-O3".to_string(),
            source_file.to_string(),
            "-o".to_string(),
            output_file.to_string(),
            "-lm".to_string(),
        ])
    }

    fn execute_command(&self, source_or_exec_file: &str) -> Vec<String> {
        vec![source_or_exec_file.to_string()]
    }
}
