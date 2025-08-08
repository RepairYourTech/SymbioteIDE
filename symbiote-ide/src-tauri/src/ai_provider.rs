use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use anyhow::{Result, anyhow};
use reqwest::Client;

use crate::{AIRequest, AIResponse};

#[derive(Debug, Clone)]
pub struct ProviderConfig {
    pub name: String,
    pub api_key: String,
    pub base_url: String,
    pub models: Vec<String>,
    pub context_window: u32,
    pub cost_per_token: f64,
}

pub struct AIProviderManager {
    providers: HashMap<String, ProviderConfig>,
    client: Client,
}

impl AIProviderManager {
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
            client: Client::new(),
        }
    }

    pub fn add_provider(&mut self, config: ProviderConfig) {
        self.providers.insert(config.name.clone(), config);
    }

    pub async fn call_provider(&self, provider_name: &str, model: &str, prompt: &str, context: Option<&str>) -> Result<AIResponse> {
        let provider = self.providers.get(provider_name)
            .ok_or_else(|| anyhow!("Provider {} not found", provider_name))?;

        match provider_name {
            "openai" => self.call_openai(provider, model, prompt, context).await,
            "anthropic" => self.call_anthropic(provider, model, prompt, context).await,
            "google" => self.call_google(provider, model, prompt, context).await,
            "openrouter" => self.call_openrouter(provider, model, prompt, context).await,
            _ => Err(anyhow!("Unsupported provider: {}", provider_name))
        }
    }

    async fn call_openai(&self, provider: &ProviderConfig, model: &str, prompt: &str, context: Option<&str>) -> Result<AIResponse> {
        let mut messages = vec![];
        
        if let Some(ctx) = context {
            messages.push(serde_json::json!({
                "role": "system",
                "content": ctx
            }));
        }
        
        messages.push(serde_json::json!({
            "role": "user",
            "content": prompt
        }));

        let request_body = serde_json::json!({
            "model": model,
            "messages": messages,
            "max_tokens": 4000
        });

        let response = self.client
            .post(&format!("{}/chat/completions", provider.base_url))
            .header("Authorization", format!("Bearer {}", provider.api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await?;

        let response_json: serde_json::Value = response.json().await?;
        
        let content = response_json["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let tokens_used = response_json["usage"]["total_tokens"]
            .as_u64()
            .unwrap_or(0) as u32;

        Ok(AIResponse {
            content,
            provider: provider.name.clone(),
            model: model.to_string(),
            tokens_used,
        })
    }

    async fn call_anthropic(&self, provider: &ProviderConfig, model: &str, prompt: &str, context: Option<&str>) -> Result<AIResponse> {
        // Anthropic API implementation
        let full_prompt = if let Some(ctx) = context {
            format!("{}\n\nHuman: {}\n\nAssistant:", ctx, prompt)
        } else {
            format!("Human: {}\n\nAssistant:", prompt)
        };

        let request_body = serde_json::json!({
            "model": model,
            "max_tokens": 4000,
            "messages": [
                {
                    "role": "user",
                    "content": full_prompt
                }
            ]
        });

        let response = self.client
            .post(&format!("{}/messages", provider.base_url))
            .header("x-api-key", &provider.api_key)
            .header("Content-Type", "application/json")
            .header("anthropic-version", "2023-06-01")
            .json(&request_body)
            .send()
            .await?;

        let response_json: serde_json::Value = response.json().await?;
        
        let content = response_json["content"][0]["text"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let tokens_used = response_json["usage"]["output_tokens"]
            .as_u64()
            .unwrap_or(0) as u32;

        Ok(AIResponse {
            content,
            provider: provider.name.clone(),
            model: model.to_string(),
            tokens_used,
        })
    }

    async fn call_google(&self, provider: &ProviderConfig, model: &str, prompt: &str, context: Option<&str>) -> Result<AIResponse> {
        // Google Gemini API implementation
        let full_prompt = if let Some(ctx) = context {
            format!("{}\n\n{}", ctx, prompt)
        } else {
            prompt.to_string()
        };

        let request_body = serde_json::json!({
            "contents": [
                {
                    "parts": [
                        {
                            "text": full_prompt
                        }
                    ]
                }
            ]
        });

        let response = self.client
            .post(&format!("{}/v1/models/{}:generateContent?key={}", provider.base_url, model, provider.api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await?;

        let response_json: serde_json::Value = response.json().await?;
        
        let content = response_json["candidates"][0]["content"]["parts"][0]["text"]
            .as_str()
            .unwrap_or("")
            .to_string();

        Ok(AIResponse {
            content,
            provider: provider.name.clone(),
            model: model.to_string(),
            tokens_used: 0, // Google doesn't return token count in this format
        })
    }

    async fn call_openrouter(&self, provider: &ProviderConfig, model: &str, prompt: &str, context: Option<&str>) -> Result<AIResponse> {
        // OpenRouter API implementation - uses OpenAI-compatible format
        let mut messages = vec![];
        
        if let Some(ctx) = context {
            messages.push(serde_json::json!({
                "role": "system",
                "content": ctx
            }));
        }
        
        messages.push(serde_json::json!({
            "role": "user",
            "content": prompt
        }));

        let request_body = serde_json::json!({
            "model": model,
            "messages": messages,
            "max_tokens": 4000,
            "temperature": 0.7
        });

        let response = self.client
            .post(&format!("{}/chat/completions", provider.base_url))
            .header("Authorization", format!("Bearer {}", provider.api_key))
            .header("Content-Type", "application/json")
            .header("HTTP-Referer", "https://symbiote-ide.com") // Required by OpenRouter
            .header("X-Title", "SymbioteIDE") // Optional but recommended
            .json(&request_body)
            .send()
            .await?;

        let response_json: serde_json::Value = response.json().await?;
        
        let content = response_json["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let tokens_used = response_json["usage"]["total_tokens"]
            .as_u64()
            .unwrap_or(0) as u32;

        Ok(AIResponse {
            content,
            provider: provider.name.clone(),
            model: model.to_string(),
            tokens_used,
        })
    }
}

// Global provider manager instance
static mut PROVIDER_MANAGER: Option<AIProviderManager> = None;

pub async fn call_provider(request: AIRequest) -> Result<AIResponse> {
    unsafe {
        if PROVIDER_MANAGER.is_none() {
            let mut manager = AIProviderManager::new();
            
            // Initialize default providers (these would come from config)
            manager.add_provider(ProviderConfig {
                name: "openai".to_string(),
                api_key: "".to_string(), // Will be set from user config
                base_url: "https://api.openai.com/v1".to_string(),
                models: vec!["gpt-4".to_string(), "gpt-3.5-turbo".to_string()],
                context_window: 8192,
                cost_per_token: 0.00003,
            });

            manager.add_provider(ProviderConfig {
                name: "openrouter".to_string(),
                api_key: "".to_string(), // Will be set from user config
                base_url: "https://openrouter.ai/api/v1".to_string(),
                models: vec![
                    "openai/gpt-4o".to_string(),
                    "openai/gpt-4o-mini".to_string(),
                    "anthropic/claude-3.5-sonnet".to_string(),
                    "anthropic/claude-3-haiku".to_string(),
                    "google/gemini-2.0-flash-exp".to_string(),
                    "deepseek/deepseek-chat".to_string(),
                    "qwen/qwen-2.5-72b-instruct".to_string(),
                    "meta-llama/llama-3.1-405b-instruct".to_string(),
                    "mistralai/mistral-large".to_string(),
                    "cohere/command-r-plus".to_string(),
                ],
                context_window: 128000, // Varies by model, using a common high value
                cost_per_token: 0.00001, // Varies by model, this is approximate
            });

            PROVIDER_MANAGER = Some(manager);
        }

        let manager = PROVIDER_MANAGER.as_ref().unwrap();
        manager.call_provider(&request.provider, &request.model, &request.prompt, request.context.as_deref()).await
    }
}
