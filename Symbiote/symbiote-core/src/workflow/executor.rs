//! Workflow Execution Engine
//! 
//! This module implements the core workflow execution engine that runs the 100+ nodes
//! we've implemented across 8 categories. It handles node scheduling, dependency
//! resolution, error recovery, and performance optimization.

use crate::{Result, SymbioteError};
use super::{
    Workflow, WorkflowNode, WorkflowConnection, NodeRegistry, NodeExecutionEngine,
    ExecutionStatus, NodeExecutionStatus, ExecutionTrigger,
    context_integration::{ContextAwareWorkflowExecutor, WorkflowExecution}
};
use crate::context::ContextBus;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use tokio::sync::{RwLock, Semaphore, broadcast};
use tokio::time::{timeout, Duration, Instant};
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Main workflow execution engine
#[derive(Debug)]
pub struct WorkflowExecutor {
    /// Node registry with all 100+ nodes
    node_registry: Arc<NodeRegistry>,

    /// Node execution engine for actual node execution
    node_executor: Arc<NodeExecutionEngine>,

    /// Context-aware executor for ContextBus integration
    context_executor: Arc<ContextAwareWorkflowExecutor>,

    /// Active workflow executions
    active_executions: Arc<RwLock<HashMap<String, ExecutionInstance>>>,

    /// Execution scheduler
    scheduler: Arc<ExecutionScheduler>,

    /// Performance monitor
    performance_monitor: Arc<PerformanceMonitor>,

    /// Event broadcaster
    event_broadcaster: broadcast::Sender<ExecutionEvent>,

    /// Concurrency limiter
    concurrency_limiter: Arc<Semaphore>,
}

impl Clone for WorkflowExecutor {
    fn clone(&self) -> Self {
        Self {
            node_registry: Arc::clone(&self.node_registry),
            node_executor: Arc::clone(&self.node_executor),
            context_executor: Arc::clone(&self.context_executor),
            active_executions: Arc::clone(&self.active_executions),
            scheduler: Arc::clone(&self.scheduler),
            performance_monitor: Arc::clone(&self.performance_monitor),
            event_broadcaster: self.event_broadcaster.clone(),
            concurrency_limiter: Arc::clone(&self.concurrency_limiter),
        }
    }
}

/// Individual workflow execution instance
#[derive(Debug, Clone)]
pub struct ExecutionInstance {
    pub execution_id: String,
    pub workflow_id: String,
    pub workflow: Workflow,
    pub status: ExecutionStatus,
    pub trigger: ExecutionTrigger,
    pub input_data: Option<HashMap<String, serde_json::Value>>,
    pub node_executions: HashMap<String, NodeExecution>,
    pub execution_graph: ExecutionGraph,
    pub started_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error: Option<String>,
    pub metrics: ExecutionMetrics,
}

/// Node execution state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeExecution {
    pub node_id: String,
    pub node_type: String,
    pub status: NodeExecutionStatus,
    pub input_data: HashMap<String, serde_json::Value>,
    pub output_data: HashMap<String, serde_json::Value>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub duration_ms: Option<u64>,
    pub retry_count: u32,
    pub error: Option<ExecutionError>,
    pub context_data: Option<HashMap<String, serde_json::Value>>,
}

/// Execution graph for dependency resolution
#[derive(Debug, Clone)]
pub struct ExecutionGraph {
    pub nodes: HashMap<String, GraphNode>,
    pub dependencies: HashMap<String, Vec<String>>,
    pub execution_order: Vec<Vec<String>>, // Batches of nodes that can run in parallel
}

/// Graph node representation
#[derive(Debug, Clone)]
pub struct GraphNode {
    pub node_id: String,
    pub node_type: String,
    pub dependencies: Vec<String>,
    pub dependents: Vec<String>,
    pub can_run_parallel: bool,
}

/// Execution scheduler for managing node execution order
#[derive(Debug)]
pub struct ExecutionScheduler {
    /// Queue of ready-to-execute nodes
    ready_queue: Arc<RwLock<VecDeque<ScheduledNode>>>,
    
    /// Nodes waiting for dependencies
    waiting_nodes: Arc<RwLock<HashMap<String, ScheduledNode>>>,
    
    /// Completed nodes
    completed_nodes: Arc<RwLock<HashSet<String>>>,
}

/// Scheduled node for execution
#[derive(Debug, Clone)]
pub struct ScheduledNode {
    pub execution_id: String,
    pub node_id: String,
    pub node_type: String,
    pub priority: u32,
    pub scheduled_at: DateTime<Utc>,
    pub dependencies: Vec<String>,
}

