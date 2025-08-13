# Planning Agent (Context-Engineered, Spec-Driven Development)

## Overview

This crate provides comprehensive planning capabilities for Symbiote, enabling context-engineered, spec-driven development with executable plans, traceability, and integration with the broader Symbiote ecosystem.

## Database Schema

### Planning System Persistence

```sql
-- Planning projects and requirements
CREATE TABLE planning_projects (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id VARCHAR(255) NOT NULL UNIQUE,
    project_name VARCHAR(255) NOT NULL,
    description TEXT,
    requirements_text TEXT,
    domain VARCHAR(100), -- 'web', 'mobile', 'api', 'data', 'ml', etc.
    complexity_level VARCHAR(50), -- 'simple', 'medium', 'complex', 'enterprise'
    estimated_duration_days INTEGER,
    status VARCHAR(50) DEFAULT 'draft', -- 'draft', 'planning', 'approved', 'in_progress', 'completed'
    created_by VARCHAR(255),
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW(),
    metadata JSONB
);

-- Generated PRDs (Product Requirements Documents)
CREATE TABLE planning_prds (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    prd_id VARCHAR(255) NOT NULL UNIQUE,
    project_id VARCHAR(255) REFERENCES planning_projects(project_id),
    title VARCHAR(500) NOT NULL,
    overview TEXT,
    goals JSONB, -- Array of goal objects
    user_stories JSONB, -- Array of user story objects
    functional_requirements JSONB, -- Array of functional requirements
    non_functional_requirements JSONB, -- Array of non-functional requirements
    constraints JSONB, -- Array of constraint objects
    assumptions JSONB, -- Array of assumption objects
    risks JSONB, -- Array of risk objects
    success_metrics JSONB, -- Array of success metric objects
    version VARCHAR(50) DEFAULT '1.0',
    status VARCHAR(50) DEFAULT 'draft',
    generated_at TIMESTAMP DEFAULT NOW(),
    approved_at TIMESTAMP,
    approved_by VARCHAR(255),
    metadata JSONB
);

-- Architecture specifications
CREATE TABLE planning_architecture_specs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    spec_id VARCHAR(255) NOT NULL UNIQUE,
    project_id VARCHAR(255) REFERENCES planning_projects(project_id),
    prd_id VARCHAR(255) REFERENCES planning_prds(prd_id),
    c4_context_diagram TEXT, -- C4 context diagram as text/mermaid
    c4_container_diagram TEXT, -- C4 container diagram as text/mermaid
    c4_component_diagram TEXT, -- C4 component diagram as text/mermaid
    system_overview TEXT,
    technology_stack JSONB, -- Technology choices and rationale
    data_models JSONB, -- Data model definitions
    api_design JSONB, -- API design specifications
    security_considerations JSONB, -- Security requirements and measures
    performance_requirements JSONB, -- Performance specifications
    scalability_considerations JSONB, -- Scalability requirements
    deployment_architecture JSONB, -- Deployment and infrastructure
    integration_points JSONB, -- External system integrations
    version VARCHAR(50) DEFAULT '1.0',
    status VARCHAR(50) DEFAULT 'draft',
    generated_at TIMESTAMP DEFAULT NOW(),
    reviewed_at TIMESTAMP,
    reviewed_by VARCHAR(255),
    metadata JSONB
);

-- API contracts and specifications
CREATE TABLE planning_api_contracts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    contract_id VARCHAR(255) NOT NULL UNIQUE,
    project_id VARCHAR(255) REFERENCES planning_projects(project_id),
    spec_id VARCHAR(255) REFERENCES planning_architecture_specs(spec_id),
    api_name VARCHAR(255) NOT NULL,
    api_version VARCHAR(50) DEFAULT 'v1',
    base_url VARCHAR(500),
    authentication_type VARCHAR(100), -- 'none', 'api_key', 'oauth2', 'jwt', 'basic'
    openapi_spec JSONB, -- OpenAPI 3.0 specification
    endpoints JSONB, -- Array of endpoint definitions
    data_models JSONB, -- API data model definitions
    error_codes JSONB, -- Error code definitions
    rate_limiting JSONB, -- Rate limiting specifications
    versioning_strategy VARCHAR(100),
    documentation_url VARCHAR(500),
    status VARCHAR(50) DEFAULT 'draft',
    generated_at TIMESTAMP DEFAULT NOW(),
    validated_at TIMESTAMP,
    metadata JSONB
);

-- Test plans and specifications
CREATE TABLE planning_test_plans (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    plan_id VARCHAR(255) NOT NULL UNIQUE,
    project_id VARCHAR(255) REFERENCES planning_projects(project_id),
    spec_id VARCHAR(255) REFERENCES planning_architecture_specs(spec_id),
    test_strategy TEXT,
    test_levels JSONB, -- unit, integration, system, acceptance
    test_types JSONB, -- functional, performance, security, usability
    test_cases JSONB, -- Array of test case definitions
    test_data_requirements JSONB, -- Test data specifications
    test_environment_requirements JSONB, -- Environment specifications
    automation_strategy JSONB, -- Test automation approach
    performance_criteria JSONB, -- Performance test criteria
    security_test_cases JSONB, -- Security test specifications
    acceptance_criteria JSONB, -- Acceptance test criteria
    test_tools JSONB, -- Recommended testing tools
    estimated_effort_hours INTEGER,
    status VARCHAR(50) DEFAULT 'draft',
    generated_at TIMESTAMP DEFAULT NOW(),
    reviewed_at TIMESTAMP,
    reviewed_by VARCHAR(255),
    metadata JSONB
);

-- Executable tasks and decomposition
CREATE TABLE planning_tasks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    task_id VARCHAR(255) NOT NULL UNIQUE,
    project_id VARCHAR(255) REFERENCES planning_projects(project_id),
    parent_task_id VARCHAR(255) REFERENCES planning_tasks(task_id),
    task_name VARCHAR(500) NOT NULL,
    description TEXT,
    task_type VARCHAR(100), -- 'development', 'testing', 'deployment', 'documentation', 'review'
    priority VARCHAR(50), -- 'low', 'medium', 'high', 'critical'
    estimated_hours INTEGER,
    complexity_score INTEGER, -- 1-10 complexity rating
    dependencies JSONB, -- Array of dependent task IDs
    acceptance_criteria JSONB, -- Array of acceptance criteria
    technical_requirements JSONB, -- Technical specifications
    assigned_to VARCHAR(255),
    status VARCHAR(50) DEFAULT 'todo', -- 'todo', 'in_progress', 'review', 'done', 'blocked'
    progress_percentage INTEGER DEFAULT 0,
    started_at TIMESTAMP,
    completed_at TIMESTAMP,
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW(),
    metadata JSONB
);

-- Context packs for planning
CREATE TABLE planning_context_packs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    pack_id VARCHAR(255) NOT NULL UNIQUE,
    project_id VARCHAR(255) REFERENCES planning_projects(project_id),
    pack_name VARCHAR(255) NOT NULL,
    purpose TEXT,
    context_sources JSONB, -- Array of context source definitions
    retrieval_plan JSONB, -- Retrieval strategy and filters
    code_context JSONB, -- Code-related context data
    spec_context JSONB, -- Specification context data
    test_context JSONB, -- Test-related context data
    ci_context JSONB, -- CI/CD context data
    domain_context JSONB, -- Domain-specific context
    size_tokens INTEGER,
    quality_score DECIMAL(3,2), -- 0.00 to 1.00
    created_at TIMESTAMP DEFAULT NOW(),
    last_used TIMESTAMP,
    usage_count INTEGER DEFAULT 0,
    metadata JSONB
);

-- Planning validation results
CREATE TABLE planning_validations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    validation_id VARCHAR(255) NOT NULL UNIQUE,
    project_id VARCHAR(255) REFERENCES planning_projects(project_id),
    validation_type VARCHAR(100), -- 'prd', 'architecture', 'api', 'test_plan', 'tasks'
    target_id VARCHAR(255), -- ID of the validated entity
    validation_status VARCHAR(50), -- 'passed', 'failed', 'warning'
    validation_score DECIMAL(3,2), -- 0.00 to 1.00
    issues_found JSONB, -- Array of validation issues
    recommendations JSONB, -- Array of improvement recommendations
    codebase_conflicts JSONB, -- Conflicts with existing codebase
    overlap_analysis JSONB, -- Overlap and duplication analysis
    validated_at TIMESTAMP DEFAULT NOW(),
    validated_by VARCHAR(255),
    metadata JSONB
);

-- Indexes for performance
CREATE INDEX idx_planning_projects_status ON planning_projects(status);
CREATE INDEX idx_planning_projects_domain ON planning_projects(domain);
CREATE INDEX idx_planning_projects_created_by ON planning_projects(created_by);
CREATE INDEX idx_planning_prds_project ON planning_prds(project_id);
CREATE INDEX idx_planning_prds_status ON planning_prds(status);
CREATE INDEX idx_planning_architecture_specs_project ON planning_architecture_specs(project_id);
CREATE INDEX idx_planning_architecture_specs_prd ON planning_architecture_specs(prd_id);
CREATE INDEX idx_planning_api_contracts_project ON planning_api_contracts(project_id);
CREATE INDEX idx_planning_api_contracts_spec ON planning_api_contracts(spec_id);
CREATE INDEX idx_planning_test_plans_project ON planning_test_plans(project_id);
CREATE INDEX idx_planning_test_plans_spec ON planning_test_plans(spec_id);
CREATE INDEX idx_planning_tasks_project ON planning_tasks(project_id);
CREATE INDEX idx_planning_tasks_parent ON planning_tasks(parent_task_id);
CREATE INDEX idx_planning_tasks_status ON planning_tasks(status);
CREATE INDEX idx_planning_tasks_assigned_to ON planning_tasks(assigned_to);
CREATE INDEX idx_planning_context_packs_project ON planning_context_packs(project_id);
CREATE INDEX idx_planning_validations_project ON planning_validations(project_id);
CREATE INDEX idx_planning_validations_type ON planning_validations(validation_type);
```

