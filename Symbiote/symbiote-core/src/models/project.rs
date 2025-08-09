//! # Project Models for Symbiote IDE
//!
//! Comprehensive project data models with settings, collaboration, and relationships.
//! Following Week 5-6 Database & Storage Systems implementation plan.

use crate::{ProjectId, UserId, AgentId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Core project entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    /// Unique project identifier
    pub id: ProjectId,

    /// Project name
    pub name: String,

    /// Project description
    pub description: Option<String>,

    /// Project owner
    pub owner_id: UserId,

    /// Project visibility
    pub visibility: ProjectVisibility,

    /// Project status
    pub status: ProjectStatus,

    /// Project type/template
    pub project_type: ProjectType,

    /// Programming languages used
    pub languages: Vec<String>,

    /// Project tags
    pub tags: Vec<String>,

    /// Repository URL
    pub repository_url: Option<String>,

    /// Local project path
    pub local_path: Option<PathBuf>,

    /// Project settings
    pub settings: ProjectSettings,

    /// Creation timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,

    /// Last update timestamp
    pub updated_at: chrono::DateTime<chrono::Utc>,

    /// Last accessed timestamp
    pub last_accessed_at: Option<chrono::DateTime<chrono::Utc>>,

    /// Project size in bytes
    pub size_bytes: Option<u64>,

    /// Number of files
    pub file_count: Option<u32>,

    /// Number of lines of code
    pub line_count: Option<u64>,
}

/// Project visibility settings
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProjectVisibility {
    Private,
    Internal,
    Public,
}

/// Project status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProjectStatus {
    Active,
    Archived,
    Deleted,
    Template,
}

/// Project type/template
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProjectType {
    WebApp,
    MobileApp,
    Desktop,
    Library,
    CLI,
    API,
    Microservice,
    Game,
    DataScience,
    MachineLearning,
    Blockchain,
    IoT,
    Custom(String),
}

/// Project settings and configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSettings {
    /// Build configuration
    pub build_config: BuildConfig,

    /// AI assistant settings
    pub ai_config: ProjectAIConfig,

    /// Code quality settings
    pub quality_config: QualityConfig,

    /// Collaboration settings
    pub collaboration_config: CollaborationConfig,

    /// Environment variables
    pub environment_variables: HashMap<String, String>,

    /// Custom settings
    pub custom_settings: HashMap<String, serde_json::Value>,
}

/// Build configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildConfig {
    /// Build command
    pub build_command: Option<String>,

    /// Test command
    pub test_command: Option<String>,

    /// Run command
    pub run_command: Option<String>,

    /// Build output directory
    pub output_dir: Option<String>,

    /// Build environment
    pub environment: String,

    /// Auto-build on save
    pub auto_build: bool,

    /// Build timeout in seconds
    pub timeout_seconds: u32,
}

/// AI configuration for the project
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectAIConfig {
    /// Enabled AI features
    pub enabled_features: Vec<String>,

    /// Default AI model for this project
    pub default_model: Option<String>,

    /// AI context settings
    pub context_settings: AIContextSettings,

    /// Code generation preferences
    pub code_generation: CodeGenerationConfig,

    /// Review and analysis settings
    pub analysis_config: AnalysisConfig,
}

/// AI context settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIContextSettings {
    /// Include project files in context
    pub include_project_files: bool,

    /// Include git history
    pub include_git_history: bool,

    /// Include documentation
    pub include_documentation: bool,

    /// Maximum context size
    pub max_context_size: u32,

    /// Context refresh interval
    pub refresh_interval_minutes: u32,
}

/// Code generation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeGenerationConfig {
    /// Auto-generate tests
    pub auto_generate_tests: bool,

    /// Auto-generate documentation
    pub auto_generate_docs: bool,

    /// Code style preferences
    pub style_preferences: HashMap<String, String>,

    /// Generation templates
    pub templates: Vec<String>,
}

/// Analysis configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisConfig {
    /// Auto code review
    pub auto_review: bool,

    /// Security analysis
    pub security_analysis: bool,

    /// Performance analysis
    pub performance_analysis: bool,

    /// Dependency analysis
    pub dependency_analysis: bool,

    /// Analysis frequency
    pub analysis_frequency: AnalysisFrequency,
}

