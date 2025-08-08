use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, Mutex};
use uuid::Uuid;

use crate::context_manager::ContextManager;
pub use crate::codebase_intelligence::AgentContext;
use crate::codebase_intelligence::CodebaseIntelligence;

/// Agent types supported by the system
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AgentType {
    // Core agents (always active)
    Orchestrator,
    Architect,
    Developer,
    Tester,
    Reviewer,
    Context,
    
    // Specialized agents (task-specific)
    Test,
    Deployment,
    Security,
    Refactor,
    PerformanceOptimizer,
    Research,
    Documentation,
    Database,
    Api,
    Mobile,
    AiMl,
    FrontendSpecialist,
    BackendSpecialist,
    
    // Language specialists
    ReactSpecialist,
    RustSpecialist,
    TypeScriptSpecialist,
    PythonSpecialist,
    GoSpecialist,
    Debug,
    DevOps,
}

/// Agent lifecycle states
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AgentState {
    Inactive,
    Initializing,
    Active,
    Busy,
    Suspended,
    Error,
    Shutdown,
}

/// Agent configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub agent_type: AgentType,
    pub model_provider: String,
    pub model_name: String,
    pub max_tokens: u32,
    pub temperature: f32,
    pub timeout_seconds: u64,
    pub retry_attempts: u32,
    pub permissions: Vec<Permission>,
    pub resource_limits: ResourceLimits,
}

/// Agent permissions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Permission {
    ReadCode,
    WriteCode,
    ExecuteCommands,
    AccessFileSystem,
    NetworkAccess,
    ModifyConfiguration,
    AccessSecrets,
    DeployCode,
    ManageInfrastructure,
}

/// Resource limits for agent sandboxing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    pub max_memory_mb: u64,
    pub max_cpu_percent: u8,
    pub max_execution_time_seconds: u64,
    pub max_file_operations: u32,
    pub max_network_requests: u32,
}

/// Agent task definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTask {
    pub id: String,
    pub agent_type: AgentType,
    pub description: String,
    pub priority: TaskPriority,
    pub dependencies: Vec<String>,
    pub required_resources: Vec<String>,
    pub context_requirements: ContextRequirements,
    pub timeout: Duration,
    pub metadata: HashMap<String, String>,
}

// TaskPriority is now defined in agent_orchestrator.rs to avoid duplication
pub use crate::agent_orchestrator::TaskPriority;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextRequirements {
    pub context_types: Vec<String>,
    pub max_context_tokens: u32,
    pub compression_allowed: bool,
    pub preserve_patterns: Vec<String>,
}

/// Agent task result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub task_id: String,
    pub agent_id: String,
    pub success: bool,
    pub output: String,
    pub artifacts: Vec<TaskArtifact>,
    pub execution_time: Duration,
    pub resource_usage: ResourceUsage,
    pub quality_score: f32,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskArtifact {
    pub artifact_type: ArtifactType,
    pub content: String,
    pub file_path: Option<String>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ArtifactType {
    Code,
    Documentation,
    Test,
    Configuration,
    Schema,
    Report,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    pub memory_used_mb: u64,
    pub cpu_time_ms: u64,
    pub file_operations: u32,
    pub network_requests: u32,
    pub tokens_consumed: u32,
}

impl Default for ResourceUsage {
    fn default() -> Self {
        Self {
            memory_used_mb: 0,
            cpu_time_ms: 0,
            file_operations: 0,
            network_requests: 0,
            tokens_consumed: 0,
        }
    }
}

/// Core agent trait that all agents must implement
#[async_trait::async_trait]
pub trait Agent: Send + Sync {
    /// Get agent configuration
    fn config(&self) -> &AgentConfig;

    /// Get current agent state
    fn state(&self) -> AgentState;

    /// Initialize the agent
    async fn initialize(&mut self) -> Result<(), AgentError>;

    /// Execute a task
    async fn execute(&mut self, task: AgentTask, context: AgentContext) -> Result<TaskResult, AgentError>;

    /// Suspend agent execution
    async fn suspend(&mut self) -> Result<(), AgentError>;

    /// Resume agent execution
    async fn resume(&mut self) -> Result<(), AgentError>;

    /// Shutdown the agent
    async fn shutdown(&mut self) -> Result<(), AgentError>;

    /// Check if agent can handle a specific task
    fn can_handle(&self, task: &AgentTask) -> bool;

    /// Get agent health status
    fn health_status(&self) -> AgentHealthStatus;
}

/// Agent health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentHealthStatus {
    pub is_healthy: bool,
    pub last_heartbeat_ms: u64,
    pub error_count: u32,
    pub success_rate: f32,
    pub average_response_time_ms: u64,
    pub resource_utilization: ResourceUsage,
}

