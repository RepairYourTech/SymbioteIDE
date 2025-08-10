//! Workflow Template Library

use crate::workflow::{Workflow, AIWorkflowTemplate};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Workflow template library
#[derive(Debug)]
pub struct WorkflowTemplateLibrary {
    /// Standard templates
    templates: HashMap<String, WorkflowTemplate>,
    
    /// AI templates
    ai_templates: HashMap<String, AIWorkflowTemplate>,
}

/// Workflow template
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowTemplate {
    pub template_id: String,
    pub name: String,
    pub description: String,
    pub category: TemplateCategory,
    pub workflow: Workflow,
    pub use_cases: Vec<String>,
}

/// Template categories
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TemplateCategory {
    DataProcessing,
    Integration,
    Automation,
    Analytics,
    Communication,
    Development,
}

impl WorkflowTemplateLibrary {
    pub fn new() -> Self {
        Self {
            templates: HashMap::new(),
            ai_templates: HashMap::new(),
        }
    }
}

impl Clone for WorkflowTemplateLibrary {
    fn clone(&self) -> Self {
        WorkflowTemplateLibrary::new()
    }
}
