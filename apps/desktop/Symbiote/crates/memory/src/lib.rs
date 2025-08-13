//! # Symbiote Memory
//!
//! Intelligent memory system that enables persistent learning and context retention. Features:
//! - Working Memory: Short-term context for active conversations and tasks
//! - Episodic Memory: Long-term storage of experiences and interactions
//! - Semantic Memory: Structured knowledge and learned concepts
//! - Procedural Memory: Learned skills and behavioral patterns
//! - Memory Consolidation: Intelligent transfer between memory types
//! - Forgetting Mechanisms: Selective forgetting and memory optimization
//! - Cross-Session Persistence: Memory that persists across application restarts

#![deny(missing_docs)]
#![warn(clippy::all)]

// Core modules
pub mod manager;
pub mod working;
pub mod episodic;
pub mod semantic;
pub mod procedural;
pub mod consolidation;
pub mod forgetting;
pub mod retrieval;
pub mod learning;
pub mod types;

// Re-export main types
pub use manager::{MemoryManager, MemoryCoordinator, MemoryStatistics};
pub use working::{WorkingMemory, WorkingMemoryItem, AttentionFocus, WorkingContext};
pub use episodic::{EpisodicMemory, Episode, Experience, Timeline, EpisodicAssociation};
pub use semantic::{SemanticMemory, Concept, ConceptHierarchy, SemanticRelationship, KnowledgeGraph};
pub use procedural::{ProceduralMemory, Skill, BehavioralPattern, Habit, SkillLearning};
pub use consolidation::{ConsolidationEngine, ConsolidationReport, MemoryConsolidator};
pub use forgetting::{ForgettingEngine, ForgettingPolicy, MemoryDecay, SelectiveForgetting};
pub use retrieval::{RetrievalEngine, MemoryQuery, MemoryRetrieval, RelevanceRanker};
pub use learning::{LearningEngine, PatternRecognition, AdaptiveLearning, LearningMetrics};
pub use types::{MemoryConfig, MemoryResult, MemoryError, MemoryId, ConceptId, SkillId};

// Core dependencies
use symbiote_core::{SymbioteResult, SymbioteError, Service};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Main memory system that orchestrates all memory operations
pub struct MemorySystem {
    /// Memory manager and coordinator
    pub manager: Arc<MemoryManager>,
    
    /// Working memory for short-term context
    pub working_memory: Arc<WorkingMemory>,
    
    /// Episodic memory for experiences
    pub episodic_memory: Arc<EpisodicMemory>,
    
    /// Semantic memory for knowledge
    pub semantic_memory: Arc<SemanticMemory>,
    
    /// Procedural memory for skills
    pub procedural_memory: Arc<ProceduralMemory>,
    
    /// Memory consolidation engine
    pub consolidation: Arc<ConsolidationEngine>,
    
    /// Forgetting and optimization engine
    pub forgetting: Arc<ForgettingEngine>,
    
    /// Memory retrieval system
    pub retrieval: Arc<RetrievalEngine>,
    
    /// Learning and adaptation engine
    pub learning: Arc<LearningEngine>,
    
    /// Configuration
    config: Arc<RwLock<MemoryConfig>>,
}

/// Configuration for the memory system
#[derive(Debug, Clone)]
pub struct MemoryConfig {
    /// Enable memory system
    pub enabled: bool,
    
    /// Enable cross-session persistence
    pub enable_persistence: bool,
    
    /// Enable memory consolidation
    pub enable_consolidation: bool,
    
    /// Enable forgetting mechanisms
    pub enable_forgetting: bool,
    
    /// Database connection string
    pub database_url: String,
    
    /// Working memory configuration
    pub working_memory: WorkingMemoryConfig,
    
    /// Episodic memory configuration
    pub episodic_memory: EpisodicMemoryConfig,
    
    /// Semantic memory configuration
    pub semantic_memory: SemanticMemoryConfig,
    
    /// Procedural memory configuration
    pub procedural_memory: ProceduralMemoryConfig,
}

/// Working memory configuration
#[derive(Debug, Clone)]
pub struct WorkingMemoryConfig {
    /// Buffer size in MB
    pub buffer_size_mb: usize,
    
    /// Context window size in tokens
    pub context_window_tokens: usize,
    
    /// Attention span in minutes
    pub attention_span_minutes: u32,
    
    /// Decay rate
    pub decay_rate: f32,
    
    /// Recency weight (0.0 to 1.0)
    pub recency_weight: f32,
    
    /// Importance weight (0.0 to 1.0)
    pub importance_weight: f32,
}

/// Episodic memory configuration
#[derive(Debug, Clone)]
pub struct EpisodicMemoryConfig {
    /// Maximum episodes to store
    pub max_episodes: usize,
    
    /// Episode retention period in days
    pub retention_days: u32,
    
    /// Enable automatic compression
    pub enable_compression: bool,
    
    /// Compression threshold in days
    pub compression_threshold_days: u32,
}

/// Semantic memory configuration
#[derive(Debug, Clone)]
pub struct SemanticMemoryConfig {
    /// Maximum concepts to store
    pub max_concepts: usize,
    
