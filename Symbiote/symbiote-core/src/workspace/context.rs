//! # Enhanced Workspace Context
//! 
//! Enhanced workspace and global context with proper separation of concerns.

use super::*;
use crate::memory::{MemorySystem, AgentRuleManager, WorkspaceMemory};
use crate::context::{FileContext, ConversationContext, WorkflowContext};
use crate::codebase_index::{CodebaseIndexEngine, WorkspaceIndex};
use super::components::*;

/// Enhanced global context shared across all workspaces
#[derive(Debug, Clone)]
pub struct EnhancedGlobalContext {
    /// User-level data (shared across workspaces)
    pub user_id: String,
    pub user_preferences: GlobalUserPreferences,
    pub session_info: SessionInfo,
    
    /// Cross-workspace data
    pub workspace_ids: std::collections::HashSet<String>,
    pub global_memory: GlobalMemory,
    pub global_knowledge_graph: GlobalKnowledgeGraph,
    
    /// System-wide state
    pub system_settings: SystemSettings,
    pub global_notifications: Vec<String>, // Simplified for now
    
    /// Cross-workspace search and insights
    pub search_index: CrossWorkspaceSearchIndex,
    pub insights: CrossWorkspaceInsights,
    
    /// Timestamps
    pub created_at: DateTime<Utc>,
    pub last_updated: DateTime<Utc>,
}

/// Enhanced workspace context with full isolation
#[derive(Debug, Clone)]
pub struct EnhancedWorkspaceContext {
    /// Workspace identity
    pub id: String,
    pub name: String,
    pub path: String,
    pub project_type: Option<String>,
    
    /// Workspace-specific data (fully isolated)
    pub files: HashMap<String, FileContext>,
    pub conversations: HashMap<String, ConversationContext>,
    pub workflows: HashMap<String, WorkflowContext>,
    pub notebooks: HashMap<String, NotebookContext>,
    pub agents: HashMap<String, WorkspaceAgentContext>,

    /// Workspace-specific components
    pub journal: WorkspaceJournal,
    pub tasks: WorkspaceTaskManager,
    pub notes: WorkspaceNotes,
    pub analytics: WorkspaceAnalytics,
    pub goals: WorkspaceGoals,
    
    /// Workspace-specific systems
    pub agent_rules: WorkspaceAgentRuleManager,
    pub memory: WorkspaceMemory,
    pub knowledge_graph: WorkspaceKnowledgeGraph,
    pub codebase_index: WorkspaceIndex,
    pub graph_3d: Workspace3DGraph,
    pub settings: WorkspaceSettings,
    
    /// Workspace state
    pub git_info: Option<GitInfo>,
    pub dependencies: Vec<Dependency>,
    pub recent_files: Vec<String>,
    pub bookmarks: Vec<Bookmark>,
    
    /// Timestamps
    pub created_at: DateTime<Utc>,
    pub last_modified: DateTime<Utc>,
    pub last_accessed: DateTime<Utc>,
}

/// Global user preferences (shared across workspaces)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalUserPreferences {
    pub theme: String,
    pub language: String,
    pub timezone: String,
    pub notification_settings: GlobalNotificationSettings,
    pub privacy_settings: PrivacySettings,
    pub accessibility_settings: AccessibilitySettings,
}

/// Session information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub session_id: String,
    pub user_id: String,
    pub started_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub device_info: DeviceInfo,
    pub ip_address: Option<String>,
}

/// Global memory (cross-workspace)
#[derive(Debug, Clone)]
pub struct GlobalMemory {
    pub user_profile: UserProfile,
    pub global_patterns: Vec<GlobalBehaviorPattern>,
    pub cross_workspace_insights: Vec<Insight>,
    pub global_preferences: HashMap<String, serde_json::Value>,
}

/// Global knowledge graph (cross-workspace relationships)
#[derive(Debug, Clone)]
pub struct GlobalKnowledgeGraph {
    pub workspace_relationships: HashMap<String, Vec<String>>,
    pub cross_workspace_entities: HashMap<String, Entity>,
    pub global_concepts: HashMap<String, Concept>,
}

/// System settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemSettings {
    pub auto_save_interval: u64,
    pub backup_settings: BackupSettings,
    pub performance_settings: PerformanceSettings,
    pub security_settings: SecuritySettings,
}

/// Cross-workspace search index
#[derive(Debug, Clone)]
pub struct CrossWorkspaceSearchIndex {
    pub indexed_content: HashMap<String, SearchableContent>,
    pub search_history: Vec<SearchQuery>,
    pub popular_searches: Vec<String>,
}

/// Cross-workspace insights
#[derive(Debug, Clone)]
pub struct CrossWorkspaceInsights {
    pub productivity_metrics: ProductivityMetrics,
    pub usage_patterns: Vec<UsagePattern>,
    pub recommendations: Vec<Recommendation>,
}

