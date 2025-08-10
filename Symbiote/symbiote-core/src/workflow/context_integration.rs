//! Workflow-Context Integration System
//! 
//! This module integrates the Visual Workflow Builder with the ContextBus system,
//! enabling workflows to read from and write to the global context, react to
//! context changes, and coordinate with other IDE components.

use crate::{Result, SymbioteError};
use crate::context::{ContextBus, ContextUpdate, SystemId, SystemContext, ContextUpdateType, UpdatePriority};
use super::{Workflow, WorkflowEvent, WorkflowNode, ExecutionTrigger, NodeExecutionStatus};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::{RwLock, broadcast};
use chrono::{DateTime, Utc};
use uuid::Uuid;
use serde_json;

/// Context data structure for workflow integration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextData {
    pub data: HashMap<String, serde_json::Value>,
    pub metadata: HashMap<String, String>,
    pub timestamp: DateTime<Utc>,
}

/// Context query for retrieving specific context data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextQuery {
    pub paths: Vec<String>,
    pub include_metadata: bool,
    pub max_depth: Option<u32>,
}

/// Context-aware workflow executor that integrates with ContextBus
#[derive(Debug)]
pub struct ContextAwareWorkflowExecutor {
    /// Reference to the global context bus
    context_bus: Arc<ContextBus>,
    
    /// Active workflow executions
    active_executions: Arc<RwLock<HashMap<String, WorkflowExecution>>>,
    
    /// Context subscriptions for workflows
    context_subscriptions: Arc<RwLock<HashMap<String, Vec<ContextSubscription>>>>,
    
    /// Event broadcaster for workflow events
    event_broadcaster: broadcast::Sender<WorkflowContextEvent>,
}

/// Workflow execution state with context integration
#[derive(Debug, Clone)]
pub struct WorkflowExecution {
    pub execution_id: String,
    pub workflow_id: String,
    pub status: ExecutionStatus,
    pub context_snapshot: ContextSnapshot,
    pub node_states: HashMap<String, NodeExecutionState>,
    pub started_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub trigger_context: Option<ContextData>,
}

/// Context subscription for workflows
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextSubscription {
    pub subscription_id: String,
    pub workflow_id: String,
    pub context_path: String,
    pub trigger_condition: TriggerCondition,
    pub node_id: Option<String>, // Specific node to trigger
    pub created_at: DateTime<Utc>,
}

/// Trigger conditions for context-based workflow execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TriggerCondition {
    /// Trigger on any change to the context path
    OnChange,
    /// Trigger when value matches condition
    OnValue(ContextValueCondition),
    /// Trigger on specific event types
    OnEvent(Vec<String>),
    /// Custom trigger logic
    Custom(String),
}

/// Context value conditions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContextValueCondition {
    Equals(serde_json::Value),
    NotEquals(serde_json::Value),
    GreaterThan(serde_json::Value),
    LessThan(serde_json::Value),
    Contains(String),
    Regex(String),
}

/// Context snapshot at workflow execution time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextSnapshot {
    pub timestamp: DateTime<Utc>,
    pub open_files: Vec<String>,
    pub active_agents: Vec<String>,
    pub current_project: Option<String>,
    pub user_context: HashMap<String, serde_json::Value>,
    pub system_state: HashMap<String, serde_json::Value>,
}

/// Node execution state with context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeExecutionState {
    pub node_id: String,
    pub status: NodeExecutionStatus,
    pub context_inputs: HashMap<String, serde_json::Value>,
    pub context_outputs: HashMap<String, serde_json::Value>,
    pub execution_context: Option<ContextData>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error: Option<String>,
}

/// Execution status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
    WaitingForContext,
}

/// Workflow-context events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkflowContextEvent {
    /// Workflow triggered by context change
    ContextTriggered {
        workflow_id: String,
        execution_id: String,
        context_path: String,
        trigger_value: serde_json::Value,
        timestamp: DateTime<Utc>,
    },
    /// Workflow updated context
    ContextUpdated {
        workflow_id: String,
        execution_id: String,
        node_id: String,
        context_path: String,
        new_value: serde_json::Value,
        timestamp: DateTime<Utc>,
    },
    /// Context subscription created
    SubscriptionCreated {
        subscription_id: String,
        workflow_id: String,
        context_path: String,
        timestamp: DateTime<Utc>,
    },
    /// Context subscription removed
    SubscriptionRemoved {
        subscription_id: String,
        workflow_id: String,
        timestamp: DateTime<Utc>,
    },
}

