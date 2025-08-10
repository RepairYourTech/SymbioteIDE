//! # Workspace Isolation
//! 
//! Utilities for ensuring proper workspace isolation and data separation.

use super::*;

/// Workspace isolation manager
#[derive(Debug)]
pub struct WorkspaceIsolationManager {
    isolation_policies: HashMap<String, IsolationPolicy>,
}

/// Isolation policy for workspace data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IsolationPolicy {
    pub workspace_id: String,
    pub isolate_files: bool,
    pub isolate_conversations: bool,
    pub isolate_agent_rules: bool,
    pub isolate_memory: bool,
    pub allow_cross_workspace_search: bool,
    pub share_global_preferences: bool,
}

impl WorkspaceIsolationManager {
    pub fn new() -> Self {
        Self {
            isolation_policies: HashMap::new(),
        }
    }

    /// Set isolation policy for workspace
    pub fn set_policy(&mut self, workspace_id: String, policy: IsolationPolicy) {
        self.isolation_policies.insert(workspace_id, policy);
    }

    /// Get isolation policy for workspace
    pub fn get_policy(&self, workspace_id: &str) -> IsolationPolicy {
        self.isolation_policies.get(workspace_id)
            .cloned()
            .unwrap_or_else(|| IsolationPolicy::default_for_workspace(workspace_id))
    }

    /// Check if data should be isolated
    pub fn should_isolate(&self, workspace_id: &str, data_type: DataType) -> bool {
        let policy = self.get_policy(workspace_id);
        
        match data_type {
            DataType::Files => policy.isolate_files,
            DataType::Conversations => policy.isolate_conversations,
            DataType::AgentRules => policy.isolate_agent_rules,
            DataType::Memory => policy.isolate_memory,
        }
    }
}

/// Types of data that can be isolated
#[derive(Debug, Clone)]
pub enum DataType {
    Files,
    Conversations,
    AgentRules,
    Memory,
}

impl IsolationPolicy {
    /// Default isolation policy for workspace
    pub fn default_for_workspace(workspace_id: &str) -> Self {
        Self {
            workspace_id: workspace_id.to_string(),
            isolate_files: true,
            isolate_conversations: true,
            isolate_agent_rules: true,
            isolate_memory: true,
            allow_cross_workspace_search: false,
            share_global_preferences: true,
        }
    }

    /// Relaxed isolation policy (allows more sharing)
    pub fn relaxed_for_workspace(workspace_id: &str) -> Self {
        Self {
            workspace_id: workspace_id.to_string(),
            isolate_files: true,
            isolate_conversations: false,
            isolate_agent_rules: true,
            isolate_memory: false,
            allow_cross_workspace_search: true,
            share_global_preferences: true,
        }
    }
}

impl Default for WorkspaceIsolationManager {
    fn default() -> Self {
        Self::new()
    }
}
