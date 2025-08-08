use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, Mutex};
use uuid::Uuid;

use crate::agent_runtime::{AgentType, AgentError, TaskResult, AgentTask};

/// Agent communication protocol with strict output protection
pub struct AgentCommunicationProtocol {
    message_router: Arc<MessageRouter>,
    output_protector: Arc<OutputProtector>,
    context_router: Arc<ContextRouter>,
    violation_monitor: Arc<ViolationMonitor>,
    handoff_manager: Arc<HandoffManager>,
}

impl AgentCommunicationProtocol {
    pub fn new() -> Self {
        Self {
            message_router: Arc::new(MessageRouter::new()),
            output_protector: Arc::new(OutputProtector::new()),
            context_router: Arc::new(ContextRouter::new()),
            violation_monitor: Arc::new(ViolationMonitor::new()),
            handoff_manager: Arc::new(HandoffManager::new()),
        }
    }
    
    /// Send a message between agents with protocol enforcement
    pub async fn send_agent_message(
        &self,
        from_agent: AgentType,
        to_agent: AgentType,
        message: AgentMessage,
    ) -> Result<(), CommunicationError> {
        // 1. Validate message against protocol rules
        self.validate_message(&from_agent, &to_agent, &message).await?;
        
        // 2. Apply output protection
        let protected_message = self.output_protector.protect_message(message, &from_agent, &to_agent).await?;
        
        // 3. Route message through appropriate channels
        self.message_router.route_message(from_agent, to_agent, protected_message).await?;
        
        Ok(())
    }
    
    /// Execute agent handoff with context transformation
    pub async fn execute_handoff(
        &self,
        from_agent: AgentType,
        to_agent: AgentType,
        task_result: TaskResult,
        context: AgentContext,
    ) -> Result<HandoffResult, CommunicationError> {
        self.handoff_manager.execute_handoff(from_agent, to_agent, task_result, context).await
    }
    
    /// Validate message against communication protocol
    async fn validate_message(
        &self,
        from_agent: &AgentType,
        to_agent: &AgentType,
        message: &AgentMessage,
    ) -> Result<(), CommunicationError> {
        // Check if communication is allowed between these agent types
        if !self.is_communication_allowed(from_agent, to_agent) {
            return Err(CommunicationError::UnauthorizedCommunication(
                format!("{:?} -> {:?}", from_agent, to_agent)
            ));
        }
        
        // Validate message content doesn't violate boundaries
        self.validate_message_content(message, from_agent, to_agent).await?;
        
        // Check rate limits
        self.check_rate_limits(from_agent, to_agent).await?;
        
        Ok(())
    }
    
    /// Check if communication is allowed between agent types
    fn is_communication_allowed(&self, from_agent: &AgentType, to_agent: &AgentType) -> bool {
        match (from_agent, to_agent) {
            // Orchestrator can communicate with all agents
            (AgentType::Orchestrator, _) | (_, AgentType::Orchestrator) => true,
            
            // Context agent can communicate with all agents
            (AgentType::Context, _) | (_, AgentType::Context) => true,
            
            // Architect can communicate with implementation agents
            (AgentType::Architect, AgentType::Developer) => true,
            (AgentType::Architect, AgentType::RustSpecialist) => true,
            (AgentType::Architect, AgentType::ReactSpecialist) => true,
            
            // Developer can communicate with specialists
            (AgentType::Developer, AgentType::RustSpecialist) => true,
            (AgentType::Developer, AgentType::ReactSpecialist) => true,
            (AgentType::Developer, AgentType::TypeScriptSpecialist) => true,
            
            // Specialists can communicate with tester and reviewer
            (AgentType::RustSpecialist, AgentType::Tester) => true,
            (AgentType::ReactSpecialist, AgentType::Tester) => true,
            (AgentType::TypeScriptSpecialist, AgentType::Tester) => true,
            
            // Any agent can communicate with debug agent
            (_, AgentType::Debug) => true,
            
            // Security agent can review any agent's output
            (_, AgentType::Security) => true,
            (AgentType::Security, _) => true,
            
            // Performance optimizer can analyze any agent's output
            (_, AgentType::PerformanceOptimizer) => true,
            
            // Documentation agent can document any agent's work
            (_, AgentType::Documentation) => true,
            
            // Default: no communication allowed
            _ => false,
        }
    }
    
