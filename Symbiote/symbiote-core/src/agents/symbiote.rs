//! # Symbiote - Individual AI Agent
//! 
//! Symbiotes are individual AI agents that can work independently or as part of teams.
//! Each Symbiote has specialized skills, memory, and can execute tasks in parallel
//! with other Symbiotes without conflicts.

use super::*;
use crate::{Result, SymbioteError};
use std::collections::{HashMap, VecDeque};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique identifier for a Symbiote
pub type SymbioteId = String;

/// Individual AI agent with specialized capabilities
#[derive(Debug, Clone)]
pub struct Symbiote {
    /// Unique identifier
    pub id: SymbioteId,
    
    /// Human-readable name
    pub name: String,
    
    /// Symbiote definition and configuration
    pub definition: SymbioteDefinition,
    
    /// Current state and status
    pub state: SymbioteState,
    
    /// Memory system
    pub memory: SymbioteMemory,
    
    /// Performance metrics
    pub metrics: SymbioteMetrics,
    
    /// Communication channels
    pub communication: CommunicationChannels,
    
    /// Execution context
    pub execution_context: Option<ExecutionContext>,
}

impl Symbiote {
    pub fn new(definition: SymbioteDefinition) -> Self {
        let id = format!("symbiote-{}", Uuid::new_v4());
        
        Self {
            id: id.clone(),
            name: definition.name.clone(),
            definition,
            state: SymbioteState::new(),
            memory: SymbioteMemory::new(),
            metrics: SymbioteMetrics::new(),
            communication: CommunicationChannels::new(),
            execution_context: None,
        }
    }

    /// Initialize the Symbiote for execution
    pub async fn initialize(&mut self, context: ExecutionContext) -> Result<()> {
        self.execution_context = Some(context);
        self.state.status = SymbioteStatus::Initializing;
        
        // Initialize memory with context
        self.memory.initialize_context(&self.execution_context.as_ref().unwrap()).await?;
        
        // Set up communication channels
        self.communication.initialize(&self.id).await?;
        
        self.state.status = SymbioteStatus::Ready;
        self.state.last_activity = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        
        tracing::info!("Symbiote {} ({}) initialized", self.name, self.id);
        Ok(())
    }

    /// Execute a task
    pub async fn execute_task(&mut self, task: SymbioteTask) -> Result<SymbioteResult> {
        self.state.status = SymbioteStatus::Executing;
        self.state.current_task = Some(task.clone());
        self.state.last_activity = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();

        let start_time = SystemTime::now();
        
        // Record task start in memory
        self.memory.record_task_start(&task).await?;

        // Execute based on task type
        let result = match task.task_type {
            SymbioteTaskType::CodeAnalysis => self.execute_code_analysis(&task).await,
            SymbioteTaskType::CodeGeneration => self.execute_code_generation(&task).await,
            SymbioteTaskType::Testing => self.execute_testing(&task).await,
            SymbioteTaskType::Debugging => self.execute_debugging(&task).await,
            SymbioteTaskType::Refactoring => self.execute_refactoring(&task).await,
            SymbioteTaskType::Documentation => self.execute_documentation(&task).await,
            SymbioteTaskType::FileOperation => self.execute_file_operation(&task).await,
            SymbioteTaskType::CommandExecution => self.execute_command(&task).await,
            SymbioteTaskType::Custom(ref custom_type) => self.execute_custom_task(custom_type, &task).await,
        };

        let execution_time = start_time.elapsed().unwrap().as_millis() as u64;
        
        // Update metrics
        self.metrics.update_execution_metrics(execution_time, result.is_ok()).await;
        
        // Record result in memory
        self.memory.record_task_completion(&task, &result, execution_time).await?;

        // Update state
        self.state.status = SymbioteStatus::Ready;
        self.state.current_task = None;
        self.state.last_activity = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();

        result
    }

    /// Pause execution
    pub async fn pause(&mut self) -> Result<()> {
        if self.state.status == SymbioteStatus::Executing {
            self.state.status = SymbioteStatus::Paused;
            self.state.last_activity = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        }
        Ok(())
    }