/// Analysis frequency
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnalysisFrequency {
    OnSave,
    OnCommit,
    Daily,
    Weekly,
    Manual,
}

/// Code quality configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityConfig {
    /// Linting enabled
    pub linting_enabled: bool,

    /// Linting rules
    pub linting_rules: Vec<String>,

    /// Code formatting
    pub formatting_config: FormattingConfig,

    /// Quality gates
    pub quality_gates: QualityGates,
}

/// Code formatting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormattingConfig {
    /// Auto-format on save
    pub auto_format: bool,

    /// Formatter to use
    pub formatter: String,

    /// Formatting rules
    pub rules: HashMap<String, serde_json::Value>,
}

/// Quality gates configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityGates {
    /// Minimum test coverage percentage
    pub min_test_coverage: Option<f32>,

    /// Maximum complexity score
    pub max_complexity: Option<u32>,

    /// Maximum technical debt ratio
    pub max_tech_debt_ratio: Option<f32>,

    /// Required code review
    pub require_code_review: bool,
}

/// Collaboration configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollaborationConfig {
    /// Real-time collaboration enabled
    pub real_time_enabled: bool,

    /// Auto-save interval for collaboration
    pub auto_save_interval: u32,

    /// Conflict resolution strategy
    pub conflict_resolution: ConflictResolution,

    /// Notification settings
    pub notifications: CollaborationNotifications,
}

/// Conflict resolution strategy
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConflictResolution {
    Manual,
    LastWriteWins,
    MergeChanges,
    CreateBranch,
}

/// Collaboration notification settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollaborationNotifications {
    /// Notify on file changes
    pub file_changes: bool,

    /// Notify on user join/leave
    pub user_activity: bool,

    /// Notify on comments
    pub comments: bool,

    /// Notify on builds
    pub builds: bool,
}

/// Project file metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectFile {
    /// File ID
    pub id: String,

    /// Project ID
    pub project_id: ProjectId,

    /// File path relative to project root
    pub path: PathBuf,

    /// File size in bytes
    pub size_bytes: u64,

    /// File type/extension
    pub file_type: String,

    /// Programming language
    pub language: Option<String>,

    /// File hash for change detection
    pub hash: String,

    /// Last modified timestamp
    pub modified_at: chrono::DateTime<chrono::Utc>,

    /// Created timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,

    /// Whether file is binary
    pub is_binary: bool,

    /// Whether file is ignored
    pub is_ignored: bool,

    /// Line count
    pub line_count: Option<u32>,

    /// Character count
    pub char_count: Option<u32>,
}

/// Project agent assignment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectAgent {
    /// Project ID
    pub project_id: ProjectId,

    /// Agent ID
    pub agent_id: AgentId,

    /// Agent role in the project
    pub role: AgentRole,

    /// Agent permissions
    pub permissions: Vec<String>,

    /// Agent configuration for this project
    pub config: HashMap<String, serde_json::Value>,

    /// When agent was assigned
    pub assigned_at: chrono::DateTime<chrono::Utc>,

    /// Agent status in project
    pub status: AgentStatus,
}

/// Agent role in project
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentRole {
    CodeAssistant,
    Reviewer,
    Tester,
    DocumentationWriter,
    SecurityAnalyzer,
    PerformanceOptimizer,
    Custom(String),
}

/// Agent status in project
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentStatus {
    Active,
    Inactive,
    Suspended,
    Error,
}

