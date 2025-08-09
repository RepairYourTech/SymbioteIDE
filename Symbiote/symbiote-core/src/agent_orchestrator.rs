//! # Agent Orchestrator for Symbiote IDE
//! 
//! Manages 10+ concurrent agents with load balancing, health monitoring,
//! and intelligent task distribution.
//! 
//! Following Week 7-8 Context Management & Agent Orchestration implementation plan.

use crate::{Result, SymbioteError, AgentId, ProjectId, UserId, context_bus::{ContextBus, ContextEvent}};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::{RwLock, Semaphore, mpsc};
use tokio::time::timeout;
use uuid::Uuid;
use dashmap::DashMap;

/// Agent types supported by the orchestrator
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AgentType {
    CodeAssistant,
    CodeReviewer,
    TestGenerator,
    DocumentationWriter,
    SecurityAnalyzer,
    PerformanceOptimizer,
    RefactoringAssistant,
    BugFinder,
    APIGenerator,
    DatabaseDesigner,
    Custom(String),
}

/// Agent status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentStatus {
    Idle,
    Busy,
    Starting,
    Stopping,
    Error,
    Maintenance,
}

/// Agent health metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentHealth {
    /// Agent status
    pub status: AgentStatus,
    
    /// CPU usage percentage
    pub cpu_usage: f32,
    
    /// Memory usage in MB
    pub memory_usage: u64,
    
    /// Number of active tasks
    pub active_tasks: u32,
    
    /// Total tasks completed
    pub completed_tasks: u64,
    
    /// Total tasks failed
    pub failed_tasks: u64,
    
    /// Average task completion time in milliseconds
    pub avg_completion_time_ms: u64,
    
    /// Last heartbeat timestamp
    pub last_heartbeat: u64,
    
    /// Error count in last hour
    pub error_count: u32,
    
    /// Agent uptime in seconds
    pub uptime_seconds: u64,
}

/// Agent configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// Agent type
    pub agent_type: AgentType,
    
    /// Maximum concurrent tasks
    pub max_concurrent_tasks: u32,
    
    /// Task timeout in seconds
    pub task_timeout_seconds: u64,
    
    /// AI model to use
    pub ai_model: String,
    
    /// Agent-specific settings
    pub settings: HashMap<String, serde_json::Value>,
    
    /// Resource limits
    pub resource_limits: ResourceLimits,
    
    /// Auto-restart on failure
    pub auto_restart: bool,
    
    /// Heartbeat interval in seconds
    pub heartbeat_interval: u64,
}

/// Resource limits for agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// Maximum memory usage in MB
    pub max_memory_mb: u64,
    
    /// Maximum CPU usage percentage
    pub max_cpu_percent: f32,
    
    /// Maximum task queue size
    pub max_queue_size: u32,
    
    /// Maximum execution time per task in seconds
    pub max_execution_time: u64,
}

/// Task to be executed by an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTask {
    /// Unique task ID
    pub id: Uuid,
    
    /// Task type
    pub task_type: String,
    
    /// Task priority (higher = more important)
    pub priority: u32,
    
    /// Task payload
    pub payload: serde_json::Value,
    
    /// Project context
    pub project_id: Option<ProjectId>,
    
    /// User context
    pub user_id: Option<UserId>,
    
    /// Task deadline
    pub deadline: Option<u64>,
    
    /// Task metadata
    pub metadata: HashMap<String, String>,
    
    /// Created timestamp
    pub created_at: u64,
    
    /// Retry count
    pub retry_count: u32,
}

/// Task execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    /// Task ID
    pub task_id: Uuid,
    
    /// Agent ID that executed the task
    pub agent_id: AgentId,
    
    /// Execution status
    pub status: TaskStatus,
    
    /// Result data
    pub result: Option<serde_json::Value>,
    
    /// Error message if failed
    pub error: Option<String>,
    
    /// Execution time in milliseconds
    pub execution_time_ms: u64,
    
    /// Completion timestamp
    pub completed_at: u64,
}

/// Task execution status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
    Timeout,
}

/// Agent instance managed by the orchestrator
#[derive(Debug)]
pub struct ManagedAgent {
    /// Agent ID
    pub id: AgentId,
    