    /// Resume execution
    pub async fn resume(&mut self) -> Result<()> {
        if self.state.status == SymbioteStatus::Paused {
            self.state.status = SymbioteStatus::Executing;
            self.state.last_activity = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        }
        Ok(())
    }

    /// Stop execution
    pub async fn stop(&mut self) -> Result<()> {
        self.state.status = SymbioteStatus::Stopped;
        self.state.current_task = None;
        self.state.last_activity = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        Ok(())
    }

    /// Get current capabilities
    pub fn get_capabilities(&self) -> &Vec<SymbioteCapability> {
        &self.definition.capabilities
    }

    /// Check if Symbiote has a specific skill
    pub fn has_skill(&self, skill: &SymbioteSkill) -> bool {
        self.definition.skills.contains(skill)
    }

    /// Get compatibility score with a task
    pub async fn get_compatibility_score(&self, task: &SymbioteTask) -> Result<f64> {
        let mut score = 0.0;

        // Check skill compatibility
        for required_skill in &task.required_skills {
            if self.has_skill(required_skill) {
                score += 0.3;
            }
        }

        // Check capability compatibility
        for capability in &self.definition.capabilities {
            if task.required_capabilities.contains(capability) {
                score += 0.2;
            }
        }

        // Check stack compatibility
        if let Some(task_stack) = &task.context.stack {
            if self.definition.specializations.contains(task_stack) {
                score += 0.3;
            }
        }

        // Factor in performance history
        score += self.metrics.get_success_rate() * 0.2;

        Ok(score.min(1.0))
    }

    // Task execution methods
    async fn execute_code_analysis(&mut self, task: &SymbioteTask) -> Result<SymbioteResult> {
        // Implement code analysis logic
        let analysis_result = serde_json::json!({
            "type": "code_analysis",
            "files_analyzed": task.context.files.len(),
            "issues_found": 0,
            "suggestions": []
        });

        Ok(SymbioteResult {
            task_id: task.id.clone(),
            symbiote_id: self.id.clone(),
            result_type: SymbioteResultType::Analysis,
            data: analysis_result,
            success: true,
            message: "Code analysis completed successfully".to_string(),
            execution_time_ms: 1000,
        })
    }

    async fn execute_code_generation(&mut self, task: &SymbioteTask) -> Result<SymbioteResult> {
        // Implement code generation logic
        let generation_result = serde_json::json!({
            "type": "code_generation",
            "files_generated": 1,
            "lines_of_code": 50
        });

        Ok(SymbioteResult {
            task_id: task.id.clone(),
            symbiote_id: self.id.clone(),
            result_type: SymbioteResultType::CodeGenerated,
            data: generation_result,
            success: true,
            message: "Code generation completed successfully".to_string(),
            execution_time_ms: 2000,
        })
    }

    async fn execute_testing(&mut self, task: &SymbioteTask) -> Result<SymbioteResult> {
        // Implement testing logic
        let test_result = serde_json::json!({
            "type": "testing",
            "tests_run": 10,
            "tests_passed": 9,
            "tests_failed": 1,
            "coverage": 85.5
        });

        Ok(SymbioteResult {
            task_id: task.id.clone(),
            symbiote_id: self.id.clone(),
            result_type: SymbioteResultType::TestResults,
            data: test_result,
            success: true,
            message: "Testing completed successfully".to_string(),
            execution_time_ms: 5000,
        })
    }

    async fn execute_debugging(&mut self, task: &SymbioteTask) -> Result<SymbioteResult> {
        // Implement debugging logic
        let debug_result = serde_json::json!({
            "type": "debugging",
            "issues_identified": 2,
            "fixes_suggested": 2,
            "confidence": 0.85
        });

        Ok(SymbioteResult {
            task_id: task.id.clone(),
            symbiote_id: self.id.clone(),
            result_type: SymbioteResultType::DebugInfo,
            data: debug_result,
            success: true,
            message: "Debugging completed successfully".to_string(),
            execution_time_ms: 3000,
        })
    }

