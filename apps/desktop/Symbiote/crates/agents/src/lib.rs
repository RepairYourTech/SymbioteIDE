//! # Symbiote Agents
//!
//! Core agent framework and runtime for Symbiote multi-agent systems. Features:
//! - High-performance agent execution environment with work-stealing scheduler
//! - Multi-agent coordination with sophisticated consensus algorithms
//! - Dynamic agent discovery and service registration
//! - Intelligent load balancing and fault tolerance
//! - Dynamic scaling based on demand and performance metrics
//! - Complex multi-agent workflow orchestration
//! - Production-ready runtime supporting 10,000+ concurrent agents

#![deny(missing_docs)]
#![warn(clippy::all)]

// Core modules
pub mod runtime;
pub mod coordination;
pub mod discovery;
pub mod load_balancing;
pub mod orchestration;
pub mod scaling;
pub mod fault_tolerance;
pub mod monitoring;
pub mod framework;
pub mod types;

// Re-export main types
pub use runtime::{AgentRuntime, AgentExecutor, AgentScheduler, AgentSupervisor};
pub use coordination::{CoordinationEngine, ConsensusManager, ElectionManager};
pub use discovery::{ServiceDiscovery, ServiceRegistry, HealthChecker};
pub use load_balancing::{LoadBalancer, LoadBalancingStrategy, HealthAwareBalancer};
pub use orchestration::{WorkflowOrchestrator, AgentChoreographer, CoordinationPatterns};
pub use scaling::{AutoScaler, ScalingMetrics, ScalingPolicies, AgentProvisioner};
pub use fault_tolerance::{CircuitBreaker, RetryMechanism, BulkheadIsolation, ErrorRecovery};
pub use monitoring::{AgentMonitor, PerformanceTracker, SystemObserver};
pub use framework::{SymbioteAgentFramework, AgentFrameworkConfig};
pub use types::{AgentConfig, AgentResult, AgentError, AgentMetrics};

// Core dependencies
use symbiote_core::{SymbioteResult, SymbioteError, Service, AgentId};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Main agent framework that orchestrates all agent operations
pub struct AgentFramework {
    /// Core agent runtime
    pub runtime: Arc<AgentRuntime>,
    
    /// Multi-agent coordination engine
    pub coordination: Arc<CoordinationEngine>,
    
    /// Service discovery and registration
    pub discovery: Arc<ServiceDiscovery>,
    
    /// Load balancing system
    pub load_balancer: Arc<LoadBalancer>,
    
    /// Workflow orchestration
    pub orchestrator: Arc<WorkflowOrchestrator>,
    
    /// Auto-scaling system
    pub auto_scaler: Arc<AutoScaler>,
    
    /// Fault tolerance mechanisms
    pub fault_tolerance: Arc<FaultToleranceManager>,
    
    /// System monitoring
    pub monitor: Arc<AgentMonitor>,
    
    /// Symbiote agent framework
    pub symbiote_framework: Arc<SymbioteAgentFramework>,
    
    /// Configuration
    config: Arc<RwLock<AgentFrameworkConfig>>,
}

/// Configuration for the agent framework
#[derive(Debug, Clone)]
pub struct AgentFrameworkConfig {
    /// Maximum number of concurrent agents
    pub max_agents: usize,
    
    /// Agent execution timeout
    pub execution_timeout: std::time::Duration,
    
    /// Enable consensus algorithms
    pub enable_consensus: bool,
    
    /// Enable auto-scaling
    pub enable_auto_scaling: bool,
    
    /// Database connection string
    pub database_url: String,
    
    /// Service discovery configuration
    pub discovery: DiscoveryConfig,
    
    /// Load balancing configuration
    pub load_balancing: LoadBalancingConfig,
}

/// Service discovery configuration
#[derive(Debug, Clone)]
pub struct DiscoveryConfig {
    /// Discovery backend (consul, etcd, custom)
    pub backend: String,
    
    /// Service registration TTL
    pub ttl: std::time::Duration,
    
    /// Health check interval
    pub health_check_interval: std::time::Duration,
}

/// Load balancing configuration
#[derive(Debug, Clone)]
pub struct LoadBalancingConfig {
    /// Load balancing strategy
    pub strategy: String,
    
