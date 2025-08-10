//! # Context Compression Engine
//! 
//! Intelligent compression of context data to reduce memory usage while
//! preserving important information and maintaining system performance.
//! 
//! Following Week 13-14 Context Management & Knowledge Graph implementation plan.

use super::*;
use crate::{Result, SymbioteError};
use std::collections::{HashMap, VecDeque};

/// Context compression result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionResult {
    pub compression_ratio: f64,
    pub archived_conversations: Vec<String>,
    pub archived_workflows: Vec<String>,
    pub memory_saved_bytes: usize,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Context compression engine
#[derive(Debug)]
pub struct ContextCompressionEngine {
    compression_strategies: HashMap<String, CompressionStrategy>,
    compression_history: Vec<CompressionResult>,
    memory_thresholds: MemoryThresholds,
}

impl ContextCompressionEngine {
    pub fn new() -> Self {
        Self {
            compression_strategies: Self::default_strategies(),
            compression_history: Vec::new(),
            memory_thresholds: MemoryThresholds::default(),
        }
    }

    /// Compress old context data to reduce memory usage
    pub async fn compress_old_context(&self, context: &GlobalContext) -> Result<CompressionResult> {
        let mut archived_conversations = Vec::new();
        let mut archived_workflows = Vec::new();
        let mut memory_saved = 0;

        // Analyze memory usage
        let memory_analysis = self.analyze_memory_usage(context).await?;

        if memory_analysis.total_memory > self.memory_thresholds.compression_trigger {
            // Identify candidates for compression
            let conversations_to_compress = self.identify_conversations_for_compression(context, &memory_analysis).await?;
            let workflows_to_compress = self.identify_workflows_for_compression(context, &memory_analysis).await?;

            // Archive old conversations
            for conversation_id in conversations_to_compress {
                archived_conversations.push(conversation_id);
                memory_saved += 1024; // Estimate 1KB saved per conversation
            }

            // Archive old workflows
            for workflow_id in workflows_to_compress {
                archived_workflows.push(workflow_id);
                memory_saved += 2048; // Estimate 2KB saved per workflow
            }
        }

        let compression_ratio = if memory_analysis.total_memory > 0 {
            1.0 - (memory_saved as f64 / memory_analysis.total_memory as f64)
        } else {
            1.0
        };

        Ok(CompressionResult {
            compression_ratio,
            archived_conversations,
            archived_workflows,
            memory_saved_bytes: memory_saved,
            timestamp: chrono::Utc::now(),
        })
    }

    /// Compress a specific conversation
    pub async fn compress_conversation(&self, conversation: &ConversationContext) -> Result<CompressedConversation> {
        let strategy = &self.compression_strategies["conversation"];
        
        // Compress messages
        let compressed_messages = self.compress_messages(&conversation.messages, strategy).await?;
        
        // Extract key information
        let summary = self.generate_conversation_summary(conversation).await?;
        let key_topics = self.extract_key_topics(conversation).await?;
        let important_decisions = self.extract_important_decisions(conversation).await?;

        Ok(CompressedConversation {
            id: conversation.id.clone(),
            title: conversation.title.clone(),
            summary,
            key_topics,
            important_decisions,
            compressed_messages: compressed_messages.clone(),
            participants: conversation.participants.clone(),
            created_at: conversation.created_at,
            last_activity: conversation.last_activity,
            original_message_count: conversation.messages.len(),
            compression_ratio: compressed_messages.len() as f64 / conversation.messages.len() as f64,
        })
    }

    /// Compress a workflow
    pub async fn compress_workflow(&self, workflow: &WorkflowContext) -> Result<CompressedWorkflow> {
        let strategy = &self.compression_strategies["workflow"];
        
        // Compress steps
        let compressed_steps = self.compress_workflow_steps(&workflow.steps, strategy).await?;
        
        // Extract execution summary
        let execution_summary = self.generate_workflow_summary(workflow).await?;
        let performance_metrics = self.extract_workflow_metrics(workflow).await?;

        Ok(CompressedWorkflow {
            id: workflow.id.clone(),
            name: workflow.name.clone(),
            status: workflow.status.clone(),
            execution_summary,
            performance_metrics,
            compressed_steps: compressed_steps.clone(),
            variables: workflow.variables.clone(),
            created_at: workflow.created_at,
            completed_at: workflow.completed_at,
            original_step_count: workflow.steps.len(),
            compression_ratio: compressed_steps.len() as f64 / workflow.steps.len() as f64,
        })
    }