impl ContextAwareWorkflowExecutor {
    /// Create a new context-aware workflow executor
    pub fn new(context_bus: Arc<ContextBus>) -> Self {
        let (event_broadcaster, _) = broadcast::channel(10000);
        
        Self {
            context_bus,
            active_executions: Arc::new(RwLock::new(HashMap::new())),
            context_subscriptions: Arc::new(RwLock::new(HashMap::new())),
            event_broadcaster,
        }
    }

    /// Execute workflow with context integration
    pub async fn execute_workflow_with_context(
        &self,
        workflow: &Workflow,
        trigger: ExecutionTrigger,
        trigger_context: Option<ContextData>,
    ) -> Result<String> {
        let execution_id = Uuid::new_v4().to_string();
        
        // Create context snapshot
        let context_snapshot = self.create_context_snapshot().await?;
        
        // Initialize workflow execution
        let execution = WorkflowExecution {
            execution_id: execution_id.clone(),
            workflow_id: workflow.id.clone(),
            status: ExecutionStatus::Running,
            context_snapshot,
            node_states: HashMap::new(),
            started_at: Utc::now(),
            updated_at: Utc::now(),
            trigger_context,
        };

        // Store execution state
        {
            let mut executions = self.active_executions.write().await;
            executions.insert(execution_id.clone(), execution);
        }

        // Execute workflow nodes with context awareness
        self.execute_nodes_with_context(workflow, &execution_id).await?;

        Ok(execution_id)
    }

    /// Subscribe workflow to context changes
    pub async fn subscribe_to_context(
        &self,
        workflow_id: String,
        context_path: String,
        trigger_condition: TriggerCondition,
        node_id: Option<String>,
    ) -> Result<String> {
        let subscription_id = Uuid::new_v4().to_string();
        
        let subscription = ContextSubscription {
            subscription_id: subscription_id.clone(),
            workflow_id: workflow_id.clone(),
            context_path: context_path.clone(),
            trigger_condition,
            node_id,
            created_at: Utc::now(),
        };

        // Store subscription
        {
            let mut subscriptions = self.context_subscriptions.write().await;
            subscriptions
                .entry(workflow_id.clone())
                .or_insert_with(Vec::new)
                .push(subscription);
        }

        // Register with context bus
        self.context_bus.subscribe_to_path(&context_path, &subscription_id).await?;

        // Broadcast subscription event
        let _ = self.event_broadcaster.send(WorkflowContextEvent::SubscriptionCreated {
            subscription_id: subscription_id.clone(),
            workflow_id,
            context_path,
            timestamp: Utc::now(),
        });

        Ok(subscription_id)
    }

    /// Remove context subscription
    pub async fn unsubscribe_from_context(
        &self,
        workflow_id: &str,
        subscription_id: &str,
    ) -> Result<()> {
        // Remove from local subscriptions
        {
            let mut subscriptions = self.context_subscriptions.write().await;
            if let Some(workflow_subs) = subscriptions.get_mut(workflow_id) {
                workflow_subs.retain(|sub| sub.subscription_id != subscription_id);
            }
        }

        // Unregister from context bus
        let system_id = SystemId::new(subscription_id);
        // Note: Arc<ContextBus> doesn't support mutable operations directly
        // Would need to implement unsubscribe differently or use RwLock

        // Broadcast removal event
        let _ = self.event_broadcaster.send(WorkflowContextEvent::SubscriptionRemoved {
            subscription_id: subscription_id.to_string(),
            workflow_id: workflow_id.to_string(),
            timestamp: Utc::now(),
        });

        Ok(())
    }

