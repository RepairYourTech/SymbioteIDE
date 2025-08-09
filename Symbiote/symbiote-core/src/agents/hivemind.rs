//! # HiveMind - The Master Orchestrator
//! 
//! HiveMind is the central intelligence that orchestrates all Symbiotes.
//! It can dynamically select existing teams or create new teams based on task requirements.
//! HiveMind makes intelligent decisions about resource allocation, task delegation,
//! and coordination between multiple Symbiotes working in parallel.

use super::*;
use crate::{Result, SymbioteError};
use std::collections::{HashMap, HashSet};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::time::{SystemTime, UNIX_EPOCH};

/// The master orchestrator agent that manages all Symbiotes
#[derive(Debug)]
pub struct HiveMind {
    /// Unique identifier for the HiveMind instance
    pub id: String,
    
    /// Current orchestration strategies
    strategies: HashMap<TaskType, OrchestrationStrategy>,
    
    /// Team selection algorithms
    team_selector: TeamSelector,
    
    /// Resource allocation manager
    resource_manager: ResourceManager,
    
    /// Decision engine for intelligent task routing
    decision_engine: DecisionEngine,
    
    /// Performance tracking for optimization
    performance_tracker: PerformanceTracker,
    
    /// Learning system for improving orchestration
    learning_system: LearningSystem,
    
    /// Active orchestrations
    active_orchestrations: HashMap<String, ActiveOrchestration>,
}

impl HiveMind {
    pub fn new() -> Self {
        Self {
            id: format!("hivemind-{}", Uuid::new_v4()),
            strategies: Self::default_strategies(),
            team_selector: TeamSelector::new(),
            resource_manager: ResourceManager::new(),
            decision_engine: DecisionEngine::new(),
            performance_tracker: PerformanceTracker::new(),
            learning_system: LearningSystem::new(),
            active_orchestrations: HashMap::new(),
        }
    }

    pub async fn initialize(&mut self) -> Result<()> {
        // Initialize decision engine with default rules
        self.decision_engine.load_default_rules().await?;
        
        // Initialize learning system
        self.learning_system.initialize().await?;
        
        // Load performance history
        self.performance_tracker.load_history().await?;

        tracing::info!("HiveMind {} initialized successfully", self.id);
        Ok(())
    }

    /// Main orchestration method - analyzes task and creates optimal execution plan
    pub async fn orchestrate_task(
        &mut self,
        request: TaskRequest,
        execution_id: &str,
        framework: &SymbioteAgentFramework,
    ) -> Result<TaskExecution> {
        tracing::info!("HiveMind orchestrating task: {} ({})", request.description, execution_id);

        // Analyze the task requirements
        let analysis = self.analyze_task(&request).await?;
        
        // Determine optimal team composition
        let team_plan = self.create_team_plan(&analysis, framework).await?;
        
        // Allocate resources
        let resource_allocation = self.resource_manager.allocate_resources(&team_plan, &request.constraints).await?;
        
        // Create execution plan
        let execution_plan = self.create_execution_plan(team_plan, resource_allocation, &analysis).await?;
        
        // Start orchestration
        let orchestration = ActiveOrchestration {
            id: execution_id.to_string(),
            request: request.clone(),
            analysis,
            execution_plan: execution_plan.clone(),
            status: OrchestrationStatus::Starting,
            start_time: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            assigned_symbiotes: Vec::new(),
            coordination_state: CoordinationState::new(),
        };

        // Store active orchestration
        self.active_orchestrations.insert(execution_id.to_string(), orchestration);

        // Execute the plan
        let task_execution = self.execute_plan(execution_plan, execution_id, framework).await?;

        Ok(task_execution)
    }

    /// Analyze task requirements and complexity
    async fn analyze_task(&self, request: &TaskRequest) -> Result<TaskAnalysis> {
        let mut analysis = TaskAnalysis {
            complexity: self.calculate_complexity(request).await?,
            required_skills: self.identify_required_skills(request).await?,
            estimated_duration: self.estimate_duration(request).await?,
            parallelization_opportunities: self.identify_parallelization(request).await?,
            resource_requirements: self.estimate_resources(request).await?,
            risk_factors: self.assess_risks(request).await?,
            dependencies: self.analyze_dependencies(request).await?,
        };

        // Use decision engine to refine analysis
        analysis = self.decision_engine.refine_analysis(analysis, request).await?;

        Ok(analysis)
    }

