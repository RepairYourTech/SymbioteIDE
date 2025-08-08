use crate::agent_runtime::{AgentType, AgentConfig, AgentContext, AgentResult, AgentError, AgentRuntime};
use crate::agent_communication::{AgentMessage, MessageType, CommunicationProtocol};
use crate::context_compression::{ContextCompressor, CompressionSettings};
// use crate::specialized_agents::{SpecializedAgent, SpecializedAgentRegistry, AgentRegistry}; // Disabled

// Stub types for disabled specialized agents
pub struct SpecializedAgent;
pub struct SpecializedAgentRegistry;
pub struct AgentRegistry;
use crate::task_manager::{TaskManager, AgentAssignmentRequest, AssignmentPriority, ModelInfo, CollaborationMode};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{RwLock, Mutex, Semaphore};
use tokio::time::timeout;
use serde::{Serialize, Deserialize};
use uuid::Uuid;

/// Task priority levels for orchestration
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum TaskPriority {
    Low = 1,
    Medium = 2,
    Normal = 2,  // Alias for Medium for compatibility
    High = 3,
    Critical = 4,
}

/// Task execution status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    Pending,
    Assigned,
    InProgress,
    Completed,
    Failed,
    Cancelled,
}

/// Task dependency type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DependencyType {
    Sequential,    // Must complete before next task
    Parallel,      // Can run in parallel
    Conditional,   // Depends on result of previous task
    Resource,      // Shares resources, may conflict
    FinishToStart, // Traditional project management dependency
}

/// Task execution tracking
#[derive(Debug, Clone)]
pub struct TaskExecution {
    pub task: OrchestratedTask,
    pub started_at: Instant,
    pub agent_id: String,
    pub status: TaskStatus,
}

/// Orchestrated task definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestratedTask {
    pub id: String,
    pub description: String,
    pub agent_type: AgentType,
    pub priority: TaskPriority,
    pub status: TaskStatus,
    pub context: String,
    pub dependencies: Vec<TaskDependency>,
    pub estimated_duration_ms: Option<u64>,
    pub created_at_ms: u64,
    pub assigned_at_ms: Option<u64>,
    pub completed_at_ms: Option<u64>,
    pub result: Option<AgentResult<String>>,
    pub error: Option<String>,
}

/// Task dependency definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskDependency {
    pub id: String,
    pub task_id: String,
    pub depends_on_task_id: String,
    pub dependency_type: DependencyType,
    pub required_output: Option<String>,
    pub created_at: u64,
    pub notes: Option<String>,
}