## Core Components

### Planning Agent System

```rust
/// Context-engineered, spec-driven planning agent
pub struct PlanningAgent {
    // Context engineering
    context_engine: ContextEngine,
    retrieval_planner: RetrievalPlanner,
    context_pack_manager: ContextPackManager,
    
    // Spec generation
    prd_generator: PRDGenerator,
    architecture_spec_generator: ArchitectureSpecGenerator,
    api_contract_generator: APIContractGenerator,
    test_plan_generator: TestPlanGenerator,
    adr_generator: ADRGenerator,
    
    // Planning workflow
    workflow_pipeline: WorkflowPipeline,
    task_decomposer: TaskDecomposer,
    dependency_analyzer: DependencyAnalyzer,
    
    // Stack presets
    stack_preset_catalog: StackPresetCatalog,
    preset_manager: PresetManager,
    
    // Integration
    editor_integration: EditorIntegration,
    task_manager_integration: TaskManagerIntegration,

    // Overlap/Conflicts Detection
    overlap_detector: OverlapDetector,
}

impl PlanningAgent {
    pub async fn new() -> PlanningResult<Self>;
    
    /// Create comprehensive plan from requirements
    pub async fn create_plan(&self, requirements: &PlanningRequirements) -> PlanningResult<ExecutablePlan>;
    
    /// Generate PRD from requirements
    pub async fn generate_prd(&self, requirements: &PlanningRequirements) -> PlanningResult<PRD>;
    
    /// Generate architecture specification
    pub async fn generate_architecture_spec(&self, prd: &PRD, context_packs: &[ContextPack]) -> PlanningResult<ArchitectureSpec>;
    
    /// Generate API contracts
    pub async fn generate_api_contracts(&self, architecture: &ArchitectureSpec) -> PlanningResult<Vec<APIContract>>;
    
    /// Generate test plan
    pub async fn generate_test_plan(&self, architecture: &ArchitectureSpec, api_contracts: &[APIContract]) -> PlanningResult<TestPlan>;
    
    /// Decompose into executable tasks
    pub async fn decompose_into_tasks(&self, plan: &ExecutablePlan) -> PlanningResult<Vec<PlanTask>>;
    
    /// Validate plan against existing codebase
    pub async fn validate_plan(&self, plan: &ExecutablePlan) -> PlanningResult<ValidationResult>;
    
    /// Check for overlaps and conflicts
    pub async fn check_overlaps(&self, plan: &ExecutablePlan) -> PlanningResult<OverlapAnalysis>;
}

/// Context engineering system
pub struct ContextEngine {
    code_graph_client: CodeGraphClient,
    embeddings_client: EmbeddingsClient,
    sql_metadata_client: SQLMetadataClient,
    repo_history_client: RepoHistoryClient,
}

impl ContextEngine {
    pub async fn create_context_pack(&self, purpose: &str, retrieval_plan: &RetrievalPlan) -> PlanningResult<ContextPack>;
    
    pub async fn retrieve_code_context(&self, query: &str, filters: &[String]) -> PlanningResult<Vec<ContextSource>>;
    
    pub async fn retrieve_spec_context(&self, spec_type: &str, filters: &[String]) -> PlanningResult<Vec<ContextSource>>;
    
    pub async fn retrieve_test_context(&self, test_patterns: &[String]) -> PlanningResult<Vec<ContextSource>>;
    
    pub async fn retrieve_ci_context(&self, build_status: &str) -> PlanningResult<Vec<ContextSource>>;
}

/// Stack preset catalog system
pub struct StackPresetCatalog {
    preset_storage: PresetStorage,
    preset_validator: PresetValidator,
    preset_generator: PresetGenerator,
}

impl StackPresetCatalog {
    pub async fn get_preset(&self, preset_id: &str) -> PlanningResult<StackPreset>;
    
    pub async fn list_presets_by_domain(&self, domain: &str) -> PlanningResult<Vec<StackPreset>>;
    
    pub async fn create_custom_preset(&self, preset: &StackPreset) -> PlanningResult<PresetId>;
    
    pub async fn generate_project_structure(&self, preset: &StackPreset, project_name: &str) -> PlanningResult<ProjectStructure>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PRD {
    pub id: String,
    pub title: String,
    pub problem: String,
    pub goals: Vec<String>,
    pub non_goals: Vec<String>,
    pub users: Vec<String>,
    pub success_metrics: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchitectureSpec {
    pub id: String,
    pub c4_context: String,
    pub c4_container: String,
    pub interfaces: Vec<APIContract>,
    pub data_models: Vec<String>,
    pub risks: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct APIContract {
    pub name: String,
    pub method: String,
    pub path: String,
    pub req_schema: String,
    pub res_schema: String,
    pub status_codes: Vec<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestPlan {
    pub id: String,
    pub cases: Vec<TestCase>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCase {
    pub id: String,
    pub name: String,
    pub given: String,
    pub when_: String,
    pub then_: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ADR {
    pub id: String,
    pub title: String,
    pub context: String,
    pub decision: String,
    pub consequences: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextPack {
    pub id: String,
    pub purpose: String,
    pub sources: Vec<ContextSource>,
    pub size_tokens: u32,
    pub freshness_ts: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContextSource {
    Code(SymbolRef),
    Spec(String),
    ADR(String),
    Tests(String),
    CI(String),
    Telemetry(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalPlan {
    pub query_graph: String,
    pub filters: Vec<String>,
    pub scoring: String,
    pub budget_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StackPreset {
    pub id: String,
    pub name: String,
    pub domains: Vec<String>,
    pub frontend: String,
    pub backend: String,
    pub mobile: Option<String>,
    pub db: String,
    pub auth: String,
    pub deploy: String,
    pub ci: String,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanTask {
    pub id: String,
    pub title: String,
    pub spec_ref: String,
    pub deps: Vec<String>,
    pub acceptance: Vec<String>,
    pub owner: String,
}

#[derive(Debug, Clone)]
pub struct ExecutablePlan {
    pub prd: PRD,
    pub architecture: ArchitectureSpec,
    pub api_contracts: Vec<APIContract>,
    pub test_plan: TestPlan,
    pub adrs: Vec<ADR>,
    pub tasks: Vec<PlanTask>,
    pub context_packs: Vec<ContextPack>,
}

#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub conflicts: Vec<String>,
    pub missing_dependencies: Vec<String>,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct OverlapAnalysis {
    pub overlapping_components: Vec<String>,
    pub duplicate_functionality: Vec<String>,
    pub merge_recommendations: Vec<String>,
    pub impact_analysis: Vec<String>,
}
```

