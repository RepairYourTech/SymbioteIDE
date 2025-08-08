// Universal Tokenizer - Priority 2 Implementation
// Multi-model token counting and cost optimization system

use std::collections::HashMap;
use std::sync::Arc;
use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

/// Comprehensive AI model providers (August 2025)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ModelProvider {
    // Major Commercial Providers
    OpenAI,
    Anthropic,
    Google,
    Microsoft, // Azure OpenAI Service
    
    // Chinese AI Providers
    DeepSeek,
    Qwen, // Alibaba
    Moonshot, // Kimi
    GLM, // Zhipu AI (Z.ai)
    Baidu, // ERNIE Bot
    ByteDance, // Doubao
    
    // European/Other Commercial
    Mistral, // French AI
    Cohere, // Canadian AI
    xAI, // Elon Musk's Grok
    Perplexity, // Search-focused AI
    
    // Infrastructure/Inference Providers
    Groq, // Fast inference hardware
    TogetherAI, // Distributed inference
    Fireworks, // Fast inference platform
    Replicate, // Model hosting platform
    Cerebras, // AI compute platform
    
    // Open Source/Community
    Meta, // Llama models
    HuggingFace, // Model hub and hosting
    Stability, // Stable Diffusion, etc.
    
    // Cloud Platform Providers
    AWS, // Amazon Bedrock
    Azure, // Microsoft Azure AI
    GCP, // Google Cloud AI
    
    // Local/Self-Hosted
    Ollama, // Local model runner
    LMStudio, // Local model interface
    LocalAI, // Self-hosted API
    VLLM, // High-performance inference
    
    // Custom/Other
    Custom(String), // Custom provider name
    Local(String), // Local model name
}

/// Model-specific tokenization configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub provider: ModelProvider,
    pub model_name: String,
    pub max_context_tokens: usize,
    pub cost_per_input_token: f64,
    pub cost_per_output_token: f64,
    pub tokenizer_type: TokenizerType,
    pub special_tokens: Vec<String>,
}

/// Different tokenization methods supported
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TokenizerType {
    GPT4, // cl100k_base
    Claude, // Custom Anthropic tokenizer
    Gemini, // SentencePiece
    Custom(String), // Custom tokenizer name
}

/// Token count result with detailed breakdown
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenCount {
    pub total_tokens: usize,
    pub input_tokens: usize,
    pub output_tokens: usize,
    pub special_tokens: usize,
    pub estimated_cost: f64,
    pub model_config: ModelConfig,
}

/// Context optimization result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextOptimization {
    pub original_tokens: usize,
    pub optimized_tokens: usize,
    pub compression_ratio: f64,
    pub preserved_content: String,
    pub optimization_strategy: OptimizationStrategy,
}

/// Context compression strategies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OptimizationStrategy {
    Truncation,
    Summarization,
    KeywordExtraction,
    SemanticCompression,
    Hybrid,
}

/// Universal tokenizer for multi-model support
pub struct UniversalTokenizer {
    model_configs: Arc<RwLock<HashMap<String, ModelConfig>>>,
    tokenizer_cache: Arc<RwLock<HashMap<String, Box<dyn TokenizerEngine + Send + Sync>>>>,
    cost_tracker: Arc<RwLock<CostTracker>>,
}

/// Trait for different tokenizer implementations
pub trait TokenizerEngine: Send + Sync {
    fn count_tokens(&self, text: &str) -> Result<usize>;
    fn encode(&self, text: &str) -> Result<Vec<u32>>;
    fn decode(&self, tokens: &[u32]) -> Result<String>;
    fn get_special_tokens(&self) -> Vec<String>;
}

/// Cost tracking and optimization
#[derive(Debug, Default)]
pub struct CostTracker {
    total_input_tokens: usize,
    total_output_tokens: usize,
    total_cost: f64,
    model_usage: HashMap<String, ModelUsage>,
}

#[derive(Debug, Default, Clone)]
pub struct ModelUsage {
    pub input_tokens: usize,
    pub output_tokens: usize,
    pub cost: f64,
    pub requests: usize,
}

impl UniversalTokenizer {
    pub fn new() -> Self {
        Self {
            model_configs: Arc::new(RwLock::new(HashMap::new())),
            tokenizer_cache: Arc::new(RwLock::new(HashMap::new())),
            cost_tracker: Arc::new(RwLock::new(CostTracker::default())),
        }
    }

