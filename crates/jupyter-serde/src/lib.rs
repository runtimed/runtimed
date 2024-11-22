use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub mod media;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct ExecutionCount(pub usize);

/// A monotonically increasing counter for tracking the number of times a cell
/// has been executed.
impl ExecutionCount {
    pub fn new(count: usize) -> Self {
        Self(count)
    }
}

impl From<usize> for ExecutionCount {
    fn from(count: usize) -> Self {
        Self(count)
    }
}

impl From<ExecutionCount> for usize {
    fn from(count: ExecutionCount) -> Self {
        count.0
    }
}

impl From<ExecutionCount> for Value {
    fn from(count: ExecutionCount) -> Self {
        Value::Number(count.0.into())
    }
}

impl ExecutionCount {
    pub fn increment(&mut self) {
        self.0 += 1;
    }

    pub fn value(&self) -> usize {
        self.0
    }
}

impl std::fmt::Display for ExecutionCount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Contents of a Jupyter JSON kernelspec file
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct JupyterKernelspec {
    /// argv must contain `{connection_file}` to be replaced by the client launching the kernel
    /// For example, `["python3", "-m", "ipykernel_launcher", "-f", "{connection_file}"]`
    #[serde(default)]
    pub argv: Vec<String>,
    pub display_name: String,
    pub language: String,
    pub metadata: Option<HashMap<String, Value>>,
    pub interrupt_mode: Option<String>,
    pub env: Option<HashMap<String, String>>,
}
