//! # Workspace Manager Implementation
//! 
//! Core workspace management functionality.

use super::*;

// Re-export the WorkspaceManager from mod.rs
pub use super::WorkspaceManager;

/// Workspace manager utilities
impl WorkspaceManager {
    /// Import workspace from path
    pub async fn import_workspace(&self, workspace_path: String) -> Result<String> {
        // Detect workspace type and configuration
        let workspace_name = std::path::Path::new(&workspace_path)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("Imported Workspace")
            .to_string();
        
        self.open_workspace(workspace_path, workspace_name).await
    }

    /// Export workspace configuration
    pub async fn export_workspace(&self, workspace_id: &str) -> Result<WorkspaceExport> {
        let workspace = self.get_workspace(workspace_id).await?;
        
        Ok(WorkspaceExport {
            workspace_info: WorkspaceInfo {
                id: workspace.id.clone(),
                name: workspace.name.clone(),
                path: workspace.path.clone(),
                project_type: workspace.project_type.clone(),
            },
            settings: workspace.settings.clone(),
            agent_rules: self.export_workspace_agent_rules(workspace_id).await?,
            bookmarks: workspace.bookmarks.clone(),
            recent_files: workspace.recent_files.clone(),
        })
    }

    /// Duplicate workspace
    pub async fn duplicate_workspace(&self, workspace_id: &str, new_name: String, new_path: String) -> Result<String> {
        let source_workspace = self.get_workspace(workspace_id).await?;
        let new_workspace_id = self.open_workspace(new_path, new_name).await?;
        
        // Copy settings and configuration
        let mut workspaces = self.workspaces.write().await;
        if let Some(new_workspace) = workspaces.get_mut(&new_workspace_id) {
            new_workspace.settings = source_workspace.settings.clone();
            new_workspace.bookmarks = source_workspace.bookmarks.clone();
            // Note: Don't copy files, conversations, etc. - just configuration
        }
        
        Ok(new_workspace_id)
    }

    async fn export_workspace_agent_rules(&self, workspace_id: &str) -> Result<HashMap<String, serde_json::Value>> {
        // Would export all agent rules for this workspace
        Ok(HashMap::new())
    }
}

/// Workspace export data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceExport {
    pub workspace_info: WorkspaceInfo,
    pub settings: WorkspaceSettings,
    pub agent_rules: HashMap<String, serde_json::Value>,
    pub bookmarks: Vec<Bookmark>,
    pub recent_files: Vec<String>,
}

/// Basic workspace information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceInfo {
    pub id: String,
    pub name: String,
    pub path: String,
    pub project_type: Option<String>,
}