    /// Initialize with default model configurations
    pub async fn initialize_default_models(&self) -> Result<()> {
        let mut configs = self.model_configs.write().await;
        
        // OpenAI Models (August 2025 - Latest)
        configs.insert("gpt-4o-2025-08".to_string(), ModelConfig {
            provider: ModelProvider::OpenAI,
            model_name: "gpt-4o-2025-08".to_string(),
            max_context_tokens: 200_000, // Increased context window
            cost_per_input_token: 0.002 / 1000.0, // Reduced pricing
            cost_per_output_token: 0.008 / 1000.0,
            tokenizer_type: TokenizerType::GPT4,
            special_tokens: vec!["<|im_start|>".to_string(), "<|im_end|>".to_string()],
        });

        configs.insert("o3-mini".to_string(), ModelConfig {
            provider: ModelProvider::OpenAI,
            model_name: "o3-mini".to_string(),
            max_context_tokens: 128_000,
            cost_per_input_token: 0.001 / 1000.0, // Very competitive pricing
            cost_per_output_token: 0.004 / 1000.0,
            tokenizer_type: TokenizerType::GPT4,
            special_tokens: vec![],
        });

        configs.insert("o3-max".to_string(), ModelConfig {
            provider: ModelProvider::OpenAI,
            model_name: "o3-max".to_string(),
            max_context_tokens: 1_000_000, // Massive context window
            cost_per_input_token: 0.02 / 1000.0,
            cost_per_output_token: 0.08 / 1000.0,
            tokenizer_type: TokenizerType::GPT4,
            special_tokens: vec![],
        });

        // Anthropic Models (August 2025 - Latest)
        configs.insert("claude-3-5-sonnet-20250801".to_string(), ModelConfig {
            provider: ModelProvider::Anthropic,
            model_name: "claude-3-5-sonnet-20250801".to_string(),
            max_context_tokens: 500_000, // Significantly increased context
            cost_per_input_token: 0.0025 / 1000.0, // Reduced pricing
            cost_per_output_token: 0.012 / 1000.0,
            tokenizer_type: TokenizerType::Claude,
            special_tokens: vec!["<|im_start|>".to_string(), "<|im_end|>".to_string()],
        });

        configs.insert("claude-4-opus".to_string(), ModelConfig {
            provider: ModelProvider::Anthropic,
            model_name: "claude-4-opus".to_string(),
            max_context_tokens: 1_000_000, // Massive context window
            cost_per_input_token: 0.015 / 1000.0,
            cost_per_output_token: 0.075 / 1000.0,
            tokenizer_type: TokenizerType::Claude,
            special_tokens: vec!["<|im_start|>".to_string(), "<|im_end|>".to_string()],
        });

        configs.insert("claude-3-5-haiku-20250715".to_string(), ModelConfig {
            provider: ModelProvider::Anthropic,
            model_name: "claude-3-5-haiku-20250715".to_string(),
            max_context_tokens: 200_000,
            cost_per_input_token: 0.0008 / 1000.0, // Very cost-effective
            cost_per_output_token: 0.004 / 1000.0,
            tokenizer_type: TokenizerType::Claude,
            special_tokens: vec!["<|im_start|>".to_string(), "<|im_end|>".to_string()],
        });

        // Google Models (August 2025 - Latest)
        configs.insert("gemini-2.5-pro".to_string(), ModelConfig {
            provider: ModelProvider::Google,
            model_name: "gemini-2.5-pro".to_string(),
            max_context_tokens: 2_000_000, // Massive 2M context window
            cost_per_input_token: 0.0001 / 1000.0, // Extremely competitive
            cost_per_output_token: 0.0004 / 1000.0,
            tokenizer_type: TokenizerType::Gemini,
            special_tokens: vec![],
        });

        configs.insert("gemini-2.0-flash-8b".to_string(), ModelConfig {
            provider: ModelProvider::Google,
            model_name: "gemini-2.0-flash-8b".to_string(),
            max_context_tokens: 1_048_576,
            cost_per_input_token: 0.00005 / 1000.0, // Ultra-low cost
            cost_per_output_token: 0.0002 / 1000.0,
            tokenizer_type: TokenizerType::Gemini,
            special_tokens: vec![],
        });

        configs.insert("gemini-3.0-ultra".to_string(), ModelConfig {
            provider: ModelProvider::Google,
            model_name: "gemini-3.0-ultra".to_string(),
            max_context_tokens: 10_000_000, // Revolutionary 10M context
            cost_per_input_token: 0.01 / 1000.0,
            cost_per_output_token: 0.03 / 1000.0,
            tokenizer_type: TokenizerType::Gemini,
            special_tokens: vec![],
        });

        // Google Vertex AI Models (Enterprise Platform)
        configs.insert("vertex-ai/gemini-2.5-pro".to_string(), ModelConfig {
            provider: ModelProvider::GCP, // Using GCP provider for Vertex AI
            model_name: "vertex-ai/gemini-2.5-pro".to_string(),
            max_context_tokens: 2_000_000, // 2M context via Vertex AI
            cost_per_input_token: 0.00012 / 1000.0, // Enterprise pricing
            cost_per_output_token: 0.00048 / 1000.0,
            tokenizer_type: TokenizerType::Gemini,
            special_tokens: vec![],
        });

        configs.insert("vertex-ai/gemini-1.5-pro".to_string(), ModelConfig {
            provider: ModelProvider::GCP,
            model_name: "vertex-ai/gemini-1.5-pro".to_string(),
            max_context_tokens: 2_000_000, // 2M context
            cost_per_input_token: 0.0035 / 1000.0,
            cost_per_output_token: 0.0105 / 1000.0,
            tokenizer_type: TokenizerType::Gemini,
            special_tokens: vec![],
        });

        configs.insert("vertex-ai/gemini-1.5-flash".to_string(), ModelConfig {
            provider: ModelProvider::GCP,
            model_name: "vertex-ai/gemini-1.5-flash".to_string(),
            max_context_tokens: 1_000_000, // 1M context
            cost_per_input_token: 0.000075 / 1000.0, // Very competitive
            cost_per_output_token: 0.0003 / 1000.0,
            tokenizer_type: TokenizerType::Gemini,
            special_tokens: vec![],
        });

        configs.insert("vertex-ai/text-bison".to_string(), ModelConfig {
            provider: ModelProvider::GCP,
            model_name: "vertex-ai/text-bison".to_string(),
            max_context_tokens: 8_192,
            cost_per_input_token: 0.001 / 1000.0,
            cost_per_output_token: 0.001 / 1000.0,
            tokenizer_type: TokenizerType::Custom("palm".to_string()),
            special_tokens: vec![],
        });

        configs.insert("vertex-ai/code-bison".to_string(), ModelConfig {
            provider: ModelProvider::GCP,
            model_name: "vertex-ai/code-bison".to_string(),
            max_context_tokens: 6_144, // Coding-optimized
            cost_per_input_token: 0.001 / 1000.0,
            cost_per_output_token: 0.001 / 1000.0,
            tokenizer_type: TokenizerType::Custom("palm".to_string()),
            special_tokens: vec![],
        });

        // Chinese Models (August 2025 - Latest)
        configs.insert("deepseek-v4-coder".to_string(), ModelConfig {
            provider: ModelProvider::DeepSeek,
            model_name: "deepseek-v4-coder".to_string(),
            max_context_tokens: 256_000, // Massive increase for coding
            cost_per_input_token: 0.0002 / 1000.0, // Even cheaper
            cost_per_output_token: 0.0008 / 1000.0,
            tokenizer_type: TokenizerType::Custom("deepseek".to_string()),
            special_tokens: vec![],
        });

        configs.insert("qwen-2.5-coder-32b".to_string(), ModelConfig {
            provider: ModelProvider::Qwen,
            model_name: "qwen-2.5-coder-32b".to_string(),
            max_context_tokens: 1_000_000, // 1M context for coding
            cost_per_input_token: 0.0003 / 1000.0,
            cost_per_output_token: 0.001 / 1000.0,
            tokenizer_type: TokenizerType::Custom("qwen".to_string()),
            special_tokens: vec![],
        });

        configs.insert("moonshot-v2-128k".to_string(), ModelConfig {
            provider: ModelProvider::Moonshot,
            model_name: "moonshot-v2-128k".to_string(),
            max_context_tokens: 128_000,
            cost_per_input_token: 0.0005 / 1000.0,
            cost_per_output_token: 0.002 / 1000.0,
            tokenizer_type: TokenizerType::Custom("moonshot".to_string()),
            special_tokens: vec![],
        });

        configs.insert("glm-4-plus".to_string(), ModelConfig {
            provider: ModelProvider::GLM,
            model_name: "glm-4-plus".to_string(),
            max_context_tokens: 512_000, // Half million context
            cost_per_input_token: 0.0004 / 1000.0,
            cost_per_output_token: 0.0015 / 1000.0,
            tokenizer_type: TokenizerType::Custom("glm".to_string()),
            special_tokens: vec![],
        });

        // Cutting-Edge Open Source Models (OpenRouter API Names)
        configs.insert("qwen/qwen-3-coder".to_string(), ModelConfig {
            provider: ModelProvider::Qwen,
            model_name: "qwen/qwen-3-coder".to_string(),
            max_context_tokens: 2_000_000, // 2M context for massive codebases
            cost_per_input_token: 0.0001 / 1000.0, // Extremely competitive
            cost_per_output_token: 0.0005 / 1000.0,
            tokenizer_type: TokenizerType::Custom("qwen3".to_string()),
            special_tokens: vec![],
        });

        configs.insert("zhipuai/glm-4.5".to_string(), ModelConfig {
            provider: ModelProvider::GLM,
            model_name: "zhipuai/glm-4.5".to_string(),
            max_context_tokens: 1_000_000, // 1M context window
            cost_per_input_token: 0.0002 / 1000.0, // Very competitive
            cost_per_output_token: 0.0008 / 1000.0,
            tokenizer_type: TokenizerType::Custom("glm45".to_string()),
            special_tokens: vec![],
        });

        configs.insert("moonshot/kimi-k2".to_string(), ModelConfig {
            provider: ModelProvider::Moonshot, // Kimi is from Moonshot AI
            model_name: "moonshot/kimi-k2".to_string(),
            max_context_tokens: 2_000_000, // 2M context for long documents
            cost_per_input_token: 0.0003 / 1000.0,
            cost_per_output_token: 0.001 / 1000.0,
            tokenizer_type: TokenizerType::Custom("kimi".to_string()),
            special_tokens: vec![],
        });

        // Latest OpenAI Models (August 2025)
        configs.insert("gpt-4o-2025-08-06".to_string(), ModelConfig {
            provider: ModelProvider::OpenAI,
            model_name: "gpt-4o-2025-08-06".to_string(),
            max_context_tokens: 500_000, // Massive context increase
            cost_per_input_token: 0.0015 / 1000.0, // Further reduced pricing
            cost_per_output_token: 0.006 / 1000.0,
            tokenizer_type: TokenizerType::GPT4,
            special_tokens: vec!["<|im_start|>".to_string(), "<|im_end|>".to_string()],
        });

        configs.insert("o3-turbo".to_string(), ModelConfig {
            provider: ModelProvider::OpenAI,
            model_name: "o3-turbo".to_string(),
            max_context_tokens: 200_000,
            cost_per_input_token: 0.0008 / 1000.0, // Balanced speed/cost
            cost_per_output_token: 0.003 / 1000.0,
            tokenizer_type: TokenizerType::GPT4,
            special_tokens: vec![],
        });

        // Additional Popular Open Source Models
        configs.insert("deepseek-v4-chat".to_string(), ModelConfig {
            provider: ModelProvider::DeepSeek,
            model_name: "deepseek-v4-chat".to_string(),
            max_context_tokens: 512_000, // Half million context
            cost_per_input_token: 0.00015 / 1000.0, // Ultra-competitive
            cost_per_output_token: 0.0006 / 1000.0,
            tokenizer_type: TokenizerType::Custom("deepseek".to_string()),
            special_tokens: vec![],
        });

        configs.insert("qwen3-72b-instruct".to_string(), ModelConfig {
            provider: ModelProvider::Qwen,
            model_name: "qwen3-72b-instruct".to_string(),
            max_context_tokens: 1_000_000, // 1M context
            cost_per_input_token: 0.0004 / 1000.0,
            cost_per_output_token: 0.0012 / 1000.0,
            tokenizer_type: TokenizerType::Custom("qwen3".to_string()),
            special_tokens: vec![],
        });

        // OpenAI's First Open Source Models (GPT-OSS - August 2025)
        configs.insert("openai/gpt-oss-120b".to_string(), ModelConfig {
            provider: ModelProvider::OpenAI,
            model_name: "openai/gpt-oss-120b".to_string(),
            max_context_tokens: 200_000, // Large context window
            cost_per_input_token: 0.0005 / 1000.0, // Open source - very competitive
            cost_per_output_token: 0.002 / 1000.0,
            tokenizer_type: TokenizerType::GPT4,
            special_tokens: vec!["<|im_start|>".to_string(), "<|im_end|>".to_string()],
        });

        configs.insert("openai/gpt-oss-20b".to_string(), ModelConfig {
            provider: ModelProvider::OpenAI,
            model_name: "openai/gpt-oss-20b".to_string(),
            max_context_tokens: 128_000, // Optimized for edge devices
            cost_per_input_token: 0.0002 / 1000.0, // Ultra-competitive for edge
            cost_per_output_token: 0.0008 / 1000.0,
            tokenizer_type: TokenizerType::GPT4,
            special_tokens: vec!["<|im_start|>".to_string(), "<|im_end|>".to_string()],
        });

        // Mysterious Horizon Models (Rumored OpenAI GPT-5 Preview)
        configs.insert("horizon-alpha".to_string(), ModelConfig {
            provider: ModelProvider::OpenAI,
            model_name: "horizon-alpha".to_string(),
            max_context_tokens: 1_000_000, // Massive context for testing
            cost_per_input_token: 0.0 / 1000.0, // Free during testing phase
            cost_per_output_token: 0.0 / 1000.0,
            tokenizer_type: TokenizerType::GPT4,
            special_tokens: vec![],
        });

        configs.insert("horizon-beta".to_string(), ModelConfig {
            provider: ModelProvider::OpenAI,
            model_name: "horizon-beta".to_string(),
            max_context_tokens: 2_000_000, // Even larger context
            cost_per_input_token: 0.0 / 1000.0, // Free during testing phase
            cost_per_output_token: 0.0 / 1000.0,
            tokenizer_type: TokenizerType::GPT4,
            special_tokens: vec![],
        });

        // xAI Grok Models
        configs.insert("xai/grok-3".to_string(), ModelConfig {
            provider: ModelProvider::xAI,
            model_name: "xai/grok-3".to_string(),
            max_context_tokens: 1_000_000, // 1M context
            cost_per_input_token: 0.001 / 1000.0,
            cost_per_output_token: 0.004 / 1000.0,
            tokenizer_type: TokenizerType::Custom("grok".to_string()),
            special_tokens: vec![],
        });

        // Mistral Models
        configs.insert("mistral/mistral-large-2".to_string(), ModelConfig {
            provider: ModelProvider::Mistral,
            model_name: "mistral/mistral-large-2".to_string(),
            max_context_tokens: 128_000,
            cost_per_input_token: 0.003 / 1000.0,
            cost_per_output_token: 0.009 / 1000.0,
            tokenizer_type: TokenizerType::Custom("mistral".to_string()),
            special_tokens: vec![],
        });

        configs.insert("mistral/codestral".to_string(), ModelConfig {
            provider: ModelProvider::Mistral,
            model_name: "mistral/codestral".to_string(),
            max_context_tokens: 32_000,
            cost_per_input_token: 0.001 / 1000.0, // Coding-optimized pricing
            cost_per_output_token: 0.003 / 1000.0,
            tokenizer_type: TokenizerType::Custom("mistral".to_string()),
            special_tokens: vec![],
        });

        // Cohere Models
        configs.insert("cohere/command-r-plus".to_string(), ModelConfig {
            provider: ModelProvider::Cohere,
            model_name: "cohere/command-r-plus".to_string(),
            max_context_tokens: 128_000,
            cost_per_input_token: 0.003 / 1000.0,
            cost_per_output_token: 0.015 / 1000.0,
            tokenizer_type: TokenizerType::Custom("cohere".to_string()),
            special_tokens: vec![],
        });

        // Meta Llama Models
        configs.insert("meta/llama-3.1-405b".to_string(), ModelConfig {
            provider: ModelProvider::Meta,
            model_name: "meta/llama-3.1-405b".to_string(),
            max_context_tokens: 128_000,
            cost_per_input_token: 0.005 / 1000.0, // Large model pricing
            cost_per_output_token: 0.015 / 1000.0,
            tokenizer_type: TokenizerType::Custom("llama".to_string()),
            special_tokens: vec![],
        });

        configs.insert("meta/llama-3.1-70b".to_string(), ModelConfig {
            provider: ModelProvider::Meta,
            model_name: "meta/llama-3.1-70b".to_string(),
            max_context_tokens: 128_000,
            cost_per_input_token: 0.0009 / 1000.0, // More affordable
            cost_per_output_token: 0.0027 / 1000.0,
            tokenizer_type: TokenizerType::Custom("llama".to_string()),
            special_tokens: vec![],
        });

        // Perplexity Models
        configs.insert("perplexity/llama-3.1-sonar-large".to_string(), ModelConfig {
            provider: ModelProvider::Perplexity,
            model_name: "perplexity/llama-3.1-sonar-large".to_string(),
            max_context_tokens: 127_072,
            cost_per_input_token: 0.001 / 1000.0, // Search-optimized pricing
            cost_per_output_token: 0.001 / 1000.0,
            tokenizer_type: TokenizerType::Custom("perplexity".to_string()),
            special_tokens: vec![],
        });

        // Groq Fast Inference Models
        configs.insert("groq/llama-3.1-70b-versatile".to_string(), ModelConfig {
            provider: ModelProvider::Groq,
            model_name: "groq/llama-3.1-70b-versatile".to_string(),
            max_context_tokens: 131_072,
            cost_per_input_token: 0.00059 / 1000.0, // Fast inference pricing
            cost_per_output_token: 0.00079 / 1000.0,
            tokenizer_type: TokenizerType::Custom("llama".to_string()),
            special_tokens: vec![],
        });

        // Together AI Models
        configs.insert("together/qwen-2.5-coder-32b-instruct".to_string(), ModelConfig {
            provider: ModelProvider::TogetherAI,
            model_name: "together/qwen-2.5-coder-32b-instruct".to_string(),
            max_context_tokens: 32_768,
            cost_per_input_token: 0.0008 / 1000.0,
            cost_per_output_token: 0.0008 / 1000.0,
            tokenizer_type: TokenizerType::Custom("qwen".to_string()),
            special_tokens: vec![],
        });

        // Local/Self-Hosted Models (Free)
        configs.insert("ollama/llama3.1:70b".to_string(), ModelConfig {
            provider: ModelProvider::Ollama,
            model_name: "ollama/llama3.1:70b".to_string(),
            max_context_tokens: 128_000,
            cost_per_input_token: 0.0, // Free for local
            cost_per_output_token: 0.0,
            tokenizer_type: TokenizerType::Custom("llama".to_string()),
            special_tokens: vec![],
        });

        configs.insert("ollama/qwen2.5-coder:32b".to_string(), ModelConfig {
            provider: ModelProvider::Ollama,
            model_name: "ollama/qwen2.5-coder:32b".to_string(),
            max_context_tokens: 32_768,
            cost_per_input_token: 0.0, // Free for local
            cost_per_output_token: 0.0,
            tokenizer_type: TokenizerType::Custom("qwen".to_string()),
            special_tokens: vec![],
        });

        Ok(())
    }