/// Workspace-specific agent rule manager
#[derive(Debug, Clone)]
pub struct WorkspaceAgentRuleManager {
    /// Agent rules specific to this workspace
    rules: HashMap<String, HashMap<String, crate::memory::AgentRules>>, // agent_type -> user_id -> rules
}

/// Workspace-specific agent context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceAgentContext {
    pub agent_id: String,
    pub agent_type: String,
    pub status: AgentStatus,
    pub workspace_specific_config: HashMap<String, serde_json::Value>,
    pub last_activity: DateTime<Utc>,
    pub performance_metrics: AgentPerformanceMetrics,
}

/// Workspace knowledge graph (workspace-specific)
#[derive(Debug, Clone)]
pub struct WorkspaceKnowledgeGraph {
    pub entities: HashMap<String, Entity>,
    pub relationships: HashMap<String, Vec<Relationship>>,
    pub concepts: HashMap<String, Concept>,
    pub workspace_specific_insights: Vec<Insight>,
}

/// Workspace settings (workspace-specific)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceSettings {
    pub auto_save: bool,
    pub git_integration: bool,
    pub ai_assistance_level: AiAssistanceLevel,
    pub code_style_preferences: CodeStylePreferences,
    pub workflow_preferences: WorkflowPreferences,
    pub notification_preferences: WorkspaceNotificationSettings,
}

/// Notebook context (workspace-specific)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotebookContext {
    pub notebook_id: String,
    pub name: String,
    pub path: String,
    pub kernel_type: String,
    pub cell_count: usize,
    pub last_executed: Option<DateTime<Utc>>,
    pub variables: HashMap<String, String>,
}

// Implementation for EnhancedWorkspaceContext
impl EnhancedWorkspaceContext {
    pub fn new(id: String, path: String, name: String) -> Self {
        let now = Utc::now();
        Self {
            id: id.clone(),
            name,
            path: path.clone(),
            project_type: None,
            files: HashMap::new(),
            conversations: HashMap::new(),
            workflows: HashMap::new(),
            notebooks: HashMap::new(),
            agents: HashMap::new(),

            // Initialize workspace components
            journal: WorkspaceJournal::new(id.clone()),
            tasks: WorkspaceTaskManager::new(id.clone()),
            notes: WorkspaceNotes::new(id.clone()),
            analytics: WorkspaceAnalytics::new(id.clone()),
            goals: WorkspaceGoals::new(id.clone()),

            agent_rules: WorkspaceAgentRuleManager::new(),
            memory: WorkspaceMemory::new(),
            knowledge_graph: WorkspaceKnowledgeGraph::new(),
            codebase_index: WorkspaceIndex::new(id.clone(), path.clone()),
            graph_3d: Workspace3DGraph::new(id.clone()),
            settings: WorkspaceSettings::default(),
            git_info: None,
            dependencies: Vec::new(),
            recent_files: Vec::new(),
            bookmarks: Vec::new(),
            created_at: now,
            last_modified: now,
            last_accessed: now,
        }
    }

    /// Add file to workspace
    pub fn add_file(&mut self, file_id: String, file_context: FileContext) {
        self.files.insert(file_id, file_context);
        self.last_modified = Utc::now();
    }

    /// Add conversation to workspace
    pub fn add_conversation(&mut self, conversation_id: String, conversation: ConversationContext) {
        self.conversations.insert(conversation_id, conversation);
        self.last_modified = Utc::now();
    }

    /// Add notebook to workspace
    pub fn add_notebook(&mut self, notebook_id: String, notebook: NotebookContext) {
        self.notebooks.insert(notebook_id, notebook);
        self.last_modified = Utc::now();
    }

    /// Update last accessed time
    pub fn touch(&mut self) {
        self.last_accessed = Utc::now();
    }
}

impl WorkspaceAgentRuleManager {
    pub fn new() -> Self {
        Self {
            rules: HashMap::new(),
        }
    }

    pub async fn get_rules(&self, agent_type: &str, user_id: &str) -> Result<crate::memory::AgentRules> {
        if let Some(agent_rules) = self.rules.get(agent_type) {
            if let Some(user_rules) = agent_rules.get(user_id) {
                return Ok(user_rules.clone());
            }
        }
        
        // Return default rules for this workspace
        Ok(crate::memory::AgentRules::default_for_agent(agent_type))
    }

    pub async fn update_rules(&mut self, agent_type: &str, user_id: &str, rules: crate::memory::AgentRules) -> Result<()> {
        self.rules.entry(agent_type.to_string())
            .or_insert_with(HashMap::new)
            .insert(user_id.to_string(), rules);
        Ok(())
    }

