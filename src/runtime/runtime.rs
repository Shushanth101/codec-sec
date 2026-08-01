pub trait Runtime: Send + Sync {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn extension(&self) -> &str;
    fn compiled(&self) -> bool;

    fn compile_command(&self, source_file: &str, output_file: &str) -> Option<Vec<String>>;
    fn execute_command(&self, source_or_exec_file: &str) -> Vec<String>;
}
