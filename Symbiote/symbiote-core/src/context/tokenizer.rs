//! # Context Tokenizer - Intelligent Context Management with Token Awareness
//! 
//! This module provides tokenizer-based context management that prevents context overflow,
//! enables intelligent compression, and ensures safe context handoffs across the entire
//! Symbiote ecosystem.
//! 
//! Based on Context Engineering best practices and tiktoken-rs integration.

use crate::{Result, SymbioteError, tokenizer::{UniversalTokenizer, TokenCount}};
use super::{GlobalContext, ContextUpdate, SystemContext};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Context health status based on token usage
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ContextHealth {
    Healthy,    // < 70% of token limit
    Warning,    // 70-90% of token limit
    Critical,   // > 90% of token limit
    Overflow,   // Exceeds token limit
}

/// Context tokenization metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextMetrics {
    pub current_tokens: usize,
    pub max_tokens: usize,
    pub usage_percentage: f64,
    pub compression_ratio: f64,
    pub health_status: ContextHealth,
    pub trend: UsageTrend,
    pub last_updated: DateTime<Utc>,
}

/// Usage trend analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UsageTrend {
    Stable,
    Growing,
    Shrinking,
    Volatile,
}

/// Compressed context with token validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenizedContext {
    /// Critical data that must never be compressed
    pub critical_data: serde_json::Value,
    /// Compressible data with intelligent reduction
    pub compressible_data: serde_json::Value,
    /// Archival references for cold storage
    pub archival_refs: Vec<String>,
    /// Token count metadata
    pub token_metadata: TokenMetadata,
    /// Integrity checksum
    pub checksum: String,
}

/// Token metadata for validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenMetadata {
    pub critical_tokens: usize,
    pub compressible_tokens: usize,
    pub total_tokens: usize,
    pub compression_applied: bool,
    pub model_used: String,
    pub created_at: DateTime<Utc>,
}

/// Context tokenizer with intelligent compression
#[derive(Debug)]
pub struct ContextTokenizer {
    /// Universal tokenizer for multiple models
    tokenizer: Arc<RwLock<UniversalTokenizer>>,
    /// Maximum token limit
    max_tokens: usize,
    /// Model to use for tokenization
    model: String,
    /// Critical fields that should never be compressed
    critical_fields: std::collections::HashSet<String>,
    /// Usage history for trend analysis
    usage_history: Arc<RwLock<std::collections::VecDeque<(DateTime<Utc>, usize)>>>,
    /// Compression strategies
    compression_strategies: HashMap<String, CompressionStrategy>,
}

/// Compression strategy for different data types
#[derive(Debug, Clone)]
pub struct CompressionStrategy {
    pub retention_ratio: f64,      // How much to keep (0.0-1.0)
    pub importance_threshold: f64, // Minimum importance score to keep
    pub max_age_hours: Option<u64>, // Maximum age before archival
    pub summarization_enabled: bool, // Whether to use AI summarization
}

impl ContextTokenizer {
    /// Create a new context tokenizer
    pub async fn new(max_tokens: usize, model: String) -> Result<Self> {
        let tokenizer = UniversalTokenizer::new()?;
        
        // Define critical fields that should never be compressed
        let mut critical_fields = std::collections::HashSet::new();
        critical_fields.insert("current_workspace".to_string());
        critical_fields.insert("active_agents".to_string());
        critical_fields.insert("user_preferences".to_string());
        critical_fields.insert("session_info".to_string());
        critical_fields.insert("open_files".to_string());
        
        // Define compression strategies
        let mut compression_strategies = HashMap::new();
        
        // Conservative strategy for conversations
        compression_strategies.insert("conversations".to_string(), CompressionStrategy {
            retention_ratio: 0.3,
            importance_threshold: 0.7,
            max_age_hours: Some(24),
            summarization_enabled: true,
        });
        
        // Aggressive strategy for workflows
        compression_strategies.insert("workflows".to_string(), CompressionStrategy {
            retention_ratio: 0.2,
            importance_threshold: 0.8,
            max_age_hours: Some(12),
            summarization_enabled: true,
        });
        
        // Moderate strategy for agent memory
        compression_strategies.insert("agent_memory".to_string(), CompressionStrategy {
            retention_ratio: 0.5,
            importance_threshold: 0.6,
            max_age_hours: Some(48),
            summarization_enabled: false,
        });
        
        Ok(Self {
            tokenizer: Arc::new(RwLock::new(tokenizer)),
            max_tokens,
            model,
            critical_fields,
            usage_history: Arc::new(RwLock::new(std::collections::VecDeque::new())),
            compression_strategies,
        })
    }
    