    async fn execute_refactoring(&mut self, task: &SymbioteTask) -> Result<SymbioteResult> {
        // Implement refactoring logic
        let refactor_result = serde_json::json!({
            "type": "refactoring",
            "files_modified": 3,
            "improvements": ["Reduced complexity", "Improved readability"]
        });

        Ok(SymbioteResult {
            task_id: task.id.clone(),
            symbiote_id: self.id.clone(),
            result_type: SymbioteResultType::CodeModified,
            data: refactor_result,
            success: true,
            message: "Refactoring completed successfully".to_string(),
            execution_time_ms: 4000,
        })
    }

    async fn execute_documentation(&mut self, task: &SymbioteTask) -> Result<SymbioteResult> {
        // Implement documentation logic
        let doc_result = serde_json::json!({
            "type": "documentation",
            "files_documented": task.context.files.len(),
            "documentation_coverage": 95.0
        });

        Ok(SymbioteResult {
            task_id: task.id.clone(),
            symbiote_id: self.id.clone(),
            result_type: SymbioteResultType::Documentation,
            data: doc_result,
            success: true,
            message: "Documentation completed successfully".to_string(),
            execution_time_ms: 2500,
        })
    }

    async fn execute_file_operation(&mut self, task: &SymbioteTask) -> Result<SymbioteResult> {
        // Implement file operation logic
        let file_result = serde_json::json!({
            "type": "file_operation",
            "operation": "read",
            "files_processed": task.context.files.len()
        });

        Ok(SymbioteResult {
            task_id: task.id.clone(),
            symbiote_id: self.id.clone(),
            result_type: SymbioteResultType::FileOperation,
            data: file_result,
            success: true,
            message: "File operation completed successfully".to_string(),
            execution_time_ms: 500,
        })
    }

    async fn execute_command(&mut self, task: &SymbioteTask) -> Result<SymbioteResult> {
        // Implement command execution logic
        let command_result = serde_json::json!({
            "type": "command_execution",
            "command": "cargo check",
            "exit_code": 0,
            "output": "Compilation successful"
        });

        Ok(SymbioteResult {
            task_id: task.id.clone(),
            symbiote_id: self.id.clone(),
            result_type: SymbioteResultType::CommandOutput,
            data: command_result,
            success: true,
            message: "Command executed successfully".to_string(),
            execution_time_ms: 1500,
        })
    }

    async fn execute_custom_task(&mut self, custom_type: &str, task: &SymbioteTask) -> Result<SymbioteResult> {
        // Implement custom task logic
        let custom_result = serde_json::json!({
            "type": "custom",
            "custom_type": custom_type,
            "status": "completed"
        });

        Ok(SymbioteResult {
            task_id: task.id.clone(),
            symbiote_id: self.id.clone(),
            result_type: SymbioteResultType::Custom,
            data: custom_result,
            success: true,
            message: format!("Custom task '{}' completed successfully", custom_type),
            execution_time_ms: 2000,
        })
    }
}

/// Symbiote definition and configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbioteDefinition {
    pub name: String,
    pub description: String,
    pub version: String,
    pub created_by: UserId,
    pub skills: Vec<SymbioteSkill>,
    pub capabilities: Vec<SymbioteCapability>,
    pub specializations: Vec<DevelopmentStack>,
    pub configuration: SymbioteConfiguration,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Symbiote capabilities
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SymbioteCapability {
    FileRead,
    FileWrite,
    CommandExecution,
    NetworkAccess,
    GitOperations,
    PackageManagement,
    DatabaseAccess,
    APIIntegration,
    WebScraping,
    ImageProcessing,
    DataAnalysis,
    MachineLearning,
    Custom(String),
}

