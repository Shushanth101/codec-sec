use crate::runtime::runtime::Runtime;

pub struct NodeRuntime;

impl NodeRuntime {
    pub fn new() -> Self {
        Self
    }
}

impl Runtime for NodeRuntime {
    fn id(&self) -> &str {
        "node"
    }

    fn name(&self) -> &str {
        "Node.js"
    }

    fn extension(&self) -> &str {
        "js"
    }

    fn compiled(&self) -> bool {
        false
    }

    fn compile_command(&self, _source_file: &str, _output_file: &str) -> Option<Vec<String>> {
        None
    }

    fn execute_command(&self, source_or_exec_file: &str) -> Vec<String> {
        vec![
            "/usr/bin/node".to_string(),
            source_or_exec_file.to_string(),
        ]
    }
}
