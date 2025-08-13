//! Core types and traits used throughout the Symbiote ecosystem

use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};
use uuid::Uuid;

pub mod id;
pub mod result;
pub mod config;
pub mod metadata;

pub use id::*;
pub use result::*;
pub use config::*;
pub use metadata::*;

/// Macro for generating typed IDs
macro_rules! typed_id {
    ($name:ident, $doc:expr) => {
        #[doc = $doc]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        pub struct $name(Uuid);

        impl $name {
            /// Create a new typed ID with a random UUID v4
            pub fn new() -> Self {
                Self(Uuid::new_v4())
            }

            /// Create a typed ID from an existing UUID
            pub fn from_uuid(uuid: Uuid) -> Self {
                Self(uuid)
            }

            /// Get the underlying UUID
            pub fn as_uuid(&self) -> Uuid {
                self.0
            }

            /// Get the ID as a string
            pub fn as_str(&self) -> String {
                self.0.to_string()
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl From<Uuid> for $name {
            fn from(uuid: Uuid) -> Self {
                Self(uuid)
            }
        }

        impl From<$name> for Uuid {
            fn from(id: $name) -> Self {
                id.0
            }
        }
    };
}

// Generate all typed IDs
typed_id!(AgentId, "Unique identifier for AI agents");
typed_id!(WorkflowId, "Unique identifier for workflows");
typed_id!(ToolId, "Unique identifier for tools");
typed_id!(UserId, "Unique identifier for users");
typed_id!(SessionId, "Unique identifier for user sessions");
typed_id!(ProjectId, "Unique identifier for projects");
typed_id!(WorkspaceId, "Unique identifier for workspaces");
typed_id!(ContainerId, "Unique identifier for containers");
typed_id!(TradeId, "Unique identifier for trades");
typed_id!(ModelId, "Unique identifier for AI models");
typed_id!(ThreadId, "Unique identifier for conversation threads");
typed_id!(RunId, "Unique identifier for agent runs");
typed_id!(ExecutionId, "Unique identifier for workflow executions");
typed_id!(NodeId, "Unique identifier for workflow nodes");
typed_id!(ConnectionId, "Unique identifier for node connections");
typed_id!(RepositoryId, "Unique identifier for repositories");
typed_id!(BranchId, "Unique identifier for git branches");
typed_id!(CommitId, "Unique identifier for git commits");
typed_id!(PullRequestId, "Unique identifier for pull requests");

/// Environment type for different deployment environments
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Environment {
    /// Development environment
    Development,
    /// Testing environment
    Testing,
    /// Staging environment
    Staging,
    /// Production environment
    Production,
    /// Custom environment
    Custom(String),
}

impl Default for Environment {
    fn default() -> Self {
        Self::Development
    }
}

impl Display for Environment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Environment::Development => write!(f, "development"),
            Environment::Testing => write!(f, "testing"),
            Environment::Staging => write!(f, "staging"),
            Environment::Production => write!(f, "production"),
            Environment::Custom(name) => write!(f, "{}", name),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_typed_id_creation() {
        let agent_id = AgentId::new();
        let workflow_id = WorkflowId::new();
        
        assert_ne!(agent_id.as_uuid(), workflow_id.as_uuid());
        assert_eq!(agent_id.as_str().len(), 36); // UUID string length
    }

    #[test]
    fn test_typed_id_conversion() {
        let uuid = Uuid::new_v4();
        let agent_id = AgentId::from_uuid(uuid);
        
        assert_eq!(agent_id.as_uuid(), uuid);
        assert_eq!(Uuid::from(agent_id), uuid);
    }

    #[test]
    fn test_environment_display() {
        assert_eq!(Environment::Development.to_string(), "development");
        assert_eq!(Environment::Production.to_string(), "production");
        assert_eq!(Environment::Custom("local".to_string()).to_string(), "local");
    }
}
