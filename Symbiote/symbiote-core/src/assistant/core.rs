//! # SYMBIOTE Core Types
//! 
//! Core types and traits for the SYMBIOTE Personal AI Assistant system.

use super::*;
use async_trait::async_trait;

/// Conversation context for maintaining chat history and state
#[derive(Debug, Clone)]
pub struct ConversationContext {
    pub id: String,
    pub messages: Vec<ConversationMessage>,
    pub context_variables: HashMap<String, serde_json::Value>,
    pub active_tasks: Vec<String>,
    pub user_preferences: UserPreferences,
    pub started_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
}

/// Individual message in conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMessage {
    pub id: String,
    pub message_type: MessageType,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Types of messages in conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageType {
    UserMessage,
    SymbioteResponse,
    AgentUpdate,
    SystemNotification,
    Error,
}

/// User preferences for SYMBIOTE behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPreferences {
    pub communication_style: CommunicationStyle,
    pub verbosity_level: VerbosityLevel,
    pub auto_execute_safe_actions: bool,
    pub preferred_agents: Vec<String>,
    pub notification_settings: NotificationSettings,
    pub privacy_settings: PrivacySettings,
}

/// Communication style preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CommunicationStyle {
    Professional,
    Casual,
    Technical,
    Concise,
    Detailed,
}

/// Verbosity level for responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VerbosityLevel {
    Minimal,
    Normal,
    Detailed,
    Verbose,
}

/// Notification settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationSettings {
    pub agent_completion: bool,
    pub agent_errors: bool,
    pub system_updates: bool,
    pub workflow_events: bool,
    pub crypto_alerts: bool,
}

/// Privacy settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacySettings {
    pub store_conversation_history: bool,
    pub share_usage_analytics: bool,
    pub allow_external_api_calls: bool,
    pub encrypt_sensitive_data: bool,
}

/// Message attachments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageAttachment {
    pub attachment_type: AttachmentType,
    pub content: Vec<u8>,
    pub metadata: HashMap<String, String>,
}

/// Types of message attachments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AttachmentType {
    File,
    Image,
    Code,
    Data,
    Link,
}

/// Cursor position in editor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorPosition {
    pub line: u32,
    pub column: u32,
}

/// Text selection in editor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextSelection {
    pub start: CursorPosition,
    pub end: CursorPosition,
    pub text: String,
}

/// Agent action taken by SYMBIOTE
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentAction {
    pub agent_id: String,
    pub agent_type: String,
    pub action: String,
    pub parameters: HashMap<String, serde_json::Value>,
    pub timestamp: DateTime<Utc>,
    pub status: ActionStatus,
}

/// Result of an agent action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionResult {
    pub action_id: String,
    pub agent_id: String,
    pub result_type: ResultType,
    pub data: serde_json::Value,
    pub success: bool,
    pub error_message: Option<String>,
    pub timestamp: DateTime<Utc>,
}

/// Status of an action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActionStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// Type of result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResultType {
    Text,
    File,
    Data,
    Workflow,
    Notification,
    Error,
}

/// Suggestion from SYMBIOTE
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Suggestion {
    pub id: String,
    pub suggestion_type: SuggestionType,
    pub title: String,
    pub description: String,
    pub action: Option<String>,
    pub confidence: f64,
}

/// Types of suggestions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SuggestionType {
    NextAction,
    Optimization,
    Alternative,
    Learning,
    Automation,
}

/// Response status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResponseStatus {
    Success,
    Partial,
    Failed,
    RequiresConfirmation,
    RequiresInput,
}

/// Conversation history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationHistory {
    pub messages: Vec<ConversationMessage>,
    pub summary: String,
    pub key_topics: Vec<String>,
    pub active_context: HashMap<String, serde_json::Value>,
}

impl ConversationContext {
    /// Create new conversation context
    pub fn new() -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            messages: Vec::new(),
            context_variables: HashMap::new(),
            active_tasks: Vec::new(),
            user_preferences: UserPreferences::default(),
            started_at: now,
            last_activity: now,
        }
    }

    /// Add user message to context
    pub fn add_user_message(&mut self, message: UserMessage) {
        let conv_message = ConversationMessage {
            id: message.id,
            message_type: MessageType::UserMessage,
            content: message.content,
            timestamp: message.timestamp,
            metadata: HashMap::new(),
        };
        
        self.messages.push(conv_message);
        self.last_activity = Utc::now();
    }

    /// Add SYMBIOTE response to context
    pub fn add_symbiote_response(&mut self, response: SymbioteResponse) {
        let conv_message = ConversationMessage {
            id: response.id,
            message_type: MessageType::SymbioteResponse,
            content: response.content,
            timestamp: response.timestamp,
            metadata: HashMap::new(),
        };
        
        self.messages.push(conv_message);
        self.last_activity = Utc::now();
    }

    /// Get conversation history
    pub fn get_history(&self) -> ConversationHistory {
        ConversationHistory {
            messages: self.messages.clone(),
            summary: self.generate_summary(),
            key_topics: self.extract_key_topics(),
            active_context: self.context_variables.clone(),
        }
    }

    /// Clear conversation
    pub fn clear(&mut self) {
        self.messages.clear();
        self.context_variables.clear();
        self.active_tasks.clear();
        self.last_activity = Utc::now();
    }

    /// Generate conversation summary
    fn generate_summary(&self) -> String {
        if self.messages.is_empty() {
            return "No conversation yet".to_string();
        }

        let recent_messages: Vec<&ConversationMessage> = self.messages
            .iter()
            .rev()
            .take(10)
            .collect();

        format!("Conversation with {} messages, recent topics: {}", 
                self.messages.len(),
                recent_messages.iter()
                    .map(|m| m.content.chars().take(50).collect::<String>())
                    .collect::<Vec<_>>()
                    .join(", "))
    }

    /// Extract key topics from conversation
    fn extract_key_topics(&self) -> Vec<String> {
        // Simple keyword extraction (would use NLP in practice)
        let mut topics = Vec::new();
        
        for message in &self.messages {
            if message.content.contains("workflow") {
                topics.push("workflows".to_string());
            }
            if message.content.contains("crypto") || message.content.contains("trading") {
                topics.push("crypto trading".to_string());
            }
            if message.content.contains("notebook") {
                topics.push("notebooks".to_string());
            }
            if message.content.contains("code") || message.content.contains("programming") {
                topics.push("programming".to_string());
            }
        }
        
        topics.sort();
        topics.dedup();
        topics
    }
}

impl Default for UserPreferences {
    fn default() -> Self {
        Self {
            communication_style: CommunicationStyle::Professional,
            verbosity_level: VerbosityLevel::Normal,
            auto_execute_safe_actions: true,
            preferred_agents: Vec::new(),
            notification_settings: NotificationSettings::default(),
            privacy_settings: PrivacySettings::default(),
        }
    }
}

impl Default for NotificationSettings {
    fn default() -> Self {
        Self {
            agent_completion: true,
            agent_errors: true,
            system_updates: true,
            workflow_events: true,
            crypto_alerts: true,
        }
    }
}

impl Default for PrivacySettings {
    fn default() -> Self {
        Self {
            store_conversation_history: true,
            share_usage_analytics: false,
            allow_external_api_calls: true,
            encrypt_sensitive_data: true,
        }
    }
}