    /// Create optimal team plan based on analysis
    async fn create_team_plan(&self, analysis: &TaskAnalysis, framework: &SymbioteAgentFramework) -> Result<TeamPlan> {
        // Check if we have a suitable preset team
        if let Some(preset_team) = self.team_selector.find_preset_team(analysis, framework).await? {
            return Ok(TeamPlan::PresetTeam(preset_team));
        }

        // Create custom team
        let custom_team = self.team_selector.create_custom_team(analysis, framework).await?;
        Ok(TeamPlan::CustomTeam(custom_team))
    }

    /// Create detailed execution plan
    async fn create_execution_plan(
        &self,
        team_plan: TeamPlan,
        resource_allocation: ResourceAllocation,
        analysis: &TaskAnalysis,
    ) -> Result<ExecutionPlan> {
        let mut plan = ExecutionPlan {
            team_plan,
            resource_allocation,
            phases: Vec::new(),
            coordination_strategy: self.select_coordination_strategy(analysis).await?,
            monitoring_strategy: self.select_monitoring_strategy(analysis).await?,
            fallback_strategies: self.create_fallback_strategies(analysis).await?,
        };

        // Create execution phases
        plan.phases = self.create_execution_phases(analysis, &plan).await?;

        Ok(plan)
    }

    /// Execute the orchestration plan
    async fn execute_plan(
        &self,
        plan: ExecutionPlan,
        execution_id: &str,
        framework: &SymbioteAgentFramework,
    ) -> Result<TaskExecution> {
        let execution_engine = framework.execution_engine.read().await;
        
        // Start execution with the plan
        let task_execution = execution_engine.execute_with_plan(plan, execution_id, self).await?;

        Ok(task_execution)
    }

    // Analysis helper methods
    async fn calculate_complexity(&self, request: &TaskRequest) -> Result<TaskComplexity> {
        // Analyze task description, context, and requirements
        let mut complexity_score = 0.0;

        // Factor in task type
        complexity_score += match request.task_type {
            TaskType::Development => 0.7,
            TaskType::Analysis => 0.5,
            TaskType::Testing => 0.4,
            TaskType::Debugging => 0.8,
            TaskType::Refactoring => 0.6,
            TaskType::Documentation => 0.3,
            TaskType::Deployment => 0.9,
            TaskType::Monitoring => 0.4,
            TaskType::Custom(_) => 0.5,
        };

        // Factor in context complexity
        complexity_score += request.context.files.len() as f64 * 0.1;
        complexity_score += request.context.dependencies.len() as f64 * 0.05;

        // Determine complexity level
        let complexity = if complexity_score < 0.3 {
            TaskComplexity::Simple
        } else if complexity_score < 0.7 {
            TaskComplexity::Moderate
        } else if complexity_score < 1.2 {
            TaskComplexity::Complex
        } else {
            TaskComplexity::VeryComplex
        };

        Ok(complexity)
    }

    async fn identify_required_skills(&self, request: &TaskRequest) -> Result<Vec<SymbioteSkill>> {
        let mut skills = Vec::new();

        // Add skills based on task type
        match request.task_type {
            TaskType::Development => {
                skills.push(SymbioteSkill::Programming);
                skills.push(SymbioteSkill::CodeGeneration);
            }
            TaskType::Analysis => {
                skills.push(SymbioteSkill::CodeAnalysis);
                skills.push(SymbioteSkill::PatternRecognition);
            }
            TaskType::Testing => {
                skills.push(SymbioteSkill::TestGeneration);
                skills.push(SymbioteSkill::QualityAssurance);
            }
            TaskType::Debugging => {
                skills.push(SymbioteSkill::Debugging);
                skills.push(SymbioteSkill::ErrorAnalysis);
            }
            _ => {}
        }

        // Add skills based on stack
        if let Some(stack) = &request.context.stack {
            skills.push(SymbioteSkill::StackSpecific(stack.clone()));
        }

        // Add skills based on context
        if !request.context.files.is_empty() {
            skills.push(SymbioteSkill::FileManagement);
        }

        Ok(skills)
    }

