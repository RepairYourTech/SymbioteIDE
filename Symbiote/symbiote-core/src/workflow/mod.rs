//! Visual Workflow Builder System
//! 
//! This module implements a comprehensive visual workflow builder that rivals n8n
//! but adds advanced AI agent capabilities. Features include:
//! - 200+ nodes across 18 categories
//! - Visual drag-and-drop interface
//! - AI agent integration
//! - Real-time execution monitoring
//! - Advanced control flow and data processing

pub mod engine;
pub mod nodes;
pub mod editor;
pub mod runtime;
pub mod integrations;
pub mod ai_agents;
pub mod state;
pub mod templates;
pub mod context_integration;
pub mod executor;
pub mod node_executor;

#[cfg(test)]
pub mod integration_test;

#[cfg(test)]
pub mod end_to_end_test;

pub use engine::*;
pub use nodes::*;
pub use editor::*;
pub use runtime::*;
pub use integrations::*;
pub use ai_agents::*;
pub use state::*;
pub use templates::*;
pub use context_integration::*;
pub use executor::*;
pub use node_executor::*;

use crate::{Result, SymbioteError, UserId, ProjectId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::broadcast;
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Visual Workflow Builder - Main orchestrator
#[derive(Debug)]
pub struct VisualWorkflowBuilder {
    /// Workflow engine for execution
    engine: Arc<WorkflowEngine>,
    
    /// Node registry with 200+ nodes
    node_registry: Arc<NodeRegistry>,
    
    /// Visual editor interface
    editor: Arc<VisualEditor>,
    
    /// Runtime execution manager
    runtime: Arc<WorkflowRuntime>,

    /// Context-aware workflow executor
    executor: Arc<WorkflowExecutor>,
    
    /// AI agent integration
    ai_integration: Arc<AIAgentIntegration>,
    
    /// State management
    state_manager: Arc<WorkflowStateManager>,
    
    /// Integration layer
    integrations: Arc<IntegrationLayer>,
    
    /// Template library
    templates: Arc<WorkflowTemplateLibrary>,
    
    /// Event broadcaster
    event_broadcaster: broadcast::Sender<WorkflowEvent>,
}

/// Workflow definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub version: String,
    pub created_by: UserId,
    pub project_id: Option<ProjectId>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub nodes: Vec<WorkflowNode>,
    pub connections: Vec<WorkflowConnection>,
    pub settings: WorkflowSettings,
    pub metadata: HashMap<String, serde_json::Value>,
    pub tags: Vec<String>,
    pub status: WorkflowStatus,
}

/// Workflow node definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowNode {
    pub id: String,
    pub node_type: String,
    pub name: String,
    pub description: Option<String>,
    pub category: NodeCategory,
    pub position: NodePosition,
    pub configuration: NodeConfiguration,
    pub inputs: Vec<NodeInput>,
    pub outputs: Vec<NodeOutput>,
    pub metadata: HashMap<String, serde_json::Value>,
    pub enabled: bool,
}

/// Node categories (18 total)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum NodeCategory {
    /// AI and Machine Learning nodes
    AI,
    /// Communication and messaging
    Communication,
    /// Development and DevOps
    Development,
    /// Data storage and databases
    DataStorage,
    /// Productivity and office tools
    Productivity,
    /// Marketing and analytics
    Marketing,
    /// Sales and CRM
    Sales,
    /// Finance and accounting
    Finance,
    /// E-commerce and retail
    ECommerce,
    /// Cybersecurity and monitoring
    Cybersecurity,
    /// System monitoring and observability
    Monitoring,
    /// Cloud services and infrastructure
    Cloud,
    /// Utility functions and helpers
    Utilities,
    /// Control flow and logic
    ControlFlow,
    /// Data processing and transformation
    Processing,
    /// Triggers and event sources
    Triggers,
    /// Outputs and destinations
    Outputs,
    /// Custom and user-defined nodes
    Custom,
}

/// Node position in the visual editor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodePosition {
    pub x: f64,
    pub y: f64,
    pub z_index: Option<i32>,
}

/// Node configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfiguration {
    pub parameters: HashMap<String, serde_json::Value>,
    pub credentials: Option<String>,
    pub retry_policy: Option<RetryPolicy>,
    pub timeout_ms: Option<u64>,
    pub cache_settings: Option<CacheSettings>,
}