## Workflow Pipeline

### Phase 1: Intake & Scope
- Parse requirements and constraints
- Define success metrics
- Identify stakeholders

### Phase 2: Context Retrieval Plan
- Build retrieval queries
- Create ContextPacks
- Gather relevant context

### Phase 3: Spec Draft
- Generate PRD
- Create C4/arc42 architecture
- Define interface contracts
- Identify risks
- Create test plan

### Phase 4: Validation
- Cross-check against code graph
- Analyze CI failures
- Check existing modules
- Generate overlap diffs

### Phase 5: Task Plan
- Decompose into milestones
- Create tasks and subtasks
- Define dependencies
- Set acceptance criteria

### Phase 6: Execution Oversight
- Gate changes with tests/linters
- Apply policy engine
- Track costs and metrics

### Phase 7: Review & Iterate
- Update ADRs
- Finalize acceptance
- Document lessons learned

## Integration Points

### Editor Integration
- Publishes spec/task references
- Provides context packs for Next Edits
- Traces decisions back to specs

### Task Manager Integration
- Creates tasks with spec references
- Tracks dependencies
- Monitors acceptance criteria
- Publishes plan events

### Code Graph Integration
- Queries existing code structure
- Identifies reusable components
- Detects potential conflicts

### Embeddings Integration
- Semantic search for similar components
- Context-aware recommendations
- Knowledge base queries