    /// Count tokens in any serializable context
    pub async fn count_tokens<T: Serialize>(&self, context: &T) -> Result<usize> {
        let json_str = serde_json::to_string(context)?;
        let mut tokenizer = self.tokenizer.write().await;
        let count_info = tokenizer.count_tokens(&json_str, &self.model)?;
        Ok(count_info.count as usize)
    }
    
    /// Check if context fits within token limits
    pub async fn fits_in_context<T: Serialize>(&self, context: &T) -> Result<bool> {
        let token_count = self.count_tokens(context).await?;
        Ok(token_count <= self.max_tokens)
    }
    
    /// Get remaining token budget
    pub async fn remaining_tokens<T: Serialize>(&self, current_context: &T) -> Result<usize> {
        let current_tokens = self.count_tokens(current_context).await?;
        Ok(self.max_tokens.saturating_sub(current_tokens))
    }

    /// Get maximum token limit
    pub fn max_tokens(&self) -> usize {
        self.max_tokens
    }
    
    /// Analyze context health and usage patterns
    pub async fn analyze_context_health<T: Serialize>(&self, context: &T) -> Result<ContextMetrics> {
        let current_tokens = self.count_tokens(context).await?;
        let usage_percentage = (current_tokens as f64 / self.max_tokens as f64) * 100.0;
        
        // Update usage history
        {
            let mut history = self.usage_history.write().await;
            let now = Utc::now();
            history.push_back((now, current_tokens));
            
            // Keep only last hour of data
            let one_hour_ago = now - chrono::Duration::hours(1);
            while let Some((timestamp, _)) = history.front() {
                if *timestamp < one_hour_ago {
                    history.pop_front();
                } else {
                    break;
                }
            }
        }
        
        // Determine health status
        let health_status = match usage_percentage {
            p if p >= 100.0 => ContextHealth::Overflow,
            p if p >= 90.0 => ContextHealth::Critical,
            p if p >= 70.0 => ContextHealth::Warning,
            _ => ContextHealth::Healthy,
        };
        
        // Calculate trend
        let trend = self.calculate_usage_trend().await;
        
        Ok(ContextMetrics {
            current_tokens,
            max_tokens: self.max_tokens,
            usage_percentage,
            compression_ratio: 1.0, // Will be updated after compression
            health_status,
            trend,
            last_updated: Utc::now(),
        })
    }
    
    /// Calculate usage trend from history
    async fn calculate_usage_trend(&self) -> UsageTrend {
        let history = self.usage_history.read().await;
        if history.len() < 3 {
            return UsageTrend::Stable;
        }
        
        let recent: Vec<usize> = history.iter().rev().take(5).map(|(_, tokens)| *tokens).collect();
        let variance = self.calculate_variance(&recent);
        let trend_slope = self.calculate_trend_slope(&recent);
        
        match (variance, trend_slope) {
            (v, _) if v > 1000.0 => UsageTrend::Volatile,
            (_, s) if s > 100.0 => UsageTrend::Growing,
            (_, s) if s < -100.0 => UsageTrend::Shrinking,
            _ => UsageTrend::Stable,
        }
    }
    
