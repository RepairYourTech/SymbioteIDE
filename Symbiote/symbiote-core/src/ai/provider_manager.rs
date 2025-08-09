//! # AI Provider Manager for Symbiote IDE
//! 
//! Manages multiple AI providers with rate limiting, cost tracking, and fallback systems.
//! 
//! Following Week 9-10 Multi-Provider AI Integration implementation plan.

use crate::{Result, SymbioteError, config::AIProvidersConfig};
use crate::ai::providers::{AIProvider, ChatRequest, ChatResponse, OpenRouterProvider, OpenAIProvider, AnthropicProvider, GoogleProvider, AIModel, TokenUsage};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};

/// Rate limiter for API requests
#[derive(Debug)]
pub struct RateLimiter {
    limits: HashMap<String, RateLimit>,
}

/// Rate limit configuration
#[derive(Debug, Clone)]
pub struct RateLimit {
    pub requests_per_minute: u32,
    pub requests_made: u32,
    pub window_start: Instant,
}

impl RateLimiter {
    pub fn new() -> Self {
        Self {
            limits: HashMap::new(),
        }
    }

    pub async fn check_rate_limit(&mut self, model: &str) -> Result<()> {
        let now = Instant::now();
        
        let limit = self.limits.entry(model.to_string()).or_insert(RateLimit {
            requests_per_minute: 60, // Default limit
            requests_made: 0,
            window_start: now,
        });

        // Reset window if a minute has passed
        if now.duration_since(limit.window_start) >= Duration::from_secs(60) {
            limit.requests_made = 0;
            limit.window_start = now;
        }

        // Check if we're within limits
        if limit.requests_made >= limit.requests_per_minute {
            return Err(SymbioteError::ai_provider(format!(
                "Rate limit exceeded for model {}: {} requests per minute",
                model, limit.requests_per_minute
            )));
        }

        limit.requests_made += 1;
        Ok(())
    }
}

/// Cost tracking for AI usage
#[derive(Debug)]
pub struct CostTracker {
    usage_history: Arc<RwLock<Vec<UsageRecord>>>,
}

/// Usage record for cost tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageRecord {
    pub provider: String,
    pub model: String,
    pub timestamp: u64,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cost: f64,
}

/// Cost summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostSummary {
    pub total_cost: f64,
    pub cost_by_provider: HashMap<String, f64>,
    pub cost_by_model: HashMap<String, f64>,
    pub total_tokens: u32,
    pub period_start: u64,
    pub period_end: u64,
}