    /// Enable concept learning
    pub enable_learning: bool,
    
    /// Concept similarity threshold
    pub similarity_threshold: f32,
    
    /// Enable knowledge graph
    pub enable_knowledge_graph: bool,
}

/// Procedural memory configuration
#[derive(Debug, Clone)]
pub struct ProceduralMemoryConfig {
    /// Maximum skills to store
    pub max_skills: usize,
    
    /// Enable skill learning
    pub enable_learning: bool,
    
    /// Skill adaptation rate
    pub adaptation_rate: f32,
    
    /// Enable habit formation
    pub enable_habits: bool,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            enable_persistence: true,
            enable_consolidation: true,
            enable_forgetting: true,
            database_url: "sqlite:memory.db".to_string(),
            working_memory: WorkingMemoryConfig {
                buffer_size_mb: 512,
                context_window_tokens: 32768,
                attention_span_minutes: 15,
                decay_rate: 0.1,
                recency_weight: 0.4,
                importance_weight: 0.6,
            },
            episodic_memory: EpisodicMemoryConfig {
                max_episodes: 100000,
                retention_days: 365,
                enable_compression: true,
                compression_threshold_days: 30,
            },
            semantic_memory: SemanticMemoryConfig {
                max_concepts: 50000,
                enable_learning: true,
                similarity_threshold: 0.8,
                enable_knowledge_graph: true,
            },
            procedural_memory: ProceduralMemoryConfig {
                max_skills: 10000,
                enable_learning: true,
                adaptation_rate: 0.05,
                enable_habits: true,
            },
        }
    }
}

impl MemorySystem {
    /// Create a new memory system instance
    pub async fn new(config: MemoryConfig) -> SymbioteResult<Self> {
        let config = Arc::new(RwLock::new(config));
        
        // Initialize core components
        let working_memory = Arc::new(WorkingMemory::new(config.clone()).await?);
        let episodic_memory = Arc::new(EpisodicMemory::new(config.clone()).await?);
        let semantic_memory = Arc::new(SemanticMemory::new(config.clone()).await?);
        let procedural_memory = Arc::new(ProceduralMemory::new(config.clone()).await?);
        let consolidation = Arc::new(ConsolidationEngine::new(config.clone()).await?);
        let forgetting = Arc::new(ForgettingEngine::new(config.clone()).await?);
        let retrieval = Arc::new(RetrievalEngine::new(config.clone()).await?);
        let learning = Arc::new(LearningEngine::new(config.clone()).await?);
        let manager = Arc::new(MemoryManager::new(
            config.clone(),
            working_memory.clone(),
            episodic_memory.clone(),
            semantic_memory.clone(),
            procedural_memory.clone(),
        ).await?);
        
        Ok(Self {
            manager,
            working_memory,
            episodic_memory,
            semantic_memory,
            procedural_memory,
            consolidation,
            forgetting,
            retrieval,
            learning,
            config,
        })
    }
    
    /// Start the memory system
    pub async fn start(&self) -> SymbioteResult<()> {
        tracing::info!("Starting Symbiote Memory System");
        
        // Initialize all components
        self.working_memory.initialize().await?;
        self.episodic_memory.initialize().await?;
        self.semantic_memory.initialize().await?;
        self.procedural_memory.initialize().await?;
        self.consolidation.initialize().await?;
        self.forgetting.initialize().await?;
        self.retrieval.initialize().await?;
        self.learning.initialize().await?;
        self.manager.initialize().await?;
        
        tracing::info!("Symbiote Memory System started successfully");
        Ok(())
    }
    
    /// Stop the memory system
    pub async fn stop(&self) -> SymbioteResult<()> {
        tracing::info!("Stopping Symbiote Memory System");
        
        // Shutdown all components in reverse order
        self.manager.shutdown().await?;
        self.learning.shutdown().await?;
        self.retrieval.shutdown().await?;
        self.forgetting.shutdown().await?;
        self.consolidation.shutdown().await?;
        self.procedural_memory.shutdown().await?;
        self.semantic_memory.shutdown().await?;
        self.episodic_memory.shutdown().await?;
        self.working_memory.shutdown().await?;
        
        tracing::info!("Symbiote Memory System stopped successfully");
        Ok(())
    }
    
    /// Store a new experience
    pub async fn store_experience(&self, experience: Experience) -> SymbioteResult<MemoryId> {
        self.manager.store_experience(experience).await
    }
    
    /// Recall memories based on query
    pub async fn recall(&self, query: MemoryQuery) -> SymbioteResult<Vec<Memory>> {
        self.retrieval.recall(query).await
    }
    
    /// Learn a new concept
    pub async fn learn_concept(&self, concept: Concept) -> SymbioteResult<ConceptId> {
        self.semantic_memory.learn_concept(concept).await
    }
    
    /// Learn a new skill
    pub async fn learn_skill(&self, skill: Skill) -> SymbioteResult<SkillId> {
        self.procedural_memory.learn_skill(skill).await
    }
    
