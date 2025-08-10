//! # Notebook Cell System
//! 
//! Cell types, management, and operations for the notebook system.

use super::*;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Notebook cell containing code, markdown, or raw content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotebookCell {
    pub id: String,
    pub cell_type: CellType,
    pub content: String,
    pub metadata: CellMetadata,
    pub outputs: Vec<CellOutput>,
    pub execution_count: Option<u32>,
    pub last_executed: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub modified_at: DateTime<Utc>,
}

/// Types of notebook cells
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CellType {
    /// Code cell with specified language
    Code { 
        language: String,
        kernel: Option<String>,
    },
    /// Markdown cell for documentation
    Markdown,
    /// Raw cell for unprocessed content
    Raw { 
        format: Option<String>,
    },
}

/// Cell metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CellMetadata {
    pub name: Option<String>,
    pub tags: Vec<String>,
    pub collapsed: bool,
    pub scrolled: bool,
    pub deletable: bool,
    pub editable: bool,
    pub format: Option<String>,
    pub custom: HashMap<String, serde_json::Value>,
}

/// Cell output from execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CellOutput {
    pub output_type: OutputType,
    pub data: OutputData,
    pub metadata: HashMap<String, serde_json::Value>,
    pub execution_count: Option<u32>,
    pub timestamp: DateTime<Utc>,
}

/// Types of cell outputs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OutputType {
    /// Stream output (stdout/stderr)
    Stream { name: StreamType },
    /// Display data (rich outputs)
    DisplayData,
    /// Execute result (return values)
    ExecuteResult,
    /// Error output
    Error,
    /// Update display data
    UpdateDisplayData { display_id: String },
}

/// Stream types for output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StreamType {
    Stdout,
    Stderr,
}

/// Output data with multiple MIME types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputData {
    pub mime_bundle: HashMap<String, serde_json::Value>,
}

impl NotebookCell {
    /// Create a new code cell
    pub fn new_code(language: String, content: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            cell_type: CellType::Code { language, kernel: None },
            content,
            metadata: CellMetadata::default(),
            outputs: Vec::new(),
            execution_count: None,
            last_executed: None,
            created_at: now,
            modified_at: now,
        }
    }

    /// Create a new markdown cell
    pub fn new_markdown(content: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            cell_type: CellType::Markdown,
            content,
            metadata: CellMetadata::default(),
            outputs: Vec::new(),
            execution_count: None,
            last_executed: None,
            created_at: now,
            modified_at: now,
        }
    }

    /// Create a new raw cell
    pub fn new_raw(content: String, format: Option<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            cell_type: CellType::Raw { format },
            content,
            metadata: CellMetadata::default(),
            outputs: Vec::new(),
            execution_count: None,
            last_executed: None,
            created_at: now,
            modified_at: now,
        }
    }

    /// Update cell content
    pub fn update_content(&mut self, content: String) {
        self.content = content;
        self.modified_at = Utc::now();
    }

    /// Add output to cell
    pub fn add_output(&mut self, output: CellOutput) {
        self.outputs.push(output);
        self.modified_at = Utc::now();
    }

    /// Clear all outputs
    pub fn clear_outputs(&mut self) {
        self.outputs.clear();
        self.modified_at = Utc::now();
    }

    /// Set execution count and timestamp
    pub fn set_executed(&mut self, execution_count: u32) {
        self.execution_count = Some(execution_count);
        self.last_executed = Some(Utc::now());
        self.modified_at = Utc::now();
    }

    /// Check if cell is executable
    pub fn is_executable(&self) -> bool {
        matches!(self.cell_type, CellType::Code { .. })
    }

    /// Get cell language if it's a code cell
    pub fn get_language(&self) -> Option<&str> {
        match &self.cell_type {
            CellType::Code { language, .. } => Some(language),
            _ => None,
        }
    }

    /// Check if cell has errors
    pub fn has_errors(&self) -> bool {
        self.outputs.iter().any(|o| matches!(o.output_type, OutputType::Error))
    }

    /// Get error outputs
    pub fn get_errors(&self) -> Vec<&CellOutput> {
        self.outputs.iter().filter(|o| matches!(o.output_type, OutputType::Error)).collect()
    }

    /// Get display outputs
    pub fn get_display_outputs(&self) -> Vec<&CellOutput> {
        self.outputs.iter().filter(|o| {
            matches!(o.output_type, OutputType::DisplayData | OutputType::ExecuteResult)
        }).collect()
    }
}