    async fn estimate_duration(&self, request: &TaskRequest) -> Result<EstimatedDuration> {
        // Use historical data and complexity analysis
        let base_duration = match request.task_type {
            TaskType::Development => 3600, // 1 hour base
            TaskType::Analysis => 1800,    // 30 minutes base
            TaskType::Testing => 2400,     // 40 minutes base
            TaskType::Debugging => 4800,   // 80 minutes base
            TaskType::Refactoring => 3000, // 50 minutes base
            TaskType::Documentation => 1200, // 20 minutes base
            TaskType::Deployment => 1800,  // 30 minutes base
            TaskType::Monitoring => 600,   // 10 minutes base
            TaskType::Custom(_) => 2400,   // 40 minutes base
        };

        // Factor in complexity multipliers
        let complexity_multiplier = request.context.files.len() as f64 * 0.1 + 1.0;
        let estimated_seconds = (base_duration as f64 * complexity_multiplier) as u64;

        Ok(EstimatedDuration {
            min_seconds: estimated_seconds / 2,
            max_seconds: estimated_seconds * 2,
            expected_seconds: estimated_seconds,
        })
    }

    async fn identify_parallelization(&self, request: &TaskRequest) -> Result<Vec<ParallelizationOpportunity>> {
        let mut opportunities = Vec::new();

        // Check if files can be processed in parallel
        if request.context.files.len() > 1 {
            opportunities.push(ParallelizationOpportunity {
                opportunity_type: ParallelizationType::FileProcessing,
                description: "Process multiple files simultaneously".to_string(),
                estimated_speedup: 2.0,
                resource_requirements: ResourceRequirements::moderate(),
            });
        }

        // Check for independent subtasks
        match request.task_type {
            TaskType::Testing => {
                opportunities.push(ParallelizationOpportunity {
                    opportunity_type: ParallelizationType::TestExecution,
                    description: "Run test suites in parallel".to_string(),
                    estimated_speedup: 3.0,
                    resource_requirements: ResourceRequirements::high(),
                });
            }
            TaskType::Analysis => {
                opportunities.push(ParallelizationOpportunity {
                    opportunity_type: ParallelizationType::AnalysisPhases,
                    description: "Analyze different aspects simultaneously".to_string(),
                    estimated_speedup: 1.5,
                    resource_requirements: ResourceRequirements::moderate(),
                });
            }
            _ => {}
        }

        Ok(opportunities)
    }

    async fn estimate_resources(&self, _request: &TaskRequest) -> Result<ResourceRequirements> {
        // Estimate resource requirements based on task analysis
        Ok(ResourceRequirements::moderate())
    }

    async fn assess_risks(&self, request: &TaskRequest) -> Result<Vec<RiskFactor>> {
        let mut risks = Vec::new();

        // Check for high-risk operations
        if request.constraints.allowed_operations.contains(&AllowedOperation::SystemModification) {
            risks.push(RiskFactor {
                risk_type: RiskType::SystemModification,
                severity: RiskSeverity::High,
                description: "Task involves system modifications".to_string(),
                mitigation_strategies: vec!["Create system backup".to_string(), "Use sandboxed environment".to_string()],
            });
        }

        if request.constraints.allowed_operations.contains(&AllowedOperation::NetworkAccess) {
            risks.push(RiskFactor {
                risk_type: RiskType::NetworkAccess,
                severity: RiskSeverity::Medium,
                description: "Task requires network access".to_string(),
                mitigation_strategies: vec!["Monitor network traffic".to_string(), "Use secure connections".to_string()],
            });
        }

        Ok(risks)
    }

