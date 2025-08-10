//! # Notebook Kernel System
//! 
//! Abstract kernel interface and base implementations for multi-language support.

use super::*;
use async_trait::async_trait;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};

/// Abstract interface for notebook kernels
#[async_trait]
pub trait NotebookKernel: Send + Sync + std::fmt::Debug {
    /// Get kernel language
    fn language(&self) -> &str;
    
    /// Get kernel version
    fn version(&self) -> &str;
    
    /// Get kernel display name
    fn display_name(&self) -> &str;
    
    /// Execute code and return result
    async fn execute_code(&self, code: &str, context: &ExecutionContext) -> Result<ExecutionResult>;
    
    /// Get current variables in kernel
    async fn get_variables(&self) -> Result<HashMap<String, Variable>>;
    
    /// Set a variable in kernel
    async fn set_variable(&self, name: &str, value: &Variable) -> Result<()>;
    
    /// Check if kernel is alive
    async fn is_alive(&self) -> bool;
    
    /// Restart kernel
    async fn restart(&self) -> Result<()>;
    
    /// Shutdown kernel
    async fn shutdown(&self) -> Result<()>;
    
    /// Get kernel info
    fn get_info(&self) -> KernelInfo;
    
    /// Complete code (for autocomplete)
    async fn complete_code(&self, code: &str, cursor_pos: usize) -> Result<CompletionResult>;
    
    /// Inspect code (for help/documentation)
    async fn inspect_code(&self, code: &str, cursor_pos: usize) -> Result<InspectionResult>;
    
    /// Get kernel capabilities
    fn get_capabilities(&self) -> KernelCapabilities;
}

/// Execution context for kernel operations
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    pub execution_count: u32,
    pub cell_id: String,
    pub notebook_id: String,
    pub user_id: Option<String>,
    pub working_directory: Option<String>,
    pub environment_variables: HashMap<String, String>,
    pub timeout_seconds: Option<u64>,
    pub allow_stdin: bool,
    pub store_history: bool,
}

/// Result of code execution
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    pub status: ExecutionStatus,
    pub outputs: Vec<CellOutput>,
    pub execution_count: u32,
    pub execution_time_ms: u64,
    pub memory_usage_bytes: Option<u64>,
    pub variables_changed: Vec<String>,
}

/// Status of code execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionStatus {
    Success,
    Error { error_type: String, message: String },
    Timeout,
    Interrupted,
    Aborted,
}

/// Variable representation for cross-kernel communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Variable {
    pub name: String,
    pub value: VariableValue,
    pub var_type: String,
    pub size_bytes: Option<u64>,
    pub shape: Option<Vec<usize>>,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Variable value types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VariableValue {
    None,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Bytes(Vec<u8>),
    List(Vec<VariableValue>),
    Dict(HashMap<String, VariableValue>),
    Object { 
        class_name: String,
        serialized: Vec<u8>,
        format: String,
    },
}

impl std::fmt::Display for VariableValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VariableValue::None => write!(f, "None"),
            VariableValue::Bool(b) => write!(f, "{}", b),
            VariableValue::Int(i) => write!(f, "{}", i),
            VariableValue::Float(fl) => write!(f, "{}", fl),
            VariableValue::String(s) => write!(f, "{}", s),
            VariableValue::Bytes(b) => write!(f, "bytes({})", b.len()),
            VariableValue::List(l) => write!(f, "list({})", l.len()),
            VariableValue::Dict(d) => write!(f, "dict({})", d.len()),
            VariableValue::Object { class_name, .. } => write!(f, "<{} object>", class_name),
        }
    }
}

/// Kernel information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KernelInfo {
    pub protocol_version: String,
    pub implementation: String,
    pub implementation_version: String,
    pub language_info: LanguageInfo,
    pub banner: String,
    pub help_links: Vec<HelpLink>,
    pub status: KernelStatus,
}

/// Help link for kernel documentation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelpLink {
    pub text: String,
    pub url: String,
}

/// Kernel status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KernelStatus {
    Starting,
    Idle,
    Busy,
    Terminating,
    Restarting,
    Dead,
}

/// Code completion result
#[derive(Debug, Clone)]
pub struct CompletionResult {
    pub matches: Vec<String>,
    pub cursor_start: usize,
    pub cursor_end: usize,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Code inspection result
#[derive(Debug, Clone)]
pub struct InspectionResult {
    pub found: bool,
    pub data: OutputData,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Kernel capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KernelCapabilities {
    pub supports_completion: bool,
    pub supports_inspection: bool,
    pub supports_rich_output: bool,
    pub supports_stdin: bool,
    pub supports_debugging: bool,
    pub supports_variable_inspection: bool,
    pub supports_plotting: bool,
    pub supports_widgets: bool,
    pub max_execution_time: Option<u64>,
    pub max_memory_usage: Option<u64>,
}

impl ExecutionContext {
    /// Create new execution context
    pub fn new(execution_count: u32, cell_id: String, notebook_id: String) -> Self {
        Self {
            execution_count,
            cell_id,
            notebook_id,
            user_id: None,
            working_directory: None,
            environment_variables: HashMap::new(),
            timeout_seconds: Some(300), // 5 minutes default
            allow_stdin: false,
            store_history: true,
        }
    }