    /// Compress agent memory
    pub async fn compress_agent_memory(&self, memory: &AgentMemory) -> Result<CompressedAgentMemory> {
        let strategy = &self.compression_strategies["agent_memory"];
        
        // Compress short-term memory
        let compressed_short_term = self.compress_memory_items(&memory.short_term, strategy).await?;
        
        // Compress long-term memory
        let compressed_long_term = self.compress_long_term_memory(&memory.long_term, strategy).await?;
        
        // Extract memory patterns
        let memory_patterns = self.extract_memory_patterns(memory).await?;

        Ok(CompressedAgentMemory {
            compressed_short_term: compressed_short_term.clone(),
            compressed_long_term: compressed_long_term.clone(),
            memory_patterns,
            working_memory: memory.working_memory.clone(),
            original_short_term_count: memory.short_term.len(),
            original_long_term_count: memory.long_term.len(),
            compression_ratio: (compressed_short_term.len() + compressed_long_term.len()) as f64 
                / (memory.short_term.len() + memory.long_term.len()) as f64,
        })
    }

    async fn analyze_memory_usage(&self, context: &GlobalContext) -> Result<MemoryAnalysis> {
        let mut analysis = MemoryAnalysis::new();

        // Analyze conversations
        for conversation in context.active_conversations.values() {
            let memory_usage = self.estimate_conversation_memory(conversation);
            analysis.conversation_memory.insert(conversation.id.clone(), memory_usage);
            analysis.total_memory += memory_usage;
        }

        // Analyze workflows
        for workflow in context.running_workflows.values() {
            let memory_usage = self.estimate_workflow_memory(workflow);
            analysis.workflow_memory.insert(workflow.id.clone(), memory_usage);
            analysis.total_memory += memory_usage;
        }

        // Analyze agents
        for agent in context.agent_states.values() {
            let memory_usage = self.estimate_agent_memory(agent);
            analysis.agent_memory.insert(agent.id.clone(), memory_usage);
            analysis.total_memory += memory_usage;
        }

        Ok(analysis)
    }

    async fn identify_conversations_for_compression(&self, context: &GlobalContext, analysis: &MemoryAnalysis) -> Result<Vec<String>> {
        let mut candidates = Vec::new();
        let threshold = self.memory_thresholds.conversation_compression_threshold;

        for (conversation_id, &memory_usage) in &analysis.conversation_memory {
            if memory_usage > threshold {
                if let Some(conversation) = context.active_conversations.get(conversation_id) {
                    // Check if conversation is old enough for compression
                    let age = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() - conversation.last_activity;
                    if age > 3600 { // 1 hour
                        candidates.push(conversation_id.clone());
                    }
                }
            }
        }

        Ok(candidates)
    }

    async fn identify_workflows_for_compression(&self, context: &GlobalContext, analysis: &MemoryAnalysis) -> Result<Vec<String>> {
        let mut candidates = Vec::new();
        let threshold = self.memory_thresholds.workflow_compression_threshold;

        for (workflow_id, &memory_usage) in &analysis.workflow_memory {
            if memory_usage > threshold {
                if let Some(workflow) = context.running_workflows.get(workflow_id) {
                    // Only compress completed workflows
                    if matches!(workflow.status, WorkflowStatus::Completed | WorkflowStatus::Failed | WorkflowStatus::Cancelled) {
                        candidates.push(workflow_id.clone());
                    }
                }
            }
        }

        Ok(candidates)
    }

    async fn identify_agents_for_compression(&self, context: &GlobalContext, analysis: &MemoryAnalysis) -> Result<Vec<String>> {
        let mut candidates = Vec::new();
        let threshold = self.memory_thresholds.agent_compression_threshold;

        for (agent_id, &memory_usage) in &analysis.agent_memory {
            if memory_usage > threshold {
                if let Some(agent) = context.agent_states.get(agent_id) {
                    // Compress memory of idle agents
                    if matches!(agent.status, AgentStatus::Idle | AgentStatus::Offline) {
                        candidates.push(agent_id.clone());
                    }
                }
            }
        }

        Ok(candidates)
    }

