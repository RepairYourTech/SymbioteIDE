//! # Symbiote Agent Framework
//! 
//! Comprehensive multi-agent system with HiveMind orchestration.
//! Symbiotes are intelligent AI agents that can work individually or in teams
//! to accomplish complex tasks across the entire Symbiote IDE ecosystem.

use crate::{Result, SymbioteError, AgentId, ProjectId, UserId};
use crate::context::{GlobalContext, ContextBus, ContextUpdate, ContextUpdateType};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use uuid::Uuid;
use std::time::{SystemTime, UNIX_EPOCH};

// Sub-modules
pub mod hivemind;
pub mod symbiote;
pub mod teams;
pub mod builder;
pub mod presets;
pub mod execution;
pub mod communication;
pub mod memory;

// Re-exports
pub use hivemind::*;
pub use symbiote::*;
pub use teams::*;
pub use builder::*;
pub use presets::*;
pub use execution::*;
pub use communication::*;
pub use memory::*;

/// Main agent framework managing all Symbiotes and orchestration
#[derive(Debug, Clone)]
pub struct SymbioteAgentFramework {
    /// The main orchestrator agent
    pub hivemind: Arc<RwLock<HiveMind>>,
    
    /// Registry of all available Symbiotes
    pub symbiote_registry: Arc<RwLock<SymbioteRegistry>>,
    
    /// Team management system
    pub team_manager: Arc<RwLock<TeamManager>>,
    
    /// Agent builder for creating new Symbiotes
    pub agent_builder: Arc<RwLock<AgentBuilder>>,
    
    /// Execution engine for running Symbiotes
    pub execution_engine: Arc<RwLock<ExecutionEngine>>,
    
    /// Communication system between Symbiotes
    pub communication_hub: Arc<RwLock<CommunicationHub>>,
    
    /// Context integration
    pub context_bus: Arc<RwLock<ContextBus>>,
    
    /// Active executions
    active_executions: Arc<RwLock<HashMap<String, ActiveExecution>>>,
    
    /// Performance metrics
    performance_metrics: Arc<RwLock<FrameworkMetrics>>,
}

impl SymbioteAgentFramework {
    pub fn new(context_bus: Arc<RwLock<ContextBus>>) -> Self {
        Self {
            hivemind: Arc::new(RwLock::new(HiveMind::new())),
            symbiote_registry: Arc::new(RwLock::new(SymbioteRegistry::new())),
            team_manager: Arc::new(RwLock::new(TeamManager::new())),
            agent_builder: Arc::new(RwLock::new(AgentBuilder::new())),
            execution_engine: Arc::new(RwLock::new(ExecutionEngine::new())),
            communication_hub: Arc::new(RwLock::new(CommunicationHub::new())),
            context_bus,
            active_executions: Arc::new(RwLock::new(HashMap::new())),
            performance_metrics: Arc::new(RwLock::new(FrameworkMetrics::new())),
        }
    }

    /// Initialize the framework with default presets
    pub async fn initialize(&self) -> Result<()> {
        // Initialize HiveMind
        let mut hivemind = self.hivemind.write().await;
        hivemind.initialize().await?;
        drop(hivemind);

        // Load preset Symbiotes
        let mut registry = self.symbiote_registry.write().await;
        registry.load_presets().await?;
        drop(registry);

        // Initialize team presets
        let mut team_manager = self.team_manager.write().await;
        team_manager.initialize_presets().await?;
        drop(team_manager);

        // Start background services
        self.start_background_services().await?;

        Ok(())
    }

    /// Execute a task using the HiveMind orchestrator
    pub async fn execute_task(&self, request: TaskRequest) -> Result<TaskExecution> {
        let execution_id = Uuid::new_v4().to_string();
        
        // Create execution context
        let execution = ActiveExecution {
            id: execution_id.clone(),
            request: request.clone(),
            status: ExecutionStatus::Planning,
            assigned_symbiotes: Vec::new(),
            start_time: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            end_time: None,
            results: Vec::new(),
            errors: Vec::new(),
        };

        // Store active execution
        self.active_executions.write().await.insert(execution_id.clone(), execution);

        // Let HiveMind orchestrate the task
        let mut hivemind = self.hivemind.write().await;
        let task_execution = hivemind.orchestrate_task(request, &execution_id, self).await?;

        Ok(task_execution)
    }

    /// Create a new Symbiote using the agent builder
    pub async fn create_symbiote(&self, definition: SymbioteDefinition, user_id: UserId) -> Result<SymbioteId> {
        let mut builder = self.agent_builder.write().await;
        let symbiote_id = builder.create_symbiote(definition, user_id).await?;
        
        // Register the new Symbiote
        let mut registry = self.symbiote_registry.write().await;
        let symbiote = builder.get_symbiote(&symbiote_id).await?;
        registry.register_symbiote(symbiote).await?;

        Ok(symbiote_id)
    }

    /// Get available Symbiotes for a specific stack/domain
    pub async fn get_symbiotes_for_stack(&self, stack: DevelopmentStack) -> Result<Vec<SymbioteInfo>> {
        let registry = self.symbiote_registry.read().await;
        registry.get_symbiotes_for_stack(stack).await
    }

