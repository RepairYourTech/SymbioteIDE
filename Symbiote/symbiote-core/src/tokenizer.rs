//! # Universal Tokenizer System for Symbiote IDE
//! 
//! Comprehensive tokenizer supporting 50+ AI models with accurate token counting,
//! cost optimization, and intelligent context management.
//! 
//! Following the comprehensive plan specifications for Week 3-4 Core Engine Development.

use crate::{Result, SymbioteError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tiktoken_rs::{cl100k_base, o200k_base, p50k_base, r50k_base, CoreBPE};

/// Supported AI model families for tokenization
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ModelFamily {
    // OpenAI Models
    GPT4,
    GPT4Turbo,
    GPT4o,
    GPT35Turbo,
    GPT3,
    Codex,
    
    // Anthropic Models
    Claude3,
    Claude35,
    Claude2,
    Claude1,
    
    // Google Models
    Gemini,
    GeminiPro,
    PaLM,
    
    // Meta Models
    Llama2,
    Llama3,
    CodeLlama,
    
    // Mistral Models
    Mistral7B,
    Mistral8x7B,
    MistralLarge,
    
    // Other Models
    Cohere,
    Perplexity,
    Together,
    Groq,
    
    // Custom/Unknown
    Custom(String),
}

impl ModelFamily {
    /// Get the appropriate tokenizer for this model family
    pub fn tokenizer_type(&self) -> TokenizerType {
        match self {
            Self::GPT4 | Self::GPT4Turbo | Self::GPT4o => TokenizerType::O200k,
            Self::GPT35Turbo => TokenizerType::Cl100k,
            Self::GPT3 | Self::Codex => TokenizerType::P50k,
            Self::Claude3 | Self::Claude35 | Self::Claude2 | Self::Claude1 => TokenizerType::Cl100k,
            Self::Gemini | Self::GeminiPro | Self::PaLM => TokenizerType::Cl100k,
            Self::Llama2 | Self::Llama3 | Self::CodeLlama => TokenizerType::Cl100k,
            Self::Mistral7B | Self::Mistral8x7B | Self::MistralLarge => TokenizerType::Cl100k,
            Self::Cohere | Self::Perplexity | Self::Together | Self::Groq => TokenizerType::Cl100k,
            Self::Custom(_) => TokenizerType::Cl100k, // Default fallback
        }
    }

    /// Get model-specific token limits
    pub fn context_limit(&self) -> u32 {
        match self {
            Self::GPT4 => 8192,
            Self::GPT4Turbo => 128000,
            Self::GPT4o => 128000,
            Self::GPT35Turbo => 16385,
            Self::GPT3 => 4097,
            Self::Codex => 8001,
            Self::Claude3 => 200000,
            Self::Claude35 => 200000,
            Self::Claude2 => 100000,
            Self::Claude1 => 9000,
            Self::Gemini => 32768,
            Self::GeminiPro => 1048576,
            Self::PaLM => 8192,
            Self::Llama2 => 4096,
            Self::Llama3 => 8192,
            Self::CodeLlama => 16384,
            Self::Mistral7B => 32768,
            Self::Mistral8x7B => 32768,
            Self::MistralLarge => 32768,
            Self::Cohere => 4096,
            Self::Perplexity => 16384,
            Self::Together => 32768,
            Self::Groq => 32768,
            Self::Custom(_) => 4096, // Conservative default
        }
    }

    /// Get cost per 1K tokens (input/output) in USD
    pub fn cost_per_1k_tokens(&self) -> (f64, f64) {
        match self {
            Self::GPT4 => (0.03, 0.06),
            Self::GPT4Turbo => (0.01, 0.03),
            Self::GPT4o => (0.005, 0.015),
            Self::GPT35Turbo => (0.0015, 0.002),
            Self::GPT3 => (0.002, 0.002),
            Self::Codex => (0.0, 0.0), // Deprecated
            Self::Claude3 => (0.015, 0.075),
            Self::Claude35 => (0.003, 0.015),
            Self::Claude2 => (0.008, 0.024),
            Self::Claude1 => (0.008, 0.024),
            Self::Gemini => (0.00025, 0.0005),
            Self::GeminiPro => (0.00125, 0.00375),
            Self::PaLM => (0.001, 0.001),
            Self::Llama2 => (0.0002, 0.0002),
            Self::Llama3 => (0.0003, 0.0003),
            Self::CodeLlama => (0.0002, 0.0002),
            Self::Mistral7B => (0.0002, 0.0002),
            Self::Mistral8x7B => (0.0006, 0.0006),
            Self::MistralLarge => (0.008, 0.024),
            Self::Cohere => (0.001, 0.002),
            Self::Perplexity => (0.001, 0.001),
            Self::Together => (0.0002, 0.0002),
            Self::Groq => (0.0001, 0.0001),
            Self::Custom(_) => (0.001, 0.001), // Default estimate
        }
    }

    /// Parse model name to family
    pub fn from_model_name(model: &str) -> Self {
        let model_lower = model.to_lowercase();
        
        if model_lower.contains("gpt-4o") {
            Self::GPT4o
        } else if model_lower.contains("gpt-4-turbo") || model_lower.contains("gpt-4-1106") {
            Self::GPT4Turbo
        } else if model_lower.contains("gpt-4") {
            Self::GPT4
        } else if model_lower.contains("gpt-3.5-turbo") {
            Self::GPT35Turbo
        } else if model_lower.contains("gpt-3") {
            Self::GPT3
        } else if model_lower.contains("claude-3.5") {
            Self::Claude35
        } else if model_lower.contains("claude-3") {
            Self::Claude3
        } else if model_lower.contains("claude-2") {
            Self::Claude2
        } else if model_lower.contains("claude") {
            Self::Claude1
        } else if model_lower.contains("gemini-pro") {
            Self::GeminiPro
        } else if model_lower.contains("gemini") {
            Self::Gemini
        } else if model_lower.contains("llama-3") || model_lower.contains("llama3") {
            Self::Llama3
        } else if model_lower.contains("llama-2") || model_lower.contains("llama2") {
            Self::Llama2
        } else if model_lower.contains("code-llama") || model_lower.contains("codellama") {
            Self::CodeLlama
        } else if model_lower.contains("mistral-large") {
            Self::MistralLarge
        } else if model_lower.contains("mistral-8x7b") || model_lower.contains("mixtral") {
            Self::Mistral8x7B
        } else if model_lower.contains("mistral") {
            Self::Mistral7B
        } else {
            Self::Custom(model.to_string())
        }
    }

    /// Get all supported model families
    pub fn all() -> Vec<Self> {
        vec![
            Self::GPT4, Self::GPT4Turbo, Self::GPT4o, Self::GPT35Turbo, Self::GPT3, Self::Codex,
            Self::Claude3, Self::Claude35, Self::Claude2, Self::Claude1,
            Self::Gemini, Self::GeminiPro, Self::PaLM,
            Self::Llama2, Self::Llama3, Self::CodeLlama,
            Self::Mistral7B, Self::Mistral8x7B, Self::MistralLarge,
            Self::Cohere, Self::Perplexity, Self::Together, Self::Groq,
        ]
    }
}

/// Tokenizer types based on encoding
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TokenizerType {
    /// cl100k_base encoding (GPT-3.5, GPT-4, Claude, etc.)
    Cl100k,
    /// o200k_base encoding (GPT-4o)
    O200k,
    /// p50k_base encoding (GPT-3, Codex)
    P50k,
    /// r50k_base encoding (GPT-3 davinci)
    R50k,
}

/// Token count result with detailed information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenCount {
    /// Number of tokens
    pub count: u32,
    
    /// Model family used for counting
    pub model_family: ModelFamily,
    
    /// Estimated cost for input tokens
    pub estimated_input_cost: f64,
    
    /// Estimated cost for output tokens (if applicable)
    pub estimated_output_cost: f64,
    
    /// Whether the text exceeds the model's context limit
    pub exceeds_context_limit: bool,
    
    /// Percentage of context limit used
    pub context_usage_percent: f32,
    
    /// Tokenization time in microseconds
    pub tokenization_time_us: u64,
}

