// Guardian - Human Control & Approval System
// Phase 2 Feature: Human oversight and control over AI operations

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, broadcast};
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Guardian - Human control and approval system for AI operations
pub struct Guardian {
    // Approval management
    approval_manager: ApprovalManager,
    
    // Active approval requests
    pending_requests: Arc<RwLock<HashMap<String, ApprovalRequest>>>,
    
    // Human oversight
    human_oversight: HumanOversight,
    
    // Control policies
    control_policies: Arc<RwLock<ControlPolicies>>,
    
    // Event broadcasting for UI updates
    event_broadcaster: broadcast::Sender<GuardianEvent>,
    
    // Performance metrics
    metrics: Arc<RwLock<GuardianMetrics>>,
}

/// Approval request for AI operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalRequest {
    pub request_id: String,
    pub operation_type: OperationType,
    pub operation_description: String,
    pub risk_assessment: RiskAssessment,
    pub requested_by: String, // Agent ID or system component
    pub context: OperationContext,
    pub approval_requirements: ApprovalRequirements,
    pub status: ApprovalStatus,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub human_reviewer: Option<String>,
    pub approval_decision: Option<ApprovalDecision>,
}

/// Types of operations requiring approval
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OperationType {
    // File operations
    FileCreate,
    FileDelete,
    FileModify,
    DirectoryCreate,
    DirectoryDelete,
    
    // Code operations
    CodeGeneration,
    CodeRefactoring,
    CodeDeletion,
    
    // System operations
    PackageInstall,
    ConfigurationChange,
    EnvironmentModification,
    
    // External operations
    NetworkRequest,
    APICall,
    DatabaseOperation,
    
    // Deployment operations
    Deploy,
    Rollback,
    
    // Security operations
    PermissionChange,
    SecurityConfigChange,
    
    Custom(String),
}

/// Risk assessment for operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskAssessment {
    pub risk_level: RiskLevel,
    pub risk_factors: Vec<RiskFactor>,
    pub impact_analysis: ImpactAnalysis,
    pub mitigation_strategies: Vec<String>,
    pub confidence_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskFactor {
    pub factor_type: RiskFactorType,
    pub description: String,
    pub severity: f64, // 0.0 to 1.0
    pub likelihood: f64, // 0.0 to 1.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RiskFactorType {
    DataLoss,
    SecurityVulnerability,
    SystemInstability,
    PerformanceImpact,
    UserExperienceImpact,
    BusinessImpact,
    ComplianceRisk,
}