    /// Count tokens for a specific model
    pub async fn count_tokens(&self, model_name: &str, text: &str) -> Result<TokenCount> {
        let configs = self.model_configs.read().await;
        let config = configs.get(model_name)
            .ok_or_else(|| anyhow!("Model '{}' not found", model_name))?;

        let tokenizer = self.get_tokenizer(&config.tokenizer_type).await?;
        let total_tokens = tokenizer.count_tokens(text)?;

        // For now, assume all tokens are input tokens
        // In practice, this would be split based on context
        let input_tokens = total_tokens;
        let output_tokens = 0;
        let special_tokens = self.count_special_tokens(text, &config.special_tokens);

        let estimated_cost = (input_tokens as f64 * config.cost_per_input_token) +
                           (output_tokens as f64 * config.cost_per_output_token);

        Ok(TokenCount {
            total_tokens,
            input_tokens,
            output_tokens,
            special_tokens,
            estimated_cost,
            model_config: config.clone(),
        })
    }

    /// Optimize context for a specific model's token limits
    pub async fn optimize_context(
        &self,
        model_name: &str,
        content: &str,
        target_tokens: Option<usize>,
    ) -> Result<ContextOptimization> {
        let configs = self.model_configs.read().await;
        let config = configs.get(model_name)
            .ok_or_else(|| anyhow!("Model '{}' not found", model_name))?;

        let tokenizer = self.get_tokenizer(&config.tokenizer_type).await?;
        let original_tokens = tokenizer.count_tokens(content)?;
        
        let target = target_tokens.unwrap_or(config.max_context_tokens * 80 / 100); // 80% of max

        if original_tokens <= target {
            return Ok(ContextOptimization {
                original_tokens,
                optimized_tokens: original_tokens,
                compression_ratio: 1.0,
                preserved_content: content.to_string(),
                optimization_strategy: OptimizationStrategy::Truncation,
            });
        }

        // Apply optimization strategy
        let (optimized_content, strategy) = self.apply_optimization_strategy(
            content,
            original_tokens,
            target,
            &config.provider,
        ).await?;

        let optimized_tokens = tokenizer.count_tokens(&optimized_content)?;
        let compression_ratio = optimized_tokens as f64 / original_tokens as f64;

        Ok(ContextOptimization {
            original_tokens,
            optimized_tokens,
            compression_ratio,
            preserved_content: optimized_content,
            optimization_strategy: strategy,
        })
    }