## Overlap/Conflicts Detector (Anti-Duplication)

```rust
/// Anti-duplication system for detecting overlaps and conflicts
pub struct OverlapDetector {
    // Detection engines
    code_graph_analyzer: CodeGraphAnalyzer,
    embeddings_analyzer: EmbeddingsAnalyzer,
    filename_pattern_matcher: FilenamePatternMatcher,
    api_signature_analyzer: APISignatureAnalyzer,
    behavior_test_analyzer: BehaviorTestAnalyzer,

    // Analysis and proposals
    similarity_calculator: SimilarityCalculator,
    merge_proposal_generator: MergeProposalGenerator,
    impact_analyzer: ImpactAnalyzer,

    // Performance and metrics
    performance_monitor: PerformanceMonitor,
    false_positive_tracker: FalsePositiveTracker,
    threshold_manager: ThresholdManager,
}

impl OverlapDetector {
    pub async fn new() -> PlanningResult<Self>;

    /// Detect duplicates in planned components
    pub async fn detect_duplicates(&self, plan: &ExecutablePlan) -> PlanningResult<Vec<DuplicateFinding>>;

    /// Analyze code graph similarity
    pub async fn analyze_code_graph_similarity(&self, component_a: &str, component_b: &str) -> PlanningResult<f32>;

    /// Analyze embeddings cosine similarity
    pub async fn analyze_embeddings_similarity(&self, component_a: &str, component_b: &str) -> PlanningResult<f32>;

    /// Match filename patterns
    pub async fn match_filename_patterns(&self, files: &[String]) -> PlanningResult<Vec<PatternMatch>>;

    /// Detect API signature collisions
    pub async fn detect_api_collisions(&self, api_contracts: &[APIContract]) -> PlanningResult<Vec<APICollision>>;

    /// Analyze behavior and test overlap
    pub async fn analyze_behavior_overlap(&self, test_plan_a: &TestPlan, test_plan_b: &TestPlan) -> PlanningResult<f32>;

    /// Generate merge proposals
    pub async fn generate_merge_proposal(&self, duplicates: &[DuplicateFinding]) -> PlanningResult<MergeProposal>;

    /// Perform impact analysis for refactoring
    pub async fn analyze_refactor_impact(&self, merge_proposal: &MergeProposal) -> PlanningResult<ImpactAnalysis>;

    /// Block new component creation
    pub async fn block_component(&self, component: &str, reason: &str) -> PlanningResult<BlockAction>;

    /// Track performance metrics
    pub async fn track_performance(&self, operation: &str, duration_ms: u64) -> PlanningResult<()>;

    /// Track false positive rate
    pub async fn track_false_positive(&self, finding: &DuplicateFinding, is_false_positive: bool) -> PlanningResult<()>;
}

/// Code graph similarity analyzer
pub struct CodeGraphAnalyzer {
    graph_client: CodeGraphClient,
    similarity_engine: SimilarityEngine,
}

impl CodeGraphAnalyzer {
    pub async fn calculate_structural_similarity(&self, node_a: &str, node_b: &str) -> PlanningResult<f32>;

    pub async fn find_similar_components(&self, component: &str, threshold: f32) -> PlanningResult<Vec<SimilarComponent>>;

    pub async fn analyze_dependency_overlap(&self, component_a: &str, component_b: &str) -> PlanningResult<DependencyOverlap>;
}

/// Embeddings similarity analyzer
pub struct EmbeddingsAnalyzer {
    embeddings_client: EmbeddingsClient,
    cosine_calculator: CosineCalculator,
}

impl EmbeddingsAnalyzer {
    pub async fn calculate_semantic_similarity(&self, text_a: &str, text_b: &str) -> PlanningResult<f32>;

    pub async fn find_semantically_similar(&self, query: &str, threshold: f32) -> PlanningResult<Vec<SemanticMatch>>;

    pub async fn cluster_similar_components(&self, components: &[String]) -> PlanningResult<Vec<ComponentCluster>>;
}

/// Merge proposal generator
pub struct MergeProposalGenerator {
    proposal_engine: ProposalEngine,
    risk_assessor: RiskAssessor,
    step_generator: StepGenerator,
}

impl MergeProposalGenerator {
    pub async fn generate_proposal(&self, duplicates: &[DuplicateFinding]) -> PlanningResult<MergeProposal>;

    pub async fn assess_merge_risk(&self, proposal: &MergeProposal) -> PlanningResult<RiskAssessment>;

    pub async fn generate_merge_steps(&self, proposal: &MergeProposal) -> PlanningResult<Vec<MergeStep>>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicateFinding {
    pub entity_type: String,
    pub entity_id: String,
    pub similarity: f32,
    pub locations: Vec<String>,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeProposal {
    pub target: String,
    pub sources: Vec<String>,
    pub rationale: String,
    pub steps: Vec<String>,
    pub risk: String,
}

#[derive(Debug, Clone)]
pub struct ImpactAnalysis {
    pub affected_components: Vec<String>,
    pub breaking_changes: Vec<String>,
    pub migration_effort: String,
    pub risk_level: RiskLevel,
}

#[derive(Debug, Clone)]
pub struct PatternMatch {
    pub pattern: String,
    pub matches: Vec<String>,
    pub confidence: f32,
}

#[derive(Debug, Clone)]
pub struct APICollision {
    pub method: String,
    pub path: String,
    pub conflicting_contracts: Vec<String>,
    pub severity: CollisionSeverity,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CollisionSeverity {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}
```

