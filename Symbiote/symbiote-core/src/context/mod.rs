//! # Context Management System for Symbiote IDE
//! 
//! Complete context management system with ContextBus, GlobalContext, 
//! knowledge graph, and context optimization.
//! 
//! Following Week 13-14 Context Management & Knowledge Graph implementation plan.

use crate::{Result, SymbioteError};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use uuid::Uuid;
use std::time::{SystemTime, UNIX_EPOCH};

pub mod bus;
pub mod global;
pub mod optimization;
pub mod compression;
pub mod knowledge_graph;
pub mod enhanced;
pub mod realtime;
pub mod intelligence;
pub mod mcp;

pub use bus::*;
pub use global::*;
pub use optimization::*;
pub use compression::*;
pub use knowledge_graph::*;
pub use enhanced::*;
pub use realtime::*;
pub use intelligence::*;
pub use mcp::*;

/// System identifier for context management
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SystemId {
    pub name: String,
    pub instance_id: String,
}

impl SystemId {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            instance_id: Uuid::new_v4().to_string(),
        }
    }
}

/// Context update event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextUpdate {
    pub id: String,
    pub system_id: SystemId,
    pub update_type: ContextUpdateType,
    pub data: serde_json::Value,
    pub timestamp: u64,
    pub priority: UpdatePriority,
}

/// Types of context updates
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ContextUpdateType {
    FileOpened,
    FileModified,
    FileClosed,
    WorkspaceChanged,
    ConversationStarted,
    ConversationUpdated,
    WorkflowStarted,
    WorkflowCompleted,
    AgentStateChanged,
    UserAction,
    SystemEvent,
}

/// Priority levels for context updates
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum UpdatePriority {
    Low,
    Normal,
    High,
    Critical,
}

impl ContextUpdate {
    pub fn new(system_id: SystemId, update_type: ContextUpdateType, data: serde_json::Value) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            system_id,
            update_type,
            data,
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            priority: UpdatePriority::Normal,
        }
    }

    pub fn with_priority(mut self, priority: UpdatePriority) -> Self {
        self.priority = priority;
        self
    }

    pub fn should_optimize(&self) -> bool {
        matches!(self.update_type, 
            ContextUpdateType::WorkspaceChanged | 
            ContextUpdateType::ConversationStarted |
            ContextUpdateType::WorkflowCompleted
        )
    }
}

/// Context subscriber for receiving updates
#[derive(Debug)]
pub struct ContextSubscriber {
    pub system_id: SystemId,
    pub sender: mpsc::UnboundedSender<ContextUpdate>,
    pub filter: ContextFilter,
}

/// Filter for context updates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextFilter {
    pub update_types: Option<Vec<ContextUpdateType>>,
    pub min_priority: UpdatePriority,
    pub system_ids: Option<Vec<SystemId>>,
}

impl ContextFilter {
    pub fn new() -> Self {
        Self {
            update_types: None,
            min_priority: UpdatePriority::Low,
            system_ids: None,
        }
    }

    pub fn with_types(mut self, types: Vec<ContextUpdateType>) -> Self {
        self.update_types = Some(types);
        self
    }

    pub fn with_min_priority(mut self, priority: UpdatePriority) -> Self {
        self.min_priority = priority;
        self
    }

    pub fn matches(&self, update: &ContextUpdate) -> bool {
        // Check priority
        if update.priority < self.min_priority {
            return false;
        }

        // Check update types
        if let Some(ref types) = self.update_types {
            if !types.contains(&update.update_type) {
                return false;
            }
        }

        // Check system IDs
        if let Some(ref system_ids) = self.system_ids {
            if !system_ids.contains(&update.system_id) {
                return false;
            }
        }

        true
    }
}

/// System-specific context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemContext {
    pub system_id: SystemId,
    pub workspace_context: Option<WorkspaceContext>,
    pub file_contexts: HashMap<String, FileContext>,
    pub conversation_contexts: HashMap<String, ConversationContext>,
    pub workflow_contexts: HashMap<String, WorkflowContext>,
    pub agent_contexts: HashMap<String, AgentContext>,
    pub last_updated: u64,
}

/// Workspace context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceContext {
    pub path: String,
    pub name: String,
    pub project_type: ProjectType,
    pub dependencies: Vec<Dependency>,
    pub configuration: HashMap<String, serde_json::Value>,
    pub recent_files: VecDeque<String>,
    pub git_info: Option<GitInfo>,
}

/// Project type classification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProjectType {
    Rust,
    Python,
    JavaScript,
    TypeScript,
    React,
    Vue,
    Angular,
    Node,
    Web,
    Mobile,
    Desktop,
    Library,
    Unknown,
}

/// Dependency information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    pub name: String,
    pub version: String,
    pub dependency_type: DependencyType,
}