    async fn calculate_compression_ratio(&self, compression: &ContextCompression, analysis: &MemoryAnalysis) -> Result<f64> {
        let mut original_size = 0;
        let mut compressed_size = 0;

        // Estimate compression for conversations
        for conversation_id in &compression.conversations_to_compress {
            if let Some(&memory_usage) = analysis.conversation_memory.get(conversation_id) {
                original_size += memory_usage;
                compressed_size += memory_usage / 3; // Estimate 3:1 compression ratio
            }
        }

        // Estimate compression for workflows
        for workflow_id in &compression.workflows_to_compress {
            if let Some(&memory_usage) = analysis.workflow_memory.get(workflow_id) {
                original_size += memory_usage;
                compressed_size += memory_usage / 4; // Estimate 4:1 compression ratio
            }
        }

        // Estimate compression for agents
        for agent_id in &compression.agents_to_compress {
            if let Some(&memory_usage) = analysis.agent_memory.get(agent_id) {
                original_size += memory_usage;
                compressed_size += memory_usage / 2; // Estimate 2:1 compression ratio
            }
        }

        if original_size > 0 {
            Ok(compressed_size as f64 / original_size as f64)
        } else {
            Ok(1.0)
        }
    }

    async fn compress_messages(&self, messages: &VecDeque<Message>, strategy: &CompressionStrategy) -> Result<Vec<CompressedMessage>> {
        let mut compressed = Vec::new();
        let keep_count = (messages.len() as f64 * strategy.retention_ratio) as usize;
        
        // Keep most recent messages
        for message in messages.iter().rev().take(keep_count) {
            compressed.push(CompressedMessage {
                id: message.id.clone(),
                sender: message.sender.clone(),
                content_summary: self.summarize_message_content(&message.content).await?,
                message_type: message.message_type.clone(),
                timestamp: message.timestamp,
                importance_score: self.calculate_message_importance(message).await?,
            });
        }

        compressed.reverse();
        Ok(compressed)
    }

    async fn compress_workflow_steps(&self, steps: &[WorkflowStep], strategy: &CompressionStrategy) -> Result<Vec<CompressedWorkflowStep>> {
        let mut compressed = Vec::new();
        let keep_count = (steps.len() as f64 * strategy.retention_ratio) as usize;
        
        // Keep most important steps
        let mut step_importance: Vec<_> = steps.iter()
            .map(|step| (step, self.calculate_step_importance(step)))
            .collect();
        
        step_importance.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        
        for (step, importance) in step_importance.into_iter().take(keep_count) {
            compressed.push(CompressedWorkflowStep {
                id: step.id.clone(),
                name: step.name.clone(),
                step_type: step.step_type.clone(),
                status: step.status.clone(),
                execution_summary: self.summarize_step_execution(step).await?,
                importance_score: importance,
            });
        }

        Ok(compressed)
    }

    async fn compress_memory_items(&self, items: &VecDeque<MemoryItem>, strategy: &CompressionStrategy) -> Result<Vec<CompressedMemoryItem>> {
        let mut compressed = Vec::new();
        let keep_count = (items.len() as f64 * strategy.retention_ratio) as usize;
        
        // Sort by importance and recency
        let mut item_scores: Vec<_> = items.iter()
            .map(|item| {
                let recency_score = 1.0 / (1.0 + (SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() - item.last_accessed) as f64 / 3600.0);
                let combined_score = item.importance * 0.7 + recency_score * 0.3;
                (item, combined_score)
            })
            .collect();
        
        item_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        
        for (item, score) in item_scores.into_iter().take(keep_count) {
            compressed.push(CompressedMemoryItem {
                id: item.id.clone(),
                content_summary: self.summarize_memory_content(&item.content).await?,
                item_type: item.item_type.clone(),
                importance: item.importance,
                created_at: item.created_at,
                access_pattern: self.analyze_access_pattern(item).await?,
                combined_score: score,
            });
        }

        Ok(compressed)
    }

    async fn compress_long_term_memory(&self, items: &HashMap<String, MemoryItem>, strategy: &CompressionStrategy) -> Result<Vec<CompressedMemoryItem>> {
        let keep_count = (items.len() as f64 * strategy.retention_ratio) as usize;
        
        // Convert to vector and sort by importance
        let mut item_vec: Vec<_> = items.values().collect();
        item_vec.sort_by(|a, b| b.importance.partial_cmp(&a.importance).unwrap());
        
        let mut compressed = Vec::new();
        for item in item_vec.into_iter().take(keep_count) {
            compressed.push(CompressedMemoryItem {
                id: item.id.clone(),
                content_summary: self.summarize_memory_content(&item.content).await?,
                item_type: item.item_type.clone(),
                importance: item.importance,
                created_at: item.created_at,
                access_pattern: self.analyze_access_pattern(item).await?,
                combined_score: item.importance,
            });
        }

        Ok(compressed)
    }