## Folder Structure

```
crates/planning/
├── src/
│   ├── lib.rs                      // Main library exports
│   ├── agent/
│   │   ├── mod.rs                  // Planning agent module
│   │   ├── planning_agent.rs       // Core planning agent implementation
│   │   ├── requirements_parser.rs  // Requirements analysis and parsing
│   │   ├── prd_generator.rs        // PRD generation logic
│   │   ├── architecture_generator.rs // Architecture specification generation
│   │   ├── api_contract_generator.rs // API contract generation
│   │   ├── test_plan_generator.rs  // Test plan generation
│   │   └── task_decomposer.rs      // Task decomposition logic
│   ├── context/
│   │   ├── mod.rs                  // Context engineering module
│   │   ├── context_engine.rs       // Context retrieval and management
│   │   ├── context_pack.rs         // Context pack creation and management
│   │   ├── code_context.rs         // Code-related context retrieval
│   │   ├── spec_context.rs         // Specification context retrieval
│   │   ├── test_context.rs         // Test context retrieval
│   │   └── ci_context.rs           // CI/CD context retrieval
│   ├── presets/
│   │   ├── mod.rs                  // Stack preset module
│   │   ├── preset_catalog.rs       // Preset catalog management
│   │   ├── preset_generator.rs     // Custom preset generation
│   │   ├── preset_validator.rs     // Preset validation logic
│   │   └── project_structure.rs    // Project structure generation
│   ├── overlap/
│   │   ├── mod.rs                  // Overlap detection module
│   │   ├── overlap_detector.rs     // Main overlap detection logic
│   │   ├── code_graph_analyzer.rs  // Code graph similarity analysis
│   │   ├── embeddings_analyzer.rs  // Semantic similarity analysis
│   │   ├── pattern_matcher.rs      // Filename and pattern matching
│   │   ├── api_collision_detector.rs // API signature collision detection
│   │   ├── behavior_analyzer.rs    // Behavior and test overlap analysis
│   │   └── merge_proposal_generator.rs // Merge proposal generation
│   ├── validation/
│   │   ├── mod.rs                  // Validation module
│   │   ├── plan_validator.rs       // Plan validation logic
│   │   ├── codebase_validator.rs   // Codebase integration validation
│   │   ├── architecture_validator.rs // Architecture validation
│   │   ├── api_validator.rs        // API contract validation
│   │   └── test_validator.rs       // Test plan validation
│   ├── models/
│   │   ├── mod.rs                  // Data models module
│   │   ├── planning_models.rs      // Core planning data structures
│   │   ├── prd_models.rs           // PRD data structures
│   │   ├── architecture_models.rs  // Architecture specification models
│   │   ├── api_models.rs           // API contract models
│   │   ├── test_models.rs          // Test plan models
│   │   ├── task_models.rs          // Task and decomposition models
│   │   ├── context_models.rs       // Context pack models
│   │   ├── overlap_models.rs       // Overlap detection models
│   │   └── validation_models.rs    // Validation result models
│   ├── storage/
│   │   ├── mod.rs                  // Storage module
│   │   ├── planning_repository.rs  // Planning data persistence
│   │   ├── prd_repository.rs       // PRD storage operations
│   │   ├── architecture_repository.rs // Architecture spec storage
│   │   ├── api_repository.rs       // API contract storage
│   │   ├── test_repository.rs      // Test plan storage
│   │   ├── task_repository.rs      // Task storage operations
│   │   ├── context_repository.rs   // Context pack storage
│   │   └── validation_repository.rs // Validation result storage
│   ├── error.rs                    // Error types and handling
│   ├── config.rs                   // Configuration management
│   └── utils.rs                    // Utility functions
├── tests/
│   ├── integration/
│   │   ├── planning_agent_tests.rs
│   │   ├── context_engine_tests.rs
│   │   ├── overlap_detection_tests.rs
│   │   └── validation_tests.rs
│   └── unit/
│       ├── prd_generation_tests.rs
│       ├── architecture_generation_tests.rs
│       ├── api_generation_tests.rs
│       └── task_decomposition_tests.rs
├── examples/
│   ├── basic_planning.rs
│   ├── context_retrieval.rs
│   ├── overlap_detection.rs
│   └── validation_example.rs
├── Cargo.toml
└── README.md
```

