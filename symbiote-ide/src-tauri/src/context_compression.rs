use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::agent_runtime::AgentType;
use crate::agent_communication::AgentContext;

/// Automatic context compression system with user-configurable settings
pub struct AutoContextCompressor {
    compression_settings: Arc<RwLock<CompressionSettings>>,
    compression_strategies: HashMap<String, Box<dyn CompressionStrategy>>,
    context_monitor: Arc<ContextWindowMonitor>,
    compression_cache: Arc<RwLock<HashMap<String, CompressedContext>>>,
}

impl AutoContextCompressor {
    pub fn new(settings: CompressionSettings) -> Self {
        let mut compressor = Self {
            compression_settings: Arc::new(RwLock::new(settings)),
            compression_strategies: HashMap::new(),
            context_monitor: Arc::new(ContextWindowMonitor::new()),
            compression_cache: Arc::new(RwLock::new(HashMap::new())),
        };
        
        compressor.initialize_compression_strategies();
        compressor
    }
    
    fn initialize_compression_strategies(&mut self) {
        self.compression_strategies.insert(
            "hierarchical_summary".to_string(),
            Box::new(HierarchicalSummaryStrategy::new())
        );
        self.compression_strategies.insert(
            "semantic_clustering".to_string(),
            Box::new(SemanticClusteringStrategy::new())
        );
        self.compression_strategies.insert(
            "priority_based".to_string(),
            Box::new(PriorityBasedStrategy::new())
        );
        self.compression_strategies.insert(
            "intelligent_truncation".to_string(),
            Box::new(IntelligentTruncationStrategy::new())
        );
        self.compression_strategies.insert(
            "adaptive_summarization".to_string(),
            Box::new(AdaptiveSummarizationStrategy::new())
        );
    }
    
    /// Process context for agent task execution (uses full context window)
    pub async fn process_context_for_agent(
        &self,
        context: AgentContext,
        agent_type: AgentType,
        max_tokens: u32,
    ) -> Result<ProcessedContext, CompressionError> {
        let current_size = self.estimate_token_count(&context).await?;
        
        // For individual agent task execution, use full context window
        // Only compress if absolutely necessary (context exceeds model limits)
        if current_size <= max_tokens {
            return Ok(ProcessedContext {
                context,
                compression_applied: false,
                original_size: current_size,
                final_size: current_size,
                compression_ratio: 1.0,
                strategy_used: None,
                compression_metadata: None,
            });
        }
        
        // Only compress if context exceeds the model's absolute maximum
        // Use minimal compression to preserve as much context as possible
        let minimal_compression = 0.1; // Only 10% compression to fit within limits
        let compressed_context = self.apply_minimal_compression(context, agent_type, minimal_compression).await?;
        let final_size = self.estimate_token_count(&compressed_context).await?;
        
        Ok(ProcessedContext {
            context: compressed_context,
            compression_applied: true,
            original_size: current_size,
            final_size,
            compression_ratio: minimal_compression,
            strategy_used: Some("minimal_preservation".to_string()),
            compression_metadata: Some(CompressionMetadata {
                timestamp: Instant::now(),
                agent_type: agent_type.clone(),
                compression_level: minimal_compression,
                quality_score: 0.95, // High quality with minimal compression
            }),
        })
    }
    
    /// Process context for agent handoffs (applies intelligent compression)
    pub async fn process_context_for_handoff(
        &self,
        context: AgentContext,
        from_agent: AgentType,
        to_agent: AgentType,
        target_max_tokens: u32,
    ) -> Result<ProcessedContext, CompressionError> {
        let current_size = self.estimate_token_count(&context).await?;
        let settings = self.compression_settings.read().unwrap();
        
        // For handoffs, apply intelligent compression to optimize for the receiving agent
        let handoff_threshold = (target_max_tokens as f32 * 0.7) as u32; // Use 70% of target agent's capacity
        
        if current_size <= handoff_threshold {
            return Ok(ProcessedContext {
                context,
                compression_applied: false,
                original_size: current_size,
                final_size: current_size,
                compression_ratio: 1.0,
                strategy_used: None,
                compression_metadata: None,
            });
        }
        
        // Apply handoff-specific compression
        let compression_level = self.calculate_handoff_compression_level(current_size, target_max_tokens, &from_agent, &to_agent);
        let compressed_context = self.apply_handoff_compression(context, from_agent, to_agent, compression_level, &settings).await?;
        let final_size = self.estimate_token_count(&compressed_context).await?;
        
        Ok(ProcessedContext {
            context: compressed_context,
            compression_applied: true,
            original_size: current_size,
            final_size,
            compression_ratio: compression_level,
            strategy_used: Some(format!("handoff_{}_{}", 
                self.agent_type_to_string(&from_agent), 
                self.agent_type_to_string(&to_agent))),
            compression_metadata: Some(CompressionMetadata {
                timestamp: Instant::now(),
                agent_type: to_agent.clone(),
                compression_level,
                quality_score: self.calculate_handoff_quality_score(compression_level, &from_agent, &to_agent),
            }),
        })
    }
    