/// Project activity log
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectActivity {
    /// Activity ID
    pub id: String,

    /// Project ID
    pub project_id: ProjectId,

    /// User who performed the activity
    pub user_id: Option<UserId>,

    /// Agent who performed the activity
    pub agent_id: Option<AgentId>,

    /// Activity type
    pub activity_type: ActivityType,

    /// Activity description
    pub description: String,

    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,

    /// Activity timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Types of project activities
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActivityType {
    FileCreated,
    FileModified,
    FileDeleted,
    FileMoved,
    CommitMade,
    BranchCreated,
    BranchMerged,
    BuildStarted,
    BuildCompleted,
    BuildFailed,
    TestsRun,
    DeploymentStarted,
    DeploymentCompleted,
    UserJoined,
    UserLeft,
    AgentAssigned,
    AgentRemoved,
    SettingsChanged,
    Custom(String),
}

impl Default for ProjectSettings {
    fn default() -> Self {
        Self {
            build_config: BuildConfig::default(),
            ai_config: ProjectAIConfig::default(),
            quality_config: QualityConfig::default(),
            collaboration_config: CollaborationConfig::default(),
            environment_variables: HashMap::new(),
            custom_settings: HashMap::new(),
        }
    }
}

impl Default for BuildConfig {
    fn default() -> Self {
        Self {
            build_command: None,
            test_command: None,
            run_command: None,
            output_dir: None,
            environment: "development".to_string(),
            auto_build: false,
            timeout_seconds: 300, // 5 minutes
        }
    }
}

impl Default for ProjectAIConfig {
    fn default() -> Self {
        Self {
            enabled_features: vec![
                "code_completion".to_string(),
                "code_review".to_string(),
                "documentation".to_string(),
            ],
            default_model: None,
            context_settings: AIContextSettings::default(),
            code_generation: CodeGenerationConfig::default(),
            analysis_config: AnalysisConfig::default(),
        }
    }
}

impl Default for AIContextSettings {
    fn default() -> Self {
        Self {
            include_project_files: true,
            include_git_history: true,
            include_documentation: true,
            max_context_size: 100000, // 100k tokens
            refresh_interval_minutes: 30,
        }
    }
}

impl Default for CodeGenerationConfig {
    fn default() -> Self {
        Self {
            auto_generate_tests: false,
            auto_generate_docs: false,
            style_preferences: HashMap::new(),
            templates: Vec::new(),
        }
    }
}

impl Default for AnalysisConfig {
    fn default() -> Self {
        Self {
            auto_review: true,
            security_analysis: true,
            performance_analysis: false,
            dependency_analysis: true,
            analysis_frequency: AnalysisFrequency::OnCommit,
        }
    }
}

impl Default for QualityConfig {
    fn default() -> Self {
        Self {
            linting_enabled: true,
            linting_rules: Vec::new(),
            formatting_config: FormattingConfig::default(),
            quality_gates: QualityGates::default(),
        }
    }
}

impl Default for FormattingConfig {
    fn default() -> Self {
        Self {
            auto_format: true,
            formatter: "default".to_string(),
            rules: HashMap::new(),
        }
    }
}

impl Default for QualityGates {
    fn default() -> Self {
        Self {
            min_test_coverage: Some(80.0),
            max_complexity: Some(10),
            max_tech_debt_ratio: Some(5.0),
            require_code_review: true,
        }
    }
}

impl Default for CollaborationConfig {
    fn default() -> Self {
        Self {
            real_time_enabled: true,
            auto_save_interval: 30, // 30 seconds
            conflict_resolution: ConflictResolution::Manual,
            notifications: CollaborationNotifications::default(),
        }
    }
}

impl Default for CollaborationNotifications {
    fn default() -> Self {
        Self {
            file_changes: true,
            user_activity: true,
            comments: true,
            builds: false,
        }
    }
}

impl Project {
    /// Create a new project
    pub fn new(name: String, owner_id: UserId, project_type: ProjectType) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: ProjectId::new(),
            name,
            description: None,
            owner_id,
            visibility: ProjectVisibility::Private,
            status: ProjectStatus::Active,
            project_type,
            languages: Vec::new(),
            tags: Vec::new(),
            repository_url: None,
            local_path: None,
            settings: ProjectSettings::default(),
            created_at: now,
            updated_at: now,
            last_accessed_at: None,
            size_bytes: None,
            file_count: None,
            line_count: None,
        }
    }

    /// Check if project is active
    pub fn is_active(&self) -> bool {
        self.status == ProjectStatus::Active
    }

    /// Update last accessed time
    pub fn update_last_accessed(&mut self) {
        self.last_accessed_at = Some(chrono::Utc::now());
        self.updated_at = chrono::Utc::now();
    }

    /// Add a language to the project
    pub fn add_language(&mut self, language: String) {
        if !self.languages.contains(&language) {
            self.languages.push(language);
            self.updated_at = chrono::Utc::now();
        }
    }

    /// Add a tag to the project
    pub fn add_tag(&mut self, tag: String) {
        if !self.tags.contains(&tag) {
            self.tags.push(tag);
            self.updated_at = chrono::Utc::now();
        }
    }
}
