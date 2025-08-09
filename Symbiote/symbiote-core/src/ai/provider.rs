//! AI provider abstractions

use crate::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIRequest {
    pub model: String,
    pub messages: Vec<AIMessage>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIResponse {
    pub content: String,
    pub model: String,
    pub usage: TokenUsage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// AI provider trait
#[async_trait::async_trait]
pub trait AIProvider: Send + Sync {
    async fn generate(&self, request: AIRequest) -> Result<AIResponse>;
    fn name(&self) -> &str;
    fn supported_models(&self) -> Vec<String>;
}