    pub async fn get_total_rules(&self) -> usize {
        self.rules.values().map(|agent_rules| agent_rules.len()).sum()
    }
}

impl WorkspaceKnowledgeGraph {
    pub fn new() -> Self {
        Self {
            entities: HashMap::new(),
            relationships: HashMap::new(),
            concepts: HashMap::new(),
            workspace_specific_insights: Vec::new(),
        }
    }
}

impl EnhancedGlobalContext {
    pub fn new() -> Self {
        let now = Utc::now();
        Self {
            user_id: "default_user".to_string(),
            user_preferences: GlobalUserPreferences::default(),
            session_info: SessionInfo::default(),
            workspace_ids: std::collections::HashSet::new(),
            global_memory: GlobalMemory::default(),
            global_knowledge_graph: GlobalKnowledgeGraph::default(),
            system_settings: SystemSettings::default(),
            global_notifications: Vec::new(),
            search_index: CrossWorkspaceSearchIndex::default(),
            insights: CrossWorkspaceInsights::default(),
            created_at: now,
            last_updated: now,
        }
    }
}

// Default implementations
impl Default for GlobalUserPreferences {
    fn default() -> Self {
        Self {
            theme: "dark".to_string(),
            language: "en".to_string(),
            timezone: "UTC".to_string(),
            notification_settings: GlobalNotificationSettings::default(),
            privacy_settings: PrivacySettings::default(),
            accessibility_settings: AccessibilitySettings::default(),
        }
    }
}

impl Default for WorkspaceSettings {
    fn default() -> Self {
        Self {
            auto_save: true,
            git_integration: true,
            ai_assistance_level: AiAssistanceLevel::Normal,
            code_style_preferences: CodeStylePreferences::default(),
            workflow_preferences: WorkflowPreferences::default(),
            notification_preferences: WorkspaceNotificationSettings::default(),
        }
    }
}

// Placeholder types (would be fully implemented)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GlobalNotificationSettings;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PrivacySettings;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AccessibilitySettings;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DeviceInfo;

#[derive(Debug, Clone, Default)]
pub struct UserProfile;

#[derive(Debug, Clone)]
pub struct GlobalBehaviorPattern;

#[derive(Debug, Clone)]
pub struct Insight;

#[derive(Debug, Clone)]
pub struct Entity;

#[derive(Debug, Clone)]
pub struct Concept;

#[derive(Debug, Clone)]
pub struct Relationship;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BackupSettings;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PerformanceSettings;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SecuritySettings;

#[derive(Debug, Clone, Default)]
pub struct SearchableContent;

#[derive(Debug, Clone)]
pub struct SearchQuery;

#[derive(Debug, Clone, Default)]
pub struct ProductivityMetrics {
    pub lines_of_code: u64,
    pub commits_per_day: f64,
    pub files_modified: u64,
    pub build_success_rate: f64,
    pub test_coverage: f64,
}

#[derive(Debug, Clone)]
pub struct UsagePattern;

#[derive(Debug, Clone)]
pub struct Recommendation;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentStatus {
    Active,
    Idle,
    Busy,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentPerformanceMetrics;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AiAssistanceLevel {
    Minimal,
    Normal,
    Aggressive,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CodeStylePreferences;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkflowPreferences;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkspaceNotificationSettings;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitInfo;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bookmark;

impl Default for SessionInfo {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            session_id: Uuid::new_v4().to_string(),
            user_id: "default_user".to_string(),
            started_at: now,
            last_activity: now,
            device_info: DeviceInfo::default(),
            ip_address: None,
        }
    }
}

impl Default for GlobalMemory {
    fn default() -> Self {
        Self {
            user_profile: UserProfile::default(),
            global_patterns: Vec::new(),
            cross_workspace_insights: Vec::new(),
            global_preferences: HashMap::new(),
        }
    }
}

impl Default for GlobalKnowledgeGraph {
    fn default() -> Self {
        Self {
            workspace_relationships: HashMap::new(),
            cross_workspace_entities: HashMap::new(),
            global_concepts: HashMap::new(),
        }
    }
}

impl Default for SystemSettings {
    fn default() -> Self {
        Self {
            auto_save_interval: 30,
            backup_settings: BackupSettings::default(),
            performance_settings: PerformanceSettings::default(),
            security_settings: SecuritySettings::default(),
        }
    }
}

impl Default for CrossWorkspaceSearchIndex {
    fn default() -> Self {
        Self {
            indexed_content: HashMap::new(),
            search_history: Vec::new(),
            popular_searches: Vec::new(),
        }
    }
}

impl Default for CrossWorkspaceInsights {
    fn default() -> Self {
        Self {
            productivity_metrics: ProductivityMetrics::default(),
            usage_patterns: Vec::new(),
            recommendations: Vec::new(),
        }
    }
}
