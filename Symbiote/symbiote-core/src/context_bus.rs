//! # High-Performance Context Bus for Symbiote IDE
//! 
//! Event handling system targeting 1000+ events/second with real-time processing,
//! intelligent routing, and context-aware distribution.
//! 
//! Following Week 7-8 Context Management & Agent Orchestration implementation plan.

use crate::{Result, SymbioteError, ProjectId, UserId, AgentId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::{broadcast, mpsc, RwLock, Semaphore};
use tokio::time::timeout;
use uuid::Uuid;
use dashmap::DashMap;

/// Context event types for the bus
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContextEvent {
    /// User context events
    UserLogin { user_id: UserId, session_id: String, timestamp: u64 },
    UserLogout { user_id: UserId, session_id: String, timestamp: u64 },
    UserActivity { user_id: UserId, activity: String, metadata: HashMap<String, String> },
    
    /// Project context events
    ProjectOpened { project_id: ProjectId, user_id: UserId, timestamp: u64 },
    ProjectClosed { project_id: ProjectId, user_id: UserId, timestamp: u64 },
    ProjectModified { project_id: ProjectId, user_id: UserId, changes: Vec<String> },
    
    /// File context events
    FileOpened { project_id: ProjectId, file_path: String, user_id: UserId },
    FileModified { project_id: ProjectId, file_path: String, user_id: UserId, changes: String },
    FileSaved { project_id: ProjectId, file_path: String, user_id: UserId },
    FileClosed { project_id: ProjectId, file_path: String, user_id: UserId },
    
    /// AI context events
    AIRequestStarted { request_id: String, agent_id: AgentId, user_id: UserId, context_size: u32 },
    AIRequestCompleted { request_id: String, agent_id: AgentId, duration_ms: u64, tokens_used: u32 },
    AIRequestFailed { request_id: String, agent_id: AgentId, error: String },
    
    /// Agent context events
    AgentStarted { agent_id: AgentId, agent_type: String, project_id: Option<ProjectId> },
    AgentStopped { agent_id: AgentId, reason: String },
    AgentError { agent_id: AgentId, error: String, recoverable: bool },
    
    /// System context events
    SystemStartup { timestamp: u64 },
    SystemShutdown { timestamp: u64 },
    PerformanceMetric { metric_name: String, value: f64, timestamp: u64 },
    
    /// Custom events
    Custom { event_type: String, data: HashMap<String, serde_json::Value> },
}

/// Event priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum EventPriority {
    Low = 1,
    Normal = 2,
    High = 3,
    Critical = 4,
}

/// Context event wrapper with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextEventWrapper {
    /// Unique event ID
    pub id: Uuid,
    
    /// Event content
    pub event: ContextEvent,
    
    /// Event priority
    pub priority: EventPriority,
    
    /// Event timestamp
    pub timestamp: u64,
    
    /// Source component
    pub source: String,
    
    /// Target components (empty = broadcast)
    pub targets: Vec<String>,
    
    /// Event metadata
    pub metadata: HashMap<String, String>,
    
    /// Processing deadline
    pub deadline: Option<u64>,
    
    /// Retry count
    pub retry_count: u32,
}

/// Event handler trait
#[async_trait::async_trait]
pub trait ContextEventHandler: Send + Sync {
    /// Handler name
    fn name(&self) -> &str;
    
    /// Event types this handler is interested in
    fn event_types(&self) -> Vec<String>;
    
    /// Handle an event
    async fn handle(&self, event: &ContextEventWrapper) -> Result<()>;
    
    /// Handler priority (higher = processed first)
    fn priority(&self) -> u32 { 100 }
    
    /// Whether this handler can process events concurrently
    fn concurrent(&self) -> bool { true }
}