    fn calculate_variance(&self, values: &[usize]) -> f64 {
        if values.len() < 2 {
            return 0.0;
        }
        
        let mean = values.iter().sum::<usize>() as f64 / values.len() as f64;
        let variance = values.iter()
            .map(|&x| (x as f64 - mean).powi(2))
            .sum::<f64>() / values.len() as f64;
        
        variance
    }
    
    fn calculate_trend_slope(&self, values: &[usize]) -> f64 {
        if values.len() < 2 {
            return 0.0;
        }
        
        let n = values.len() as f64;
        let x_sum = (0..values.len()).sum::<usize>() as f64;
        let y_sum = values.iter().sum::<usize>() as f64;
        let xy_sum = values.iter().enumerate()
            .map(|(i, &y)| i as f64 * y as f64)
            .sum::<f64>();
        let x2_sum = (0..values.len())
            .map(|i| (i as f64).powi(2))
            .sum::<f64>();
        
        (n * xy_sum - x_sum * y_sum) / (n * x2_sum - x_sum.powi(2))
    }
}

impl ContextTokenizer {
    /// Intelligently compress context to fit within token limits
    pub async fn compress_context(&self, context: &GlobalContext) -> Result<TokenizedContext> {
        // 1. Serialize and measure full context
        let full_json = serde_json::to_string_pretty(context)?;
        let full_tokens = self.count_tokens(&full_json).await?;

        if full_tokens <= self.max_tokens {
            // No compression needed
            return Ok(TokenizedContext {
                critical_data: serde_json::to_value(context)?,
                compressible_data: serde_json::Value::Null,
                archival_refs: Vec::new(),
                token_metadata: TokenMetadata {
                    critical_tokens: full_tokens,
                    compressible_tokens: 0,
                    total_tokens: full_tokens,
                    compression_applied: false,
                    model_used: self.model.clone(),
                    created_at: Utc::now(),
                },
                checksum: self.calculate_checksum(&full_json),
            });
        }

        // 2. Extract critical vs compressible data
        let (critical_data, compressible_data) = self.separate_critical_data(context)?;

        // 3. Measure critical data tokens (never compress this)
        let critical_json = serde_json::to_string(&critical_data)?;
        let critical_tokens = self.count_tokens(&critical_json).await?;

        if critical_tokens > self.max_tokens {
            return Err(SymbioteError::ContextCorruption(
                "Critical context data exceeds token limit - cannot compress safely".to_string()
            ));
        }

        // 4. Use remaining budget for compressible data
        let remaining_budget = self.max_tokens - critical_tokens;
        let compressed_data = self.compress_to_budget(compressible_data, remaining_budget).await?;

        // 5. Calculate final metrics
        let compressed_json = serde_json::to_string(&compressed_data)?;
        let compressible_tokens = self.count_tokens(&compressed_json).await?;
        let total_tokens = critical_tokens + compressible_tokens;

        Ok(TokenizedContext {
            critical_data,
            compressible_data: compressed_data,
            archival_refs: Vec::new(), // TODO: Implement archival
            token_metadata: TokenMetadata {
                critical_tokens,
                compressible_tokens,
                total_tokens,
                compression_applied: true,
                model_used: self.model.clone(),
                created_at: Utc::now(),
            },
            checksum: self.calculate_checksum(&full_json),
        })
    }

    /// Separate critical from compressible data
    fn separate_critical_data(&self, context: &GlobalContext) -> Result<(serde_json::Value, serde_json::Value)> {
        let full_value = serde_json::to_value(context)?;
        let mut critical = serde_json::Map::new();
        let mut compressible = serde_json::Map::new();

        if let serde_json::Value::Object(obj) = full_value {
            for (key, value) in obj {
                if self.critical_fields.contains(&key) {
                    critical.insert(key, value);
                } else {
                    compressible.insert(key, value);
                }
            }
        }

        Ok((
            serde_json::Value::Object(critical),
            serde_json::Value::Object(compressible)
        ))
    }