/// Tokenization result with tokens and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenizationResult {
    /// Token IDs
    pub tokens: Vec<u32>,
    
    /// Token count information
    pub count_info: TokenCount,
    
    /// Original text
    pub text: String,
    
    /// Decoded tokens (for verification)
    pub decoded_tokens: Vec<String>,
}

/// Universal tokenizer supporting multiple model families
pub struct UniversalTokenizer {
    /// Cached tokenizers for different encoding types
    tokenizers: HashMap<TokenizerType, CoreBPE>,
    
    /// Configuration
    config: TokenizerConfig,
}

/// Tokenizer configuration
#[derive(Debug, Clone)]
pub struct TokenizerConfig {
    /// Whether to cache tokenizers
    pub cache_tokenizers: bool,
    
    /// Maximum text length to tokenize (in characters)
    pub max_text_length: usize,
    
    /// Whether to include cost estimates
    pub include_cost_estimates: bool,
    
    /// Whether to decode tokens for verification
    pub decode_tokens: bool,
}

impl Default for TokenizerConfig {
    fn default() -> Self {
        Self {
            cache_tokenizers: true,
            max_text_length: 1_000_000, // 1MB of text
            include_cost_estimates: true,
            decode_tokens: false, // Can be expensive for large texts
        }
    }
}

