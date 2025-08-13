//! # Symbiote Tools
//!
//! Comprehensive typed tool SDK for creating, managing, and executing tools within Symbiote. Features:
//! - Type-safe tool definitions with compile-time validation
//! - Automatic JSON Schema generation from Rust types
//! - Sandboxed tool execution with resource limits and permission controls
//! - Tool discovery with dynamic registration and versioning
//! - Validation framework with comprehensive input/output validation
//! - Performance monitoring with tool execution metrics and optimization
//! - Tool composition with advanced chaining and optimization capabilities

#![deny(missing_docs)]
#![warn(clippy::all)]

// Core modules
pub mod definition;
pub mod execution;
pub mod registry;
pub mod validation;
pub mod composition;
pub mod monitoring;
pub mod security;
pub mod macros;
pub mod types;

// Re-export main types
pub use definition::{Tool, ToolDefinition, ToolMetadata, ToolSchema, DynamicTool};
pub use execution::{ToolExecutor, ToolContext, ExecutionResult, SandboxedExecutor};
pub use registry::{ToolRegistry, ToolInfo, ToolDiscovery, VersionManager};
pub use validation::{InputValidator, OutputValidator, SchemaValidator, ValidationResult};
pub use composition::{ToolComposer, ToolChain, ChainBuilder, ChainOptimizer};
pub use monitoring::{ToolMonitor, ExecutionMetrics, PerformanceProfiler, UsageTracker};
pub use security::{PermissionManager, SecurityPolicy, ResourceLimits, SandboxConfig};
pub use macros::{tool, Tool};
pub use types::{ToolConfig, ToolResult, ToolError, ToolId, ExecutionId};

// Core dependencies
use symbiote_core::{SymbioteResult, SymbioteError, Service};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Main tool system that orchestrates all tool operations
pub struct ToolSystem {
    /// Tool registry for discovery and management
    pub registry: Arc<ToolRegistry>,
    
    /// Tool executor for sandboxed execution
    pub executor: Arc<ToolExecutor>,
    
    /// Validation system
    pub validator: Arc<SchemaValidator>,
    
    /// Tool composition system
    pub composer: Arc<ToolComposer>,
    
    /// Performance monitoring
    pub monitor: Arc<ToolMonitor>,
    
    /// Security and permission management
    pub security: Arc<PermissionManager>,
    
    /// Configuration
    config: Arc<RwLock<ToolConfig>>,
}

/// Configuration for the tool system
#[derive(Debug, Clone)]
pub struct ToolConfig {
    /// Enable tool system
    pub enabled: bool,
    
    /// Enable sandboxed execution
    pub enable_sandbox: bool,
    
    /// Enable performance monitoring
    pub enable_monitoring: bool,
    
    /// Database connection string
    pub database_url: String,
    
    /// Security configuration
    pub security: SecurityConfig,
    
    /// Execution configuration
    pub execution: ExecutionConfig,
    
    /// Registry configuration
    pub registry: RegistryConfig,
}

/// Security configuration
#[derive(Debug, Clone)]
pub struct SecurityConfig {
    /// Enable permission checking
    pub enable_permissions: bool,
    
    /// Default permission policy
    pub default_policy: String,
    
    /// Enable audit logging
    pub enable_audit: bool,
    
    /// Sandbox configuration
    pub sandbox: SandboxConfig,
}

/// Execution configuration
#[derive(Debug, Clone)]
pub struct ExecutionConfig {
    /// Default execution timeout
    pub default_timeout: std::time::Duration,
    
    /// Maximum concurrent executions
    pub max_concurrent: usize,
    
    /// Enable execution caching
    pub enable_caching: bool,
    
    /// Resource limits
    pub resource_limits: ResourceLimits,
}

/// Registry configuration
#[derive(Debug, Clone)]
pub struct RegistryConfig {
    /// Enable tool discovery
    pub enable_discovery: bool,
    
    /// Tool cache size
    pub cache_size: usize,
    
    /// Enable versioning
    pub enable_versioning: bool,
    
    /// Auto-load built-in tools
    pub auto_load_builtin: bool,
}

impl Default for ToolConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            enable_sandbox: true,
            enable_monitoring: true,
            database_url: "postgresql://localhost/symbiote_tools".to_string(),
            security: SecurityConfig {
                enable_permissions: true,
                default_policy: "restrictive".to_string(),
                enable_audit: true,
                sandbox: SandboxConfig::default(),
            },
            execution: ExecutionConfig {
                default_timeout: std::time::Duration::from_secs(30),
                max_concurrent: 100,
                enable_caching: true,
                resource_limits: ResourceLimits::default(),
            },
            registry: RegistryConfig {
                enable_discovery: true,
                cache_size: 1000,
                enable_versioning: true,
                auto_load_builtin: true,
            },
        }
    }
}

impl ToolSystem {
    /// Create a new tool system instance
    pub async fn new(config: ToolConfig) -> SymbioteResult<Self> {
        let config = Arc::new(RwLock::new(config));
        
        // Initialize core components
        let security = Arc::new(PermissionManager::new(config.clone()).await?);
        let registry = Arc::new(ToolRegistry::new(config.clone()).await?);
        let executor = Arc::new(ToolExecutor::new(config.clone(), security.clone()).await?);
        let validator = Arc::new(SchemaValidator::new(config.clone()).await?);
        let composer = Arc::new(ToolComposer::new(config.clone(), registry.clone()).await?);
        let monitor = Arc::new(ToolMonitor::new(config.clone()).await?);
        
        Ok(Self {
            registry,
            executor,
            validator,
            composer,
            monitor,
            security,
            config,
        })
    }
    
