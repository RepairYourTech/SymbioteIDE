// SymbioteIDE - Multi-Agent Orchestrator
// 26+ Specialized Agents System

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentOrchestrator {
    pub name: String,
}

impl AgentOrchestrator {
    pub fn new() -> Self {
        Self {
            name: "Agent Orchestrator".to_string(),
        }
    }
}

impl Default for AgentOrchestrator {
    fn default() -> Self {
        Self::new()
    }
}