/// Conflict detection and resolution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskConflict {
    pub task_ids: Vec<String>,
    pub conflict_type: ConflictType,
    pub severity: ConflictSeverity,
    pub resolution_strategy: ResolutionStrategy,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConflictType {
    ResourceContention,  // Same file/resource
    OutputModification,  // Both modify same output
    DependencyLoop,      // Circular dependencies
    AgentBoundary,       // Cross-agent boundary violation
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ConflictSeverity {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResolutionStrategy {
    Sequential,          // Run tasks sequentially
    Partition,          // Partition resources
    Merge,              // Merge compatible tasks
    Cancel,             // Cancel conflicting task
    UserDecision,       // Escalate to user
}

/// Agent orchestrator for intelligent parallel execution
pub struct AgentOrchestrator {
    runtime: Arc<AgentRuntime>,
    communication: Arc<CommunicationProtocol>,
    compressor: Arc<ContextCompressor>,
    agent_registry: Arc<AgentRegistry>,
    
    task_queue: Arc<RwLock<Vec<OrchestratedTask>>>,
    active_tasks: Arc<RwLock<HashMap<String, TaskExecution>>>,
    completed_tasks: Arc<RwLock<HashMap<String, OrchestratedTask>>>,
    task_dependencies: Arc<RwLock<HashMap<String, Vec<String>>>>,
    orchestration_stats: Arc<RwLock<OrchestrationStats>>,
    task_manager: Arc<RwLock<Option<Arc<TaskManager>>>>,
    
    execution_semaphore: Arc<Semaphore>,
    conflict_detector: Arc<ConflictDetector>,
    dependency_resolver: Arc<DependencyResolver>,
    
    // Configuration
    max_parallel_tasks: usize,
    task_timeout: Duration,
    conflict_resolution_enabled: bool,
}

impl AgentOrchestrator {
    pub fn new(
        runtime: Arc<AgentRuntime>,
        communication: Arc<CommunicationProtocol>,
        compressor: Arc<ContextCompressor>,
        agent_registry: Arc<AgentRegistry>,
        max_parallel_tasks: usize,
    ) -> Self {
        let execution_semaphore = Arc::new(Semaphore::new(max_parallel_tasks));
        
        Self {
            runtime,
            communication,
            compressor,
            agent_registry,
            task_queue: Arc::new(RwLock::new(Vec::new())),
            active_tasks: Arc::new(RwLock::new(HashMap::new())),
            completed_tasks: Arc::new(RwLock::new(HashMap::new())),
            task_dependencies: Arc::new(RwLock::new(HashMap::new())),
            orchestration_stats: Arc::new(RwLock::new(OrchestrationStats::default())),
            task_manager: Arc::new(RwLock::new(None)),
            execution_semaphore,
            conflict_detector: Arc::new(ConflictDetector::new()),
            dependency_resolver: Arc::new(DependencyResolver::new()),
            max_parallel_tasks,
            task_timeout: Duration::from_secs(300), // 5 minutes default
            conflict_resolution_enabled: true,
        }
    }
    
    /// Set task manager reference for integration
    pub async fn set_task_manager(&self, task_manager: Arc<TaskManager>) {
        let mut tm = self.task_manager.write().await;
        *tm = Some(task_manager);
    }
    
    /// Initialize the orchestrator with specialized agents
    pub async fn initialize(&self) -> Result<(), AgentError> {
        // Register all specialized agents
        // self.register_specialized_agents().await?; // Disabled
        
        // Start the orchestration loop
        self.start_orchestration_loop().await;
        
        Ok(())
    }
    
    /// Submit a new task for orchestration
    pub async fn submit_task(&self, task: OrchestratedTask) -> Result<String, AgentError> {
        let task_id = task.id.clone();
        
        // Validate task
        self.validate_task(&task).await?;
        
        // Add to queue
        let mut queue = self.task_queue.write().await;
        queue.push_back(task);
        
        // Sort queue by priority
        let mut tasks: Vec<_> = queue.drain(..).collect();
        tasks.sort_by(|a, b| b.priority.cmp(&a.priority));
        queue.extend(tasks);
        
        Ok(task_id)
    }
    
    /// Submit multiple related tasks with dependencies
    pub async fn submit_task_batch(&self, tasks: Vec<OrchestratedTask>) -> Result<Vec<String>, AgentError> {
        // Detect conflicts before submission
        let conflicts = self.conflict_detector.detect_conflicts(&tasks).await?;
        
        if !conflicts.is_empty() && self.conflict_resolution_enabled {
            self.resolve_conflicts(conflicts).await?;
        }
        
        // Submit tasks in dependency order
        let ordered_tasks = self.dependency_resolver.resolve_dependencies(tasks).await?;
        let mut task_ids = Vec::new();
        
        for task in ordered_tasks {
            let task_id = self.submit_task(task).await?;
            task_ids.push(task_id);
        }
        
        Ok(task_ids)
    }
    
    /// Get task status
    pub async fn get_task_status(&self, task_id: &str) -> Option<TaskStatus> {
        // Check active tasks first
        if let Some(task) = self.active_tasks.read().await.get(task_id) {
            return Some(task.status.clone());
        }
        
        // Check completed tasks
        if let Some(task) = self.completed_tasks.read().await.get(task_id) {
            return Some(task.status.clone());
        }
        
        // Check queue
        let queue = self.task_queue.read().await;
        for task in queue.iter() {
            if task.id == task_id {
                return Some(task.status.clone());
            }
        }
        
        None
    }
    
    /// Get task result
    pub async fn get_task_result(&self, task_id: &str) -> Option<AgentResult<String>> {
        if let Some(task) = self.completed_tasks.read().await.get(task_id) {
            task.result.clone()
        } else {
            None
        }
    }
    
    /// Cancel a task
    pub async fn cancel_task(&self, task_id: &str) -> Result<(), AgentError> {
        // Try to remove from queue first
        let mut queue = self.task_queue.write().await;
        if let Some(pos) = queue.iter().position(|t| t.id == task_id) {
            let mut task = queue.remove(pos).unwrap();
            task.status = TaskStatus::Cancelled;
            self.completed_tasks.write().await.insert(task_id.to_string(), task);
            return Ok(());
        }
        
        // If task is active, mark for cancellation
        let mut active_tasks = self.active_tasks.write().await;
        if let Some(task) = active_tasks.get_mut(task_id) {
            task.status = TaskStatus::Cancelled;
            // Note: Actual cancellation depends on agent implementation
            return Ok(());
        }
        
        Err(AgentError::TaskNotFound(task_id.to_string()))
    }
    
    /// Get orchestration statistics
    pub async fn get_orchestration_stats(&self) -> OrchestrationStats {
        let queue_size = self.task_queue.read().await.len();
        let active_count = self.active_tasks.read().await.len();
        let completed_count = self.completed_tasks.read().await.len();
        
        let completed_tasks = self.completed_tasks.read().await;
        let success_count = completed_tasks.values()
            .filter(|t| t.status == TaskStatus::Completed)
            .count();
        let failed_count = completed_tasks.values()
            .filter(|t| t.status == TaskStatus::Failed)
            .count();
        
        OrchestrationStats {
            queue_size,
            active_tasks: active_count,
            completed_tasks: completed_count,
            success_rate: if completed_count > 0 {
                success_count as f32 / completed_count as f32
            } else {
                0.0
            },
            failed_tasks: failed_count,
            average_execution_time: self.calculate_average_execution_time().await,
        }
    }
    
    /// Private methods
    
    /*
    async fn register_specialized_agents(&self) -> Result<(), AgentError> {
        let communication = self.communication.clone();
        let compressor = self.compressor.clone();
        
        // Register core agents
        let agents: Vec<Box<dyn SpecializedAgent>> = vec![
            Box::new(crate::specialized_agents::ArchitectAgent::new(
                AgentConfig::default(), communication.clone(), compressor.clone()
            )),
            Box::new(crate::specialized_agents::DeveloperAgent::new(
                AgentConfig::default(), communication.clone(), compressor.clone()
            )),
            Box::new(crate::specialized_agents::TesterAgent::new(
                AgentConfig::default(), communication.clone(), compressor.clone()
            )),
            Box::new(crate::specialized_agents::DebugAgent::new(
                AgentConfig::default(), communication.clone(), compressor.clone()
            )),
            Box::new(crate::specialized_agents::SecurityAgent::new(
                AgentConfig::default(), communication.clone(), compressor.clone()
            )),
            // Language specialists
            Box::new(ReactSpecialistAgent::new(
                AgentConfig::default(), communication.clone(), compressor.clone()
            )),
            Box::new(RustSpecialistAgent::new(
                AgentConfig::default(), communication.clone(), compressor.clone()
            )),
            Box::new(PythonSpecialistAgent::new(
                AgentConfig::default(), communication.clone(), compressor.clone()
            )),
            Box::new(DevOpsSpecialistAgent::new(
                AgentConfig::default(), communication.clone(), compressor.clone()
            )),
            Box::new(PerformanceOptimizerAgent::new(
                AgentConfig::default(), communication.clone(), compressor.clone()
            )),
            Box::new(ResearcherAgent::new(
                AgentConfig::default(), communication.clone(), compressor.clone()
            )),
        ];
        
        for agent in agents {
            self.agent_registry.register_agent(agent).await;
        }
        
        Ok(())
    }
    */
    
    async fn start_orchestration_loop(&self) {
        let orchestrator = self.clone_for_loop();
        
        tokio::spawn(async move {
            loop {
                orchestrator.process_task_queue().await;
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        });
    }
    
    fn clone_for_loop(&self) -> Self {
        // Create a clone for the async loop
        // Note: This is a simplified approach - in practice, you'd use Arc references
        Self {
            runtime: self.runtime.clone(),
            communication: self.communication.clone(),
            compressor: self.compressor.clone(),
            agent_registry: self.agent_registry.clone(),
            task_queue: self.task_queue.clone(),
            active_tasks: self.active_tasks.clone(),
            completed_tasks: self.completed_tasks.clone(),
            execution_semaphore: self.execution_semaphore.clone(),
            conflict_detector: self.conflict_detector.clone(),
            dependency_resolver: self.dependency_resolver.clone(),
            max_parallel_tasks: self.max_parallel_tasks,
            task_timeout: self.task_timeout,
            conflict_resolution_enabled: self.conflict_resolution_enabled,
        }
    }
    
    async fn process_task_queue(&self) {
        // Get next available task
        let task = {
            let mut queue = self.task_queue.write().await;
            if queue.is_empty() {
                return;
            }
            
            // Find a task that can be executed (dependencies satisfied)
            let mut task_index = None;
            for (i, task) in queue.iter().enumerate() {
                if self.are_dependencies_satisfied(&task).await {
                    task_index = Some(i);
                    break;
                }
            }
            
            if let Some(index) = task_index {
                queue.remove(index)
            } else {
                return; // No tasks ready for execution
            }
        };
        
        // Try to acquire execution permit
        if let Ok(permit) = self.execution_semaphore.try_acquire() {
            let task_id = task.id.clone();
            
            // Move task to active
            {
                let mut active_tasks = self.active_tasks.write().await;
                active_tasks.insert(task_id.clone(), task.clone());
            }
            
            // Execute task
            let orchestrator = self.clone_for_loop();
            tokio::spawn(async move {
                orchestrator.execute_task_with_timeout(task).await;
                drop(permit); // Release permit when done
            });
        } else {
            // No permits available, put task back in queue
            let mut queue = self.task_queue.write().await;
            queue.push_front(task);
        }
    }
    
    async fn execute_task_with_timeout(&self, mut task: OrchestratedTask) {
        let task_id = task.id.clone();
        
        // Update task status
        task.status = TaskStatus::InProgress;
        task.assigned_at = Some(Instant::now());
        
        {
            let mut active_tasks = self.active_tasks.write().await;
            active_tasks.insert(task_id.clone(), task.clone());
        }
        
        // Execute with timeout
        let execution_result = timeout(
            self.task_timeout,
            self.execute_single_task(&task)
        ).await;
        
        // Process result
        match execution_result {
            Ok(Ok(result)) => {
                task.status = TaskStatus::Completed;
                task.result = Some(result);
                task.completed_at = Some(Instant::now());
            },
            Ok(Err(error)) => {
                task.status = TaskStatus::Failed;
                task.error = Some(error.to_string());
                task.completed_at = Some(Instant::now());
            },
            Err(_) => {
                task.status = TaskStatus::Failed;
                task.error = Some("Task execution timeout".to_string());
                task.completed_at = Some(Instant::now());
            }
        }
        
        // Move to completed
        {
            let mut active_tasks = self.active_tasks.write().await;
            active_tasks.remove(&task_id);
            
            let mut completed_tasks = self.completed_tasks.write().await;
            completed_tasks.insert(task_id, task);
        }
    }
    
    async fn execute_single_task(&self, task: &OrchestratedTask) -> Result<AgentResult<String>, AgentError> {
        // Find suitable agent
        let agent = self.agent_registry.get_agent(&task.agent_type).await
            .ok_or_else(|| AgentError::AgentNotFound(format!("{:?}", task.agent_type)))?;
        
        // Create context
        let context = AgentContext {
            task_id: task.id.clone(),
            agent_type: task.agent_type.clone(),
            context_data: task.context.clone(),
            max_tokens: 32000, // Default, should be configurable
            priority: task.priority as u8,
        };
        
        // Execute task
        agent.execute_task(context, &task.description).await
    }
    
    async fn validate_task(&self, task: &OrchestratedTask) -> Result<(), AgentError> {
        // Validate agent type exists
        let agents = self.agent_registry.list_agents().await;
        if !agents.contains(&task.agent_type) {
            return Err(AgentError::AgentNotFound(format!("{:?}", task.agent_type)));
        }
        
        // Validate dependencies exist
        for dep in &task.dependencies {
            // Check if dependency task exists in system
            // This is a simplified check
        }
        
        Ok(())
    }
    
    async fn are_dependencies_satisfied(&self, task: &OrchestratedTask) -> bool {
        for dependency in &task.dependencies {
            match dependency.dependency_type {
                DependencyType::Sequential => {
                    // Check if dependency task is completed
                    if let Some(dep_task) = self.completed_tasks.read().await.get(&dependency.task_id) {
                        if dep_task.status != TaskStatus::Completed {
                            return false;
                        }
                    } else {
                        return false; // Dependency not found or not completed
                    }
                },
                DependencyType::Parallel => {
                    // Parallel tasks don't block each other
                    continue;
                },
                DependencyType::Conditional => {
                    // Check if dependency completed with required output
                    if let Some(dep_task) = self.completed_tasks.read().await.get(&dependency.task_id) {
                        if dep_task.status != TaskStatus::Completed {
                            return false;
                        }
                        // Additional logic to check required output
                    } else {
                        return false;
                    }
                },
                DependencyType::Resource => {
                    // Check for resource conflicts
                    let active_tasks = self.active_tasks.read().await;
                    for active_task in active_tasks.values() {
                        if self.tasks_conflict(task, active_task) {
                            return false;
                        }
                    }
                }
            }
        }
        
        true
    }
    
    fn tasks_conflict(&self, task1: &OrchestratedTask, task2: &OrchestratedTask) -> bool {
        // Simplified conflict detection
        // In practice, this would analyze file access, output modifications, etc.
        false
    }
    
    async fn resolve_conflicts(&self, conflicts: Vec<TaskConflict>) -> Result<(), AgentError> {
        for conflict in conflicts {
            match conflict.resolution_strategy {
                ResolutionStrategy::Sequential => {
                    // Modify tasks to run sequentially
                    self.make_tasks_sequential(&conflict.task_ids).await?;
                },
                ResolutionStrategy::Partition => {
                    // Partition resources between tasks
                    self.partition_task_resources(&conflict.task_ids).await?;
                },
                ResolutionStrategy::Merge => {
                    // Merge compatible tasks
                    self.merge_tasks(&conflict.task_ids).await?;
                },
                ResolutionStrategy::Cancel => {
                    // Cancel lower priority task
                    self.cancel_lower_priority_task(&conflict.task_ids).await?;
                },
                ResolutionStrategy::UserDecision => {
                    // Escalate to user (would trigger UI notification)
                    return Err(AgentError::ConflictRequiresUserDecision(conflict));
                }
            }
        }
        
        Ok(())
    }
    
    async fn make_tasks_sequential(&self, task_ids: &[String]) -> Result<(), AgentError> {
        // Implementation would modify task dependencies
        Ok(())
    }
    
    async fn partition_task_resources(&self, task_ids: &[String]) -> Result<(), AgentError> {
        // Implementation would modify task contexts to avoid conflicts
        Ok(())
    }
    
    async fn merge_tasks(&self, task_ids: &[String]) -> Result<(), AgentError> {
        // Implementation would combine compatible tasks
        Ok(())
    }
    
    async fn cancel_lower_priority_task(&self, task_ids: &[String]) -> Result<(), AgentError> {
        // Implementation would cancel the task with lowest priority
        Ok(())
    }
    
    async fn calculate_average_execution_time(&self) -> Duration {
        let completed_tasks = self.completed_tasks.read().await;
        let mut total_duration = Duration::ZERO;
        let mut count = 0;
        
        for task in completed_tasks.values() {
            if let (Some(assigned), Some(completed)) = (task.assigned_at, task.completed_at) {
                total_duration += completed.duration_since(assigned);
                count += 1;
            }
        }
        
        if count > 0 {
            total_duration / count as u32
        } else {
            Duration::ZERO
        }
    }

    // Missing methods for PA system integration
    pub async fn get_available_agents(&self) -> Vec<AgentType> {
        // Stub implementation - would return actual available agents
        vec![
            AgentType::Orchestrator,
            AgentType::Architect,
            AgentType::Developer,
            AgentType::RustSpecialist,
            AgentType::ReactSpecialist,
            AgentType::Debug,
        ]
    }

    pub async fn register_agent(&self, agent_type: AgentType, _capabilities: Vec<String>) -> Result<(), AgentError> {
        // Stub implementation - would register agent with capabilities
        println!("Registering agent: {:?}", agent_type);
        Ok(())
    }
}

/// Conflict detection system
pub struct ConflictDetector {
    // Conflict detection logic
}

impl ConflictDetector {
    pub fn new() -> Self {
        Self {}
    }
    
    pub async fn detect_conflicts(&self, tasks: &[OrchestratedTask]) -> Result<Vec<TaskConflict>, AgentError> {
        let mut conflicts = Vec::new();
        
        // Analyze tasks for potential conflicts
        for i in 0..tasks.len() {
            for j in (i + 1)..tasks.len() {
                if let Some(conflict) = self.analyze_task_pair(&tasks[i], &tasks[j]).await {
                    conflicts.push(conflict);
                }
            }
        }
        
        Ok(conflicts)
    }
    
    async fn analyze_task_pair(&self, task1: &OrchestratedTask, task2: &OrchestratedTask) -> Option<TaskConflict> {
        // Simplified conflict detection logic
        None
    }
}

/// Dependency resolution system
pub struct DependencyResolver {
    // Dependency resolution logic
}

impl DependencyResolver {
    pub fn new() -> Self {
        Self {}
    }
    
    pub async fn resolve_dependencies(&self, tasks: Vec<OrchestratedTask>) -> Result<Vec<OrchestratedTask>, AgentError> {
        // Topological sort based on dependencies
        // Simplified implementation
        Ok(tasks)
    }
}

/// Orchestration statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestrationStats {
    pub queue_size: usize,
    pub active_tasks: usize,
    pub completed_tasks: usize,
    pub success_rate: f32,
    pub failed_tasks: usize,
    pub average_execution_time: Duration,
}

impl Default for OrchestrationStats {
    fn default() -> Self {
        Self {
            queue_size: 0,
            active_tasks: 0,
            completed_tasks: 0,
            success_rate: 0.0,
            failed_tasks: 0,
            average_execution_time: Duration::ZERO,
        }
    }
}

/// Custom error types for orchestration
#[derive(Debug, thiserror::Error)]
pub enum OrchestrationError {
    #[error("Task conflict requires user decision: {0:?}")]
    ConflictRequiresUserDecision(TaskConflict),
    
    #[error("Dependency cycle detected in tasks: {0:?}")]
    DependencyCycle(Vec<String>),
    
    #[error("Resource exhausted: {0}")]
    ResourceExhausted(String),
}

// Extend AgentError to include orchestration errors
impl From<OrchestrationError> for AgentError {
    fn from(err: OrchestrationError) -> Self {
        match err {
            OrchestrationError::ConflictRequiresUserDecision(conflict) => {
                AgentError::ConflictRequiresUserDecision(conflict)
            },
            _ => AgentError::OrchestrationError(err.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_task_submission() {
        let runtime = Arc::new(AgentRuntime::new());
        let communication = Arc::new(CommunicationProtocol::new());
        let compressor = Arc::new(ContextCompressor::new(CompressionSettings::default()));
        
        let orchestrator = AgentOrchestrator::new(runtime, communication, compressor, 4);
        
        let task = OrchestratedTask {
            id: Uuid::new_v4().to_string(),
            description: "Test task".to_string(),
            agent_type: AgentType::Developer,
            priority: TaskPriority::Normal,
            status: TaskStatus::Pending,
            context: "Test context".to_string(),
            dependencies: vec![],
            estimated_duration: Some(Duration::from_secs(60)),
            created_at: Instant::now(),
            assigned_at: None,
            completed_at: None,
            result: None,
            error: None,
        };
        
        let task_id = orchestrator.submit_task(task).await.unwrap();
        assert!(!task_id.is_empty());
    }
}