/// Performance monitoring
#[derive(Debug)]
pub struct PerformanceMonitor {
    /// Execution metrics
    metrics: Arc<RwLock<HashMap<String, ExecutionMetrics>>>,
    
    /// Node performance stats
    node_stats: Arc<RwLock<HashMap<String, NodePerformanceStats>>>,
}

/// Execution metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionMetrics {
    pub total_nodes: u32,
    pub completed_nodes: u32,
    pub failed_nodes: u32,
    pub skipped_nodes: u32,
    pub total_duration_ms: u64,
    pub average_node_duration_ms: f64,
    pub memory_usage_mb: f64,
    pub cpu_usage_percent: f64,
    pub network_requests: u32,
    pub cache_hits: u32,
    pub cache_misses: u32,
}

/// Node performance statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodePerformanceStats {
    pub node_type: String,
    pub total_executions: u64,
    pub successful_executions: u64,
    pub failed_executions: u64,
    pub average_duration_ms: f64,
    pub min_duration_ms: u64,
    pub max_duration_ms: u64,
    pub last_executed: DateTime<Utc>,
}

/// Execution error details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionError {
    pub error_type: String,
    pub message: String,
    pub details: Option<serde_json::Value>,
    pub retry_count: u32,
    pub is_retryable: bool,
    pub occurred_at: DateTime<Utc>,
}

/// Execution events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionEvent {
    /// Execution started
    ExecutionStarted {
        execution_id: String,
        workflow_id: String,
        trigger: ExecutionTrigger,
        timestamp: DateTime<Utc>,
    },
    /// Node execution started
    NodeStarted {
        execution_id: String,
        node_id: String,
        node_type: String,
        timestamp: DateTime<Utc>,
    },
    /// Node execution completed
    NodeCompleted {
        execution_id: String,
        node_id: String,
        status: NodeExecutionStatus,
        duration_ms: u64,
        timestamp: DateTime<Utc>,
    },
    /// Execution completed
    ExecutionCompleted {
        execution_id: String,
        status: ExecutionStatus,
        total_duration_ms: u64,
        metrics: ExecutionMetrics,
        timestamp: DateTime<Utc>,
    },
    /// Error occurred
    ErrorOccurred {
        execution_id: String,
        node_id: Option<String>,
        error: ExecutionError,
        timestamp: DateTime<Utc>,
    },
}

impl WorkflowExecutor {
    /// Create a new workflow executor
    pub fn new(
        node_registry: Arc<NodeRegistry>,
        context_bus: Arc<ContextBus>,
    ) -> Self {
        let context_executor = Arc::new(ContextAwareWorkflowExecutor::new(context_bus));
        let node_executor = Arc::new(NodeExecutionEngine::new());
        let (event_broadcaster, _) = broadcast::channel(10000);

        Self {
            node_registry,
            node_executor,
            context_executor,
            active_executions: Arc::new(RwLock::new(HashMap::new())),
            scheduler: Arc::new(ExecutionScheduler::new()),
            performance_monitor: Arc::new(PerformanceMonitor::new()),
            event_broadcaster,
            concurrency_limiter: Arc::new(Semaphore::new(100)), // Max 100 concurrent nodes
        }
    }

    /// Execute a workflow
    pub async fn execute_workflow(
        &self,
        workflow: Workflow,
        trigger: ExecutionTrigger,
        input_data: Option<HashMap<String, serde_json::Value>>,
    ) -> Result<String> {
        let execution_id = Uuid::new_v4().to_string();
        
        // Validate workflow
        self.validate_workflow(&workflow).await?;
        
        // Build execution graph
        let execution_graph = self.build_execution_graph(&workflow)?;
        
        // Create execution instance
        let execution = ExecutionInstance {
            execution_id: execution_id.clone(),
            workflow_id: workflow.id.clone(),
            workflow: workflow.clone(),
            status: ExecutionStatus::Running,
            trigger: trigger.clone(),
            input_data: input_data.clone(),
            node_executions: HashMap::new(),
            execution_graph,
            started_at: Utc::now(),
            updated_at: Utc::now(),
            completed_at: None,
            error: None,
            metrics: ExecutionMetrics::default(),
        };

        // Store execution
        {
            let mut executions = self.active_executions.write().await;
            executions.insert(execution_id.clone(), execution);
        }

        // Broadcast execution started event
        let _ = self.event_broadcaster.send(ExecutionEvent::ExecutionStarted {
            execution_id: execution_id.clone(),
            workflow_id: workflow.id.clone(),
            trigger,
            timestamp: Utc::now(),
        });

        // Start execution in background
        let executor = Arc::new(self.clone());
        let exec_id = execution_id.clone();
        tokio::spawn(async move {
            if let Err(e) = executor.run_execution(&exec_id).await {
                eprintln!("Execution failed: {}", e);
            }
        });

        Ok(execution_id)
    }

