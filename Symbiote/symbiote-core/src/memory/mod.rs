//! # Memory System
//! 
//! Comprehensive memory system for SYMBIOTE Personal AI Assistant
//! Similar to mem0 - provides persistent memory for users, conversations,
//! preferences, and context across sessions.

pub mod core;
pub mod storage;
pub mod retrieval;
pub mod compression;
pub mod rules;

// Re-export main types
pub use core::*;
pub use storage::*;
pub use retrieval::*;
pub use compression::*;
pub use rules::*;

use crate::{Result, SymbioteError};
use crate::context::ContextBus;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Main memory system for SYMBIOTE
/// 
/// Provides persistent memory capabilities similar to mem0:
/// - User memory and preferences
/// - Conversation history and context
/// - Agent behavior rules and customizations
/// - Cross-session knowledge retention
#[derive(Debug)]
pub struct MemorySystem {
    /// Core memory engine
    memory_engine: MemoryEngine,
    
    /// Storage backend for persistent memory
    storage: MemoryStorage,
    
    /// Retrieval system for finding relevant memories
    retrieval: MemoryRetrieval,
    
    /// Compression for managing memory size
    compression: MemoryCompression,
    
    /// Agent rule management
    agent_rules: AgentRuleManager,
    
    /// Integration with context system
    context_bus: Option<Arc<ContextBus>>,
}

impl MemorySystem {
    /// Create new memory system
    pub fn new() -> Self {
        Self {
            memory_engine: MemoryEngine::new(),
            storage: MemoryStorage::new(),
            retrieval: MemoryRetrieval::new(),
            compression: MemoryCompression::new(),
            agent_rules: AgentRuleManager::new(),
            context_bus: None,
        }
    }

    /// Create memory system with context integration
    pub fn with_context_bus(context_bus: Arc<ContextBus>) -> Self {
        let mut system = Self::new();
        system.context_bus = Some(context_bus);
        system
    }

    /// Store a memory
    pub async fn store_memory(&self, memory: Memory) -> Result<String> {
        // Store in memory engine
        let memory_id = self.memory_engine.store(memory.clone()).await?;
        
        // Persist to storage
        self.storage.save_memory(&memory_id, &memory).await?;
        
        // Update retrieval index
        self.retrieval.index_memory(&memory_id, &memory).await?;
        
        // Integrate with context if available
        if let Some(context_bus) = &self.context_bus {
            self.update_context_with_memory(context_bus, &memory).await?;
        }
        
        Ok(memory_id)
    }

    /// Retrieve memories by query
    pub async fn retrieve_memories(&self, query: &MemoryQuery) -> Result<Vec<Memory>> {
        self.retrieval.search(query).await
    }

    /// Get user memory profile
    pub async fn get_user_memory(&self, user_id: &str) -> Result<UserMemory> {
        self.memory_engine.get_user_memory(user_id).await
    }

    /// Update user preferences
    pub async fn update_user_preferences(&self, user_id: &str, preferences: UserPreferences) -> Result<()> {
        self.memory_engine.update_user_preferences(user_id, preferences).await
    }

    /// Store conversation memory
    pub async fn store_conversation(&self, conversation: ConversationMemory) -> Result<()> {
        let memory = Memory {
            id: Uuid::new_v4().to_string(),
            memory_type: MemoryType::Conversation,
            content: serde_json::to_value(&conversation)?,
            user_id: conversation.user_id.clone(),
            timestamp: Utc::now(),
            importance: self.calculate_conversation_importance(&conversation).await?,
            tags: conversation.extract_tags(),
            metadata: HashMap::new(),
        };
        
        self.store_memory(memory).await?;
        Ok(())
    }

    /// Get agent rules for specific agent
    pub async fn get_agent_rules(&self, workspace_id: &str, agent_type: &str, user_id: &str) -> Result<AgentRules> {
        self.agent_rules.get_rules(workspace_id, agent_type, user_id).await
    }

    /// Update agent rules
    pub async fn update_agent_rules(&self, workspace_id: &str, agent_type: &str, user_id: &str, rules: AgentRules) -> Result<()> {
        self.agent_rules.update_rules(workspace_id, agent_type, user_id, rules).await
    }

    /// Get system prompt for agent with rules applied
    pub async fn get_agent_system_prompt(&self, workspace_id: &str, agent_type: &str, user_id: &str, base_prompt: &str) -> Result<String> {
        let rules = self.get_agent_rules(workspace_id, agent_type, user_id).await?;
        Ok(rules.apply_to_prompt(base_prompt))
    }

    /// Compress old memories
    pub async fn compress_memories(&self) -> Result<()> {
        let old_memories = self.memory_engine.get_old_memories().await?;
        let compressed = self.compression.compress_memories(&old_memories).await?;
        
        for (memory_id, compressed_memory) in compressed {
            self.storage.save_compressed_memory(&memory_id, &compressed_memory).await?;
        }
        
        Ok(())
    }

    /// Search memories with semantic similarity
    pub async fn semantic_search(&self, query: &str, user_id: &str, limit: usize) -> Result<Vec<Memory>> {
        let query = MemoryQuery {
            text: Some(query.to_string()),
            user_id: Some(user_id.to_string()),
            memory_types: None,
            time_range: None,
            importance_threshold: None,
            limit: Some(limit),
        };
        
        self.retrieve_memories(&query).await
    }