impl Default for CellMetadata {
    fn default() -> Self {
        Self {
            name: None,
            tags: Vec::new(),
            collapsed: false,
            scrolled: false,
            deletable: true,
            editable: true,
            format: None,
            custom: HashMap::new(),
        }
    }
}

impl OutputData {
    /// Create new output data
    pub fn new() -> Self {
        Self {
            mime_bundle: HashMap::new(),
        }
    }

    /// Add text output
    pub fn add_text(&mut self, text: String) {
        self.mime_bundle.insert("text/plain".to_string(), serde_json::Value::String(text));
    }

    /// Add HTML output
    pub fn add_html(&mut self, html: String) {
        self.mime_bundle.insert("text/html".to_string(), serde_json::Value::String(html));
    }

    /// Add JSON output
    pub fn add_json(&mut self, json: serde_json::Value) {
        self.mime_bundle.insert("application/json".to_string(), json);
    }

    /// Add image output
    pub fn add_image(&mut self, format: &str, data: String) {
        let mime_type = match format.to_lowercase().as_str() {
            "png" => "image/png",
            "jpg" | "jpeg" => "image/jpeg",
            "svg" => "image/svg+xml",
            "gif" => "image/gif",
            _ => "application/octet-stream",
        };
        self.mime_bundle.insert(mime_type.to_string(), serde_json::Value::String(data));
    }

    /// Get text representation
    pub fn get_text(&self) -> Option<&str> {
        self.mime_bundle.get("text/plain")?.as_str()
    }

    /// Get HTML representation
    pub fn get_html(&self) -> Option<&str> {
        self.mime_bundle.get("text/html")?.as_str()
    }

    /// Check if has specific MIME type
    pub fn has_mime_type(&self, mime_type: &str) -> bool {
        self.mime_bundle.contains_key(mime_type)
    }

    /// Get all MIME types
    pub fn get_mime_types(&self) -> Vec<&String> {
        self.mime_bundle.keys().collect()
    }
}

impl CellOutput {
    /// Create stream output
    pub fn stream(name: StreamType, text: String) -> Self {
        let mut data = OutputData::new();
        data.add_text(text);
        
        Self {
            output_type: OutputType::Stream { name },
            data,
            metadata: HashMap::new(),
            execution_count: None,
            timestamp: Utc::now(),
        }
    }

    /// Create display data output
    pub fn display_data(data: OutputData) -> Self {
        Self {
            output_type: OutputType::DisplayData,
            data,
            metadata: HashMap::new(),
            execution_count: None,
            timestamp: Utc::now(),
        }
    }

    /// Create execute result output
    pub fn execute_result(data: OutputData, execution_count: u32) -> Self {
        Self {
            output_type: OutputType::ExecuteResult,
            data,
            metadata: HashMap::new(),
            execution_count: Some(execution_count),
            timestamp: Utc::now(),
        }
    }

    /// Create error output
    pub fn error(error_name: String, error_value: String, traceback: Vec<String>) -> Self {
        let mut data = OutputData::new();
        data.add_text(format!("{}: {}", error_name, error_value));
        
        let mut error_data = HashMap::new();
        error_data.insert("ename".to_string(), serde_json::Value::String(error_name));
        error_data.insert("evalue".to_string(), serde_json::Value::String(error_value));
        error_data.insert("traceback".to_string(), serde_json::Value::Array(
            traceback.into_iter().map(serde_json::Value::String).collect()
        ));
        data.mime_bundle.insert("application/vnd.jupyter.error".to_string(), 
                               serde_json::Value::Object(error_data.into_iter().collect()));
        
        Self {
            output_type: OutputType::Error,
            data,
            metadata: HashMap::new(),
            execution_count: None,
            timestamp: Utc::now(),
        }
    }
}