    /// Health check configuration
    pub health_check: bool,
    
    /// Circuit breaker configuration
    pub circuit_breaker: bool,
}

/// Fault tolerance manager
pub struct FaultToleranceManager {
    /// Circuit breaker system
    pub circuit_breaker: Arc<CircuitBreaker>,
    
    /// Retry mechanisms
    pub retry_mechanism: Arc<RetryMechanism>,
    
    /// Bulkhead isolation
    pub bulkhead: Arc<BulkheadIsolation>,
    
    /// Error recovery
    pub error_recovery: Arc<ErrorRecovery>,
}

impl Default for AgentFrameworkConfig {
    fn default() -> Self {
        Self {
            max_agents: 10000,
            execution_timeout: std::time::Duration::from_secs(300), // 5 minutes
            enable_consensus: true,
            enable_auto_scaling: true,
            database_url: "postgresql://localhost/symbiote_agents".to_string(),
            discovery: DiscoveryConfig {
                backend: "consul".to_string(),
                ttl: std::time::Duration::from_secs(30),
                health_check_interval: std::time::Duration::from_secs(10),
            },
            load_balancing: LoadBalancingConfig {
                strategy: "round_robin".to_string(),
                health_check: true,
                circuit_breaker: true,
            },
        }
    }
}

impl AgentFramework {
    /// Create a new agent framework instance
    pub async fn new(config: AgentFrameworkConfig) -> SymbioteResult<Self> {
        let config = Arc::new(RwLock::new(config));
        
        // Initialize core components
        let runtime = Arc::new(AgentRuntime::new(config.clone()).await?);
        let coordination = Arc::new(CoordinationEngine::new(config.clone()).await?);
        let discovery = Arc::new(ServiceDiscovery::new(config.clone()).await?);
        let load_balancer = Arc::new(LoadBalancer::new(config.clone()).await?);
        let orchestrator = Arc::new(WorkflowOrchestrator::new(config.clone()).await?);
        let auto_scaler = Arc::new(AutoScaler::new(config.clone()).await?);
        let monitor = Arc::new(AgentMonitor::new(config.clone()).await?);
        
        // Initialize fault tolerance components
        let circuit_breaker = Arc::new(CircuitBreaker::new(config.clone()).await?);
        let retry_mechanism = Arc::new(RetryMechanism::new(config.clone()).await?);
        let bulkhead = Arc::new(BulkheadIsolation::new(config.clone()).await?);
        let error_recovery = Arc::new(ErrorRecovery::new(config.clone()).await?);
        
        let fault_tolerance = Arc::new(FaultToleranceManager {
            circuit_breaker,
            retry_mechanism,
            bulkhead,
            error_recovery,
        });
        
        // Initialize Symbiote agent framework
        let symbiote_framework = Arc::new(SymbioteAgentFramework::new(config.clone()).await?);
        
        Ok(Self {
            runtime,
            coordination,
            discovery,
            load_balancer,
            orchestrator,
            auto_scaler,
            fault_tolerance,
            monitor,
            symbiote_framework,
            config,
        })
    }
    
    /// Start the agent framework
    pub async fn start(&self) -> SymbioteResult<()> {
        tracing::info!("Starting Symbiote Agent Framework");
        
        // Initialize all components
        self.runtime.initialize().await?;
        self.coordination.initialize().await?;
        self.discovery.initialize().await?;
        self.load_balancer.initialize().await?;
        self.orchestrator.initialize().await?;
        self.auto_scaler.initialize().await?;
        self.fault_tolerance.circuit_breaker.initialize().await?;
        self.fault_tolerance.retry_mechanism.initialize().await?;
        self.fault_tolerance.bulkhead.initialize().await?;
        self.fault_tolerance.error_recovery.initialize().await?;
        self.monitor.initialize().await?;
        self.symbiote_framework.initialize().await?;
        
        tracing::info!("Symbiote Agent Framework started successfully");
        Ok(())
    }
    