impl Default for AgentHealthStatus {
    fn default() -> Self {
        Self {
            is_healthy: true,
            last_heartbeat: Instant::now(),
            error_count: 0,
            success_rate: 1.0,
            average_response_time: Duration::ZERO,
            resource_utilization: ResourceUsage::default(),
        }
    }
}

/// Agent runtime errors
#[derive(Debug, Clone, thiserror::Error)]
pub enum AgentError {
    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),
    
    #[error("Agent not found: {0}")]
    AgentNotFound(String),
    
    #[error("Task execution failed: {0}")]
    TaskExecutionFailed(String),
    
    #[error("Resource limit exceeded: {0}")]
    ResourceLimitExceeded(String),
    
    #[error("Communication error: {0}")]
    CommunicationError(String),
    
    #[error("Context error: {0}")]
    ContextError(String),

    #[error("Configuration error: {0}")]
    ConfigurationError(String),

    #[error("Execution failed: {0}")]
    ExecutionFailed(String),
    
    #[error("Invalid task: {0}")]
    InvalidTask(String),
    
    #[error("Task not found: {0}")]
    TaskNotFound(String),
    
    #[error("Conflict requires user decision: {0:?}")]
    ConflictRequiresUserDecision(crate::agent_orchestrator::TaskConflict),
    
    #[error("Orchestration error: {0}")]
    OrchestrationError(String),
}

/// Agent result type
pub type AgentResult<T> = Result<T, AgentError>;

/// Agent capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCapability {
    pub name: String,
    pub description: String,
    pub version: String,
    pub supported_languages: Vec<String>,
    pub supported_frameworks: Vec<String>,
    pub resource_requirements: ResourceUsage,
}

/// Agent runtime manager
pub struct AgentRuntime {
    agents: Arc<RwLock<HashMap<String, Box<dyn Agent>>>>,
    agent_configs: Arc<RwLock<HashMap<AgentType, AgentConfig>>>,
    active_tasks: Arc<RwLock<HashMap<String, AgentTask>>>,
    task_results: Arc<RwLock<HashMap<String, TaskResult>>>,
    context_manager: Arc<Mutex<ContextManager>>,
    codebase_intelligence: Arc<CodebaseIntelligence>,
    message_bus: Arc<MessageBus>,
    performance_monitor: Arc<PerformanceMonitor>,
}

impl AgentRuntime {
    pub fn new(
        context_manager: ContextManager,
        codebase_intelligence: CodebaseIntelligence,
    ) -> Self {
        Self {
            agents: Arc::new(RwLock::new(HashMap::new())),
            agent_configs: Arc::new(RwLock::new(HashMap::new())),
            active_tasks: Arc::new(RwLock::new(HashMap::new())),
            task_results: Arc::new(RwLock::new(HashMap::new())),
            context_manager: Arc::new(Mutex::new(context_manager)),
            codebase_intelligence: Arc::new(codebase_intelligence),
            message_bus: Arc::new(MessageBus::new()),
            performance_monitor: Arc::new(PerformanceMonitor::new()),
        }
    }
    
    /// Register an agent configuration
    pub async fn register_agent_config(&self, config: AgentConfig) -> Result<(), AgentError> {
        let mut configs = self.agent_configs.write().unwrap();
        configs.insert(config.agent_type.clone(), config);
        Ok(())
    }
    
    /// Activate an agent
    pub async fn activate_agent(&self, agent_type: AgentType) -> Result<String, AgentError> {
        let config = {
            let configs = self.agent_configs.read().unwrap();
            configs.get(&agent_type)
                .ok_or_else(|| AgentError::ConfigurationError(format!("No config for agent type: {:?}", agent_type)))?
                .clone()
        };
        
        let agent_id = Uuid::new_v4().to_string();
        let mut agent = self.create_agent(config).await?;
        
        // Initialize the agent
        agent.initialize().await?;
        
        // Register the agent
        {
            let mut agents = self.agents.write().unwrap();
            agents.insert(agent_id.clone(), agent);
        }
        
        // Start monitoring
        self.performance_monitor.start_monitoring(&agent_id).await;
        
        Ok(agent_id)
    }
    