impl CostTracker {
    pub fn new() -> Self {
        Self {
            usage_history: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn track_usage(&self, provider: &str, usage: &TokenUsage) {
        // Calculate cost based on provider and model
        let cost = self.calculate_cost(provider, usage.prompt_tokens, usage.completion_tokens);
        
        let record = UsageRecord {
            provider: provider.to_string(),
            model: "unknown".to_string(), // TODO: Pass model info
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            input_tokens: usage.prompt_tokens,
            output_tokens: usage.completion_tokens,
            cost,
        };

        let mut history = self.usage_history.write().await;
        history.push(record);
        
        // Keep only last 10,000 records to prevent memory bloat
        if history.len() > 10000 {
            history.drain(0..1000);
        }
    }

    pub async fn get_summary(&self) -> CostSummary {
        let history = self.usage_history.read().await;
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        let period_start = now - 86400 * 30; // Last 30 days

        let mut total_cost = 0.0;
        let mut cost_by_provider = HashMap::new();
        let mut cost_by_model = HashMap::new();
        let mut total_tokens = 0;

        for record in history.iter() {
            if record.timestamp >= period_start {
                total_cost += record.cost;
                *cost_by_provider.entry(record.provider.clone()).or_insert(0.0) += record.cost;
                *cost_by_model.entry(record.model.clone()).or_insert(0.0) += record.cost;
                total_tokens += record.input_tokens + record.output_tokens;
            }
        }

        CostSummary {
            total_cost,
            cost_by_provider,
            cost_by_model,
            total_tokens,
            period_start,
            period_end: now,
        }
    }

    fn calculate_cost(&self, provider: &str, input_tokens: u32, output_tokens: u32) -> f64 {
        // Simplified cost calculation - in reality this would be model-specific
        let (input_cost_per_1k, output_cost_per_1k) = match provider {
            "openai" => (0.01, 0.03),
            "anthropic" => (0.003, 0.015),
            "google" => (0.00015, 0.0006),
            "openrouter" => (0.005, 0.015), // Average
            _ => (0.001, 0.002), // Default
        };

        let input_cost = (input_tokens as f64 / 1000.0) * input_cost_per_1k;
        let output_cost = (output_tokens as f64 / 1000.0) * output_cost_per_1k;
        
        input_cost + output_cost
    }
}

/// AI Provider Manager coordinating all providers
pub struct AIProviderManager {
    providers: HashMap<String, Box<dyn AIProvider>>,
    default_provider: String,
    fallback_providers: Vec<String>,
    rate_limiter: Arc<RwLock<RateLimiter>>,
    cost_tracker: Arc<CostTracker>,
}

impl AIProviderManager {
    pub fn new(config: &AIProvidersConfig) -> Result<Self> {
        let mut providers: HashMap<String, Box<dyn AIProvider>> = HashMap::new();

        // Initialize providers based on available API keys
        if let Some(api_key) = &config.openrouter_api_key {
            providers.insert("openrouter".to_string(), Box::new(OpenRouterProvider::new(api_key.clone())));
        }

        if let Some(api_key) = &config.openai_api_key {
            providers.insert("openai".to_string(), Box::new(OpenAIProvider::new(api_key.clone())));
        }

        if let Some(api_key) = &config.anthropic_api_key {
            providers.insert("anthropic".to_string(), Box::new(AnthropicProvider::new(api_key.clone())));
        }

        if let Some(api_key) = &config.google_api_key {
            providers.insert("google".to_string(), Box::new(GoogleProvider::new(api_key.clone())));
        }

        Ok(Self {
            providers,
            default_provider: config.default_provider.clone(),
            fallback_providers: vec!["openrouter".to_string(), "openai".to_string()],
            rate_limiter: Arc::new(RwLock::new(RateLimiter::new())),
            cost_tracker: Arc::new(CostTracker::new()),
        })
    }

    pub async fn chat_completion(&self, provider: Option<String>, request: ChatRequest) -> Result<ChatResponse> {
        let provider_name = provider.unwrap_or_else(|| self.default_provider.clone());

        // Try primary provider
        if let Some(provider) = self.providers.get(&provider_name) {
            match self.try_provider(provider.as_ref(), &request).await {
                Ok(response) => {
                    self.cost_tracker.track_usage(&provider_name, &response.usage).await;
                    return Ok(response);
                }
                Err(e) => {
                    tracing::warn!("Provider {} failed: {}", provider_name, e);
                }
            }
        }

        // Try fallback providers
        for fallback in &self.fallback_providers {
            if fallback != &provider_name {
                if let Some(provider) = self.providers.get(fallback) {
                    match self.try_provider(provider.as_ref(), &request).await {
                        Ok(response) => {
                            self.cost_tracker.track_usage(fallback, &response.usage).await;
                            return Ok(response);
                        }
                        Err(e) => {
                            tracing::warn!("Fallback provider {} failed: {}", fallback, e);
                        }
                    }
                }
            }
        }

        Err(SymbioteError::ai_provider("All providers failed".to_string()))
    }

    async fn try_provider(&self, provider: &dyn AIProvider, request: &ChatRequest) -> Result<ChatResponse> {
        // Check rate limits
        {
            let mut rate_limiter = self.rate_limiter.write().await;
            rate_limiter.check_rate_limit(&request.model).await?;
        }

        // Execute request
        provider.chat_completion(request.clone()).await
    }

    pub fn get_available_models(&self) -> Vec<AIModel> {
        let mut models = Vec::new();
        for provider in self.providers.values() {
            models.extend(provider.get_models());
        }
        models.sort_by(|a, b| a.name.cmp(&b.name));
        models
    }

    pub async fn get_cost_summary(&self) -> CostSummary {
        self.cost_tracker.get_summary().await
    }

    pub fn get_available_providers(&self) -> Vec<String> {
        self.providers.keys().cloned().collect()
    }
}