/// Event processing statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventStats {
    /// Total events processed
    pub total_processed: u64,
    
    /// Events processed per second (current)
    pub events_per_second: f64,
    
    /// Average processing time in microseconds
    pub avg_processing_time_us: u64,
    
    /// Failed events count
    pub failed_events: u64,
    
    /// Retry events count
    pub retry_events: u64,
    
    /// Queue size
    pub queue_size: usize,
    
    /// Active handlers count
    pub active_handlers: usize,
    
    /// Last reset timestamp
    pub last_reset: u64,
}

/// Context bus configuration
#[derive(Debug, Clone)]
pub struct ContextBusConfig {
    /// Maximum queue size
    pub max_queue_size: usize,
    
    /// Maximum concurrent event processors
    pub max_concurrent_processors: usize,
    
    /// Event processing timeout
    pub processing_timeout: Duration,
    
    /// Maximum retry attempts
    pub max_retries: u32,
    
    /// Retry delay
    pub retry_delay: Duration,
    
    /// Statistics update interval
    pub stats_interval: Duration,
    
    /// Whether to persist events
    pub persist_events: bool,
    
    /// Event retention period
    pub retention_period: Duration,
}

impl Default for ContextBusConfig {
    fn default() -> Self {
        Self {
            max_queue_size: 10000,
            max_concurrent_processors: 100,
            processing_timeout: Duration::from_secs(30),
            max_retries: 3,
            retry_delay: Duration::from_millis(100),
            stats_interval: Duration::from_secs(1),
            persist_events: false,
            retention_period: Duration::from_secs(24 * 60 * 60), // 24 hours
        }
    }
}

/// High-performance context bus
pub struct ContextBus {
    /// Configuration
    config: ContextBusConfig,
    
    /// Event handlers by name
    handlers: Arc<DashMap<String, Arc<dyn ContextEventHandler>>>,
    
    /// Event queue sender
    event_sender: mpsc::UnboundedSender<ContextEventWrapper>,
    
    /// Broadcast channel for real-time subscribers
    broadcast_sender: broadcast::Sender<ContextEventWrapper>,
    
    /// Processing semaphore
    processing_semaphore: Arc<Semaphore>,
    
    /// Event statistics
    stats: Arc<RwLock<EventStats>>,
    
    /// Running flag
    running: Arc<RwLock<bool>>,
}

impl ContextBus {
    /// Create a new context bus
    pub fn new(config: ContextBusConfig) -> Self {
        let (event_sender, event_receiver) = mpsc::unbounded_channel();
        let (broadcast_sender, _) = broadcast::channel(1000);
        let processing_semaphore = Arc::new(Semaphore::new(config.max_concurrent_processors));
        
        let bus = Self {
            config: config.clone(),
            handlers: Arc::new(DashMap::new()),
            event_sender,
            broadcast_sender: broadcast_sender.clone(),
            processing_semaphore,
            stats: Arc::new(RwLock::new(EventStats {
                total_processed: 0,
                events_per_second: 0.0,
                avg_processing_time_us: 0,
                failed_events: 0,
                retry_events: 0,
                queue_size: 0,
                active_handlers: 0,
                last_reset: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            })),
            running: Arc::new(RwLock::new(false)),
        };

        // Start event processor
        bus.start_event_processor(event_receiver);
        
        // Start statistics updater
        bus.start_stats_updater();
        
        bus
    }

    /// Register an event handler
    pub async fn register_handler(&self, handler: Arc<dyn ContextEventHandler>) -> Result<()> {
        let name = handler.name().to_string();
        self.handlers.insert(name.clone(), handler);
        
        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.active_handlers = self.handlers.len();
        }
        
