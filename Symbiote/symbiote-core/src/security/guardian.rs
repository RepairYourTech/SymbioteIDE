//! # Symbiote Guardian - Human-in-the-loop Control System
//! 
//! Advanced human oversight and control system for AI operations.
//! Provides transparency, approval workflows, and intervention capabilities.
//! 
//! Following Week 11-12 Security & Control implementation plan.

use crate::{Result, SymbioteError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use uuid::Uuid;
use std::time::{SystemTime, UNIX_EPOCH};

/// Approval request for AI operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalRequest {
    pub id: String,
    pub operation_type: OperationType,
    pub description: String,
    pub risk_level: RiskLevel,
    pub requested_by: String,
    pub context: HashMap<String, serde_json::Value>,
    pub auto_approve_eligible: bool,
    pub timeout_seconds: u64,
    pub created_at: u64,
}

/// Types of operations requiring approval
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum OperationType {
    FileModification,
    FileCreation,
    FileDeletion,
    CommandExecution,
    NetworkRequest,
    DatabaseOperation,
    AIModelInvocation,
    SystemConfiguration,
    SecuritySetting,
    DataExport,
}

/// Risk levels for operations
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

/// Approval decision
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalDecision {
    pub request_id: String,
    pub decision: Decision,
    pub approved_by: String,
    pub reason: Option<String>,
    pub conditions: Vec<String>,
    pub decided_at: u64,
}

/// Decision types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Decision {
    Approved,
    Rejected,
    ConditionalApproval,
    Timeout,
}

/// Approval engine managing approval workflows
pub struct ApprovalEngine {
    pending_requests: Arc<RwLock<HashMap<String, ApprovalRequest>>>,
    approval_history: Arc<RwLock<Vec<ApprovalDecision>>>,
    auto_approval_rules: Arc<RwLock<Vec<AutoApprovalRule>>>,
    approval_sender: mpsc::UnboundedSender<ApprovalRequest>,
    decision_receiver: Arc<RwLock<mpsc::UnboundedReceiver<ApprovalDecision>>>,
}

/// Auto-approval rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoApprovalRule {
    pub id: String,
    pub name: String,
    pub operation_types: Vec<OperationType>,
    pub max_risk_level: RiskLevel,
    pub conditions: Vec<String>,
    pub enabled: bool,
}

/// Transparency window for AI operations
pub struct TransparencyWindow {
    operation_log: Arc<RwLock<Vec<OperationRecord>>>,
    real_time_subscribers: Arc<RwLock<Vec<mpsc::UnboundedSender<OperationRecord>>>>,
}

/// Record of AI operations for transparency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationRecord {
    pub id: String,
    pub operation_type: OperationType,
    pub description: String,
    pub input_data: serde_json::Value,
    pub output_data: Option<serde_json::Value>,
    pub ai_model_used: Option<String>,
    pub cost: Option<f64>,
    pub execution_time_ms: u64,
    pub risk_assessment: RiskAssessment,
    pub approval_status: Option<ApprovalDecision>,
    pub timestamp: u64,
}

/// Risk assessment for operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskAssessment {
    pub overall_risk: RiskLevel,
    pub risk_factors: Vec<RiskFactor>,
    pub mitigation_suggestions: Vec<String>,
    pub confidence: f64,
}

/// Individual risk factor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskFactor {
    pub factor_type: String,
    pub severity: RiskLevel,
    pub description: String,
    pub likelihood: f64,
}

/// Intervention system for emergency stops
pub struct InterventionSystem {
    emergency_stops: Arc<RwLock<Vec<EmergencyStop>>>,
    intervention_rules: Arc<RwLock<Vec<InterventionRule>>>,
    active_interventions: Arc<RwLock<HashMap<String, ActiveIntervention>>>,
}

/// Emergency stop record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmergencyStop {
    pub id: String,
    pub triggered_by: String,
    pub reason: String,
    pub affected_operations: Vec<String>,
    pub timestamp: u64,
    pub resolved: bool,
}

/// Intervention rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterventionRule {
    pub id: String,
    pub name: String,
    pub trigger_conditions: Vec<String>,
    pub intervention_type: InterventionType,
    pub auto_trigger: bool,
    pub enabled: bool,
}

/// Types of interventions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InterventionType {
    StopOperation,
    RequireApproval,
    AlertUser,
    LogWarning,
    RateLimitOperation,
}