/// Dependency types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DependencyType {
    Runtime,
    Development,
    Build,
    Test,
}

/// Git repository information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitInfo {
    pub branch: String,
    pub commit_hash: String,
    pub remote_url: Option<String>,
    pub status: GitStatus,
}

/// Git status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitStatus {
    pub modified_files: Vec<String>,
    pub staged_files: Vec<String>,
    pub untracked_files: Vec<String>,
}

/// File context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileContext {
    pub path: String,
    pub language: String,
    pub content_hash: String,
    pub symbols: Vec<Symbol>,
    pub imports: Vec<Import>,
    pub exports: Vec<Export>,
    pub last_modified: u64,
    pub cursor_position: Option<Position>,
    pub selection: Option<Range>,
}

/// Symbol in a file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Symbol {
    pub name: String,
    pub symbol_type: SymbolType,
    pub position: Position,
    pub range: Range,
    pub visibility: Visibility,
}

/// Symbol types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SymbolType {
    Function,
    Class,
    Interface,
    Variable,
    Constant,
    Type,
    Module,
    Namespace,
}

/// Symbol visibility
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Visibility {
    Public,
    Private,
    Protected,
    Internal,
}

/// Import statement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Import {
    pub source: String,
    pub symbols: Vec<String>,
    pub alias: Option<String>,
    pub position: Position,
}

/// Export statement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Export {
    pub symbol: String,
    pub export_type: ExportType,
    pub position: Position,
}

/// Export types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExportType {
    Default,
    Named,
    Namespace,
}

/// Position in a file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub line: u32,
    pub column: u32,
}

/// Range in a file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

/// Conversation context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationContext {
    pub id: String,
    pub title: String,
    pub messages: VecDeque<Message>,
    pub participants: Vec<Participant>,
    pub context_files: Vec<String>,
    pub created_at: u64,
    pub last_activity: u64,
}

/// Message in a conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub sender: Participant,
    pub content: String,
    pub message_type: MessageType,
    pub timestamp: u64,
    pub attachments: Vec<Attachment>,
}

/// Message types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageType {
    Text,
    Code,
    File,
    Image,
    Command,
    System,
}

/// Conversation participant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Participant {
    User(String),
    AI(String),
    System,
}

/// Message attachment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    pub id: String,
    pub attachment_type: AttachmentType,
    pub data: serde_json::Value,
}

/// Attachment types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AttachmentType {
    File,
    Image,
    Code,
    Link,
}

/// Workflow context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowContext {
    pub id: String,
    pub name: String,
    pub status: WorkflowStatus,
    pub steps: Vec<WorkflowStep>,
    pub current_step: Option<usize>,
    pub variables: HashMap<String, serde_json::Value>,
    pub created_at: u64,
    pub started_at: Option<u64>,
    pub completed_at: Option<u64>,
}

/// Workflow status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkflowStatus {
    Created,
    Running,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

/// Workflow step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    pub id: String,
    pub name: String,
    pub step_type: WorkflowStepType,
    pub status: StepStatus,
    pub input: serde_json::Value,
    pub output: Option<serde_json::Value>,
    pub error: Option<String>,
}

/// Workflow step types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkflowStepType {
    FileOperation,
    AIInvocation,
    Command,
    Condition,
    Loop,
    Parallel,
    Custom(String),
}

/// Step status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StepStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Skipped,
}

/// Agent context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentContext {
    pub id: String,
    pub name: String,
    pub agent_type: AgentType,
    pub status: AgentStatus,
    pub current_task: Option<String>,
    pub capabilities: Vec<String>,
    pub memory: AgentMemory,
    pub performance_metrics: PerformanceMetrics,
}

/// Agent types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentType {
    CodeGenerator,
    Reviewer,
    Tester,
    Debugger,
    Optimizer,
    Security,
    Documentation,
    Custom(String),
}

/// Agent status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentStatus {
    Idle,
    Working,
    Waiting,
    Error,
    Offline,
}

/// Agent memory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMemory {
    pub short_term: VecDeque<MemoryItem>,
    pub long_term: HashMap<String, MemoryItem>,
    pub working_memory: HashMap<String, serde_json::Value>,
}

/// Memory item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryItem {
    pub id: String,
    pub content: String,
    pub item_type: MemoryType,
    pub importance: f64,
    pub created_at: u64,
    pub last_accessed: u64,
    pub access_count: u32,
}

/// Memory types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MemoryType {
    Fact,
    Procedure,
    Experience,
    Pattern,
    Preference,
}

/// Performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub tasks_completed: u32,
    pub success_rate: f64,
    pub average_response_time: f64,
    pub user_satisfaction: f64,
    pub last_updated: u64,
}