    // Helper methods for memory estimation
    fn estimate_conversation_memory(&self, conversation: &ConversationContext) -> usize {
        conversation.messages.len() * 512 // Estimate 512 bytes per message
    }

    fn estimate_workflow_memory(&self, workflow: &WorkflowContext) -> usize {
        workflow.steps.len() * 1024 + workflow.variables.len() * 256 // Estimate step and variable sizes
    }

    fn estimate_agent_memory(&self, agent: &AgentContext) -> usize {
        agent.memory.short_term.len() * 256 + agent.memory.long_term.len() * 512
    }

    // Helper methods for content analysis
    async fn summarize_message_content(&self, content: &str) -> Result<String> {
        // Simplified summarization - in practice would use AI
        if content.len() > 100 {
            Ok(format!("{}...", &content[..97]))
        } else {
            Ok(content.to_string())
        }
    }

    async fn calculate_message_importance(&self, _message: &Message) -> Result<f64> {
        // Simplified importance calculation
        Ok(0.5)
    }

    fn calculate_step_importance(&self, step: &WorkflowStep) -> f64 {
        match step.status {
            StepStatus::Failed => 0.9,
            StepStatus::Completed => 0.7,
            StepStatus::Running => 0.8,
            _ => 0.3,
        }
    }

    async fn summarize_step_execution(&self, step: &WorkflowStep) -> Result<String> {
        Ok(format!("{}: {:?}", step.name, step.status))
    }

    async fn summarize_memory_content(&self, content: &str) -> Result<String> {
        if content.len() > 50 {
            Ok(format!("{}...", &content[..47]))
        } else {
            Ok(content.to_string())
        }
    }

    async fn analyze_access_pattern(&self, item: &MemoryItem) -> Result<AccessPattern> {
        Ok(AccessPattern {
            frequency: item.access_count as f64,
            recency: (SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() - item.last_accessed) as f64,
            pattern_type: if item.access_count > 10 { "frequent".to_string() } else { "occasional".to_string() },
        })
    }

    // Additional helper methods
    async fn generate_conversation_summary(&self, _conversation: &ConversationContext) -> Result<String> {
        Ok("Conversation summary".to_string())
    }

    async fn extract_key_topics(&self, _conversation: &ConversationContext) -> Result<Vec<String>> {
        Ok(vec!["topic1".to_string(), "topic2".to_string()])
    }

    async fn extract_important_decisions(&self, _conversation: &ConversationContext) -> Result<Vec<String>> {
        Ok(vec!["decision1".to_string()])
    }

    async fn generate_workflow_summary(&self, _workflow: &WorkflowContext) -> Result<String> {
        Ok("Workflow execution summary".to_string())
    }

    async fn extract_workflow_metrics(&self, _workflow: &WorkflowContext) -> Result<WorkflowMetrics> {
        Ok(WorkflowMetrics {
            total_execution_time: 1000,
            success_rate: 0.95,
            error_count: 1,
        })
    }

    async fn extract_memory_patterns(&self, _memory: &AgentMemory) -> Result<Vec<MemoryPattern>> {
        Ok(vec![MemoryPattern {
            pattern_type: "frequent_access".to_string(),
            description: "Frequently accessed memories".to_string(),
            strength: 0.8,
        }])
    }

    fn default_strategies() -> HashMap<String, CompressionStrategy> {
        let mut strategies = HashMap::new();

        strategies.insert("conversation".to_string(), CompressionStrategy {
            retention_ratio: 0.3,
            importance_threshold: 0.5,
            age_threshold: 3600, // 1 hour
        });

        strategies.insert("workflow".to_string(), CompressionStrategy {
            retention_ratio: 0.5,
            importance_threshold: 0.6,
            age_threshold: 7200, // 2 hours
        });

        strategies.insert("agent_memory".to_string(), CompressionStrategy {
            retention_ratio: 0.4,
            importance_threshold: 0.7,
            age_threshold: 1800, // 30 minutes
        });

        strategies
    }
}

/// Compression strategy configuration
#[derive(Debug, Clone)]
pub struct CompressionStrategy {
    pub retention_ratio: f64,
    pub importance_threshold: f64,
    pub age_threshold: u64,
}