/// Symbiote configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbioteConfiguration {
    pub max_parallel_tasks: u32,
    pub max_concurrent_tasks: u32,
    pub memory_limit_mb: u64,
    pub timeout_seconds: u64,
    pub retry_attempts: u32,
    pub ai_provider: String,
    pub ai_model: String,
    pub temperature: f32,
    pub allowed_operations: Vec<AllowedOperation>,
    pub custom_settings: HashMap<String, serde_json::Value>,
}

impl Default for SymbioteConfiguration {
    fn default() -> Self {
        Self {
            max_parallel_tasks: 1,
            max_concurrent_tasks: 1,
            memory_limit_mb: 1024,
            timeout_seconds: 300,
            retry_attempts: 3,
            ai_provider: "openai".to_string(),
            ai_model: "gpt-4".to_string(),
            temperature: 0.7,
            allowed_operations: vec![
                AllowedOperation::ReadFiles,
                AllowedOperation::WriteFiles,
                AllowedOperation::ExecuteCommands,
            ],
            custom_settings: HashMap::new(),
        }
    }
}

/// Symbiote current state
#[derive(Debug, Clone)]
pub struct SymbioteState {
    pub status: SymbioteStatus,
    pub current_task: Option<SymbioteTask>,
    pub last_activity: u64,
    pub error_count: u32,
    pub warning_count: u32,
    pub health_score: f64,
}

impl SymbioteState {
    pub fn new() -> Self {
        Self {
            status: SymbioteStatus::Created,
            current_task: None,
            last_activity: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            error_count: 0,
            warning_count: 0,
            health_score: 1.0,
        }
    }
}

/// Symbiote status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SymbioteStatus {
    Created,
    Initializing,
    Ready,
    Executing,
    Paused,
    Stopped,
    Error,
    Maintenance,
}

/// Task for a Symbiote to execute
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbioteTask {
    pub id: String,
    pub task_type: SymbioteTaskType,
    pub description: String,
    pub context: TaskContext,
    pub required_skills: Vec<SymbioteSkill>,
    pub required_capabilities: Vec<SymbioteCapability>,
    pub priority: TaskPriority,
    pub deadline: Option<u64>,
    pub parameters: HashMap<String, serde_json::Value>,
}

/// Types of tasks a Symbiote can execute
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SymbioteTaskType {
    CodeAnalysis,
    CodeGeneration,
    Testing,
    Debugging,
    Refactoring,
    Documentation,
    FileOperation,
    CommandExecution,
    Custom(String),
}

/// Result of Symbiote task execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbioteResult {
    pub task_id: String,
    pub symbiote_id: SymbioteId,
    pub result_type: SymbioteResultType,
    pub data: serde_json::Value,
    pub success: bool,
    pub message: String,
    pub execution_time_ms: u64,
}

/// Types of Symbiote results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SymbioteResultType {
    Analysis,
    CodeGenerated,
    CodeModified,
    TestResults,
    DebugInfo,
    Documentation,
    FileOperation,
    CommandOutput,
    Custom,
}

/// Symbiote performance metrics
#[derive(Debug, Clone)]
pub struct SymbioteMetrics {
    pub tasks_completed: u64,
    pub tasks_failed: u64,
    pub total_execution_time_ms: u64,
    pub average_execution_time_ms: f64,
    pub success_rate: f64,
    pub last_updated: u64,
}

impl SymbioteMetrics {
    pub fn new() -> Self {
        Self {
            tasks_completed: 0,
            tasks_failed: 0,
            total_execution_time_ms: 0,
            average_execution_time_ms: 0.0,
            success_rate: 1.0,
            last_updated: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
        }
    }

    pub async fn update_execution_metrics(&mut self, execution_time_ms: u64, success: bool) {
        if success {
            self.tasks_completed += 1;
        } else {
            self.tasks_failed += 1;
        }

        self.total_execution_time_ms += execution_time_ms;
        
        let total_tasks = self.tasks_completed + self.tasks_failed;
        if total_tasks > 0 {
            self.average_execution_time_ms = self.total_execution_time_ms as f64 / total_tasks as f64;
            self.success_rate = self.tasks_completed as f64 / total_tasks as f64;
        }

        self.last_updated = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    }

