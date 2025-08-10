//! # Workspace Management System
//! 
//! Enhanced workspace management with proper separation of concerns between
//! global and workspace-specific data, contexts, and agent rules.

pub mod manager;
pub mod context;
pub mod isolation;
pub mod chat_context;
pub mod components;
pub mod graph_3d;
pub mod graph_builder;

// Re-export main types
pub use manager::*;
pub use context::*;
pub use isolation::*;
pub use chat_context::*;
pub use components::*;
pub use graph_3d::*;
pub use graph_builder::*;

use crate::{Result, SymbioteError};
use crate::context::{ContextBus, GlobalContext};
use crate::memory::{MemorySystem, AgentRuleManager};
use crate::assistant::Symbiote;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Enhanced workspace manager supporting multiple concurrent workspaces
/// with proper isolation and global/workspace separation of concerns
#[derive(Debug)]
pub struct WorkspaceManager {
    /// All open workspaces
    workspaces: Arc<RwLock<HashMap<String, EnhancedWorkspaceContext>>>,
    
    /// Currently active workspace
    active_workspace_id: Arc<RwLock<Option<String>>>,
    
    /// Global context shared across workspaces
    global_context: Arc<RwLock<EnhancedGlobalContext>>,
    
    /// Chat context manager for workspace switching
    chat_context: ChatContextManager,
    
    /// Integration with existing context bus
    context_bus: Option<Arc<ContextBus>>,
}

impl WorkspaceManager {
    /// Create new workspace manager
    pub fn new() -> Self {
        Self {
            workspaces: Arc::new(RwLock::new(HashMap::new())),
            active_workspace_id: Arc::new(RwLock::new(None)),
            global_context: Arc::new(RwLock::new(EnhancedGlobalContext::new())),
            chat_context: ChatContextManager::new(),
            context_bus: None,
        }
    }

    /// Create workspace manager with context bus integration
    pub fn with_context_bus(context_bus: Arc<ContextBus>) -> Self {
        let mut manager = Self::new();
        manager.context_bus = Some(context_bus);
        manager
    }

    /// Open a new workspace
    pub async fn open_workspace(&self, workspace_path: String, workspace_name: String) -> Result<String> {
        let workspace_id = Uuid::new_v4().to_string();
        
        let workspace = EnhancedWorkspaceContext::new(
            workspace_id.clone(),
            workspace_path,
            workspace_name,
        );
        
        // Store workspace
        {
            let mut workspaces = self.workspaces.write().await;
            workspaces.insert(workspace_id.clone(), workspace);
        }
        
        // Set as active if no active workspace
        {
            let mut active = self.active_workspace_id.write().await;
            if active.is_none() {
                *active = Some(workspace_id.clone());
            }
        }
        
        // Update global context
        {
            let mut global = self.global_context.write().await;
            global.workspace_ids.insert(workspace_id.clone());
        }
        
        // Notify context bus
        if let Some(context_bus) = &self.context_bus {
            // Would integrate with existing context bus
        }
        
        Ok(workspace_id)
    }

    /// Close a workspace
    pub async fn close_workspace(&self, workspace_id: &str) -> Result<()> {
        // Remove from workspaces
        {
            let mut workspaces = self.workspaces.write().await;
            workspaces.remove(workspace_id);
        }
        
        // Update active workspace if this was active
        {
            let mut active = self.active_workspace_id.write().await;
            if active.as_ref() == Some(&workspace_id.to_string()) {
                // Set to another workspace or None
                let workspaces = self.workspaces.read().await;
                *active = workspaces.keys().next().cloned();
            }
        }
        
        // Update global context
        {
            let mut global = self.global_context.write().await;
            global.workspace_ids.remove(workspace_id);
        }
        
        Ok(())
    }

    /// Switch active workspace
    pub async fn switch_workspace(&self, workspace_id: &str) -> Result<()> {
        // Verify workspace exists
        {
            let workspaces = self.workspaces.read().await;
            if !workspaces.contains_key(workspace_id) {
                return Err(SymbioteError::NotFound(format!("Workspace not found: {}", workspace_id)));
            }
        }
        
        // Update active workspace
        {
            let mut active = self.active_workspace_id.write().await;
            *active = Some(workspace_id.to_string());
        }
        
        // Update chat context if in workspace mode
        self.chat_context.update_workspace_context(Some(workspace_id.to_string())).await?;
        
        Ok(())
    }