/// Node input definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInput {
    pub name: String,
    pub data_type: DataType,
    pub required: bool,
    pub default_value: Option<serde_json::Value>,
    pub description: Option<String>,
    pub validation: Option<InputValidation>,
}

/// Node output definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeOutput {
    pub name: String,
    pub data_type: DataType,
    pub description: Option<String>,
    pub schema: Option<serde_json::Value>,
}

/// Data types supported in workflows
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DataType {
    String,
    Number,
    Boolean,
    Array,
    Object,
    Binary,
    DateTime,
    Any,
    Custom(String),
}

/// Input validation rules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputValidation {
    pub min_length: Option<usize>,
    pub max_length: Option<usize>,
    pub pattern: Option<String>,
    pub allowed_values: Option<Vec<serde_json::Value>>,
    pub custom_validator: Option<String>,
}

/// Workflow connection between nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowConnection {
    pub id: String,
    pub source_node_id: String,
    pub source_output: String,
    pub target_node_id: String,
    pub target_input: String,
    pub connection_type: ConnectionType,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Connection types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConnectionType {
    /// Standard data flow
    Data,
    /// Control flow (execution order)
    Control,
    /// Error handling
    Error,
    /// Conditional flow
    Conditional,
}

/// Workflow settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowSettings {
    pub execution_timeout_ms: u64,
    pub max_concurrent_executions: u32,
    pub retry_policy: RetryPolicy,
    pub error_handling: ErrorHandlingStrategy,
    pub logging_level: LoggingLevel,
    pub environment_variables: HashMap<String, String>,
    pub resource_limits: ResourceLimits,
}

/// Retry policy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub initial_delay_ms: u64,
    pub max_delay_ms: u64,
    pub backoff_multiplier: f64,
    pub retry_on_errors: Vec<String>,
}

/// Cache settings for nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheSettings {
    pub enabled: bool,
    pub ttl_seconds: u64,
    pub cache_key_template: Option<String>,
    pub invalidation_rules: Vec<String>,
}

/// Error handling strategies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ErrorHandlingStrategy {
    /// Stop execution on first error
    StopOnError,
    /// Continue execution, collect errors
    ContinueOnError,
    /// Retry failed nodes
    RetryOnError,
    /// Custom error handling
    Custom(String),
}

/// Logging levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LoggingLevel {
    Debug,
    Info,
    Warning,
    Error,
    None,
}

/// Resource limits for workflow execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    pub max_memory_mb: u64,
    pub max_cpu_percent: f64,
    pub max_execution_time_ms: u64,
    pub max_file_size_mb: u64,
    pub max_network_requests: u32,
}

/// Workflow status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkflowStatus {
    Draft,
    Active,
    Paused,
    Archived,
    Error(String),
}

/// Workflow events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkflowEvent {
    /// Workflow created
    WorkflowCreated {
        workflow_id: String,
        user_id: UserId,
        timestamp: DateTime<Utc>,
    },
    /// Workflow updated
    WorkflowUpdated {
        workflow_id: String,
        user_id: UserId,
        changes: Vec<String>,
        timestamp: DateTime<Utc>,
    },
    /// Workflow execution started
    ExecutionStarted {
        workflow_id: String,
        execution_id: String,
        trigger: ExecutionTrigger,
        timestamp: DateTime<Utc>,
    },
    /// Workflow execution completed
    ExecutionCompleted {
        workflow_id: String,
        execution_id: String,
        status: ExecutionStatus,
        duration_ms: u64,
        timestamp: DateTime<Utc>,
    },
    /// Node execution event
    NodeExecuted {
        workflow_id: String,
        execution_id: String,
        node_id: String,
        status: NodeExecutionStatus,
        duration_ms: u64,
        timestamp: DateTime<Utc>,
    },
    /// Error occurred
    ErrorOccurred {
        workflow_id: String,
        execution_id: Option<String>,
        node_id: Option<String>,
        error: String,
        timestamp: DateTime<Utc>,
    },
}

/// Execution trigger types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionTrigger {
    Manual,
    Schedule,
    Webhook,
    Event,
    API,
    AIAgent,
}

/// Execution status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionStatus {
    Running,
    Completed,
    Failed,
    Cancelled,
    Timeout,
}

/// Node execution status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NodeExecutionStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Skipped,
    Cancelled,
}