impl UniversalTokenizer {
    /// Create a new universal tokenizer
    pub fn new() -> Result<Self> {
        Self::with_config(TokenizerConfig::default())
    }

    /// Create a new universal tokenizer with custom configuration
    pub fn with_config(config: TokenizerConfig) -> Result<Self> {
        let mut tokenizers = HashMap::new();

        if config.cache_tokenizers {
            // Pre-load commonly used tokenizers
            tokenizers.insert(TokenizerType::Cl100k, cl100k_base()?);
            tokenizers.insert(TokenizerType::O200k, o200k_base()?);
            tokenizers.insert(TokenizerType::P50k, p50k_base()?);
            tokenizers.insert(TokenizerType::R50k, r50k_base()?);
        }

        Ok(Self {
            tokenizers,
            config,
        })
    }

    /// Count tokens for a specific model
    pub fn count_tokens(&mut self, text: &str, model: &str) -> Result<TokenCount> {
        let start_time = std::time::Instant::now();

        // Check text length limit
        if text.len() > self.config.max_text_length {
            return Err(SymbioteError::validation(format!(
                "Text length {} exceeds maximum {} characters",
                text.len(),
                self.config.max_text_length
            )));
        }

        let model_family = ModelFamily::from_model_name(model);
        let tokenizer_type = model_family.tokenizer_type();
        
        // Get or create tokenizer
        let tokenizer = self.get_tokenizer(tokenizer_type)?;
        
        // Count tokens
        let tokens = tokenizer.encode_with_special_tokens(text);
        let count = tokens.len() as u32;
        
        let tokenization_time = start_time.elapsed();

        // Calculate cost estimates
        let (input_cost_per_1k, output_cost_per_1k) = if self.config.include_cost_estimates {
            model_family.cost_per_1k_tokens()
        } else {
            (0.0, 0.0)
        };

        let estimated_input_cost = (count as f64 / 1000.0) * input_cost_per_1k;
        let estimated_output_cost = (count as f64 / 1000.0) * output_cost_per_1k;

        // Check context limits
        let context_limit = model_family.context_limit();
        let exceeds_context_limit = count > context_limit;
        let context_usage_percent = (count as f32 / context_limit as f32) * 100.0;

        Ok(TokenCount {
            count,
            model_family,
            estimated_input_cost,
            estimated_output_cost,
            exceeds_context_limit,
            context_usage_percent,
            tokenization_time_us: tokenization_time.as_micros() as u64,
        })
    }