    /// Get workspace context
    pub async fn get_workspace(&self, workspace_id: &str) -> Result<EnhancedWorkspaceContext> {
        let workspaces = self.workspaces.read().await;
        workspaces.get(workspace_id)
            .cloned()
            .ok_or_else(|| SymbioteError::NotFound(format!("Workspace not found: {}", workspace_id)))
    }

    /// Get active workspace
    pub async fn get_active_workspace(&self) -> Result<Option<EnhancedWorkspaceContext>> {
        let active_id = {
            let active = self.active_workspace_id.read().await;
            active.clone()
        };
        
        if let Some(workspace_id) = active_id {
            Ok(Some(self.get_workspace(&workspace_id).await?))
        } else {
            Ok(None)
        }
    }

    /// List all workspaces
    pub async fn list_workspaces(&self) -> Vec<WorkspaceSummary> {
        let workspaces = self.workspaces.read().await;
        workspaces.values().map(|w| WorkspaceSummary {
            id: w.id.clone(),
            name: w.name.clone(),
            path: w.path.clone(),
            is_active: false, // Would check against active_workspace_id
            file_count: w.files.len(),
            last_accessed: w.last_accessed,
        }).collect()
    }

    /// Get global context
    pub async fn get_global_context(&self) -> EnhancedGlobalContext {
        let global = self.global_context.read().await;
        global.clone()
    }

    /// Get chat context manager
    pub fn get_chat_context(&self) -> &ChatContextManager {
        &self.chat_context
    }

    /// Get workspace-specific agent rules
    pub async fn get_workspace_agent_rules(&self, workspace_id: &str, agent_type: &str, user_id: &str) -> Result<crate::memory::AgentRules> {
        let workspace = self.get_workspace(workspace_id).await?;
        workspace.agent_rules.get_rules(agent_type, user_id).await
    }

    /// Update workspace-specific agent rules
    pub async fn update_workspace_agent_rules(
        &self, 
        workspace_id: &str, 
        agent_type: &str, 
        user_id: &str, 
        rules: crate::memory::AgentRules
    ) -> Result<()> {
        let mut workspaces = self.workspaces.write().await;
        if let Some(workspace) = workspaces.get_mut(workspace_id) {
            workspace.agent_rules.update_rules(agent_type, user_id, rules).await?;
            workspace.last_modified = Utc::now();
        }
        Ok(())
    }

    /// Get workspace statistics
    pub async fn get_workspace_stats(&self, workspace_id: &str) -> Result<WorkspaceStats> {
        let workspace = self.get_workspace(workspace_id).await?;

        Ok(WorkspaceStats {
            file_count: workspace.files.len(),
            conversation_count: workspace.conversations.len(),
            workflow_count: workspace.workflows.len(),
            notebook_count: workspace.notebooks.len(),
            agent_rule_count: workspace.agent_rules.get_total_rules().await,
            memory_size: workspace.memory.get_size().await,
            task_count: workspace.tasks.tasks.len(),
            note_count: workspace.notes.notes.len(),
            goal_count: workspace.goals.goals.len(),
            journal_entry_count: workspace.journal.daily_entries.len(),
            last_accessed: workspace.last_accessed,
            created_at: workspace.created_at,
        })
    }

    /// Create task in workspace
    pub async fn create_workspace_task(&self, workspace_id: &str, title: String, description: Option<String>) -> Result<String> {
        let mut workspaces = self.workspaces.write().await;
        if let Some(workspace) = workspaces.get_mut(workspace_id) {
            let task_id = workspace.tasks.create_task(title, description);
            workspace.last_modified = Utc::now();
            Ok(task_id)
        } else {
            Err(SymbioteError::NotFound("Workspace not found".to_string()))
        }
    }

    /// Create note in workspace
    pub async fn create_workspace_note(&self, workspace_id: &str, title: String, content: String, note_type: NoteType) -> Result<String> {
        let mut workspaces = self.workspaces.write().await;
        if let Some(workspace) = workspaces.get_mut(workspace_id) {
            let note_id = workspace.notes.create_note(title, content, note_type);
            workspace.last_modified = Utc::now();
            Ok(note_id)
        } else {
            Err(SymbioteError::NotFound("Workspace not found".to_string()))
        }
    }

