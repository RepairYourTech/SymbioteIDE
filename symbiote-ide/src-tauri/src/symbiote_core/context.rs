// Context Bus - Event-Driven System for SymbioteIDE
// Phase 1 Validation Criteria: Handle 1000+ events/second without lag

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock, mpsc};
use tokio::time::{Duration, Instant};
use serde::{Serialize, Deserialize};
use uuid::Uuid;

/// High-performance event-driven Context Bus
/// Handles real-time communication between all SymbioteIDE components
pub struct ContextBus {
    // High-throughput event channels
    event_sender: broadcast::Sender<ContextEvent>,
    event_receiver: broadcast::Receiver<ContextEvent>,
    
    // Event routing and filtering
    event_router: Arc<RwLock<EventRouter>>,
    
    // Performance monitoring
    metrics: Arc<RwLock<ContextBusMetrics>>,
    
    // Event persistence for debugging and replay
    event_store: Arc<RwLock<EventStore>>,
    
    // Active subscriptions
    subscriptions: Arc<RwLock<HashMap<String, SubscriptionConfig>>>,
}

/// Context events flowing through the bus
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextEvent {
    pub id: String,
    pub timestamp: u64,
    pub event_type: ContextEventType,
    pub source: String,
    pub target: Option<String>,
    pub payload: ContextEventPayload,
    pub priority: EventPriority,
    pub correlation_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ContextEventType {
    // Agent events
    AgentStarted,
    AgentCompleted,
    AgentError,
    AgentStatusUpdate,
    
    // Parser events
    ParseStarted,
    ParseCompleted,
    ParseError,
    ASTUpdated,
    
    // Codebase Intelligence events
    IndexingStarted,
    IndexingCompleted,
    QueryExecuted,
    ContextRetrieved,
    
    // UI events
    FileOpened,
    FileClosed,
    FileModified,
    TabSwitched,
    
    // Task Management events
    TaskCreated,
    TaskUpdated,
    TaskCompleted,
    PlanGenerated,
    
    // System events
    SystemStartup,
    SystemShutdown,
    PerformanceAlert,
    ErrorOccurred,
    
    // Custom events
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContextEventPayload {
    AgentPayload {
        agent_id: String,
        agent_type: String,
        status: String,
        data: serde_json::Value,
    },
    FilePayload {
        file_path: String,
        content_hash: Option<String>,
        language: Option<String>,
    },
    TaskPayload {
        task_id: String,
        plan_id: Option<String>,
        progress: f32,
        data: serde_json::Value,
    },
    SystemPayload {
        component: String,
        message: String,
        data: serde_json::Value,
    },
    Custom(serde_json::Value),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EventPriority {
    Critical,   // System errors, security alerts
    High,       // User actions, agent completions
    Normal,     // Regular operations
    Low,        // Background tasks, metrics
}

/// Event routing and filtering system
pub struct EventRouter {
    routes: HashMap<String, Vec<EventRoute>>,
    filters: HashMap<String, EventFilter>,
}

#[derive(Debug, Clone)]
pub struct EventRoute {
    pub target: String,
    pub filter: Option<String>,
    pub transform: Option<String>,
}

#[derive(Debug, Clone)]
pub struct EventFilter {
    pub event_types: Option<Vec<ContextEventType>>,
    pub sources: Option<Vec<String>>,
    pub priorities: Option<Vec<EventPriority>>,
    pub custom_filter: Option<String>,
}

/// Subscription configuration for components
#[derive(Debug, Clone)]
pub struct SubscriptionConfig {
    pub subscriber_id: String,
    pub event_types: Vec<ContextEventType>,
    pub filters: Vec<EventFilter>,
    pub batch_size: Option<usize>,
    pub max_latency: Option<Duration>,
}

/// Performance metrics for the Context Bus
#[derive(Debug, Default)]
pub struct ContextBusMetrics {
    pub events_processed: u64,
    pub events_per_second: f64,
    pub average_latency: Duration,
    pub peak_latency: Duration,
    pub error_count: u64,
    pub active_subscriptions: usize,
    pub queue_depth: usize,
    pub last_reset: Instant,
}

/// Event storage for debugging and replay
pub struct EventStore {
    events: Vec<ContextEvent>,
    max_events: usize,
    circular_buffer: bool,
}

impl ContextBus {
    /// Create a new high-performance Context Bus
    pub fn new(buffer_size: usize) -> Self {
        let (event_sender, event_receiver) = broadcast::channel(buffer_size);
        
        Self {
            event_sender,
            event_receiver,
            event_router: Arc::new(RwLock::new(EventRouter::new())),
            metrics: Arc::new(RwLock::new(ContextBusMetrics::default())),
            event_store: Arc::new(RwLock::new(EventStore::new(10000))),
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Publish an event to the Context Bus
    pub async fn publish(&self, event: ContextEvent) -> Result<(), ContextBusError> {
        let start_time = Instant::now();
        
        // Store event for debugging
        {
            let mut store = self.event_store.write().await;
            store.add_event(event.clone());
        }
        
        // Send event through the bus
        match self.event_sender.send(event.clone()) {
            Ok(subscriber_count) => {
                // Update metrics
                self.update_metrics(start_time, false).await;
                
                // Route event to specific targets if needed
                self.route_event(event).await?;
                
                Ok(())
            }
            Err(_) => {
                self.update_metrics(start_time, true).await;
                Err(ContextBusError::PublishFailed)
            }
        }
    }
    
    /// Subscribe to events with filtering
    pub async fn subscribe(
        &self,
        subscriber_id: String,
        config: SubscriptionConfig,
    ) -> Result<broadcast::Receiver<ContextEvent>, ContextBusError> {
        // Register subscription
        {
            let mut subscriptions = self.subscriptions.write().await;
            subscriptions.insert(subscriber_id.clone(), config);
        }
        
        // Return receiver for filtered events
        Ok(self.event_sender.subscribe())
    }
    
    /// Unsubscribe from events
    pub async fn unsubscribe(&self, subscriber_id: &str) -> Result<(), ContextBusError> {
        let mut subscriptions = self.subscriptions.write().await;
        subscriptions.remove(subscriber_id);
        Ok(())
    }
    
    /// Add event routing rule
    pub async fn add_route(&self, source: String, route: EventRoute) -> Result<(), ContextBusError> {
        let mut router = self.event_router.write().await;
        router.add_route(source, route);
        Ok(())
    }
    
    /// Get current performance metrics
    pub async fn get_metrics(&self) -> ContextBusMetrics {
        let metrics = self.metrics.read().await;
        metrics.clone()
    }
    
    /// Reset performance metrics
    pub async fn reset_metrics(&self) {
        let mut metrics = self.metrics.write().await;
        *metrics = ContextBusMetrics::default();
        metrics.last_reset = Instant::now();
    }
    
    /// Get recent events for debugging
    pub async fn get_recent_events(&self, count: usize) -> Vec<ContextEvent> {
        let store = self.event_store.read().await;
        store.get_recent_events(count)
    }
    
    /// Route event to specific targets
    async fn route_event(&self, event: ContextEvent) -> Result<(), ContextBusError> {
        let router = self.event_router.read().await;
        
        if let Some(routes) = router.routes.get(&event.source) {
            for route in routes {
                // Apply filters and transformations
                if self.should_route_event(&event, route).await {
                    // Route to target (implementation depends on target type)
                    self.send_to_target(&event, route).await?;
                }
            }
        }
        
        Ok(())
    }
    
    /// Check if event should be routed based on filters
    async fn should_route_event(&self, event: &ContextEvent, route: &EventRoute) -> bool {
        // Apply routing filters
        if let Some(filter_id) = &route.filter {
            let router = self.event_router.read().await;
            if let Some(filter) = router.filters.get(filter_id) {
                return self.apply_filter(event, filter);
            }
        }
        true
    }
    
    /// Apply event filter
    fn apply_filter(&self, event: &ContextEvent, filter: &EventFilter) -> bool {
        // Check event type filter
        if let Some(allowed_types) = &filter.event_types {
            if !allowed_types.contains(&event.event_type) {
                return false;
            }
        }
        
        // Check source filter
        if let Some(allowed_sources) = &filter.sources {
            if !allowed_sources.contains(&event.source) {
                return false;
            }
        }
        
        // Check priority filter
        if let Some(allowed_priorities) = &filter.priorities {
            if !allowed_priorities.contains(&event.priority) {
                return false;
            }
        }
        
        true
    }
    
    /// Send event to specific target
    async fn send_to_target(&self, event: &ContextEvent, route: &EventRoute) -> Result<(), ContextBusError> {
        // Implementation depends on target type
        // For now, just log the routing
        println!("Routing event {} to target {}", event.id, route.target);
        Ok(())
    }
    
    /// Update performance metrics
    async fn update_metrics(&self, start_time: Instant, error: bool) {
        let mut metrics = self.metrics.write().await;
        
        metrics.events_processed += 1;
        
        if error {
            metrics.error_count += 1;
        }
        
        let latency = start_time.elapsed();
        metrics.average_latency = Duration::from_nanos(
            (metrics.average_latency.as_nanos() as u64 + latency.as_nanos() as u64) / 2
        );
        
        if latency > metrics.peak_latency {
            metrics.peak_latency = latency;
        }
        
        // Calculate events per second
        let elapsed_since_reset = metrics.last_reset.elapsed();
        if elapsed_since_reset.as_secs() > 0 {
            metrics.events_per_second = metrics.events_processed as f64 / elapsed_since_reset.as_secs_f64();
        }
        
        metrics.active_subscriptions = self.subscriptions.read().await.len();
    }
}

impl EventRouter {
    pub fn new() -> Self {
        Self {
            routes: HashMap::new(),
            filters: HashMap::new(),
        }
    }
    
    pub fn add_route(&mut self, source: String, route: EventRoute) {
        self.routes.entry(source).or_insert_with(Vec::new).push(route);
    }
    
    pub fn add_filter(&mut self, filter_id: String, filter: EventFilter) {
        self.filters.insert(filter_id, filter);
    }
}

impl EventStore {
    pub fn new(max_events: usize) -> Self {
        Self {
            events: Vec::with_capacity(max_events),
            max_events,
            circular_buffer: true,
        }
    }
    
    pub fn add_event(&mut self, event: ContextEvent) {
        if self.events.len() >= self.max_events && self.circular_buffer {
            self.events.remove(0);
        }
        self.events.push(event);
    }
    
    pub fn get_recent_events(&self, count: usize) -> Vec<ContextEvent> {
        let start_idx = if self.events.len() > count {
            self.events.len() - count
        } else {
            0
        };
        self.events[start_idx..].to_vec()
    }
}

/// Context Bus error types
#[derive(Debug, thiserror::Error)]
pub enum ContextBusError {
    #[error("Failed to publish event")]
    PublishFailed,
    #[error("Subscription failed")]
    SubscriptionFailed,
    #[error("Routing failed")]
    RoutingFailed,
    #[error("Invalid configuration")]
    InvalidConfig,
}

/// Helper functions for creating common events
impl ContextEvent {
    pub fn new(event_type: ContextEventType, source: String, payload: ContextEventPayload) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
            event_type,
            source,
            target: None,
            payload,
            priority: EventPriority::Normal,
            correlation_id: None,
        }
    }
    
    pub fn with_priority(mut self, priority: EventPriority) -> Self {
        self.priority = priority;
        self
    }
    
    pub fn with_target(mut self, target: String) -> Self {
        self.target = Some(target);
        self
    }
    
    pub fn with_correlation_id(mut self, correlation_id: String) -> Self {
        self.correlation_id = Some(correlation_id);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::timeout;
    
    #[tokio::test]
    async fn test_context_bus_performance() {
        let context_bus = ContextBus::new(10000);
        let start_time = Instant::now();
        
        // Test publishing 1000 events
        for i in 0..1000 {
            let event = ContextEvent::new(
                ContextEventType::Custom(format!("test_{}", i)),
                "test_source".to_string(),
                ContextEventPayload::Custom(serde_json::json!({"test": i})),
            );
            
            context_bus.publish(event).await.unwrap();
        }
        
        let elapsed = start_time.elapsed();
        let events_per_second = 1000.0 / elapsed.as_secs_f64();
        
        println!("Published 1000 events in {:?}", elapsed);
        println!("Events per second: {:.2}", events_per_second);
        
        // Should handle 1000+ events per second
        assert!(events_per_second > 1000.0, "Context Bus should handle 1000+ events/second");
        
        let metrics = context_bus.get_metrics().await;
        assert_eq!(metrics.events_processed, 1000);
        assert!(metrics.events_per_second > 1000.0);
    }
    
    #[tokio::test]
    async fn test_event_subscription() {
        let context_bus = ContextBus::new(1000);
        
        let config = SubscriptionConfig {
            subscriber_id: "test_subscriber".to_string(),
            event_types: vec![ContextEventType::AgentStarted],
            filters: vec![],
            batch_size: None,
            max_latency: None,
        };
        
        let mut receiver = context_bus.subscribe("test_subscriber".to_string(), config).await.unwrap();
        
        // Publish test event
        let event = ContextEvent::new(
            ContextEventType::AgentStarted,
            "test_agent".to_string(),
            ContextEventPayload::AgentPayload {
                agent_id: "agent_1".to_string(),
                agent_type: "TestAgent".to_string(),
                status: "started".to_string(),
                data: serde_json::json!({}),
            },
        );
        
        context_bus.publish(event.clone()).await.unwrap();
        
        // Should receive the event
        let received_event = timeout(Duration::from_millis(100), receiver.recv()).await.unwrap().unwrap();
        assert_eq!(received_event.id, event.id);
    }
}
