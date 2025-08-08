use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use anyhow::{Result, anyhow};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use tokio::sync::RwLock;
use std::sync::Arc;

// Hybrid Codebase Intelligence System
// Combines Qdrant (vector search) + Neo4j (knowledge graph) for enterprise-scale codebase awareness

#[derive(Debug)]
pub struct CodebaseIntelligence {
    pub vector_store: Arc<QdrantStore>,
    pub knowledge_graph: Arc<Neo4jGraph>,
    pub indexing_engine: Arc<IndexingEngine>,
    pub context_orchestrator: Arc<ContextOrchestrator>,
    pub cache: Arc<RwLock<IntelligenceCache>>,
}

// QdrantStore and Neo4jGraph are now defined in their respective modules

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexingEngine {
    pub parsers: HashMap<String, LanguageParser>,
    pub embedders: HashMap<String, EmbeddingModel>,
    pub extractors: Vec<FeatureExtractor>,
    pub processors: Vec<ContentProcessor>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextOrchestrator {
    pub retrieval_strategies: Vec<RetrievalStrategy>,
    pub fusion_algorithms: Vec<FusionAlgorithm>,
    pub ranking_models: Vec<RankingModel>,
    pub context_optimizers: Vec<ContextOptimizer>,
}

// Core Data Structures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeEntity {
    pub id: String,
    pub entity_type: EntityType,
    pub name: String,
    pub file_path: String,
    pub start_line: u32,
    pub end_line: u32,
    pub language: String,
    pub content: String,
    pub signature: Option<String>,
    pub docstring: Option<String>,
    pub metadata: EntityMetadata,
    pub embedding: Option<Vec<f32>>,
    pub relationships: Vec<Relationship>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EntityType {
    Function,
    Class,
    Interface,
    Struct,
    Enum,
    Variable,
    Constant,
    Module,
    Package,
    Component,
    Hook,
    Service,
    Test,
    Configuration,
    Documentation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityMetadata {
    pub complexity: u32,
    pub dependencies: Vec<String>,
    pub dependents: Vec<String>,
    pub test_coverage: f32,
    pub last_modified: DateTime<Utc>,
    pub author: String,
    pub tags: Vec<String>,
    pub annotations: Vec<Annotation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    pub from: String,
    pub to: String,
    pub relationship_type: RelationshipType,
    pub strength: f32,
    pub metadata: RelationshipMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RelationshipType {
    // Code relationships
    Calls,
    CalledBy,
    Imports,
    ImportedBy,
    Inherits,
    InheritedBy,
    Implements,
    ImplementedBy,
    Uses,
    UsedBy,
    Contains,
    ContainedBy,
    
    // Semantic relationships
    Similar,
    Related,
    Duplicate,
    Alternative,
    
    // Temporal relationships
    ModifiedTogether,
    CreatedTogether,
    TestedTogether,
    
    // Architectural relationships
    LayerAbove,
    LayerBelow,
    SameLayer,
    CrossCutting,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipMetadata {
    pub confidence: f32,
    pub frequency: u32,
    pub last_observed: DateTime<Utc>,
    pub context: String,
}

// Chunking Strategies for Vector Storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChunkingStrategy {
    Semantic {
        max_tokens: usize,
        overlap: usize,
        preserve_structure: bool,
    },
    Hierarchical {
        levels: Vec<ChunkLevel>,
        cross_references: bool,
    },
    Adaptive {
        complexity_threshold: f32,
        context_window: usize,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkLevel {
    pub name: String,
    pub max_size: usize,
    pub entity_types: Vec<EntityType>,
}

// Advanced Context Retrieval
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextQuery {
    pub query_text: String,
    pub query_type: QueryType,
    pub scope: QueryScope,
    pub filters: Vec<QueryFilter>,
    pub max_results: usize,
    pub include_relationships: bool,
    pub fusion_strategy: FusionStrategy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QueryType {
    Semantic,
    Structural,
    Hybrid,
    Exploratory,
    Analytical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QueryScope {
    Global,
    Project,
    Module,
    File,
    Function,
    Custom(Vec<String>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryFilter {
    pub field: String,
    pub operator: FilterOperator,
    pub value: FilterValue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FilterOperator {
    Equals,
    NotEquals,
    Contains,
    StartsWith,
    EndsWith,
    GreaterThan,
    LessThan,
    InRange,
    InList,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FilterValue {
    String(String),
    Number(f64),
    Boolean(bool),
    List(Vec<String>),
    Range(f64, f64),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FusionStrategy {
    RankFusion,
    ScoreFusion,
    HybridFusion,
    LearningToRank,
}

// Implementation
impl CodebaseIntelligence {
    pub async fn new(config: IntelligenceConfig) -> Result<Self> {
        let vector_store = Arc::new(QdrantStore::new(config.qdrant_config).await?);
        let knowledge_graph = Arc::new(Neo4jGraph::new(config.neo4j_config).await?);
        let indexing_engine = Arc::new(IndexingEngine::new(config.indexing_config).await?);
        let context_orchestrator = Arc::new(ContextOrchestrator::new(config.orchestrator_config).await?);
        let cache = Arc::new(RwLock::new(IntelligenceCache::new()));

        Ok(Self {
            vector_store,
            knowledge_graph,
            indexing_engine,
            context_orchestrator,
            cache,
        })
    }

    pub async fn index_codebase(&self, codebase_path: &str) -> Result<IndexingResult> {
        let mut result = IndexingResult::new();
        
        // 1. Parse and extract entities
        let entities = self.indexing_engine.extract_entities(codebase_path).await?;
        result.entities_processed = entities.len();

        // 2. Generate embeddings for vector search
        let embedded_entities = self.indexing_engine.generate_embeddings(&entities).await?;
        
        // 3. Store in Qdrant
        self.vector_store.store_entities(&embedded_entities).await?;
        result.vectors_stored = embedded_entities.len();

        // 4. Extract relationships
        let relationships = self.indexing_engine.extract_relationships(&entities).await?;
        result.relationships_extracted = relationships.len();

        // 5. Store in Neo4j
        self.knowledge_graph.store_graph(&entities, &relationships).await?;
        result.graph_nodes_created = entities.len();
        result.graph_edges_created = relationships.len();

        // 6. Build cross-references
        self.build_cross_references(&entities).await?;

        // 7. Update cache
        self.update_cache(&entities).await?;

        Ok(result)
    }

    pub async fn query_context(&self, query: ContextQuery) -> Result<ContextResult> {
        // Hybrid retrieval: Vector + Graph + Cache
        let vector_results = self.vector_search(&query).await?;
        let graph_results = self.graph_search(&query).await?;
        let cached_results = self.cache_search(&query).await?;

        // Fusion and ranking
        let fused_results = self.context_orchestrator
            .fuse_results(vector_results, graph_results, cached_results, &query)
            .await?;

        // Context optimization
        let optimized_context = self.context_orchestrator
            .optimize_context(fused_results, &query)
            .await?;

        Ok(optimized_context)
    }

    async fn vector_search(&self, query: &ContextQuery) -> Result<Vec<VectorResult>> {
        // Generate query embedding
        let query_embedding = self.indexing_engine
            .embed_query(&query.query_text)
            .await?;

        // Search with filters
        let search_params = VectorSearchParams {
            vector: query_embedding,
            limit: query.max_results,
            filters: self.build_vector_filters(&query.filters),
            include_metadata: true,
            score_threshold: Some(0.7),
        };

        self.vector_store.search(search_params, 10, 0.7).await
    }

    async fn graph_search(&self, query: &ContextQuery) -> Result<Vec<GraphResult>> {
        match query.query_type {
            QueryType::Structural => {
                self.knowledge_graph.structural_search(query).await
            },
            QueryType::Exploratory => {
                self.knowledge_graph.exploratory_search(query).await
            },
            QueryType::Analytical => {
                self.knowledge_graph.analytical_search(query).await
            },
            _ => {
                self.knowledge_graph.semantic_search(query).await
            }
        }
    }

    async fn cache_search(&self, query: &ContextQuery) -> Result<Vec<CachedResult>> {
        let cache = self.cache.read().await;
        cache.search(query).await
    }

    async fn build_cross_references(&self, entities: &[CodeEntity]) -> Result<()> {
        // Build bidirectional references between vector store and graph
        for entity in entities {
            let vector_id = self.vector_store.get_id_for_entity(&entity.id).await?;
            let graph_id = self.knowledge_graph.get_id_for_entity(&entity.id).await?;
            
            // Store cross-references
            self.vector_store.add_graph_reference(&vector_id, &graph_id).await?;
            self.knowledge_graph.add_vector_reference(&graph_id, &vector_id).await?;
        }
        Ok(())
    }

    async fn update_cache(&self, entities: &[CodeEntity]) -> Result<()> {
        let mut cache = self.cache.write().await;
        
        // Update frequently accessed entities
        for entity in entities {
            if self.is_frequently_accessed(&entity.id).await? {
                cache.store_entity(entity.clone()).await?;
            }
        }
        
        // Update relationship patterns
        cache.update_relationship_patterns(entities).await?;
        
        Ok(())
    }

    async fn is_frequently_accessed(&self, entity_id: &str) -> Result<bool> {
        // Check access patterns from both stores
        let vector_access = self.vector_store.get_access_count(entity_id).await?;
        let graph_access = self.knowledge_graph.get_access_count(entity_id).await?;
        
        Ok(vector_access as u64 + graph_access as u64 > 10) // Threshold for frequent access
    }

    // Agent-specific context retrieval
    pub async fn get_agent_context(
        &self,
        agent_id: &str,
        task_context: &str,
        max_tokens: usize
    ) -> Result<AgentContext> {
        let query = ContextQuery {
            query_text: task_context.to_string(),
            query_type: QueryType::Hybrid,
            scope: QueryScope::Global,
            filters: vec![],
            max_results: 50,
            include_relationships: true,
            fusion_strategy: FusionStrategy::HybridFusion,
        };

        let context_result = self.query_context(query).await?;
        
        // Optimize for agent's token limit
        let optimized = self.context_orchestrator
            .optimize_for_agent(context_result, agent_id, max_tokens)
            .await?;

        Ok(optimized)
    }

    // Real-time incremental updates
    pub async fn update_entity(&self, entity: CodeEntity) -> Result<()> {
        // Update vector store
        let embedded_entity = self.indexing_engine
            .generate_embedding_for_entity(&entity)
            .await?;
        self.vector_store.update_entity(embedded_entity).await?;

        // Update knowledge graph
        let relationships = self.indexing_engine
            .extract_entity_relationships(&entity)
            .await?;
        self.knowledge_graph.update_entity(&entity, &relationships).await?;

        // Invalidate cache
        let mut cache = self.cache.write().await;
        cache.invalidate_entity(&entity.id).await?;

        Ok(())
    }

    // Massive codebase optimization
    pub async fn optimize_for_scale(&self) -> Result<()> {
        // Partition vector collections by language/module
        self.vector_store.create_partitions().await?;
        
        // Create graph indices for common query patterns
        self.knowledge_graph.create_performance_indices().await?;
        
        // Precompute frequent relationship paths
        self.knowledge_graph.precompute_paths().await?;
        
        // Optimize cache with LRU and predictive loading
        let mut cache = self.cache.write().await;
        cache.optimize_for_scale().await?;
        
        Ok(())
    }

    // Task-specific methods for task manager integration
    pub async fn generate_task_embedding(&self, task: &crate::task_manager::Task) -> Result<Vec<f32>> {
        let task_text = format!("{} {} {}", task.title, task.description.as_deref().unwrap_or(""),
                               task.tags.join(" "));
        self.indexing_engine.generate_text_embedding(&task_text).await
    }

    pub async fn add_task_to_graph(&self, task: &crate::task_manager::Task) -> Result<String> {
        let node_id = format!("task_{}", task.id);
        // Create task node in knowledge graph
        self.knowledge_graph.create_node(&node_id, "Task", &[
            ("title", &task.title),
            ("description", &task.description),
            ("status", &format!("{:?}", task.status)),
            ("priority", &format!("{:?}", task.priority)),
        ]).await?;
        Ok(node_id)
    }

    pub async fn update_task_in_graph(&self, task: &crate::task_manager::Task) -> Result<()> {
        let node_id = format!("task_{}", task.id);
        self.knowledge_graph.update_node(&node_id, &[
            ("title", &task.title),
            ("description", &task.description),
            ("status", &format!("{:?}", task.status)),
            ("priority", &format!("{:?}", task.priority)),
        ]).await
    }

    pub async fn remove_task_from_graph(&self, task_id: &str) -> Result<()> {
        let node_id = format!("task_{}", task_id);
        self.knowledge_graph.delete_node(&node_id).await
    }

    pub async fn semantic_search_tasks(&self, query: &str, limit: usize) -> Result<Vec<String>> {
        let embedding = self.indexing_engine.generate_text_embedding(query).await?;
        let results = self.vector_store.search(&embedding, limit, &[]).await?;
        Ok(results.into_iter()
            .filter_map(|r| r.payload.get("task_id").map(|id| id.to_string()))
            .collect())
    }

    pub async fn find_related_task_nodes(&self, task_id: &str, max_depth: usize) -> Result<Vec<String>> {
        let node_id = format!("task_{}", task_id);
        let related = self.knowledge_graph.find_related_nodes(&node_id, max_depth).await?;
        Ok(related.into_iter()
            .filter_map(|node| {
                if node.starts_with("task_") {
                    Some(node.strip_prefix("task_").unwrap().to_string())
                } else {
                    None
                }
            })
            .collect())
    }
}

// Supporting structures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntelligenceConfig {
    pub qdrant_config: QdrantConfig,
    pub neo4j_config: Neo4jConfig,
    pub indexing_config: IndexingConfig,
    pub orchestrator_config: OrchestratorConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexingResult {
    pub entities_processed: usize,
    pub vectors_stored: usize,
    pub relationships_extracted: usize,
    pub graph_nodes_created: usize,
    pub graph_edges_created: usize,
    pub duration_ms: u64,
}

impl IndexingResult {
    pub fn new() -> Self {
        Self {
            entities_processed: 0,
            vectors_stored: 0,
            relationships_extracted: 0,
            graph_nodes_created: 0,
            graph_edges_created: 0,
            duration_ms: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextResult {
    pub entities: Vec<CodeEntity>,
    pub relationships: Vec<Relationship>,
    pub metadata: ContextMetadata,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextMetadata {
    pub sources: Vec<String>,
    pub retrieval_time_ms: u64,
    pub fusion_strategy_used: String,
    pub total_candidates: usize,
    pub filtered_candidates: usize,
}

pub type AgentContext = ContextResult;
pub type VectorResult = CodeEntity;
pub type GraphResult = CodeEntity;
pub type CachedResult = CodeEntity;

// Implementation imports
use crate::qdrant_store::{QdrantStore, QdrantConfig, VectorSearchParams, SearchResult};
use crate::neo4j_graph::{Neo4jGraph, Neo4jConfig, Neo4jGraphResult};

// Additional implementation structures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntelligenceCache {
    entities: HashMap<String, CodeEntity>,
    relationships: HashMap<String, Vec<Relationship>>,
    frequent_queries: HashMap<String, ContextResult>,
    access_patterns: HashMap<String, u32>,
}

// Duplicate definitions removed - using the ones defined earlier in the file

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageParser {
    pub name: String,
    pub language: String,
    pub file_extensions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingModel {
    pub name: String,
    pub dimensions: usize,
    pub max_tokens: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureExtractor {
    pub name: String,
    pub feature_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentProcessor {
    pub name: String,
    pub processor_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalStrategy {
    pub name: String,
    pub strategy_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FusionAlgorithm {
    pub name: String,
    pub algorithm_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankingModel {
    pub name: String,
    pub model_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextOptimizer {
    pub name: String,
    pub optimizer_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Annotation {
    pub annotation_type: String,
    pub value: String,
    pub confidence: f32,
}

impl IntelligenceCache {
    pub fn new() -> Self {
        Self {
            entities: HashMap::new(),
            relationships: HashMap::new(),
            frequent_queries: HashMap::new(),
            access_patterns: HashMap::new(),
        }
    }

    pub async fn search(&self, query: &ContextQuery) -> Result<Vec<CachedResult>> {
        // Search through cached entities based on query
        let mut results = Vec::new();
        
        for entity in self.entities.values() {
            if self.matches_query(entity, query) {
                results.push(entity.clone());
            }
        }
        
        Ok(results)
    }

    pub async fn store_entity(&mut self, entity: CodeEntity) -> Result<()> {
        self.entities.insert(entity.id.clone(), entity);
        Ok(())
    }

    pub async fn invalidate_entity(&mut self, entity_id: &str) -> Result<()> {
        self.entities.remove(entity_id);
        self.relationships.remove(entity_id);
        Ok(())
    }

    pub async fn update_relationship_patterns(&mut self, entities: &[CodeEntity]) -> Result<()> {
        // Update relationship patterns based on entities
        for entity in entities {
            for relationship in &entity.relationships {
                let relationships = self.relationships.entry(entity.id.clone()).or_insert_with(Vec::new);
                relationships.push(relationship.clone());
            }
        }
        Ok(())
    }

    pub async fn optimize_for_scale(&mut self) -> Result<()> {
        // Implement LRU eviction and predictive loading
        if self.entities.len() > 10000 {
            // Remove least accessed entities
            let mut entities_by_access: Vec<_> = self.entities.iter()
                .map(|(id, entity)| (id.clone(), self.access_patterns.get(id).unwrap_or(&0)))
                .collect();
            entities_by_access.sort_by(|a, b| a.1.cmp(b.1));
            
            // Remove bottom 20%
            let to_remove = entities_by_access.len() / 5;
            for (id, _) in entities_by_access.iter().take(to_remove) {
                self.entities.remove(id);
                self.access_patterns.remove(id);
            }
        }
        Ok(())
    }

    fn matches_query(&self, entity: &CodeEntity, query: &ContextQuery) -> bool {
        // Simple matching logic - in production this would be more sophisticated
        entity.name.contains(&query.query_text) || 
        entity.content.contains(&query.query_text) ||
        entity.docstring.as_ref().map_or(false, |doc| doc.contains(&query.query_text))
    }
}

impl IndexingEngine {
    pub async fn new(config: IndexingConfig) -> Result<Self> {
        Ok(Self {
            parsers: HashMap::new(),
            embedders: HashMap::new(),
            extractors: vec![],
            processors: vec![],
        })
    }

    pub async fn extract_entities(&self, codebase_path: &str) -> Result<Vec<CodeEntity>> {
        // Mock implementation - in production this would parse the codebase
        Ok(vec![])
    }

    pub async fn generate_embeddings(&self, entities: &[CodeEntity]) -> Result<Vec<CodeEntity>> {
        // Mock implementation - in production this would generate embeddings
        Ok(entities.to_vec())
    }

    pub async fn generate_text_embedding(&self, text: &str) -> Result<Vec<f32>> {
        // Mock implementation - in production this would generate embeddings from text
        // Return a dummy embedding vector
        Ok(vec![0.1; 384]) // Common embedding dimension
    }

    pub async fn extract_relationships(&self, entities: &[CodeEntity]) -> Result<Vec<Relationship>> {
        // Mock implementation - in production this would extract relationships
        Ok(vec![])
    }

    pub async fn generate_embedding_for_entity(&self, entity: &CodeEntity) -> Result<CodeEntity> {
        // Mock implementation
        Ok(entity.clone())
    }

    pub async fn extract_entity_relationships(&self, entity: &CodeEntity) -> Result<Vec<Relationship>> {
        // Mock implementation
        Ok(entity.relationships.clone())
    }

    pub async fn embed_query(&self, query: &str) -> Result<Vec<f32>> {
        // Mock implementation
        Ok(vec![0.0; 384])
    }
}

impl ContextOrchestrator {
    pub async fn new(config: OrchestratorConfig) -> Result<Self> {
        Ok(Self {
            retrieval_strategies: vec![],
            fusion_algorithms: vec![],
            ranking_models: vec![],
            context_optimizers: vec![],
        })
    }

    pub async fn fuse_results(
        &self,
        vector_results: Vec<VectorResult>,
        graph_results: Vec<GraphResult>,
        cached_results: Vec<CachedResult>,
        query: &ContextQuery,
    ) -> Result<ContextResult> {
        // Combine results from different sources
        let mut all_entities = Vec::new();
        all_entities.extend(vector_results);
        all_entities.extend(graph_results);
        all_entities.extend(cached_results);

        Ok(ContextResult {
            entities: all_entities,
            relationships: vec![],
            metadata: ContextMetadata {
                sources: vec!["vector".to_string(), "graph".to_string(), "cache".to_string()],
                retrieval_time_ms: 100,
                fusion_strategy_used: "hybrid".to_string(),
                total_candidates: 0,
                filtered_candidates: 0,
            },
            confidence: 0.8,
        })
    }

    pub async fn optimize_context(&self, context: ContextResult, query: &ContextQuery) -> Result<ContextResult> {
        // Optimize context for the specific query
        Ok(context)
    }

    pub async fn optimize_for_agent(
        &self,
        context: ContextResult,
        agent_id: &str,
        max_tokens: usize,
    ) -> Result<AgentContext> {
        // Optimize context for specific agent and token limit
        Ok(context)
    }
}

// Configuration structures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexingConfig {
    pub languages: Vec<String>,
    pub max_file_size: usize,
    pub embedding_model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestratorConfig {
    pub fusion_strategy: String,
    pub ranking_model: String,
    pub max_results: usize,
}
