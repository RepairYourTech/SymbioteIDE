// SymbioteIDE - Workflow Automation
// Workflow Automation System

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowEngine {
    pub name: String,
}

impl WorkflowEngine {
    pub fn new() -> Self {
        Self {
            name: "Workflow Engine".to_string(),
        }
    }
}

impl Default for WorkflowEngine {
    fn default() -> Self {
        Self::new()
    }
}
