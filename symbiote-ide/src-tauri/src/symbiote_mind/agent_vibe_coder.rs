// Agent Vibe Coder - Agents Building Agents
// Phase 4 Feature: Revolutionary system where AI agents create, modify, and optimize other AI agents

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Agent Vibe Coder - The meta-agent system for agent creation and optimization
pub struct AgentVibeCoder {
    // Core agent creation system
    agent_architect: AgentArchitect,
    code_generator: AgentCodeGenerator,
    behavior_synthesizer: BehaviorSynthesizer,
    
    // Agent optimization and evolution
    performance_optimizer: PerformanceOptimizer,
    capability_enhancer: CapabilityEnhancer,
    evolution_engine: EvolutionEngine,
    
    // Agent templates and patterns
    template_library: Arc<RwLock<HashMap<String, AgentTemplate>>>,
    pattern_registry: Arc<RwLock<HashMap<String, AgentPattern>>>,
    
    // Agent lifecycle management
    lifecycle_manager: AgentLifecycleManager,
    version_control: AgentVersionControl,
    
    // Quality assurance and testing
    agent_tester: AgentTester,
    quality_assessor: QualityAssessor,
    safety_validator: SafetyValidator,
    
    // Collaboration and learning
    collaboration_engine: CollaborationEngine,
    learning_system: AgentLearningSystem,
    
    // Registry and tracking
    created_agents: Arc<RwLock<HashMap<String, CreatedAgent>>>,
    metrics: Arc<RwLock<VibeCoderMetrics>>,
}