    /// Deactivate an agent
    pub async fn deactivate_agent(&self, agent_id: &str) -> Result<(), AgentError> {
        let mut agent = {
            let mut agents = self.agents.write().unwrap();
            agents.remove(agent_id)
                .ok_or_else(|| AgentError::AgentNotFound(agent_id.to_string()))?
        };
        
        // Shutdown the agent
        agent.shutdown().await?;
        
        // Stop monitoring
        self.performance_monitor.stop_monitoring(agent_id).await;
        
        Ok(())
    }
    
    /// Execute a task with an agent
    pub async fn execute_task(&self, task: AgentTask) -> Result<TaskResult, AgentError> {
        // Find suitable agent
        let agent_id = self.find_suitable_agent(&task).await?;
        
        // Get optimized context for the task
        let context = {
            let mut context_manager = self.context_manager.lock().await;
            context_manager.get_agent_context(&agent_id, &task.description, task.context_requirements.max_context_tokens as usize).await
                .map_err(|e| AgentError::ExecutionFailed(format!("Context retrieval failed: {}", e)))?
        };
        
        // Execute the task
        let start_time = Instant::now();
        let result = {
            let mut agents = self.agents.write().unwrap();
            let agent = agents.get_mut(&agent_id)
                .ok_or_else(|| AgentError::AgentNotFound(agent_id.clone()))?;
            
            agent.execute(task.clone(), context).await?
        };
        
        // Record performance metrics
        self.performance_monitor.record_task_execution(&agent_id, &task, &result, start_time.elapsed()).await;
        
        // Store result
        {
            let mut results = self.task_results.write().unwrap();
            results.insert(task.id.clone(), result.clone());
        }
        
        Ok(result)
    }
    
    /// Find a suitable agent for a task
    async fn find_suitable_agent(&self, task: &AgentTask) -> Result<String, AgentError> {
        let agents = self.agents.read().unwrap();
        
        for (agent_id, agent) in agents.iter() {
            if agent.config().agent_type == task.agent_type && agent.can_handle(task) {
                return Ok(agent_id.clone());
            }
        }
        
        // No suitable agent found, try to activate one
        drop(agents);
        let agent_id = self.activate_agent(task.agent_type.clone()).await?;
        Ok(agent_id)
    }
    
    /// Create an agent instance based on configuration
    async fn create_agent(&self, config: AgentConfig) -> Result<Box<dyn Agent>, AgentError> {
        match config.agent_type {
            AgentType::Orchestrator => Ok(Box::new(OrchestratorAgent::new(config))),
            AgentType::Architect => Ok(Box::new(ArchitectAgent::new(config))),
            AgentType::Developer => Ok(Box::new(DeveloperAgent::new(config))),
            AgentType::RustSpecialist => Ok(Box::new(RustSpecialistAgent::new(config))),
            AgentType::ReactSpecialist => Ok(Box::new(ReactSpecialistAgent::new(config))),
            AgentType::Debug => Ok(Box::new(DebugAgent::new(config))),
            // Add other agent types as needed
            _ => Err(AgentError::ConfigurationError(format!("Unsupported agent type: {:?}", config.agent_type))),
        }
    }
    
    /// Get agent health status
    pub async fn get_agent_health(&self, agent_id: &str) -> Result<AgentHealthStatus, AgentError> {
        let agents = self.agents.read().unwrap();
        let agent = agents.get(agent_id)
            .ok_or_else(|| AgentError::AgentNotFound(agent_id.to_string()))?;
        
        Ok(agent.health_status())
    }
    
    /// Get all active agents
    pub async fn get_active_agents(&self) -> Vec<(String, AgentType, AgentState)> {
        let agents = self.agents.read().unwrap();
        agents.iter()
            .map(|(id, agent)| (id.clone(), agent.config().agent_type.clone(), agent.state()))
            .collect()
    }
}

/// Message bus for agent communication
pub struct MessageBus {
    channels: Arc<RwLock<HashMap<String, mpsc::UnboundedSender<AgentMessage>>>>,
}