    async fn validate_message_content(
        &self,
        message: &AgentMessage,
        from_agent: &AgentType,
        to_agent: &AgentType,
    ) -> Result<(), CommunicationError> {
        // Check for attempts to modify other agent's outputs
        if self.contains_output_modification_attempt(&message.content) {
            self.violation_monitor.record_violation(
                from_agent.clone(),
                ViolationType::OutputModificationAttempt,
                format!("Attempted to modify output in message to {:?}", to_agent),
            ).await;
            return Err(CommunicationError::ProtocolViolation(
                "Attempted to modify another agent's output".to_string()
            ));
        }
        
        // Check for role boundary violations
        if self.violates_role_boundaries(&message.content, from_agent, to_agent) {
            self.violation_monitor.record_violation(
                from_agent.clone(),
                ViolationType::RoleBoundaryViolation,
                format!("Role boundary violation in message to {:?}", to_agent),
            ).await;
            return Err(CommunicationError::ProtocolViolation(
                "Message violates role boundaries".to_string()
            ));
        }
        
        Ok(())
    }
    
    fn contains_output_modification_attempt(&self, content: &str) -> bool {
        let forbidden_patterns = [
            "modify the output of",
            "change the result from",
            "override the decision by",
            "replace the code written by",
            "alter the implementation from",
        ];
        
        forbidden_patterns.iter().any(|pattern| content.to_lowercase().contains(pattern))
    }
    
    fn violates_role_boundaries(&self, content: &str, from_agent: &AgentType, to_agent: &AgentType) -> bool {
        // Check if agent is trying to perform tasks outside its role
        match from_agent {
            AgentType::RustSpecialist => {
                // Rust specialist shouldn't give React-specific advice
                if matches!(to_agent, AgentType::ReactSpecialist) {
                    return content.to_lowercase().contains("react") && 
                           (content.contains("useState") || content.contains("useEffect"));
                }
            },
            AgentType::ReactSpecialist => {
                // React specialist shouldn't give Rust-specific advice
                if matches!(to_agent, AgentType::RustSpecialist) {
                    return content.to_lowercase().contains("rust") && 
                           (content.contains("ownership") || content.contains("borrowing"));
                }
            },
            AgentType::Security => {
                // Security agent shouldn't implement features, only review
                return content.contains("implement this feature") || content.contains("write this code");
            },
            _ => {}
        }
        
        false
    }
    
    async fn check_rate_limits(&self, from_agent: &AgentType, to_agent: &AgentType) -> Result<(), CommunicationError> {
        // Implement rate limiting to prevent message spam
        // This is a simplified implementation
        Ok(())
    }
}

/// Message router for agent communication
pub struct MessageRouter {
    channels: Arc<RwLock<HashMap<AgentType, mpsc::UnboundedSender<RoutedMessage>>>>,
    message_history: Arc<RwLock<Vec<MessageRecord>>>,
}