    fn calculate_handoff_compression_level(&self, current_size: u32, target_max_tokens: u32, from_agent: &AgentType, to_agent: &AgentType) -> f32 {
        let target_size = (target_max_tokens as f32 * 0.7) as u32; // Use 70% of target capacity
        let compression_needed = if current_size > target_size {
            1.0 - (target_size as f32 / current_size as f32)
        } else {
            0.0
        };
        
        // Adjust compression based on agent types and handoff requirements
        let base_compression = compression_needed;
        
        // Apply agent-specific adjustments
        let adjustment = match (from_agent, to_agent) {
            // Architect -> Developer: Preserve design decisions, compress background info
            (AgentType::Architect, AgentType::Developer) => 0.1,
            // Developer -> Specialist: Preserve implementation details, compress general context
            (AgentType::Developer, AgentType::RustSpecialist) => 0.05,
            (AgentType::Developer, AgentType::ReactSpecialist) => 0.05,
            // Any -> Debug: Aggressive compression, focus on error context
            (_, AgentType::Debug) => 0.3,
            // Specialist -> Test: Preserve code, compress implementation details
            (AgentType::RustSpecialist, AgentType::Test) => 0.15,
            (AgentType::ReactSpecialist, AgentType::Test) => 0.15,
            // Default: moderate compression
            _ => 0.2,
        };
        
        (base_compression + adjustment).min(0.8) // Cap at 80% compression
    }
    
    async fn apply_minimal_compression(&self, context: AgentContext, agent_type: AgentType, compression_level: f32) -> Result<AgentContext, CompressionError> {
        // Apply only essential compression to fit within model limits
        // Preserve all critical information for the agent's task execution
        
        let compressed_code = if let Some(code_context) = context.relevant_code {
            Some(self.minimal_compress_code(code_context, compression_level).await?)
        } else {
            None
        };
        
        Ok(AgentContext {
            task_context: context.task_context, // Never compress task context
            relevant_code: compressed_code,
            file_context: context.file_context, // Preserve file context for agents
            project_context: context.project_context, // Preserve project context
            agent_context: context.agent_context, // Preserve agent-specific context
        })
    }
    
    async fn apply_handoff_compression(
        &self,
        context: AgentContext,
        from_agent: AgentType,
        to_agent: AgentType,
        compression_level: f32,
        settings: &CompressionSettings,
    ) -> Result<AgentContext, CompressionError> {
        // Apply intelligent compression optimized for the receiving agent
        let strategy_name = self.select_handoff_strategy(&from_agent, &to_agent);
        
        if let Some(strategy) = self.compression_strategies.get(&strategy_name) {
            let compression_params = CompressionParams {
                level: compression_level,
                agent_type: to_agent,
                quality: CompressionQuality::HighQuality, // Use high quality for handoffs
                preserve_patterns: self.get_handoff_preserve_patterns(&from_agent, &to_agent),
                exemptions: settings.compression_exemptions.clone(),
                use_ai: settings.use_ai_compression,
                ai_model: settings.ai_compression_model.clone(),
            };
            
            strategy.compress(context, compression_params).await
        } else {
            // Fallback to default handoff compression
            self.apply_default_handoff_compression(context, from_agent, to_agent, compression_level).await
        }
    }
    
