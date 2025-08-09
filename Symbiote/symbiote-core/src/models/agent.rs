//! Agent models

use crate::{AgentId, UserId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub id: AgentId,
    pub name: String,
    pub agent_type: String,
    pub owner_id: UserId,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub status: AgentStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentStatus {
    Idle,
    Running,
    Paused,
    Error,
}
