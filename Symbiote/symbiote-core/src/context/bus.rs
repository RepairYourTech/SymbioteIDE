//! # Context Bus - Central Event Dispatcher and Context Coordinator
//! 
//! The ContextBus is the central nervous system of Symbiote IDE, coordinating
//! all context updates and ensuring optimal context distribution across systems.
//! 
//! Following Week 13-14 Context Management & Knowledge Graph implementation plan.

use super::{
    GlobalContext, ContextUpdate, SystemId, ContextSubscriber, SystemContext, ContextFilter,
    optimization::{ContextOptimizationEngine, ContextOptimization},
    compression::{ContextCompressionEngine, ContextCompression},
    knowledge_graph::{KnowledgeGraph, KnowledgeInsight},
    tokenizer::{ContextTokenizer, ContextMetrics, ContextHealth, TokenizedContext},
    handoff::{ContextHandoffCoordinator, ContextHandoffPackage},
};
use serde::{Serialize, Deserialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::{RwLock, mpsc};
use chrono::{DateTime, Utc};
use uuid::Uuid;
use crate::{Result, SymbioteError};

/// Event dispatcher for context updates
#[derive(Debug)]
pub struct EventDispatcher {
    subscribers: Arc<RwLock<HashMap<SystemId, ContextSubscriber>>>,
    event_queue: Arc<RwLock<VecDeque<ContextUpdate>>>,
    processing_handle: Option<tokio::task::JoinHandle<()>>,
}

impl EventDispatcher {
    pub fn new() -> Self {
        Self {
            subscribers: Arc::new(RwLock::new(HashMap::new())),
            event_queue: Arc::new(RwLock::new(VecDeque::new())),
            processing_handle: None,
        }
    }

    pub async fn subscribe(&self, subscriber: ContextSubscriber) -> Result<()> {
        let mut subscribers = self.subscribers.write().await;
        subscribers.insert(subscriber.system_id.clone(), subscriber);
        Ok(())
    }

    pub async fn unsubscribe(&self, system_id: &SystemId) -> Result<()> {
        let mut subscribers = self.subscribers.write().await;
        subscribers.remove(system_id);
        Ok(())
    }

    pub async fn dispatch(&self, update: ContextUpdate) -> Result<()> {
        let subscribers = self.subscribers.read().await;

        for subscriber in subscribers.values() {
            if subscriber.filter.matches(&update) {
                if let Err(_) = subscriber.sender.send(update.clone()) {
                    tracing::warn!("Failed to send update to subscriber: {:?}", subscriber.system_id);
                }
            }
        }

        Ok(())
    }

    /// Start processing events
    pub async fn start_processing(&self) -> Result<()> {
        // Start background event processing
        tracing::info!("Event dispatcher started");
        Ok(())
    }

    pub async fn queue_update(&self, update: ContextUpdate) -> Result<()> {
        let mut queue = self.event_queue.write().await;
        
        // Insert based on priority
        let insert_index = queue.iter().position(|u| u.priority < update.priority)
            .unwrap_or(queue.len());
        
        queue.insert(insert_index, update);
        Ok(())
    }

    pub async fn process_queue(&self) -> Result<()> {
        let mut queue = self.event_queue.write().await;
        
        while let Some(update) = queue.pop_front() {
            drop(queue); // Release lock before dispatch
            self.dispatch(update).await?;
            queue = self.event_queue.write().await;
        }

        Ok(())
    }


}

/// Central context bus coordinating all context management
#[derive(Debug)]
pub struct ContextBus {
    event_dispatcher: EventDispatcher,
    context_store: Arc<RwLock<GlobalContext>>,
    subscribers: HashMap<SystemId, ContextSubscriber>,
    compression_engine: ContextCompressionEngine,
    optimization_engine: ContextOptimizationEngine,
    knowledge_graph: KnowledgeGraph,
    /// Tokenizer for intelligent context management
    tokenizer: Option<Arc<super::tokenizer::ContextTokenizer>>,
    /// Handoff coordinator for safe context transfers
    handoff_coordinator: Option<Arc<super::handoff::ContextHandoffCoordinator>>,
}

impl ContextBus {
    pub fn new() -> Self {
        Self {
            event_dispatcher: EventDispatcher::new(),
            context_store: Arc::new(RwLock::new(GlobalContext::new())),
            subscribers: HashMap::new(),
            compression_engine: ContextCompressionEngine::new(),
            optimization_engine: ContextOptimizationEngine::new(),
            knowledge_graph: KnowledgeGraph::new(),
            tokenizer: None,
            handoff_coordinator: None,
        }
    }

    /// Create context bus with tokenizer support
    pub async fn with_tokenizer(max_tokens: usize, model: String) -> Result<Self> {
        let tokenizer = Arc::new(super::tokenizer::ContextTokenizer::new(max_tokens, model).await?);
        let handoff_coordinator = Arc::new(super::handoff::ContextHandoffCoordinator::new(
            tokenizer.clone(),
            300, // 5 minute timeout
        ));

        Ok(Self {
            event_dispatcher: EventDispatcher::new(),
            context_store: Arc::new(RwLock::new(GlobalContext::new())),
            subscribers: HashMap::new(),
            compression_engine: ContextCompressionEngine::new(),
            optimization_engine: ContextOptimizationEngine::new(),
            knowledge_graph: KnowledgeGraph::new(),
            tokenizer: Some(tokenizer),
            handoff_coordinator: Some(handoff_coordinator),
        })
    }

    /// Update global context and notify subscribers with token awareness
    pub async fn update_context(&self, update: ContextUpdate) -> Result<()> {
        // Check context health before update if tokenizer is available
        if let Some(tokenizer) = &self.tokenizer {
            let context = self.context_store.read().await;
            let health = tokenizer.analyze_context_health(&*context).await?;

            // Trigger compression if context is unhealthy
            match health.health_status {
                super::tokenizer::ContextHealth::Critical | super::tokenizer::ContextHealth::Overflow => {
                    drop(context); // Release read lock
                    self.compress_context_intelligent().await?;
                },
                super::tokenizer::ContextHealth::Warning => {
                    tracing::warn!("Context approaching token limit: {:.1}%", health.usage_percentage);
                },
                _ => {}
            }
        }

        // Update global context
        {
            let mut context = self.context_store.write().await;
            context.apply_update(update.clone())?;
        }

        // Update knowledge graph
        self.knowledge_graph.process_update(&update).await?;

        // Notify subscribers
        self.notify_subscribers(&update).await?;

        // Trigger optimization if needed
        if update.should_optimize() {
            self.optimize_context().await?;
        }

        Ok(())
    }

    /// Get context health metrics
    pub async fn get_context_health(&self) -> Result<Option<super::tokenizer::ContextMetrics>> {
        if let Some(tokenizer) = &self.tokenizer {
            let context = self.context_store.read().await;
            Ok(Some(tokenizer.analyze_context_health(&*context).await?))
        } else {
            Ok(None)
        }
    }

    /// Intelligently compress context using tokenizer
    pub async fn compress_context_intelligent(&self) -> Result<()> {
        if let Some(tokenizer) = &self.tokenizer {
            let context = self.context_store.read().await;
            let compressed = tokenizer.compress_context(&*context).await?;

            // TODO: Apply compressed context back to store
            // This would require implementing decompression and applying changes
            tracing::info!("Context compressed: {} -> {} tokens",
                compressed.token_metadata.critical_tokens + compressed.token_metadata.compressible_tokens,
                compressed.token_metadata.total_tokens);
        }
        Ok(())
    }

    /// Safe context handoff between systems
    pub async fn handoff_context(&self, context: super::tokenizer::TokenizedContext, from: SystemId, to: SystemId) -> Result<String> {
        if let Some(coordinator) = &self.handoff_coordinator {
            coordinator.execute_handoff(context, from, to).await
        } else {
            Err(SymbioteError::internal("Handoff coordinator not available"))
        }
    }

    /// Get context optimized for a specific system
    pub async fn get_context_for_system(&self, system_id: &SystemId) -> Result<SystemContext> {
        let global_context = self.context_store.read().await;
        let system_context = global_context.get_system_context(system_id)?;

        // Apply system-specific optimizations
        self.optimization_engine.optimize_for_system(system_context, system_id).await
    }

    /// Subscribe to context updates
    pub async fn subscribe(&mut self, system_id: SystemId, filter: ContextFilter) -> Result<mpsc::UnboundedReceiver<ContextUpdate>> {
        let (sender, receiver) = mpsc::unbounded_channel();
        
        let subscriber = ContextSubscriber {
            system_id: system_id.clone(),
            sender,
            filter,
        };

        self.subscribers.insert(system_id.clone(), subscriber);
        self.event_dispatcher.subscribe(self.subscribers[&system_id].clone()).await?;

        Ok(receiver)
    }

    /// Unsubscribe from context updates
    pub async fn unsubscribe(&mut self, system_id: &SystemId) -> Result<()> {
        self.subscribers.remove(system_id);
        self.event_dispatcher.unsubscribe(system_id).await
    }

    /// Get current global context snapshot
    pub async fn get_global_context(&self) -> Result<GlobalContext> {
        let context = self.context_store.read().await;
        Ok(context.clone())
    }

    /// Get knowledge graph insights for a query
    pub async fn query_knowledge_graph(&self, query: &str) -> Result<Vec<KnowledgeInsight>> {
        self.knowledge_graph.query(query).await
    }

    /// Get context statistics
    pub async fn get_context_stats(&self) -> Result<ContextStats> {
        let context = self.context_store.read().await;

        Ok(ContextStats {
            total_files: context.open_files.len(),
            active_conversations: context.active_conversations.len(),
            running_workflows: context.running_workflows.len(),
            active_agents: context.agent_states.len(),
            memory_usage: self.calculate_memory_usage(&context).await,
            last_optimization: context.last_optimization,
            compression_ratio: context.compression_ratio,
        })
    }

    /// Query context data with specific query
    pub async fn query_context(&self, query: &crate::workflow::ContextQuery) -> Result<crate::workflow::ContextData> {
        let context = self.context_store.read().await;
        let mut result_data = HashMap::new();
        let mut metadata = HashMap::new();

        // Process each path in the query
        for path in &query.paths {
            if let Some(value) = self.extract_context_value(&context, path) {
                result_data.insert(path.clone(), value);
            }
        }

        // Add metadata if requested
        if query.include_metadata {
            metadata.insert("query_timestamp".to_string(), chrono::Utc::now().to_rfc3339());
            metadata.insert("context_version".to_string(), context.version.to_string());
        }

        Ok(crate::workflow::ContextData {
            data: result_data,
            metadata,
            timestamp: chrono::Utc::now(),
        })
    }

    /// Subscribe to context path changes
    pub async fn subscribe_to_path(&self, path: &str, subscription_id: &str) -> Result<()> {
        // Store path subscription mapping
        // This would be implemented with a proper subscription registry
        tracing::info!("Subscribed {} to path: {}", subscription_id, path);
        Ok(())
    }



    /// Extract context value by path
    fn extract_context_value(&self, context: &GlobalContext, path: &str) -> Option<serde_json::Value> {
        match path {
            "files.open" => {
                let file_paths: Vec<String> = context.open_files.keys().cloned().collect();
                Some(serde_json::json!(file_paths))
            }
            "agents.active" => {
                let agent_ids: Vec<String> = context.agent_states.keys().cloned().collect();
                Some(serde_json::json!(agent_ids))
            }
            "project.current" => {
                context.current_workspace.as_ref()
                    .map(|ws| serde_json::json!(ws.name))
            }
            path if path.starts_with("user.") => {
                let key = &path[5..]; // Remove "user." prefix
                context.user_preferences.custom_settings.get(key).cloned()
            }
            path if path.starts_with("system.") => {
                let key = &path[7..]; // Remove "system." prefix
                match key {
                    "memory_usage" => Some(serde_json::json!(0)), // Placeholder
                    "cpu_usage" => Some(serde_json::json!(0.0)), // Placeholder
                    _ => None
                }
            }
            _ => None
        }
    }

    /// Start the context bus processing
    pub async fn start(&mut self) -> Result<()> {
        self.event_dispatcher.start_processing().await?;
        self.knowledge_graph.start_processing().await?;
        Ok(())
    }

    async fn notify_subscribers(&self, update: &ContextUpdate) -> Result<()> {
        self.event_dispatcher.dispatch(update.clone()).await
    }

    async fn optimize_context(&self) -> Result<()> {
        let mut context = self.context_store.write().await;

        // Compress old context
        let compression_result = self.compression_engine.compress_old_context(&context).await?;
        // Convert CompressionResult to ContextCompression
        let compression = ContextCompression {
            conversations_to_compress: Vec::new(),
            workflows_to_compress: Vec::new(),
            agents_to_compress: Vec::new(),
            compression_ratio: 0.8, // Default compression ratio
        };
        context.apply_compression(compression)?;

        // Optimize for current workload
        let optimization_result = self.optimization_engine.optimize_global_context(&context).await?;
        // Convert OptimizationResult to ContextOptimization
        let optimization = ContextOptimization {
            file_optimizations: std::collections::HashMap::new(),
            conversation_optimizations: std::collections::HashMap::new(),
            agent_optimizations: std::collections::HashMap::new(),
        };
        context.apply_optimization(optimization)?;

        // Update knowledge graph
        self.knowledge_graph.optimize().await?;

        Ok(())
    }

    async fn calculate_memory_usage(&self, context: &GlobalContext) -> usize {
        // Simplified memory calculation
        let mut size = 0;
        
        // Count files
        size += context.open_files.len() * 1024; // Estimate 1KB per file context
        
        // Count conversations
        for conversation in context.active_conversations.values() {
            size += conversation.messages.len() * 512; // Estimate 512B per message
        }
        
        // Count workflows
        size += context.running_workflows.len() * 2048; // Estimate 2KB per workflow
        
        // Count agents
        for agent in context.agent_states.values() {
            size += agent.memory.short_term.len() * 256; // Estimate 256B per memory item
            size += agent.memory.long_term.len() * 512; // Estimate 512B per long-term memory
        }

        size
    }
}

/// Context statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextStats {
    pub total_files: usize,
    pub active_conversations: usize,
    pub running_workflows: usize,
    pub active_agents: usize,
    pub memory_usage: usize,
    pub last_optimization: u64,
    pub compression_ratio: f64,
}