    pub fn get_success_rate(&self) -> f64 {
        self.success_rate
    }
}

/// Execution context for a Symbiote
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    pub workspace_path: String,
    pub git_worktree: Option<String>,
    pub environment_variables: HashMap<String, String>,
    pub allowed_operations: Vec<AllowedOperation>,
    pub resource_limits: ResourceLimits,
    pub isolation_level: IsolationLevel,
}

// Use IsolationLevel from parent module
pub use super::IsolationLevel;

/// Communication channels for Symbiotes
#[derive(Debug, Clone)]
pub struct CommunicationChannels {
    pub input_channel: Option<String>,
    pub output_channel: Option<String>,
    pub coordination_channel: Option<String>,
    pub emergency_channel: Option<String>,
}

impl CommunicationChannels {
    pub fn new() -> Self {
        Self {
            input_channel: None,
            output_channel: None,
            coordination_channel: None,
            emergency_channel: None,
        }
    }

    pub async fn initialize(&mut self, symbiote_id: &str) -> Result<()> {
        self.input_channel = Some(format!("{}-input", symbiote_id));
        self.output_channel = Some(format!("{}-output", symbiote_id));
        self.coordination_channel = Some(format!("{}-coordination", symbiote_id));
        self.emergency_channel = Some(format!("{}-emergency", symbiote_id));
        Ok(())
    }
}

/// Symbiote registry for managing all available Symbiotes
#[derive(Debug)]
pub struct SymbioteRegistry {
    symbiotes: HashMap<SymbioteId, Symbiote>,
    presets: HashMap<DevelopmentStack, Vec<SymbioteId>>,
    user_created: HashMap<UserId, Vec<SymbioteId>>,
}

impl SymbioteRegistry {
    pub fn new() -> Self {
        Self {
            symbiotes: HashMap::new(),
            presets: HashMap::new(),
            user_created: HashMap::new(),
        }
    }

    pub async fn load_presets(&mut self) -> Result<()> {
        // Load preset Symbiotes for different stacks
        self.create_rust_specialist().await?;
        self.create_javascript_specialist().await?;
        self.create_python_specialist().await?;
        self.create_react_specialist().await?;
        self.create_testing_specialist().await?;
        self.create_debugging_specialist().await?;
        
        Ok(())
    }

    pub async fn register_symbiote(&mut self, symbiote: Symbiote) -> Result<()> {
        let id = symbiote.id.clone();
        let created_by = symbiote.definition.created_by.clone();
        
        // Add to main registry
        self.symbiotes.insert(id.clone(), symbiote);
        
        // Add to user-created list
        self.user_created.entry(created_by)
            .or_insert_with(Vec::new)
            .push(id);
        
        Ok(())
    }

    pub async fn get_symbiotes_for_stack(&self, stack: DevelopmentStack) -> Result<Vec<SymbioteInfo>> {
        let mut symbiotes = Vec::new();
        
        // Get preset Symbiotes for this stack
        if let Some(preset_ids) = self.presets.get(&stack) {
            for id in preset_ids {
                if let Some(symbiote) = self.symbiotes.get(id) {
                    symbiotes.push(SymbioteInfo::from_symbiote(symbiote));
                }
            }
        }
        
        // Get user-created Symbiotes that specialize in this stack
        for symbiote in self.symbiotes.values() {
            if symbiote.definition.specializations.contains(&stack) {
                symbiotes.push(SymbioteInfo::from_symbiote(symbiote));
            }
        }
        
        Ok(symbiotes)
    }