    /// Get available teams
    pub async fn get_available_teams(&self) -> Result<Vec<TeamInfo>> {
        let team_manager = self.team_manager.read().await;
        team_manager.get_available_teams().await
    }

    /// Get performance metrics for the framework
    pub async fn get_performance_metrics(&self) -> Result<FrameworkMetrics> {
        let metrics = self.performance_metrics.read().await;
        Ok(metrics.clone())
    }

    /// Start parallel execution of multiple Symbiotes
    pub async fn start_parallel_execution(&self, requests: Vec<ParallelTaskRequest>) -> Result<Vec<TaskExecution>> {
        let execution_engine = self.execution_engine.read().await;
        execution_engine.execute_parallel(requests, self).await
    }

    /// Get execution status
    pub async fn get_execution_status(&self, execution_id: &str) -> Result<ExecutionStatus> {
        let executions = self.active_executions.read().await;
        executions.get(execution_id)
            .map(|e| e.status.clone())
            .ok_or_else(|| SymbioteError::context(format!("Execution not found: {}", execution_id)))
    }

    /// Stop an execution
    pub async fn stop_execution(&self, execution_id: &str) -> Result<()> {
        let execution_engine = self.execution_engine.read().await;
        execution_engine.stop_execution(execution_id).await
    }

    /// Get framework performance metrics
    pub async fn get_metrics(&self) -> Result<FrameworkMetrics> {
        let metrics = self.performance_metrics.read().await;
        Ok(metrics.clone())
    }

    // Private helper methods
    async fn start_background_services(&self) -> Result<()> {
        // Start context monitoring
        self.start_context_monitoring().await?;
        
        // Start performance monitoring
        self.start_performance_monitoring().await?;
        
        // Start cleanup service
        self.start_cleanup_service().await?;

        Ok(())
    }

    async fn start_context_monitoring(&self) -> Result<()> {
        let context_bus = Arc::clone(&self.context_bus);

        tokio::spawn(async move {
            let system_id = crate::context::SystemId {
                name: "symbiote-agent-framework".to_string(),
                instance_id: "main".to_string(),
            };
            let filter = crate::context::ContextFilter {
                update_types: None,
                min_priority: crate::context::UpdatePriority::Low,
                system_ids: None,
            };

            // Get a mutable reference to the context bus for subscription
            // Since ContextBus is wrapped in Arc<RwLock<_>>, we need to get a write lock
            let mut context_bus_guard = context_bus.write().await;
            if let Ok(mut receiver) = context_bus_guard.subscribe(system_id, filter).await {
                // Drop the guard to avoid holding the lock during the monitoring loop
                drop(context_bus_guard);

                while let Some(update) = receiver.recv().await {
                    // For now, just log the update since we can't access the framework
                    tracing::debug!("Received context update: {:?}", update);
                    // TODO: Implement proper context update handling
                    // This would require a different architecture to avoid lifetime issues
                }
            } else {
                tracing::error!("Failed to subscribe to context updates");
            }
        });

        tracing::info!("Context monitoring started for Symbiote Agent Framework");
        Ok(())
    }

    async fn handle_context_update(&self, update: ContextUpdate) -> Result<()> {
        match update.update_type {
            ContextUpdateType::FileModified => {
                // Notify relevant Symbiotes about file changes
                self.notify_symbiotes_of_file_change(&update).await?;
            }
            ContextUpdateType::WorkflowStarted => {
                // Check if any Symbiotes should be involved
                self.check_workflow_symbiote_involvement(&update).await?;
            }
            ContextUpdateType::AgentStateChanged => {
                // Update Symbiote state
                self.update_symbiote_state(&update).await?;
            }
            _ => {}
        }

        Ok(())
    }

    async fn notify_symbiotes_of_file_change(&self, update: &ContextUpdate) -> Result<()> {
        // Implementation for notifying Symbiotes of file changes
        Ok(())
    }

    async fn check_workflow_symbiote_involvement(&self, update: &ContextUpdate) -> Result<()> {
        // Implementation for checking if Symbiotes should be involved in workflows
        Ok(())
    }

    async fn update_symbiote_state(&self, update: &ContextUpdate) -> Result<()> {
        // Implementation for updating Symbiote state
        Ok(())
    }

    async fn start_performance_monitoring(&self) -> Result<()> {
        let metrics = Arc::clone(&self.performance_metrics);
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));
            
            loop {
                interval.tick().await;
                
                // Update performance metrics
                let mut metrics_guard = metrics.write().await;
                metrics_guard.update_metrics().await;
            }
        });

        Ok(())
    }

    async fn start_cleanup_service(&self) -> Result<()> {
        let executions = Arc::clone(&self.active_executions);
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(300)); // 5 minutes
            
            loop {
                interval.tick().await;
                
                // Clean up completed executions
                let mut executions_guard = executions.write().await;
                let current_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
                
                executions_guard.retain(|_, execution| {
                    match execution.status {
                        ExecutionStatus::Completed | ExecutionStatus::Failed | ExecutionStatus::Cancelled => {
                            // Keep for 1 hour after completion
                            execution.end_time.map_or(true, |end_time| current_time - end_time < 3600)
                        }
                        _ => true
                    }
                });
            }
        });

        Ok(())
    }
}

