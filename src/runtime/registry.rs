use std::collections::HashMap;
use std::sync::Arc;
use crate::runtime::runtime::Runtime;
use crate::runtime::c::CRuntime;
use crate::runtime::cpp::CppRuntime;
use crate::runtime::java::JavaRuntime;
use crate::runtime::python::PythonRuntime;
use crate::runtime::node::NodeRuntime;
use crate::runtime::rust::RustRuntime;
use crate::runtime::ruby::RubyRuntime;

pub struct RuntimeRegistry {
    runtimes: HashMap<String, Arc<dyn Runtime>>,
}

impl RuntimeRegistry {
    pub fn new() -> Self {
        let mut runtimes: HashMap<String, Arc<dyn Runtime>> = HashMap::new();
        runtimes.insert("c".to_string(), Arc::new(CRuntime::new()));
        runtimes.insert("cpp".to_string(), Arc::new(CppRuntime::new()));
        runtimes.insert("java".to_string(), Arc::new(JavaRuntime::new()));
        runtimes.insert("python".to_string(), Arc::new(PythonRuntime::new()));
        runtimes.insert("node".to_string(), Arc::new(NodeRuntime::new()));
        runtimes.insert("rust".to_string(), Arc::new(RustRuntime::new()));
        runtimes.insert("ruby".to_string(), Arc::new(RubyRuntime::new()));

        Self { runtimes }
    }

    pub fn get(&self, id: &str) -> Option<Arc<dyn Runtime>> {
        self.runtimes.get(id).cloned()
    }

    pub fn list(&self) -> Vec<Arc<dyn Runtime>> {
        let mut list: Vec<Arc<dyn Runtime>> = self.runtimes.values().cloned().collect();
        // sort by ID for deterministic sorting
        list.sort_by(|a, b| a.id().cmp(b.id()));
        list
    }
}