    /// Get relevant context for current conversation
    pub async fn get_conversation_context(&self, user_id: &str, current_message: &str) -> Result<ConversationContext> {
        // Get recent conversation history
        let recent_conversations = self.semantic_search(current_message, user_id, 5).await?;
        
        // Get user preferences and memory
        let user_memory = self.get_user_memory(user_id).await?;
        
        // Build context
        let preferences = user_memory.preferences.clone();
        Ok(ConversationContext {
            user_memory,
            recent_conversations,
            relevant_facts: self.extract_relevant_facts(current_message, user_id).await?,
            preferences,
        })
    }

    /// Learn from user interaction
    pub async fn learn_from_interaction(&self, interaction: UserInteraction) -> Result<()> {
        // Extract learnings from interaction
        let learnings = self.extract_learnings(&interaction).await?;
        
        // Store as memories
        for learning in learnings {
            self.store_memory(learning).await?;
        }
        
        // Update user preferences if needed
        if let Some(preference_updates) = self.extract_preference_updates(&interaction).await? {
            self.update_user_preferences(&interaction.user_id, preference_updates).await?;
        }
        
        Ok(())
    }

    /// Helper methods
    async fn calculate_conversation_importance(&self, conversation: &ConversationMemory) -> Result<f64> {
        // Calculate importance based on various factors
        let mut importance = 0.5; // Base importance
        
        // Increase importance for longer conversations
        importance += (conversation.messages.len() as f64 * 0.1).min(0.3);
        
        // Increase importance for conversations with specific keywords
        let important_keywords = ["error", "problem", "help", "important", "urgent"];
        for message in &conversation.messages {
            for keyword in important_keywords {
                if message.content.to_lowercase().contains(keyword) {
                    importance += 0.1;
                }
            }
        }
        
        Ok(importance.min(1.0))
    }

    async fn update_context_with_memory(&self, context_bus: &ContextBus, memory: &Memory) -> Result<()> {
        // Would integrate with context bus to update global context
        Ok(())
    }

    async fn extract_relevant_facts(&self, message: &str, user_id: &str) -> Result<Vec<String>> {
        // Extract facts relevant to current message
        let facts_query = MemoryQuery {
            text: Some(message.to_string()),
            user_id: Some(user_id.to_string()),
            memory_types: Some(vec![MemoryType::Fact, MemoryType::Preference]),
            time_range: None,
            importance_threshold: Some(0.7),
            limit: Some(10),
        };
        
        let memories = self.retrieve_memories(&facts_query).await?;
        Ok(memories.into_iter()
            .filter_map(|m| m.content.as_str().map(String::from))
            .collect())
    }

    async fn extract_learnings(&self, interaction: &UserInteraction) -> Result<Vec<Memory>> {
        let mut learnings = Vec::new();
        
        // Extract preferences
        if interaction.message.contains("I prefer") || interaction.message.contains("I like") {
            learnings.push(Memory {
                id: Uuid::new_v4().to_string(),
                memory_type: MemoryType::Preference,
                content: serde_json::json!(interaction.message),
                user_id: interaction.user_id.clone(),
                timestamp: interaction.timestamp,
                importance: 0.8,
                tags: vec!["preference".to_string()],
                metadata: HashMap::new(),
            });
        }
        
        // Extract facts
        if interaction.message.contains("My name is") || interaction.message.contains("I work at") {
            learnings.push(Memory {
                id: Uuid::new_v4().to_string(),
                memory_type: MemoryType::Fact,
                content: serde_json::json!(interaction.message),
                user_id: interaction.user_id.clone(),
                timestamp: interaction.timestamp,
                importance: 0.9,
                tags: vec!["fact".to_string(), "personal".to_string()],
                metadata: HashMap::new(),
            });
        }
        
        Ok(learnings)
    }

    async fn extract_preference_updates(&self, interaction: &UserInteraction) -> Result<Option<UserPreferences>> {
        // Extract preference updates from interaction
        // This would use NLP to understand preference changes
        Ok(None) // Simplified for now
    }
}

/// User interaction for learning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInteraction {
    pub user_id: String,
    pub message: String,
    pub response: String,
    pub timestamp: DateTime<Utc>,
    pub context: HashMap<String, serde_json::Value>,
}

/// Conversation context for AI responses
#[derive(Debug, Clone)]
pub struct ConversationContext {
    pub user_memory: UserMemory,
    pub recent_conversations: Vec<Memory>,
    pub relevant_facts: Vec<String>,
    pub preferences: UserPreferences,
}

impl Default for MemorySystem {
    fn default() -> Self {
        Self::new()
    }
}

/// Workspace-specific memory system
#[derive(Debug, Clone)]
pub struct WorkspaceMemory {
    /// Workspace ID
    workspace_id: String,

    /// Workspace-specific memories
    memories: HashMap<String, Memory>,

    /// Workspace context
    context: HashMap<String, serde_json::Value>,

    /// Last updated timestamp
    last_updated: DateTime<Utc>,
}

impl WorkspaceMemory {
    pub fn new() -> Self {
        Self {
            workspace_id: "default".to_string(),
            memories: HashMap::new(),
            context: HashMap::new(),
            last_updated: Utc::now(),
        }
    }

    pub async fn get_size(&self) -> u64 {
        // Calculate approximate memory size
        self.memories.len() as u64 * 1024 // Rough estimate
    }
}

impl Default for WorkspaceMemory {
    fn default() -> Self {
        Self::new()
    }
}