    /// Create goal in workspace
    pub async fn create_workspace_goal(&self, workspace_id: &str, title: String, description: Option<String>, goal_type: GoalType) -> Result<String> {
        let mut workspaces = self.workspaces.write().await;
        if let Some(workspace) = workspaces.get_mut(workspace_id) {
            let goal_id = workspace.goals.create_goal(title, description, goal_type);
            workspace.last_modified = Utc::now();
            Ok(goal_id)
        } else {
            Err(SymbioteError::NotFound("Workspace not found".to_string()))
        }
    }

    /// Add journal entry to workspace
    pub async fn add_workspace_journal_entry(&self, workspace_id: &str, date: chrono::NaiveDate, entry: JournalEntry) -> Result<()> {
        let mut workspaces = self.workspaces.write().await;
        if let Some(workspace) = workspaces.get_mut(workspace_id) {
            workspace.journal.add_daily_entry(date, entry);
            workspace.last_modified = Utc::now();
            Ok(())
        } else {
            Err(SymbioteError::NotFound("Workspace not found".to_string()))
        }
    }

    /// Get workspace productivity metrics
    pub async fn get_workspace_productivity(&self, workspace_id: &str) -> Result<context::ProductivityMetrics> {
        let workspace = self.get_workspace(workspace_id).await?;
        // Convert from components::ProductivityMetrics to context::ProductivityMetrics
        Ok(context::ProductivityMetrics {
            lines_of_code: workspace.analytics.productivity_metrics.lines_of_code,
            commits_per_day: workspace.analytics.productivity_metrics.commits_per_day,
            files_modified: workspace.analytics.productivity_metrics.files_modified,
            build_success_rate: workspace.analytics.productivity_metrics.build_success_rate,
            test_coverage: workspace.analytics.productivity_metrics.test_coverage,
        })
    }

    /// Build 3D graph for workspace
    pub async fn build_workspace_3d_graph(&self, workspace_id: &str) -> Result<Workspace3DGraph> {
        let workspace = self.get_workspace(workspace_id).await?;
        let mut builder = Workspace3DGraphBuilder::new(workspace_id.to_string());
        builder.build_from_workspace(&workspace).await
    }

    /// Get workspace 3D graph
    pub async fn get_workspace_3d_graph(&self, workspace_id: &str) -> Result<Workspace3DGraph> {
        let workspace = self.get_workspace(workspace_id).await?;
        Ok(workspace.graph_3d.clone())
    }

    /// Update workspace 3D graph
    pub async fn update_workspace_3d_graph(&self, workspace_id: &str) -> Result<()> {
        let graph = self.build_workspace_3d_graph(workspace_id).await?;

        let mut workspaces = self.workspaces.write().await;
        if let Some(workspace) = workspaces.get_mut(workspace_id) {
            workspace.graph_3d = graph;
            workspace.last_modified = Utc::now();
        }

        Ok(())
    }

    /// Get 3D graph statistics
    pub async fn get_3d_graph_stats(&self, workspace_id: &str) -> Result<Graph3DStatistics> {
        let workspace = self.get_workspace(workspace_id).await?;
        Ok(workspace.graph_3d.get_statistics())
    }

    /// Filter 3D graph
    pub async fn filter_3d_graph(&self, workspace_id: &str, filter: &GraphFilter) -> Result<Workspace3DGraph> {
        let workspace = self.get_workspace(workspace_id).await?;
        Ok(workspace.graph_3d.filter(filter))
    }
}

/// Workspace summary for listing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceSummary {
    pub id: String,
    pub name: String,
    pub path: String,
    pub is_active: bool,
    pub file_count: usize,
    pub last_accessed: DateTime<Utc>,
}

/// Workspace statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceStats {
    pub file_count: usize,
    pub conversation_count: usize,
    pub workflow_count: usize,
    pub notebook_count: usize,
    pub agent_rule_count: usize,
    pub memory_size: u64,
    pub task_count: usize,
    pub note_count: usize,
    pub goal_count: usize,
    pub journal_entry_count: usize,
    pub last_accessed: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

impl Default for WorkspaceManager {
    fn default() -> Self {
        Self::new()
    }
}