/// Agent creation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCreationRequest {
    pub request_id: String,
    pub creator_agent_id: String,
    pub agent_specification: AgentSpecification,
    pub creation_context: CreationContext,
    pub optimization_goals: Vec<OptimizationGoal>,
    pub constraints: CreationConstraints,
    pub timeline: CreationTimeline,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSpecification {
    pub name: String,
    pub description: String,
    pub purpose: String,
    pub domain: AgentDomain,
    pub capabilities: Vec<AgentCapability>,
    pub behavioral_traits: Vec<BehavioralTrait>,
    pub interaction_patterns: Vec<InteractionPattern>,
    pub knowledge_requirements: Vec<KnowledgeRequirement>,
    pub performance_requirements: PerformanceRequirements,
    pub ethical_guidelines: Vec<EthicalGuideline>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentDomain {
    // Development domains
    CodeGeneration,
    Testing,
    Debugging,
    Refactoring,
    Documentation,
    
    // Analysis domains
    CodeAnalysis,
    PerformanceAnalysis,
    SecurityAnalysis,
    QualityAssurance,
    
    // Project management
    Planning,
    Coordination,
    Monitoring,
    Reporting,
    
    // Specialized domains
    MachineLearning,
    DataScience,
    DevOps,
    UI_UX,
    Architecture,
    
    // Meta domains
    AgentCreation,
    AgentOptimization,
    AgentCoordination,
    
    // Custom domain
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCapability {
    pub capability_id: String,
    pub name: String,
    pub description: String,
    pub capability_type: CapabilityType,
    pub proficiency_level: ProficiencyLevel,
    pub dependencies: Vec<String>,
    pub implementation_hints: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CapabilityType {
    // Core capabilities
    Reasoning,
    ProblemSolving,
    PatternRecognition,
    Learning,
    
    // Technical capabilities
    CodeGeneration,
    CodeAnalysis,
    Testing,
    Debugging,
    
    // Communication capabilities
    NaturalLanguage,
    TechnicalWriting,
    Collaboration,
    Explanation,
    
    // Domain-specific
    DomainExpertise(String),
    ToolUsage(String),
    APIIntegration(String),
    
    // Meta capabilities
    SelfReflection,
    SelfImprovement,
    AgentCreation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProficiencyLevel {
    Novice,
    Intermediate,
    Advanced,
    Expert,
    Master,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehavioralTrait {
    pub trait_id: String,
    pub name: String,
    pub description: String,
    pub trait_type: TraitType,
    pub intensity: f64, // 0.0 to 1.0
    pub context_dependent: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TraitType {
    // Personality traits
    Curiosity,
    Persistence,
    Creativity,
    Analytical,
    Methodical,
    
    // Work style traits
    Collaborative,
    Independent,
    DetailOriented,
    BigPicture,
    RiskTaking,
    
    // Communication traits
    Verbose,
    Concise,
    Formal,
    Casual,
    Empathetic,
    
    // Learning traits
    Adaptive,
    Conservative,
    Experimental,
    Systematic,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractionPattern {
    pub pattern_id: String,
    pub name: String,
    pub description: String,
    pub trigger_conditions: Vec<String>,
    pub response_strategy: ResponseStrategy,
    pub collaboration_style: CollaborationStyle,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResponseStrategy {
    Immediate,
    Thoughtful,
    Collaborative,
    Delegating,
    Escalating,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CollaborationStyle {
    Leader,
    Follower,
    Peer,
    Mentor,
    Student,
    Facilitator,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeRequirement {
    pub domain: String,
    pub depth_level: KnowledgeDepth,
    pub sources: Vec<KnowledgeSource>,
    pub update_frequency: UpdateFrequency,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KnowledgeDepth {
    Basic,
    Intermediate,
    Advanced,
    Expert,
    Cutting_Edge,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KnowledgeSource {
    Documentation,
    CodeRepositories,
    Research_Papers,
    Best_Practices,
    Community_Knowledge,
    Real_Time_Data,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UpdateFrequency {
    Static,
    Daily,
    Weekly,
    Monthly,
    Real_Time,
    On_Demand,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceRequirements {
    pub response_time_ms: u32,
    pub accuracy_threshold: f64,
    pub throughput_requests_per_second: u32,
    pub memory_limit_mb: u32,
    pub cpu_limit_percentage: f64,
    pub concurrent_tasks: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EthicalGuideline {
    pub guideline_id: String,
    pub principle: String,
    pub description: String,
    pub enforcement_level: EnforcementLevel,
    pub violation_consequences: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EnforcementLevel {
    Advisory,
    Warning,
    Blocking,
    Terminating,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreationContext {
    pub project_context: String,
    pub team_context: String,
    pub technical_stack: Vec<String>,
    pub existing_agents: Vec<String>,
    pub integration_requirements: Vec<String>,
    pub resource_constraints: ResourceConstraints,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceConstraints {
    pub max_memory_mb: u32,
    pub max_cpu_percentage: f64,
    pub max_network_bandwidth: u32,
    pub max_storage_mb: u32,
    pub budget_constraints: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationGoal {
    pub goal_id: String,
    pub name: String,
    pub description: String,
    pub goal_type: OptimizationGoalType,
    pub target_value: f64,
    pub priority: Priority,
    pub measurement_method: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OptimizationGoalType {
    Performance,
    Accuracy,
    Efficiency,
    Reliability,
    Maintainability,
    Scalability,
    UserSatisfaction,
    CostEffectiveness,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Priority {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreationConstraints {
    pub time_limit: std::time::Duration,
    pub complexity_limit: ComplexityLevel,
    pub dependency_restrictions: Vec<String>,
    pub platform_restrictions: Vec<String>,
    pub security_requirements: Vec<String>,
    pub compliance_requirements: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComplexityLevel {
    Simple,
    Moderate,
    Complex,
    Advanced,
    Expert,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreationTimeline {
    pub start_time: DateTime<Utc>,
    pub estimated_completion: DateTime<Utc>,
    pub milestones: Vec<CreationMilestone>,
    pub dependencies: Vec<TimelineDependency>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreationMilestone {
    pub milestone_id: String,
    pub name: String,
    pub description: String,
    pub target_date: DateTime<Utc>,
    pub completion_criteria: Vec<String>,
    pub deliverables: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineDependency {
    pub dependency_id: String,
    pub name: String,
    pub dependency_type: DependencyType,
    pub blocking_factor: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DependencyType {
    Sequential,
    Parallel,
    Conditional,
    Resource,
    External,
}

/// Created agent record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatedAgent {
    pub agent_id: String,
    pub name: String,
    pub creator_agent_id: String,
    pub specification: AgentSpecification,
    pub implementation: AgentImplementation,
    pub version: AgentVersion,
    pub status: AgentStatus,
    pub performance_metrics: PerformanceMetrics,
    pub learning_progress: LearningProgress,
    pub collaboration_history: Vec<CollaborationRecord>,
    pub created_at: DateTime<Utc>,
    pub last_updated: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentImplementation {
    pub implementation_id: String,
    pub architecture: AgentArchitecture,
    pub code_modules: Vec<CodeModule>,
    pub configuration: AgentConfiguration,
    pub runtime_environment: RuntimeEnvironment,
    pub integration_points: Vec<IntegrationPoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentArchitecture {
    pub architecture_type: ArchitectureType,
    pub components: Vec<ArchitectureComponent>,
    pub data_flow: Vec<DataFlowEdge>,
    pub decision_points: Vec<DecisionPoint>,
    pub feedback_loops: Vec<FeedbackLoop>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ArchitectureType {
    Reactive,
    Deliberative,
    Hybrid,
    Layered,
    Blackboard,
    Multi_Agent,
}

/// Supporting types and systems
pub struct AgentArchitect;
pub struct AgentCodeGenerator;
pub struct BehaviorSynthesizer;
pub struct PerformanceOptimizer;
pub struct CapabilityEnhancer;
pub struct EvolutionEngine;
pub struct AgentLifecycleManager;
pub struct AgentVersionControl;
pub struct AgentTester;
pub struct QualityAssessor;
pub struct SafetyValidator;
pub struct CollaborationEngine;
pub struct AgentLearningSystem;

// Additional supporting types will be defined in separate modules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchitectureComponent {
    pub component_id: String,
    pub name: String,
    pub component_type: ComponentType,
    pub responsibilities: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComponentType {
    Perception,
    Reasoning,
    Planning,
    Execution,
    Learning,
    Communication,
    Memory,
    Control,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataFlowEdge {
    pub from_component: String,
    pub to_component: String,
    pub data_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionPoint {
    pub decision_id: String,
    pub name: String,
    pub criteria: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackLoop {
    pub loop_id: String,
    pub name: String,
    pub components: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeModule {
    pub module_id: String,
    pub name: String,
    pub language: String,
    pub code: String,
    pub dependencies: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfiguration {
    pub config_id: String,
    pub parameters: HashMap<String, serde_json::Value>,
    pub feature_flags: HashMap<String, bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeEnvironment {
    pub environment_id: String,
    pub platform: String,
    pub runtime_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationPoint {
    pub integration_id: String,
    pub name: String,
    pub integration_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentVersion {
    pub version_id: String,
    pub version_number: String,
    pub release_notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentStatus {
    Creating,
    Testing,
    Active,
    Optimizing,
    Deprecated,
    Archived,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub response_time_avg: f64,
    pub accuracy_score: f64,
    pub throughput: f64,
    pub error_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningProgress {
    pub learning_milestones: Vec<String>,
    pub skill_improvements: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollaborationRecord {
    pub collaboration_id: String,
    pub partner_agent_id: String,
    pub collaboration_type: String,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTemplate {
    pub template_id: String,
    pub name: String,
    pub description: String,
    pub category: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPattern {
    pub pattern_id: String,
    pub name: String,
    pub description: String,
    pub pattern_type: String,
}

/// Metrics
#[derive(Debug, Default)]
pub struct VibeCoderMetrics {
    pub total_agents_created: u32,
    pub active_agents: u32,
    pub successful_creations: u32,
    pub failed_creations: u32,
    pub average_creation_time: f64,
}

/// Error types
#[derive(Debug, thiserror::Error)]
pub enum VibeCoderError {
    #[error("Invalid specification: {0}")]
    InvalidSpecification(String),
    #[error("Invalid constraints: {0}")]
    InvalidConstraints(String),
    #[error("Creation failed: {0}")]
    CreationFailed(String),
    #[error("Testing failed: {0}")]
    TestingFailed(String),
    #[error("Agent not found: {0}")]
    AgentNotFound(String),
}

impl AgentVibeCoder {
    /// Create a new Agent Vibe Coder
    pub fn new() -> Self {
        Self {
            agent_architect: AgentArchitect,
            code_generator: AgentCodeGenerator,
            behavior_synthesizer: BehaviorSynthesizer,
            performance_optimizer: PerformanceOptimizer,
            capability_enhancer: CapabilityEnhancer,
            evolution_engine: EvolutionEngine,
            template_library: Arc::new(RwLock::new(HashMap::new())),
            pattern_registry: Arc::new(RwLock::new(HashMap::new())),
            lifecycle_manager: AgentLifecycleManager,
            version_control: AgentVersionControl,
            agent_tester: AgentTester,
            quality_assessor: QualityAssessor,
            safety_validator: SafetyValidator,
            collaboration_engine: CollaborationEngine,
            learning_system: AgentLearningSystem,
            created_agents: Arc::new(RwLock::new(HashMap::new())),
            metrics: Arc::new(RwLock::new(VibeCoderMetrics::default())),
        }
    }
    
    /// Create a new agent based on specification
    pub async fn create_agent(&self, request: AgentCreationRequest) -> Result<String, VibeCoderError> {
        let agent_id = Uuid::new_v4().to_string();
        
        // Validate request
        self.validate_creation_request(&request).await?;
        
        // Generate agent architecture
        let architecture = self.generate_architecture(&request.agent_specification).await?;
        
        // Generate code implementation
        let code_modules = self.generate_code(&request.agent_specification, &architecture).await?;
        
        // Create agent implementation
        let implementation = AgentImplementation {
            implementation_id: Uuid::new_v4().to_string(),
            architecture,
            code_modules,
            configuration: AgentConfiguration {
                config_id: Uuid::new_v4().to_string(),
                parameters: HashMap::new(),
                feature_flags: HashMap::new(),
            },
            runtime_environment: RuntimeEnvironment {
                environment_id: Uuid::new_v4().to_string(),
                platform: "SymbioteIDE".to_string(),
                runtime_version: "1.0.0".to_string(),
            },
            integration_points: Vec::new(),
        };
        
        // Create agent record
        let created_agent = CreatedAgent {
            agent_id: agent_id.clone(),
            name: request.agent_specification.name.clone(),
            creator_agent_id: request.creator_agent_id,
            specification: request.agent_specification,
            implementation,
            version: AgentVersion {
                version_id: Uuid::new_v4().to_string(),
                version_number: "1.0.0".to_string(),
                release_notes: "Initial agent creation".to_string(),
            },
            status: AgentStatus::Creating,
            performance_metrics: PerformanceMetrics {
                response_time_avg: 0.0,
                accuracy_score: 0.0,
                throughput: 0.0,
                error_rate: 0.0,
            },
            learning_progress: LearningProgress {
                learning_milestones: Vec::new(),
                skill_improvements: Vec::new(),
            },
            collaboration_history: Vec::new(),
            created_at: Utc::now(),
            last_updated: Utc::now(),
        };
        
        // Store created agent
        {
            let mut agents = self.created_agents.write().await;
            agents.insert(agent_id.clone(), created_agent);
        }
        
        // Update metrics
        {
            let mut metrics = self.metrics.write().await;
            metrics.total_agents_created += 1;
            metrics.active_agents += 1;
            metrics.successful_creations += 1;
        }
        
        Ok(agent_id)
    }
    
    /// Validate creation request
    async fn validate_creation_request(&self, request: &AgentCreationRequest) -> Result<(), VibeCoderError> {
        if request.agent_specification.name.is_empty() {
            return Err(VibeCoderError::InvalidSpecification("Agent name is required".to_string()));
        }
        
        if request.agent_specification.capabilities.is_empty() {
            return Err(VibeCoderError::InvalidSpecification("At least one capability is required".to_string()));
        }
        
        Ok(())
    }
    
    /// Generate agent architecture
    async fn generate_architecture(&self, spec: &AgentSpecification) -> Result<AgentArchitecture, VibeCoderError> {
        // Mock architecture generation based on capabilities
        let components = spec.capabilities.iter().map(|cap| {
            ArchitectureComponent {
                component_id: Uuid::new_v4().to_string(),
                name: cap.name.clone(),
                component_type: match cap.capability_type {
                    CapabilityType::Reasoning => ComponentType::Reasoning,
                    CapabilityType::CodeGeneration => ComponentType::Execution,
                    CapabilityType::Learning => ComponentType::Learning,
                    _ => ComponentType::Control,
                },
                responsibilities: vec![cap.description.clone()],
            }
        }).collect();
        
        Ok(AgentArchitecture {
            architecture_type: ArchitectureType::Hybrid,
            components,
            data_flow: Vec::new(),
            decision_points: Vec::new(),
            feedback_loops: Vec::new(),
        })
    }
    
    /// Generate code modules
    async fn generate_code(&self, spec: &AgentSpecification, architecture: &AgentArchitecture) -> Result<Vec<CodeModule>, VibeCoderError> {
        // Mock code generation
        let modules = architecture.components.iter().map(|component| {
            CodeModule {
                module_id: Uuid::new_v4().to_string(),
                name: format!("{}_module", component.name.to_lowercase()),
                language: "rust".to_string(),
                code: format!("// Generated code for {}\npub struct {} {{}}", component.name, component.name),
                dependencies: Vec::new(),
            }
        }).collect();
        
        Ok(modules)
    }
    
    /// Get created agent
    pub async fn get_agent(&self, agent_id: String) -> Option<CreatedAgent> {
        let agents = self.created_agents.read().await;
        agents.get(&agent_id).cloned()
    }
    
    /// List all created agents
    pub async fn list_agents(&self) -> Vec<CreatedAgent> {
        let agents = self.created_agents.read().await;
        agents.values().cloned().collect()
    }
    
    /// Get metrics
    pub async fn get_metrics(&self) -> VibeCoderMetrics {
        let metrics = self.metrics.read().await;
        metrics.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_agent_creation() {
        let vibe_coder = AgentVibeCoder::new();
        
        let request = AgentCreationRequest {
            request_id: Uuid::new_v4().to_string(),
            creator_agent_id: "test_creator".to_string(),
            agent_specification: AgentSpecification {
                name: "TestAgent".to_string(),
                description: "A test agent".to_string(),
                purpose: "Testing".to_string(),
                domain: AgentDomain::Testing,
                capabilities: vec![AgentCapability {
                    capability_id: Uuid::new_v4().to_string(),
                    name: "Test Execution".to_string(),
                    description: "Execute tests".to_string(),
                    capability_type: CapabilityType::Testing,
                    proficiency_level: ProficiencyLevel::Advanced,
                    dependencies: Vec::new(),
                    implementation_hints: Vec::new(),
                }],
                behavioral_traits: Vec::new(),
                interaction_patterns: Vec::new(),
                knowledge_requirements: Vec::new(),
                performance_requirements: PerformanceRequirements {
                    response_time_ms: 1000,
                    accuracy_threshold: 0.95,
                    throughput_requests_per_second: 10,
                    memory_limit_mb: 512,
                    cpu_limit_percentage: 50.0,
                    concurrent_tasks: 5,
                },
                ethical_guidelines: Vec::new(),
            },
            creation_context: CreationContext {
                project_context: "Test project".to_string(),
                team_context: "Test team".to_string(),
                technical_stack: vec!["Rust".to_string()],
                existing_agents: Vec::new(),
                integration_requirements: Vec::new(),
                resource_constraints: ResourceConstraints {
                    max_memory_mb: 1024,
                    max_cpu_percentage: 80.0,
                    max_network_bandwidth: 1000,
                    max_storage_mb: 2048,
                    budget_constraints: None,
                },
            },
            optimization_goals: Vec::new(),
            constraints: CreationConstraints {
                time_limit: std::time::Duration::from_secs(3600),
                complexity_limit: ComplexityLevel::Moderate,
                dependency_restrictions: Vec::new(),
                platform_restrictions: Vec::new(),
                security_requirements: Vec::new(),
                compliance_requirements: Vec::new(),
            },
            timeline: CreationTimeline {
                start_time: Utc::now(),
                estimated_completion: Utc::now() + chrono::Duration::hours(1),
                milestones: Vec::new(),
                dependencies: Vec::new(),
            },
        };
        
        let agent_id = vibe_coder.create_agent(request).await.unwrap();
        assert!(!agent_id.is_empty());
        
        let agent = vibe_coder.get_agent(agent_id).await.unwrap();
        assert_eq!(agent.name, "TestAgent");
    }
}
