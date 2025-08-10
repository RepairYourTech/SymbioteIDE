//! # Notebook System - Main Orchestrator
//! 
//! Main notebook system managing kernels, execution, variables, output rendering,
//! and collaboration features.

use super::*;
use crate::context::ContextBus;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Main notebook system managing all notebook operations
#[derive(Debug)]
pub struct NotebookSystem {
    kernels: Arc<RwLock<HashMap<String, Box<dyn NotebookKernel>>>>,
    execution_engine: ExecutionEngine,
    variable_bridge: VariableBridge,
    output_renderer: OutputRenderer,
    collaboration_engine: Arc<RwLock<NotebookCollaboration>>,
    sessions: Arc<RwLock<HashMap<String, NotebookSession>>>,
    notebooks: Arc<RwLock<HashMap<String, Notebook>>>,
    context_bus: Option<Arc<ContextBus>>,
}

impl NotebookSystem {
    /// Create new notebook system
    pub fn new() -> Self {
        Self {
            kernels: Arc::new(RwLock::new(HashMap::new())),
            execution_engine: ExecutionEngine::new(),
            variable_bridge: VariableBridge::new(),
            output_renderer: OutputRenderer::new(),
            collaboration_engine: Arc::new(RwLock::new(NotebookCollaboration::new())),
            sessions: Arc::new(RwLock::new(HashMap::new())),
            notebooks: Arc::new(RwLock::new(HashMap::new())),
            context_bus: None,
        }
    }

    /// Create notebook system with context bus integration
    pub fn with_context_bus(context_bus: Arc<ContextBus>) -> Self {
        let mut system = Self::new();
        system.context_bus = Some(context_bus);
        system
    }

    /// Register a kernel
    pub async fn register_kernel(&self, name: String, kernel: Box<dyn NotebookKernel>) -> Result<()> {
        let mut kernels = self.kernels.write().await;
        kernels.insert(name, kernel);
        Ok(())
    }

    /// Get available kernels
    pub async fn get_available_kernels(&self) -> Vec<String> {
        let kernels = self.kernels.read().await;
        kernels.keys().cloned().collect()
    }

    /// Create a new notebook
    pub async fn create_notebook(&self, name: String, kernel_name: String) -> Result<String> {
        let kernels = self.kernels.read().await;
        let kernel = kernels.get(&kernel_name)
            .ok_or_else(|| SymbioteError::NotFound(format!("Kernel not found: {}", kernel_name)))?;

        let kernel_spec = KernelSpec {
            name: kernel_name.clone(),
            display_name: kernel.display_name().to_string(),
            language: kernel.language().to_string(),
            version: kernel.version().to_string(),
            executable: kernel_name.clone(),
            args: Vec::new(),
            env: HashMap::new(),
        };

        let notebook = Notebook::new(name, kernel_spec);
        let notebook_id = notebook.id.clone();

        let mut notebooks = self.notebooks.write().await;
        notebooks.insert(notebook_id.clone(), notebook);

        // Notify context bus if available
        if let Some(context_bus) = &self.context_bus {
            // Would integrate with context bus here
        }

        Ok(notebook_id)
    }

    /// Open an existing notebook
    pub async fn open_notebook(&self, notebook_id: &str) -> Result<Notebook> {
        let notebooks = self.notebooks.read().await;
        notebooks.get(notebook_id)
            .cloned()
            .ok_or_else(|| SymbioteError::NotFound(format!("Notebook not found: {}", notebook_id)))
    }

    /// Save notebook
    pub async fn save_notebook(&self, notebook: Notebook) -> Result<()> {
        let mut notebooks = self.notebooks.write().await;
        notebooks.insert(notebook.id.clone(), notebook);
        Ok(())
    }

    /// Start a notebook session
    pub async fn start_session(&self, notebook_id: String, kernel_name: String) -> Result<String> {
        let kernels = self.kernels.read().await;
        let kernel = kernels.get(&kernel_name)
            .ok_or_else(|| SymbioteError::NotFound(format!("Kernel not found: {}", kernel_name)))?;

        // Clone the kernel for the session (would need proper kernel instantiation)
        let session_kernel = kernel.get_info(); // Placeholder - would create new kernel instance
        
        // For now, create a mock session
        let session_id = Uuid::new_v4().to_string();
        
        // Would create actual session with kernel instance
        // let session = NotebookSession::new(notebook_id, session_kernel);
        // let mut sessions = self.sessions.write().await;
        // sessions.insert(session_id.clone(), session);

        Ok(session_id)
    }

    /// Execute a cell
    pub async fn execute_cell(
        &self,
        session_id: &str,
        cell_id: &str,
        code: &str,
    ) -> Result<ExecutionResult> {
        // Get session
        let sessions = self.sessions.read().await;
        let _session = sessions.get(session_id)
            .ok_or_else(|| SymbioteError::NotFound(format!("Session not found: {}", session_id)))?;

        // Create execution context
        let context = ExecutionContext::new(1, cell_id.to_string(), "notebook".to_string());

        // Execute through execution engine
        self.execution_engine.execute_code(code, &context).await
    }