    /// Get cost estimate for a conversation
    pub async fn estimate_conversation_cost(
        &self,
        model_name: &str,
        input_text: &str,
        expected_output_tokens: usize,
    ) -> Result<f64> {
        let token_count = self.count_tokens(model_name, input_text).await?;
        let config = &token_count.model_config;

        let input_cost = token_count.input_tokens as f64 * config.cost_per_input_token;
        let output_cost = expected_output_tokens as f64 * config.cost_per_output_token;

        Ok(input_cost + output_cost)
    }

    /// Track actual usage for cost monitoring
    pub async fn track_usage(
        &self,
        model_name: &str,
        input_tokens: usize,
        output_tokens: usize,
    ) -> Result<()> {
        let configs = self.model_configs.read().await;
        let config = configs.get(model_name)
            .ok_or_else(|| anyhow!("Model '{}' not found", model_name))?;

        let cost = (input_tokens as f64 * config.cost_per_input_token) +
                  (output_tokens as f64 * config.cost_per_output_token);

        let mut tracker = self.cost_tracker.write().await;
        tracker.total_input_tokens += input_tokens;
        tracker.total_output_tokens += output_tokens;
        tracker.total_cost += cost;

        let usage = tracker.model_usage.entry(model_name.to_string()).or_default();
        usage.input_tokens += input_tokens;
        usage.output_tokens += output_tokens;
        usage.cost += cost;
        usage.requests += 1;

        Ok(())
    }