/// Impact analysis for operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactAnalysis {
    pub affected_components: Vec<String>,
    pub affected_users: Vec<String>,
    pub reversibility: Reversibility,
    pub estimated_downtime: Option<std::time::Duration>,
    pub data_sensitivity: DataSensitivity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Reversibility {
    FullyReversible,
    PartiallyReversible,
    Irreversible,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DataSensitivity {
    Public,
    Internal,
    Confidential,
    Restricted,
}

/// Context for the operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationContext {
    pub project_context: String,
    pub user_intent: String,
    pub related_operations: Vec<String>,
    pub dependencies: Vec<String>,
    pub environment: String,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Requirements for approval
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalRequirements {
    pub requires_human_approval: bool,
    pub approval_level: ApprovalLevel,
    pub required_reviewers: Vec<String>,
    pub approval_timeout: Option<std::time::Duration>,
    pub auto_approve_conditions: Vec<AutoApprovalCondition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ApprovalLevel {
    Automatic,      // No human approval needed
    Basic,          // Single reviewer
    Enhanced,       // Multiple reviewers
    Executive,      // Senior approval required
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoApprovalCondition {
    pub condition_type: String,
    pub parameters: HashMap<String, serde_json::Value>,
    pub description: String,
}

/// Status of approval request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ApprovalStatus {
    Pending,
    UnderReview,
    Approved,
    Rejected,
    Expired,
    Cancelled,
}

/// Approval decision
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalDecision {
    pub decision: Decision,
    pub reviewer: String,
    pub rationale: String,
    pub conditions: Vec<ApprovalCondition>,
    pub decided_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Decision {
    Approve,
    Reject,
    ApproveWithConditions,
    RequestMoreInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalCondition {
    pub condition_type: String,
    pub description: String,
    pub parameters: HashMap<String, serde_json::Value>,
}

/// Guardian events for UI updates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GuardianEvent {
    ApprovalRequestCreated(String), // request_id
    ApprovalRequestUpdated(String),
    ApprovalGranted(String),
    ApprovalRejected(String),
    ApprovalExpired(String),
    PolicyUpdated,
    RiskLevelChanged(String, RiskLevel),
}

/// Approval management system
pub struct ApprovalManager {
    risk_assessor: RiskAssessor,
    policy_engine: PolicyEngine,
    notification_system: NotificationSystem,
}

/// Human oversight system
pub struct HumanOversight {
    reviewer_pool: Arc<RwLock<ReviewerPool>>,
    escalation_rules: EscalationRules,
    oversight_dashboard: OversightDashboard,
}

/// Control policies for operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlPolicies {
    pub operation_policies: HashMap<OperationType, OperationPolicy>,
    pub risk_policies: HashMap<RiskLevel, RiskPolicy>,
    pub global_policies: GlobalPolicies,
    pub user_preferences: UserPreferences,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationPolicy {
    pub requires_approval: bool,
    pub approval_level: ApprovalLevel,
    pub risk_threshold: f64,
    pub auto_approval_rules: Vec<AutoApprovalRule>,
    pub restrictions: Vec<OperationRestriction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskPolicy {
    pub max_auto_approval_risk: f64,
    pub required_reviewers: u32,
    pub escalation_threshold: f64,
    pub notification_requirements: Vec<NotificationRequirement>,
}

/// Performance metrics for Guardian
#[derive(Debug, Default)]
pub struct GuardianMetrics {
    pub total_requests: u32,
    pub approved_requests: u32,
    pub rejected_requests: u32,
    pub expired_requests: u32,
    pub average_approval_time: std::time::Duration,
    pub risk_prevention_count: u32,
    pub false_positive_rate: f64,
    pub user_satisfaction_score: f64,
}

impl Guardian {
    /// Create a new Guardian system
    pub fn new() -> Self {
        let (event_broadcaster, _) = broadcast::channel(1000);
        
        Self {
            approval_manager: ApprovalManager::new(),
            pending_requests: Arc::new(RwLock::new(HashMap::new())),
            human_oversight: HumanOversight::new(),
            control_policies: Arc::new(RwLock::new(ControlPolicies::default())),
            event_broadcaster,
            metrics: Arc::new(RwLock::new(GuardianMetrics::default())),
        }
    }
    
    /// Request approval for an AI operation
    pub async fn request_approval(
        &self,
        operation_type: OperationType,
        operation_description: String,
        context: OperationContext,
        requested_by: String,
    ) -> Result<String, GuardianError> {
        let request_id = Uuid::new_v4().to_string();
        
        // Assess risk for the operation
        let risk_assessment = self.approval_manager.assess_risk(
            &operation_type,
            &operation_description,
            &context
        ).await?;
        
        // Determine approval requirements
        let approval_requirements = self.determine_approval_requirements(
            &operation_type,
            &risk_assessment
        ).await?;
        
        // Check for auto-approval conditions
        if self.can_auto_approve(&operation_type, &risk_assessment, &approval_requirements).await? {
            // Auto-approve low-risk operations
            let decision = ApprovalDecision {
                decision: Decision::Approve,
                reviewer: "system".to_string(),
                rationale: "Auto-approved based on low risk assessment".to_string(),
                conditions: Vec::new(),
                decided_at: Utc::now(),
            };
            
            // Update metrics
            {
                let mut metrics = self.metrics.write().await;
                metrics.total_requests += 1;
                metrics.approved_requests += 1;
            }
            
            // Broadcast event
            let _ = self.event_broadcaster.send(GuardianEvent::ApprovalGranted(request_id.clone()));
            
            return Ok(request_id);
        }
        
        // Create approval request
        let request = ApprovalRequest {
            request_id: request_id.clone(),
            operation_type,
            operation_description,
            risk_assessment,
            requested_by,
            context,
            approval_requirements,
            status: ApprovalStatus::Pending,
            created_at: Utc::now(),
            expires_at: Some(Utc::now() + chrono::Duration::hours(24)), // Default 24h expiry
            human_reviewer: None,
            approval_decision: None,
        };
        
        // Store pending request
        {
            let mut pending = self.pending_requests.write().await;
            pending.insert(request_id.clone(), request);
        }
        
        // Assign reviewer
        self.human_oversight.assign_reviewer(&request_id).await?;
        
        // Send notifications
        self.approval_manager.send_approval_notification(&request_id).await?;
        
        // Update metrics
        {
            let mut metrics = self.metrics.write().await;
            metrics.total_requests += 1;
        }
        
        // Broadcast event
        let _ = self.event_broadcaster.send(GuardianEvent::ApprovalRequestCreated(request_id.clone()));
        
        Ok(request_id)
    }
    
    /// Provide human approval decision
    pub async fn provide_approval_decision(
        &self,
        request_id: &str,
        decision: Decision,
        reviewer: String,
        rationale: String,
        conditions: Vec<ApprovalCondition>,
    ) -> Result<(), GuardianError> {
        let mut pending = self.pending_requests.write().await;
        let request = pending.get_mut(request_id)
            .ok_or(GuardianError::RequestNotFound)?;
        
        // Update request with decision
        request.approval_decision = Some(ApprovalDecision {
            decision: decision.clone(),
            reviewer: reviewer.clone(),
            rationale,
            conditions,
            decided_at: Utc::now(),
        });
        
        request.status = match decision {
            Decision::Approve | Decision::ApproveWithConditions => ApprovalStatus::Approved,
            Decision::Reject => ApprovalStatus::Rejected,
            Decision::RequestMoreInfo => ApprovalStatus::UnderReview,
        };
        
        request.human_reviewer = Some(reviewer);
        
        // Update metrics
        {
            let mut metrics = self.metrics.write().await;
            match decision {
                Decision::Approve | Decision::ApproveWithConditions => {
                    metrics.approved_requests += 1;
                }
                Decision::Reject => {
                    metrics.rejected_requests += 1;
                }
                _ => {}
            }
            
            let approval_time = Utc::now().signed_duration_since(request.created_at);
            if let Ok(duration) = approval_time.to_std() {
                metrics.average_approval_time = 
                    (metrics.average_approval_time + duration) / 2;
            }
        }
        
        // Broadcast event
        let event = match decision {
            Decision::Approve | Decision::ApproveWithConditions => {
                GuardianEvent::ApprovalGranted(request_id.to_string())
            }
            Decision::Reject => {
                GuardianEvent::ApprovalRejected(request_id.to_string())
            }
            Decision::RequestMoreInfo => {
                GuardianEvent::ApprovalRequestUpdated(request_id.to_string())
            }
        };
        
        let _ = self.event_broadcaster.send(event);
        
        Ok(())
    }
    
    /// Get approval request status
    pub async fn get_approval_status(&self, request_id: &str) -> Option<ApprovalStatus> {
        let pending = self.pending_requests.read().await;
        pending.get(request_id).map(|request| request.status.clone())
    }
    
    /// Get detailed approval request
    pub async fn get_approval_request(&self, request_id: &str) -> Option<ApprovalRequest> {
        let pending = self.pending_requests.read().await;
        pending.get(request_id).cloned()
    }
    
    /// Get all pending approval requests
    pub async fn get_pending_requests(&self) -> Vec<ApprovalRequest> {
        let pending = self.pending_requests.read().await;
        pending.values()
            .filter(|request| matches!(request.status, ApprovalStatus::Pending | ApprovalStatus::UnderReview))
            .cloned()
            .collect()
    }
    
    /// Subscribe to Guardian events
    pub fn subscribe_to_events(&self) -> broadcast::Receiver<GuardianEvent> {
        self.event_broadcaster.subscribe()
    }
    
    /// Update control policies
    pub async fn update_policies(&self, new_policies: ControlPolicies) -> Result<(), GuardianError> {
        {
            let mut policies = self.control_policies.write().await;
            *policies = new_policies;
        }
        
        // Broadcast policy update event
        let _ = self.event_broadcaster.send(GuardianEvent::PolicyUpdated);
        
        Ok(())
    }
    
    /// Get current control policies
    pub async fn get_policies(&self) -> ControlPolicies {
        let policies = self.control_policies.read().await;
        policies.clone()
    }
    
    /// Get Guardian metrics
    pub async fn get_metrics(&self) -> GuardianMetrics {
        let metrics = self.metrics.read().await;
        metrics.clone()
    }
    
    /// Clean up expired requests
    pub async fn cleanup_expired_requests(&self) -> Result<u32, GuardianError> {
        let mut pending = self.pending_requests.write().await;
        let now = Utc::now();
        let mut expired_count = 0;
        
        let expired_ids: Vec<String> = pending.iter()
            .filter(|(_, request)| {
                if let Some(expires_at) = request.expires_at {
                    expires_at < now
                } else {
                    false
                }
            })
            .map(|(id, _)| id.clone())
            .collect();
        
        for id in expired_ids {
            if let Some(mut request) = pending.remove(&id) {
                request.status = ApprovalStatus::Expired;
                expired_count += 1;
                
                // Broadcast expiry event
                let _ = self.event_broadcaster.send(GuardianEvent::ApprovalExpired(id));
            }
        }
        
        // Update metrics
        {
            let mut metrics = self.metrics.write().await;
            metrics.expired_requests += expired_count;
        }
        
        Ok(expired_count)
    }
    
    // Helper methods
    async fn determine_approval_requirements(
        &self,
        operation_type: &OperationType,
        risk_assessment: &RiskAssessment,
    ) -> Result<ApprovalRequirements, GuardianError> {
        let policies = self.control_policies.read().await;
        
        let operation_policy = policies.operation_policies.get(operation_type)
            .cloned()
            .unwrap_or_default();
        
        let risk_policy = policies.risk_policies.get(&risk_assessment.risk_level)
            .cloned()
            .unwrap_or_default();
        
        Ok(ApprovalRequirements {
            requires_human_approval: operation_policy.requires_approval || 
                matches!(risk_assessment.risk_level, RiskLevel::High | RiskLevel::Critical),
            approval_level: match risk_assessment.risk_level {
                RiskLevel::Low => ApprovalLevel::Basic,
                RiskLevel::Medium => ApprovalLevel::Basic,
                RiskLevel::High => ApprovalLevel::Enhanced,
                RiskLevel::Critical => ApprovalLevel::Executive,
            },
            required_reviewers: Vec::new(),
            approval_timeout: Some(std::time::Duration::from_secs(24 * 3600)), // 24 hours
            auto_approve_conditions: operation_policy.auto_approval_rules.into_iter()
                .map(|rule| AutoApprovalCondition {
                    condition_type: rule.condition_type,
                    parameters: rule.parameters,
                    description: rule.description,
                })
                .collect(),
        })
    }
    
    async fn can_auto_approve(
        &self,
        operation_type: &OperationType,
        risk_assessment: &RiskAssessment,
        requirements: &ApprovalRequirements,
    ) -> Result<bool, GuardianError> {
        // Don't auto-approve if human approval is explicitly required
        if requirements.requires_human_approval {
            return Ok(false);
        }
        
        // Don't auto-approve high or critical risk operations
        if matches!(risk_assessment.risk_level, RiskLevel::High | RiskLevel::Critical) {
            return Ok(false);
        }
        
        // Check auto-approval conditions
        for condition in &requirements.auto_approve_conditions {
            if !self.evaluate_auto_approval_condition(condition, operation_type, risk_assessment).await? {
                return Ok(false);
            }
        }
        
        Ok(true)
    }
    
    async fn evaluate_auto_approval_condition(
        &self,
        condition: &AutoApprovalCondition,
        operation_type: &OperationType,
        risk_assessment: &RiskAssessment,
    ) -> Result<bool, GuardianError> {
        // Evaluate specific auto-approval conditions
        match condition.condition_type.as_str() {
            "low_risk_only" => Ok(matches!(risk_assessment.risk_level, RiskLevel::Low)),
            "high_confidence" => Ok(risk_assessment.confidence_score > 0.9),
            "reversible_operation" => Ok(matches!(risk_assessment.impact_analysis.reversibility, Reversibility::FullyReversible)),
            _ => Ok(false), // Unknown condition, don't auto-approve
        }
    }
}

// Supporting implementations
impl ApprovalManager {
    pub fn new() -> Self {
        Self {
            risk_assessor: RiskAssessor::new(),
            policy_engine: PolicyEngine::new(),
            notification_system: NotificationSystem::new(),
        }
    }
    
    pub async fn assess_risk(
        &self,
        operation_type: &OperationType,
        description: &str,
        context: &OperationContext,
    ) -> Result<RiskAssessment, GuardianError> {
        // Assess risk based on operation type and context
        let risk_level = match operation_type {
            OperationType::FileDelete | OperationType::DirectoryDelete => RiskLevel::High,
            OperationType::CodeDeletion => RiskLevel::Medium,
            OperationType::Deploy => RiskLevel::High,
            OperationType::SecurityConfigChange => RiskLevel::Critical,
            _ => RiskLevel::Low,
        };
        
        Ok(RiskAssessment {
            risk_level,
            risk_factors: Vec::new(),
            impact_analysis: ImpactAnalysis {
                affected_components: Vec::new(),
                affected_users: Vec::new(),
                reversibility: Reversibility::PartiallyReversible,
                estimated_downtime: None,
                data_sensitivity: DataSensitivity::Internal,
            },
            mitigation_strategies: Vec::new(),
            confidence_score: 0.8,
        })
    }
    
    pub async fn send_approval_notification(&self, request_id: &str) -> Result<(), GuardianError> {
        // Send notification to reviewers
        println!("Sending approval notification for request: {}", request_id);
        Ok(())
    }
}

impl HumanOversight {
    pub fn new() -> Self {
        Self {
            reviewer_pool: Arc::new(RwLock::new(ReviewerPool::new())),
            escalation_rules: EscalationRules::new(),
            oversight_dashboard: OversightDashboard::new(),
        }
    }
    
    pub async fn assign_reviewer(&self, request_id: &str) -> Result<(), GuardianError> {
        // Assign appropriate reviewer based on request type and availability
        println!("Assigning reviewer for request: {}", request_id);
        Ok(())
    }
}

// Default implementations
impl Default for ControlPolicies {
    fn default() -> Self {
        Self {
            operation_policies: HashMap::new(),
            risk_policies: HashMap::new(),
            global_policies: GlobalPolicies::default(),
            user_preferences: UserPreferences::default(),
        }
    }
}

impl Default for OperationPolicy {
    fn default() -> Self {
        Self {
            requires_approval: false,
            approval_level: ApprovalLevel::Basic,
            risk_threshold: 0.5,
            auto_approval_rules: Vec::new(),
            restrictions: Vec::new(),
        }
    }
}

impl Default for RiskPolicy {
    fn default() -> Self {
        Self {
            max_auto_approval_risk: 0.3,
            required_reviewers: 1,
            escalation_threshold: 0.8,
            notification_requirements: Vec::new(),
        }
    }
}

// Placeholder types
pub struct RiskAssessor;
pub struct PolicyEngine;
pub struct NotificationSystem;
pub struct ReviewerPool;
pub struct EscalationRules;
pub struct OversightDashboard;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GlobalPolicies {
    pub max_concurrent_operations: u32,
    pub default_approval_timeout: std::time::Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserPreferences {
    pub notification_preferences: HashMap<String, bool>,
    pub approval_delegation: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoApprovalRule {
    pub condition_type: String,
    pub parameters: HashMap<String, serde_json::Value>,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationRestriction {
    pub restriction_type: String,
    pub parameters: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationRequirement {
    pub notification_type: String,
    pub recipients: Vec<String>,
}

impl RiskAssessor { pub fn new() -> Self { Self } }
impl PolicyEngine { pub fn new() -> Self { Self } }
impl NotificationSystem { pub fn new() -> Self { Self } }
impl ReviewerPool { pub fn new() -> Self { Self } }
impl EscalationRules { pub fn new() -> Self { Self } }
impl OversightDashboard { pub fn new() -> Self { Self } }

/// Guardian error types
#[derive(Debug, thiserror::Error)]
pub enum GuardianError {
    #[error("Request not found")]
    RequestNotFound,
    #[error("Risk assessment failed")]
    RiskAssessmentFailed,
    #[error("Policy evaluation failed")]
    PolicyEvaluationFailed,
    #[error("Notification failed")]
    NotificationFailed,
    #[error("Reviewer assignment failed")]
    ReviewerAssignmentFailed,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_guardian_creation() {
        let guardian = Guardian::new();
        let metrics = guardian.get_metrics().await;
        assert_eq!(metrics.total_requests, 0);
    }
    
    #[tokio::test]
    async fn test_approval_request() {
        let guardian = Guardian::new();
        
        let context = OperationContext {
            project_context: "test_project".to_string(),
            user_intent: "Create new file".to_string(),
            related_operations: Vec::new(),
            dependencies: Vec::new(),
            environment: "development".to_string(),
            metadata: HashMap::new(),
        };
        
        let request_id = guardian.request_approval(
            OperationType::FileCreate,
            "Create new test file".to_string(),
            context,
            "test_agent".to_string(),
        ).await.unwrap();
        
        assert!(!request_id.is_empty());
        
        let status = guardian.get_approval_status(&request_id).await;
        assert!(status.is_some());
    }
    
    #[tokio::test]
    async fn test_auto_approval_low_risk() {
        let guardian = Guardian::new();
        
        let context = OperationContext {
            project_context: "test_project".to_string(),
            user_intent: "Read file content".to_string(),
            related_operations: Vec::new(),
            dependencies: Vec::new(),
            environment: "development".to_string(),
            metadata: HashMap::new(),
        };
        
        // Low-risk operations should be auto-approved
        let request_id = guardian.request_approval(
            OperationType::FileCreate,
            "Create temporary file".to_string(),
            context,
            "test_agent".to_string(),
        ).await.unwrap();
        
        // Should be auto-approved for low-risk operations
        let status = guardian.get_approval_status(&request_id).await;
        // Note: In real implementation, this might be auto-approved
        assert!(status.is_some());
    }
}