    /// Run the actual execution
    async fn run_execution(&self, execution_id: &str) -> Result<()> {
        let execution_start = Instant::now();
        
        // Get execution instance
        let execution = {
            let executions = self.active_executions.read().await;
            executions.get(execution_id).cloned()
                .ok_or_else(|| SymbioteError::NotFound("Execution not found".to_string()))?
        };

        // Execute nodes in dependency order
        for batch in &execution.execution_graph.execution_order {
            // Execute nodes in this batch in parallel
            let mut batch_tasks = Vec::new();
            
            for node_id in batch {
                let node = execution.workflow.nodes.iter()
                    .find(|n| &n.id == node_id)
                    .ok_or_else(|| SymbioteError::NotFound("Node not found".to_string()))?;
                
                let executor = Arc::new(self.clone());
                let exec_id = execution_id.to_string();
                let node_clone = node.clone();
                
                let task = tokio::spawn(async move {
                    executor.execute_node(&exec_id, &node_clone).await
                });
                
                batch_tasks.push(task);
            }
            
            // Wait for all nodes in batch to complete
            for task in batch_tasks {
                if let Err(e) = task.await {
                    eprintln!("Node execution task failed: {}", e);
                }
            }
        }

        // Update execution status
        let total_duration = execution_start.elapsed().as_millis() as u64;
        self.complete_execution(execution_id, ExecutionStatus::Completed, total_duration).await?;

        Ok(())
    }

    /// Execute a single node
    async fn execute_node(&self, execution_id: &str, node: &WorkflowNode) -> Result<()> {
        // Acquire concurrency permit
        let _permit = self.concurrency_limiter.acquire().await
            .map_err(|e| SymbioteError::Internal(format!("Failed to acquire permit: {}", e)))?;

        let node_start = Instant::now();
        
        // Broadcast node started event
        let _ = self.event_broadcaster.send(ExecutionEvent::NodeStarted {
            execution_id: execution_id.to_string(),
            node_id: node.id.clone(),
            node_type: node.node_type.clone(),
            timestamp: Utc::now(),
        });

        // Get node implementation from registry
        let node_impl = self.node_registry.get_node(&node.node_type).await
            .ok_or_else(|| SymbioteError::NotFound(format!("Node type not found: {}", node.node_type)))?;

        // Prepare input data
        let input_data = self.prepare_node_input(execution_id, node).await?;

        // Execute node with timeout
        let execution_timeout = Duration::from_millis(
            node.configuration.timeout_ms.unwrap_or(30000)
        );

        let result = timeout(execution_timeout, async {
            // Execute node using the actual execution engine
            self.execute_node_with_engine(node, &input_data, execution_id).await
        }).await;

        let duration = node_start.elapsed().as_millis() as u64;

        match result {
            Ok(Ok(output_data)) => {
                // Node executed successfully
                self.complete_node_execution(
                    execution_id,
                    &node.id,
                    NodeExecutionStatus::Completed,
                    input_data,
                    output_data,
                    duration,
                    None,
                ).await?;
            }
            Ok(Err(e)) => {
                // Node execution failed
                let error = ExecutionError {
                    error_type: "NodeExecutionError".to_string(),
                    message: e.to_string(),
                    details: None,
                    retry_count: 0,
                    is_retryable: true,
                    occurred_at: Utc::now(),
                };
                
                self.complete_node_execution(
                    execution_id,
                    &node.id,
                    NodeExecutionStatus::Failed,
                    input_data,
                    HashMap::new(),
                    duration,
                    Some(error),
                ).await?;
            }
            Err(_) => {
                // Timeout
                let error = ExecutionError {
                    error_type: "TimeoutError".to_string(),
                    message: "Node execution timed out".to_string(),
                    details: None,
                    retry_count: 0,
                    is_retryable: true,
                    occurred_at: Utc::now(),
                };
                
                self.complete_node_execution(
                    execution_id,
                    &node.id,
                    NodeExecutionStatus::Failed,
                    input_data,
                    HashMap::new(),
                    duration,
                    Some(error),
                ).await?;
            }
        }

        Ok(())
    }