    /// Get usage statistics
    pub async fn get_usage_stats(&self) -> Result<CostTracker> {
        let tracker = self.cost_tracker.read().await;
        Ok(tracker.clone())
    }

    /// Add custom model configuration
    pub async fn add_model_config(&self, model_name: String, config: ModelConfig) -> Result<()> {
        let mut configs = self.model_configs.write().await;
        configs.insert(model_name, config);
        Ok(())
    }

    /// Get available models
    pub async fn get_available_models(&self) -> Result<Vec<String>> {
        let configs = self.model_configs.read().await;
        Ok(configs.keys().cloned().collect())
    }

    /// Compare costs across models for the same input
    pub async fn compare_model_costs(&self, input_text: &str, expected_output_tokens: usize) -> Result<Vec<(String, f64)>> {
        let models = self.get_available_models().await?;
        let mut costs = Vec::new();

        for model in models {
            if let Ok(cost) = self.estimate_conversation_cost(&model, input_text, expected_output_tokens).await {
                costs.push((model, cost));
            }
        }

        // Sort by cost (ascending)
        costs.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        Ok(costs)
    }

    // Private helper methods

    async fn get_tokenizer(&self, tokenizer_type: &TokenizerType) -> Result<Arc<dyn TokenizerEngine + Send + Sync>> {
        let cache = self.tokenizer_cache.read().await;
        let key = format!("{:?}", tokenizer_type);
        
        if let Some(tokenizer) = cache.get(&key) {
            // Note: This is a simplified approach. In practice, we'd need to handle
            // the Arc<dyn Trait> cloning properly or use a different caching strategy
            return Err(anyhow!("Tokenizer caching needs proper Arc handling"));
        }

        drop(cache);

        // Create new tokenizer
        let tokenizer = self.create_tokenizer(tokenizer_type).await?;
        let mut cache = self.tokenizer_cache.write().await;
        cache.insert(key, tokenizer.clone());
        
        Ok(tokenizer)
    }