    async fn analyze_dependencies(&self, request: &TaskRequest) -> Result<Vec<TaskDependency>> {
        let mut dependencies = Vec::new();

        // Analyze file dependencies
        for file in &request.context.files {
            dependencies.push(TaskDependency {
                dependency_type: DependencyType::File,
                resource: file.clone(),
                required: true,
                description: format!("Required file: {}", file),
            });
        }

        // Analyze package dependencies
        for dep in &request.context.dependencies {
            dependencies.push(TaskDependency {
                dependency_type: DependencyType::Package,
                resource: dep.clone(),
                required: true,
                description: format!("Required package: {}", dep),
            });
        }

        Ok(dependencies)
    }

    async fn select_coordination_strategy(&self, analysis: &TaskAnalysis) -> Result<CoordinationStrategy> {
        match analysis.complexity {
            TaskComplexity::Simple => Ok(CoordinationStrategy::Sequential),
            TaskComplexity::Moderate => Ok(CoordinationStrategy::Pipeline),
            TaskComplexity::Complex => Ok(CoordinationStrategy::Hierarchical),
            TaskComplexity::VeryComplex => Ok(CoordinationStrategy::Adaptive),
            TaskComplexity::Expert => Ok(CoordinationStrategy::Adaptive),
        }
    }

    async fn select_monitoring_strategy(&self, analysis: &TaskAnalysis) -> Result<MonitoringStrategy> {
        Ok(MonitoringStrategy {
            frequency: match analysis.complexity {
                TaskComplexity::Simple => MonitoringFrequency::Low,
                TaskComplexity::Moderate => MonitoringFrequency::Medium,
                TaskComplexity::Complex => MonitoringFrequency::High,
                TaskComplexity::VeryComplex => MonitoringFrequency::RealTime,
                TaskComplexity::Expert => MonitoringFrequency::RealTime,
            },
            metrics: vec![
                MonitoringMetric::Progress,
                MonitoringMetric::ResourceUsage,
                MonitoringMetric::ErrorRate,
            ],
        })
    }

    async fn create_fallback_strategies(&self, _analysis: &TaskAnalysis) -> Result<Vec<FallbackStrategy>> {
        Ok(vec![
            FallbackStrategy {
                trigger: FallbackTrigger::HighErrorRate,
                action: FallbackAction::ReduceParallelism,
                description: "Reduce parallelism if error rate is high".to_string(),
            },
            FallbackStrategy {
                trigger: FallbackTrigger::ResourceExhaustion,
                action: FallbackAction::ScaleDown,
                description: "Scale down if resources are exhausted".to_string(),
            },
        ])
    }

    async fn create_execution_phases(&self, analysis: &TaskAnalysis, _plan: &ExecutionPlan) -> Result<Vec<ExecutionPhase>> {
        let mut phases = Vec::new();

        // Create phases based on task complexity and type
        phases.push(ExecutionPhase {
            id: "preparation".to_string(),
            name: "Preparation".to_string(),
            description: "Prepare environment and resources".to_string(),
            dependencies: Vec::new(),
            estimated_duration: analysis.estimated_duration.expected_seconds / 10,
            phase_type: PhaseType::Preparation,
        });

        phases.push(ExecutionPhase {
            id: "execution".to_string(),
            name: "Main Execution".to_string(),
            description: "Execute the main task".to_string(),
            dependencies: vec!["preparation".to_string()],
            estimated_duration: analysis.estimated_duration.expected_seconds * 8 / 10,
            phase_type: PhaseType::Execution,
        });

        phases.push(ExecutionPhase {
            id: "finalization".to_string(),
            name: "Finalization".to_string(),
            description: "Finalize results and cleanup".to_string(),
            dependencies: vec!["execution".to_string()],
            estimated_duration: analysis.estimated_duration.expected_seconds / 10,
            phase_type: PhaseType::Finalization,
        });

        Ok(phases)
    }

    fn default_strategies() -> HashMap<TaskType, OrchestrationStrategy> {
        let mut strategies = HashMap::new();
        
        strategies.insert(TaskType::Development, OrchestrationStrategy::Collaborative);
        strategies.insert(TaskType::Analysis, OrchestrationStrategy::Parallel);
        strategies.insert(TaskType::Testing, OrchestrationStrategy::Parallel);
        strategies.insert(TaskType::Debugging, OrchestrationStrategy::Sequential);
        strategies.insert(TaskType::Refactoring, OrchestrationStrategy::Collaborative);
        strategies.insert(TaskType::Documentation, OrchestrationStrategy::Sequential);
        strategies.insert(TaskType::Deployment, OrchestrationStrategy::Pipeline);
        strategies.insert(TaskType::Monitoring, OrchestrationStrategy::Continuous);
        
        strategies
    }
}