    fn select_handoff_strategy(&self, from_agent: &AgentType, to_agent: &AgentType) -> String {
        match (from_agent, to_agent) {
            // Architect -> Developer: Preserve architectural decisions
            (AgentType::Architect, AgentType::Developer) => "hierarchical_summary".to_string(),
            // Developer -> Specialist: Focus on relevant code
            (AgentType::Developer, AgentType::RustSpecialist) => "priority_based".to_string(),
            (AgentType::Developer, AgentType::ReactSpecialist) => "priority_based".to_string(),
            // Any -> Debug: Error-focused compression
            (_, AgentType::Debug) => "intelligent_truncation".to_string(),
            // Specialist -> Test: Code-focused compression
            (AgentType::RustSpecialist, AgentType::Test) => "semantic_clustering".to_string(),
            (AgentType::ReactSpecialist, AgentType::Test) => "semantic_clustering".to_string(),
            // Default: balanced approach
            _ => "hierarchical_summary".to_string(),
        }
    }
    
    fn get_handoff_preserve_patterns(&self, from_agent: &AgentType, to_agent: &AgentType) -> Vec<String> {
        let mut patterns = vec!["currentTask".to_string(), "immediateContext".to_string()];
        
        match (from_agent, to_agent) {
            (AgentType::Architect, AgentType::Developer) => {
                patterns.extend(vec!["design_decision".to_string(), "architecture_constraint".to_string()]);
            },
            (_, AgentType::Debug) => {
                patterns.extend(vec!["error_context".to_string(), "stack_trace".to_string(), "failing_code".to_string()]);
            },
            (_, AgentType::Security) => {
                patterns.extend(vec!["auth_code".to_string(), "security_sensitive".to_string(), "vulnerability".to_string()]);
            },
            _ => {}
        }
        
        patterns
    }
    
    async fn minimal_compress_code(&self, code_context: HashMap<String, String>, compression_level: f32) -> Result<HashMap<String, String>, CompressionError> {
        let mut compressed = HashMap::new();
        
        for (file_path, content) in code_context {
            if compression_level < 0.2 {
                // Very light compression: only remove excessive whitespace
                compressed.insert(file_path, self.remove_excessive_whitespace(&content));
            } else {
                // Still preserve most content, just remove comments
                compressed.insert(file_path, self.remove_comments_preserve_structure(&content));
            }
        }
        
        Ok(compressed)
    }
    