/// Task request for Symbiote execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRequest {
    pub id: String,
    pub user_id: UserId,
    pub project_id: Option<ProjectId>,
    pub task_type: TaskType,
    pub description: String,
    pub complexity: TaskComplexity,
    pub context: TaskContext,
    pub constraints: TaskConstraints,
    pub priority: TaskPriority,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Types of tasks that can be executed
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum TaskType {
    Development,
    Analysis,
    Testing,
    Debugging,
    Refactoring,
    Documentation,
    Deployment,
    Monitoring,
    Custom(String),
}

/// Task execution context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskContext {
    pub workspace_path: Option<String>,
    pub files: Vec<String>,
    pub environment: HashMap<String, String>,
    pub dependencies: Vec<String>,
    pub stack: Option<DevelopmentStack>,
}

/// Task execution constraints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskConstraints {
    pub max_execution_time: Option<u64>,
    pub timeout_seconds: Option<u64>,
    pub max_parallel_symbiotes: Option<usize>,
    pub allowed_operations: Vec<AllowedOperation>,
    pub resource_limits: ResourceLimits,
    pub isolation_level: IsolationLevel,
}

/// Allowed operations for Symbiotes
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AllowedOperation {
    ReadFiles,
    WriteFiles,
    FileRead,
    FileWrite,
    ExecuteCommands,
    CommandExecution,
    NetworkAccess,
    GitOperations,
    PackageInstallation,
    SystemModification,
}

/// Isolation levels for task execution
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum IsolationLevel {
    None,
    Process,
    Container,
    VirtualMachine,
}

/// Resource limits for execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    pub max_memory_mb: Option<u64>,
    pub max_cpu_cores: u32,
    pub max_cpu_percent: Option<u8>,
    pub max_disk_mb: u32,
    pub max_disk_space_mb: Option<u64>,
    pub max_network_connections: u32,
    pub max_network_requests: Option<u32>,
}

/// Task priority levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum TaskPriority {
    Low,
    Medium,
    Normal,
    High,
    Critical,
}

/// Task complexity levels
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskComplexity {
    Simple,
    Moderate,
    Complex,
    VeryComplex,
    Expert,
}

/// Development stack types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum DevelopmentStack {
    Rust,
    JavaScript,
    TypeScript,
    Python,
    React,
    NextJs,
    Vue,
    Angular,
    NodeJs,
    Deno,
    Go,
    Java,
    CSharp,
    Cpp,
    Swift,
    Kotlin,
    PHP,
    Ruby,
    Elixir,
    Haskell,
    Scala,
    Clojure,
    Docker,
    Kubernetes,
    AWS,
    Azure,
    GCP,
    Terraform,
    Ansible,
    Custom(String),
}

/// Active execution tracking
#[derive(Debug, Clone)]
pub struct ActiveExecution {
    pub id: String,
    pub request: TaskRequest,
    pub status: ExecutionStatus,
    pub assigned_symbiotes: Vec<SymbioteId>,
    pub start_time: u64,
    pub end_time: Option<u64>,
    pub results: Vec<ExecutionResult>,
    pub errors: Vec<ExecutionError>,
}

/// Execution status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionStatus {
    Planning,
    Running,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

/// Execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub symbiote_id: SymbioteId,
    pub result_type: ResultType,
    pub data: serde_json::Value,
    pub timestamp: u64,
}

/// Types of execution results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResultType {
    FileCreated,
    FileModified,
    CommandExecuted,
    AnalysisCompleted,
    TestResults,
    Documentation,
    Recommendation,
    Error,
}

/// Execution error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionError {
    pub symbiote_id: SymbioteId,
    pub error_type: ErrorType,
    pub message: String,
    pub timestamp: u64,
    pub recoverable: bool,
}

/// Types of execution errors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ErrorType {
    PermissionDenied,
    ResourceExhausted,
    NetworkError,
    FileSystemError,
    CompilationError,
    RuntimeError,
    TimeoutError,
    ConfigurationError,
}

/// Framework performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameworkMetrics {
    pub total_executions: u64,
    pub successful_executions: u64,
    pub failed_executions: u64,
    pub average_execution_time: f64,
    pub active_symbiotes: u32,
    pub total_symbiotes: u32,
    pub memory_usage: u64,
    pub cpu_usage: f32,
    pub last_updated: u64,
}

impl FrameworkMetrics {
    pub fn new() -> Self {
        Self {
            total_executions: 0,
            successful_executions: 0,
            failed_executions: 0,
            average_execution_time: 0.0,
            active_symbiotes: 0,
            total_symbiotes: 0,
            memory_usage: 0,
            cpu_usage: 0.0,
            last_updated: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
        }
    }

    pub async fn update_metrics(&mut self) {
        // Implementation for updating metrics
        self.last_updated = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    }
}