    /// Update working memory context
    pub async fn update_context(&self, context: WorkingContext) -> SymbioteResult<()> {
        self.working_memory.update_context(context).await
    }
    
    /// Consolidate memories
    pub async fn consolidate_memories(&self) -> SymbioteResult<ConsolidationReport> {
        self.consolidation.consolidate_memories().await
    }
    
    /// Optimize memory usage
    pub async fn optimize_memory(&self) -> SymbioteResult<OptimizationReport> {
        self.forgetting.optimize_memory().await
    }
    
    /// Get memory statistics
    pub async fn get_statistics(&self) -> SymbioteResult<MemoryStatistics> {
        self.manager.get_statistics().await
    }
    
    /// Get the current configuration
    pub async fn get_config(&self) -> MemoryConfig {
        self.config.read().await.clone()
    }
    
    /// Update the configuration
    pub async fn update_config(&self, new_config: MemoryConfig) -> SymbioteResult<()> {
        let mut config = self.config.write().await;
        *config = new_config;
        
        // Notify all components of config change
        self.manager.on_config_changed(&*config).await?;
        self.working_memory.on_config_changed(&*config).await?;
        self.episodic_memory.on_config_changed(&*config).await?;
        self.semantic_memory.on_config_changed(&*config).await?;
        self.procedural_memory.on_config_changed(&*config).await?;
        self.consolidation.on_config_changed(&*config).await?;
        self.forgetting.on_config_changed(&*config).await?;
        self.retrieval.on_config_changed(&*config).await?;
        self.learning.on_config_changed(&*config).await?;
        
        Ok(())
    }
}

/// Memory representation
#[derive(Debug, Clone)]
pub struct Memory {
    /// Memory ID
    pub id: MemoryId,
    
    /// Memory content
    pub content: MemoryContent,
    
    /// Memory type
    pub memory_type: MemoryType,
    
    /// Creation timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
    
    /// Last accessed timestamp
    pub last_accessed: chrono::DateTime<chrono::Utc>,
    
    /// Access count
    pub access_count: u32,
    
    /// Importance score
    pub importance: f32,
    
    /// Associations with other memories
    pub associations: Vec<MemoryAssociation>,
}

/// Memory content
#[derive(Debug, Clone)]
pub enum MemoryContent {
    /// Text content
    Text(String),
    
    /// Structured data
    Structured(serde_json::Value),
    
    /// Binary data
    Binary(Vec<u8>),
    
    /// Reference to external content
    Reference(String),
}

/// Memory type
#[derive(Debug, Clone)]
pub enum MemoryType {
    /// Working memory
    Working,
    
    /// Episodic memory
    Episodic,
    
    /// Semantic memory
    Semantic,
    
    /// Procedural memory
    Procedural,
}

/// Memory association
#[derive(Debug, Clone)]
pub struct MemoryAssociation {
    /// Target memory ID
    pub target_id: MemoryId,
    
    /// Association strength
    pub strength: f32,
    
    /// Association type
    pub association_type: AssociationType,
}

/// Association type
#[derive(Debug, Clone)]
pub enum AssociationType {
    /// Temporal association
    Temporal,
    
    /// Semantic association
    Semantic,
    
    /// Causal association
    Causal,
    
    /// Similarity association
    Similarity,
}

/// Optimization report
#[derive(Debug, Clone)]
pub struct OptimizationReport {
    /// Memories optimized
    pub memories_optimized: usize,
    
    /// Memory freed (bytes)
    pub memory_freed: usize,
    
    /// Optimization duration
    pub duration: std::time::Duration,
    
    /// Optimization details
    pub details: Vec<String>,
}

#[async_trait::async_trait]
impl Service for MemorySystem {
    fn name(&self) -> &'static str {
        "memory_system"
    }
    
    fn dependencies(&self) -> Vec<&'static str> {
        vec!["context_system", "storage"]
    }
    
    async fn initialize(&mut self) -> SymbioteResult<()> {
        self.start().await
    }
    
    async fn shutdown(&mut self) -> SymbioteResult<()> {
        self.stop().await
    }
}

/// Initialize the memory system with default configuration
pub async fn initialize_memory_system() -> SymbioteResult<MemorySystem> {
    let config = MemoryConfig::default();
    MemorySystem::new(config).await
}

/// Initialize the memory system with custom configuration
pub async fn initialize_memory_system_with_config(config: MemoryConfig) -> SymbioteResult<MemorySystem> {
    MemorySystem::new(config).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_memory_system_creation() {
        let config = MemoryConfig::default();
        let memory_system = MemorySystem::new(config).await;
        assert!(memory_system.is_ok());
    }

    #[tokio::test]
    async fn test_memory_system_lifecycle() {
        let config = MemoryConfig::default();
        let memory_system = MemorySystem::new(config).await.unwrap();
        
        let start_result = memory_system.start().await;
        assert!(start_result.is_ok());
        
        let stop_result = memory_system.stop().await;
        assert!(stop_result.is_ok());
    }
}
