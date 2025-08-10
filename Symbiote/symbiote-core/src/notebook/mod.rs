//! # Notebook System - Interactive Development Environment
//! 
//! Complete notebook system implementation for Symbiote IDE providing
//! Jupyter-like functionality with multi-language support, real-time collaboration,
//! and deep integration with the IDE's AI and workflow systems.
//! 
//! Following Week 15-16 Advanced Features & Specialized Systems implementation plan.

pub mod system;
pub mod kernel;
pub mod kernels;
pub mod execution;
pub mod variables;
pub mod output;
pub mod collaboration;
pub mod cell;

#[cfg(test)]
pub mod tests;

// Re-export main types
pub use system::*;
pub use kernel::*;
pub use execution::*;
pub use variables::*;
pub use output::*;
pub use collaboration::*;
pub use cell::*;

// Re-export kernel implementations
pub use kernels::*;

use crate::{Result, SymbioteError};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Main notebook system managing all notebook operations
pub type NotebookManager = system::NotebookSystem;

/// Kernel registry for managing available language kernels
pub type KernelRegistry = HashMap<String, Box<dyn NotebookKernel>>;

/// Notebook document containing cells and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notebook {
    pub id: String,
    pub name: String,
    pub cells: Vec<NotebookCell>,
    pub metadata: NotebookMetadata,
    pub kernel_spec: KernelSpec,
    pub created_at: DateTime<Utc>,
    pub modified_at: DateTime<Utc>,
    pub version: String,
}

/// Notebook metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotebookMetadata {
    pub title: Option<String>,
    pub description: Option<String>,
    pub authors: Vec<String>,
    pub tags: Vec<String>,
    pub language_info: LanguageInfo,
    pub custom: HashMap<String, serde_json::Value>,
}

/// Kernel specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KernelSpec {
    pub name: String,
    pub display_name: String,
    pub language: String,
    pub version: String,
    pub executable: String,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
}

/// Language information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageInfo {
    pub name: String,
    pub version: String,
    pub mimetype: String,
    pub file_extension: String,
    pub pygments_lexer: Option<String>,
    pub codemirror_mode: Option<String>,
}

impl Notebook {
    /// Create a new notebook
    pub fn new(name: String, kernel_spec: KernelSpec) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            cells: Vec::new(),
            metadata: NotebookMetadata {
                title: None,
                description: None,
                authors: Vec::new(),
                tags: Vec::new(),
                language_info: LanguageInfo {
                    name: kernel_spec.language.clone(),
                    version: kernel_spec.version.clone(),
                    mimetype: format!("text/x-{}", kernel_spec.language),
                    file_extension: format!(".{}", kernel_spec.language),
                    pygments_lexer: None,
                    codemirror_mode: None,
                },
                custom: HashMap::new(),
            },
            kernel_spec,
            created_at: now,
            modified_at: now,
            version: "1.0.0".to_string(),
        }
    }

    /// Add a new cell to the notebook
    pub fn add_cell(&mut self, cell: NotebookCell) {
        self.cells.push(cell);
        self.modified_at = Utc::now();
    }

    /// Insert a cell at a specific position
    pub fn insert_cell(&mut self, index: usize, cell: NotebookCell) {
        if index <= self.cells.len() {
            self.cells.insert(index, cell);
            self.modified_at = Utc::now();
        }
    }

    /// Remove a cell by ID
    pub fn remove_cell(&mut self, cell_id: &str) -> Option<NotebookCell> {
        if let Some(pos) = self.cells.iter().position(|c| c.id == cell_id) {
            self.modified_at = Utc::now();
            Some(self.cells.remove(pos))
        } else {
            None
        }
    }

    /// Get a cell by ID
    pub fn get_cell(&self, cell_id: &str) -> Option<&NotebookCell> {
        self.cells.iter().find(|c| c.id == cell_id)
    }

    /// Get a mutable reference to a cell by ID
    pub fn get_cell_mut(&mut self, cell_id: &str) -> Option<&mut NotebookCell> {
        self.cells.iter_mut().find(|c| c.id == cell_id)
    }

    /// Get all code cells
    pub fn get_code_cells(&self) -> Vec<&NotebookCell> {
        self.cells.iter().filter(|c| matches!(c.cell_type, CellType::Code { .. })).collect()
    }

    /// Get execution statistics
    pub fn get_execution_stats(&self) -> ExecutionStats {
        let code_cells = self.get_code_cells();
        let total_cells = code_cells.len();
        let executed_cells = code_cells.iter().filter(|c| c.execution_count.is_some()).count();
        let failed_cells = code_cells.iter().filter(|c| {
            c.outputs.iter().any(|o| matches!(o.output_type, OutputType::Error))
        }).count();

        ExecutionStats {
            total_cells,
            executed_cells,
            failed_cells,
            success_rate: if total_cells > 0 {
                (executed_cells - failed_cells) as f64 / total_cells as f64
            } else {
                0.0
            },
        }
    }
}

/// Execution statistics for a notebook
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionStats {
    pub total_cells: usize,
    pub executed_cells: usize,
    pub failed_cells: usize,
    pub success_rate: f64,
}

/// Notebook session managing runtime state
#[derive(Debug)]
pub struct NotebookSession {
    pub id: String,
    pub notebook_id: String,
    pub kernel: Box<dyn NotebookKernel>,
    pub variables: HashMap<String, Variable>,
    pub execution_count: u32,
    pub started_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
}

impl NotebookSession {
    /// Create a new notebook session
    pub fn new(notebook_id: String, kernel: Box<dyn NotebookKernel>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            notebook_id,
            kernel,
            variables: HashMap::new(),
            execution_count: 0,
            started_at: now,
            last_activity: now,
        }
    }

    /// Update last activity timestamp
    pub fn update_activity(&mut self) {
        self.last_activity = Utc::now();
    }

    /// Increment execution count
    pub fn increment_execution(&mut self) -> u32 {
        self.execution_count += 1;
        self.update_activity();
        self.execution_count
    }
}
