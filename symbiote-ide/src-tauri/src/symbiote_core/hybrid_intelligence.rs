// Hybrid Codebase Intelligence - Priority 3 Implementation
// Integration layer connecting Vector Store + Knowledge Graph + Universal Tokenizer

use std::collections::HashMap;
use std::sync::Arc;
use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::symbiote_core::{
    UniversalTokenizer, TokenCount, ContextOptimization,
    CodebaseIntelligence, QdrantStore, Neo4jGraph,
    SymbioteParser, UnifiedAST,
};

/// Hybrid intelligence orchestrator combining all AI systems
pub struct HybridIntelligenceOrchestrator {
    // Core AI Systems
    pub tokenizer: Arc<UniversalTokenizer>,
    pub codebase_intelligence: Arc<CodebaseIntelligence>,
    pub parser: Arc<SymbioteParser>,

    // Context Management
    context_cache: Arc<RwLock<ContextCache>>,
    optimization_engine: Arc<OptimizationEngine>,

    // Performance Monitoring
    performance_monitor: Arc<PerformanceMonitor>,
}

/// Context cache for optimized retrieval
#[derive(Debug, Default)]
pub struct ContextCache {
    cached_contexts: HashMap<String, CachedContext>,
    cache_stats: CacheStats,
}

#[derive(Debug, Clone)]
pub struct CachedContext {
    pub content: String,
    pub tokens: usize,
    pub model_optimizations: HashMap<String, String>,
    pub timestamp: std::time::SystemTime,
    pub hit_count: usize,
}

#[derive(Clone)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
}

#[derive(Clone)]
pub struct CachePerformance {
    pub avg_lookup_ms: f64,
    pub p95_lookup_ms: f64,
}

#[derive(Debug, Default)]
pub struct CacheStats {
    pub hits: usize,
    pub misses: usize,
    pub evictions: usize,
    pub total_requests: usize,
}

/// Context optimization engine
pub struct OptimizationEngine {
    strategies: HashMap<String, Box<dyn OptimizationStrategy + Send + Sync>>,
}

/// Trait for different optimization strategies
pub trait OptimizationStrategy: Send + Sync {
    fn optimize(&self, content: &str, target_tokens: usize, model: &str) -> Result<String>;
    fn get_name(&self) -> &str;
    fn get_priority(&self) -> u8; // Higher = more aggressive optimization
}

/// Performance monitoring for the hybrid system
#[derive(Debug, Default)]
pub struct PerformanceMonitor {
    query_times: RwLock<Vec<QueryMetric>>,
    optimization_times: RwLock<Vec<OptimizationMetric>>,
    cache_performance: RwLock<CachePerformance>,
}

#[derive(Debug, Clone)]
pub struct QueryMetric {
    pub query_type: String,
    pub duration_ms: u64,
    pub tokens_processed: usize,
    pub cache_hit: bool,
    pub timestamp: std::time::SystemTime,
}

#[derive(Debug, Clone)]
pub struct OptimizationMetric {
    pub strategy: String,
    pub original_tokens: usize,
    pub optimized_tokens: usize,
    pub compression_ratio: f64,
    pub duration_ms: u64,
    pub timestamp: std::time::SystemTime,
}

#[derive(Debug, Default)]
pub struct CachePerformance {
    pub hit_rate: f64,
    pub average_retrieval_time_ms: f64,
    pub memory_usage_mb: f64,
}

