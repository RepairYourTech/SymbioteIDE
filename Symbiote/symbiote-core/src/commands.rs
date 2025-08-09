//! Command system for Symbiote IDE
//! 
//! Handles command execution and management across the system.

use crate::{Result, SymbioteError};
use serde::{Deserialize, Serialize};

/// Command trait for all executable commands
#[async_trait::async_trait]
pub trait Command: Send + Sync {
    /// Execute the command
    async fn execute(&self) -> Result<CommandResult>;

    /// Get command name
    fn name(&self) -> &str;

    /// Get command description
    fn description(&self) -> &str;
}

/// Result of command execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandResult {
    pub success: bool,
    pub message: String,
    pub data: Option<serde_json::Value>,
}

impl CommandResult {
    pub fn success(message: impl Into<String>) -> Self {
        Self {
            success: true,
            message: message.into(),
            data: None,
        }
    }

    pub fn success_with_data(message: impl Into<String>, data: serde_json::Value) -> Self {
        Self {
            success: true,
            message: message.into(),
            data: Some(data),
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            success: false,
            message: message.into(),
            data: None,
        }
    }
}

/// Command registry for managing available commands
#[derive(Default)]
pub struct CommandRegistry {
    commands: std::collections::HashMap<String, Box<dyn Command>>,
}

impl CommandRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, command: Box<dyn Command>) {
        self.commands.insert(command.name().to_string(), command);
    }

    pub async fn execute(&self, name: &str) -> Result<CommandResult> {
        let command = self.commands.get(name)
            .ok_or_else(|| SymbioteError::not_found(format!("Command not found: {}", name)))?;
        
        command.execute().await
    }

    pub fn list_commands(&self) -> Vec<(&str, &str)> {
        self.commands.values()
            .map(|cmd| (cmd.name(), cmd.description()))
            .collect()
    }
}