    fn remove_excessive_whitespace(&self, content: &str) -> String {
        // Remove excessive whitespace while preserving code structure
        content
            .lines()
            .map(|line| {
                let trimmed = line.trim_end();
                if trimmed.is_empty() {
                    "".to_string()
                } else {
                    trimmed.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
    
    fn remove_comments_preserve_structure(&self, content: &str) -> String {
        // Remove comments but preserve all code structure and logic
        content
            .lines()
            .filter_map(|line| {
                let trimmed = line.trim();
                if trimmed.starts_with("//") || trimmed.starts_with("#") || trimmed.starts_with("/*") {
                    None // Remove comment lines
                } else if line.contains("//") {
                    // Remove inline comments but keep code
                    Some(line.split("//").next().unwrap_or(line).trim_end().to_string())
                } else {
                    Some(line.to_string())
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
    
    async fn apply_default_handoff_compression(&self, context: AgentContext, _from_agent: AgentType, _to_agent: AgentType, compression_level: f32) -> Result<AgentContext, CompressionError> {
        // Default handoff compression preserves task context and applies light compression to other areas
        let compressed_code = if let Some(code_context) = context.relevant_code {
            Some(self.minimal_compress_code(code_context, compression_level).await?)
        } else {
            None
        };
        
        Ok(AgentContext {
            task_context: context.task_context, // Always preserve task context
            relevant_code: compressed_code,
            file_context: if compression_level > 0.5 { None } else { context.file_context },
            project_context: context.project_context,
            agent_context: context.agent_context,
        })
    }
    
    fn calculate_handoff_quality_score(&self, compression_level: f32, _from_agent: &AgentType, _to_agent: &AgentType) -> f32 {
        // Quality score for handoffs focuses on preserving relevant information for the target agent
        1.0 - (compression_level * 0.3) // Less quality impact for handoff compression
    }
    
    fn agent_type_to_string(&self, agent_type: &AgentType) -> String {
        format!("{:?}", agent_type).to_lowercase()
    }
    
    async fn apply_compression(
        &self,
        context: AgentContext,
        agent_type: AgentType,
        compression_level: f32,
        settings: &CompressionSettings,
    ) -> Result<AgentContext, CompressionError> {
        let strategy_name = &settings.preferred_strategy;
        
        if let Some(strategy) = self.compression_strategies.get(strategy_name) {
            let compression_params = CompressionParams {
                level: compression_level,
                agent_type,
                quality: settings.compression_quality.clone(),
                preserve_patterns: settings.always_preserve.clone(),
                exemptions: settings.compression_exemptions.clone(),
                use_ai: settings.use_ai_compression,
                ai_model: settings.ai_compression_model.clone(),
            };
            
            strategy.compress(context, compression_params).await
        } else {
            Err(CompressionError::StrategyNotFound(strategy_name.clone()))
        }
    }
    
    async fn estimate_token_count(&self, context: &AgentContext) -> Result<u32, CompressionError> {
        // Simplified token estimation - in real implementation, use proper tokenizer
        let mut total = 0;
        
        total += context.task_context.len() as u32 / 4; // Rough estimate: 4 chars per token
        
        if let Some(ref code_context) = context.relevant_code {
            for content in code_context.values() {
                total += content.len() as u32 / 4;
            }
        }
        
        if let Some(ref file_context) = context.file_context {
            for content in file_context.values() {
                total += content.len() as u32 / 4;
            }
        }
        
        if let Some(ref project_context) = context.project_context {
            total += project_context.len() as u32 / 4;
        }
        
        if let Some(ref agent_context) = context.agent_context {
            total += agent_context.len() as u32 / 4;
        }
        
        Ok(total)
    }
    
    fn calculate_quality_score(&self, compression_level: f32) -> f32 {
        // Quality decreases with higher compression
        1.0 - (compression_level * 0.5) // Max 50% quality loss at 100% compression
    }
    
    /// Update compression settings
    pub async fn update_settings(&self, new_settings: CompressionSettings) {
        let mut settings = self.compression_settings.write().unwrap();
        *settings = new_settings;
    }
    
    /// Get current compression settings
    pub async fn get_settings(&self) -> CompressionSettings {
        self.compression_settings.read().unwrap().clone()
    }
    
    /// Get compression statistics
    pub async fn get_compression_stats(&self, agent_type: &AgentType) -> CompressionStats {
        self.context_monitor.get_compression_stats(agent_type).await
    }
    
    /// Get handoff compression statistics
    pub async fn get_handoff_stats(&self, from_agent: &AgentType, to_agent: &AgentType) -> HandoffStats {
        self.context_monitor.get_handoff_stats(from_agent, to_agent).await
    }
}

/// Compression strategy trait
#[async_trait::async_trait]
pub trait CompressionStrategy: Send + Sync {
    async fn compress(&self, context: AgentContext, params: CompressionParams) -> Result<AgentContext, CompressionError>;
    fn get_strategy_info(&self) -> StrategyInfo;
}

/// Hierarchical summary compression strategy
pub struct HierarchicalSummaryStrategy;

impl HierarchicalSummaryStrategy {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl CompressionStrategy for HierarchicalSummaryStrategy {
    async fn compress(&self, context: AgentContext, params: CompressionParams) -> Result<AgentContext, CompressionError> {
        let compression_ratio = params.level;
        
        // Preserve critical context based on agent type
        let preserved_context = self.preserve_critical_context(&context, &params);
        
        // Compress remaining context hierarchically
        let compressed_code = if let Some(code_context) = context.relevant_code {
            Some(self.compress_code_hierarchically(code_context, compression_ratio).await?)
        } else {
            None
        };
        
        let compressed_files = if let Some(file_context) = context.file_context {
            Some(self.compress_files_hierarchically(file_context, compression_ratio).await?)
        } else {
            None
        };
        
        Ok(AgentContext {
            task_context: preserved_context.task_context,
            relevant_code: compressed_code,
            file_context: compressed_files,
            project_context: self.compress_project_context(context.project_context, compression_ratio).await?,
            agent_context: preserved_context.agent_context,
        })
    }
    
    fn get_strategy_info(&self) -> StrategyInfo {
        StrategyInfo {
            name: "Hierarchical Summary".to_string(),
            description: "Compresses context in layers, preserving the most important information".to_string(),
            quality_rating: 0.8,
            speed_rating: 0.7,
            best_for: vec!["General purpose".to_string(), "Balanced compression".to_string()],
        }
    }
}

impl HierarchicalSummaryStrategy {
    fn preserve_critical_context(&self, context: &AgentContext, params: &CompressionParams) -> AgentContext {
        // Always preserve task context and patterns specified in settings
        let mut preserved = AgentContext {
            task_context: context.task_context.clone(),
            relevant_code: None,
            file_context: None,
            project_context: None,
            agent_context: None,
        };
        
        // Preserve agent context for certain agent types
        match params.agent_type {
            AgentType::Debug | AgentType::Security => {
                preserved.agent_context = context.agent_context.clone();
            },
            _ => {}
        }
        
        preserved
    }
    
    async fn compress_code_hierarchically(&self, code_context: HashMap<String, String>, compression_ratio: f32) -> Result<HashMap<String, String>, CompressionError> {
        let mut compressed = HashMap::new();
        
        for (file_path, content) in code_context {
            if compression_ratio < 0.3 {
                // Light compression: remove comments and whitespace
                compressed.insert(file_path, self.remove_comments_and_whitespace(&content));
            } else if compression_ratio < 0.6 {
                // Medium compression: summarize functions
                compressed.insert(file_path, self.summarize_functions(&content).await?);
            } else {
                // Heavy compression: extract signatures only
                compressed.insert(file_path, self.extract_signatures(&content).await?);
            }
        }
        
        Ok(compressed)
    }
    
    async fn compress_files_hierarchically(&self, file_context: HashMap<String, String>, compression_ratio: f32) -> Result<HashMap<String, String>, CompressionError> {
        let mut compressed = HashMap::new();
        
        for (file_path, content) in file_context {
            if compression_ratio < 0.5 {
                // Keep important files, truncate others
                compressed.insert(file_path, self.truncate_content(&content, (1.0 - compression_ratio) as usize * 1000));
            } else {
                // Summarize file content
                compressed.insert(file_path, self.summarize_file_content(&content).await?);
            }
        }
        
        Ok(compressed)
    }
    
    async fn compress_project_context(&self, project_context: Option<String>, compression_ratio: f32) -> Result<Option<String>, CompressionError> {
        if let Some(context) = project_context {
            if compression_ratio > 0.5 {
                Ok(Some(self.summarize_project_context(&context).await?))
            } else {
                Ok(Some(context))
            }
        } else {
            Ok(None)
        }
    }
    
    fn remove_comments_and_whitespace(&self, content: &str) -> String {
        // Simple implementation - remove line comments and extra whitespace
        content
            .lines()
            .filter(|line| !line.trim_start().starts_with("//") && !line.trim_start().starts_with("#"))
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join("\n")
    }
    
    async fn summarize_functions(&self, content: &str) -> Result<String, CompressionError> {
        // Mock implementation - extract function signatures and first line of body
        let mut summary = String::new();
        let lines: Vec<&str> = content.lines().collect();
        
        for (i, line) in lines.iter().enumerate() {
            if line.contains("fn ") || line.contains("function ") || line.contains("def ") {
                summary.push_str(line);
                summary.push('\n');
                
                // Add first line of function body if available
                if i + 1 < lines.len() {
                    let next_line = lines[i + 1].trim();
                    if !next_line.is_empty() && !next_line.starts_with('}') {
                        summary.push_str("  // ");
                        summary.push_str(next_line);
                        summary.push_str("\n");
                    }
                }
            }
        }
        
        Ok(summary)
    }
    
    async fn extract_signatures(&self, content: &str) -> Result<String, CompressionError> {
        // Extract only function/method signatures
        let signatures: Vec<&str> = content
            .lines()
            .filter(|line| {
                line.contains("fn ") || 
                line.contains("function ") || 
                line.contains("def ") ||
                line.contains("class ") ||
                line.contains("interface ") ||
                line.contains("struct ") ||
                line.contains("enum ")
            })
            .collect();
        
        Ok(signatures.join("\n"))
    }
    
    fn truncate_content(&self, content: &str, max_length: usize) -> String {
        if content.len() <= max_length {
            content.to_string()
        } else {
            format!("{}...[truncated]", &content[..max_length])
        }
    }
    
    async fn summarize_file_content(&self, content: &str) -> Result<String, CompressionError> {
        // Simple summarization - first and last few lines
        let lines: Vec<&str> = content.lines().collect();
        if lines.len() <= 10 {
            Ok(content.to_string())
        } else {
            let mut summary = String::new();
            
            // First 3 lines
            for line in lines.iter().take(3) {
                summary.push_str(line);
                summary.push('\n');
            }
            
            summary.push_str("...[content summarized]...\n");
            
            // Last 3 lines
            for line in lines.iter().rev().take(3).rev() {
                summary.push_str(line);
                summary.push('\n');
            }
            
            Ok(summary)
        }
    }
    
    async fn summarize_project_context(&self, context: &str) -> Result<String, CompressionError> {
        // Extract key project information
        let lines: Vec<&str> = context.lines().collect();
        let mut summary = String::new();
        
        for line in lines {
            if line.contains("project") || line.contains("framework") || line.contains("language") || line.contains("version") {
                summary.push_str(line);
                summary.push('\n');
            }
        }
        
        if summary.is_empty() {
            Ok("Project context summarized".to_string())
        } else {
            Ok(summary)
        }
    }
}

/// Other compression strategies (simplified implementations)
pub struct SemanticClusteringStrategy;
pub struct PriorityBasedStrategy;
pub struct IntelligentTruncationStrategy;
pub struct AdaptiveSummarizationStrategy;

impl SemanticClusteringStrategy {
    pub fn new() -> Self { Self }
}

impl PriorityBasedStrategy {
    pub fn new() -> Self { Self }
}

impl IntelligentTruncationStrategy {
    pub fn new() -> Self { Self }
}

impl AdaptiveSummarizationStrategy {
    pub fn new() -> Self { Self }
}

// Implement CompressionStrategy for other strategies (similar pattern)
#[async_trait::async_trait]
impl CompressionStrategy for SemanticClusteringStrategy {
    async fn compress(&self, context: AgentContext, _params: CompressionParams) -> Result<AgentContext, CompressionError> {
        // Mock implementation
        Ok(context)
    }
    
    fn get_strategy_info(&self) -> StrategyInfo {
        StrategyInfo {
            name: "Semantic Clustering".to_string(),
            description: "Groups semantically similar content for intelligent compression".to_string(),
            quality_rating: 0.9,
            speed_rating: 0.5,
            best_for: vec!["Complex codebases".to_string(), "High quality compression".to_string()],
        }
    }
}

#[async_trait::async_trait]
impl CompressionStrategy for PriorityBasedStrategy {
    async fn compress(&self, context: AgentContext, _params: CompressionParams) -> Result<AgentContext, CompressionError> {
        // Mock implementation
        Ok(context)
    }
    
    fn get_strategy_info(&self) -> StrategyInfo {
        StrategyInfo {
            name: "Priority Based".to_string(),
            description: "Compresses based on content priority and importance".to_string(),
            quality_rating: 0.8,
            speed_rating: 0.8,
            best_for: vec!["Task-focused compression".to_string(), "Efficient processing".to_string()],
        }
    }
}

#[async_trait::async_trait]
impl CompressionStrategy for IntelligentTruncationStrategy {
    async fn compress(&self, context: AgentContext, _params: CompressionParams) -> Result<AgentContext, CompressionError> {
        // Mock implementation
        Ok(context)
    }
    
    fn get_strategy_info(&self) -> StrategyInfo {
        StrategyInfo {
            name: "Intelligent Truncation".to_string(),
            description: "Smart truncation preserving logical boundaries".to_string(),
            quality_rating: 0.7,
            speed_rating: 0.9,
            best_for: vec!["Fast compression".to_string(), "Real-time processing".to_string()],
        }
    }
}

#[async_trait::async_trait]
impl CompressionStrategy for AdaptiveSummarizationStrategy {
    async fn compress(&self, context: AgentContext, _params: CompressionParams) -> Result<AgentContext, CompressionError> {
        // Mock implementation
        Ok(context)
    }
    
    fn get_strategy_info(&self) -> StrategyInfo {
        StrategyInfo {
            name: "AI Summarization".to_string(),
            description: "AI-powered adaptive summarization for highest quality".to_string(),
            quality_rating: 0.95,
            speed_rating: 0.4,
            best_for: vec!["Highest quality".to_string(), "Complex content".to_string()],
        }
    }
}

/// Context window monitoring
pub struct ContextWindowMonitor {
    usage_history: Arc<RwLock<HashMap<AgentType, Vec<ContextUsageRecord>>>>,
    compression_history: Arc<RwLock<HashMap<AgentType, Vec<CompressionRecord>>>>,
    handoff_history: Arc<RwLock<HashMap<(AgentType, AgentType), Vec<HandoffRecord>>>>,
}

impl ContextWindowMonitor {
    pub fn new() -> Self {
        Self {
            usage_history: Arc::new(RwLock::new(HashMap::new())),
            compression_history: Arc::new(RwLock::new(HashMap::new())),
            handoff_history: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    pub async fn record_usage(&self, agent_type: AgentType, usage: ContextUsageRecord) {
        let mut history = self.usage_history.write().unwrap();
        history.entry(agent_type).or_insert_with(Vec::new).push(usage);
    }
    
    pub async fn record_compression(&self, agent_type: AgentType, compression: CompressionRecord) {
        let mut history = self.compression_history.write().unwrap();
        history.entry(agent_type).or_insert_with(Vec::new).push(compression);
    }
    
    pub async fn record_handoff(&self, from_agent: AgentType, to_agent: AgentType, handoff: HandoffRecord) {
        let mut history = self.handoff_history.write().unwrap();
        history.entry((from_agent, to_agent)).or_insert_with(Vec::new).push(handoff);
    }
    
    pub async fn get_compression_stats(&self, agent_type: &AgentType) -> CompressionStats {
        let compression_history = self.compression_history.read().unwrap();
        
        if let Some(records) = compression_history.get(agent_type) {
            let total_compressions = records.len() as u32;
            let average_ratio = records.iter().map(|r| r.compression_ratio).sum::<f32>() / total_compressions as f32;
            let average_quality = records.iter().map(|r| r.quality_score).sum::<f32>() / total_compressions as f32;
            
            CompressionStats {
                agent_type: agent_type.clone(),
                total_compressions,
                average_compression_ratio: average_ratio,
                average_quality_score: average_quality,
                last_compression: records.last().map(|r| r.timestamp),
            }
        } else {
            CompressionStats {
                agent_type: agent_type.clone(),
                total_compressions: 0,
                average_compression_ratio: 1.0,
                average_quality_score: 1.0,
                last_compression: None,
            }
        }
    }
    
    pub async fn get_handoff_stats(&self, from_agent: &AgentType, to_agent: &AgentType) -> HandoffStats {
        let handoff_history = self.handoff_history.read().unwrap();
        
        if let Some(records) = handoff_history.get(&(from_agent.clone(), to_agent.clone())) {
            let total_handoffs = records.len() as u32;
            let average_compression = records.iter().map(|r| r.compression_ratio).sum::<f32>() / total_handoffs as f32;
            let average_continuity = records.iter().map(|r| r.continuity_score).sum::<f32>() / total_handoffs as f32;
            let average_quality = records.iter().map(|r| r.quality_score).sum::<f32>() / total_handoffs as f32;
            
            HandoffStats {
                from_agent: from_agent.clone(),
                to_agent: to_agent.clone(),
                total_handoffs,
                average_compression_ratio: average_compression,
                average_continuity_score: average_continuity,
                average_quality_score: average_quality,
                last_handoff: records.last().map(|r| r.timestamp),
            }
        } else {
            HandoffStats {
                from_agent: from_agent.clone(),
                to_agent: to_agent.clone(),
                total_handoffs: 0,
                average_compression_ratio: 1.0,
                average_continuity_score: 1.0,
                average_quality_score: 1.0,
                last_handoff: None,
            }
        }
    }
}

// Supporting types and structures

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionSettings {
    pub compression_threshold: u8,        // % of max tokens before compression (default: 85)
    pub light_compression: u8,            // % reduction for light compression (default: 15)
    pub medium_compression: u8,           // % reduction for medium compression (default: 40)
    pub heavy_compression: u8,            // % reduction for heavy compression (default: 70)
    pub aggressive_compression: u8,       // % reduction for aggressive compression (default: 85)
    pub preferred_strategy: String,       // Preferred compression strategy
    pub compression_quality: CompressionQuality,
    pub always_preserve: Vec<String>,     // Context types to never compress
    pub compression_exemptions: Vec<String>, // Patterns to preserve
    pub use_ai_compression: bool,
    pub ai_compression_model: String,
    pub track_effectiveness: bool,
    pub auto_adjust_settings: bool,
}

impl Default for CompressionSettings {
    fn default() -> Self {
        Self {
            compression_threshold: 85,
            light_compression: 15,
            medium_compression: 40,
            heavy_compression: 70,
            aggressive_compression: 85,
            preferred_strategy: "hierarchical_summary".to_string(),
            compression_quality: CompressionQuality::Balanced,
            always_preserve: vec!["currentTask".to_string(), "immediateContext".to_string(), "errorContext".to_string()],
            compression_exemptions: vec!["critical_error".to_string(), "security_issue".to_string(), "breaking_change".to_string()],
            use_ai_compression: true,
            ai_compression_model: "claude-3-haiku".to_string(),
            track_effectiveness: true,
            auto_adjust_settings: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CompressionQuality {
    Fast,
    Balanced,
    HighQuality,
}

#[derive(Debug, Clone)]
pub struct CompressionParams {
    pub level: f32,
    pub agent_type: AgentType,
    pub quality: CompressionQuality,
    pub preserve_patterns: Vec<String>,
    pub exemptions: Vec<String>,
    pub use_ai: bool,
    pub ai_model: String,
}

#[derive(Debug, Clone)]
pub struct ProcessedContext {
    pub context: AgentContext,
    pub compression_applied: bool,
    pub original_size: u32,
    pub final_size: u32,
    pub compression_ratio: f32,
    pub strategy_used: Option<String>,
    pub compression_metadata: Option<CompressionMetadata>,
}

#[derive(Debug, Clone)]
pub struct CompressionMetadata {
    pub timestamp: Instant,
    pub agent_type: AgentType,
    pub compression_level: f32,
    pub quality_score: f32,
}

#[derive(Debug, Clone)]
pub struct CompressedContext {
    pub summary: String,
    pub prioritized_sections: Vec<ContextSection>,
    pub metadata: CompressionMetadata,
}

#[derive(Debug, Clone)]
pub struct ContextSection {
    pub section_type: String,
    pub content: String,
    pub priority: u8,
    pub token_estimate: u32,
}

#[derive(Debug, Clone)]
pub struct StrategyInfo {
    pub name: String,
    pub description: String,
    pub quality_rating: f32,
    pub speed_rating: f32,
    pub best_for: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ContextUsageRecord {
    pub timestamp: Instant,
    pub agent_type: AgentType,
    pub context_size: u32,
    pub max_tokens: u32,
    pub utilization_percentage: f32,
}

#[derive(Debug, Clone)]
pub struct CompressionRecord {
    pub timestamp: Instant,
    pub agent_type: AgentType,
    pub original_size: u32,
    pub compressed_size: u32,
    pub compression_ratio: f32,
    pub strategy_used: String,
    pub quality_score: f32,
}

#[derive(Debug, Clone)]
pub struct CompressionStats {
    pub agent_type: AgentType,
    pub total_compressions: u32,
    pub average_compression_ratio: f32,
    pub average_quality_score: f32,
    pub last_compression: Option<Instant>,
}

#[derive(Debug, Clone)]
pub struct HandoffStats {
    pub from_agent: AgentType,
    pub to_agent: AgentType,
    pub total_handoffs: u32,
    pub average_compression_ratio: f32,
    pub average_continuity_score: f32,
    pub average_quality_score: f32,
    pub last_handoff: Option<Instant>,
}

#[derive(Debug, Clone)]
pub struct HandoffRecord {
    pub timestamp: Instant,
    pub from_agent: AgentType,
    pub to_agent: AgentType,
    pub original_size: u32,
    pub compressed_size: u32,
    pub compression_ratio: f32,
    pub continuity_score: f32,
    pub quality_score: f32,
    pub strategy_used: String,
}

#[derive(Debug, thiserror::Error)]
pub enum CompressionError {
    #[error("Compression strategy not found: {0}")]
    StrategyNotFound(String),
    
    #[error("Token estimation failed: {0}")]
    TokenEstimationFailed(String),
    
    #[error("Compression failed: {0}")]
    CompressionFailed(String),
    
    #[error("AI compression failed: {0}")]
    AiCompressionFailed(String),
    
    #[error("Invalid compression settings: {0}")]
    InvalidSettings(String),
}

/// Type alias for compatibility
pub type ContextCompressor = AutoContextCompressor;