/// Orchestration strategies for different task types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OrchestrationStrategy {
    Sequential,
    Parallel,
    Pipeline,
    Collaborative,
    Hierarchical,
    Adaptive,
    Continuous,
}

/// Task analysis result
#[derive(Debug, Clone)]
pub struct TaskAnalysis {
    pub complexity: TaskComplexity,
    pub required_skills: Vec<SymbioteSkill>,
    pub estimated_duration: EstimatedDuration,
    pub parallelization_opportunities: Vec<ParallelizationOpportunity>,
    pub resource_requirements: ResourceRequirements,
    pub risk_factors: Vec<RiskFactor>,
    pub dependencies: Vec<TaskDependency>,
}

// Use TaskComplexity from parent module
pub use super::TaskComplexity;

/// Symbiote skills
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SymbioteSkill {
    Programming,
    CodeGeneration,
    CodeAnalysis,
    PatternRecognition,
    TestGeneration,
    QualityAssurance,
    Debugging,
    ErrorAnalysis,
    FileManagement,
    StackSpecific(DevelopmentStack),
    Custom(String),
}

/// Estimated duration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EstimatedDuration {
    pub min_seconds: u64,
    pub max_seconds: u64,
    pub expected_seconds: u64,
}

/// Parallelization opportunity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelizationOpportunity {
    pub opportunity_type: ParallelizationType,
    pub description: String,
    pub estimated_speedup: f64,
    pub resource_requirements: ResourceRequirements,
}

/// Types of parallelization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ParallelizationType {
    FileProcessing,
    TestExecution,
    AnalysisPhases,
    IndependentTasks,
    DataProcessing,
}

/// Resource requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceRequirements {
    pub cpu_cores: u32,
    pub memory_mb: u64,
    pub disk_space_mb: u64,
    pub network_bandwidth: u64,
}

impl ResourceRequirements {
    pub fn low() -> Self {
        Self {
            cpu_cores: 1,
            memory_mb: 512,
            disk_space_mb: 100,
            network_bandwidth: 1024,
        }
    }

    pub fn moderate() -> Self {
        Self {
            cpu_cores: 2,
            memory_mb: 2048,
            disk_space_mb: 1024,
            network_bandwidth: 10240,
        }
    }

    pub fn high() -> Self {
        Self {
            cpu_cores: 4,
            memory_mb: 8192,
            disk_space_mb: 10240,
            network_bandwidth: 102400,
        }
    }
}

/// Risk factors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskFactor {
    pub risk_type: RiskType,
    pub severity: RiskSeverity,
    pub description: String,
    pub mitigation_strategies: Vec<String>,
}

/// Types of risks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RiskType {
    SystemModification,
    NetworkAccess,
    DataLoss,
    SecurityBreach,
    ResourceExhaustion,
    PerformanceDegradation,
}

/// Risk severity levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RiskSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Task dependencies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskDependency {
    pub dependency_type: DependencyType,
    pub resource: String,
    pub required: bool,
    pub description: String,
}

/// Types of dependencies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DependencyType {
    File,
    Package,
    Service,
    Environment,
    Tool,
}

/// Active orchestration tracking
#[derive(Debug, Clone)]
pub struct ActiveOrchestration {
    pub id: String,
    pub request: TaskRequest,
    pub analysis: TaskAnalysis,
    pub execution_plan: ExecutionPlan,
    pub status: OrchestrationStatus,
    pub start_time: u64,
    pub assigned_symbiotes: Vec<SymbioteId>,
    pub coordination_state: CoordinationState,
}

/// Orchestration status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OrchestrationStatus {
    Starting,
    Planning,
    Executing,
    Monitoring,
    Coordinating,
    Finalizing,
    Completed,
    Failed,
}

