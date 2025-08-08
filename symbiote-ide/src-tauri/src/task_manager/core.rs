use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

// Type alias for agent identification
pub type AgentId = String;

// TaskManager is defined in manager.rs

/// Project status
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProjectStatus {
    Planning,
    Active,
    OnHold,
    Completed,
    Cancelled,
}

/// Task manager configuration modes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskManagerMode {
    Basic,      // SQLite only (always available)
    Advanced,   // SQLite + Qdrant + Neo4j (requires user setup)
}

// TaskPriority is now defined in agent_orchestrator.rs to avoid duplication
pub use crate::agent_orchestrator::TaskPriority;

/// Task status
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TaskStatus {
    Todo,
    InProgress,
    Blocked,
    Review,
    Done,
    Cancelled,
}

/// Task type for categorization
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskType {
    Feature,
    Bug,
    Research,
    Documentation,
    Testing,
    Refactor,
    Planning,
    Meeting,
    Deployment,
    Security,
    Performance,
    Custom(String),
}

/// Core task structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub status: TaskStatus,
    pub priority: TaskPriority,
    pub task_type: TaskType,
    
    // Hierarchy and organization
    pub parent_id: Option<String>,
    pub project_id: Option<String>,
    pub epic_id: Option<String>,
    
    // Assignment and ownership
    pub assigned_to: Option<String>, // User or Agent ID
    pub created_by: String,
    pub reviewers: Vec<String>,
    
    // Multi-agent collaboration tracking
    pub active_agents: Vec<AgentAssignment>,
    pub agent_history: Vec<AgentActivity>,
    pub collaboration_mode: CollaborationMode,

    // Task relationships and requirements
    pub dependencies: Vec<String>,        // Task IDs this task depends on
    pub subtasks: Vec<String>,           // Child task IDs
    pub required_capabilities: Vec<String>, // Required agent capabilities
    
    // Timing and scheduling
    pub created_at: u64,
    pub updated_at: u64,
    pub due_date: Option<u64>,
    pub start_date: Option<u64>,
    pub estimated_hours: Option<f32>,
    pub actual_hours: Option<f32>,

    // Duration aliases for compatibility
    pub estimated_duration: Option<f32>, // Alias for estimated_hours
    pub actual_duration: Option<f32>,    // Alias for actual_hours
    
    // Progress tracking
    pub progress_percentage: u8, // 0-100
    pub completion_notes: Option<String>,
    pub blockers: Vec<String>,
    
    // Metadata and categorization
    pub tags: Vec<String>,
    pub labels: Vec<String>,
    pub components: Vec<String>, // Code components affected
    pub files_affected: Vec<String>, // File paths
    pub metadata: HashMap<String, String>,
    
    // Advanced features (when available)
    pub semantic_embedding: Option<Vec<f32>>, // For Qdrant search
    pub knowledge_graph_id: Option<String>,   // For Neo4j relationships
}

/// Project structure for organizing tasks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub status: ProjectStatus,
    pub created_at: u64,
    pub updated_at: u64,
    pub due_date: Option<u64>,
    pub start_date: Option<u64>,
    pub owner: String,
    pub team_members: Vec<String>,
    pub tags: Vec<String>,
    pub repository_url: Option<String>,
    pub documentation_url: Option<String>,
    pub metadata: HashMap<String, String>,
}

// Duplicate ProjectStatus removed - using the one defined earlier

/// Epic for grouping related features
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Epic {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub project_id: String,
    pub status: EpicStatus,
    pub created_at: u64,
    pub updated_at: u64,
    pub due_date: Option<u64>,
    pub owner: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EpicStatus {
    Planning,
    InProgress,
    Done,
    Cancelled,
}

/// Task dependency relationship
// TaskDependency is now defined in agent_orchestrator.rs to avoid duplication
pub use crate::agent_orchestrator::TaskDependency;

// DependencyType is now defined in agent_orchestrator.rs to avoid duplication
pub use crate::agent_orchestrator::DependencyType;

/// Task comment for collaboration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskComment {
    pub id: String,
    pub task_id: String,
    pub author: String,
    pub content: String,
    pub created_at: u64,
    pub updated_at: Option<u64>,
    pub comment_type: CommentType,
    pub mentions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CommentType {
    General,
    StatusUpdate,
    Question,
    Blocker,
    Solution,
    Review,
}

/// Time tracking entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeEntry {
    pub id: String,
    pub task_id: String,
    pub user_id: String,
    pub start_time: u64,
    pub end_time: Option<u64>,
    pub duration_hours: Option<f32>,
    pub description: Option<String>,
    pub created_at: u64,
}