    async fn create_tokenizer(&self, tokenizer_type: &TokenizerType) -> Result<Box<dyn TokenizerEngine + Send + Sync>> {
        match tokenizer_type {
            TokenizerType::GPT4 => Ok(Box::new(GPT4Tokenizer::new()?)),
            TokenizerType::Claude => Ok(Box::new(ClaudeTokenizer::new()?)),
            TokenizerType::Gemini => Ok(Box::new(GeminiTokenizer::new()?)),
            TokenizerType::Custom(name) => Ok(Box::new(CustomTokenizer::new(name)?)),
        }
    }

    fn count_special_tokens(&self, text: &str, special_tokens: &[String]) -> usize {
        special_tokens.iter()
            .map(|token| text.matches(token).count())
            .sum()
    }

    async fn apply_optimization_strategy(
        &self,
        content: &str,
        original_tokens: usize,
        target_tokens: usize,
        provider: &ModelProvider,
    ) -> Result<(String, OptimizationStrategy)> {
        // For now, implement simple truncation
        // In practice, this would use more sophisticated strategies
        let compression_ratio = target_tokens as f64 / original_tokens as f64;
        let target_length = (content.len() as f64 * compression_ratio) as usize;
        
        let optimized = if target_length < content.len() {
            content.chars().take(target_length).collect()
        } else {
            content.to_string()
        };

        Ok((optimized, OptimizationStrategy::Truncation))
    }
}

