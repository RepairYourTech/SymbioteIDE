// PA (Planner Agent) System - Core module
// Revolutionary AI agent that manages tasks, plans, and coordinates with other agents

pub mod pa_agent;
pub mod task_planner;
pub mod real_time_updates;
pub mod websocket_handler;
pub mod pa_integration;
pub mod tauri_commands;

pub use pa_agent::*;
pub use task_planner::*;
pub use real_time_updates::*;
pub use websocket_handler::*;
pub use pa_integration::*;
pub use tauri_commands::*;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Task assignment for PA system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskAssignment {
    pub task_id: String,
    pub agent_id: String,
    pub assigned_at: DateTime<Utc>,
    pub priority: u8,
    pub estimated_duration: Option<std::time::Duration>,
}

/// PA System configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PASystemConfig {
    pub agent_id: String,
    pub model_provider: String,
    pub model_name: String,
    pub max_context_tokens: usize,
    pub planning_temperature: f32,
    pub real_time_updates: bool,
    pub websocket_port: u16,
}

impl Default for PASystemConfig {
    fn default() -> Self {
        Self {
            agent_id: format!("pa-agent-{}", Uuid::new_v4()),
            model_provider: "openai".to_string(),
            model_name: "gpt-4o".to_string(),
            max_context_tokens: 128000,
            planning_temperature: 0.7,
            real_time_updates: true,
            websocket_port: 8081,
        }
    }
}

/// PA System events for real-time updates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PASystemEvent {
    TaskCreated { task_id: String, task: serde_json::Value },
    TaskUpdated { task_id: String, changes: serde_json::Value },
    TaskCompleted { task_id: String, result: serde_json::Value },
    PlanGenerated { plan_id: String, plan: serde_json::Value },
    AgentAssigned { task_id: String, agent_id: String },
    AgentStatusChanged { agent_id: String, status: String },
}

/// PA System error types
#[derive(Debug, thiserror::Error)]
pub enum PASystemError {
    #[error("Agent error: {0}")]
    AgentError(String),
    
    #[error("Planning error: {0}")]
    PlanningError(String),
    
    #[error("WebSocket error: {0}")]
    WebSocketError(String),
    
    #[error("Database error: {0}")]
    DatabaseError(String),
    
    #[error("Integration error: {0}")]
    IntegrationError(String),
}