    /// Set working directory
    pub fn with_working_directory(mut self, dir: String) -> Self {
        self.working_directory = Some(dir);
        self
    }

    /// Set timeout
    pub fn with_timeout(mut self, seconds: u64) -> Self {
        self.timeout_seconds = Some(seconds);
        self
    }

    /// Add environment variable
    pub fn with_env_var(mut self, key: String, value: String) -> Self {
        self.environment_variables.insert(key, value);
        self
    }
}

impl Variable {
    /// Create a new variable
    pub fn new(name: String, value: VariableValue, var_type: String) -> Self {
        Self {
            name,
            value,
            var_type,
            size_bytes: None,
            shape: None,
            metadata: HashMap::new(),
        }
    }

    /// Create string variable
    pub fn string(name: String, value: String) -> Self {
        Self::new(name, VariableValue::String(value), "str".to_string())
    }

    /// Create integer variable
    pub fn int(name: String, value: i64) -> Self {
        Self::new(name, VariableValue::Int(value), "int".to_string())
    }

    /// Create float variable
    pub fn float(name: String, value: f64) -> Self {
        Self::new(name, VariableValue::Float(value), "float".to_string())
    }

    /// Create boolean variable
    pub fn bool(name: String, value: bool) -> Self {
        Self::new(name, VariableValue::Bool(value), "bool".to_string())
    }

    /// Get variable as string representation
    pub fn to_string_repr(&self) -> String {
        match &self.value {
            VariableValue::None => "None".to_string(),
            VariableValue::Bool(b) => b.to_string(),
            VariableValue::Int(i) => i.to_string(),
            VariableValue::Float(f) => f.to_string(),
            VariableValue::String(s) => format!("\"{}\"", s),
            VariableValue::Bytes(b) => format!("bytes({})", b.len()),
            VariableValue::List(l) => format!("list({})", l.len()),
            VariableValue::Dict(d) => format!("dict({})", d.len()),
            VariableValue::Object { class_name, .. } => format!("<{} object>", class_name),
        }
    }

    /// Check if variable is serializable
    pub fn is_serializable(&self) -> bool {
        match &self.value {
            VariableValue::Object { .. } => false,
            VariableValue::List(l) => l.iter().all(|v| Variable::is_value_serializable(v)),
            VariableValue::Dict(d) => d.values().all(|v| Variable::is_value_serializable(v)),
            _ => true,
        }
    }

    fn is_value_serializable(value: &VariableValue) -> bool {
        match value {
            VariableValue::Object { .. } => false,
            VariableValue::List(l) => l.iter().all(Variable::is_value_serializable),
            VariableValue::Dict(d) => d.values().all(Variable::is_value_serializable),
            _ => true,
        }
    }
}

impl Default for KernelCapabilities {
    fn default() -> Self {
        Self {
            supports_completion: true,
            supports_inspection: true,
            supports_rich_output: true,
            supports_stdin: false,
            supports_debugging: false,
            supports_variable_inspection: true,
            supports_plotting: true,
            supports_widgets: false,
            max_execution_time: Some(300),
            max_memory_usage: None,
        }
    }
}

impl ExecutionResult {
    /// Create successful execution result
    pub fn success(outputs: Vec<CellOutput>, execution_count: u32, execution_time_ms: u64) -> Self {
        Self {
            status: ExecutionStatus::Success,
            outputs,
            execution_count,
            execution_time_ms,
            memory_usage_bytes: None,
            variables_changed: Vec::new(),
        }
    }

    /// Create error execution result
    pub fn error(error_type: String, message: String, execution_count: u32) -> Self {
        Self {
            status: ExecutionStatus::Error { error_type, message },
            outputs: Vec::new(),
            execution_count,
            execution_time_ms: 0,
            memory_usage_bytes: None,
            variables_changed: Vec::new(),
        }
    }

    /// Check if execution was successful
    pub fn is_success(&self) -> bool {
        matches!(self.status, ExecutionStatus::Success)
    }

    /// Get error message if execution failed
    pub fn get_error_message(&self) -> Option<String> {
        match &self.status {
            ExecutionStatus::Error { message, .. } => Some(message.clone()),
            _ => None,
        }
    }
}
