//! File system agent for handling file operations

use super::*;
use std::collections::HashMap;

/// File system agent for handling file operations
#[derive(Debug)]
pub struct FileSystemAgent {
    config: AgentConfig,
}

impl FileSystemAgent {
    pub fn new() -> Self {
        Self {
            config: AgentConfig::default(),
        }
    }
}

#[async_trait]
impl SpecializedAgent for FileSystemAgent {
    fn agent_type(&self) -> &str {
        "file_system"
    }

    fn display_name(&self) -> &str {
        "File System Agent"
    }

    fn description(&self) -> &str {
        "Handles file system operations including file management and directory operations"
    }

    fn capabilities(&self) -> Vec<String> {
        vec![
            "file_manage".to_string(),
            "directory_ops".to_string(),
        ]
    }

    async fn can_handle_task(&self, task: &Task) -> bool {
        matches!(task.task_type.as_str(),
                "file_manage" | "directory_ops")
    }

    async fn execute_task(&self, task: &Task) -> Result<AgentResult> {
        Ok(AgentResult {
            agent_id: "file_system".to_string(),
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

impl Default for FileSystemAgent {
    fn default() -> Self {
        Self::new()
    }
}