/// Coordination state
#[derive(Debug, Clone)]
pub struct CoordinationState {
    pub active_phases: HashSet<String>,
    pub completed_phases: HashSet<String>,
    pub symbiote_assignments: HashMap<SymbioteId, String>,
    pub communication_channels: HashMap<String, String>,
}

impl CoordinationState {
    pub fn new() -> Self {
        Self {
            active_phases: HashSet::new(),
            completed_phases: HashSet::new(),
            symbiote_assignments: HashMap::new(),
            communication_channels: HashMap::new(),
        }
    }
}

// Additional types needed for HiveMind functionality

/// Team selection algorithms
#[derive(Debug)]
pub struct TeamSelector {
    selection_algorithms: HashMap<String, SelectionAlgorithm>,
}

impl TeamSelector {
    pub fn new() -> Self {
        Self {
            selection_algorithms: HashMap::new(),
        }
    }

    pub async fn find_preset_team(&self, _analysis: &TaskAnalysis, _framework: &SymbioteAgentFramework) -> Result<Option<PresetTeam>> {
        // Implementation for finding preset teams
        Ok(None)
    }

    pub async fn create_custom_team(&self, _analysis: &TaskAnalysis, _framework: &SymbioteAgentFramework) -> Result<CustomTeam> {
        // Implementation for creating custom teams
        Ok(CustomTeam {
            id: Uuid::new_v4().to_string(),
            name: "Custom Team".to_string(),
            members: Vec::new(),
            coordination_strategy: CoordinationStrategy::Sequential,
        })
    }
}

/// Resource allocation manager
#[derive(Debug)]
pub struct ResourceManager {
    allocation_strategies: HashMap<String, AllocationStrategy>,
}

impl ResourceManager {
    pub fn new() -> Self {
        Self {
            allocation_strategies: HashMap::new(),
        }
    }

    pub async fn allocate_resources(&self, _team_plan: &TeamPlan, _constraints: &TaskConstraints) -> Result<ResourceAllocation> {
        Ok(ResourceAllocation {
            cpu_allocation: HashMap::new(),
            memory_allocation: HashMap::new(),
            storage_allocation: HashMap::new(),
            network_allocation: HashMap::new(),
        })
    }
}

/// Decision engine for intelligent task routing
#[derive(Debug)]
pub struct DecisionEngine {
    rules: Vec<DecisionRule>,
}

impl DecisionEngine {
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
        }
    }

    pub async fn load_default_rules(&mut self) -> Result<()> {
        // Load default decision rules
        Ok(())
    }

    pub async fn refine_analysis(&self, analysis: TaskAnalysis, _request: &TaskRequest) -> Result<TaskAnalysis> {
        // Refine analysis using decision rules
        Ok(analysis)
    }
}

/// Performance tracking for optimization
#[derive(Debug)]
pub struct PerformanceTracker {
    metrics: HashMap<String, PerformanceMetric>,
}

impl PerformanceTracker {
    pub fn new() -> Self {
        Self {
            metrics: HashMap::new(),
        }
    }

    pub async fn load_history(&mut self) -> Result<()> {
        // Load performance history
        Ok(())
    }
}

/// Learning system for improving orchestration
#[derive(Debug)]
pub struct LearningSystem {
    patterns: Vec<LearningPattern>,
}

impl LearningSystem {
    pub fn new() -> Self {
        Self {
            patterns: Vec::new(),
        }
    }

    pub async fn initialize(&mut self) -> Result<()> {
        // Initialize learning system
        Ok(())
    }
}

// Supporting types

#[derive(Debug, Clone)]
pub enum SelectionAlgorithm {
    SkillBased,
    PerformanceBased,
    LoadBalanced,
    Custom(String),
}

#[derive(Debug, Clone)]
pub struct PresetTeam {
    pub id: String,
    pub name: String,
    pub members: Vec<SymbioteId>,
    pub specialization: DevelopmentStack,
}

#[derive(Debug, Clone)]
pub struct CustomTeam {
    pub id: String,
    pub name: String,
    pub members: Vec<SymbioteId>,
    pub coordination_strategy: CoordinationStrategy,
}