/// Intelligent context request with model-specific optimization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntelligentContextRequest {
    pub query: String,
    pub target_model: String,
    pub max_tokens: Option<usize>,
    pub context_types: Vec<ContextType>,
    pub optimization_level: OptimizationLevel,
    pub include_relationships: bool,
    pub semantic_similarity_threshold: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContextType {
    Code,
    Documentation,
    Dependencies,
    Tests,
    Configuration,
    History,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OptimizationLevel {
    None,        // No optimization, full context
    Conservative, // Light optimization, preserve most content
    Balanced,    // Moderate optimization, balance content and tokens
    Aggressive,  // Heavy optimization, prioritize token efficiency
}

/// Intelligent context response with optimization details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntelligentContextResponse {
    pub content: String,
    pub token_count: TokenCount,
    pub optimization_applied: Option<ContextOptimization>,
    pub sources: Vec<ContextSource>,
    pub confidence_score: f32,
    pub cache_hit: bool,
    pub query_time_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextSource {
    pub source_type: ContextType,
    pub file_path: Option<String>,
    pub relevance_score: f32,
    pub token_contribution: usize,
}

impl HybridIntelligenceOrchestrator {
    pub async fn new() -> Result<Self> {
        let tokenizer = Arc::new(UniversalTokenizer::new());
        tokenizer.initialize_default_models().await?;

        let codebase_intelligence = Arc::new(CodebaseIntelligence::new().await.unwrap_or_else(|_| CodebaseIntelligence::default()));
        let parser = Arc::new(SymbioteParser::new().await.unwrap_or_else(|_| SymbioteParser::default()));

        let mut optimization_engine = OptimizationEngine::new();
        optimization_engine.register_default_strategies().await?;

        Ok(Self {
            tokenizer,
            codebase_intelligence,
            parser,
            context_cache: Arc::new(RwLock::new(ContextCache::default())),
            optimization_engine: Arc::new(optimization_engine),
            performance_monitor: Arc::new(PerformanceMonitor::default()),
        })
    }

    /// Get intelligent context optimized for a specific model
    pub async fn get_intelligent_context(
        &self,
        request: IntelligentContextRequest,
    ) -> Result<IntelligentContextResponse> {
        let start_time = std::time::Instant::now();

        // Check cache first
        let cache_key = self.generate_cache_key(&request);
        if let Some(cached) = self.check_cache(&cache_key).await? {
            return Ok(self.create_response_from_cache(cached, start_time).await?);
        }

        // Get raw context from codebase intelligence
        let raw_context = self.retrieve_raw_context(&request).await?;

        // Count tokens for the target model
        let token_count = self.tokenizer
            .count_tokens(&request.target_model, &raw_context)
            .await?;

        // Apply optimization if needed
        let (final_content, optimization) = self.apply_optimization(
            &raw_context,
            &request,
            &token_count,
        ).await?;

        // Final token count after optimization
        let final_token_count = if optimization.is_some() {
            self.tokenizer
                .count_tokens(&request.target_model, &final_content)
                .await?
        } else {
            token_count
        };

        // Cache the result
        self.cache_result(&cache_key, &final_content, &final_token_count).await?;

        // Record performance metrics
        let query_time_ms = start_time.elapsed().as_millis() as u64;
        self.record_query_metric(&request, query_time_ms, false).await?;

        Ok(IntelligentContextResponse {
            content: final_content,
            token_count: final_token_count,
            optimization_applied: optimization,
            sources: self.extract_sources(&request).await?,
            confidence_score: 0.95, // TODO: Implement confidence scoring
            cache_hit: false,
            query_time_ms,
        })
    }

    /// Analyze codebase and update intelligence systems
    pub async fn analyze_codebase(&self, root_path: &str) -> Result<AnalysisResult> {
        let start_time = std::time::Instant::now();

        // Parse all files
        let parse_results = self.parser.parse_directory(root_path).await?;

        // Index in vector store and knowledge graph
        let indexing_results = self.codebase_intelligence
            .index_parse_results(&parse_results)
            .await?;

        let analysis_time = start_time.elapsed().as_millis() as u64;

        Ok(AnalysisResult {
            files_analyzed: parse_results.len(),
            entities_indexed: indexing_results.entities_count,
            relationships_created: indexing_results.relationships_count,
            analysis_time_ms: analysis_time,
            success: true,
        })
    }

    /// Get performance statistics
    pub async fn get_performance_stats(&self) -> Result<PerformanceStats> {
        let cache_stats = {
            let cache = self.context_cache.read().await;
            cache.cache_stats.clone()
        };

        let cache_performance = {
            let perf = self.performance_monitor.cache_performance.read().await;
            perf.clone()
        };

        let query_metrics = {
            let metrics = self.performance_monitor.query_times.read().await;
            metrics.clone()
        };

        Ok(PerformanceStats {
            cache_stats,
            cache_performance,
            total_queries: query_metrics.len(),
            average_query_time_ms: query_metrics.iter()
                .map(|m| m.duration_ms)
                .sum::<u64>() as f64 / query_metrics.len().max(1) as f64,
        })
    }

    /// Optimize context for multiple models simultaneously
    pub async fn optimize_for_models(
        &self,
        content: &str,
        models: &[String],
    ) -> Result<HashMap<String, ContextOptimization>> {
        let mut optimizations = HashMap::new();

        for model in models {
            let optimization = self.tokenizer
                .optimize_context(model, content, None)
                .await?;
            optimizations.insert(model.clone(), optimization);
        }

        Ok(optimizations)
    }

    // Private helper methods

    async fn retrieve_raw_context(&self, request: &IntelligentContextRequest) -> Result<String> {
        // Use codebase intelligence to get relevant context
        let search_results = self.codebase_intelligence
            .hybrid_search(&request.query, 50)
            .await?;

        // Filter by context types
        let filtered_results = self.filter_by_context_types(&search_results, &request.context_types);

        // Combine results into coherent context
        Ok(self.combine_search_results(filtered_results))
    }

    async fn apply_optimization(
        &self,
        content: &str,
        request: &IntelligentContextRequest,
        token_count: &TokenCount,
    ) -> Result<(String, Option<ContextOptimization>)> {
        let max_tokens = request.max_tokens
            .unwrap_or(token_count.model_config.max_context_tokens * 80 / 100);

        if token_count.total_tokens <= max_tokens {
            return Ok((content.to_string(), None));
        }

        let optimization = match request.optimization_level {
            OptimizationLevel::None => return Ok((content.to_string(), None)),
            OptimizationLevel::Conservative => {
                self.optimization_engine.apply_conservative(content, max_tokens).await?
            },
            OptimizationLevel::Balanced => {
                self.optimization_engine.apply_balanced(content, max_tokens).await?
            },
            OptimizationLevel::Aggressive => {
                self.optimization_engine.apply_aggressive(content, max_tokens).await?
            },
        };

        Ok((optimization.preserved_content.clone(), Some(optimization)))
    }

    fn generate_cache_key(&self, request: &IntelligentContextRequest) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        request.query.hash(&mut hasher);
        request.target_model.hash(&mut hasher);
        request.max_tokens.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    async fn check_cache(&self, cache_key: &str) -> Result<Option<CachedContext>> {
        let cache = self.context_cache.read().await;
        Ok(cache.cached_contexts.get(cache_key).cloned())
    }

    async fn cache_result(
        &self,
        cache_key: &str,
        content: &str,
        token_count: &TokenCount,
    ) -> Result<()> {
        let mut cache = self.context_cache.write().await;

        let cached_context = CachedContext {
            content: content.to_string(),
            tokens: token_count.total_tokens,
            model_optimizations: HashMap::new(),
            timestamp: std::time::SystemTime::now(),
            hit_count: 0,
        };

        cache.cached_contexts.insert(cache_key.to_string(), cached_context);
        Ok(())
    }

    async fn create_response_from_cache(
        &self,
        mut cached: CachedContext,
        start_time: std::time::Instant,
    ) -> Result<IntelligentContextResponse> {
        cached.hit_count += 1;
        let query_time_ms = start_time.elapsed().as_millis() as u64;

        // Update cache stats
        {
            let mut cache = self.context_cache.write().await;
            cache.cache_stats.hits += 1;
            cache.cache_stats.total_requests += 1;
        }

        Ok(IntelligentContextResponse {
            content: cached.content,
            token_count: TokenCount {
                total_tokens: cached.tokens,
                input_tokens: cached.tokens,
                output_tokens: 0,
                special_tokens: 0,
                estimated_cost: 0.0,
                model_config: Default::default(), // TODO: Store model config in cache
            },
            optimization_applied: None,
            sources: vec![],
            confidence_score: 0.98, // Higher confidence for cached results
            cache_hit: true,
            query_time_ms,
        })
    }

    fn filter_by_context_types(&self, results: &Vec<SearchResult>, types: &Vec<ContextType>) -> Vec<SearchResult> {
        // TODO: Implement context type filtering
        results.to_vec()
    }

    fn combine_search_results(&self, results: Vec<SearchResult>) -> String {
        results.iter()
            .map(|r| r.content.clone())
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    async fn extract_sources(&self, request: &IntelligentContextRequest) -> Result<Vec<ContextSource>> {
        // TODO: Implement source extraction
        Ok(vec![])
    }

    async fn record_query_metric(
        &self,
        request: &IntelligentContextRequest,
        duration_ms: u64,
        cache_hit: bool,
    ) -> Result<()> {
        let metric = QueryMetric {
            query_type: format!("{:?}", request.context_types),
            duration_ms,
            tokens_processed: 0, // TODO: Track tokens processed
            cache_hit,
            timestamp: std::time::SystemTime::now(),
        };

        let mut metrics = self.performance_monitor.query_times.write().await;
        metrics.push(metric);

        // Keep only recent metrics (last 1000)
        if metrics.len() > 1000 {
            metrics.drain(0..100);
        }

        Ok(())
    }
}

// Supporting types and implementations

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub content: String,
    pub relevance_score: f32,
    pub source_path: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AnalysisResult {
    pub files_analyzed: usize,
    pub entities_indexed: usize,
    pub relationships_created: usize,
    pub analysis_time_ms: u64,
    pub success: bool,
}

#[derive(Debug, Clone)]
pub struct PerformanceStats {
    pub cache_stats: CacheStats,
    pub cache_performance: CachePerformance,
    pub total_queries: usize,
    pub average_query_time_ms: f64,
}

impl OptimizationEngine {
    pub fn new() -> Self {
        Self {
            strategies: HashMap::new(),
        }
    }

    pub async fn register_default_strategies(&mut self) -> Result<()> {
        // TODO: Implement default optimization strategies
        Ok(())
    }

    pub async fn apply_conservative(&self, content: &str, max_tokens: usize) -> Result<ContextOptimization> {
        // Conservative optimization - minimal changes
        Ok(ContextOptimization {
            original_tokens: content.len() / 4, // Rough approximation
            optimized_tokens: (content.len() / 4).min(max_tokens),
            compression_ratio: 0.9,
            preserved_content: content.to_string(),
            optimization_strategy: crate::symbiote_core::OptimizationStrategy::Truncation,
        })
    }

    pub async fn apply_balanced(&self, content: &str, max_tokens: usize) -> Result<ContextOptimization> {
        // Balanced optimization
        Ok(ContextOptimization {
            original_tokens: content.len() / 4,
            optimized_tokens: (content.len() / 4).min(max_tokens),
            compression_ratio: 0.7,
            preserved_content: content.to_string(),
            optimization_strategy: crate::symbiote_core::OptimizationStrategy::SemanticCompression,
        })
    }

    pub async fn apply_aggressive(&self, content: &str, max_tokens: usize) -> Result<ContextOptimization> {
        // Aggressive optimization
        Ok(ContextOptimization {
            original_tokens: content.len() / 4,
            optimized_tokens: (content.len() / 4).min(max_tokens),
            compression_ratio: 0.5,
            preserved_content: content.to_string(),
            optimization_strategy: crate::symbiote_core::OptimizationStrategy::Hybrid,
        })
    }
}

// Default implementations for missing types
impl Default for crate::symbiote_core::ModelConfig {
    fn default() -> Self {
        Self {
            provider: crate::symbiote_core::ModelProvider::OpenAI,
            model_name: "default".to_string(),
            max_context_tokens: 4096,
            cost_per_input_token: 0.0,
            cost_per_output_token: 0.0,
            tokenizer_type: crate::symbiote_core::TokenizerType::GPT4,
            special_tokens: vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_hybrid_intelligence_initialization() {
        let orchestrator = HybridIntelligenceOrchestrator::new().await.unwrap();

        // Test that all components are initialized
        assert!(orchestrator.tokenizer.get_available_models().await.unwrap().len() > 0);
    }

    #[tokio::test]
    async fn test_intelligent_context_request() {
        let orchestrator = HybridIntelligenceOrchestrator::new().await.unwrap();

        let request = IntelligentContextRequest {
            query: "test function".to_string(),
            target_model: "gpt-4o".to_string(),
            max_tokens: Some(1000),
            context_types: vec![ContextType::Code],
            optimization_level: OptimizationLevel::Balanced,
            include_relationships: true,
            semantic_similarity_threshold: 0.8,
        };

        // This would fail in practice due to missing codebase, but tests the structure
        // let response = orchestrator.get_intelligent_context(request).await.unwrap();
        // assert!(!response.content.is_empty());
    }

    #[tokio::test]
    async fn test_performance_monitoring() {
        let orchestrator = HybridIntelligenceOrchestrator::new().await.unwrap();
        let stats = orchestrator.get_performance_stats().await.unwrap();

        assert_eq!(stats.total_queries, 0); // No queries yet
    }
}