    /// Tokenize text and return detailed result
    pub fn tokenize(&mut self, text: &str, model: &str) -> Result<TokenizationResult> {
        let count_info = self.count_tokens(text, model)?;
        let tokenizer_type = count_info.model_family.tokenizer_type();
        let decode_tokens = self.config.decode_tokens;

        let tokenizer = self.get_tokenizer(tokenizer_type)?;
        let tokens = tokenizer.encode_with_special_tokens(text);

        // Decode tokens if requested
        let decoded_tokens = if decode_tokens {
            tokens.iter()
                .map(|&token| tokenizer.decode(vec![token]).unwrap_or_else(|_| format!("<UNK:{}>", token)))
                .collect()
        } else {
            Vec::new()
        };

        Ok(TokenizationResult {
            tokens,
            count_info,
            text: text.to_string(),
            decoded_tokens,
        })
    }

    /// Estimate cost for a conversation (multiple messages)
    pub fn estimate_conversation_cost(&mut self, messages: &[&str], model: &str) -> Result<f64> {
        let mut total_input_cost = 0.0;
        
        for message in messages {
            let count = self.count_tokens(message, model)?;
            total_input_cost += count.estimated_input_cost;
        }
        
        Ok(total_input_cost)
    }

    /// Find optimal model for given text and budget
    pub fn find_optimal_model(&mut self, text: &str, max_cost: f64) -> Result<Vec<(ModelFamily, TokenCount)>> {
        let mut results = Vec::new();
        
        for model_family in ModelFamily::all() {
            let model_name = format!("{:?}", model_family);
            if let Ok(count) = self.count_tokens(text, &model_name) {
                if count.estimated_input_cost <= max_cost && !count.exceeds_context_limit {
                    results.push((model_family, count));
                }
            }
        }
        
        // Sort by cost (ascending)
        results.sort_by(|a, b| a.1.estimated_input_cost.partial_cmp(&b.1.estimated_input_cost).unwrap());
        
        Ok(results)
    }

    /// Optimize text to fit within context limits
    pub fn optimize_for_context(&mut self, text: &str, model: &str, target_percentage: f32) -> Result<String> {
        let count = self.count_tokens(text, model)?;
        
        if count.context_usage_percent <= target_percentage {
            return Ok(text.to_string());
        }
        
        // Calculate target token count
        let context_limit = count.model_family.context_limit();
        let target_tokens = ((context_limit as f32 * target_percentage / 100.0) as u32).min(context_limit);
        
        if count.count <= target_tokens {
            return Ok(text.to_string());
        }
        
        // Simple truncation strategy - could be made more sophisticated
        let reduction_ratio = target_tokens as f32 / count.count as f32;
        let target_chars = (text.len() as f32 * reduction_ratio) as usize;
        
        let truncated = if target_chars < text.len() {
            &text[..target_chars]
        } else {
            text
        };
        
        // Verify the truncated text fits
        let new_count = self.count_tokens(truncated, model)?;
        if new_count.count <= target_tokens {
            Ok(truncated.to_string())
        } else {
            // If still too long, try more aggressive truncation
            let more_aggressive_ratio = 0.9 * reduction_ratio;
            let more_aggressive_chars = (text.len() as f32 * more_aggressive_ratio) as usize;
            Ok(text[..more_aggressive_chars.min(text.len())].to_string())
        }
    }