// Tokenizer implementations (simplified for now)

pub struct GPT4Tokenizer;
impl GPT4Tokenizer {
    pub fn new() -> Result<Self> {
        Ok(Self)
    }
}

impl TokenizerEngine for GPT4Tokenizer {
    fn count_tokens(&self, text: &str) -> Result<usize> {
        // Simplified approximation: ~4 characters per token for English
        Ok(text.len() / 4)
    }

    fn encode(&self, text: &str) -> Result<Vec<u32>> {
        // Simplified implementation
        Ok(text.chars().map(|c| c as u32).collect())
    }

    fn decode(&self, tokens: &[u32]) -> Result<String> {
        // Simplified implementation
        Ok(tokens.iter().map(|&t| char::from_u32(t).unwrap_or('?')).collect())
    }

    fn get_special_tokens(&self) -> Vec<String> {
        vec!["<|im_start|>".to_string(), "<|im_end|>".to_string()]
    }
}

pub struct ClaudeTokenizer;
impl ClaudeTokenizer {
    pub fn new() -> Result<Self> {
        Ok(Self)
    }
}

impl TokenizerEngine for ClaudeTokenizer {
    fn count_tokens(&self, text: &str) -> Result<usize> {
        // Simplified approximation: ~3.5 characters per token for English
        Ok((text.len() as f64 / 3.5) as usize)
    }

