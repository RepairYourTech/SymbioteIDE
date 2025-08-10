//! # Task Decomposer
//! 
//! Breaks down complex user requests into actionable tasks for agents.

use super::*;

/// Task decomposer for breaking down complex requests
#[derive(Debug)]
pub struct TaskDecomposer {
    decomposition_strategies: Vec<DecompositionStrategy>,
}

impl TaskDecomposer {
    pub fn new() -> Self {
        Self {
            decomposition_strategies: vec![
                DecompositionStrategy::Sequential,
                DecompositionStrategy::Parallel,
                DecompositionStrategy::Hierarchical,
            ],
        }
    }

    pub async fn decompose_intent(
        &self,
        intent: &UserIntent,
        context: &SymbioteContext,
    ) -> Result<Vec<Task>> {
        let mut tasks = Vec::new();
        
        // Simple decomposition based on intent type
        match intent.intent_type.as_str() {
            "web_search" => {
                tasks.push(Task {
                    id: Uuid::new_v4().to_string(),
                    task_type: "web_search".to_string(),
                    description: format!("Search for: {}", intent.query),
                    parameters: intent.parameters.clone(),
                    context: HashMap::new(),
                    dependencies: None,
                    priority: 1,
                    timeout: Some(300),
                });
            },
            "create_workflow" => {
                tasks.push(Task {
                    id: Uuid::new_v4().to_string(),
                    task_type: "create_workflow".to_string(),
                    description: "Create new workflow".to_string(),
                    parameters: intent.parameters.clone(),
                    context: HashMap::new(),
                    dependencies: None,
                    priority: 1,
                    timeout: Some(600),
                });
            },
            _ => {
                tasks.push(Task {
                    id: Uuid::new_v4().to_string(),
                    task_type: intent.intent_type.clone(),
                    description: intent.query.clone(),
                    parameters: intent.parameters.clone(),
                    context: HashMap::new(),
                    dependencies: None,
                    priority: 1,
                    timeout: Some(300),
                });
            }
        }
        
        Ok(tasks)
    }
}

/// Decomposition strategies
#[derive(Debug, Clone)]
pub enum DecompositionStrategy {
    Sequential,
    Parallel,
    Hierarchical,
}

/// User intent parsed from natural language
#[derive(Debug, Clone)]
pub struct UserIntent {
    pub intent_type: String,
    pub query: String,
    pub confidence: f64,
    pub parameters: HashMap<String, serde_json::Value>,
    pub entities: Vec<String>,
}