    /// Compress data to fit within token budget
    async fn compress_to_budget(&self, data: serde_json::Value, token_budget: usize) -> Result<serde_json::Value> {
        let data_json = serde_json::to_string(&data)?;
        let data_tokens = self.count_tokens(&data_json).await?;

        if data_tokens <= token_budget {
            return Ok(data); // Fits as-is
        }

        // Apply intelligent compression strategies
        match data {
            serde_json::Value::Object(mut obj) => {
                // Priority order for compression (least important first)
                let compression_order = [
                    "running_workflows",
                    "active_conversations",
                    "agent_states",
                    "open_files"
                ];

                for field in compression_order {
                    if obj.contains_key(field) {
                        if let Some(strategy) = self.compression_strategies.get(field) {
                            let compressed_field = self.compress_field(&obj[field], strategy).await?;
                            obj.insert(field.to_string(), compressed_field);

                            // Check if we're within budget now
                            let test_json = serde_json::to_string(&obj)?;
                            if self.count_tokens(&test_json).await? <= token_budget {
                                break;
                            }
                        }
                    }
                }

                Ok(serde_json::Value::Object(obj))
            },
            _ => Ok(data)
        }
    }

    /// Compress a specific field using its strategy
    async fn compress_field(&self, field_value: &serde_json::Value, strategy: &CompressionStrategy) -> Result<serde_json::Value> {
        match field_value {
            serde_json::Value::Object(obj) => {
                // For objects (like conversations, workflows), keep most important items
                let mut items: Vec<_> = obj.iter().collect();
                let keep_count = (items.len() as f64 * strategy.retention_ratio) as usize;

                // Sort by importance (simplified - in real implementation, use actual importance scoring)
                items.sort_by(|a, b| {
                    // Prefer more recent items (simplified heuristic)
                    let a_time = self.extract_timestamp(a.1).unwrap_or(0);
                    let b_time = self.extract_timestamp(b.1).unwrap_or(0);
                    b_time.cmp(&a_time)
                });

                let mut compressed = serde_json::Map::new();
                for (key, value) in items.into_iter().take(keep_count) {
                    compressed.insert(key.clone(), value.clone());
                }

                Ok(serde_json::Value::Object(compressed))
            },
            serde_json::Value::Array(arr) => {
                // For arrays, keep most recent items
                let keep_count = (arr.len() as f64 * strategy.retention_ratio) as usize;
                let compressed: Vec<_> = arr.iter().rev().take(keep_count).rev().cloned().collect();
                Ok(serde_json::Value::Array(compressed))
            },
            _ => Ok(field_value.clone())
        }
    }

    /// Extract timestamp from JSON value (simplified)
    fn extract_timestamp(&self, value: &serde_json::Value) -> Option<u64> {
        value.get("timestamp")
            .or_else(|| value.get("created_at"))
            .or_else(|| value.get("last_updated"))
            .and_then(|v| v.as_u64())
    }

    /// Calculate checksum for integrity verification
    fn calculate_checksum(&self, data: &str) -> String {
        // Simple hash for now - in production use proper cryptographic hash
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        data.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    /// Validate compressed context integrity
    pub async fn validate_compression(&self, compressed: &TokenizedContext, original_checksum: &str) -> Result<bool> {
        // Reconstruct and verify critical data integrity
        let critical_json = serde_json::to_string(&compressed.critical_data)?;
        let critical_checksum = self.calculate_checksum(&critical_json);

        // For now, just verify the structure is intact
        // In production, you'd want more sophisticated validation
        Ok(!compressed.critical_data.is_null() &&
           compressed.token_metadata.total_tokens <= self.max_tokens)
    }
}

impl Default for CompressionStrategy {
    fn default() -> Self {
        Self {
            retention_ratio: 0.5,
            importance_threshold: 0.5,
            max_age_hours: Some(24),
            summarization_enabled: false,
        }
    }
}