    /// Get context data for workflow node
    pub async fn get_context_for_node(
        &self,
        execution_id: &str,
        node_id: &str,
        context_query: &ContextQuery,
    ) -> Result<ContextData> {
        // Get execution context
        let execution = {
            let executions = self.active_executions.read().await;
            executions.get(execution_id).cloned()
                .ok_or_else(|| SymbioteError::NotFound("Execution not found".to_string()))?
        };

        // Query context bus with execution context
        let mut context_data = self.context_bus.query_context(context_query).await?;
        
        // Merge with execution-specific context
        if let Some(trigger_context) = &execution.trigger_context {
            context_data.merge(trigger_context.clone());
        }

        Ok(context_data)
    }

    /// Update context from workflow node
    pub async fn update_context_from_node(
        &self,
        execution_id: &str,
        node_id: &str,
        context_path: &str,
        value: serde_json::Value,
    ) -> Result<()> {
        // Create context update
        let mut data = serde_json::Map::new();
        data.insert("path".to_string(), serde_json::Value::String(context_path.to_string()));
        data.insert("value".to_string(), value.clone());
        data.insert("source".to_string(), serde_json::Value::String(format!("workflow:{}:node:{}", execution_id, node_id)));

        let update = ContextUpdate {
            id: Uuid::new_v4().to_string(),
            system_id: SystemId::new("workflow"),
            update_type: ContextUpdateType::WorkflowStarted,
            data: serde_json::Value::Object(data),
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            priority: UpdatePriority::Normal,
        };

        // Apply update to context bus
        self.context_bus.update_context(update).await?;

        // Update node execution state
        {
            let mut executions = self.active_executions.write().await;
            if let Some(execution) = executions.get_mut(execution_id) {
                if let Some(node_state) = execution.node_states.get_mut(node_id) {
                    node_state.context_outputs.insert(context_path.to_string(), value.clone());
                }
            }
        }

        // Broadcast context update event
        let _ = self.event_broadcaster.send(WorkflowContextEvent::ContextUpdated {
            workflow_id: execution_id.to_string(), // This should be workflow_id
            execution_id: execution_id.to_string(),
            node_id: node_id.to_string(),
            context_path: context_path.to_string(),
            new_value: value,
            timestamp: Utc::now(),
        });

        Ok(())
    }

    /// Handle context change notifications
    pub async fn handle_context_change(
        &self,
        context_path: &str,
        new_value: &serde_json::Value,
    ) -> Result<()> {
        let subscriptions = {
            let subs = self.context_subscriptions.read().await;
            subs.clone()
        };

        for (workflow_id, workflow_subs) in subscriptions {
            for subscription in workflow_subs {
                if subscription.context_path == context_path {
                    // Check trigger condition
                    if self.should_trigger(&subscription.trigger_condition, new_value) {
                        // Trigger workflow execution
                        self.trigger_workflow_from_context(
                            &workflow_id,
                            &subscription,
                            new_value.clone(),
                        ).await?;
                    }
                }
            }
        }

        Ok(())
    }

    /// Create context snapshot
    async fn create_context_snapshot(&self) -> Result<ContextSnapshot> {
        // Query current context state
        let context_query = ContextQuery {
            paths: vec![
                "files.open".to_string(),
                "agents.active".to_string(),
                "project.current".to_string(),
                "user.*".to_string(),
                "system.*".to_string(),
            ],
            include_metadata: true,
            max_depth: Some(3),
        };

        let context_data = self.context_bus.query_context(&context_query).await?;
        
        Ok(ContextSnapshot {
            timestamp: Utc::now(),
            open_files: context_data.get_array("files.open").unwrap_or_default(),
            active_agents: context_data.get_array("agents.active").unwrap_or_default(),
            current_project: context_data.get_string("project.current"),
            user_context: context_data.get_object("user").unwrap_or_default(),
            system_state: context_data.get_object("system").unwrap_or_default(),
        })
    }