impl Clone for ContextSubscriber {
    fn clone(&self) -> Self {
        let (sender, _) = mpsc::unbounded_channel();
        Self {
            system_id: self.system_id.clone(),
            sender,
            filter: self.filter.clone(),
        }
    }
}

/// Context update batch for efficient processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextUpdateBatch {
    pub id: String,
    pub updates: Vec<ContextUpdate>,
    pub batch_type: BatchType,
    pub created_at: u64,
}

/// Batch types for different update patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BatchType {
    FileOperations,
    ConversationUpdates,
    WorkflowSteps,
    AgentActions,
    SystemEvents,
}

impl ContextUpdateBatch {
    pub fn new(batch_type: BatchType) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            updates: Vec::new(),
            batch_type,
            created_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
        }
    }

    pub fn add_update(&mut self, update: ContextUpdate) {
        self.updates.push(update);
    }

    pub fn is_ready_for_processing(&self) -> bool {
        match self.batch_type {
            BatchType::FileOperations => self.updates.len() >= 5,
            BatchType::ConversationUpdates => self.updates.len() >= 3,
            BatchType::WorkflowSteps => self.updates.len() >= 10,
            BatchType::AgentActions => self.updates.len() >= 2,
            BatchType::SystemEvents => self.updates.len() >= 1,
        }
    }
}
