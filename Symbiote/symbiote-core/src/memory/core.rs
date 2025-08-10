//! # Memory Core Types
//! 
//! Core memory types and structures for the memory system.

use super::*;

/// Core memory engine
#[derive(Debug)]
pub struct MemoryEngine {
    /// In-memory storage for active memories
    active_memories: Arc<RwLock<HashMap<String, Memory>>>,
    
    /// User memory profiles
    user_memories: Arc<RwLock<HashMap<String, UserMemory>>>,
    
    /// Memory statistics
    stats: Arc<RwLock<MemoryStats>>,
}

impl MemoryEngine {
    pub fn new() -> Self {
        Self {
            active_memories: Arc::new(RwLock::new(HashMap::new())),
            user_memories: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(MemoryStats::default())),
        }
    }

    /// Store a memory
    pub async fn store(&self, memory: Memory) -> Result<String> {
        let memory_id = memory.id.clone();
        
        // Store in active memories
        {
            let mut active = self.active_memories.write().await;
            active.insert(memory_id.clone(), memory.clone());
        }
        
        // Update user memory profile
        {
            let mut user_memories = self.user_memories.write().await;
            let user_memory = user_memories.entry(memory.user_id.clone())
                .or_insert_with(|| UserMemory::new(memory.user_id.clone()));
            
            user_memory.add_memory(&memory);
        }
        
        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.total_memories += 1;
            stats.memories_by_type.entry(memory.memory_type.clone())
                .and_modify(|count| *count += 1)
                .or_insert(1);
        }
        
        Ok(memory_id)
    }

    /// Get user memory profile
    pub async fn get_user_memory(&self, user_id: &str) -> Result<UserMemory> {
        let user_memories = self.user_memories.read().await;
        Ok(user_memories.get(user_id)
            .cloned()
            .unwrap_or_else(|| UserMemory::new(user_id.to_string())))
    }

    /// Update user preferences
    pub async fn update_user_preferences(&self, user_id: &str, preferences: UserPreferences) -> Result<()> {
        let mut user_memories = self.user_memories.write().await;
        let user_memory = user_memories.entry(user_id.to_string())
            .or_insert_with(|| UserMemory::new(user_id.to_string()));
        
        user_memory.preferences = preferences;
        user_memory.updated_at = Utc::now();
        
        Ok(())
    }

    /// Get old memories for compression
    pub async fn get_old_memories(&self) -> Result<Vec<Memory>> {
        let active = self.active_memories.read().await;
        let cutoff = Utc::now() - chrono::Duration::days(30); // 30 days old
        
        Ok(active.values()
            .filter(|m| m.timestamp < cutoff && m.importance < 0.5)
            .cloned()
            .collect())
    }

    /// Get memory statistics
    pub async fn get_stats(&self) -> MemoryStats {
        let stats = self.stats.read().await;
        stats.clone()
    }
}

/// Individual memory item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memory {
    pub id: String,
    pub memory_type: MemoryType,
    pub content: serde_json::Value,
    pub user_id: String,
    pub timestamp: DateTime<Utc>,
    pub importance: f64, // 0.0 to 1.0
    pub tags: Vec<String>,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Types of memories
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum MemoryType {
    /// User conversation
    Conversation,
    /// User preference
    Preference,
    /// Factual information
    Fact,
    /// Task or goal
    Task,
    /// Agent interaction
    AgentInteraction,
    /// System event
    SystemEvent,
    /// User behavior pattern
    BehaviorPattern,
    /// Knowledge or learning
    Knowledge,
}

/// User memory profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserMemory {
    pub user_id: String,
    pub preferences: UserPreferences,
    pub personality_profile: PersonalityProfile,
    pub interaction_history: InteractionHistory,
    pub learned_patterns: Vec<BehaviorPattern>,
    pub important_facts: Vec<String>,
    pub goals: Vec<UserGoal>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// User preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPreferences {
    /// Communication style
    pub communication_style: String,
    
    /// Preferred response length
    pub response_length: String,
    
    /// Notification preferences
    pub notifications: NotificationPreferences,
    
    /// Privacy settings
    pub privacy: PrivacySettings,
    
    /// Agent preferences
    pub agent_preferences: HashMap<String, AgentPreference>,
    
    /// Custom settings
    pub custom_settings: HashMap<String, serde_json::Value>,
}

/// Notification preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationPreferences {
    pub email_notifications: bool,
    pub push_notifications: bool,
    pub agent_completion_alerts: bool,
    pub error_alerts: bool,
    pub daily_summary: bool,
}

/// Privacy settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacySettings {
    pub store_conversations: bool,
    pub share_analytics: bool,
    pub allow_learning: bool,
    pub data_retention_days: u32,
}

/// Agent preference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPreference {
    pub preferred_agents: Vec<String>,
    pub blocked_agents: Vec<String>,
    pub agent_settings: HashMap<String, serde_json::Value>,
}

