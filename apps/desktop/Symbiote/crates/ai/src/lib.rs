//! # Symbiote AI
//!
//! Unified AI provider integration and model management for Symbiote. Features:
//! - Unified provider interface across 40+ AI providers (OpenAI, Anthropic, Google, etc.)
//! - Smart model selection with cost optimization and performance prediction
//! - Real-time cost tracking and budget management
//! - Performance optimization with caching, batching, and request optimization
//! - Robust error handling with retry logic and fallback mechanisms
//! - Streaming support for real-time chat and completion APIs
//! - Function calling support across all providers

#![deny(missing_docs)]
#![warn(clippy::all)]

// Core modules
pub mod providers;
pub mod models;
pub mod requests;
pub mod responses;
pub mod cost;
pub mod tools;
pub mod optimization;
pub mod monitoring;
pub mod manager;
pub mod types;

// Re-export main types
pub use providers::{AIProvider, OpenAIProvider, AnthropicProvider, GoogleProvider, OpenRouterProvider};
pub use models::{ModelCatalog, ModelInfo, ModelCapability, ModelSelection, SmartModelSelector};
pub use requests::{ChatRequest, CompletionRequest, EmbeddingRequest, ImageRequest, AudioRequest};
pub use responses::{ChatResponse, CompletionResponse, StreamingResponse, ResponseParser};
pub use cost::{CostTracker, BudgetManager, CostOptimizer, CostBreakdown};
pub use tools::{ToolDefinition, ToolCall, ToolExecution, FunctionCalling};
pub use optimization::{RequestOptimizer, ResponseCache, BatchProcessor, LoadBalancer};
pub use monitoring::{PerformanceMonitor, HealthChecker, MetricsCollector};
pub use manager::{AIProviderManager, MultiProviderOrchestrator, ReliabilityPolicy, BudgetGuard};
pub use types::{AIConfig, AIResult, AIError, ProviderId, ModelId};

// Core dependencies
use symbiote_core::{SymbioteResult, SymbioteError, Service};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Main AI system that orchestrates all AI provider interactions
pub struct AISystem {
    /// Unified AI provider manager
    pub provider_manager: Arc<AIProviderManager>,
    
    /// Model catalog and selection
    pub model_catalog: Arc<ModelCatalog>,
    
    /// Smart model selector
    pub model_selector: Arc<SmartModelSelector>,
    
    /// Cost tracking and management
    pub cost_tracker: Arc<CostTracker>,
    
    /// Budget management
    pub budget_manager: Arc<BudgetManager>,
    
    /// Performance optimization
    pub optimizer: Arc<RequestOptimizer>,
    
    /// System monitoring
    pub monitor: Arc<PerformanceMonitor>,
    
    /// Configuration
    config: Arc<RwLock<AIConfig>>,
}

/// Configuration for the AI system
#[derive(Debug, Clone)]
pub struct AIConfig {
    /// Enable AI features
    pub enabled: bool,
    
    /// Default provider
    pub default_provider: String,
    
    /// Enable cost tracking
    pub enable_cost_tracking: bool,
    
    /// Enable caching
    pub enable_caching: bool,
    
    /// Cache TTL in seconds
    pub cache_ttl: u64,
    
    /// Database connection string
    pub database_url: String,
    
    /// Provider configurations
    pub providers: ProviderConfigs,
    
    /// Budget configuration
    pub budget: BudgetConfig,
}

/// Provider configurations
#[derive(Debug, Clone)]
pub struct ProviderConfigs {
    /// OpenAI configuration
    pub openai: Option<ProviderConfig>,
    
    /// Anthropic configuration
    pub anthropic: Option<ProviderConfig>,
    
    /// Google configuration
    pub google: Option<ProviderConfig>,
    
    /// OpenRouter configuration
    pub openrouter: Option<ProviderConfig>,
    
    /// Local model configuration
    pub local: Option<LocalConfig>,
}

/// Individual provider configuration
#[derive(Debug, Clone)]
pub struct ProviderConfig {
    /// API key (stored securely in vault)
    pub api_key_id: String,
    