## Error Handling

### Planning Error Types

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PlanningError {
    #[error("Requirements parsing failed: {input} - {reason}")]
    RequirementsParsingFailed { input: String, reason: String },

    #[error("PRD generation failed: {project_id} - {error}")]
    PrdGenerationFailed { project_id: String, error: String },

    #[error("Architecture generation failed: {project_id} - {reason}")]
    ArchitectureGenerationFailed { project_id: String, reason: String },

    #[error("API contract generation failed: {api_name} - {error}")]
    ApiContractGenerationFailed { api_name: String, error: String },

    #[error("Test plan generation failed: {project_id} - {reason}")]
    TestPlanGenerationFailed { project_id: String, reason: String },

    #[error("Task decomposition failed: {plan_id} - {error}")]
    TaskDecompositionFailed { plan_id: String, error: String },

    #[error("Context retrieval failed: {query} - {reason}")]
    ContextRetrievalFailed { query: String, reason: String },

    #[error("Context pack creation failed: {purpose} - {error}")]
    ContextPackCreationFailed { purpose: String, error: String },

    #[error("Overlap detection failed: {component_a} vs {component_b} - {reason}")]
    OverlapDetectionFailed { component_a: String, component_b: String, reason: String },

    #[error("Validation failed: {validation_type} - {target_id} - {error}")]
    ValidationFailed { validation_type: String, target_id: String, error: String },

    #[error("Codebase integration failed: {project_id} - {reason}")]
    CodebaseIntegrationFailed { project_id: String, reason: String },

    #[error("Stack preset not found: {preset_id}")]
    StackPresetNotFound { preset_id: String },

    #[error("Project structure generation failed: {preset_id} - {error}")]
    ProjectStructureGenerationFailed { preset_id: String, error: String },

    #[error("Merge proposal generation failed: {duplicates_count} duplicates - {reason}")]
    MergeProposalGenerationFailed { duplicates_count: usize, reason: String },

    #[error("Impact analysis failed: {merge_proposal_id} - {error}")]
    ImpactAnalysisFailed { merge_proposal_id: String, error: String },

    #[error("Configuration error: {setting} - {issue}")]
    ConfigurationError { setting: String, issue: String },

    #[error("Permission denied: {operation} - {resource}")]
    PermissionDenied { operation: String, resource: String },

    #[error("Database error: {operation} - {error}")]
    DatabaseError { operation: String, error: String },

    #[error("Serialization error: {data_type} - {error}")]
    SerializationError { data_type: String, error: String },

    #[error("External service error: {service} - {error}")]
    ExternalServiceError { service: String, error: String },

    #[error("Internal error: {details}")]
    InternalError { details: String },
}

pub type PlanningResult<T> = Result<T, PlanningError>;

impl From<std::io::Error> for PlanningError {
    fn from(err: std::io::Error) -> Self {
        PlanningError::InternalError {
            details: format!("IO error: {}", err),
        }
    }
}