/// Active intervention
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveIntervention {
    pub id: String,
    pub rule_id: String,
    pub operation_id: String,
    pub intervention_type: InterventionType,
    pub started_at: u64,
    pub resolved: bool,
}

/// Trust metrics for AI operations
pub struct TrustMetrics {
    operation_success_rates: Arc<RwLock<HashMap<OperationType, f64>>>,
    ai_model_reliability: Arc<RwLock<HashMap<String, ModelReliability>>>,
    user_satisfaction_scores: Arc<RwLock<Vec<SatisfactionScore>>>,
    trust_trends: Arc<RwLock<Vec<TrustTrend>>>,
}

/// Model reliability metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelReliability {
    pub model_name: String,
    pub success_rate: f64,
    pub average_confidence: f64,
    pub error_rate: f64,
    pub user_approval_rate: f64,
    pub last_updated: u64,
}

/// User satisfaction score
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SatisfactionScore {
    pub operation_id: String,
    pub user_id: String,
    pub score: f64, // 1.0 to 5.0
    pub feedback: Option<String>,
    pub timestamp: u64,
}

/// Trust trend data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustTrend {
    pub timestamp: u64,
    pub overall_trust_score: f64,
    pub operation_type_scores: HashMap<OperationType, f64>,
    pub model_scores: HashMap<String, f64>,
}

/// Main Symbiote Guardian system
pub struct SymbioteGuardian {
    approval_engine: ApprovalEngine,
    transparency_window: TransparencyWindow,
    intervention_system: InterventionSystem,
    trust_metrics: TrustMetrics,
}

impl SymbioteGuardian {
    pub fn new() -> Self {
        let (approval_sender, _approval_receiver) = mpsc::unbounded_channel();
        let (_decision_sender, decision_receiver) = mpsc::unbounded_channel();

        Self {
            approval_engine: ApprovalEngine::new(approval_sender, decision_receiver),
            transparency_window: TransparencyWindow::new(),
            intervention_system: InterventionSystem::new(),
            trust_metrics: TrustMetrics::new(),
        }
    }

    /// Request approval for an operation
    pub async fn request_approval(&self, request: ApprovalRequest) -> Result<String> {
        // Check if auto-approval is possible
        if self.approval_engine.can_auto_approve(&request).await? {
            let decision = ApprovalDecision {
                request_id: request.id.clone(),
                decision: Decision::Approved,
                approved_by: "system".to_string(),
                reason: Some("Auto-approved based on rules".to_string()),
                conditions: vec![],
                decided_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            };
            
            self.approval_engine.record_decision(decision).await?;
            return Ok(request.id);
        }

        // Add to pending requests
        self.approval_engine.add_pending_request(request.clone()).await?;
        
        // Send for human approval
        self.approval_engine.approval_sender.send(request.clone())
            .map_err(|e| SymbioteError::security(format!("Failed to send approval request: {}", e)))?;

        Ok(request.id)
    }

    /// Submit approval decision
    pub async fn submit_decision(&self, decision: ApprovalDecision) -> Result<()> {
        self.approval_engine.record_decision(decision).await
    }

    /// Log operation for transparency
    pub async fn log_operation(&self, record: OperationRecord) -> Result<()> {
        self.transparency_window.log_operation(record).await
    }

    /// Trigger emergency stop
    pub async fn emergency_stop(&self, reason: String, triggered_by: String) -> Result<String> {
        self.intervention_system.trigger_emergency_stop(reason, triggered_by).await
    }

    /// Get trust metrics
    pub async fn get_trust_metrics(&self) -> Result<TrustSummary> {
        self.trust_metrics.get_summary().await
    }

    /// Check if operation should be intervened
    pub async fn check_intervention(&self, operation: &OperationRecord) -> Result<Option<InterventionType>> {
        self.intervention_system.evaluate_operation(operation).await
    }
}

/// Trust summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustSummary {
    pub overall_trust_score: f64,
    pub operation_success_rates: HashMap<OperationType, f64>,
    pub top_performing_models: Vec<String>,
    pub recent_satisfaction_average: f64,
    pub trust_trend_direction: TrendDirection,
}

/// Trend direction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TrendDirection {
    Improving,
    Stable,
    Declining,
}

