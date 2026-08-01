use crate::runtime::runtime::Runtime;

pub struct RubyRuntime;

impl RubyRuntime {
    pub fn new() -> Self {
        Self
    }
}

impl Runtime for RubyRuntime {
    fn id(&self) -> &str {
        "ruby"
    }

    fn name(&self) -> &str {
        "Ruby"
    }

    fn extension(&self) -> &str {
        "rb"
    }

    fn compiled(&self) -> bool {
        false
    }

    fn compile_command(&self, _source_file: &str, _output_file: &str) -> Option<Vec<String>> {
        None
    }

    fn execute_command(&self, source_or_exec_file: &str) -> Vec<String> {
        vec![
            "/usr/bin/ruby".to_string(),
            source_or_exec_file.to_string(),
        ]
    }
}
