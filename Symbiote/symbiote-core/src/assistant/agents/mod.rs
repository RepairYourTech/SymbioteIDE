//! # Specialized Agents
//! 
//! Collection of specialized agents that SYMBIOTE can orchestrate to accomplish tasks.

pub mod registry;
pub mod web_research;
pub mod email;
pub mod workflow;
pub mod notebook;
pub mod ide;
pub mod crypto_trading;
pub mod file_system;
pub mod strategy;
pub mod web_access;

// Re-export agent implementations
pub use registry::*;
pub use web_research::*;
pub use email::*;
pub use workflow::*;
pub use notebook::*;
pub use ide::*;
pub use crypto_trading::*;
pub use file_system::*;
pub use strategy::*;
pub use web_access::*;

use super::*;
use async_trait::async_trait;

/// Base trait for all specialized agents
#[async_trait]
pub trait SpecializedAgent: Send + Sync + std::fmt::Debug {
    /// Get agent type identifier
    fn agent_type(&self) -> &str;
    
    /// Get agent display name
    fn display_name(&self) -> &str;
    
    /// Get agent description
    fn description(&self) -> &str;
    
    /// Get agent capabilities
    fn capabilities(&self) -> Vec<String>;
    
    /// Check if agent can handle a specific task
    async fn can_handle_task(&self, task: &Task) -> bool;
    
    /// Execute a task
    async fn execute_task(&self, task: &Task) -> Result<AgentResult>;
    
    /// Get current agent status
    async fn get_status(&self) -> AgentStatus;
    
    /// Get agent load (0.0 to 1.0)
    async fn get_load(&self) -> f64;
    
    /// Cancel current task
    async fn cancel_task(&self, task_id: &str) -> Result<()>;
    
    /// Get agent configuration
    fn get_config(&self) -> AgentConfig;
    
    /// Update agent configuration
    async fn update_config(&self, config: AgentConfig) -> Result<()>;
}

/// Agent configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub enabled: bool,
    pub max_concurrent_tasks: usize,
    pub timeout_seconds: u64,
    pub retry_attempts: u32,
    pub priority: u32,
    pub custom_settings: HashMap<String, serde_json::Value>,
}

/// Agent capability definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCapability {
    pub name: String,
    pub description: String,
    pub parameters: Vec<CapabilityParameter>,
    pub examples: Vec<String>,
}

/// Parameter for agent capability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityParameter {
    pub name: String,
    pub parameter_type: ParameterType,
    pub required: bool,
    pub description: String,
    pub default_value: Option<serde_json::Value>,
}

/// Parameter types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ParameterType {
    String,
    Number,
    Boolean,
    Array,
    Object,
    File,
    Url,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_concurrent_tasks: 3,
            timeout_seconds: 300,
            retry_attempts: 3,
            priority: 1,
            custom_settings: HashMap::new(),
        }
    }
}

/// Create all default specialized agents
pub fn create_default_agents() -> Vec<Box<dyn SpecializedAgent>> {
    vec![
        Box::new(WebResearchAgent::new()),
        Box::new(EmailAgent::new()),
        Box::new(WorkflowAgent::new()),
        Box::new(NotebookAgent::new()),
        Box::new(IDEAgent::new()),
        Box::new(CryptoTradingAgent::new()),
        Box::new(FileSystemAgent::new()),
        Box::new(StrategyAgent::new()),
        Box::new(WebAccessAgent::new()),
    ]
}

/// Get agent by type
pub fn get_agent_by_type(agent_type: &str) -> Option<Box<dyn SpecializedAgent>> {
    match agent_type {
        "web_research" => Some(Box::new(WebResearchAgent::new())),
        "email" => Some(Box::new(EmailAgent::new())),
        "workflow" => Some(Box::new(WorkflowAgent::new())),
        "notebook" => Some(Box::new(NotebookAgent::new())),
        "ide" => Some(Box::new(IDEAgent::new())),
        "crypto_trading" => Some(Box::new(CryptoTradingAgent::new())),
        "file_system" => Some(Box::new(FileSystemAgent::new())),
        "strategy" => Some(Box::new(StrategyAgent::new())),
        "web_access" => Some(Box::new(WebAccessAgent::new())),
        _ => None,
    }
}

/// Check if agent type is supported
pub fn is_agent_type_supported(agent_type: &str) -> bool {
    get_agent_by_type(agent_type).is_some()
}

/// Get all supported agent types
pub fn get_supported_agent_types() -> Vec<String> {
    vec![
        "web_research".to_string(),
        "email".to_string(),
        "workflow".to_string(),
        "notebook".to_string(),
        "ide".to_string(),
        "crypto_trading".to_string(),
        "file_system".to_string(),
        "strategy".to_string(),
        "web_access".to_string(),
    ]
}
