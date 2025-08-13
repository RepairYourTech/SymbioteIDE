//! # Symbiote Context
//!
//! Hybrid context engine that combines multiple storage and retrieval technologies. Features:
//! - Hybrid architecture with SQLite + Qdrant + Neo4j for optimal performance
//! - Semantic search with vector embeddings for intelligent retrieval
//! - Graph relationships for complex relationship modeling and traversal
//! - Fast metadata queries with SQLite for rapid indexing
//! - Context management with intelligent context window optimization
//! - Multi-modal support for text, code, images, and structured data
//! - Real-time updates with incremental indexing and live synchronization

#![deny(missing_docs)]
#![warn(clippy::all)]

// Core modules
pub mod engine;
pub mod storage;
pub mod search;
pub mod indexing;
pub mod embeddings;
pub mod graph;
pub mod chunking;
pub mod retrieval;
pub mod monitoring;
pub mod codebase;
pub mod types;

// Re-export main types
pub use engine::{ContextEngine, SymbioteContextEngine, HybridContextEngine};
pub use storage::{SqliteStorage, QdrantStorage, Neo4jStorage, StorageManager};
pub use search::{SearchEngine, SemanticSearch, HybridSearch, SearchResults};
pub use indexing::{IndexingPipeline, DocumentIndexer, IncrementalIndexer};
pub use embeddings::{EmbeddingManager, EmbeddingProvider, VectorStore};
pub use graph::{GraphDatabase, RelationshipManager, KnowledgeGraph};
pub use chunking::{ChunkingStrategy, IntelligentChunker, ContextChunk};
pub use retrieval::{HybridRetriever, ContextRanker, ContextBudgetPlanner};
pub use monitoring::{PerformanceMonitor, ContextMetrics, UsageTracker};
pub use codebase::{CodebaseIntelligence, LSPManager, ASTIndexer, DependencyTracker};
pub use types::{ContextConfig, ContextResult, ContextError, DocumentId, ContextId};

// Core dependencies
use symbiote_core::{SymbioteResult, SymbioteError, Service};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Main context system that orchestrates all context operations
pub struct ContextSystem {
    /// Hybrid context engine
    pub engine: Arc<SymbioteContextEngine>,
    
    /// Storage manager
    pub storage: Arc<StorageManager>,
    
    /// Search engine
    pub search: Arc<SearchEngine>,
    
    /// Indexing pipeline
    pub indexing: Arc<IndexingPipeline>,
    
    /// Embedding manager
    pub embeddings: Arc<EmbeddingManager>,
    
    /// Graph database
    pub graph: Arc<GraphDatabase>,
    
    /// Retrieval system
    pub retrieval: Arc<HybridRetriever>,
    
    /// Performance monitoring
    pub monitor: Arc<PerformanceMonitor>,
    
    /// Codebase intelligence
    pub codebase: Arc<CodebaseIntelligence>,
    
    /// Configuration
    config: Arc<RwLock<ContextConfig>>,
}

/// Configuration for the context system
#[derive(Debug, Clone)]
pub struct ContextConfig {
    /// Enable context features
    pub enabled: bool,
    
    /// SQLite database path
    pub sqlite_path: String,
    
    /// Qdrant configuration (optional)
    pub qdrant: Option<QdrantConfig>,
    
    /// Neo4j configuration (optional)
    pub neo4j: Option<Neo4jConfig>,
    
    /// Embedding configuration
    pub embeddings: EmbeddingConfig,
    
    /// Indexing configuration
    pub indexing: IndexingConfig,
    
    /// Search configuration
    pub search: SearchConfig,
    
    /// Performance configuration
    pub performance: PerformanceConfig,
}

/// Qdrant configuration
#[derive(Debug, Clone)]
pub struct QdrantConfig {
    /// Qdrant server URL
    pub url: String,
    
    /// Collection name
    pub collection: String,
    
    /// Vector dimension
    pub dimension: usize,
    
    /// Enable Qdrant
    pub enabled: bool,
}

/// Neo4j configuration
#[derive(Debug, Clone)]
pub struct Neo4jConfig {
    /// Neo4j server URL
    pub url: String,
    
    /// Username
    pub username: String,
    
    /// Password
    pub password: String,
    
    /// Database name
    pub database: String,
    
    /// Enable Neo4j
    pub enabled: bool,
}

/// Embedding configuration
#[derive(Debug, Clone)]
pub struct EmbeddingConfig {
    /// Embedding provider
    pub provider: String,
    
    /// Model name
    pub model: String,
    
    /// Embedding dimension
    pub dimension: usize,
    
    /// Batch size for embedding generation
    pub batch_size: usize,
}

/// Indexing configuration
#[derive(Debug, Clone)]
pub struct IndexingConfig {
    /// Enable real-time indexing
    pub real_time: bool,
    