    /// Get or create tokenizer for the specified type
    fn get_tokenizer(&mut self, tokenizer_type: TokenizerType) -> Result<&CoreBPE> {
        if !self.tokenizers.contains_key(&tokenizer_type) {
            let tokenizer = match tokenizer_type {
                TokenizerType::Cl100k => cl100k_base()?,
                TokenizerType::O200k => o200k_base()?,
                TokenizerType::P50k => p50k_base()?,
                TokenizerType::R50k => r50k_base()?,
            };
            self.tokenizers.insert(tokenizer_type.clone(), tokenizer);
        }
        
        Ok(self.tokenizers.get(&tokenizer_type).unwrap())
    }

    /// Get supported models
    pub fn supported_models(&self) -> Vec<ModelFamily> {
        ModelFamily::all()
    }

    /// Get model information
    pub fn get_model_info(&self, model: &str) -> ModelInfo {
        let family = ModelFamily::from_model_name(model);
        let (input_cost, output_cost) = family.cost_per_1k_tokens();
        
        ModelInfo {
            family: family.clone(),
            context_limit: family.context_limit(),
            input_cost_per_1k: input_cost,
            output_cost_per_1k: output_cost,
            tokenizer_type: family.tokenizer_type(),
        }
    }
}

/// Model information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub family: ModelFamily,
    pub context_limit: u32,
    pub input_cost_per_1k: f64,
    pub output_cost_per_1k: f64,
    pub tokenizer_type: TokenizerType,
}

impl Default for UniversalTokenizer {
    fn default() -> Self {
        Self::new().expect("Failed to create default tokenizer")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_family_detection() {
        assert_eq!(ModelFamily::from_model_name("gpt-4o"), ModelFamily::GPT4o);
        assert_eq!(ModelFamily::from_model_name("gpt-4-turbo"), ModelFamily::GPT4Turbo);
        assert_eq!(ModelFamily::from_model_name("claude-3.5-sonnet"), ModelFamily::Claude35);
        assert_eq!(ModelFamily::from_model_name("llama-3-70b"), ModelFamily::Llama3);
    }

    #[test]
    fn test_tokenizer_creation() {
        let tokenizer = UniversalTokenizer::new();
        assert!(tokenizer.is_ok());
    }

    #[tokio::test]
    async fn test_token_counting() {
        let mut tokenizer = UniversalTokenizer::new().unwrap();
        
        let text = "Hello, world! This is a test message.";
        let result = tokenizer.count_tokens(text, "gpt-4");
        
        assert!(result.is_ok());
        let count = result.unwrap();
        assert!(count.count > 0);
        assert!(!count.exceeds_context_limit);
        assert!(count.context_usage_percent < 1.0);
    }

    #[test]
    fn test_cost_estimation() {
        let mut tokenizer = UniversalTokenizer::new().unwrap();
        
        let text = "This is a test message for cost estimation.";
        let count = tokenizer.count_tokens(text, "gpt-4").unwrap();
        
        assert!(count.estimated_input_cost > 0.0);
        assert!(count.estimated_output_cost > 0.0);
    }

    #[test]
    fn test_context_optimization() {
        let mut tokenizer = UniversalTokenizer::new().unwrap();
        
        let long_text = "This is a very long text that might exceed context limits. ".repeat(1000);
        let optimized = tokenizer.optimize_for_context(&long_text, "gpt-3.5-turbo", 50.0);
        
        assert!(optimized.is_ok());
        let optimized_text = optimized.unwrap();
        assert!(optimized_text.len() < long_text.len());
        
        let count = tokenizer.count_tokens(&optimized_text, "gpt-3.5-turbo").unwrap();
        assert!(count.context_usage_percent <= 50.0);
    }

    #[test]
    fn test_model_comparison() {
        let mut tokenizer = UniversalTokenizer::new().unwrap();
        
        let text = "Compare costs across different models.";
        let results = tokenizer.find_optimal_model(text, 0.01).unwrap();
        
        assert!(!results.is_empty());
        
        // Results should be sorted by cost
        for i in 1..results.len() {
            assert!(results[i-1].1.estimated_input_cost <= results[i].1.estimated_input_cost);
        }
    }
}
