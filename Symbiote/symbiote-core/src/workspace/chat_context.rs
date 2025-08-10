//! # Chat Context Manager
//! 
//! Manages chat context switching between global and workspace-specific modes.

use super::*;

/// Chat context manager for workspace-aware conversations
#[derive(Debug, Clone)]
pub struct ChatContextManager {
    /// Current chat mode
    mode: Arc<RwLock<ChatMode>>,
    
    /// Chat history per context
    chat_history: Arc<RwLock<HashMap<String, Vec<ChatMessage>>>>,
    
    /// Context-specific settings
    context_settings: Arc<RwLock<HashMap<String, ChatContextSettings>>>,
}

/// Chat mode - global or workspace-specific
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChatMode {
    /// Global mode - can see and operate across all workspaces
    Global {
        /// Whether to show cross-workspace insights
        show_cross_workspace: bool,
        /// Active workspace for context (if any)
        context_workspace: Option<String>,
    },
    /// Workspace mode - locked to specific workspace
    Workspace {
        /// Workspace ID this chat is locked to
        workspace_id: String,
        /// Whether to allow global operations
        allow_global_operations: bool,
    },
}

/// Individual chat message with context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: String,
    pub content: String,
    pub role: ChatRole,
    pub timestamp: DateTime<Utc>,
    pub context: ChatMessageContext,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Chat message role
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChatRole {
    User,
    Assistant,
    System,
    Agent(String),
}

/// Context information for chat message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessageContext {
    /// Mode when message was sent
    pub chat_mode: ChatMode,
    /// Workspace context (if any)
    pub workspace_id: Option<String>,
    /// Files that were open
    pub open_files: Vec<String>,
    /// Active agents
    pub active_agents: Vec<String>,
    /// User's current panel
    pub current_panel: Option<String>,
}

/// Chat context settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatContextSettings {
    /// Auto-switch to workspace mode when workspace is selected
    pub auto_switch_workspace: bool,
    /// Show workspace indicator in chat
    pub show_workspace_indicator: bool,
    /// Include cross-workspace suggestions
    pub cross_workspace_suggestions: bool,
    /// Maximum chat history per context
    pub max_history_per_context: usize,
}