    /// Batch size for indexing
    pub batch_size: usize,
    
    /// Indexing interval (seconds)
    pub interval: u64,
    
    /// Maximum file size to index (bytes)
    pub max_file_size: usize,
}

/// Search configuration
#[derive(Debug, Clone)]
pub struct SearchConfig {
    /// Default search limit
    pub default_limit: usize,
    
    /// Similarity threshold
    pub similarity_threshold: f32,
    
    /// Enable hybrid search
    pub hybrid_search: bool,
    
    /// Enable reranking
    pub reranking: bool,
}

/// Performance configuration
#[derive(Debug, Clone)]
pub struct PerformanceConfig {
    /// Cache size (number of items)
    pub cache_size: usize,
    
    /// Cache TTL (seconds)
    pub cache_ttl: u64,
    
    /// Enable performance monitoring
    pub monitoring: bool,
    
    /// Maximum context window size (tokens)
    pub max_context_tokens: usize,
}

impl Default for ContextConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            sqlite_path: "context.db".to_string(),
            qdrant: Some(QdrantConfig {
                url: "http://localhost:6333".to_string(),
                collection: "symbiote_context".to_string(),
                dimension: 1536, // OpenAI embedding dimension
                enabled: false, // Disabled by default
            }),
            neo4j: Some(Neo4jConfig {
                url: "bolt://localhost:7687".to_string(),
                username: "neo4j".to_string(),
                password: "password".to_string(),
                database: "neo4j".to_string(),
                enabled: false, // Disabled by default
            }),
            embeddings: EmbeddingConfig {
                provider: "openai".to_string(),
                model: "text-embedding-3-small".to_string(),
                dimension: 1536,
                batch_size: 100,
            },
            indexing: IndexingConfig {
                real_time: true,
                batch_size: 50,
                interval: 60, // 1 minute
                max_file_size: 10 * 1024 * 1024, // 10MB
            },
            search: SearchConfig {
                default_limit: 50,
                similarity_threshold: 0.75,
                hybrid_search: true,
                reranking: true,
            },
            performance: PerformanceConfig {
                cache_size: 10000,
                cache_ttl: 3600, // 1 hour
                monitoring: true,
                max_context_tokens: 8192,
            },
        }
    }
}

impl ContextSystem {
    /// Create a new context system instance
    pub async fn new(config: ContextConfig) -> SymbioteResult<Self> {
        let config = Arc::new(RwLock::new(config));
        
        // Initialize core components
        let storage = Arc::new(StorageManager::new(config.clone()).await?);
        let embeddings = Arc::new(EmbeddingManager::new(config.clone()).await?);
        let graph = Arc::new(GraphDatabase::new(config.clone()).await?);
        let indexing = Arc::new(IndexingPipeline::new(config.clone(), storage.clone(), embeddings.clone()).await?);
        let search = Arc::new(SearchEngine::new(config.clone(), storage.clone(), embeddings.clone()).await?);
        let retrieval = Arc::new(HybridRetriever::new(config.clone(), search.clone(), graph.clone()).await?);
        let monitor = Arc::new(PerformanceMonitor::new(config.clone()).await?);
        let codebase = Arc::new(CodebaseIntelligence::new(config.clone()).await?);
        let engine = Arc::new(SymbioteContextEngine::new(config.clone(), storage.clone(), search.clone(), retrieval.clone()).await?);
        
        Ok(Self {
            engine,
            storage,
            search,
            indexing,
            embeddings,
            graph,
            retrieval,
            monitor,
            codebase,
            config,
        })
    }
    
    /// Start the context system
    pub async fn start(&self) -> SymbioteResult<()> {
        tracing::info!("Starting Symbiote Context System");
        
        // Initialize all components
        self.storage.initialize().await?;
        self.embeddings.initialize().await?;
        self.graph.initialize().await?;
        self.indexing.initialize().await?;
        self.search.initialize().await?;
        self.retrieval.initialize().await?;
        self.monitor.initialize().await?;
        self.codebase.initialize().await?;
        self.engine.initialize().await?;
        
        tracing::info!("Symbiote Context System started successfully");
        Ok(())
    }
    
    /// Stop the context system
    pub async fn stop(&self) -> SymbioteResult<()> {
        tracing::info!("Stopping Symbiote Context System");
        
        // Shutdown all components in reverse order
        self.engine.shutdown().await?;
        self.codebase.shutdown().await?;
        self.monitor.shutdown().await?;
        self.retrieval.shutdown().await?;
        self.search.shutdown().await?;
        self.indexing.shutdown().await?;
        self.graph.shutdown().await?;
        self.embeddings.shutdown().await?;
        self.storage.shutdown().await?;
        
        tracing::info!("Symbiote Context System stopped successfully");
        Ok(())
    }
    