impl VisualWorkflowBuilder {
    /// Create a new visual workflow builder
    pub fn new(context_bus: Arc<crate::context::ContextBus>) -> Self {
        let (event_broadcaster, _) = broadcast::channel(10000);
        let node_registry = Arc::new(NodeRegistry::new());
        let executor = Arc::new(WorkflowExecutor::new(
            Arc::clone(&node_registry),
            context_bus,
        ));

        Self {
            engine: Arc::new(WorkflowEngine::new()),
            node_registry,
            editor: Arc::new(VisualEditor::new()),
            runtime: Arc::new(WorkflowRuntime::new()),
            ai_integration: Arc::new(AIAgentIntegration::new()),
            state_manager: Arc::new(WorkflowStateManager::new()),
            integrations: Arc::new(IntegrationLayer::new()),
            templates: Arc::new(WorkflowTemplateLibrary::new()),
            executor,
            event_broadcaster,
        }
    }

    /// Create a new workflow
    pub async fn create_workflow(
        &self,
        name: String,
        description: Option<String>,
        user_id: UserId,
        project_id: Option<ProjectId>,
    ) -> Result<Workflow> {
        let workflow = Workflow {
            id: Uuid::new_v4().to_string(),
            name: name.clone(),
            description,
            version: "1.0.0".to_string(),
            created_by: user_id.clone(),
            project_id: project_id.clone(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            nodes: Vec::new(),
            connections: Vec::new(),
            settings: WorkflowSettings::default(),
            metadata: HashMap::new(),
            tags: Vec::new(),
            status: WorkflowStatus::Draft,
        };

        // Broadcast creation event
        let _ = self.event_broadcaster.send(WorkflowEvent::WorkflowCreated {
            workflow_id: workflow.id.clone(),
            user_id,
            timestamp: Utc::now(),
        });

        Ok(workflow)
    }

    /// Get available node categories
    pub fn get_node_categories(&self) -> Vec<NodeCategory> {
        vec![
            NodeCategory::AI,
            NodeCategory::Communication,
            NodeCategory::Development,
            NodeCategory::DataStorage,
            NodeCategory::Productivity,
            NodeCategory::Marketing,
            NodeCategory::Sales,
            NodeCategory::Finance,
            NodeCategory::ECommerce,
            NodeCategory::Cybersecurity,
            NodeCategory::Monitoring,
            NodeCategory::Cloud,
            NodeCategory::Utilities,
            NodeCategory::ControlFlow,
            NodeCategory::Processing,
            NodeCategory::Triggers,
            NodeCategory::Outputs,
            NodeCategory::Custom,
        ]
    }

    /// Subscribe to workflow events
    pub fn subscribe_to_events(&self) -> broadcast::Receiver<WorkflowEvent> {
        self.event_broadcaster.subscribe()
    }

    /// Execute a workflow
    pub async fn execute_workflow(
        &self,
        workflow_id: &str,
        trigger: ExecutionTrigger,
        input_data: Option<HashMap<String, serde_json::Value>>,
    ) -> Result<String> {
        // For now, delegate to runtime - in future this would load the workflow
        // and use the new executor
        self.runtime.execute_workflow(workflow_id, trigger, input_data).await
    }

    /// Execute a workflow with the new context-aware executor
    pub async fn execute_workflow_with_context(
        &self,
        workflow: Workflow,
        trigger: ExecutionTrigger,
        input_data: Option<HashMap<String, serde_json::Value>>,
    ) -> Result<String> {
        self.executor.execute_workflow(workflow, trigger, input_data).await
    }

    /// Get workflow execution status
    pub async fn get_execution_status(&self, execution_id: &str) -> Result<ExecutionStatus> {
        self.runtime.get_execution_status(execution_id).await
    }
}

impl Default for WorkflowSettings {
    fn default() -> Self {
        Self {
            execution_timeout_ms: 300000, // 5 minutes
            max_concurrent_executions: 10,
            retry_policy: RetryPolicy {
                max_attempts: 3,
                initial_delay_ms: 1000,
                max_delay_ms: 30000,
                backoff_multiplier: 2.0,
                retry_on_errors: vec!["timeout".to_string(), "network".to_string()],
            },
            error_handling: ErrorHandlingStrategy::StopOnError,
            logging_level: LoggingLevel::Info,
            environment_variables: HashMap::new(),
            resource_limits: ResourceLimits {
                max_memory_mb: 1024,
                max_cpu_percent: 80.0,
                max_execution_time_ms: 300000,
                max_file_size_mb: 100,
                max_network_requests: 1000,
            },
        }
    }
}