/// Personality profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalityProfile {
    pub traits: HashMap<String, f64>,
    pub communication_patterns: Vec<String>,
    pub preferred_interaction_style: String,
    pub expertise_areas: Vec<String>,
    pub interests: Vec<String>,
}

/// Interaction history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractionHistory {
    pub total_interactions: u64,
    pub successful_interactions: u64,
    pub failed_interactions: u64,
    pub average_session_length: u64,
    pub most_used_agents: Vec<String>,
    pub common_tasks: Vec<String>,
}

/// Behavior pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorPattern {
    pub pattern_type: String,
    pub description: String,
    pub frequency: f64,
    pub confidence: f64,
    pub examples: Vec<String>,
}

/// User goal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserGoal {
    pub id: String,
    pub title: String,
    pub description: String,
    pub status: GoalStatus,
    pub priority: u32,
    pub created_at: DateTime<Utc>,
    pub target_date: Option<DateTime<Utc>>,
    pub progress: f64,
}

/// Goal status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GoalStatus {
    Active,
    Completed,
    Paused,
    Cancelled,
}

/// Conversation memory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMemory {
    pub conversation_id: String,
    pub user_id: String,
    pub messages: Vec<ConversationMessage>,
    pub summary: String,
    pub topics: Vec<String>,
    pub sentiment: f64,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
}

/// Individual conversation message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMessage {
    pub id: String,
    pub role: MessageRole,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Message role
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageRole {
    User,
    Assistant,
    System,
    Agent(String),
}

/// Memory query for retrieval
#[derive(Debug, Clone)]
pub struct MemoryQuery {
    pub text: Option<String>,
    pub user_id: Option<String>,
    pub memory_types: Option<Vec<MemoryType>>,
    pub time_range: Option<(DateTime<Utc>, DateTime<Utc>)>,
    pub importance_threshold: Option<f64>,
    pub limit: Option<usize>,
}

/// Memory statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemoryStats {
    pub total_memories: u64,
    pub memories_by_type: HashMap<MemoryType, u64>,
    pub total_users: u64,
    pub average_memories_per_user: f64,
    pub storage_size_bytes: u64,
}

impl UserMemory {
    pub fn new(user_id: String) -> Self {
        let now = Utc::now();
        Self {
            user_id,
            preferences: UserPreferences::default(),
            personality_profile: PersonalityProfile::default(),
            interaction_history: InteractionHistory::default(),
            learned_patterns: Vec::new(),
            important_facts: Vec::new(),
            goals: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }

    pub fn add_memory(&mut self, memory: &Memory) {
        // Update interaction history
        self.interaction_history.total_interactions += 1;
        
        // Extract important facts
        if memory.importance > 0.8 {
            if let Some(content_str) = memory.content.as_str() {
                self.important_facts.push(content_str.to_string());
            }
        }
        
        self.updated_at = Utc::now();
    }
}

impl ConversationMemory {
    pub fn extract_tags(&self) -> Vec<String> {
        let mut tags = Vec::new();
        
        // Add topic tags
        tags.extend(self.topics.clone());
        
        // Add sentiment tag
        if self.sentiment > 0.5 {
            tags.push("positive".to_string());
        } else if self.sentiment < -0.5 {
            tags.push("negative".to_string());
        } else {
            tags.push("neutral".to_string());
        }
        
        // Add length tag
        if self.messages.len() > 10 {
            tags.push("long_conversation".to_string());
        } else {
            tags.push("short_conversation".to_string());
        }
        
        tags
    }
}

impl Default for UserPreferences {
    fn default() -> Self {
        Self {
            communication_style: "professional".to_string(),
            response_length: "normal".to_string(),
            notifications: NotificationPreferences::default(),
            privacy: PrivacySettings::default(),
            agent_preferences: HashMap::new(),
            custom_settings: HashMap::new(),
        }
    }
}

impl Default for NotificationPreferences {
    fn default() -> Self {
        Self {
            email_notifications: true,
            push_notifications: true,
            agent_completion_alerts: true,
            error_alerts: true,
            daily_summary: false,
        }
    }
}

impl Default for PrivacySettings {
    fn default() -> Self {
        Self {
            store_conversations: true,
            share_analytics: false,
            allow_learning: true,
            data_retention_days: 365,
        }
    }
}

impl Default for PersonalityProfile {
    fn default() -> Self {
        Self {
            traits: HashMap::new(),
            communication_patterns: Vec::new(),
            preferred_interaction_style: "collaborative".to_string(),
            expertise_areas: Vec::new(),
            interests: Vec::new(),
        }
    }
}

impl Default for InteractionHistory {
    fn default() -> Self {
        Self {
            total_interactions: 0,
            successful_interactions: 0,
            failed_interactions: 0,
            average_session_length: 0,
            most_used_agents: Vec::new(),
            common_tasks: Vec::new(),
        }
    }
}