impl ApprovalEngine {
    pub fn new(
        approval_sender: mpsc::UnboundedSender<ApprovalRequest>,
        decision_receiver: mpsc::UnboundedReceiver<ApprovalDecision>,
    ) -> Self {
        Self {
            pending_requests: Arc::new(RwLock::new(HashMap::new())),
            approval_history: Arc::new(RwLock::new(Vec::new())),
            auto_approval_rules: Arc::new(RwLock::new(Vec::new())),
            approval_sender,
            decision_receiver: Arc::new(RwLock::new(decision_receiver)),
        }
    }

    pub async fn can_auto_approve(&self, request: &ApprovalRequest) -> Result<bool> {
        if !request.auto_approve_eligible {
            return Ok(false);
        }

        let rules = self.auto_approval_rules.read().await;
        for rule in rules.iter() {
            if rule.enabled 
                && rule.operation_types.contains(&request.operation_type)
                && request.risk_level <= rule.max_risk_level {
                return Ok(true);
            }
        }

        Ok(false)
    }

    pub async fn add_pending_request(&self, request: ApprovalRequest) -> Result<()> {
        let mut pending = self.pending_requests.write().await;
        pending.insert(request.id.clone(), request);
        Ok(())
    }

    pub async fn record_decision(&self, decision: ApprovalDecision) -> Result<()> {
        // Remove from pending
        {
            let mut pending = self.pending_requests.write().await;
            pending.remove(&decision.request_id);
        }

        // Add to history
        {
            let mut history = self.approval_history.write().await;
            history.push(decision);
        }

        Ok(())
    }
}

impl TransparencyWindow {
    pub fn new() -> Self {
        Self {
            operation_log: Arc::new(RwLock::new(Vec::new())),
            real_time_subscribers: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn log_operation(&self, record: OperationRecord) -> Result<()> {
        // Add to log
        {
            let mut log = self.operation_log.write().await;
            log.push(record.clone());
            
            // Keep only last 10,000 records
            if log.len() > 10000 {
                log.drain(0..1000);
            }
        }

        // Notify subscribers
        {
            let subscribers = self.real_time_subscribers.read().await;
            for sender in subscribers.iter() {
                let _ = sender.send(record.clone());
            }
        }

        Ok(())
    }
}

impl InterventionSystem {
    pub fn new() -> Self {
        Self {
            emergency_stops: Arc::new(RwLock::new(Vec::new())),
            intervention_rules: Arc::new(RwLock::new(Vec::new())),
            active_interventions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn trigger_emergency_stop(&self, reason: String, triggered_by: String) -> Result<String> {
        let stop = EmergencyStop {
            id: Uuid::new_v4().to_string(),
            triggered_by,
            reason,
            affected_operations: vec![], // TODO: Determine affected operations
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            resolved: false,
        };

        let stop_id = stop.id.clone();
        
        {
            let mut stops = self.emergency_stops.write().await;
            stops.push(stop);
        }

        Ok(stop_id)
    }

    pub async fn evaluate_operation(&self, _operation: &OperationRecord) -> Result<Option<InterventionType>> {
        // TODO: Implement intervention rule evaluation
        Ok(None)
    }
}

impl TrustMetrics {
    pub fn new() -> Self {
        Self {
            operation_success_rates: Arc::new(RwLock::new(HashMap::new())),
            ai_model_reliability: Arc::new(RwLock::new(HashMap::new())),
            user_satisfaction_scores: Arc::new(RwLock::new(Vec::new())),
            trust_trends: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn get_summary(&self) -> Result<TrustSummary> {
        let success_rates = self.operation_success_rates.read().await.clone();
        let model_reliability = self.ai_model_reliability.read().await;
        let satisfaction_scores = self.user_satisfaction_scores.read().await;

        let recent_satisfaction_average = if satisfaction_scores.is_empty() {
            0.0
        } else {
            satisfaction_scores.iter().map(|s| s.score).sum::<f64>() / satisfaction_scores.len() as f64
        };

        let top_performing_models = model_reliability
            .iter()
            .filter(|(_, reliability)| reliability.success_rate > 0.9)
            .map(|(name, _)| name.clone())
            .collect();

        Ok(TrustSummary {
            overall_trust_score: 0.85, // TODO: Calculate actual score
            operation_success_rates: success_rates,
            top_performing_models,
            recent_satisfaction_average,
            trust_trend_direction: TrendDirection::Stable,
        })
    }
}