    /// Execute workflow nodes with context awareness
    async fn execute_nodes_with_context(
        &self,
        workflow: &Workflow,
        execution_id: &str,
    ) -> Result<()> {
        // This would integrate with the actual workflow execution engine
        // For now, we'll create a placeholder that shows the integration points
        
        for node in &workflow.nodes {
            // Initialize node execution state
            let node_state = NodeExecutionState {
                node_id: node.id.clone(),
                status: NodeExecutionStatus::Running,
                context_inputs: HashMap::new(),
                context_outputs: HashMap::new(),
                execution_context: None,
                started_at: Some(Utc::now()),
                completed_at: None,
                error: None,
            };

            // Update execution state
            {
                let mut executions = self.active_executions.write().await;
                if let Some(execution) = executions.get_mut(execution_id) {
                    execution.node_states.insert(node.id.clone(), node_state);
                }
            }

            // Execute node with context integration
            // This would call the actual node execution logic
            // self.execute_single_node_with_context(execution_id, node).await?;
        }

        Ok(())
    }

    /// Check if trigger condition is met
    fn should_trigger(
        &self,
        condition: &TriggerCondition,
        value: &serde_json::Value,
    ) -> bool {
        match condition {
            TriggerCondition::OnChange => true,
            TriggerCondition::OnValue(value_condition) => {
                self.check_value_condition(value_condition, value)
            }
            TriggerCondition::OnEvent(events) => {
                // This would check if the context change represents one of the specified events
                false // Placeholder
            }
            TriggerCondition::Custom(_) => {
                // This would evaluate custom trigger logic
                false // Placeholder
            }
        }
    }

    /// Check value condition
    fn check_value_condition(
        &self,
        condition: &ContextValueCondition,
        value: &serde_json::Value,
    ) -> bool {
        match condition {
            ContextValueCondition::Equals(expected) => value == expected,
            ContextValueCondition::NotEquals(expected) => value != expected,
            ContextValueCondition::GreaterThan(expected) => {
                // This would implement proper comparison logic
                false // Placeholder
            }
            ContextValueCondition::LessThan(expected) => {
                // This would implement proper comparison logic
                false // Placeholder
            }
            ContextValueCondition::Contains(substring) => {
                if let Some(str_value) = value.as_str() {
                    str_value.contains(substring)
                } else {
                    false
                }
            }
            ContextValueCondition::Regex(pattern) => {
                // This would implement regex matching
                false // Placeholder
            }
        }
    }

    /// Trigger workflow from context change
    async fn trigger_workflow_from_context(
        &self,
        workflow_id: &str,
        subscription: &ContextSubscription,
        trigger_value: serde_json::Value,
    ) -> Result<()> {
        let execution_id = Uuid::new_v4().to_string();

        // Broadcast trigger event
        let _ = self.event_broadcaster.send(WorkflowContextEvent::ContextTriggered {
            workflow_id: workflow_id.to_string(),
            execution_id: execution_id.clone(),
            context_path: subscription.context_path.clone(),
            trigger_value,
            timestamp: Utc::now(),
        });

        // This would trigger the actual workflow execution
        // self.execute_workflow_with_context(workflow, ExecutionTrigger::Event, Some(context_data)).await?;

        Ok(())
    }

    /// Subscribe to workflow-context events
    pub fn subscribe_to_events(&self) -> broadcast::Receiver<WorkflowContextEvent> {
        self.event_broadcaster.subscribe()
    }

    /// Get active executions
    pub async fn get_active_executions(&self) -> HashMap<String, WorkflowExecution> {
        let executions = self.active_executions.read().await;
        executions.clone()
    }

    /// Get context subscriptions for workflow
    pub async fn get_workflow_subscriptions(&self, workflow_id: &str) -> Vec<ContextSubscription> {
        let subscriptions = self.context_subscriptions.read().await;
        subscriptions.get(workflow_id).cloned().unwrap_or_default()
    }
}

/// Context data extensions for workflow integration
impl ContextData {
    /// Get array value from context
    pub fn get_array(&self, path: &str) -> Option<Vec<String>> {
        // This would implement path-based array extraction
        None // Placeholder
    }

    /// Get string value from context
    pub fn get_string(&self, path: &str) -> Option<String> {
        // This would implement path-based string extraction
        None // Placeholder
    }

    /// Get object value from context
    pub fn get_object(&self, path: &str) -> Option<HashMap<String, serde_json::Value>> {
        // This would implement path-based object extraction
        None // Placeholder
    }

    /// Merge context data
    pub fn merge(&mut self, other: ContextData) {
        // This would implement context data merging
        // Placeholder
    }
}
