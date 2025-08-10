//! IDE agent for handling IDE-related tasks

use super::*;
use std::collections::HashMap;

/// IDE agent for handling IDE operations
#[derive(Debug)]
pub struct IDEAgent {
    config: AgentConfig,
}

impl IDEAgent {
    pub fn new() -> Self {
        Self {
            config: AgentConfig::default(),
        }
    }
}

#[async_trait]
impl SpecializedAgent for IDEAgent {
    fn agent_type(&self) -> &str {
        "ide"
    }

    fn display_name(&self) -> &str {
        "IDE Agent"
    }

    fn description(&self) -> &str {
        "Handles IDE-related tasks including code generation and file management"
    }

    fn capabilities(&self) -> Vec<String> {
        vec![
            "code_generate".to_string(),
            "file_manage".to_string(),
        ]
    }

    async fn can_handle_task(&self, task: &Task) -> bool {
        matches!(task.task_type.as_str(),
                "code_generate" | "file_manage")
    }

    async fn execute_task(&self, task: &Task) -> Result<AgentResult> {
        Ok(AgentResult {
            agent_id: "ide".to_string(),
            task_id: task.id.clone(),
            success: true,
            output: Some(serde_json::json!({"status": "not_implemented"})),
            error: None,
            execution_time: 100,
            metadata: HashMap::new(),
        })
    }

    async fn get_status(&self) -> AgentStatus {
        AgentStatus::Available
    }

    async fn get_load(&self) -> f64 {
        0.0
    }

    async fn cancel_task(&self, _task_id: &str) -> Result<()> {
        Ok(())
    }

    fn get_config(&self) -> AgentConfig {
        self.config.clone()
    }

    async fn update_config(&self, config: AgentConfig) -> Result<()> {
        let _ = config;
        Ok(())
    }
}

impl Default for IDEAgent {
    fn default() -> Self {
        Self::new()
    }
}