    /// Get notebook variables
    pub async fn get_variables(&self, session_id: &str) -> Result<HashMap<String, Variable>> {
        let sessions = self.sessions.read().await;
        let session = sessions.get(session_id)
            .ok_or_else(|| SymbioteError::NotFound(format!("Session not found: {}", session_id)))?;

        session.kernel.get_variables().await
    }

    /// Set notebook variable
    pub async fn set_variable(&self, session_id: &str, name: &str, value: &Variable) -> Result<()> {
        let sessions = self.sessions.read().await;
        let session = sessions.get(session_id)
            .ok_or_else(|| SymbioteError::NotFound(format!("Session not found: {}", session_id)))?;

        session.kernel.set_variable(name, value).await
    }

    /// Share variables between sessions
    pub async fn share_variables(
        &self,
        source_session: &str,
        target_session: &str,
        variable_names: Vec<String>,
    ) -> Result<()> {
        self.variable_bridge.transfer_variables(source_session, target_session, variable_names).await
    }

    /// Render output
    pub async fn render_output(&self, output: &CellOutput) -> Result<RenderedOutput> {
        self.output_renderer.render(output).await
    }

    /// Start collaboration session
    pub async fn start_collaboration(&self, notebook_id: &str, user_id: &str) -> Result<String> {
        let mut engine = self.collaboration_engine.write().await;
        engine.start_session(notebook_id, user_id).await
    }

    /// Stop collaboration session
    pub async fn stop_collaboration(&self, collaboration_id: &str) -> Result<()> {
        let mut engine = self.collaboration_engine.write().await;
        engine.stop_session(collaboration_id).await
    }

    /// Get notebook statistics
    pub async fn get_notebook_stats(&self, notebook_id: &str) -> Result<NotebookStats> {
        let notebooks = self.notebooks.read().await;
        let notebook = notebooks.get(notebook_id)
            .ok_or_else(|| SymbioteError::NotFound(format!("Notebook not found: {}", notebook_id)))?;

        let execution_stats = notebook.get_execution_stats();
        let total_cells = notebook.cells.len();
        let code_cells = notebook.get_code_cells().len();
        let markdown_cells = notebook.cells.iter()
            .filter(|c| matches!(c.cell_type, CellType::Markdown))
            .count();

        Ok(NotebookStats {
            total_cells,
            code_cells,
            markdown_cells,
            execution_stats,
            last_modified: notebook.modified_at,
            size_bytes: self.calculate_notebook_size(notebook),
        })
    }

    /// List all notebooks
    pub async fn list_notebooks(&self) -> Vec<NotebookSummary> {
        let notebooks = self.notebooks.read().await;
        notebooks.values().map(|notebook| NotebookSummary {
            id: notebook.id.clone(),
            name: notebook.name.clone(),
            kernel_language: notebook.kernel_spec.language.clone(),
            cell_count: notebook.cells.len(),
            created_at: notebook.created_at,
            modified_at: notebook.modified_at,
        }).collect()
    }

    /// Delete notebook
    pub async fn delete_notebook(&self, notebook_id: &str) -> Result<()> {
        let mut notebooks = self.notebooks.write().await;
        notebooks.remove(notebook_id)
            .ok_or_else(|| SymbioteError::NotFound(format!("Notebook not found: {}", notebook_id)))?;
        Ok(())
    }

    /// Shutdown system
    pub async fn shutdown(&self) -> Result<()> {
        // Stop all sessions
        let sessions = self.sessions.read().await;
        for session in sessions.values() {
            let _ = session.kernel.shutdown().await;
        }

        // Stop collaboration
        let mut engine = self.collaboration_engine.write().await;
        engine.shutdown().await?;

        Ok(())
    }

    fn calculate_notebook_size(&self, notebook: &Notebook) -> u64 {
        // Rough calculation of notebook size
        let mut size = 0u64;
        
        // Metadata size
        size += notebook.name.len() as u64;
        
        // Cells size
        for cell in &notebook.cells {
            size += cell.content.len() as u64;
            for output in &cell.outputs {
                size += output.data.mime_bundle.len() as u64 * 100; // Rough estimate
            }
        }
        
        size
    }
}

/// Notebook statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotebookStats {
    pub total_cells: usize,
    pub code_cells: usize,
    pub markdown_cells: usize,
    pub execution_stats: ExecutionStats,
    pub last_modified: DateTime<Utc>,
    pub size_bytes: u64,
}

/// Notebook summary for listing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotebookSummary {
    pub id: String,
    pub name: String,
    pub kernel_language: String,
    pub cell_count: usize,
    pub created_at: DateTime<Utc>,
    pub modified_at: DateTime<Utc>,
}

impl Default for NotebookSystem {
    fn default() -> Self {
        Self::new()
    }
}