        tracing::info!("Registered context event handler: {}", name);
        Ok(())
    }

    /// Unregister an event handler
    pub async fn unregister_handler(&self, name: &str) -> Result<()> {
        self.handlers.remove(name);
        
        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.active_handlers = self.handlers.len();
        }
        
        tracing::info!("Unregistered context event handler: {}", name);
        Ok(())
    }

    /// Publish an event to the bus
    pub async fn publish(&self, event: ContextEvent) -> Result<()> {
        self.publish_with_priority(event, EventPriority::Normal, None).await
    }

    /// Publish an event with specific priority and targets
    pub async fn publish_with_priority(
        &self,
        event: ContextEvent,
        priority: EventPriority,
        targets: Option<Vec<String>>,
    ) -> Result<()> {
        let wrapper = ContextEventWrapper {
            id: Uuid::new_v4(),
            event,
            priority,
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            source: "context_bus".to_string(),
            targets: targets.unwrap_or_default(),
            metadata: HashMap::new(),
            deadline: None,
            retry_count: 0,
        };

        // Send to event queue
        self.event_sender.send(wrapper.clone())
            .map_err(|_| SymbioteError::internal("Failed to send event to queue"))?;

        // Send to broadcast channel for real-time subscribers
        let _ = self.broadcast_sender.send(wrapper);

        Ok(())
    }

    /// Subscribe to real-time events
    pub fn subscribe(&self) -> broadcast::Receiver<ContextEventWrapper> {
        self.broadcast_sender.subscribe()
    }

    /// Get current statistics
    pub async fn stats(&self) -> EventStats {
        self.stats.read().await.clone()
    }

    /// Start the context bus
    pub async fn start(&self) -> Result<()> {
        let mut running = self.running.write().await;
        if *running {
            return Err(SymbioteError::validation("Context bus is already running"));
        }
        *running = true;
        
        tracing::info!("Context bus started");
        Ok(())
    }

    /// Stop the context bus
    pub async fn stop(&self) -> Result<()> {
        let mut running = self.running.write().await;
        if !*running {
            return Err(SymbioteError::validation("Context bus is not running"));
        }
        *running = false;
        
        tracing::info!("Context bus stopped");
        Ok(())
    }

    /// Start event processor task
    fn start_event_processor(&self, mut event_receiver: mpsc::UnboundedReceiver<ContextEventWrapper>) {
        let handlers = self.handlers.clone();
        let semaphore = self.processing_semaphore.clone();
        let stats = self.stats.clone();
        let config = self.config.clone();
        let running = self.running.clone();

        tokio::spawn(async move {
            while let Some(event) = event_receiver.recv().await {
                // Check if still running
                if !*running.read().await {
                    break;
                }

                // Acquire processing permit
                let permit = semaphore.clone().acquire_owned().await.unwrap();
                let handlers = handlers.clone();
                let stats = stats.clone();
                let config = config.clone();

                tokio::spawn(async move {
                    let _permit = permit; // Keep permit alive
                    let start_time = Instant::now();

                    // Process event
                    let result = Self::process_event_internal(event, handlers, config).await;
                    
                    let processing_time = start_time.elapsed();

                    // Update statistics
                    {
                        let mut stats = stats.write().await;
                        stats.total_processed += 1;
                        
                        // Update average processing time
                        let new_avg = (stats.avg_processing_time_us as f64 * 0.9) + 
                                     (processing_time.as_micros() as f64 * 0.1);
                        stats.avg_processing_time_us = new_avg as u64;

                        if result.is_err() {
                            stats.failed_events += 1;
                        }
                    }

                    if let Err(e) = result {
                        tracing::error!("Failed to process context event: {}", e);
                    }
                });
            }
        });
    }

    /// Process a single event
    async fn process_event_internal(
        event: ContextEventWrapper,
        handlers: Arc<DashMap<String, Arc<dyn ContextEventHandler>>>,
        config: ContextBusConfig,
    ) -> Result<()> {
        // Find matching handlers
        let mut matching_handlers = Vec::new();
        
        for handler_entry in handlers.iter() {
            let handler = handler_entry.value();
            let event_type = format!("{:?}", event.event).split('(').next().unwrap_or("").to_string();
            
            if handler.event_types().is_empty() || handler.event_types().contains(&event_type) {
                matching_handlers.push((handler.clone(), handler.priority()));
            }
        }

        // Sort by priority (higher first)
        matching_handlers.sort_by(|a, b| b.1.cmp(&a.1));

        // Process with timeout
        let processing_future = async {
            for (handler, _) in matching_handlers {
                if let Err(e) = handler.handle(&event).await {
                    tracing::error!("Handler {} failed to process event: {}", handler.name(), e);
                }
            }
        };

        timeout(config.processing_timeout, processing_future).await
            .map_err(|_| SymbioteError::timeout("Event processing timeout"))?;

        Ok(())
    }

    /// Start statistics updater task
    fn start_stats_updater(&self) {
        let stats = self.stats.clone();
        let interval = self.config.stats_interval;
        let running = self.running.clone();

        tokio::spawn(async move {
            let mut last_processed = 0u64;
            let mut last_time = Instant::now();

            loop {
                tokio::time::sleep(interval).await;

                if !*running.read().await {
                    break;
                }

                let mut stats = stats.write().await;
                let current_time = Instant::now();
                let time_diff = current_time.duration_since(last_time).as_secs_f64();
                
                if time_diff > 0.0 {
                    let processed_diff = stats.total_processed - last_processed;
                    stats.events_per_second = processed_diff as f64 / time_diff;
                }

                last_processed = stats.total_processed;
                last_time = current_time;
            }
        });
    }
}

