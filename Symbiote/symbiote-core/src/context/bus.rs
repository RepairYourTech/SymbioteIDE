//! # Context Bus - Central Event Dispatcher and Context Coordinator
//! 
//! The ContextBus is the central nervous system of Symbiote IDE, coordinating
//! all context updates and ensuring optimal context distribution across systems.
//! 
//! Following Week 13-14 Context Management & Knowledge Graph implementation plan.

use super::*;
use crate::{Result, SymbioteError};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};

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

    pub fn start_processing(&mut self) {
        let event_queue = self.event_queue.clone();
        let subscribers = self.subscribers.clone();

        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(100));
            
            loop {
                interval.tick().await;
                
                let mut queue = event_queue.write().await;
                if let Some(update) = queue.pop_front() {
                    drop(queue);
                    
                    let subscribers_guard = subscribers.read().await;
                    for subscriber in subscribers_guard.values() {
                        if subscriber.filter.matches(&update) {
                            let _ = subscriber.sender.send(update.clone());
                        }
                    }
                }
            }
        });

        self.processing_handle = Some(handle);
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
        }
    }

    /// Update global context and notify subscribers
    pub async fn update_context(&self, update: ContextUpdate) -> Result<()> {
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

    /// Start the context bus processing
    pub async fn start(&mut self) -> Result<()> {
        self.event_dispatcher.start_processing();
        self.knowledge_graph.start_processing().await?;
        Ok(())
    }

    async fn notify_subscribers(&self, update: &ContextUpdate) -> Result<()> {
        self.event_dispatcher.dispatch(update.clone()).await
    }

    async fn optimize_context(&self) -> Result<()> {
        let mut context = self.context_store.write().await;

        // Compress old context
        let compressed = self.compression_engine.compress_old_context(&context).await?;
        context.apply_compression(compressed)?;

        // Optimize for current workload
        let optimized = self.optimization_engine.optimize_global_context(&context).await?;
        context.apply_optimization(optimized)?;

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