#[derive(Debug, Clone)]
pub enum TeamPlan {
    PresetTeam(PresetTeam),
    CustomTeam(CustomTeam),
}

#[derive(Debug, Clone)]
pub struct ResourceAllocation {
    pub cpu_allocation: HashMap<SymbioteId, u32>,
    pub memory_allocation: HashMap<SymbioteId, u64>,
    pub storage_allocation: HashMap<SymbioteId, u64>,
    pub network_allocation: HashMap<SymbioteId, u64>,
}

#[derive(Debug, Clone)]
pub struct ExecutionPlan {
    pub team_plan: TeamPlan,
    pub resource_allocation: ResourceAllocation,
    pub phases: Vec<ExecutionPhase>,
    pub coordination_strategy: CoordinationStrategy,
    pub monitoring_strategy: MonitoringStrategy,
    pub fallback_strategies: Vec<FallbackStrategy>,
}

#[derive(Debug, Clone)]
pub struct ExecutionPhase {
    pub id: String,
    pub name: String,
    pub description: String,
    pub dependencies: Vec<String>,
    pub estimated_duration: u64,
    pub phase_type: PhaseType,
}

#[derive(Debug, Clone)]
pub enum PhaseType {
    Preparation,
    Execution,
    Finalization,
    Monitoring,
    Cleanup,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CoordinationStrategy {
    Sequential,
    Parallel,
    Pipeline,
    Collaborative,
    Hierarchical,
    Adaptive,
}

#[derive(Debug, Clone)]
pub struct MonitoringStrategy {
    pub frequency: MonitoringFrequency,
    pub metrics: Vec<MonitoringMetric>,
}

#[derive(Debug, Clone)]
pub enum MonitoringFrequency {
    Low,
    Medium,
    High,
    RealTime,
}

#[derive(Debug, Clone)]
pub enum MonitoringMetric {
    Progress,
    ResourceUsage,
    ErrorRate,
    Performance,
}

#[derive(Debug, Clone)]
pub struct FallbackStrategy {
    pub trigger: FallbackTrigger,
    pub action: FallbackAction,
    pub description: String,
}

#[derive(Debug, Clone)]
pub enum FallbackTrigger {
    HighErrorRate,
    ResourceExhaustion,
    TimeoutExceeded,
    QualityThresholdNotMet,
}

#[derive(Debug, Clone)]
pub enum FallbackAction {
    ReduceParallelism,
    ScaleDown,
    SwitchStrategy,
    RequestHumanIntervention,
}

#[derive(Debug, Clone)]
pub enum AllocationStrategy {
    EqualDistribution,
    PerformanceBased,
    TaskComplexityBased,
    Custom(String),
}

#[derive(Debug, Clone)]
pub struct DecisionRule {
    pub id: String,
    pub condition: String,
    pub action: String,
    pub priority: u32,
}

#[derive(Debug, Clone)]
pub struct PerformanceMetric {
    pub name: String,
    pub value: f64,
    pub timestamp: u64,
}

#[derive(Debug, Clone)]
pub struct LearningPattern {
    pub pattern_type: String,
    pub description: String,
    pub confidence: f64,
    pub usage_count: u32,
}

/// Task execution result from HiveMind
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskExecution {
    pub id: String,
    pub status: ExecutionStatus,
    pub assigned_symbiotes: Vec<SymbioteId>,
    pub results: Vec<ExecutionResult>,
    pub start_time: u64,
    pub end_time: Option<u64>,
    pub total_duration: Option<u64>,
}

/// Parallel task request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelTaskRequest {
    pub task: TaskRequest,
    pub isolation_requirements: IsolationRequirements,
    pub coordination_needs: CoordinationNeeds,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IsolationRequirements {
    pub separate_worktree: bool,
    pub separate_environment: bool,
    pub resource_isolation: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinationNeeds {
    pub requires_coordination: bool,
    pub coordination_frequency: CoordinationFrequency,
    pub shared_resources: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CoordinationFrequency {
    Never,
    OnCompletion,
    Periodic,
    RealTime,
}
