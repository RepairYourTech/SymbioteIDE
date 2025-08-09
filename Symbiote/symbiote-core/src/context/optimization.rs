//! # Context Optimization Engine
//! 
//! Intelligent optimization of context data for improved performance and relevance.
//! Provides system-specific optimizations and global context optimization.
//! 
//! Following Week 13-14 Context Management & Knowledge Graph implementation plan.

use super::*;
use crate::{Result, SymbioteError};
use std::collections::HashMap;

/// Context optimization engine
#[derive(Debug)]
pub struct ContextOptimizationEngine {
    optimization_strategies: HashMap<String, OptimizationStrategy>,
    performance_metrics: PerformanceTracker,
    learning_engine: OptimizationLearningEngine,
}

impl ContextOptimizationEngine {
    pub fn new() -> Self {
        Self {
            optimization_strategies: Self::default_strategies(),
            performance_metrics: PerformanceTracker::new(),
            learning_engine: OptimizationLearningEngine::new(),
        }
    }

    /// Optimize context for a specific system
    pub async fn optimize_for_system(&self, mut context: SystemContext, system_id: &SystemId) -> Result<SystemContext> {
        let strategy = self.get_strategy_for_system(system_id);
        
        // Apply system-specific optimizations
        context = self.apply_file_optimizations(context, &strategy).await?;
        context = self.apply_conversation_optimizations(context, &strategy).await?;
        context = self.apply_workflow_optimizations(context, &strategy).await?;
        context = self.apply_agent_optimizations(context, &strategy).await?;

        // Learn from optimization results
        self.learning_engine.record_optimization(&context, system_id).await?;

        Ok(context)
    }

    /// Optimize global context
    pub async fn optimize_global_context(&self, context: &GlobalContext) -> Result<ContextOptimization> {
        let mut optimization = ContextOptimization::new();

        // Analyze context usage patterns
        let usage_patterns = self.analyze_usage_patterns(context).await?;

        // Generate file optimizations
        for (file_path, file_context) in &context.open_files {
            if let Some(file_optimization) = self.optimize_file_context(file_context, &usage_patterns).await? {
                optimization.file_optimizations.insert(file_path.clone(), file_optimization);
            }
        }

        // Generate conversation optimizations
        for (conversation_id, conversation) in &context.active_conversations {
            if let Some(conversation_optimization) = self.optimize_conversation_context(conversation, &usage_patterns).await? {
                optimization.conversation_optimizations.insert(conversation_id.clone(), conversation_optimization);
            }
        }

        // Generate agent optimizations
        for (agent_id, agent) in &context.agent_states {
            if let Some(agent_optimization) = self.optimize_agent_context(agent, &usage_patterns).await? {
                optimization.agent_optimizations.insert(agent_id.clone(), agent_optimization);
            }
        }

        Ok(optimization)
    }

    fn get_strategy_for_system(&self, system_id: &SystemId) -> &OptimizationStrategy {
        self.optimization_strategies.get(&system_id.name)
            .unwrap_or(&self.optimization_strategies["default"])
    }

    async fn apply_file_optimizations(&self, mut context: SystemContext, strategy: &OptimizationStrategy) -> Result<SystemContext> {
        // Sort files by relevance
        let mut file_relevance: Vec<_> = context.file_contexts.iter()
            .map(|(path, file_context)| {
                let relevance = self.calculate_file_relevance(file_context, &context);
                (path.clone(), relevance)
            })
            .collect();

        file_relevance.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        // Keep only the most relevant files based on strategy
        let max_files = strategy.max_files_per_system;
        if file_relevance.len() > max_files {
            let files_to_keep: std::collections::HashSet<_> = file_relevance
                .into_iter()
                .take(max_files)
                .map(|(path, _)| path)
                .collect();

            context.file_contexts.retain(|path, _| files_to_keep.contains(path));
        }

        Ok(context)
    }

    async fn apply_conversation_optimizations(&self, mut context: SystemContext, strategy: &OptimizationStrategy) -> Result<SystemContext> {
        // Keep only recent and relevant conversations
        let max_conversations = strategy.max_conversations_per_system;
        if context.conversation_contexts.len() > max_conversations {
            let mut conversations: Vec<_> = context.conversation_contexts.into_iter().collect();
            conversations.sort_by_key(|(_, conv)| std::cmp::Reverse(conv.last_activity));
            
            context.conversation_contexts = conversations
                .into_iter()
                .take(max_conversations)
                .collect();
        }

        Ok(context)
    }

    async fn apply_workflow_optimizations(&self, mut context: SystemContext, strategy: &OptimizationStrategy) -> Result<SystemContext> {
        // Keep only active and recent workflows
        let max_workflows = strategy.max_workflows_per_system;
        if context.workflow_contexts.len() > max_workflows {
            let mut workflows: Vec<_> = context.workflow_contexts.into_iter().collect();
            workflows.sort_by_key(|(_, workflow)| {
                match workflow.status {
                    WorkflowStatus::Running => 0,
                    WorkflowStatus::Paused => 1,
                    _ => 2,
                }
            });
            
            context.workflow_contexts = workflows
                .into_iter()
                .take(max_workflows)
                .collect();
        }

        Ok(context)
    }