    /// Agent configuration
    pub config: AgentConfig,
    
    /// Agent health metrics
    pub health: Arc<RwLock<AgentHealth>>,
    
    /// Task queue
    pub task_queue: Arc<RwLock<Vec<AgentTask>>>,
    
    /// Task execution semaphore
    pub task_semaphore: Arc<Semaphore>,
    
    /// Agent start time
    pub started_at: Instant,
    
    /// Task sender channel
    pub task_sender: mpsc::UnboundedSender<AgentTask>,
}

/// Load balancing strategy
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadBalancingStrategy {
    RoundRobin,
    LeastLoaded,
    Random,
    PriorityBased,
    ResourceAware,
}

/// Orchestrator configuration
#[derive(Debug, Clone)]
pub struct OrchestratorConfig {
    /// Maximum number of agents
    pub max_agents: usize,
    
    /// Load balancing strategy
    pub load_balancing: LoadBalancingStrategy,
    
    /// Health check interval
    pub health_check_interval: Duration,
    
    /// Task timeout
    pub default_task_timeout: Duration,
    
    /// Maximum task retries
    pub max_task_retries: u32,
    
    /// Agent restart threshold (error count)
    pub restart_threshold: u32,
    
    /// Enable auto-scaling
    pub auto_scaling: bool,
    
    /// Scaling thresholds
    pub scale_up_threshold: f32,
    pub scale_down_threshold: f32,
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            max_agents: 20,
            load_balancing: LoadBalancingStrategy::LeastLoaded,
            health_check_interval: Duration::from_secs(30),
            default_task_timeout: Duration::from_secs(300),
            max_task_retries: 3,
            restart_threshold: 10,
            auto_scaling: true,
            scale_up_threshold: 0.8, // 80% utilization
            scale_down_threshold: 0.3, // 30% utilization
        }
    }
}

/// Agent orchestrator managing multiple agents
pub struct AgentOrchestrator {
    /// Configuration
    config: OrchestratorConfig,
    
    /// Managed agents
    agents: Arc<DashMap<AgentId, Arc<ManagedAgent>>>,
    
    /// Context bus for events
    context_bus: Arc<ContextBus>,
    
    /// Global task queue
    global_queue: Arc<RwLock<Vec<AgentTask>>>,
    
    /// Task results
    task_results: Arc<DashMap<Uuid, TaskResult>>,
    
    /// Running flag
    running: Arc<RwLock<bool>>,
    
    /// Round-robin counter for load balancing
    round_robin_counter: Arc<RwLock<usize>>,
}

impl AgentOrchestrator {
    /// Create a new agent orchestrator
    pub fn new(config: OrchestratorConfig, context_bus: Arc<ContextBus>) -> Self {
        Self {
            config,
            agents: Arc::new(DashMap::new()),
            context_bus,
            global_queue: Arc::new(RwLock::new(Vec::new())),
            task_results: Arc::new(DashMap::new()),
            running: Arc::new(RwLock::new(false)),
            round_robin_counter: Arc::new(RwLock::new(0)),
        }
    }

    /// Start the orchestrator
    pub async fn start(&self) -> Result<()> {
        let mut running = self.running.write().await;
        if *running {
            return Err(SymbioteError::validation("Orchestrator is already running"));
        }
        *running = true;

        // Start health monitoring
        self.start_health_monitor();
        
        // Start task dispatcher
        self.start_task_dispatcher();
        
        // Start auto-scaling if enabled
        if self.config.auto_scaling {
            self.start_auto_scaler();
        }

        // Publish startup event
        self.context_bus.publish(ContextEvent::SystemStartup {
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
        }).await?;

        tracing::info!("Agent orchestrator started");
        Ok(())
    }

    /// Stop the orchestrator
    pub async fn stop(&self) -> Result<()> {
        let mut running = self.running.write().await;
        if !*running {
            return Err(SymbioteError::validation("Orchestrator is not running"));
        }
        *running = false;

        // Stop all agents
        for agent_entry in self.agents.iter() {
            self.stop_agent_internal(agent_entry.key()).await?;
        }

        tracing::info!("Agent orchestrator stopped");
        Ok(())
    }