    fn encode(&self, text: &str) -> Result<Vec<u32>> {
        Ok(text.chars().map(|c| c as u32).collect())
    }

    fn decode(&self, tokens: &[u32]) -> Result<String> {
        Ok(tokens.iter().map(|&t| char::from_u32(t).unwrap_or('?')).collect())
    }

    fn get_special_tokens(&self) -> Vec<String> {
        vec!["<|im_start|>".to_string(), "<|im_end|>".to_string()]
    }
}

pub struct GeminiTokenizer;
impl GeminiTokenizer {
    pub fn new() -> Result<Self> {
        Ok(Self)
    }
}

impl TokenizerEngine for GeminiTokenizer {
    fn count_tokens(&self, text: &str) -> Result<usize> {
        // Simplified approximation: ~3 characters per token for English
        Ok(text.len() / 3)
    }

    fn encode(&self, text: &str) -> Result<Vec<u32>> {
        Ok(text.chars().map(|c| c as u32).collect())
    }

    fn decode(&self, tokens: &[u32]) -> Result<String> {
        Ok(tokens.iter().map(|&t| char::from_u32(t).unwrap_or('?')).collect())
    }

    fn get_special_tokens(&self) -> Vec<String> {
        vec![]
    }
}

pub struct CustomTokenizer {
    name: String,
}

impl CustomTokenizer {
    pub fn new(name: &str) -> Result<Self> {
        Ok(Self { name: name.to_string() })
    }
}

impl TokenizerEngine for CustomTokenizer {
    fn count_tokens(&self, text: &str) -> Result<usize> {
        // Default approximation
        Ok(text.len() / 4)
    }

    fn encode(&self, text: &str) -> Result<Vec<u32>> {
        Ok(text.chars().map(|c| c as u32).collect())
    }

    fn decode(&self, tokens: &[u32]) -> Result<String> {
        Ok(tokens.iter().map(|&t| char::from_u32(t).unwrap_or('?')).collect())
    }

    fn get_special_tokens(&self) -> Vec<String> {
        vec![]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_universal_tokenizer_initialization() {
        let tokenizer = UniversalTokenizer::new();
        tokenizer.initialize_default_models().await.unwrap();
        
        let models = tokenizer.get_available_models().await.unwrap();
        assert!(!models.is_empty());
        assert!(models.contains(&"gpt-4o".to_string()));
        assert!(models.contains(&"claude-3-5-sonnet-20241022".to_string()));
    }

    #[tokio::test]
    async fn test_token_counting() {
        let tokenizer = UniversalTokenizer::new();
        tokenizer.initialize_default_models().await.unwrap();
        
        let test_text = "Hello, world! This is a test message.";
        let result = tokenizer.count_tokens("gpt-4o", test_text).await.unwrap();
        
        assert!(result.total_tokens > 0);
        assert!(result.estimated_cost > 0.0);
        assert_eq!(result.model_config.model_name, "gpt-4o");
    }

    #[tokio::test]
    async fn test_cost_comparison() {
        let tokenizer = UniversalTokenizer::new();
        tokenizer.initialize_default_models().await.unwrap();
        
        let test_text = "This is a test message for cost comparison.";
        let costs = tokenizer.compare_model_costs(test_text, 100).await.unwrap();
        
        assert!(!costs.is_empty());
        // Should be sorted by cost
        for i in 1..costs.len() {
            assert!(costs[i-1].1 <= costs[i].1);
        }
    }

    #[tokio::test]
    async fn test_usage_tracking() {
        let tokenizer = UniversalTokenizer::new();
        tokenizer.initialize_default_models().await.unwrap();
        
        tokenizer.track_usage("gpt-4o", 100, 50).await.unwrap();
        tokenizer.track_usage("gpt-4o", 200, 75).await.unwrap();
        
        let stats = tokenizer.get_usage_stats().await.unwrap();
        assert_eq!(stats.total_input_tokens, 300);
        assert_eq!(stats.total_output_tokens, 125);
        assert!(stats.total_cost > 0.0);
        
        let gpt4_usage = stats.model_usage.get("gpt-4o").unwrap();
        assert_eq!(gpt4_usage.requests, 2);
    }

    #[tokio::test]
    async fn test_context_optimization() {
        let tokenizer = UniversalTokenizer::new();
        tokenizer.initialize_default_models().await.unwrap();
        
        let long_text = "This is a very long text that needs to be optimized. ".repeat(1000);
        let result = tokenizer.optimize_context("gpt-4o", &long_text, Some(100)).await.unwrap();
        
        assert!(result.optimized_tokens <= 100);
        assert!(result.compression_ratio < 1.0);
        assert!(!result.preserved_content.is_empty());
    }
}