impl From<serde_json::Error> for PlanningError {
    fn from(err: serde_json::Error) -> Self {
        PlanningError::SerializationError {
            data_type: "json".to_string(),
            error: err.to_string(),
        }
    }
}
```

## UI Specifications

### Planning Management Dashboard

```
┌─────────────────────────────────────────────────────────────────────────────┐
│ 📋 AI Planning Center                              [🔄] [⚙️] [📊] [🔒] [🧠] │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│ 📊 Planning Overview                                                        │
│ ┌─────────────────────────────────────────────────────────────────────────┐ │
│ │ Active Projects: 12       │ Generated PRDs: 47    │ Validated Plans: 34 │ │
│ │ In Planning: 5            │ Architecture Specs: 23│ Overlap Detected: 8 │ │
│ │ Success Rate: 94.2%       │ API Contracts: 156    │ Context Packs: 67   │ │
│ └─────────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
│ 🚀 Quick Actions                                                            │
│ ┌─────────────────────────────────────────────────────────────────────────┐ │
│ │ 🧠 AI Planning        │ 📋 PRD Generator     │ 🏗️ Architecture      │ │
│ │ Start intelligent     │ Generate product     │ Create system         │ │
│ │ project planning      │ requirements doc     │ architecture specs    │ │
│ │ [🧠 Start Planning]   │ [📋 Generate PRD]    │ [🏗️ Design Arch]     │ │
│ │                                                                         │ │
│ │ 🔍 Overlap Detection  │ 📊 Context Engine    │ ✅ Plan Validation    │ │
│ │ Find duplicates and   │ Gather relevant      │ Validate against      │ │
│ │ conflicts in plans    │ context for planning │ existing codebase     │ │
│ │ [🔍 Detect Overlaps]  │ [📊 Build Context]   │ [✅ Validate]         │ │
│ └─────────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
│ 📂 Active Planning Projects                                                 │
│ ┌─────────────────────────────────────────────────────────────────────────┐ │
│ │ Project              │ Domain   │ Status    │ Progress │ Actions         │ │
│ │ E-commerce Platform  │ Web      │ 🟢 Active │ 78%      │ [📋][🏗️][✅]   │ │
│ │ Mobile Banking App   │ Mobile   │ 🟡 Review │ 45%      │ [📋][🔍][⚙️]   │ │
│ │ ML Analytics Engine  │ Data/ML  │ 🟢 Active │ 92%      │ [📊][✅][🚀]   │ │
│ │ IoT Device Manager   │ IoT      │ 🔴 Issues │ 23%      │ [🔧][📋][🔍]   │ │
│ │ [➕ New Project] [📥 Import] [🔄 Refresh] [⚙️ Settings]                 │ │
│ └─────────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
│ 🚨 Recent Activity                                                          │
│ │ • AI generated comprehensive PRD for "E-commerce Platform" - 94% quality │ │
│ │ • Overlap detected: "UserService" conflicts with existing component      │ │
│ │ • Architecture validation passed for "ML Analytics Engine"               │ │
│ │ • Context pack created with 47 relevant code examples                    │ │
│ │ [📋 View All Activity] [🔔 Notifications] [📊 Planning Report]           │ │
└─────────────────────────────────────────────────────────────────────────────┘
```

### AI Planning Wizard Interface

```
┌─────────────────────────────────────────────────────────────────────────────┐
│ 🧠 AI Planning Wizard - Step 2/5: Requirements Analysis    [💾] [⏮️] [⏭️] [❌] │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│ 📝 Project Requirements                                                     │
│ ┌─────────────────────────────────────────────────────────────────────────┐ │
│ │ Project Name: E-commerce Platform                                       │ │
│ │ Domain: Web Application                                                 │ │
│ │ Complexity: Enterprise                                                  │ │
│ │                                                                         │ │
│ │ Requirements Text:                                                      │ │
│ │ ┌─────────────────────────────────────────────────────────────────────┐ │ │
│ │ │ Build a modern e-commerce platform with user authentication,       │ │ │
│ │ │ product catalog, shopping cart, payment processing, order          │ │ │
│ │ │ management, and admin dashboard. Must support 10,000+ concurrent   │ │ │
│ │ │ users, integrate with Stripe/PayPal, and include mobile API.       │ │ │
│ │ │ Requires real-time inventory updates, recommendation engine,       │ │ │
│ │ │ and comprehensive analytics dashboard.                              │ │ │
│ │ └─────────────────────────────────────────────────────────────────────┘ │ │
│ └─────────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
│ 🧠 AI Analysis Results                                                      │
│ ┌─────────────────────────────────────────────────────────────────────────┐ │
│ │ ✅ Identified Components:                                               │ │
│ │ • User Authentication Service (OAuth2, JWT)                            │ │
│ │ • Product Catalog Service (Search, Categories, Inventory)              │ │
│ │ • Shopping Cart Service (Session Management, Persistence)              │ │
│ │ • Payment Processing Service (Stripe, PayPal Integration)              │ │
│ │ • Order Management Service (Workflow, Status Tracking)                 │ │
│ │ • Admin Dashboard (Analytics, User Management, Content)                │ │
│ │ • Recommendation Engine (ML-based, Real-time)                          │ │
│ │ • Mobile API Gateway (REST, GraphQL, Rate Limiting)                    │ │
│ │                                                                         │ │
│ │ ⚠️ Potential Overlaps Detected:                                         │ │
│ │ • "UserService" exists in current codebase - 87% similarity            │ │
│ │ • "PaymentProcessor" found in archived project - 72% similarity        │ │
│ │                                                                         │ │
│ │ 💡 Recommendations:                                                     │ │
│ │ • Reuse existing UserService with authentication enhancements          │ │
│ │ • Consider microservices architecture for scalability                  │ │
│ │ • Implement event-driven architecture for real-time features           │ │
│ │ [📋 Detailed Analysis] [🔍 View Overlaps] [🏗️ Architecture Preview]    │ │
│ └─────────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
│ 🎯 Next Steps                                                               │
│ │ [⏮️ Previous: Project Setup] [⏭️ Next: Architecture Design] [💾 Save Draft] │ │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Security Model

### Planning Security Framework