impl MessageBus {
    pub fn new() -> Self {
        Self {
            channels: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    pub async fn send_message(&self, to_agent: &str, message: AgentMessage) -> Result<(), AgentError> {
        let channels = self.channels.read().unwrap();
        if let Some(sender) = channels.get(to_agent) {
            sender.send(message)
                .map_err(|e| AgentError::CommunicationError(format!("Failed to send message: {}", e)))?;
        }
        Ok(())
    }
}

/// Agent communication message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessage {
    pub from_agent: String,
    pub to_agent: String,
    pub message_type: MessageType,
    pub content: String,
    pub timestamp: Instant,
    pub correlation_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MessageType {
    TaskRequest,
    TaskResult,
    ContextUpdate,
    StatusUpdate,
    Error,
    Heartbeat,
}

/// Performance monitoring for agents
pub struct PerformanceMonitor {
    metrics: Arc<RwLock<HashMap<String, AgentMetrics>>>,
}

impl PerformanceMonitor {
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    pub async fn start_monitoring(&self, agent_id: &str) {
        let mut metrics = self.metrics.write().unwrap();
        metrics.insert(agent_id.to_string(), AgentMetrics::new());
    }
    
    pub async fn stop_monitoring(&self, agent_id: &str) {
        let mut metrics = self.metrics.write().unwrap();
        metrics.remove(agent_id);
    }
    
    pub async fn record_task_execution(&self, agent_id: &str, task: &AgentTask, result: &TaskResult, duration: Duration) {
        let mut metrics = self.metrics.write().unwrap();
        if let Some(agent_metrics) = metrics.get_mut(agent_id) {
            agent_metrics.record_execution(task, result, duration);
        }
    }
}

#[derive(Debug, Clone)]
pub struct AgentMetrics {
    pub total_tasks: u64,
    pub successful_tasks: u64,
    pub failed_tasks: u64,
    pub average_execution_time: Duration,
    pub total_execution_time: Duration,
    pub last_activity: Instant,
}

impl Default for AgentMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentMetrics {
    pub fn new() -> Self {
        Self {
            total_tasks: 0,
            successful_tasks: 0,
            failed_tasks: 0,
            average_execution_time: Duration::from_secs(0),
            total_execution_time: Duration::from_secs(0),
            last_activity: Instant::now(),
        }
    }
    
    pub fn record_execution(&mut self, _task: &AgentTask, result: &TaskResult, duration: Duration) {
        self.total_tasks += 1;
        if result.success {
            self.successful_tasks += 1;
        } else {
            self.failed_tasks += 1;
        }
        
        self.total_execution_time += duration;
        self.average_execution_time = self.total_execution_time / self.total_tasks as u32;
        self.last_activity = Instant::now();
    }
}

// Placeholder agent implementations (to be expanded)
pub struct OrchestratorAgent {
    config: AgentConfig,
    state: AgentState,
}

impl OrchestratorAgent {
    pub fn new(config: AgentConfig) -> Self {
        Self {
            config,
            state: AgentState::Inactive,
        }
    }
}

#[async_trait::async_trait]
impl Agent for OrchestratorAgent {
    fn config(&self) -> &AgentConfig { &self.config }
    fn state(&self) -> AgentState { self.state.clone() }
    
    async fn initialize(&mut self) -> Result<(), AgentError> {
        self.state = AgentState::Active;
        Ok(())
    }
    
    async fn execute(&mut self, task: AgentTask, _context: AgentContext) -> Result<TaskResult, AgentError> {
        // Mock implementation - to be expanded
        Ok(TaskResult {
            task_id: task.id,
            agent_id: "orchestrator".to_string(),
            success: true,
            output: "Task orchestrated successfully".to_string(),
            artifacts: vec![],
            execution_time: Duration::from_millis(100),
            resource_usage: ResourceUsage {
                memory_used_mb: 10,
                cpu_time_ms: 50,
                file_operations: 0,
                network_requests: 1,
                tokens_consumed: 500,
            },
            quality_score: 0.95,
            metadata: HashMap::new(),
        })
    }
    
    async fn suspend(&mut self) -> Result<(), AgentError> {
        self.state = AgentState::Suspended;
        Ok(())
    }
    
    async fn resume(&mut self) -> Result<(), AgentError> {
        self.state = AgentState::Active;
        Ok(())
    }
    
    async fn shutdown(&mut self) -> Result<(), AgentError> {
        self.state = AgentState::Shutdown;
        Ok(())
    }
    
    fn can_handle(&self, task: &AgentTask) -> bool {
        task.agent_type == AgentType::Orchestrator
    }
    
    fn health_status(&self) -> AgentHealthStatus {
        AgentHealthStatus {
            is_healthy: true,
            last_heartbeat: Instant::now(),
            error_count: 0,
            success_rate: 1.0,
            average_response_time: Duration::from_millis(100),
            resource_utilization: ResourceUsage {
                memory_used_mb: 10,
                cpu_time_ms: 50,
                file_operations: 0,
                network_requests: 1,
                tokens_consumed: 500,
            },
        }
    }
}

// Additional placeholder agents
pub struct ArchitectAgent {
    config: AgentConfig,
    state: AgentState,
}

impl ArchitectAgent {
    pub fn new(config: AgentConfig) -> Self {
        Self { config, state: AgentState::Inactive }
    }
}

// Implement Agent trait for ArchitectAgent
#[async_trait::async_trait]
impl Agent for ArchitectAgent {
    fn config(&self) -> &AgentConfig {
        &self.config
    }

    fn state(&self) -> AgentState {
        self.state.clone()
    }

    async fn initialize(&mut self) -> Result<(), AgentError> {
        self.state = AgentState::Active;
        Ok(())
    }

    async fn execute(&mut self, task: AgentTask, _context: AgentContext) -> Result<TaskResult, AgentError> {
        Ok(TaskResult {
            task_id: task.id,
            agent_id: "architect".to_string(),
            success: true,
            output: "Architectural analysis completed".to_string(),
            artifacts: vec![],
            execution_time: Duration::from_millis(200),
            resource_usage: ResourceUsage {
                memory_used_mb: 15,
                cpu_time_ms: 100,
                file_operations: 0,
                network_requests: 0,
                tokens_consumed: 800,
            },
            quality_score: 0.92,
            metadata: HashMap::new(),
        })
    }

    async fn suspend(&mut self) -> Result<(), AgentError> {
        self.state = AgentState::Suspended;
        Ok(())
    }

    async fn resume(&mut self) -> Result<(), AgentError> {
        self.state = AgentState::Active;
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<(), AgentError> {
        self.state = AgentState::Shutdown;
        Ok(())
    }
}

pub struct DeveloperAgent {
    config: AgentConfig,
    state: AgentState,
}

impl DeveloperAgent {
    pub fn new(config: AgentConfig) -> Self {
        Self { config, state: AgentState::Inactive }
    }
}

// Implement Agent trait for DeveloperAgent
#[async_trait::async_trait]
impl Agent for DeveloperAgent {
    fn config(&self) -> &AgentConfig {
        &self.config
    }

    fn state(&self) -> AgentState {
        self.state.clone()
    }

    async fn initialize(&mut self) -> Result<(), AgentError> {
        self.state = AgentState::Active;
        Ok(())
    }

    async fn execute(&mut self, task: AgentTask, _context: AgentContext) -> Result<TaskResult, AgentError> {
        Ok(TaskResult {
            task_id: task.id,
            agent_id: "developer".to_string(),
            success: true,
            output: "Development task completed".to_string(),
            artifacts: vec![],
            execution_time: Duration::from_millis(300),
            resource_usage: ResourceUsage {
                memory_used_mb: 20,
                cpu_time_ms: 150,
                file_operations: 5,
                network_requests: 0,
                tokens_consumed: 1200,
            },
            quality_score: 0.88,
            metadata: HashMap::new(),
        })
    }

    async fn suspend(&mut self) -> Result<(), AgentError> {
        self.state = AgentState::Suspended;
        Ok(())
    }

    async fn resume(&mut self) -> Result<(), AgentError> {
        self.state = AgentState::Active;
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<(), AgentError> {
        self.state = AgentState::Shutdown;
        Ok(())
    }
}

pub struct RustSpecialistAgent {
    config: AgentConfig,
    state: AgentState,
}

impl RustSpecialistAgent {
    pub fn new(config: AgentConfig) -> Self {
        Self { config, state: AgentState::Inactive }
    }
}

// Implement Agent trait for RustSpecialistAgent
#[async_trait::async_trait]
impl Agent for RustSpecialistAgent {
    fn config(&self) -> &AgentConfig {
        &self.config
    }

    fn state(&self) -> AgentState {
        self.state.clone()
    }

    async fn initialize(&mut self) -> Result<(), AgentError> {
        self.state = AgentState::Active;
        Ok(())
    }

    async fn execute(&mut self, task: AgentTask, _context: AgentContext) -> Result<TaskResult, AgentError> {
        Ok(TaskResult {
            task_id: task.id,
            agent_id: "rust_specialist".to_string(),
            success: true,
            output: "Rust specialization task completed".to_string(),
            artifacts: vec![],
            execution_time: Duration::from_millis(250),
            resource_usage: ResourceUsage {
                memory_used_mb: 18,
                cpu_time_ms: 120,
                file_operations: 3,
                network_requests: 0,
                tokens_consumed: 1000,
            },
            quality_score: 0.90,
            metadata: HashMap::new(),
        })
    }

    async fn suspend(&mut self) -> Result<(), AgentError> {
        self.state = AgentState::Suspended;
        Ok(())
    }

    async fn resume(&mut self) -> Result<(), AgentError> {
        self.state = AgentState::Active;
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<(), AgentError> {
        self.state = AgentState::Shutdown;
        Ok(())
    }
}

pub struct ReactSpecialistAgent {
    config: AgentConfig,
    state: AgentState,
}

impl ReactSpecialistAgent {
    pub fn new(config: AgentConfig) -> Self {
        Self { config, state: AgentState::Inactive }
    }
}

// Implement Agent trait for ReactSpecialistAgent
#[async_trait::async_trait]
impl Agent for ReactSpecialistAgent {
    fn config(&self) -> &AgentConfig {
        &self.config
    }

    fn state(&self) -> AgentState {
        self.state.clone()
    }

    async fn initialize(&mut self) -> Result<(), AgentError> {
        self.state = AgentState::Active;
        Ok(())
    }

    async fn execute(&mut self, task: AgentTask, _context: AgentContext) -> Result<TaskResult, AgentError> {
        Ok(TaskResult {
            task_id: task.id,
            agent_id: "react_specialist".to_string(),
            success: true,
            output: "React specialization task completed".to_string(),
            artifacts: vec![],
            execution_time: Duration::from_millis(220),
            resource_usage: ResourceUsage {
                memory_used_mb: 16,
                cpu_time_ms: 110,
                file_operations: 2,
                network_requests: 0,
                tokens_consumed: 900,
            },
            quality_score: 0.87,
            metadata: HashMap::new(),
        })
    }

    async fn suspend(&mut self) -> Result<(), AgentError> {
        self.state = AgentState::Suspended;
        Ok(())
    }

    async fn resume(&mut self) -> Result<(), AgentError> {
        self.state = AgentState::Active;
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<(), AgentError> {
        self.state = AgentState::Shutdown;
        Ok(())
    }
}

pub struct DebugAgent {
    config: AgentConfig,
    state: AgentState,
}

impl DebugAgent {
    pub fn new(config: AgentConfig) -> Self {
        Self { config, state: AgentState::Inactive }
    }
}

// Implement Agent trait for DebugAgent
#[async_trait::async_trait]
impl Agent for DebugAgent {
    fn config(&self) -> &AgentConfig {
        &self.config
    }

    fn state(&self) -> AgentState {
        self.state.clone()
    }

    async fn initialize(&mut self) -> Result<(), AgentError> {
        self.state = AgentState::Active;
        Ok(())
    }

    async fn execute(&mut self, task: AgentTask, _context: AgentContext) -> Result<TaskResult, AgentError> {
        Ok(TaskResult {
            task_id: task.id,
            agent_id: "debug_agent".to_string(),
            success: true,
            output: "Debug task completed".to_string(),
            artifacts: vec![],
            execution_time: Duration::from_millis(180),
            resource_usage: ResourceUsage {
                memory_used_mb: 12,
                cpu_time_ms: 90,
                file_operations: 1,
                network_requests: 0,
                tokens_consumed: 600,
            },
            quality_score: 0.85,
            metadata: HashMap::new(),
        })
    }

    async fn suspend(&mut self) -> Result<(), AgentError> {
        self.state = AgentState::Suspended;
        Ok(())
    }

    async fn resume(&mut self) -> Result<(), AgentError> {
        self.state = AgentState::Active;
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<(), AgentError> {
        self.state = AgentState::Shutdown;
        Ok(())
    }
}

// Implement Agent trait for other agent types (similar pattern)
// This will be expanded in subsequent implementations