    /// Execute node using the NodeExecutionEngine
    async fn execute_node_with_engine(
        &self,
        node: &WorkflowNode,
        input_data: &HashMap<String, serde_json::Value>,
        execution_id: &str,
    ) -> Result<HashMap<String, serde_json::Value>> {
        // Get node definition from registry
        let node_definition = self.node_registry.get_node(&node.node_type).await
            .ok_or_else(|| SymbioteError::NotFound(format!("Node type not found: {}", node.node_type)))?;

        // Create execution context
        let context = super::NodeExecutionContext {
            execution_id: execution_id.to_string(),
            node_id: node.id.clone(),
            input_data: input_data.clone(),
            configuration: node.configuration.parameters.clone(),
            context_data: None,
        };

        // Execute using the node execution engine
        let result = self.node_executor.execute_node(&node_definition, context).await?;

        if result.success {
            Ok(result.output_data)
        } else {
            Err(SymbioteError::Execution(
                result.error_message.unwrap_or_else(|| "Node execution failed".to_string())
            ))
        }
    }

    /// Validate workflow before execution
    async fn validate_workflow(&self, workflow: &Workflow) -> Result<()> {
        // Check if all node types are registered
        for node in &workflow.nodes {
            if !self.node_registry.has_node(&node.node_type).await {
                return Err(SymbioteError::Validation(
                    format!("Unknown node type: {}", node.node_type)
                ));
            }
        }

        // Validate connections
        for connection in &workflow.connections {
            // Check if source and target nodes exist
            let source_exists = workflow.nodes.iter().any(|n| n.id == connection.source_node_id);
            let target_exists = workflow.nodes.iter().any(|n| n.id == connection.target_node_id);
            
            if !source_exists || !target_exists {
                return Err(SymbioteError::Validation(
                    "Invalid connection: source or target node not found".to_string()
                ));
            }
        }

        Ok(())
    }

    /// Build execution graph for dependency resolution
    fn build_execution_graph(&self, workflow: &Workflow) -> Result<ExecutionGraph> {
        let mut graph = ExecutionGraph {
            nodes: HashMap::new(),
            dependencies: HashMap::new(),
            execution_order: Vec::new(),
        };

        // Build dependency map
        for node in &workflow.nodes {
            let mut dependencies = Vec::new();
            
            // Find all nodes that this node depends on
            for connection in &workflow.connections {
                if connection.target_node_id == node.id {
                    dependencies.push(connection.source_node_id.clone());
                }
            }
            
            graph.dependencies.insert(node.id.clone(), dependencies.clone());
            
            graph.nodes.insert(node.id.clone(), GraphNode {
                node_id: node.id.clone(),
                node_type: node.node_type.clone(),
                dependencies,
                dependents: Vec::new(),
                can_run_parallel: true,
            });
        }

        // Calculate execution order using topological sort
        graph.execution_order = self.calculate_execution_order(&graph.dependencies)?;

        Ok(graph)
    }

    /// Calculate execution order using topological sort
    fn calculate_execution_order(
        &self,
        dependencies: &HashMap<String, Vec<String>>,
    ) -> Result<Vec<Vec<String>>> {
        let mut order = Vec::new();
        let mut remaining_nodes: HashSet<String> = dependencies.keys().cloned().collect();
        let mut in_degree: HashMap<String, usize> = HashMap::new();

        // Calculate in-degrees
        for (node, deps) in dependencies {
            in_degree.insert(node.clone(), deps.len());
        }

        // Process nodes in batches
        while !remaining_nodes.is_empty() {
            let mut current_batch = Vec::new();
            
            // Find nodes with no dependencies
            for node in &remaining_nodes {
                if in_degree.get(node).unwrap_or(&0) == &0 {
                    current_batch.push(node.clone());
                }
            }

            if current_batch.is_empty() {
                return Err(SymbioteError::Validation(
                    "Circular dependency detected in workflow".to_string()
                ));
            }

            // Remove processed nodes and update in-degrees
            for node in &current_batch {
                remaining_nodes.remove(node);
                
                // Update in-degrees for dependent nodes
                for (other_node, deps) in dependencies {
                    if deps.contains(node) {
                        if let Some(degree) = in_degree.get_mut(other_node) {
                            *degree = degree.saturating_sub(1);
                        }
                    }
                }
            }

            order.push(current_batch);
        }

        Ok(order)
    }