```rust
pub struct PlanningSecurityManager {
    access_control: PlanningAccessControl,
    data_protection: DataProtectionManager,
    audit_logger: PlanningAuditLogger,
    compliance_manager: ComplianceManager,
}

impl PlanningSecurityManager {
    /// Validate planning operation permissions
    pub async fn validate_planning_access(&self, user_id: &str, operation: PlanningOperation) -> PlanningResult<AccessDecision>;

    /// Secure sensitive data in planning documents
    pub async fn secure_planning_data(&self, data: &PlanningData) -> PlanningResult<SecurePlanningData>;

    /// Scan planning content for security issues
    pub async fn scan_planning_content(&self, content: &str) -> PlanningResult<SecurityScanResult>;

    /// Enforce security policies on planning operations
    pub async fn enforce_security_policies(&self, operation: &PlanningOperation, policies: &[SecurityPolicy]) -> PlanningResult<PolicyEnforcement>;

    /// Log planning operations for audit and compliance
    pub async fn log_planning_operation(&self, operation: &PlanningOperation, user_id: &str, result: &OperationResult) -> PlanningResult<()>;

    /// Handle sensitive data in requirements and specifications
    pub async fn handle_sensitive_requirements(&self, requirements: &str) -> PlanningResult<SanitizedRequirements>;

    /// Validate codebase access for context retrieval
    pub async fn validate_codebase_access(&self, user_id: &str, codebase_path: &str) -> PlanningResult<CodebaseAccess>;
}

#[derive(Debug, Clone)]
pub enum PlanningOperation {
    CreateProject { project_name: String, requirements: String },
    GeneratePRD { project_id: String, requirements: String },
    GenerateArchitecture { project_id: String, prd_data: String },
    AccessCodebase { codebase_path: String, query: String },
    ValidatePlan { project_id: String, plan_data: String },
    ExportPlan { project_id: String, export_format: String },
    ImportContext { context_source: String, filters: Vec<String> },
}
```

## Integration Points

### Upstream Dependencies

```rust
/// Integration with other Symbiote crates
pub struct SymbiotePlanningIntegration {
    ai_client: AiClient,                    // For AI-powered planning and generation
    context_engine: ContextEngine,          // For intelligent context retrieval
    storage_manager: StorageManager,        // For planning data persistence
    security_manager: SecurityManager,      // For planning security and access control
    agents_framework: AgentsFramework,      // For specialized planning agents
    codebase_analyzer: CodebaseAnalyzer,    // For codebase analysis and integration
}

impl SymbiotePlanningIntegration {
    /// Initialize planning with Symbiote ecosystem
    pub async fn initialize_planning(&self, config: PlanningConfig) -> PlanningResult<PlanningAgent>;

    /// Use AI for intelligent planning and generation
    pub async fn ai_generate_plan(&self, requirements: &PlanningRequirements) -> PlanningResult<ExecutablePlan>;

    /// Retrieve context for planning using context engine
    pub async fn get_planning_context(&self, query: &str, filters: &[String]) -> PlanningResult<ContextPack>;

    /// Store planning data using storage crate
    pub async fn persist_planning_data(&self, data: &PlanningData) -> PlanningResult<()>;

    /// Validate planning permissions using security crate
    pub async fn validate_planning_permissions(&self, user_id: &str, operation: &PlanningOperation) -> PlanningResult<bool>;

    /// Analyze codebase for planning integration
    pub async fn analyze_codebase_integration(&self, plan: &ExecutablePlan) -> PlanningResult<IntegrationAnalysis>;
}
```

### Downstream Consumers

```rust
/// Services that consume planning capabilities
pub trait PlanningConsumer {
    /// Handle planning events and notifications
    async fn on_planning_event(&self, event: PlanningEvent) -> PlanningResult<()>;

    /// Process plan generation events
    async fn on_plan_generated(&self, generation_event: PlanGenerationEvent) -> PlanningResult<()>;

    /// Handle validation events
    async fn on_plan_validated(&self, validation_event: ValidationEvent) -> PlanningResult<()>;

    /// Process planning errors and failures
    async fn on_planning_error(&self, error: PlanningErrorEvent) -> PlanningResult<()>;
}

/// Planning event types
#[derive(Debug, Clone)]
pub enum PlanningEvent {
    ProjectCreated { project_id: String, project_name: String, domain: String },
    PRDGenerated { project_id: String, prd_id: String, quality_score: f32 },
    ArchitectureGenerated { project_id: String, spec_id: String, components_count: u32 },
    OverlapDetected { project_id: String, overlap_type: String, similarity_score: f32 },
    PlanValidated { project_id: String, validation_status: String, issues_count: u32 },
    TasksDecomposed { project_id: String, tasks_count: u32, estimated_hours: u32 },
    ContextPackCreated { pack_id: String, purpose: String, sources_count: u32 },
}
```

## Implementation Details

### Technology Stack

- **AI Framework**: Built on Symbiote's AI agent framework for intelligent planning
- **Context Engine**: Advanced context retrieval and engineering for relevant information
- **Planning Algorithms**: Sophisticated algorithms for requirements analysis and decomposition
- **Overlap Detection**: Multi-layered similarity analysis using code graphs and embeddings
- **Validation Engine**: Comprehensive validation against existing codebases and standards
- **Database**: PostgreSQL for planning data persistence with complex relationship modeling
- **Security**: Enterprise-grade security for sensitive planning data and codebase access
- **Integration**: Deep integration with Symbiote ecosystem for seamless workflow

### Key Features

1. **AI-Powered Planning**: Intelligent project planning with comprehensive requirement analysis
2. **Automated PRD Generation**: High-quality Product Requirements Documents from natural language
3. **Architecture Specification**: Detailed system architecture with C4 diagrams and specifications
4. **API Contract Generation**: Complete API specifications with OpenAPI documentation
5. **Test Plan Creation**: Comprehensive test strategies and detailed test case generation
6. **Overlap Detection**: Advanced duplicate detection and conflict resolution
7. **Context Engineering**: Intelligent context retrieval for informed planning decisions
8. **Validation Framework**: Multi-layered validation against existing codebases and standards

This comprehensive planning system transforms Symbiote into the most intelligent project planning platform available, capable of generating complete, validated, and executable project plans that seamlessly integrate with existing codebases while preventing duplication and ensuring optimal architecture.