    /// Start the tool system
    pub async fn start(&self) -> SymbioteResult<()> {
        tracing::info!("Starting Symbiote Tool System");
        
        // Initialize all components
        self.security.initialize().await?;
        self.registry.initialize().await?;
        self.executor.initialize().await?;
        self.validator.initialize().await?;
        self.composer.initialize().await?;
        self.monitor.initialize().await?;
        
        // Load built-in tools if configured
        let config = self.config.read().await;
        if config.registry.auto_load_builtin {
            self.load_builtin_tools().await?;
        }
        
        tracing::info!("Symbiote Tool System started successfully");
        Ok(())
    }
    
    /// Stop the tool system
    pub async fn stop(&self) -> SymbioteResult<()> {
        tracing::info!("Stopping Symbiote Tool System");
        
        // Shutdown all components in reverse order
        self.monitor.shutdown().await?;
        self.composer.shutdown().await?;
        self.validator.shutdown().await?;
        self.executor.shutdown().await?;
        self.registry.shutdown().await?;
        self.security.shutdown().await?;
        
        tracing::info!("Symbiote Tool System stopped successfully");
        Ok(())
    }
    
    /// Register a new tool
    pub async fn register_tool<T: Tool + 'static>(&self, tool: T) -> SymbioteResult<ToolId> {
        self.registry.register_tool(tool).await
    }
    
    /// Execute a tool by ID
    pub async fn execute_tool(&self, tool_id: ToolId, input: serde_json::Value, context: ToolContext) -> SymbioteResult<ExecutionResult> {
        self.executor.execute_tool(tool_id, input, context).await
    }
    
    /// Execute a tool by name
    pub async fn execute_tool_by_name(&self, name: &str, input: serde_json::Value, context: ToolContext) -> SymbioteResult<ExecutionResult> {
        self.executor.execute_tool_by_name(name, input, context).await
    }
    
    /// Get tool information
    pub async fn get_tool_info(&self, tool_id: ToolId) -> SymbioteResult<Option<ToolInfo>> {
        self.registry.get_tool_info(tool_id).await
    }
    
    /// List all available tools
    pub async fn list_tools(&self) -> SymbioteResult<Vec<ToolInfo>> {
        self.registry.list_tools().await
    }
    
    /// Search for tools
    pub async fn search_tools(&self, query: &str) -> SymbioteResult<Vec<ToolInfo>> {
        self.registry.search_tools(query).await
    }
    
    /// Create a tool chain
    pub async fn create_chain(&self, tools: Vec<ToolId>) -> SymbioteResult<ToolChain> {
        self.composer.create_chain(tools).await
    }
    
    /// Execute a tool chain
    pub async fn execute_chain(&self, chain: ToolChain, input: serde_json::Value, context: ToolContext) -> SymbioteResult<ExecutionResult> {
        self.executor.execute_chain(chain, input, context).await
    }
    
    /// Get execution metrics
    pub async fn get_metrics(&self) -> SymbioteResult<ExecutionMetrics> {
        self.monitor.get_metrics().await
    }
    
    /// Load built-in tools
    async fn load_builtin_tools(&self) -> SymbioteResult<()> {
        // Load built-in tool implementations
        // This would include file system tools, web tools, AI tools, etc.
        tracing::info!("Loading built-in tools");
        
        // Example: Load file system tools
        // self.register_tool(FileReaderTool::new()).await?;
        // self.register_tool(FileWriterTool::new()).await?;
        // ... more tools
        
        Ok(())
    }
    
    /// Get the current configuration
    pub async fn get_config(&self) -> ToolConfig {
        self.config.read().await.clone()
    }
    
    /// Update the configuration
    pub async fn update_config(&self, new_config: ToolConfig) -> SymbioteResult<()> {
        let mut config = self.config.write().await;
        *config = new_config;
        
        // Notify all components of config change
        self.security.on_config_changed(&*config).await?;
        self.registry.on_config_changed(&*config).await?;
        self.executor.on_config_changed(&*config).await?;
        self.validator.on_config_changed(&*config).await?;
        self.composer.on_config_changed(&*config).await?;
        self.monitor.on_config_changed(&*config).await?;
        
        Ok(())
    }
}

#[async_trait::async_trait]
impl Service for ToolSystem {
    fn name(&self) -> &'static str {
        "tool_system"
    }
    
    fn dependencies(&self) -> Vec<&'static str> {
        vec!["security", "storage"]
    }
    
    async fn initialize(&mut self) -> SymbioteResult<()> {
        self.start().await
    }
    
    async fn shutdown(&mut self) -> SymbioteResult<()> {
        self.stop().await
    }
}

/// Initialize the tool system with default configuration
pub async fn initialize_tool_system() -> SymbioteResult<ToolSystem> {
    let config = ToolConfig::default();
    ToolSystem::new(config).await
}

/// Initialize the tool system with custom configuration
pub async fn initialize_tool_system_with_config(config: ToolConfig) -> SymbioteResult<ToolSystem> {
    ToolSystem::new(config).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_tool_system_creation() {
        let config = ToolConfig::default();
        let tool_system = ToolSystem::new(config).await;
        assert!(tool_system.is_ok());
    }

    #[tokio::test]
    async fn test_tool_system_lifecycle() {
        let config = ToolConfig::default();
        let tool_system = ToolSystem::new(config).await.unwrap();
        
        let start_result = tool_system.start().await;
        assert!(start_result.is_ok());
        
        let stop_result = tool_system.stop().await;
        assert!(stop_result.is_ok());
    }
}
