//! # Symbiote Workflow
//!
//! Visual workflow builder and automation engine that combines the power of visual workflow
//! automation with AI intelligence and agent capabilities. Features:
//! - Visual workflow builder with 200+ nodes across 18 categories
//! - AI-powered workflow generation and optimization
//! - Agent integration where every workflow node is an AI agent
//! - Real-time execution with monitoring and analytics
//! - Enterprise connectors for 100+ services

#![deny(missing_docs)]
#![warn(clippy::all)]

// Core modules
pub mod engine;
pub mod builder;
pub mod nodes;
pub mod triggers;
pub mod connectors;
pub mod ai_integration;
pub mod monitoring;
pub mod types;
pub mod ui;
pub mod server;

// Re-export main types
pub use engine::{WorkflowEngine, WorkflowExecutor, WorkflowScheduler};
pub use builder::{VisualEditor, WorkflowCanvas, NodeManager};
pub use nodes::{WorkflowNode, NodeRegistry, NodeInput, NodeOutput};
pub use triggers::{TriggerManager, TriggerType, TriggerEvent};
pub use connectors::{ConnectorManager, ServiceConnector, ConnectorConfig};
pub use ai_integration::{WorkflowAiAssistant, AIWorkflowIntegration, WorkflowGenerator};
pub use monitoring::{WorkflowMonitor, ExecutionMetrics, PerformanceAnalyzer};
pub use types::{Workflow, WorkflowDefinition, ExecutionResult, WorkflowId};
pub use ui::{WorkflowBuilderUI, NodePalette, PropertiesPanel};
pub use server::{WorkflowServer, WebSocketHandler, ApiRouter};

// Core dependencies
use symbiote_core::{SymbioteResult, SymbioteError, Service};
use symbiote_agents::AgentFramework;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Main workflow engine that orchestrates all workflow functionality
pub struct WorkflowSystem {
    /// Core workflow execution engine
    pub engine: Arc<WorkflowEngine>,
    
    /// Visual workflow builder
    pub builder: Arc<VisualEditor>,
    
    /// Node registry and management
    pub node_registry: Arc<NodeRegistry>,
    
    /// Trigger management system
    pub trigger_manager: Arc<TriggerManager>,
    
    /// Service connector management
    pub connector_manager: Arc<ConnectorManager>,
    
    /// AI-powered workflow features
    pub ai_assistant: Arc<WorkflowAiAssistant>,
    
    /// Workflow monitoring and analytics
    pub monitor: Arc<WorkflowMonitor>,
    
    /// Agent framework integration
    pub agent_framework: Arc<AgentFramework>,
    
    /// Configuration
    config: Arc<RwLock<WorkflowConfig>>,
}

/// Configuration for the workflow system
#[derive(Debug, Clone)]
pub struct WorkflowConfig {
    /// Maximum concurrent workflow executions
    pub max_concurrent_executions: usize,
    
    /// Default execution timeout
    pub default_timeout: std::time::Duration,
    
    /// Enable AI-powered features
    pub enable_ai_features: bool,
    
    /// Database connection string
    pub database_url: String,
    
    /// Redis connection string for state management
    pub redis_url: String,
    
    /// Server configuration
    pub server: ServerConfig,
}

/// Server configuration
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Server host
    pub host: String,
    
    /// Server port
    pub port: u16,
    
    /// Enable WebSocket support
    pub enable_websocket: bool,
}

impl Default for WorkflowConfig {
    fn default() -> Self {
        Self {
            max_concurrent_executions: 1000,
            default_timeout: std::time::Duration::from_secs(300), // 5 minutes
            enable_ai_features: true,
            database_url: "postgresql://localhost/symbiote_workflow".to_string(),
            redis_url: "redis://localhost:6379".to_string(),
            server: ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 8081,
                enable_websocket: true,
            },
        }
    }
}