    async fn apply_agent_optimizations(&self, mut context: SystemContext, strategy: &OptimizationStrategy) -> Result<SystemContext> {
        // Optimize agent memory
        for agent in context.agent_contexts.values_mut() {
            self.optimize_agent_memory(&mut agent.memory, strategy).await?;
        }

        Ok(context)
    }

    async fn optimize_agent_memory(&self, memory: &mut AgentMemory, strategy: &OptimizationStrategy) -> Result<()> {
        // Limit short-term memory
        while memory.short_term.len() > strategy.max_short_term_memory {
            memory.short_term.pop_front();
        }

        // Promote important short-term memories to long-term
        let important_memories: Vec<_> = memory.short_term.iter()
            .filter(|item| item.importance > 0.8)
            .cloned()
            .collect();

        for memory_item in important_memories {
            memory.long_term.insert(memory_item.id.clone(), memory_item);
        }

        // Limit long-term memory
        if memory.long_term.len() > strategy.max_long_term_memory {
            let mut long_term_items: Vec<_> = memory.long_term.values().cloned().collect();
            long_term_items.sort_by(|a, b| b.importance.partial_cmp(&a.importance).unwrap());
            
            memory.long_term.clear();
            for item in long_term_items.into_iter().take(strategy.max_long_term_memory) {
                memory.long_term.insert(item.id.clone(), item);
            }
        }

        Ok(())
    }

    fn calculate_file_relevance(&self, file_context: &FileContext, system_context: &SystemContext) -> f64 {
        let mut relevance = 0.0;

        // Recent activity boost
        let time_since_modified = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() - file_context.last_modified;
        relevance += (1.0 / (1.0 + time_since_modified as f64 / 3600.0)) * 0.3; // Decay over hours

        // Workspace relevance
        if let Some(ref workspace) = system_context.workspace_context {
            if file_context.path.starts_with(&workspace.path) {
                relevance += 0.2;
            }
        }

        // Symbol count (complexity indicator)
        relevance += (file_context.symbols.len() as f64 / 100.0).min(0.2);

        // Import/export connections
        relevance += ((file_context.imports.len() + file_context.exports.len()) as f64 / 50.0).min(0.3);

        relevance.min(1.0)
    }

    async fn analyze_usage_patterns(&self, context: &GlobalContext) -> Result<UsagePatterns> {
        let mut patterns = UsagePatterns::new();

        // Analyze file access patterns
        for file_context in context.open_files.values() {
            patterns.file_access_frequency.insert(
                file_context.path.clone(),
                self.calculate_file_access_frequency(file_context)
            );
        }

        // Analyze conversation patterns
        for conversation in context.active_conversations.values() {
            patterns.conversation_activity.insert(
                conversation.id.clone(),
                conversation.messages.len() as f64
            );
        }

        // Analyze agent usage patterns
        for agent in context.agent_states.values() {
            patterns.agent_usage.insert(
                agent.id.clone(),
                agent.performance_metrics.tasks_completed as f64
            );
        }

        Ok(patterns)
    }

    fn calculate_file_access_frequency(&self, _file_context: &FileContext) -> f64 {
        // Simplified calculation - in reality would track actual access patterns
        1.0
    }

    async fn optimize_file_context(&self, _file_context: &FileContext, _patterns: &UsagePatterns) -> Result<Option<FileOptimization>> {
        // Generate file-specific optimizations
        Ok(Some(FileOptimization {
            reduce_symbol_detail: false,
            cache_parsed_content: true,
            preload_dependencies: false,
        }))
    }

    async fn optimize_conversation_context(&self, _conversation: &ConversationContext, _patterns: &UsagePatterns) -> Result<Option<ConversationOptimization>> {
        // Generate conversation-specific optimizations
        Ok(Some(ConversationOptimization {
            compress_old_messages: true,
            cache_frequent_responses: true,
            preload_context_files: false,
        }))
    }

    async fn optimize_agent_context(&self, _agent: &AgentContext, _patterns: &UsagePatterns) -> Result<Option<AgentOptimization>> {
        // Generate agent-specific optimizations
        Ok(Some(AgentOptimization {
            compress_memory: true,
            cache_frequent_patterns: true,
            optimize_performance_tracking: true,
        }))
    }

    fn default_strategies() -> HashMap<String, OptimizationStrategy> {
        let mut strategies = HashMap::new();

        strategies.insert("default".to_string(), OptimizationStrategy {
            max_files_per_system: 50,
            max_conversations_per_system: 10,
            max_workflows_per_system: 5,
            max_short_term_memory: 100,
            max_long_term_memory: 1000,
            optimization_frequency: OptimizationFrequency::Moderate,
        });

        strategies.insert("ai_provider".to_string(), OptimizationStrategy {
            max_files_per_system: 20,
            max_conversations_per_system: 5,
            max_workflows_per_system: 3,
            max_short_term_memory: 50,
            max_long_term_memory: 500,
            optimization_frequency: OptimizationFrequency::High,
        });

        strategies.insert("editor".to_string(), OptimizationStrategy {
            max_files_per_system: 100,
            max_conversations_per_system: 20,
            max_workflows_per_system: 10,
            max_short_term_memory: 200,
            max_long_term_memory: 2000,
            optimization_frequency: OptimizationFrequency::Low,
        });

        strategies
    }
}