    /// Create and start a new agent
    pub async fn create_agent(&self, config: AgentConfig) -> Result<AgentId> {
        if self.agents.len() >= self.config.max_agents {
            return Err(SymbioteError::validation("Maximum number of agents reached"));
        }

        let agent_id = AgentId::new();
        let (task_sender, task_receiver) = mpsc::unbounded_channel();

        let agent = Arc::new(ManagedAgent {
            id: agent_id,
            config: config.clone(),
            health: Arc::new(RwLock::new(AgentHealth {
                status: AgentStatus::Starting,
                cpu_usage: 0.0,
                memory_usage: 0,
                active_tasks: 0,
                completed_tasks: 0,
                failed_tasks: 0,
                avg_completion_time_ms: 0,
                last_heartbeat: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
                error_count: 0,
                uptime_seconds: 0,
            })),
            task_queue: Arc::new(RwLock::new(Vec::new())),
            task_semaphore: Arc::new(Semaphore::new(config.max_concurrent_tasks as usize)),
            started_at: Instant::now(),
            task_sender,
        });

        // Start agent task processor
        self.start_agent_processor(agent.clone(), task_receiver);

        // Register agent
        self.agents.insert(agent_id, agent);

        // Update agent status to idle
        if let Some(agent) = self.agents.get(&agent_id) {
            let mut health = agent.health.write().await;
            health.status = AgentStatus::Idle;
        }

        // Publish agent started event
        self.context_bus.publish(ContextEvent::AgentStarted {
            agent_id,
            agent_type: format!("{:?}", config.agent_type),
            project_id: None,
        }).await?;

        tracing::info!("Created agent {} of type {:?}", agent_id, config.agent_type);
        Ok(agent_id)
    }

    /// Stop and remove an agent
    pub async fn stop_agent(&self, agent_id: &AgentId) -> Result<()> {
        self.stop_agent_internal(agent_id).await?;
        self.agents.remove(agent_id);

        // Publish agent stopped event
        self.context_bus.publish(ContextEvent::AgentStopped {
            agent_id: *agent_id,
            reason: "Manual stop".to_string(),
        }).await?;

        tracing::info!("Stopped agent {}", agent_id);
        Ok(())
    }

    /// Submit a task for execution
    pub async fn submit_task(&self, task: AgentTask) -> Result<()> {
        // Find best agent for the task
        let agent_id = self.select_agent_for_task(&task).await?;

        if let Some(agent) = self.agents.get(&agent_id) {
            // Send task to agent
            agent.task_sender.send(task.clone())
                .map_err(|_| SymbioteError::internal("Failed to send task to agent"))?;

            // Update agent health
            {
                let mut health = agent.health.write().await;
                health.active_tasks += 1;
                if health.status == AgentStatus::Idle {
                    health.status = AgentStatus::Busy;
                }
            }

            tracing::debug!("Submitted task {} to agent {}", task.id, agent_id);
        } else {
            return Err(SymbioteError::not_found("Agent not found"));
        }

        Ok(())
    }

    /// Get task result
    pub async fn get_task_result(&self, task_id: &Uuid) -> Option<TaskResult> {
        self.task_results.get(task_id).map(|entry| entry.clone())
    }

    /// Get agent health
    pub async fn get_agent_health(&self, agent_id: &AgentId) -> Option<AgentHealth> {
        if let Some(agent) = self.agents.get(agent_id) {
            Some(agent.health.read().await.clone())
        } else {
            None
        }
    }

    /// Get orchestrator statistics
    pub async fn get_stats(&self) -> OrchestratorStats {
        let mut total_tasks = 0u64;
        let mut total_completed = 0u64;
        let mut total_failed = 0u64;
        let mut active_agents = 0usize;

        for agent_entry in self.agents.iter() {
            let health = agent_entry.health.read().await;
            total_completed += health.completed_tasks;
            total_failed += health.failed_tasks;
            total_tasks += health.completed_tasks + health.failed_tasks;
            
            if health.status != AgentStatus::Error && health.status != AgentStatus::Maintenance {
                active_agents += 1;
            }
        }

        OrchestratorStats {
            total_agents: self.agents.len(),
            active_agents,
            total_tasks,
            completed_tasks: total_completed,
            failed_tasks: total_failed,
            pending_tasks: self.global_queue.read().await.len(),
            avg_response_time_ms: 0, // TODO: Calculate from task results
        }
    }

