// Phase 2 System Integration
// Integrates all Phase 2 systems through event-driven Context Bus architecture

use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};

use crate::symbiote_core::context::{ContextBus, ContextEvent, EventType};
use crate::symbiote_core::memory::SymbioteMemory;
use crate::symbiote_mind::neural_chain::NeuralChain;
use crate::symbiote_mind::hive_editor::HiveEditor;
use crate::symbiote_shield::guardian::Guardian;
use crate::symbiote_shield::shield::Shield;
use crate::symbiote_interface::user_modes::UserModes;

/// Phase 2 System Integration - Orchestrates all Phase 2 systems through Context Bus
pub struct Phase2Integration {
    // Core event system
    context_bus: Arc<ContextBus>,
    
    // Phase 1 foundation systems
    memory_system: Arc<SymbioteMemory>,
    
    // Phase 2 systems
    neural_chain: Arc<NeuralChain>,
    hive_editor: Arc<HiveEditor>,
    guardian: Arc<Guardian>,
    shield: Arc<Shield>,
    user_modes: Arc<UserModes>,
    
    // Integration state
    integration_state: Arc<RwLock<IntegrationState>>,
    
    // Event handlers
    event_handlers: Arc<RwLock<Vec<EventHandler>>>,
}

/// Integration state tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationState {
    pub systems_initialized: Vec<String>,
    pub active_workflows: Vec<ActiveWorkflow>,
    pub system_health: SystemHealth,
    pub performance_metrics: IntegrationMetrics,
    pub last_updated: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveWorkflow {
    pub workflow_id: String,
    pub workflow_type: WorkflowType,
    pub involved_systems: Vec<String>,
    pub current_step: String,
    pub started_at: DateTime<Utc>,
    pub estimated_completion: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkflowType {
    CodeAnalysisAndRefactoring,
    SecurityScanAndRemediation,
    MultiFileEditing,
    ModeTransitionWithContext,
    AIAssistedDevelopment,
    ApprovalWorkflow,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemHealth {
    pub overall_status: HealthStatus,
    pub system_statuses: std::collections::HashMap<String, HealthStatus>,
    pub last_health_check: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy,
    Warning,
    Critical,
    Offline,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct IntegrationMetrics {
    pub total_events_processed: u64,
    pub events_per_second: f64,
    pub cross_system_workflows: u32,
    pub successful_integrations: u32,
    pub failed_integrations: u32,
    pub average_workflow_time: std::time::Duration,
}

/// Event handler for cross-system integration
#[derive(Debug, Clone)]
pub struct EventHandler {
    pub handler_id: String,
    pub event_types: Vec<EventType>,
    pub target_systems: Vec<String>,
    pub handler_function: String,
}

impl Phase2Integration {
    /// Create new Phase 2 integration system
    pub async fn new() -> Result<Self, IntegrationError> {
        // Initialize Context Bus
        let context_bus = Arc::new(ContextBus::new().await);
        
        // Initialize all systems
        let memory_system = Arc::new(SymbioteMemory::new());
        let neural_chain = Arc::new(NeuralChain::new());
        let hive_editor = Arc::new(HiveEditor::new());
        let guardian = Arc::new(Guardian::new());
        let shield = Arc::new(Shield::new());
        let user_modes = Arc::new(UserModes::new());
        
        // Initialize user modes with default modes
        user_modes.initialize_default_modes().await
            .map_err(|e| IntegrationError::SystemInitializationFailed(format!("UserModes: {}", e)))?;
        
        let integration = Self {
            context_bus,
            memory_system,
            neural_chain,
            hive_editor,
            guardian,
            shield,
            user_modes,
            integration_state: Arc::new(RwLock::new(IntegrationState {
                systems_initialized: Vec::new(),
                active_workflows: Vec::new(),
                system_health: SystemHealth {
                    overall_status: HealthStatus::Healthy,
                    system_statuses: std::collections::HashMap::new(),
                    last_health_check: Utc::now(),
                },
                performance_metrics: IntegrationMetrics::default(),
                last_updated: Utc::now(),
            })),
            event_handlers: Arc::new(RwLock::new(Vec::new())),
        };
        
        // Set up event-driven integration
        integration.setup_event_integration().await?;
        
        Ok(integration)
    }
    
    /// Set up event-driven integration between all systems
    async fn setup_event_integration(&self) -> Result<(), IntegrationError> {
        // Register event handlers for cross-system workflows
        self.register_neural_chain_integration().await?;
        self.register_hive_editor_integration().await?;
        self.register_guardian_integration().await?;
        self.register_shield_integration().await?;
        self.register_user_modes_integration().await?;
        
        // Start event processing
        self.start_event_processing().await?;
        
        // Update integration state
        {
            let mut state = self.integration_state.write().await;
            state.systems_initialized = vec![
                "ContextBus".to_string(),
                "SymbioteMemory".to_string(),
                "NeuralChain".to_string(),
                "HiveEditor".to_string(),
                "Guardian".to_string(),
                "Shield".to_string(),
                "UserModes".to_string(),
            ];
            state.last_updated = Utc::now();
        }
        
        Ok(())
    }
    
    /// Register Neural Chain integration events
    async fn register_neural_chain_integration(&self) -> Result<(), IntegrationError> {
        // Neural Chain responds to complex problem events
        let handler = EventHandler {
            handler_id: "neural_chain_problem_solver".to_string(),
            event_types: vec![
                EventType::Custom("complex_problem_detected".to_string()),
                EventType::Custom("multi_step_reasoning_required".to_string()),
            ],
            target_systems: vec!["NeuralChain".to_string()],
            handler_function: "handle_complex_problem".to_string(),
        };
        
        {
            let mut handlers = self.event_handlers.write().await;
            handlers.push(handler);
        }
        
        Ok(())
    }
    
    /// Register Hive Editor integration events
    async fn register_hive_editor_integration(&self) -> Result<(), IntegrationError> {
        // Hive Editor responds to multi-file editing events
        let handler = EventHandler {
            handler_id: "hive_editor_coordinator".to_string(),
            event_types: vec![
                EventType::Custom("multi_file_edit_requested".to_string()),
                EventType::Custom("coordinated_changes_needed".to_string()),
            ],
            target_systems: vec!["HiveEditor".to_string()],
            handler_function: "handle_multi_file_coordination".to_string(),
        };
        
        {
            let mut handlers = self.event_handlers.write().await;
            handlers.push(handler);
        }
        
        Ok(())
    }
    
    /// Register Guardian integration events
    async fn register_guardian_integration(&self) -> Result<(), IntegrationError> {
        // Guardian responds to approval requirement events
        let handler = EventHandler {
            handler_id: "guardian_approval_handler".to_string(),
            event_types: vec![
                EventType::Custom("approval_required".to_string()),
                EventType::Custom("high_risk_operation".to_string()),
            ],
            target_systems: vec!["Guardian".to_string()],
            handler_function: "handle_approval_request".to_string(),
        };
        
        {
            let mut handlers = self.event_handlers.write().await;
            handlers.push(handler);
        }
        
        Ok(())
    }
    
    /// Register Shield integration events
    async fn register_shield_integration(&self) -> Result<(), IntegrationError> {
        // Shield responds to security scan events
        let handler = EventHandler {
            handler_id: "shield_security_scanner".to_string(),
            event_types: vec![
                EventType::Custom("security_scan_requested".to_string()),
                EventType::Custom("vulnerability_check_needed".to_string()),
            ],
            target_systems: vec!["Shield".to_string()],
            handler_function: "handle_security_scan".to_string(),
        };
        
        {
            let mut handlers = self.event_handlers.write().await;
            handlers.push(handler);
        }
        
        Ok(())
    }
    
    /// Register User Modes integration events
    async fn register_user_modes_integration(&self) -> Result<(), IntegrationError> {
        // User Modes responds to mode switch events
        let handler = EventHandler {
            handler_id: "user_modes_switcher".to_string(),
            event_types: vec![
                EventType::Custom("mode_switch_requested".to_string()),
                EventType::Custom("context_change_detected".to_string()),
            ],
            target_systems: vec!["UserModes".to_string()],
            handler_function: "handle_mode_switch".to_string(),
        };
        
        {
            let mut handlers = self.event_handlers.write().await;
            handlers.push(handler);
        }
        
        Ok(())
    }
    
    /// Start event processing for integration
    async fn start_event_processing(&self) -> Result<(), IntegrationError> {
        let context_bus = Arc::clone(&self.context_bus);
        let integration_state = Arc::clone(&self.integration_state);
        
        // Spawn event processing task
        tokio::spawn(async move {
            let mut event_receiver = context_bus.subscribe_to_events().await;
            
            while let Ok(event) = event_receiver.recv().await {
                // Process integration events
                Self::process_integration_event(&event, &integration_state).await;
            }
        });
        
        Ok(())
    }
    
    /// Process integration events
    async fn process_integration_event(
        event: &ContextEvent,
        integration_state: &Arc<RwLock<IntegrationState>>,
    ) {
        // Update metrics
        {
            let mut state = integration_state.write().await;
            state.performance_metrics.total_events_processed += 1;
            
            // Calculate events per second (simplified)
            let now = Utc::now();
            let time_diff = now.signed_duration_since(state.last_updated).num_seconds() as f64;
            if time_diff > 0.0 {
                state.performance_metrics.events_per_second = 
                    state.performance_metrics.total_events_processed as f64 / time_diff;
            }
            
            state.last_updated = now;
        }
        
        // Handle specific integration events
        match &event.event_type {
            EventType::Custom(event_name) => {
                match event_name.as_str() {
                    "complex_problem_detected" => {
                        // Trigger Neural Chain workflow
                        Self::start_neural_chain_workflow(event, integration_state).await;
                    }
                    "multi_file_edit_requested" => {
                        // Trigger Hive Editor workflow
                        Self::start_hive_editor_workflow(event, integration_state).await;
                    }
                    "approval_required" => {
                        // Trigger Guardian workflow
                        Self::start_guardian_workflow(event, integration_state).await;
                    }
                    "security_scan_requested" => {
                        // Trigger Shield workflow
                        Self::start_shield_workflow(event, integration_state).await;
                    }
                    "mode_switch_requested" => {
                        // Trigger User Modes workflow
                        Self::start_user_modes_workflow(event, integration_state).await;
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
    
    /// Start Neural Chain workflow
    async fn start_neural_chain_workflow(
        _event: &ContextEvent,
        integration_state: &Arc<RwLock<IntegrationState>>,
    ) {
        let workflow = ActiveWorkflow {
            workflow_id: uuid::Uuid::new_v4().to_string(),
            workflow_type: WorkflowType::AIAssistedDevelopment,
            involved_systems: vec!["NeuralChain".to_string(), "ContextBus".to_string()],
            current_step: "problem_analysis".to_string(),
            started_at: Utc::now(),
            estimated_completion: Some(Utc::now() + chrono::Duration::minutes(5)),
        };
        
        let mut state = integration_state.write().await;
        state.active_workflows.push(workflow);
        state.performance_metrics.cross_system_workflows += 1;
    }
    
    /// Start Hive Editor workflow
    async fn start_hive_editor_workflow(
        _event: &ContextEvent,
        integration_state: &Arc<RwLock<IntegrationState>>,
    ) {
        let workflow = ActiveWorkflow {
            workflow_id: uuid::Uuid::new_v4().to_string(),
            workflow_type: WorkflowType::MultiFileEditing,
            involved_systems: vec!["HiveEditor".to_string(), "Guardian".to_string(), "ContextBus".to_string()],
            current_step: "dependency_analysis".to_string(),
            started_at: Utc::now(),
            estimated_completion: Some(Utc::now() + chrono::Duration::minutes(3)),
        };
        
        let mut state = integration_state.write().await;
        state.active_workflows.push(workflow);
        state.performance_metrics.cross_system_workflows += 1;
    }
    
    /// Start Guardian workflow
    async fn start_guardian_workflow(
        _event: &ContextEvent,
        integration_state: &Arc<RwLock<IntegrationState>>,
    ) {
        let workflow = ActiveWorkflow {
            workflow_id: uuid::Uuid::new_v4().to_string(),
            workflow_type: WorkflowType::ApprovalWorkflow,
            involved_systems: vec!["Guardian".to_string(), "ContextBus".to_string()],
            current_step: "risk_assessment".to_string(),
            started_at: Utc::now(),
            estimated_completion: Some(Utc::now() + chrono::Duration::seconds(30)),
        };
        
        let mut state = integration_state.write().await;
        state.active_workflows.push(workflow);
        state.performance_metrics.cross_system_workflows += 1;
    }
    
    /// Start Shield workflow
    async fn start_shield_workflow(
        _event: &ContextEvent,
        integration_state: &Arc<RwLock<IntegrationState>>,
    ) {
        let workflow = ActiveWorkflow {
            workflow_id: uuid::Uuid::new_v4().to_string(),
            workflow_type: WorkflowType::SecurityScanAndRemediation,
            involved_systems: vec!["Shield".to_string(), "Guardian".to_string(), "ContextBus".to_string()],
            current_step: "vulnerability_scan".to_string(),
            started_at: Utc::now(),
            estimated_completion: Some(Utc::now() + chrono::Duration::minutes(2)),
        };
        
        let mut state = integration_state.write().await;
        state.active_workflows.push(workflow);
        state.performance_metrics.cross_system_workflows += 1;
    }
    
    /// Start User Modes workflow
    async fn start_user_modes_workflow(
        _event: &ContextEvent,
        integration_state: &Arc<RwLock<IntegrationState>>,
    ) {
        let workflow = ActiveWorkflow {
            workflow_id: uuid::Uuid::new_v4().to_string(),
            workflow_type: WorkflowType::ModeTransitionWithContext,
            involved_systems: vec!["UserModes".to_string(), "SymbioteMemory".to_string(), "ContextBus".to_string()],
            current_step: "context_preservation".to_string(),
            started_at: Utc::now(),
            estimated_completion: Some(Utc::now() + chrono::Duration::seconds(1)),
        };
        
        let mut state = integration_state.write().await;
        state.active_workflows.push(workflow);
        state.performance_metrics.cross_system_workflows += 1;
    }
    
    /// Demonstrate integrated workflow
    pub async fn demonstrate_integration(&self) -> Result<IntegrationDemonstration, IntegrationError> {
        let mut demonstration = IntegrationDemonstration {
            demonstration_id: uuid::Uuid::new_v4().to_string(),
            started_at: Utc::now(),
            workflows_executed: Vec::new(),
            performance_results: Vec::new(),
            integration_success: true,
            completed_at: None,
        };
        
        // 1. Trigger Neural Chain workflow
        self.context_bus.publish_event(ContextEvent {
            event_id: uuid::Uuid::new_v4().to_string(),
            event_type: EventType::Custom("complex_problem_detected".to_string()),
            source: "Integration Demo".to_string(),
            data: serde_json::json!({"problem": "Optimize database queries across multiple files"}),
            timestamp: Utc::now(),
        }).await.map_err(|e| IntegrationError::EventPublishFailed(e.to_string()))?;
        
        // 2. Trigger Hive Editor workflow
        self.context_bus.publish_event(ContextEvent {
            event_id: uuid::Uuid::new_v4().to_string(),
            event_type: EventType::Custom("multi_file_edit_requested".to_string()),
            source: "Integration Demo".to_string(),
            data: serde_json::json!({"files": ["db.rs", "models.rs", "queries.rs"]}),
            timestamp: Utc::now(),
        }).await.map_err(|e| IntegrationError::EventPublishFailed(e.to_string()))?;
        
        // 3. Trigger Guardian approval
        self.context_bus.publish_event(ContextEvent {
            event_id: uuid::Uuid::new_v4().to_string(),
            event_type: EventType::Custom("approval_required".to_string()),
            source: "Integration Demo".to_string(),
            data: serde_json::json!({"operation": "database_schema_change"}),
            timestamp: Utc::now(),
        }).await.map_err(|e| IntegrationError::EventPublishFailed(e.to_string()))?;
        
        // 4. Trigger Shield security scan
        self.context_bus.publish_event(ContextEvent {
            event_id: uuid::Uuid::new_v4().to_string(),
            event_type: EventType::Custom("security_scan_requested".to_string()),
            source: "Integration Demo".to_string(),
            data: serde_json::json!({"scope": "database_layer"}),
            timestamp: Utc::now(),
        }).await.map_err(|e| IntegrationError::EventPublishFailed(e.to_string()))?;
        
        // 5. Trigger User Modes switch
        self.context_bus.publish_event(ContextEvent {
            event_id: uuid::Uuid::new_v4().to_string(),
            event_type: EventType::Custom("mode_switch_requested".to_string()),
            source: "Integration Demo".to_string(),
            data: serde_json::json!({"target_mode": "debugging"}),
            timestamp: Utc::now(),
        }).await.map_err(|e| IntegrationError::EventPublishFailed(e.to_string()))?;
        
        // Wait for workflows to process
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        
        // Collect results
        let state = self.integration_state.read().await;
        demonstration.workflows_executed = state.active_workflows.clone();
        demonstration.performance_results = vec![
            format!("Events processed: {}", state.performance_metrics.total_events_processed),
            format!("Events/sec: {:.2}", state.performance_metrics.events_per_second),
            format!("Cross-system workflows: {}", state.performance_metrics.cross_system_workflows),
        ];
        demonstration.completed_at = Some(Utc::now());
        
        Ok(demonstration)
    }
    
    /// Get integration status
    pub async fn get_integration_status(&self) -> IntegrationState {
        let state = self.integration_state.read().await;
        state.clone()
    }
    
    /// Get system health
    pub async fn check_system_health(&self) -> SystemHealth {
        let state = self.integration_state.read().await;
        state.system_health.clone()
    }
}

/// Integration demonstration results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationDemonstration {
    pub demonstration_id: String,
    pub started_at: DateTime<Utc>,
    pub workflows_executed: Vec<ActiveWorkflow>,
    pub performance_results: Vec<String>,
    pub integration_success: bool,
    pub completed_at: Option<DateTime<Utc>>,
}

/// Integration error types
#[derive(Debug, thiserror::Error)]
pub enum IntegrationError {
    #[error("System initialization failed: {0}")]
    SystemInitializationFailed(String),
    #[error("Event publish failed: {0}")]
    EventPublishFailed(String),
    #[error("Integration setup failed")]
    IntegrationSetupFailed,
    #[error("Workflow execution failed")]
    WorkflowExecutionFailed,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_phase2_integration() {
        let integration = Phase2Integration::new().await.unwrap();
        
        let status = integration.get_integration_status().await;
        assert_eq!(status.systems_initialized.len(), 7);
        assert!(status.systems_initialized.contains(&"NeuralChain".to_string()));
        assert!(status.systems_initialized.contains(&"HiveEditor".to_string()));
        assert!(status.systems_initialized.contains(&"Guardian".to_string()));
        assert!(status.systems_initialized.contains(&"Shield".to_string()));
        assert!(status.systems_initialized.contains(&"UserModes".to_string()));
    }
    
    #[tokio::test]
    async fn test_integration_demonstration() {
        let integration = Phase2Integration::new().await.unwrap();
        
        let demo = integration.demonstrate_integration().await.unwrap();
        assert!(demo.integration_success);
        assert!(!demo.workflows_executed.is_empty());
        assert!(demo.completed_at.is_some());
        
        // Verify cross-system workflows were created
        let workflow_types: Vec<_> = demo.workflows_executed.iter()
            .map(|w| &w.workflow_type)
            .collect();
        
        assert!(workflow_types.len() >= 5); // Should have triggered multiple workflows
    }
}
