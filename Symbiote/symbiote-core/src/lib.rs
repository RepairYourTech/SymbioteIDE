//! # Symbiote Core
//! 
//! Core types, errors, and utilities for the Symbiote IDE system.
//! This module provides the foundational components that all other modules depend on.

// Core modules as defined in the plan
pub mod commands;
pub mod events;
pub mod services;
pub mod models;
pub mod database;
pub mod ai;
pub mod core;
pub mod security;
pub mod specialized;
pub mod notebook;
pub mod terminal;
pub mod file_system;
pub mod git;
pub mod testing;
pub mod performance;
pub mod collaboration;
pub mod extensions;
pub mod utils;
pub mod config;
pub mod error;

// Re-export commonly used types
pub use error::{SymbioteError, Result};
pub use config::{AppConfig, DatabaseConfig, AIProvidersConfig, SecurityConfig, PerformanceConfig, FeatureFlags};

/// User ID type for consistent user identification across the system
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct UserId(uuid::Uuid);

impl UserId {
    /// Create a new random user ID
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }

    /// Create a user ID from a UUID
    pub fn from_uuid(uuid: uuid::Uuid) -> Self {
        Self(uuid)
    }

    /// Get the inner UUID
    pub fn as_uuid(&self) -> uuid::Uuid {
        self.0
    }
}

impl Default for UserId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for UserId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for UserId {
    type Err = uuid::Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(Self(uuid::Uuid::parse_str(s)?))
    }
}

/// Session ID type for tracking user sessions
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct SessionId(uuid::Uuid);

impl SessionId {
    /// Create a new random session ID
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }

    /// Get the inner UUID
    pub fn as_uuid(&self) -> uuid::Uuid {
        self.0
    }
}

impl Default for SessionId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for SessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Project ID type for project identification
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct ProjectId(uuid::Uuid);

impl ProjectId {
    /// Create a new random project ID
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }

    /// Get the inner UUID
    pub fn as_uuid(&self) -> uuid::Uuid {
        self.0
    }
}

impl Default for ProjectId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ProjectId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Agent ID type for AI agent identification
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct AgentId(uuid::Uuid);

impl AgentId {
    /// Create a new random agent ID
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }

    /// Get the inner UUID
    pub fn as_uuid(&self) -> uuid::Uuid {
        self.0
    }
}

impl Default for AgentId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for AgentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Execution ID type for tracking operations
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct ExecutionId(uuid::Uuid);

impl ExecutionId {
    /// Create a new random execution ID
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }

    /// Get the inner UUID
    pub fn as_uuid(&self) -> uuid::Uuid {
        self.0
    }
}

impl Default for ExecutionId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ExecutionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Initialize the core system with configuration
pub async fn initialize(config: AppConfig) -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    tracing::info!("Initializing Symbiote Core system");
    tracing::debug!("Configuration: {:?}", config);

    // Validate configuration
    config.validate()?;

    tracing::info!("Symbiote Core system initialized successfully");
    Ok(())
}

/// Shutdown the core system gracefully
pub async fn shutdown() -> Result<()> {
    tracing::info!("Shutting down Symbiote Core system");
    
    // Perform any necessary cleanup
    
    tracing::info!("Symbiote Core system shutdown complete");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_id_creation() {
        let id1 = UserId::new();
        let id2 = UserId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_user_id_from_string() {
        let uuid_str = "550e8400-e29b-41d4-a716-446655440000";
        let user_id: UserId = uuid_str.parse().unwrap();
        assert_eq!(user_id.to_string(), uuid_str);
    }

    #[test]
    fn test_session_id_creation() {
        let id1 = SessionId::new();
        let id2 = SessionId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_project_id_creation() {
        let id1 = ProjectId::new();
        let id2 = ProjectId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_agent_id_creation() {
        let id1 = AgentId::new();
        let id2 = AgentId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_execution_id_creation() {
        let id1 = ExecutionId::new();
        let id2 = ExecutionId::new();
        assert_ne!(id1, id2);
    }
}