/// Memory thresholds for triggering compression
#[derive(Debug, Clone)]
pub struct MemoryThresholds {
    pub compression_trigger: usize,
    pub conversation_compression_threshold: usize,
    pub workflow_compression_threshold: usize,
    pub agent_compression_threshold: usize,
}

impl Default for MemoryThresholds {
    fn default() -> Self {
        Self {
            compression_trigger: 100 * 1024 * 1024, // 100MB
            conversation_compression_threshold: 10 * 1024 * 1024, // 10MB
            workflow_compression_threshold: 20 * 1024 * 1024, // 20MB
            agent_compression_threshold: 5 * 1024 * 1024, // 5MB
        }
    }
}

/// Context compression result
#[derive(Debug, Clone)]
pub struct ContextCompression {
    pub conversations_to_compress: Vec<String>,
    pub workflows_to_compress: Vec<String>,
    pub agents_to_compress: Vec<String>,
    pub compression_ratio: f64,
}

impl ContextCompression {
    pub fn new() -> Self {
        Self {
            conversations_to_compress: Vec::new(),
            workflows_to_compress: Vec::new(),
            agents_to_compress: Vec::new(),
            compression_ratio: 1.0,
        }
    }
}

/// Memory usage analysis
#[derive(Debug)]
pub struct MemoryAnalysis {
    pub total_memory: usize,
    pub conversation_memory: HashMap<String, usize>,
    pub workflow_memory: HashMap<String, usize>,
    pub agent_memory: HashMap<String, usize>,
}

impl MemoryAnalysis {
    pub fn new() -> Self {
        Self {
            total_memory: 0,
            conversation_memory: HashMap::new(),
            workflow_memory: HashMap::new(),
            agent_memory: HashMap::new(),
        }
    }
}

/// Compressed conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressedConversation {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub key_topics: Vec<String>,
    pub important_decisions: Vec<String>,
    pub compressed_messages: Vec<CompressedMessage>,
    pub participants: Vec<Participant>,
    pub created_at: u64,
    pub last_activity: u64,
    pub original_message_count: usize,
    pub compression_ratio: f64,
}

/// Compressed message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressedMessage {
    pub id: String,
    pub sender: Participant,
    pub content_summary: String,
    pub message_type: MessageType,
    pub timestamp: u64,
    pub importance_score: f64,
}

/// Compressed workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressedWorkflow {
    pub id: String,
    pub name: String,
    pub status: WorkflowStatus,
    pub execution_summary: String,
    pub performance_metrics: WorkflowMetrics,
    pub compressed_steps: Vec<CompressedWorkflowStep>,
    pub variables: HashMap<String, serde_json::Value>,
    pub created_at: u64,
    pub completed_at: Option<u64>,
    pub original_step_count: usize,
    pub compression_ratio: f64,
}

/// Compressed workflow step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressedWorkflowStep {
    pub id: String,
    pub name: String,
    pub step_type: WorkflowStepType,
    pub status: StepStatus,
    pub execution_summary: String,
    pub importance_score: f64,
}

/// Workflow metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowMetrics {
    pub total_execution_time: u64,
    pub success_rate: f64,
    pub error_count: u32,
}

/// Compressed agent memory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressedAgentMemory {
    pub compressed_short_term: Vec<CompressedMemoryItem>,
    pub compressed_long_term: Vec<CompressedMemoryItem>,
    pub memory_patterns: Vec<MemoryPattern>,
    pub working_memory: HashMap<String, serde_json::Value>,
    pub original_short_term_count: usize,
    pub original_long_term_count: usize,
    pub compression_ratio: f64,
}

/// Compressed memory item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressedMemoryItem {
    pub id: String,
    pub content_summary: String,
    pub item_type: MemoryType,
    pub importance: f64,
    pub created_at: u64,
    pub access_pattern: AccessPattern,
    pub combined_score: f64,
}

/// Access pattern analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessPattern {
    pub frequency: f64,
    pub recency: f64,
    pub pattern_type: String,
}

/// Memory pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryPattern {
    pub pattern_type: String,
    pub description: String,
    pub strength: f64,
}

/// Compression result tracking (extended for workflow integration)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionResultOld {
    pub compression_type: String,
    pub original_size: usize,
    pub compressed_size: usize,
    pub compression_ratio: f64,
    pub timestamp: u64,
}