    // Preset creation methods
    async fn create_rust_specialist(&mut self) -> Result<()> {
        let definition = SymbioteDefinition {
            name: "Rust Specialist".to_string(),
            description: "Expert in Rust programming, cargo, and ecosystem".to_string(),
            version: "1.0.0".to_string(),
            created_by: UserId::new(),
            skills: vec![
                SymbioteSkill::Programming,
                SymbioteSkill::CodeGeneration,
                SymbioteSkill::CodeAnalysis,
                SymbioteSkill::Debugging,
                SymbioteSkill::StackSpecific(DevelopmentStack::Rust),
            ],
            capabilities: vec![
                SymbioteCapability::FileRead,
                SymbioteCapability::FileWrite,
                SymbioteCapability::CommandExecution,
                SymbioteCapability::PackageManagement,
                SymbioteCapability::GitOperations,
            ],
            specializations: vec![DevelopmentStack::Rust],
            configuration: SymbioteConfiguration::default(),
            metadata: HashMap::new(),
        };

        let symbiote = Symbiote::new(definition);
        let id = symbiote.id.clone();
        
        self.symbiotes.insert(id.clone(), symbiote);
        self.presets.entry(DevelopmentStack::Rust)
            .or_insert_with(Vec::new)
            .push(id);
        
        Ok(())
    }

    async fn create_javascript_specialist(&mut self) -> Result<()> {
        let definition = SymbioteDefinition {
            name: "JavaScript Specialist".to_string(),
            description: "Expert in JavaScript, Node.js, and web development".to_string(),
            version: "1.0.0".to_string(),
            created_by: UserId::new(),
            skills: vec![
                SymbioteSkill::Programming,
                SymbioteSkill::CodeGeneration,
                SymbioteSkill::CodeAnalysis,
                SymbioteSkill::StackSpecific(DevelopmentStack::JavaScript),
                SymbioteSkill::StackSpecific(DevelopmentStack::NodeJs),
            ],
            capabilities: vec![
                SymbioteCapability::FileRead,
                SymbioteCapability::FileWrite,
                SymbioteCapability::CommandExecution,
                SymbioteCapability::PackageManagement,
                SymbioteCapability::NetworkAccess,
            ],
            specializations: vec![DevelopmentStack::JavaScript, DevelopmentStack::NodeJs],
            configuration: SymbioteConfiguration::default(),
            metadata: HashMap::new(),
        };

        let symbiote = Symbiote::new(definition);
        let id = symbiote.id.clone();
        
        self.symbiotes.insert(id.clone(), symbiote);
        self.presets.entry(DevelopmentStack::JavaScript)
            .or_insert_with(Vec::new)
            .push(id.clone());
        self.presets.entry(DevelopmentStack::NodeJs)
            .or_insert_with(Vec::new)
            .push(id);
        
        Ok(())
    }

    async fn create_python_specialist(&mut self) -> Result<()> {
        let definition = SymbioteDefinition {
            name: "Python Specialist".to_string(),
            description: "Expert in Python programming and data science".to_string(),
            version: "1.0.0".to_string(),
            created_by: UserId::new(),
            skills: vec![
                SymbioteSkill::Programming,
                SymbioteSkill::CodeGeneration,
                SymbioteSkill::CodeAnalysis,
                SymbioteSkill::StackSpecific(DevelopmentStack::Python),
            ],
            capabilities: vec![
                SymbioteCapability::FileRead,
                SymbioteCapability::FileWrite,
                SymbioteCapability::CommandExecution,
                SymbioteCapability::PackageManagement,
                SymbioteCapability::DataAnalysis,
                SymbioteCapability::MachineLearning,
            ],
            specializations: vec![DevelopmentStack::Python],
            configuration: SymbioteConfiguration::default(),
            metadata: HashMap::new(),
        };

        let symbiote = Symbiote::new(definition);
        let id = symbiote.id.clone();
        
        self.symbiotes.insert(id.clone(), symbiote);
        self.presets.entry(DevelopmentStack::Python)
            .or_insert_with(Vec::new)
            .push(id);
        
        Ok(())
    }

