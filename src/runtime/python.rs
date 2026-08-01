use crate::runtime::runtime::Runtime;

pub struct PythonRuntime;

impl PythonRuntime {
    pub fn new() -> Self {
        Self
    }
}

impl Runtime for PythonRuntime {
    fn id(&self) -> &str {
        "python"
    }

    fn name(&self) -> &str {
        "Python"
    }

    fn extension(&self) -> &str {
        "py"
    }

    fn compiled(&self) -> bool {
        false
    }

    fn compile_command(&self, _source_file: &str, _output_file: &str) -> Option<Vec<String>> {
        None
    }

    fn execute_command(&self, source_or_exec_file: &str) -> Vec<String> {
        vec![
            "/usr/bin/python3".to_string(),
            source_or_exec_file.to_string(),
        ]
    }
}
