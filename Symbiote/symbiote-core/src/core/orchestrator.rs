//! Agent orchestrator

use crate::{Result, AgentId};

/// Agent orchestrator
pub struct AgentOrchestrator {
    agents: std::collections::HashMap<AgentId, String>,
}

impl AgentOrchestrator {
    pub fn new() -> Self {
        Self {
            agents: std::collections::HashMap::new(),
        }
    }

    pub async fn register_agent(&mut self, agent_id: AgentId, agent_type: String) -> Result<()> {
        self.agents.insert(agent_id, agent_type);
        Ok(())
    }

    pub fn get_agent_count(&self) -> usize {
        self.agents.len()
    }
}

impl Default for AgentOrchestrator {
    fn default() -> Self {
        Self::new()
    }
}