/// Task manager configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskManagerConfig {
    pub mode: TaskManagerMode,
    pub enable_semantic_search: bool,
    pub enable_knowledge_graph: bool,
    pub auto_assign_agents: bool,
    pub smart_scheduling: bool,
    pub time_tracking: bool,
    pub notifications: bool,
}

impl Default for TaskManagerConfig {
    fn default() -> Self {
        Self {
            mode: TaskManagerMode::Basic,
            enable_semantic_search: false,
            enable_knowledge_graph: false,
            auto_assign_agents: false,
            smart_scheduling: true,
            time_tracking: true,
            notifications: true,
        }
    }
}

/// Error types
#[derive(Debug, thiserror::Error)]
pub enum TaskManagerError {
    #[error("Database error: {0}")]
    DatabaseError(String),
    
    #[error("Task not found: {0}")]
    TaskNotFound(String),
    
    #[error("Project not found: {0}")]
    ProjectNotFound(String),
    
    #[error("Epic not found: {0}")]
    EpicNotFound(String),
    
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    
    #[error("Circular dependency detected")]
    CircularDependency,
    
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    
    #[error("Search error: {0}")]
    SearchError(String),
    
    #[error("Assignment error: {0}")]
    AssignmentError(String),
}

/// Agent assignment to task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentAssignment {
    pub agent_id: String,
    pub agent_type: String, // e.g., "Developer", "Tester", "Security"
    pub model_info: ModelInfo,
    pub assigned_at: u64,
    pub status: AgentTaskStatus,
    pub progress_notes: Option<String>,
    pub estimated_completion: Option<u64>,
}

/// Model information for agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub provider: String,    // e.g., "OpenAI", "Anthropic", "Google"
    pub model_name: String,  // e.g., "gpt-4", "claude-3-sonnet", "gemini-pro"
    pub version: Option<String>,
    pub context_window: Option<u32>,
    pub icon_url: Option<String>, // For UI display
}

/// Agent task status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[allow(non_camel_case_types)]
pub enum AgentTaskStatus {
    Assigned,
    Working,
    Paused,
    Completed,
    Failed,
    Handed_Off,
}

/// Agent activity history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentActivity {
    pub agent_id: String,
    pub agent_type: String,
    pub model_info: ModelInfo,
    pub activity_type: AgentActivityType,
    pub timestamp: u64,
    pub description: String,
    pub duration_seconds: Option<u32>,
    pub result: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(non_camel_case_types)]
pub enum AgentActivityType {
    Assigned,
    Started,
    Progress_Update,
    Paused,
    Resumed,
    Completed,
    Failed,
    Handed_Off,
    Comment_Added,
}

/// Task collaboration mode
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CollaborationMode {
    Single,     // Only one agent can work on this task
    Parallel,   // Multiple agents can work simultaneously
    Sequential, // Agents work in sequence (handoffs)
    Review,     // Multiple agents for review/validation
}

impl Default for CollaborationMode {
    fn default() -> Self {
        Self::Single
    }
}

/// Agent coordination information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCoordination {
    pub coordination_id: String,
    pub participating_agents: Vec<String>,
    pub coordination_type: CoordinationType,
    pub started_at: u64,
    pub status: CoordinationStatus,
    pub shared_context: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(non_camel_case_types)]
pub enum CoordinationType {
    Parallel_Work,
    Sequential_Handoff,
    Collaborative_Review,
    Conflict_Resolution,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CoordinationStatus {
    Active,
    Completed,
    Failed,
    Cancelled,
}

/// Utility functions
pub fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

pub fn generate_id() -> String {
    Uuid::new_v4().to_string()
}