impl MessageRouter {
    pub fn new() -> Self {
        Self {
            channels: Arc::new(RwLock::new(HashMap::new())),
            message_history: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    pub async fn route_message(
        &self,
        from_agent: AgentType,
        to_agent: AgentType,
        message: AgentMessage,
    ) -> Result<(), CommunicationError> {
        let routed_message = RoutedMessage {
            id: Uuid::new_v4().to_string(),
            from_agent: from_agent.clone(),
            to_agent: to_agent.clone(),
            message,
            timestamp: Instant::now(),
            delivery_attempts: 0,
        };
        
        // Record message in history
        {
            let mut history = self.message_history.write().unwrap();
            history.push(MessageRecord {
                id: routed_message.id.clone(),
                from_agent: from_agent.clone(),
                to_agent: to_agent.clone(),
                timestamp: routed_message.timestamp,
                delivered: false,
            });
        }
        
        // Route to appropriate channel
        let channels = self.channels.read().unwrap();
        if let Some(sender) = channels.get(&to_agent) {
            sender.send(routed_message)
                .map_err(|e| CommunicationError::DeliveryFailed(format!("Failed to route message: {}", e)))?;
        } else {
            return Err(CommunicationError::AgentNotAvailable(format!("{:?}", to_agent)));
        }
        
        Ok(())
    }
    
    pub async fn register_agent_channel(&self, agent_type: AgentType, sender: mpsc::UnboundedSender<RoutedMessage>) {
        let mut channels = self.channels.write().unwrap();
        channels.insert(agent_type, sender);
    }
}

/// Output protection system
pub struct OutputProtector {
    protection_rules: Arc<RwLock<HashMap<AgentType, ProtectionRules>>>,
}

impl OutputProtector {
    pub fn new() -> Self {
        let mut protector = Self {
            protection_rules: Arc::new(RwLock::new(HashMap::new())),
        };
        protector.initialize_protection_rules();
        protector
    }
    
    fn initialize_protection_rules(&mut self) {
        let mut rules = self.protection_rules.write().unwrap();
        
        // Core agents have strict output protection
        rules.insert(AgentType::Orchestrator, ProtectionRules {
            output_immutable: true,
            context_filtering: ContextFilterLevel::Strict,
            allowed_modifications: vec![],
            sensitive_patterns: vec!["execution_plan".to_string(), "agent_coordination".to_string()],
        });
        
        rules.insert(AgentType::Architect, ProtectionRules {
            output_immutable: true,
            context_filtering: ContextFilterLevel::Moderate,
            allowed_modifications: vec![],
            sensitive_patterns: vec!["architecture_decision".to_string(), "design_pattern".to_string()],
        });
        
        // Specialist agents have moderate protection
        rules.insert(AgentType::RustSpecialist, ProtectionRules {
            output_immutable: false,
            context_filtering: ContextFilterLevel::Moderate,
            allowed_modifications: vec![ModificationType::CodeOptimization, ModificationType::BugFix],
            sensitive_patterns: vec!["unsafe".to_string(), "memory_management".to_string()],
        });
        
        rules.insert(AgentType::ReactSpecialist, ProtectionRules {
            output_immutable: false,
            context_filtering: ContextFilterLevel::Moderate,
            allowed_modifications: vec![ModificationType::CodeOptimization, ModificationType::StyleImprovement],
            sensitive_patterns: vec!["state_management".to_string(), "component_lifecycle".to_string()],
        });
        
        // Debug agent has special privileges
        rules.insert(AgentType::Debug, ProtectionRules {
            output_immutable: false,
            context_filtering: ContextFilterLevel::Minimal,
            allowed_modifications: vec![ModificationType::BugFix, ModificationType::DiagnosticImprovement],
            sensitive_patterns: vec!["error_trace".to_string(), "debug_info".to_string()],
        });
    }
    
    pub async fn protect_message(
        &self,
        mut message: AgentMessage,
        from_agent: &AgentType,
        to_agent: &AgentType,
    ) -> Result<AgentMessage, CommunicationError> {
        let rules = self.protection_rules.read().unwrap();
        
        // Apply sender's protection rules
        if let Some(sender_rules) = rules.get(from_agent) {
            message = self.apply_sender_protection(message, sender_rules)?;
        }
        
        // Apply receiver's filtering rules
        if let Some(receiver_rules) = rules.get(to_agent) {
            message = self.apply_receiver_filtering(message, receiver_rules)?;
        }
        
        Ok(message)
    }
    
    fn apply_sender_protection(&self, mut message: AgentMessage, rules: &ProtectionRules) -> Result<AgentMessage, CommunicationError> {
        // Filter out sensitive patterns
        for pattern in &rules.sensitive_patterns {
            if message.content.contains(pattern) {
                message.content = message.content.replace(pattern, "[PROTECTED]");
            }
        }
        
        // Apply context filtering
        match rules.context_filtering {
            ContextFilterLevel::Strict => {
                message.context = message.context.map(|ctx| self.filter_context_strict(ctx));
            },
            ContextFilterLevel::Moderate => {
                message.context = message.context.map(|ctx| self.filter_context_moderate(ctx));
            },
            ContextFilterLevel::Minimal => {
                // Minimal filtering
            },
        }
        
        Ok(message)
    }
    
    fn apply_receiver_filtering(&self, message: AgentMessage, _rules: &ProtectionRules) -> Result<AgentMessage, CommunicationError> {
        // Apply receiver-specific filtering
        Ok(message)
    }
    
    fn filter_context_strict(&self, context: AgentContext) -> AgentContext {
        // Remove sensitive context information
        AgentContext {
            task_context: context.task_context,
            relevant_code: None, // Remove code context
            file_context: None,  // Remove file context
            project_context: context.project_context,
            agent_context: None, // Remove agent-specific context
        }
    }
    
    fn filter_context_moderate(&self, mut context: AgentContext) -> AgentContext {
        // Partially filter context
        if let Some(ref mut code_context) = context.relevant_code {
            // Remove sensitive code sections
            code_context.retain(|_, content| !content.contains("password") && !content.contains("secret"));
        }
        context
    }
}

/// Context routing for intelligent context delivery
pub struct ContextRouter {
    routing_rules: Arc<RwLock<HashMap<(AgentType, AgentType), ContextRoutingRule>>>,
}

impl ContextRouter {
    pub fn new() -> Self {
        let mut router = Self {
            routing_rules: Arc::new(RwLock::new(HashMap::new())),
        };
        router.initialize_routing_rules();
        router
    }
    
    fn initialize_routing_rules(&mut self) {
        let mut rules = self.routing_rules.write().unwrap();
        
        // Architect -> Developer: Pass design decisions and constraints
        rules.insert((AgentType::Architect, AgentType::Developer), ContextRoutingRule {
            allowed_context_types: vec!["design_decisions".to_string(), "constraints".to_string()],
            context_transformation: ContextTransformation::DesignToImplementation,
            max_context_size: 32000,
        });
        
        // Developer -> Specialist: Pass implementation requirements
        rules.insert((AgentType::Developer, AgentType::RustSpecialist), ContextRoutingRule {
            allowed_context_types: vec!["implementation_spec".to_string(), "rust_requirements".to_string()],
            context_transformation: ContextTransformation::GeneralToSpecialized,
            max_context_size: 24000,
        });
        
        // Any -> Debug: Pass error context only
        for agent_type in [AgentType::Developer, AgentType::RustSpecialist, AgentType::ReactSpecialist] {
            rules.insert((agent_type, AgentType::Debug), ContextRoutingRule {
                allowed_context_types: vec!["error_context".to_string(), "failing_code".to_string()],
                context_transformation: ContextTransformation::ErrorFocused,
                max_context_size: 16000,
            });
        }
    }
    
    pub async fn route_context(
        &self,
        from_agent: AgentType,
        to_agent: AgentType,
        context: AgentContext,
    ) -> Result<AgentContext, CommunicationError> {
        let rules = self.routing_rules.read().unwrap();
        
        if let Some(rule) = rules.get(&(from_agent, to_agent)) {
            self.apply_routing_rule(context, rule)
        } else {
            // Default: minimal context
            Ok(AgentContext {
                task_context: context.task_context,
                relevant_code: None,
                file_context: None,
                project_context: None,
                agent_context: None,
            })
        }
    }
    
    fn apply_routing_rule(&self, context: AgentContext, rule: &ContextRoutingRule) -> Result<AgentContext, CommunicationError> {
        match rule.context_transformation {
            ContextTransformation::DesignToImplementation => {
                Ok(AgentContext {
                    task_context: context.task_context,
                    relevant_code: context.relevant_code,
                    file_context: None, // Remove file context for implementation
                    project_context: context.project_context,
                    agent_context: None,
                })
            },
            ContextTransformation::GeneralToSpecialized => {
                Ok(AgentContext {
                    task_context: context.task_context,
                    relevant_code: context.relevant_code,
                    file_context: context.file_context,
                    project_context: None, // Remove general project context
                    agent_context: None,
                })
            },
            ContextTransformation::ErrorFocused => {
                Ok(AgentContext {
                    task_context: context.task_context,
                    relevant_code: context.relevant_code, // Keep only failing code
                    file_context: None,
                    project_context: None,
                    agent_context: None,
                })
            },
        }
    }
}

/// Violation monitoring and enforcement
pub struct ViolationMonitor {
    violations: Arc<RwLock<Vec<ProtocolViolation>>>,
    violation_counts: Arc<RwLock<HashMap<AgentType, u32>>>,
}

impl ViolationMonitor {
    pub fn new() -> Self {
        Self {
            violations: Arc::new(RwLock::new(Vec::new())),
            violation_counts: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    pub async fn record_violation(&self, agent: AgentType, violation_type: ViolationType, description: String) {
        let violation = ProtocolViolation {
            id: Uuid::new_v4().to_string(),
            agent: agent.clone(),
            violation_type,
            description,
            timestamp: Instant::now(),
            severity: ViolationSeverity::Medium,
        };
        
        // Record violation
        {
            let mut violations = self.violations.write().unwrap();
            violations.push(violation);
        }
        
        // Update violation count
        {
            let mut counts = self.violation_counts.write().unwrap();
            *counts.entry(agent).or_insert(0) += 1;
        }
    }
    
    pub async fn get_violation_report(&self, agent: &AgentType) -> ViolationReport {
        let violations = self.violations.read().unwrap();
        let counts = self.violation_counts.read().unwrap();
        
        let agent_violations: Vec<_> = violations.iter()
            .filter(|v| v.agent == *agent)
            .cloned()
            .collect();
        
        ViolationReport {
            agent: agent.clone(),
            total_violations: counts.get(agent).copied().unwrap_or(0),
            recent_violations: agent_violations,
            risk_level: self.calculate_risk_level(agent, &counts),
        }
    }
    
    fn calculate_risk_level(&self, agent: &AgentType, counts: &HashMap<AgentType, u32>) -> RiskLevel {
        let violation_count = counts.get(agent).copied().unwrap_or(0);
        
        match violation_count {
            0 => RiskLevel::Low,
            1..=3 => RiskLevel::Medium,
            4..=10 => RiskLevel::High,
            _ => RiskLevel::Critical,
        }
    }
}

/// Agent handoff management
pub struct HandoffManager {
    handoff_protocols: Arc<RwLock<HashMap<(AgentType, AgentType), HandoffProtocol>>>,
    context_transformers: Arc<RwLock<HashMap<String, Box<dyn ContextTransformer>>>>,
}

impl HandoffManager {
    pub fn new() -> Self {
        Self {
            handoff_protocols: Arc::new(RwLock::new(HashMap::new())),
            context_transformers: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    pub async fn execute_handoff(
        &self,
        from_agent: AgentType,
        to_agent: AgentType,
        task_result: TaskResult,
        context: AgentContext,
    ) -> Result<HandoffResult, CommunicationError> {
        let handoff_key = (from_agent.clone(), to_agent.clone());
        let protocols = self.handoff_protocols.read().unwrap();
        
        if let Some(protocol) = protocols.get(&handoff_key) {
            self.execute_with_protocol(from_agent, to_agent, task_result, context, protocol).await
        } else {
            self.execute_default_handoff(from_agent, to_agent, task_result, context).await
        }
    }
    
    async fn execute_with_protocol(
        &self,
        from_agent: AgentType,
        to_agent: AgentType,
        task_result: TaskResult,
        context: AgentContext,
        protocol: &HandoffProtocol,
    ) -> Result<HandoffResult, CommunicationError> {
        // Extract relevant information based on protocol
        let extracted_info = self.extract_relevant_info(&task_result, protocol)?;
        
        // Transform context for target agent
        let transformed_context = self.transform_context_for_agent(context, &from_agent, &to_agent, &extracted_info)?;
        
        Ok(HandoffResult {
            transformed_context,
            extracted_info,
            continuity_score: 0.95, // Mock score
            compression_ratio: 0.7,  // Mock ratio
        })
    }
    
    async fn execute_default_handoff(
        &self,
        _from_agent: AgentType,
        _to_agent: AgentType,
        task_result: TaskResult,
        context: AgentContext,
    ) -> Result<HandoffResult, CommunicationError> {
        // Default handoff with minimal context transformation
        Ok(HandoffResult {
            transformed_context: context,
            extracted_info: ExtractedInfo {
                key_decisions: vec![],
                implementation_details: HashMap::new(),
                constraints: vec![],
                recommendations: vec![],
            },
            continuity_score: 0.8,
            compression_ratio: 1.0,
        })
    }
    
    fn extract_relevant_info(&self, task_result: &TaskResult, _protocol: &HandoffProtocol) -> Result<ExtractedInfo, CommunicationError> {
        // Extract relevant information from task result
        Ok(ExtractedInfo {
            key_decisions: vec!["Mock decision".to_string()],
            implementation_details: HashMap::new(),
            constraints: vec!["Mock constraint".to_string()],
            recommendations: vec!["Mock recommendation".to_string()],
        })
    }
    
    fn transform_context_for_agent(
        &self,
        context: AgentContext,
        _from_agent: &AgentType,
        _to_agent: &AgentType,
        _extracted_info: &ExtractedInfo,
    ) -> Result<AgentContext, CommunicationError> {
        // Transform context based on target agent needs
        Ok(context)
    }
}

// Supporting types and structures

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessage {
    pub id: String,
    pub message_type: MessageType,
    pub content: String,
    pub context: Option<AgentContext>,
    pub timestamp: Instant,
    pub priority: MessagePriority,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MessageType {
    TaskRequest,
    TaskResult,
    ContextUpdate,
    StatusUpdate,
    Error,
    Handoff,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MessagePriority {
    Low,
    Normal,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentContext {
    pub task_context: String,
    pub relevant_code: Option<HashMap<String, String>>,
    pub file_context: Option<HashMap<String, String>>,
    pub project_context: Option<String>,
    pub agent_context: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RoutedMessage {
    pub id: String,
    pub from_agent: AgentType,
    pub to_agent: AgentType,
    pub message: AgentMessage,
    pub timestamp: Instant,
    pub delivery_attempts: u32,
}

#[derive(Debug, Clone)]
pub struct MessageRecord {
    pub id: String,
    pub from_agent: AgentType,
    pub to_agent: AgentType,
    pub timestamp: Instant,
    pub delivered: bool,
}

#[derive(Debug, Clone)]
pub struct ProtectionRules {
    pub output_immutable: bool,
    pub context_filtering: ContextFilterLevel,
    pub allowed_modifications: Vec<ModificationType>,
    pub sensitive_patterns: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ContextFilterLevel {
    Strict,
    Moderate,
    Minimal,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ModificationType {
    CodeOptimization,
    BugFix,
    StyleImprovement,
    DiagnosticImprovement,
}

#[derive(Debug, Clone)]
pub struct ContextRoutingRule {
    pub allowed_context_types: Vec<String>,
    pub context_transformation: ContextTransformation,
    pub max_context_size: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ContextTransformation {
    DesignToImplementation,
    GeneralToSpecialized,
    ErrorFocused,
}

#[derive(Debug, Clone)]
pub struct ProtocolViolation {
    pub id: String,
    pub agent: AgentType,
    pub violation_type: ViolationType,
    pub description: String,
    pub timestamp: Instant,
    pub severity: ViolationSeverity,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ViolationType {
    OutputModificationAttempt,
    RoleBoundaryViolation,
    UnauthorizedCommunication,
    ContextLeakage,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ViolationSeverity {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone)]
pub struct ViolationReport {
    pub agent: AgentType,
    pub total_violations: u32,
    pub recent_violations: Vec<ProtocolViolation>,
    pub risk_level: RiskLevel,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone)]
pub struct HandoffProtocol {
    pub required_info: Vec<String>,
    pub context_transformation: ContextTransformation,
    pub validation_rules: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct HandoffResult {
    pub transformed_context: AgentContext,
    pub extracted_info: ExtractedInfo,
    pub continuity_score: f32,
    pub compression_ratio: f32,
}

#[derive(Debug, Clone)]
pub struct ExtractedInfo {
    pub key_decisions: Vec<String>,
    pub implementation_details: HashMap<String, String>,
    pub constraints: Vec<String>,
    pub recommendations: Vec<String>,
}

pub trait ContextTransformer: Send + Sync {
    fn transform(&self, context: AgentContext, extracted_info: &ExtractedInfo) -> Result<AgentContext, CommunicationError>;
}

#[derive(Debug, thiserror::Error)]
pub enum CommunicationError {
    #[error("Unauthorized communication: {0}")]
    UnauthorizedCommunication(String),
    
    #[error("Protocol violation: {0}")]
    ProtocolViolation(String),
    
    #[error("Message delivery failed: {0}")]
    DeliveryFailed(String),
    
    #[error("Agent not available: {0}")]
    AgentNotAvailable(String),
    
    #[error("Context transformation failed: {0}")]
    ContextTransformationFailed(String),
    
    #[error("Rate limit exceeded: {0}")]
    RateLimitExceeded(String),
}

/// Type alias for compatibility
pub type CommunicationProtocol = AgentCommunicationProtocol;