    /// Index a document
    pub async fn index_document(&self, document: Document) -> SymbioteResult<DocumentId> {
        self.indexing.index_document(document).await
    }
    
    /// Search for context
    pub async fn search(&self, query: &str, limit: Option<usize>) -> SymbioteResult<SearchResults> {
        self.search.search(query, limit.unwrap_or(50)).await
    }
    
    /// Build context window for AI interactions
    pub async fn build_context(&self, query: &str, max_tokens: usize) -> SymbioteResult<ContextWindow> {
        self.retrieval.build_context(query, max_tokens).await
    }
    
    /// Get document by ID
    pub async fn get_document(&self, id: DocumentId) -> SymbioteResult<Option<Document>> {
        self.storage.get_document(id).await
    }
    
    /// Get system metrics
    pub async fn get_metrics(&self) -> SymbioteResult<ContextMetrics> {
        self.monitor.get_metrics().await
    }
    
    /// Get the current configuration
    pub async fn get_config(&self) -> ContextConfig {
        self.config.read().await.clone()
    }
    
    /// Update the configuration
    pub async fn update_config(&self, new_config: ContextConfig) -> SymbioteResult<()> {
        let mut config = self.config.write().await;
        *config = new_config;
        
        // Notify all components of config change
        self.storage.on_config_changed(&*config).await?;
        self.embeddings.on_config_changed(&*config).await?;
        self.graph.on_config_changed(&*config).await?;
        self.indexing.on_config_changed(&*config).await?;
        self.search.on_config_changed(&*config).await?;
        self.retrieval.on_config_changed(&*config).await?;
        self.monitor.on_config_changed(&*config).await?;
        self.codebase.on_config_changed(&*config).await?;
        self.engine.on_config_changed(&*config).await?;
        
        Ok(())
    }
}

/// Document for indexing
#[derive(Debug, Clone)]
pub struct Document {
    /// Document ID
    pub id: Option<DocumentId>,
    
    /// Document content
    pub content: String,
    
    /// Document metadata
    pub metadata: DocumentMetadata,
    
    /// Document type
    pub doc_type: DocumentType,
}

/// Document metadata
#[derive(Debug, Clone)]
pub struct DocumentMetadata {
    /// File path
    pub path: Option<String>,
    
    /// File size
    pub size: Option<usize>,
    
    /// Creation time
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    
    /// Modification time
    pub modified_at: Option<chrono::DateTime<chrono::Utc>>,
    
    /// Language
    pub language: Option<String>,
    
    /// Tags
    pub tags: Vec<String>,
}

/// Document type
#[derive(Debug, Clone)]
pub enum DocumentType {
    /// Text document
    Text,
    /// Code file
    Code,
    /// Markdown document
    Markdown,
    /// JSON data
    Json,
    /// Configuration file
    Config,
    /// Other type
    Other(String),
}

/// Context window for AI interactions
#[derive(Debug, Clone)]
pub struct ContextWindow {
    /// Context content
    pub content: String,
    
    /// Token count
    pub token_count: usize,
    
    /// Source documents
    pub sources: Vec<ContextSource>,
    
    /// Relevance score
    pub relevance_score: f32,
}

/// Context source
#[derive(Debug, Clone)]
pub struct ContextSource {
    /// Document ID
    pub document_id: DocumentId,
    
    /// Chunk content
    pub content: String,
    
    /// Relevance score
    pub score: f32,
    
    /// Source metadata
    pub metadata: DocumentMetadata,
}

#[async_trait::async_trait]
impl Service for ContextSystem {
    fn name(&self) -> &'static str {
        "context_system"
    }
    
    fn dependencies(&self) -> Vec<&'static str> {
        vec!["ai_system", "storage"]
    }
    
    async fn initialize(&mut self) -> SymbioteResult<()> {
        self.start().await
    }
    
    async fn shutdown(&mut self) -> SymbioteResult<()> {
        self.stop().await
    }
}

/// Initialize the context system with default configuration
pub async fn initialize_context_system() -> SymbioteResult<ContextSystem> {
    let config = ContextConfig::default();
    ContextSystem::new(config).await
}

/// Initialize the context system with custom configuration
pub async fn initialize_context_system_with_config(config: ContextConfig) -> SymbioteResult<ContextSystem> {
    ContextSystem::new(config).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_context_system_creation() {
        let config = ContextConfig::default();
        let context_system = ContextSystem::new(config).await;
        assert!(context_system.is_ok());
    }

    #[tokio::test]
    async fn test_context_system_lifecycle() {
        let config = ContextConfig::default();
        let context_system = ContextSystem::new(config).await.unwrap();
        
        let start_result = context_system.start().await;
        assert!(start_result.is_ok());
        
        let stop_result = context_system.stop().await;
        assert!(stop_result.is_ok());
    }
}
