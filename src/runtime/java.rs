use crate::runtime::runtime::Runtime;

pub struct JavaRuntime;

impl JavaRuntime {
    pub fn new() -> Self {
        Self
    }
}

impl Runtime for JavaRuntime {
    fn id(&self) -> &str {
        "java"
    }

    fn name(&self) -> &str {
        "Java"
    }

    fn extension(&self) -> &str {
        "java"
    }

    fn compiled(&self) -> bool {
        true
    }

    fn compile_command(&self, source_file: &str, _output_file: &str) -> Option<Vec<String>> {
        Some(vec![
            "/usr/lib/jvm/java-17-openjdk-amd64/bin/javac".to_string(),
            source_file.to_string(),
        ])
    }

    fn execute_command(&self, _source_or_exec_file: &str) -> Vec<String> {
        // Main class is assumed to be named Main, we write the file as Main.java
        vec![
            "/usr/lib/jvm/java-17-openjdk-amd64/bin/java".to_string(),
            "Main".to_string(),
        ]
    }
}