impl WorkflowSystem {
    /// Create a new workflow system instance
    pub async fn new(config: WorkflowConfig, agent_framework: Arc<AgentFramework>) -> SymbioteResult<Self> {
        let config = Arc::new(RwLock::new(config));
        
        // Initialize core components
        let engine = Arc::new(WorkflowEngine::new(config.clone(), agent_framework.clone()).await?);
        let builder = Arc::new(VisualEditor::new(config.clone()).await?);
        let node_registry = Arc::new(NodeRegistry::new(config.clone(), agent_framework.clone()).await?);
        let trigger_manager = Arc::new(TriggerManager::new(config.clone()).await?);
        let connector_manager = Arc::new(ConnectorManager::new(config.clone()).await?);
        let ai_assistant = Arc::new(WorkflowAiAssistant::new(config.clone()).await?);
        let monitor = Arc::new(WorkflowMonitor::new(config.clone()).await?);
        
        Ok(Self {
            engine,
            builder,
            node_registry,
            trigger_manager,
            connector_manager,
            ai_assistant,
            monitor,
            agent_framework,
            config,
        })
    }
    
    /// Start the workflow system
    pub async fn start(&self) -> SymbioteResult<()> {
        tracing::info!("Starting Symbiote Workflow System");
        
        // Initialize all components
        self.engine.initialize().await?;
        self.builder.initialize().await?;
        self.node_registry.initialize().await?;
        self.trigger_manager.initialize().await?;
        self.connector_manager.initialize().await?;
        self.ai_assistant.initialize().await?;
        self.monitor.initialize().await?;
        
        tracing::info!("Symbiote Workflow System started successfully");
        Ok(())
    }
    
    /// Stop the workflow system
    pub async fn stop(&self) -> SymbioteResult<()> {
        tracing::info!("Stopping Symbiote Workflow System");
        
        // Shutdown all components in reverse order
        self.monitor.shutdown().await?;
        self.ai_assistant.shutdown().await?;
        self.connector_manager.shutdown().await?;
        self.trigger_manager.shutdown().await?;
        self.node_registry.shutdown().await?;
        self.builder.shutdown().await?;
        self.engine.shutdown().await?;
        
        tracing::info!("Symbiote Workflow System stopped successfully");
        Ok(())
    }
    
    /// Get the current configuration
    pub async fn get_config(&self) -> WorkflowConfig {
        self.config.read().await.clone()
    }
    
    /// Update the configuration
    pub async fn update_config(&self, new_config: WorkflowConfig) -> SymbioteResult<()> {
        let mut config = self.config.write().await;
        *config = new_config;
        
        // Notify all components of config change
        self.engine.on_config_changed(&*config).await?;
        self.builder.on_config_changed(&*config).await?;
        self.node_registry.on_config_changed(&*config).await?;
        self.trigger_manager.on_config_changed(&*config).await?;
        self.connector_manager.on_config_changed(&*config).await?;
        self.ai_assistant.on_config_changed(&*config).await?;
        self.monitor.on_config_changed(&*config).await?;
        
        Ok(())
    }
}

#[async_trait::async_trait]
impl Service for WorkflowSystem {
    fn name(&self) -> &'static str {
        "workflow_system"
    }
    
    fn dependencies(&self) -> Vec<&'static str> {
        vec!["agent_framework", "ai_provider", "storage"]
    }
    
    async fn initialize(&mut self) -> SymbioteResult<()> {
        self.start().await
    }
    
    async fn shutdown(&mut self) -> SymbioteResult<()> {
        self.stop().await
    }
}

/// Initialize the workflow system with default configuration
pub async fn initialize_workflow_system(agent_framework: Arc<AgentFramework>) -> SymbioteResult<WorkflowSystem> {
    let config = WorkflowConfig::default();
    WorkflowSystem::new(config, agent_framework).await
}

/// Initialize the workflow system with custom configuration
pub async fn initialize_workflow_system_with_config(
    config: WorkflowConfig,
    agent_framework: Arc<AgentFramework>,
) -> SymbioteResult<WorkflowSystem> {
    WorkflowSystem::new(config, agent_framework).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use symbiote_agents::AgentFramework;

    #[tokio::test]
    async fn test_workflow_system_creation() {
        let agent_framework = Arc::new(AgentFramework::new().await.unwrap());
        let config = WorkflowConfig::default();
        let workflow_system = WorkflowSystem::new(config, agent_framework).await;
        assert!(workflow_system.is_ok());
    }

    #[tokio::test]
    async fn test_workflow_system_lifecycle() {
        let agent_framework = Arc::new(AgentFramework::new().await.unwrap());
        let config = WorkflowConfig::default();
        let workflow_system = WorkflowSystem::new(config, agent_framework).await.unwrap();
        
        let start_result = workflow_system.start().await;
        assert!(start_result.is_ok());
        
        let stop_result = workflow_system.stop().await;
        assert!(stop_result.is_ok());
    }
}