impl ContextEventWrapper {
    /// Create a new event wrapper
    pub fn new(event: ContextEvent, source: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            event,
            priority: EventPriority::Normal,
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            source,
            targets: Vec::new(),
            metadata: HashMap::new(),
            deadline: None,
            retry_count: 0,
        }
    }

    /// Set event priority
    pub fn with_priority(mut self, priority: EventPriority) -> Self {
        self.priority = priority;
        self
    }

    /// Set event targets
    pub fn with_targets(mut self, targets: Vec<String>) -> Self {
        self.targets = targets;
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// Set processing deadline
    pub fn with_deadline(mut self, deadline: u64) -> Self {
        self.deadline = Some(deadline);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestHandler {
        name: String,
        events_handled: Arc<RwLock<u32>>,
    }

    impl TestHandler {
        fn new(name: String) -> Self {
            Self {
                name,
                events_handled: Arc::new(RwLock::new(0)),
            }
        }

        async fn events_handled(&self) -> u32 {
            *self.events_handled.read().await
        }
    }

    #[async_trait::async_trait]
    impl ContextEventHandler for TestHandler {
        fn name(&self) -> &str {
            &self.name
        }

        fn event_types(&self) -> Vec<String> {
            vec!["UserLogin".to_string()]
        }

        async fn handle(&self, _event: &ContextEventWrapper) -> Result<()> {
            let mut count = self.events_handled.write().await;
            *count += 1;
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_context_bus_creation() {
        let config = ContextBusConfig::default();
        let bus = ContextBus::new(config);
        
        let stats = bus.stats().await;
        assert_eq!(stats.total_processed, 0);
        assert_eq!(stats.active_handlers, 0);
    }

    #[tokio::test]
    async fn test_handler_registration() {
        let config = ContextBusConfig::default();
        let bus = ContextBus::new(config);
        
        let handler = Arc::new(TestHandler::new("test_handler".to_string()));
        bus.register_handler(handler.clone()).await.unwrap();
        
        let stats = bus.stats().await;
        assert_eq!(stats.active_handlers, 1);
    }

    #[tokio::test]
    async fn test_event_publishing() {
        let config = ContextBusConfig::default();
        let bus = ContextBus::new(config);
        
        let handler = Arc::new(TestHandler::new("test_handler".to_string()));
        bus.register_handler(handler.clone()).await.unwrap();
        bus.start().await.unwrap();
        
        let event = ContextEvent::UserLogin {
            user_id: UserId::new(),
            session_id: "test_session".to_string(),
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
        };
        
        bus.publish(event).await.unwrap();
        
        // Wait a bit for processing
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        assert_eq!(handler.events_handled().await, 1);
    }
}