/// Optimization strategy for different systems
#[derive(Debug, Clone)]
pub struct OptimizationStrategy {
    pub max_files_per_system: usize,
    pub max_conversations_per_system: usize,
    pub max_workflows_per_system: usize,
    pub max_short_term_memory: usize,
    pub max_long_term_memory: usize,
    pub optimization_frequency: OptimizationFrequency,
}

/// Context optimization result
#[derive(Debug, Clone)]
pub struct ContextOptimization {
    pub file_optimizations: HashMap<String, FileOptimization>,
    pub conversation_optimizations: HashMap<String, ConversationOptimization>,
    pub agent_optimizations: HashMap<String, AgentOptimization>,
}

impl ContextOptimization {
    pub fn new() -> Self {
        Self {
            file_optimizations: HashMap::new(),
            conversation_optimizations: HashMap::new(),
            agent_optimizations: HashMap::new(),
        }
    }
}

/// File-specific optimization
#[derive(Debug, Clone)]
pub struct FileOptimization {
    pub reduce_symbol_detail: bool,
    pub cache_parsed_content: bool,
    pub preload_dependencies: bool,
}

/// Conversation-specific optimization
#[derive(Debug, Clone)]
pub struct ConversationOptimization {
    pub compress_old_messages: bool,
    pub cache_frequent_responses: bool,
    pub preload_context_files: bool,
}

/// Agent-specific optimization
#[derive(Debug, Clone)]
pub struct AgentOptimization {
    pub compress_memory: bool,
    pub cache_frequent_patterns: bool,
    pub optimize_performance_tracking: bool,
}

/// Usage patterns analysis
#[derive(Debug, Clone)]
pub struct UsagePatterns {
    pub file_access_frequency: HashMap<String, f64>,
    pub conversation_activity: HashMap<String, f64>,
    pub agent_usage: HashMap<String, f64>,
}

impl UsagePatterns {
    pub fn new() -> Self {
        Self {
            file_access_frequency: HashMap::new(),
            conversation_activity: HashMap::new(),
            agent_usage: HashMap::new(),
        }
    }
}

/// Performance tracking for optimization
#[derive(Debug)]
pub struct PerformanceTracker {
    optimization_times: HashMap<String, Vec<u64>>,
    memory_usage: HashMap<String, Vec<usize>>,
    system_response_times: HashMap<String, Vec<u64>>,
}

impl PerformanceTracker {
    pub fn new() -> Self {
        Self {
            optimization_times: HashMap::new(),
            memory_usage: HashMap::new(),
            system_response_times: HashMap::new(),
        }
    }

    pub fn record_optimization_time(&mut self, system_id: &str, time_ms: u64) {
        self.optimization_times.entry(system_id.to_string())
            .or_insert_with(Vec::new)
            .push(time_ms);
    }

    pub fn record_memory_usage(&mut self, system_id: &str, memory_bytes: usize) {
        self.memory_usage.entry(system_id.to_string())
            .or_insert_with(Vec::new)
            .push(memory_bytes);
    }

    pub fn get_average_optimization_time(&self, system_id: &str) -> Option<f64> {
        self.optimization_times.get(system_id)
            .and_then(|times| {
                if times.is_empty() {
                    None
                } else {
                    Some(times.iter().sum::<u64>() as f64 / times.len() as f64)
                }
            })
    }
}

/// Learning engine for optimization improvement
#[derive(Debug)]
pub struct OptimizationLearningEngine {
    optimization_history: HashMap<String, Vec<OptimizationResult>>,
    performance_correlations: HashMap<String, f64>,
}

impl OptimizationLearningEngine {
    pub fn new() -> Self {
        Self {
            optimization_history: HashMap::new(),
            performance_correlations: HashMap::new(),
        }
    }

    pub async fn record_optimization(&self, _context: &SystemContext, _system_id: &SystemId) -> Result<()> {
        // Record optimization results for learning
        Ok(())
    }

    pub async fn suggest_improvements(&self, _system_id: &SystemId) -> Result<Vec<OptimizationSuggestion>> {
        // Generate optimization suggestions based on learning
        Ok(vec![])
    }
}

/// Optimization result for learning
#[derive(Debug, Clone)]
pub struct OptimizationResult {
    pub system_id: String,
    pub optimization_type: String,
    pub performance_improvement: f64,
    pub memory_reduction: f64,
    pub timestamp: u64,
}

/// Optimization suggestion
#[derive(Debug, Clone)]
pub struct OptimizationSuggestion {
    pub suggestion_type: String,
    pub description: String,
    pub expected_improvement: f64,
    pub confidence: f64,
}