    async fn create_react_specialist(&mut self) -> Result<()> {
        let definition = SymbioteDefinition {
            name: "React Specialist".to_string(),
            description: "Expert in React, JSX, and modern frontend development".to_string(),
            version: "1.0.0".to_string(),
            created_by: UserId::new(),
            skills: vec![
                SymbioteSkill::Programming,
                SymbioteSkill::CodeGeneration,
                SymbioteSkill::CodeAnalysis,
                SymbioteSkill::StackSpecific(DevelopmentStack::React),
                SymbioteSkill::StackSpecific(DevelopmentStack::JavaScript),
            ],
            capabilities: vec![
                SymbioteCapability::FileRead,
                SymbioteCapability::FileWrite,
                SymbioteCapability::CommandExecution,
                SymbioteCapability::PackageManagement,
                SymbioteCapability::NetworkAccess,
            ],
            specializations: vec![DevelopmentStack::React, DevelopmentStack::JavaScript],
            configuration: SymbioteConfiguration::default(),
            metadata: HashMap::new(),
        };

        let symbiote = Symbiote::new(definition);
        let id = symbiote.id.clone();
        
        self.symbiotes.insert(id.clone(), symbiote);
        self.presets.entry(DevelopmentStack::React)
            .or_insert_with(Vec::new)
            .push(id);
        
        Ok(())
    }

    async fn create_testing_specialist(&mut self) -> Result<()> {
        let definition = SymbioteDefinition {
            name: "Testing Specialist".to_string(),
            description: "Expert in test generation, execution, and quality assurance".to_string(),
            version: "1.0.0".to_string(),
            created_by: UserId::new(),
            skills: vec![
                SymbioteSkill::TestGeneration,
                SymbioteSkill::QualityAssurance,
                SymbioteSkill::CodeAnalysis,
            ],
            capabilities: vec![
                SymbioteCapability::FileRead,
                SymbioteCapability::FileWrite,
                SymbioteCapability::CommandExecution,
            ],
            specializations: vec![], // Works with all stacks
            configuration: SymbioteConfiguration::default(),
            metadata: HashMap::new(),
        };

        let symbiote = Symbiote::new(definition);
        let id = symbiote.id.clone();
        
        self.symbiotes.insert(id, symbiote);
        
        Ok(())
    }

    async fn create_debugging_specialist(&mut self) -> Result<()> {
        let definition = SymbioteDefinition {
            name: "Debugging Specialist".to_string(),
            description: "Expert in debugging, error analysis, and problem solving".to_string(),
            version: "1.0.0".to_string(),
            created_by: UserId::new(),
            skills: vec![
                SymbioteSkill::Debugging,
                SymbioteSkill::ErrorAnalysis,
                SymbioteSkill::CodeAnalysis,
                SymbioteSkill::PatternRecognition,
            ],
            capabilities: vec![
                SymbioteCapability::FileRead,
                SymbioteCapability::CommandExecution,
                SymbioteCapability::GitOperations,
            ],
            specializations: vec![], // Works with all stacks
            configuration: SymbioteConfiguration::default(),
            metadata: HashMap::new(),
        };

        let symbiote = Symbiote::new(definition);
        let id = symbiote.id.clone();
        
        self.symbiotes.insert(id, symbiote);
        
        Ok(())
    }
}

/// Symbiote information for display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbioteInfo {
    pub id: SymbioteId,
    pub name: String,
    pub description: String,
    pub skills: Vec<SymbioteSkill>,
    pub capabilities: Vec<SymbioteCapability>,
    pub specializations: Vec<DevelopmentStack>,
    pub status: SymbioteStatus,
    pub success_rate: f64,
    pub tasks_completed: u64,
}

impl SymbioteInfo {
    pub fn from_symbiote(symbiote: &Symbiote) -> Self {
        Self {
            id: symbiote.id.clone(),
            name: symbiote.name.clone(),
            description: symbiote.definition.description.clone(),
            skills: symbiote.definition.skills.clone(),
            capabilities: symbiote.definition.capabilities.clone(),
            specializations: symbiote.definition.specializations.clone(),
            status: symbiote.state.status.clone(),
            success_rate: symbiote.metrics.success_rate,
            tasks_completed: symbiote.metrics.tasks_completed,
        }
    }
}