    /// Select best agent for a task using load balancing strategy
    async fn select_agent_for_task(&self, _task: &AgentTask) -> Result<AgentId> {
        let available_agents: Vec<_> = self.agents.iter()
            .filter(|entry| {
                // Filter by agent status and capacity
                let health = entry.health.try_read();
                if let Ok(health) = health {
                    health.status == AgentStatus::Idle || health.status == AgentStatus::Busy
                } else {
                    false
                }
            })
            .collect();

        if available_agents.is_empty() {
            return Err(SymbioteError::not_found("No available agents"));
        }

        match self.config.load_balancing {
            LoadBalancingStrategy::RoundRobin => {
                let mut counter = self.round_robin_counter.write().await;
                let index = *counter % available_agents.len();
                *counter += 1;
                Ok(*available_agents[index].key())
            }
            LoadBalancingStrategy::LeastLoaded => {
                let mut best_agent = None;
                let mut min_load = u32::MAX;

                for agent_entry in available_agents {
                    if let Ok(health) = agent_entry.health.try_read() {
                        if health.active_tasks < min_load {
                            min_load = health.active_tasks;
                            best_agent = Some(*agent_entry.key());
                        }
                    }
                }

                best_agent.ok_or_else(|| SymbioteError::internal("Failed to select agent"))
            }
            LoadBalancingStrategy::Random => {
                use rand::Rng;
                let index = rand::thread_rng().gen_range(0..available_agents.len());
                Ok(*available_agents[index].key())
            }
            _ => {
                // Default to first available agent
                Ok(*available_agents[0].key())
            }
        }
    }

    /// Start agent task processor
    fn start_agent_processor(&self, agent: Arc<ManagedAgent>, mut task_receiver: mpsc::UnboundedReceiver<AgentTask>) {
        let context_bus = self.context_bus.clone();
        let task_results = self.task_results.clone();
        let config = self.config.clone();

        tokio::spawn(async move {
            while let Some(task) = task_receiver.recv().await {
                let permit = agent.task_semaphore.clone().acquire_owned().await.unwrap();
                let agent = agent.clone();
                let context_bus = context_bus.clone();
                let task_results = task_results.clone();
                let _config = config.clone();

                tokio::spawn(async move {
                    let _permit = permit;
                    let start_time = Instant::now();

                    // Simulate task execution (in real implementation, this would call actual AI agents)
                    let result = timeout(
                        Duration::from_secs(agent.config.task_timeout_seconds),
                        Self::execute_task_simulation(task.clone())
                    ).await;

                    let execution_time = start_time.elapsed();
                    let task_result = match result {
                        Ok(Ok(result_data)) => TaskResult {
                            task_id: task.id,
                            agent_id: agent.id,
                            status: TaskStatus::Completed,
                            result: Some(result_data),
                            error: None,
                            execution_time_ms: execution_time.as_millis() as u64,
                            completed_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
                        },
                        Ok(Err(e)) => TaskResult {
                            task_id: task.id,
                            agent_id: agent.id,
                            status: TaskStatus::Failed,
                            result: None,
                            error: Some(e.to_string()),
                            execution_time_ms: execution_time.as_millis() as u64,
                            completed_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
                        },
                        Err(_) => TaskResult {
                            task_id: task.id,
                            agent_id: agent.id,
                            status: TaskStatus::Timeout,
                            result: None,
                            error: Some("Task timeout".to_string()),
                            execution_time_ms: execution_time.as_millis() as u64,
                            completed_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
                        },
                    };

                    // Store result
                    task_results.insert(task.id, task_result.clone());

                    // Update agent health
                    {
                        let mut health = agent.health.write().await;
                        health.active_tasks = health.active_tasks.saturating_sub(1);
                        
                        match task_result.status {
                            TaskStatus::Completed => {
                                health.completed_tasks += 1;
                                // Update average completion time
                                let new_avg = (health.avg_completion_time_ms as f64 * 0.9) + 
                                             (execution_time.as_millis() as f64 * 0.1);
                                health.avg_completion_time_ms = new_avg as u64;
                            }
                            TaskStatus::Failed | TaskStatus::Timeout => {
                                health.failed_tasks += 1;
                                health.error_count += 1;
                            }
                            _ => {}
                        }

                        if health.active_tasks == 0 {
                            health.status = AgentStatus::Idle;
                        }
                    }

                    // Publish completion event
                    let event = match task_result.status {
                        TaskStatus::Completed => ContextEvent::AIRequestCompleted {
                            request_id: task.id.to_string(),
                            agent_id: agent.id,
                            duration_ms: execution_time.as_millis() as u64,
                            tokens_used: 0, // TODO: Track actual token usage
                        },
                        _ => ContextEvent::AIRequestFailed {
                            request_id: task.id.to_string(),
                            agent_id: agent.id,
                            error: task_result.error.unwrap_or_else(|| "Unknown error".to_string()),
                        },
                    };

                    let _ = context_bus.publish(event).await;
                });
            }
        });
    }

