use crate::task_manager::core::*;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use std::collections::HashMap;

// Type alias for compatibility
pub type TaskUpdateEvent = TaskEvent;

/// Real-time task update events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskEvent {
    TaskCreated(Task),
    TaskUpdated(Task),
    TaskDeleted(String),
    TaskStatusChanged { task_id: String, old_status: TaskStatus, new_status: TaskStatus },
    TaskAssigned { task_id: String, assigned_to: String },
    TaskCommentAdded(TaskComment),
    TaskProgressUpdated { task_id: String, progress: u8 },
    ProjectCreated(Project),
    ProjectUpdated(Project),
    EpicCreated(Epic),
    EpicUpdated(Epic),
    DependencyAdded(TaskDependency),
    DependencyRemoved(String),
    TimeEntryAdded(TimeEntry),
    TimeEntryUpdated(TimeEntry),
    BulkTaskUpdate(Vec<Task>),
    AgentTaskCreated { agent_id: String, task: Task },
    AgentTaskCompleted { agent_id: String, task_id: String, result: String },
}

/// Event manager for real-time task updates
pub struct TaskEventManager {
    sender: broadcast::Sender<TaskEvent>,
    subscribers: HashMap<String, broadcast::Receiver<TaskEvent>>,
}

impl TaskEventManager {
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(1000);
        
        Self {
            sender,
            subscribers: HashMap::new(),
        }
    }
    
    /// Send an event to all subscribers
    pub fn send_event(&self, event: TaskEvent) -> Result<(), TaskManagerError> {
        self.sender.send(event)
            .map_err(|e| TaskManagerError::DatabaseError(format!("Failed to send event: {}", e)))?;
        Ok(())
    }
    
    /// Subscribe to task events
    pub fn subscribe(&self) -> broadcast::Receiver<TaskEvent> {
        self.sender.subscribe()
    }
    
    /// Send multiple events in batch
    pub fn send_batch_events(&self, events: Vec<TaskEvent>) -> Result<(), TaskManagerError> {
        for event in events {
            self.send_event(event)?;
        }
        Ok(())
    }
    
    /// Send task creation event with metadata
    pub fn send_task_created(&self, task: Task, created_by_agent: Option<String>) -> Result<(), TaskManagerError> {
        if let Some(agent_id) = created_by_agent {
            self.send_event(TaskEvent::AgentTaskCreated { agent_id, task: task.clone() })?;
        }
        self.send_event(TaskEvent::TaskCreated(task))?;
        Ok(())
    }
    
    /// Send task completion event with result
    pub fn send_task_completed(&self, task: Task, completed_by_agent: Option<String>, result: Option<String>) -> Result<(), TaskManagerError> {
        if let (Some(agent_id), Some(result)) = (completed_by_agent, result) {
            self.send_event(TaskEvent::AgentTaskCompleted { 
                agent_id, 
                task_id: task.id.clone(), 
                result 
            })?;
        }
        self.send_event(TaskEvent::TaskUpdated(task))?;
        Ok(())
    }
    
    /// Send status change event
    pub fn send_status_change(&self, task_id: String, old_status: TaskStatus, new_status: TaskStatus) -> Result<(), TaskManagerError> {
        self.send_event(TaskEvent::TaskStatusChanged { task_id, old_status, new_status })?;
        Ok(())
    }
    
    /// Send assignment event
    pub fn send_assignment(&self, task_id: String, assigned_to: String) -> Result<(), TaskManagerError> {
        self.send_event(TaskEvent::TaskAssigned { task_id, assigned_to })?;
        Ok(())
    }
    
    /// Send progress update event
    pub fn send_progress_update(&self, task_id: String, progress: u8) -> Result<(), TaskManagerError> {
        self.send_event(TaskEvent::TaskProgressUpdated { task_id, progress })?;
        Ok(())
    }
}