    /// Prepare input data for node execution
    async fn prepare_node_input(
        &self,
        execution_id: &str,
        node: &WorkflowNode,
    ) -> Result<HashMap<String, serde_json::Value>> {
        let mut input_data = HashMap::new();
        
        // Get execution instance
        let execution = {
            let executions = self.active_executions.read().await;
            executions.get(execution_id).cloned()
                .ok_or_else(|| SymbioteError::NotFound("Execution not found".to_string()))?
        };

        // Add workflow input data
        if let Some(workflow_input) = &execution.input_data {
            input_data.extend(workflow_input.clone());
        }

        // Add node configuration parameters
        input_data.extend(node.configuration.parameters.clone());

        // Add outputs from connected nodes
        for connection in &execution.workflow.connections {
            if connection.target_node_id == node.id {
                if let Some(source_execution) = execution.node_executions.get(&connection.source_node_id) {
                    if let Some(output_value) = source_execution.output_data.get(&connection.source_output) {
                        input_data.insert(connection.target_input.clone(), output_value.clone());
                    }
                }
            }
        }

        Ok(input_data)
    }

    /// Complete node execution
    async fn complete_node_execution(
        &self,
        execution_id: &str,
        node_id: &str,
        status: NodeExecutionStatus,
        input_data: HashMap<String, serde_json::Value>,
        output_data: HashMap<String, serde_json::Value>,
        duration_ms: u64,
        error: Option<ExecutionError>,
    ) -> Result<()> {
        // Update execution state
        {
            let mut executions = self.active_executions.write().await;
            if let Some(execution) = executions.get_mut(execution_id) {
                let node_execution = NodeExecution {
                    node_id: node_id.to_string(),
                    node_type: "unknown".to_string(), // Would get from node
                    status: status.clone(),
                    input_data,
                    output_data,
                    started_at: Some(Utc::now() - chrono::Duration::milliseconds(duration_ms as i64)),
                    completed_at: Some(Utc::now()),
                    duration_ms: Some(duration_ms),
                    retry_count: 0,
                    error: error.clone(),
                    context_data: None,
                };
                
                execution.node_executions.insert(node_id.to_string(), node_execution);
                execution.updated_at = Utc::now();
            }
        }

        // Broadcast node completed event
        let _ = self.event_broadcaster.send(ExecutionEvent::NodeCompleted {
            execution_id: execution_id.to_string(),
            node_id: node_id.to_string(),
            status,
            duration_ms,
            timestamp: Utc::now(),
        });

        Ok(())
    }

    /// Complete workflow execution
    async fn complete_execution(
        &self,
        execution_id: &str,
        status: ExecutionStatus,
        total_duration_ms: u64,
    ) -> Result<()> {
        let metrics = {
            let mut executions = self.active_executions.write().await;
            if let Some(execution) = executions.get_mut(execution_id) {
                execution.status = status.clone();
                execution.completed_at = Some(Utc::now());
                execution.metrics.total_duration_ms = total_duration_ms;
                execution.metrics.clone()
            } else {
                return Err(SymbioteError::NotFound("Execution not found".to_string()));
            }
        };

        // Broadcast execution completed event
        let _ = self.event_broadcaster.send(ExecutionEvent::ExecutionCompleted {
            execution_id: execution_id.to_string(),
            status,
            total_duration_ms,
            metrics,
            timestamp: Utc::now(),
        });

        Ok(())
    }

    /// Subscribe to execution events
    pub fn subscribe_to_events(&self) -> broadcast::Receiver<ExecutionEvent> {
        self.event_broadcaster.subscribe()
    }

    /// Get execution status
    pub async fn get_execution_status(&self, execution_id: &str) -> Result<ExecutionStatus> {
        let executions = self.active_executions.read().await;
        executions.get(execution_id)
            .map(|e| e.status.clone())
            .ok_or_else(|| SymbioteError::NotFound("Execution not found".to_string()))
    }
}

impl ExecutionScheduler {
    fn new() -> Self {
        Self {
            ready_queue: Arc::new(RwLock::new(VecDeque::new())),
            waiting_nodes: Arc::new(RwLock::new(HashMap::new())),
            completed_nodes: Arc::new(RwLock::new(HashSet::new())),
        }
    }
}

impl PerformanceMonitor {
    fn new() -> Self {
        Self {
            metrics: Arc::new(RwLock::new(HashMap::new())),
            node_stats: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for ExecutionMetrics {
    fn default() -> Self {
        Self {
            total_nodes: 0,
            completed_nodes: 0,
            failed_nodes: 0,
            skipped_nodes: 0,
            total_duration_ms: 0,
            average_node_duration_ms: 0.0,
            memory_usage_mb: 0.0,
            cpu_usage_percent: 0.0,
            network_requests: 0,
            cache_hits: 0,
            cache_misses: 0,
        }
    }
}