    /// Simulate task execution (placeholder for actual AI agent execution)
    async fn execute_task_simulation(task: AgentTask) -> Result<serde_json::Value> {
        // Simulate processing time
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        // Return mock result
        Ok(serde_json::json!({
            "task_id": task.id,
            "result": "Task completed successfully",
            "timestamp": SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
        }))
    }

    /// Stop agent internal implementation
    async fn stop_agent_internal(&self, agent_id: &AgentId) -> Result<()> {
        if let Some(agent) = self.agents.get(agent_id) {
            let mut health = agent.health.write().await;
            health.status = AgentStatus::Stopping;
        }
        Ok(())
    }

    /// Start health monitoring task
    fn start_health_monitor(&self) {
        let agents = self.agents.clone();
        let interval = self.config.health_check_interval;
        let running = self.running.clone();

        tokio::spawn(async move {
            while *running.read().await {
                tokio::time::sleep(interval).await;

                for agent_entry in agents.iter() {
                    let mut health = agent_entry.health.write().await;
                    health.last_heartbeat = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
                    health.uptime_seconds = agent_entry.started_at.elapsed().as_secs();
                }
            }
        });
    }

    /// Start task dispatcher
    fn start_task_dispatcher(&self) {
        // TODO: Implement global task dispatcher
    }

    /// Start auto-scaler
    fn start_auto_scaler(&self) {
        // TODO: Implement auto-scaling logic
    }
}

/// Orchestrator statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestratorStats {
    pub total_agents: usize,
    pub active_agents: usize,
    pub total_tasks: u64,
    pub completed_tasks: u64,
    pub failed_tasks: u64,
    pub pending_tasks: usize,
    pub avg_response_time_ms: u64,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            agent_type: AgentType::CodeAssistant,
            max_concurrent_tasks: 5,
            task_timeout_seconds: 300,
            ai_model: "gpt-4".to_string(),
            settings: HashMap::new(),
            resource_limits: ResourceLimits::default(),
            auto_restart: true,
            heartbeat_interval: 30,
        }
    }
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_memory_mb: 1024, // 1GB
            max_cpu_percent: 80.0,
            max_queue_size: 100,
            max_execution_time: 600, // 10 minutes
        }
    }
}

impl AgentTask {
    /// Create a new agent task
    pub fn new(task_type: String, payload: serde_json::Value) -> Self {
        Self {
            id: Uuid::new_v4(),
            task_type,
            priority: 100, // Normal priority
            payload,
            project_id: None,
            user_id: None,
            deadline: None,
            metadata: HashMap::new(),
            created_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            retry_count: 0,
        }
    }

    /// Set task priority
    pub fn with_priority(mut self, priority: u32) -> Self {
        self.priority = priority;
        self
    }

    /// Set project context
    pub fn with_project(mut self, project_id: ProjectId) -> Self {
        self.project_id = Some(project_id);
        self
    }

    /// Set user context
    pub fn with_user(mut self, user_id: UserId) -> Self {
        self.user_id = Some(user_id);
        self
    }
}
