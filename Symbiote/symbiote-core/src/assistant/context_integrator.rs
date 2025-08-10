//! # Context Integrator
//! 
//! Deep integration with Symbiote's context systems for workspace awareness.

use super::*;
use crate::context::ContextBus;
use std::sync::Arc;

/// Context integrator for deep Symbiote integration
#[derive(Debug)]
pub struct ContextIntegrator {
    context_bus: Option<Arc<ContextBus>>,
    current_panel: Option<PanelInfo>,
    workspace_context: WorkspaceSummary,
}

impl ContextIntegrator {
    pub fn new() -> Self {
        Self {
            context_bus: None,
            current_panel: None,
            workspace_context: WorkspaceSummary {
                project_name: None,
                open_files: Vec::new(),
                recent_activity: Vec::new(),
                git_status: None,
            },
        }
    }

    pub fn set_context_bus(&mut self, context_bus: Arc<ContextBus>) {
        self.context_bus = Some(context_bus);
    }

    pub async fn get_current_context(&self) -> Result<SymbioteContext> {
        Ok(SymbioteContext {
            panel_info: self.current_panel.clone(),
            workspace: self.workspace_context.clone(),
            active_workflows: self.get_active_workflow_count().await,
            open_notebooks: self.get_open_notebook_count().await,
        })
    }

    pub async fn update_panel_context(&mut self, panel_info: PanelInfo) -> Result<()> {
        self.current_panel = Some(panel_info);
        Ok(())
    }

    pub async fn get_active_workflow_count(&self) -> usize {
        // Would query actual workflow system
        0
    }

    pub async fn get_open_notebook_count(&self) -> usize {
        // Would query actual notebook system
        0
    }

    pub async fn get_current_panel(&self) -> Option<PanelInfo> {
        self.current_panel.clone()
    }

    pub async fn get_workspace_summary(&self) -> WorkspaceSummary {
        self.workspace_context.clone()
    }
}

/// Symbiote context information
#[derive(Debug, Clone)]
pub struct SymbioteContext {
    pub panel_info: Option<PanelInfo>,
    pub workspace: WorkspaceSummary,
    pub active_workflows: usize,
    pub open_notebooks: usize,
}