    /// Stop the agent framework
    pub async fn stop(&self) -> SymbioteResult<()> {
        tracing::info!("Stopping Symbiote Agent Framework");
        
        // Shutdown all components in reverse order
        self.symbiote_framework.shutdown().await?;
        self.monitor.shutdown().await?;
        self.fault_tolerance.error_recovery.shutdown().await?;
        self.fault_tolerance.bulkhead.shutdown().await?;
        self.fault_tolerance.retry_mechanism.shutdown().await?;
        self.fault_tolerance.circuit_breaker.shutdown().await?;
        self.auto_scaler.shutdown().await?;
        self.orchestrator.shutdown().await?;
        self.load_balancer.shutdown().await?;
        self.discovery.shutdown().await?;
        self.coordination.shutdown().await?;
        self.runtime.shutdown().await?;
        
        tracing::info!("Symbiote Agent Framework stopped successfully");
        Ok(())
    }
    
    /// Deploy a new agent
    pub async fn deploy_agent(&self, agent_config: AgentConfig) -> SymbioteResult<AgentId> {
        self.runtime.deploy_agent(agent_config).await
    }
    
    /// Remove an agent
    pub async fn remove_agent(&self, agent_id: AgentId) -> SymbioteResult<()> {
        self.runtime.remove_agent(agent_id).await
    }
    
    /// Get agent metrics
    pub async fn get_agent_metrics(&self, agent_id: AgentId) -> SymbioteResult<AgentMetrics> {
        self.monitor.get_agent_metrics(agent_id).await
    }
    
    /// Get framework metrics
    pub async fn get_framework_metrics(&self) -> SymbioteResult<FrameworkMetrics> {
        self.monitor.get_framework_metrics().await
    }
    
    /// Get the current configuration
    pub async fn get_config(&self) -> AgentFrameworkConfig {
        self.config.read().await.clone()
    }
    
    /// Update the configuration
    pub async fn update_config(&self, new_config: AgentFrameworkConfig) -> SymbioteResult<()> {
        let mut config = self.config.write().await;
        *config = new_config;
        
        // Notify all components of config change
        self.runtime.on_config_changed(&*config).await?;
        self.coordination.on_config_changed(&*config).await?;
        self.discovery.on_config_changed(&*config).await?;
        self.load_balancer.on_config_changed(&*config).await?;
        self.orchestrator.on_config_changed(&*config).await?;
        self.auto_scaler.on_config_changed(&*config).await?;
        self.monitor.on_config_changed(&*config).await?;
        self.symbiote_framework.on_config_changed(&*config).await?;
        
        Ok(())
    }
}

/// Framework-wide metrics
#[derive(Debug, Clone)]
pub struct FrameworkMetrics {
    /// Total number of agents
    pub total_agents: usize,
    
    /// Active agents
    pub active_agents: usize,
    
    /// Failed agents
    pub failed_agents: usize,
    
    /// Total tasks processed
    pub total_tasks: u64,
    
    /// Success rate
    pub success_rate: f64,
    
    /// Average response time
    pub avg_response_time: std::time::Duration,
}

#[async_trait::async_trait]
impl Service for AgentFramework {
    fn name(&self) -> &'static str {
        "agent_framework"
    }
    
    fn dependencies(&self) -> Vec<&'static str> {
        vec!["storage", "monitoring", "security"]
    }
    
    async fn initialize(&mut self) -> SymbioteResult<()> {
        self.start().await
    }
    
    async fn shutdown(&mut self) -> SymbioteResult<()> {
        self.stop().await
    }
}

/// Initialize the agent framework with default configuration
pub async fn initialize_agent_framework() -> SymbioteResult<AgentFramework> {
    let config = AgentFrameworkConfig::default();
    AgentFramework::new(config).await
}

/// Initialize the agent framework with custom configuration
pub async fn initialize_agent_framework_with_config(config: AgentFrameworkConfig) -> SymbioteResult<AgentFramework> {
    AgentFramework::new(config).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_agent_framework_creation() {
        let config = AgentFrameworkConfig::default();
        let framework = AgentFramework::new(config).await;
        assert!(framework.is_ok());
    }

    #[tokio::test]
    async fn test_agent_framework_lifecycle() {
        let config = AgentFrameworkConfig::default();
        let framework = AgentFramework::new(config).await.unwrap();
        
        let start_result = framework.start().await;
        assert!(start_result.is_ok());
        
        let stop_result = framework.stop().await;
        assert!(stop_result.is_ok());
    }
}