impl ChatContextManager {
    /// Create new chat context manager
    pub fn new() -> Self {
        Self {
            mode: Arc::new(RwLock::new(ChatMode::Global {
                show_cross_workspace: true,
                context_workspace: None,
            })),
            chat_history: Arc::new(RwLock::new(HashMap::new())),
            context_settings: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get current chat mode
    pub async fn get_mode(&self) -> ChatMode {
        let mode = self.mode.read().await;
        mode.clone()
    }

    /// Switch to global mode
    pub async fn switch_to_global(&self, show_cross_workspace: bool) -> Result<()> {
        let mut mode = self.mode.write().await;
        *mode = ChatMode::Global {
            show_cross_workspace,
            context_workspace: None,
        };
        Ok(())
    }

    /// Switch to workspace mode
    pub async fn switch_to_workspace(&self, workspace_id: String, allow_global_operations: bool) -> Result<()> {
        let mut mode = self.mode.write().await;
        *mode = ChatMode::Workspace {
            workspace_id,
            allow_global_operations,
        };
        Ok(())
    }

    /// Update workspace context (for global mode)
    pub async fn update_workspace_context(&self, workspace_id: Option<String>) -> Result<()> {
        let mut mode = self.mode.write().await;
        if let ChatMode::Global { show_cross_workspace, .. } = &*mode {
            *mode = ChatMode::Global {
                show_cross_workspace: *show_cross_workspace,
                context_workspace: workspace_id,
            };
        }
        Ok(())
    }

    /// Add message to chat history
    pub async fn add_message(&self, message: ChatMessage) -> Result<()> {
        let context_key = self.get_context_key(&message.context.chat_mode).await;
        
        let mut history = self.chat_history.write().await;
        let messages = history.entry(context_key).or_insert_with(Vec::new);
        
        let chat_mode = message.context.chat_mode.clone();
        messages.push(message);

        // Limit history size
        let max_history = self.get_max_history_for_context(&chat_mode).await;
        if messages.len() > max_history {
            messages.drain(0..messages.len() - max_history);
        }
        
        Ok(())
    }

    /// Get chat history for current context
    pub async fn get_current_history(&self) -> Vec<ChatMessage> {
        let mode = self.get_mode().await;
        let context_key = self.get_context_key(&mode).await;
        
        let history = self.chat_history.read().await;
        history.get(&context_key).cloned().unwrap_or_default()
    }

    /// Get chat history for specific context
    pub async fn get_history_for_context(&self, mode: &ChatMode) -> Vec<ChatMessage> {
        let context_key = self.get_context_key(mode).await;
        
        let history = self.chat_history.read().await;
        history.get(&context_key).cloned().unwrap_or_default()
    }

    /// Clear chat history for current context
    pub async fn clear_current_history(&self) -> Result<()> {
        let mode = self.get_mode().await;
        let context_key = self.get_context_key(&mode).await;
        
        let mut history = self.chat_history.write().await;
        history.remove(&context_key);
        
        Ok(())
    }

    /// Get context settings
    pub async fn get_context_settings(&self, context_key: &str) -> ChatContextSettings {
        let settings = self.context_settings.read().await;
        settings.get(context_key)
            .cloned()
            .unwrap_or_else(ChatContextSettings::default)
    }

    /// Update context settings
    pub async fn update_context_settings(&self, context_key: String, settings: ChatContextSettings) -> Result<()> {
        let mut context_settings = self.context_settings.write().await;
        context_settings.insert(context_key, settings);
        Ok(())
    }

    /// Check if current mode allows global operations
    pub async fn allows_global_operations(&self) -> bool {
        let mode = self.get_mode().await;
        match mode {
            ChatMode::Global { .. } => true,
            ChatMode::Workspace { allow_global_operations, .. } => allow_global_operations,
        }
    }

    /// Get workspace ID for current context (if any)
    pub async fn get_current_workspace_id(&self) -> Option<String> {
        let mode = self.get_mode().await;
        match mode {
            ChatMode::Global { context_workspace, .. } => context_workspace,
            ChatMode::Workspace { workspace_id, .. } => Some(workspace_id),
        }
    }

    /// Create chat message context from current state
    pub async fn create_message_context(&self, workspace_id: Option<String>, open_files: Vec<String>, active_agents: Vec<String>, current_panel: Option<String>) -> ChatMessageContext {
        let mode = self.get_mode().await;
        
        ChatMessageContext {
            chat_mode: mode,
            workspace_id,
            open_files,
            active_agents,
            current_panel,
        }
    }

    /// Get context key for chat mode
    async fn get_context_key(&self, mode: &ChatMode) -> String {
        match mode {
            ChatMode::Global { context_workspace, .. } => {
                if let Some(workspace_id) = context_workspace {
                    format!("global_with_context:{}", workspace_id)
                } else {
                    "global".to_string()
                }
            },
            ChatMode::Workspace { workspace_id, .. } => {
                format!("workspace:{}", workspace_id)
            },
        }
    }

    /// Get maximum history for context
    async fn get_max_history_for_context(&self, mode: &ChatMode) -> usize {
        let context_key = self.get_context_key(mode).await;
        let settings = self.get_context_settings(&context_key).await;
        settings.max_history_per_context
    }
}

impl ChatMessage {
    /// Create new user message
    pub fn new_user(content: String, context: ChatMessageContext) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            content,
            role: ChatRole::User,
            timestamp: Utc::now(),
            context,
            metadata: HashMap::new(),
        }
    }

    /// Create new assistant message
    pub fn new_assistant(content: String, context: ChatMessageContext) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            content,
            role: ChatRole::Assistant,
            timestamp: Utc::now(),
            context,
            metadata: HashMap::new(),
        }
    }

    /// Create new agent message
    pub fn new_agent(agent_type: String, content: String, context: ChatMessageContext) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            content,
            role: ChatRole::Agent(agent_type),
            timestamp: Utc::now(),
            context,
            metadata: HashMap::new(),
        }
    }
}

impl Default for ChatContextSettings {
    fn default() -> Self {
        Self {
            auto_switch_workspace: true,
            show_workspace_indicator: true,
            cross_workspace_suggestions: true,
            max_history_per_context: 1000,
        }
    }
}

impl Default for ChatContextManager {
    fn default() -> Self {
        Self::new()
    }
}