    /// Base URL for the provider
    pub base_url: Option<String>,
    
    /// Enable this provider
    pub enabled: bool,
    
    /// Rate limit (requests per minute)
    pub rate_limit: Option<u32>,
}

/// Local model configuration
#[derive(Debug, Clone)]
pub struct LocalConfig {
    /// Ollama endpoint
    pub ollama_endpoint: Option<String>,
    
    /// Enable local models
    pub enabled: bool,
}

/// Budget configuration
#[derive(Debug, Clone)]
pub struct BudgetConfig {
    /// Daily budget limit
    pub daily_limit: Option<rust_decimal::Decimal>,
    
    /// Monthly budget limit
    pub monthly_limit: Option<rust_decimal::Decimal>,
    
    /// Enable budget alerts
    pub enable_alerts: bool,
    
    /// Alert threshold (percentage)
    pub alert_threshold: f64,
}

impl Default for AIConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            default_provider: "openai".to_string(),
            enable_cost_tracking: true,
            enable_caching: true,
            cache_ttl: 3600, // 1 hour
            database_url: "postgresql://localhost/symbiote_ai".to_string(),
            providers: ProviderConfigs {
                openai: None,
                anthropic: None,
                google: None,
                openrouter: None,
                local: Some(LocalConfig {
                    ollama_endpoint: Some("http://localhost:11434".to_string()),
                    enabled: false,
                }),
            },
            budget: BudgetConfig {
                daily_limit: None,
                monthly_limit: None,
                enable_alerts: true,
                alert_threshold: 0.8, // 80%
            },
        }
    }
}

impl AISystem {
    /// Create a new AI system instance
    pub async fn new(config: AIConfig) -> SymbioteResult<Self> {
        let config = Arc::new(RwLock::new(config));
        
        // Initialize core components
        let provider_manager = Arc::new(AIProviderManager::new(config.clone()).await?);
        let model_catalog = Arc::new(ModelCatalog::new(config.clone()).await?);
        let model_selector = Arc::new(SmartModelSelector::new(config.clone(), model_catalog.clone()).await?);
        let cost_tracker = Arc::new(CostTracker::new(config.clone()).await?);
        let budget_manager = Arc::new(BudgetManager::new(config.clone(), cost_tracker.clone()).await?);
        let optimizer = Arc::new(RequestOptimizer::new(config.clone()).await?);
        let monitor = Arc::new(PerformanceMonitor::new(config.clone()).await?);
        
        Ok(Self {
            provider_manager,
            model_catalog,
            model_selector,
            cost_tracker,
            budget_manager,
            optimizer,
            monitor,
            config,
        })
    }
    
    /// Start the AI system
    pub async fn start(&self) -> SymbioteResult<()> {
        tracing::info!("Starting Symbiote AI System");
        
        // Initialize all components
        self.provider_manager.initialize().await?;
        self.model_catalog.initialize().await?;
        self.model_selector.initialize().await?;
        self.cost_tracker.initialize().await?;
        self.budget_manager.initialize().await?;
        self.optimizer.initialize().await?;
        self.monitor.initialize().await?;
        
        tracing::info!("Symbiote AI System started successfully");
        Ok(())
    }
    
    /// Stop the AI system
    pub async fn stop(&self) -> SymbioteResult<()> {
        tracing::info!("Stopping Symbiote AI System");
        
        // Shutdown all components in reverse order
        self.monitor.shutdown().await?;
        self.optimizer.shutdown().await?;
        self.budget_manager.shutdown().await?;
        self.cost_tracker.shutdown().await?;
        self.model_selector.shutdown().await?;
        self.model_catalog.shutdown().await?;
        self.provider_manager.shutdown().await?;
        
        tracing::info!("Symbiote AI System stopped successfully");
        Ok(())
    }
    
    /// Execute a chat request with optimal provider selection
    pub async fn chat(&self, request: ChatRequest) -> SymbioteResult<ChatResponse> {
        self.provider_manager.execute_chat_request(request).await
    }
    
    /// Execute a chat request with streaming response
    pub async fn chat_stream(&self, request: ChatRequest) -> SymbioteResult<StreamingResponse> {
        self.provider_manager.execute_chat_stream(request).await
    }
    
