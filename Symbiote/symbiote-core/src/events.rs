//! Event system for Symbiote IDE
//! 
//! Provides event-driven architecture for system communication.

use crate::{Result, UserId, ProjectId, AgentId, ExecutionId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::broadcast;

/// Event types in the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Event {
    /// User events
    UserLoggedIn { user_id: UserId, timestamp: chrono::DateTime<chrono::Utc> },
    UserLoggedOut { user_id: UserId, timestamp: chrono::DateTime<chrono::Utc> },
    
    /// Project events
    ProjectCreated { project_id: ProjectId, user_id: UserId, name: String },
    ProjectOpened { project_id: ProjectId, user_id: UserId },
    ProjectClosed { project_id: ProjectId, user_id: UserId },
    
    /// File events
    FileOpened { path: String, user_id: UserId, project_id: Option<ProjectId> },
    FileModified { path: String, user_id: UserId, project_id: Option<ProjectId> },
    FileSaved { path: String, user_id: UserId, project_id: Option<ProjectId> },
    
    /// AI events
    AIRequestStarted { execution_id: ExecutionId, agent_id: AgentId, user_id: UserId },
    AIRequestCompleted { execution_id: ExecutionId, agent_id: AgentId, success: bool },
    
    /// System events
    SystemStarted { timestamp: chrono::DateTime<chrono::Utc> },
    SystemShutdown { timestamp: chrono::DateTime<chrono::Utc> },
    
    /// Custom events
    Custom { event_type: String, data: serde_json::Value },
}

impl Event {
    /// Get event timestamp
    pub fn timestamp(&self) -> chrono::DateTime<chrono::Utc> {
        match self {
            Event::UserLoggedIn { timestamp, .. } => *timestamp,
            Event::UserLoggedOut { timestamp, .. } => *timestamp,
            Event::SystemStarted { timestamp } => *timestamp,
            Event::SystemShutdown { timestamp } => *timestamp,
            _ => chrono::Utc::now(),
        }
    }

    /// Get event type as string
    pub fn event_type(&self) -> &str {
        match self {
            Event::UserLoggedIn { .. } => "user_logged_in",
            Event::UserLoggedOut { .. } => "user_logged_out",
            Event::ProjectCreated { .. } => "project_created",
            Event::ProjectOpened { .. } => "project_opened",
            Event::ProjectClosed { .. } => "project_closed",
            Event::FileOpened { .. } => "file_opened",
            Event::FileModified { .. } => "file_modified",
            Event::FileSaved { .. } => "file_saved",
            Event::AIRequestStarted { .. } => "ai_request_started",
            Event::AIRequestCompleted { .. } => "ai_request_completed",
            Event::SystemStarted { .. } => "system_started",
            Event::SystemShutdown { .. } => "system_shutdown",
            Event::Custom { event_type, .. } => event_type,
        }
    }
}

/// Event handler trait
#[async_trait::async_trait]
pub trait EventHandler: Send + Sync {
    /// Handle an event
    async fn handle(&self, event: &Event) -> Result<()>;
    
    /// Get handler name
    fn name(&self) -> &str;
}

/// Event bus for managing events and handlers
pub struct EventBus {
    sender: broadcast::Sender<Event>,
    handlers: HashMap<String, Box<dyn EventHandler>>,
}

impl EventBus {
    /// Create a new event bus
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self {
            sender,
            handlers: HashMap::new(),
        }
    }

    /// Register an event handler
    pub fn register_handler(&mut self, handler: Box<dyn EventHandler>) {
        self.handlers.insert(handler.name().to_string(), handler);
    }

    /// Publish an event
    pub async fn publish(&self, event: Event) -> Result<()> {
        // Send to broadcast channel
        if let Err(e) = self.sender.send(event.clone()) {
            tracing::warn!("Failed to broadcast event: {}", e);
        }

        // Send to registered handlers
        for handler in self.handlers.values() {
            if let Err(e) = handler.handle(&event).await {
                tracing::error!("Handler {} failed to process event: {}", handler.name(), e);
            }
        }

        Ok(())
    }

    /// Subscribe to events
    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.sender.subscribe()
    }

    /// Get number of active subscribers
    pub fn subscriber_count(&self) -> usize {
        self.sender.receiver_count()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new(1000) // Default capacity of 1000 events
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestHandler {
        name: String,
    }

    #[async_trait::async_trait]
    impl EventHandler for TestHandler {
        async fn handle(&self, _event: &Event) -> Result<()> {
            Ok(())
        }

        fn name(&self) -> &str {
            &self.name
        }
    }

    #[tokio::test]
    async fn test_event_bus() {
        let mut bus = EventBus::new(10);
        let handler = TestHandler {
            name: "test_handler".to_string(),
        };
        
        bus.register_handler(Box::new(handler));
        
        let event = Event::SystemStarted {
            timestamp: chrono::Utc::now(),
        };
        
        assert!(bus.publish(event).await.is_ok());
    }

    #[test]
    fn test_event_type() {
        let event = Event::UserLoggedIn {
            user_id: UserId::new(),
            timestamp: chrono::Utc::now(),
        };
        
        assert_eq!(event.event_type(), "user_logged_in");
    }
}