    /// Select the best model for a task
    pub async fn select_model(&self, requirements: ModelRequirements) -> SymbioteResult<ModelSelection> {
        self.model_selector.select_optimal_model(requirements).await
    }
    
    /// Get cost breakdown for a time period
    pub async fn get_cost_breakdown(&self, period: TimePeriod) -> SymbioteResult<CostBreakdown> {
        self.cost_tracker.get_cost_breakdown(period).await
    }
    
    /// Get system performance metrics
    pub async fn get_performance_metrics(&self) -> SymbioteResult<PerformanceMetrics> {
        self.monitor.get_system_metrics().await
    }
    
    /// Get the current configuration
    pub async fn get_config(&self) -> AIConfig {
        self.config.read().await.clone()
    }
    
    /// Update the configuration
    pub async fn update_config(&self, new_config: AIConfig) -> SymbioteResult<()> {
        let mut config = self.config.write().await;
        *config = new_config;
        
        // Notify all components of config change
        self.provider_manager.on_config_changed(&*config).await?;
        self.model_catalog.on_config_changed(&*config).await?;
        self.model_selector.on_config_changed(&*config).await?;
        self.cost_tracker.on_config_changed(&*config).await?;
        self.budget_manager.on_config_changed(&*config).await?;
        self.optimizer.on_config_changed(&*config).await?;
        self.monitor.on_config_changed(&*config).await?;
        
        Ok(())
    }
}

/// Model requirements for selection
#[derive(Debug, Clone)]
pub struct ModelRequirements {
    /// Required capabilities
    pub capabilities: Vec<ModelCapability>,
    
    /// Maximum cost per request
    pub max_cost: Option<rust_decimal::Decimal>,
    
    /// Minimum context length
    pub min_context_length: Option<u32>,
    
    /// Maximum latency
    pub max_latency: Option<std::time::Duration>,
    
    /// Quality threshold
    pub quality_threshold: Option<f64>,
}

/// Time period for cost analysis
#[derive(Debug, Clone)]
pub enum TimePeriod {
    /// Last hour
    Hour,
    /// Last day
    Day,
    /// Last week
    Week,
    /// Last month
    Month,
    /// Custom range
    Custom { start: chrono::DateTime<chrono::Utc>, end: chrono::DateTime<chrono::Utc> },
}

/// Performance metrics
#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    /// Total requests processed
    pub total_requests: u64,
    
    /// Average response time
    pub avg_response_time: std::time::Duration,
    
    /// Success rate
    pub success_rate: f64,
    
    /// Cache hit rate
    pub cache_hit_rate: f64,
    
    /// Total cost
    pub total_cost: rust_decimal::Decimal,
}

#[async_trait::async_trait]
impl Service for AISystem {
    fn name(&self) -> &'static str {
        "ai_system"
    }
    
    fn dependencies(&self) -> Vec<&'static str> {
        vec!["vault", "storage"]
    }
    
    async fn initialize(&mut self) -> SymbioteResult<()> {
        self.start().await
    }
    
    async fn shutdown(&mut self) -> SymbioteResult<()> {
        self.stop().await
    }
}

/// Initialize the AI system with default configuration
pub async fn initialize_ai_system() -> SymbioteResult<AISystem> {
    let config = AIConfig::default();
    AISystem::new(config).await
}

/// Initialize the AI system with custom configuration
pub async fn initialize_ai_system_with_config(config: AIConfig) -> SymbioteResult<AISystem> {
    AISystem::new(config).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_ai_system_creation() {
        let config = AIConfig::default();
        let ai_system = AISystem::new(config).await;
        assert!(ai_system.is_ok());
    }

    #[tokio::test]
    async fn test_ai_system_lifecycle() {
        let config = AIConfig::default();
        let ai_system = AISystem::new(config).await.unwrap();
        
        let start_result = ai_system.start().await;
        assert!(start_result.is_ok());
        
        let stop_result = ai_system.stop().await;
        assert!(stop_result.is_ok());
    }
}
