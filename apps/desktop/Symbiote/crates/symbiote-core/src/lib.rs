//! # Symbiote Core
//!
//! Foundation crate for the Symbiote ecosystem providing:
//! - Unified type system with typed IDs
//! - Comprehensive error handling
//! - Dependency injection container
//! - Configuration management
//! - Async utilities and patterns
//! - Global event system (hooks)
//! - Structured logging and observability

#![deny(missing_docs)]
#![warn(clippy::all)]

// Public API exports
pub use types::*;
pub use error::*;
pub use di::*;
pub use async_utils::*;
pub use logging::*;
pub use validation::*;
pub use events::*;
pub use metrics::*;
pub use security::*;
pub use plugins::*;

// Core modules
pub mod types;
pub mod error;
pub mod di;
pub mod async_utils;
pub mod logging;
pub mod validation;
pub mod events;
pub mod metrics;
pub mod security;
pub mod plugins;

// Re-exports for convenience
pub use chrono::{DateTime, Utc};
pub use serde::{Deserialize, Serialize};
pub use std::time::Duration;
pub use tokio;
pub use tracing;
pub use uuid::Uuid;

/// Core result type used throughout Symbiote
pub type SymbioteResult<T> = Result<T, SymbioteError>;

/// Async result type for futures
pub type AsyncResult<T> = std::pin::Pin<Box<dyn std::future::Future<Output = SymbioteResult<T>> + Send>>;

/// Initialize the Symbiote core system
pub async fn initialize() -> SymbioteResult<()> {
    // Initialize logging
    logging::init_logging()?;
    
    // Initialize metrics
    metrics::init_metrics()?;
    
    // Initialize global event system
    events::init_global_hooks().await?;
    
    tracing::info!("Symbiote core initialized successfully");
    Ok(())
}

/// Shutdown the Symbiote core system
pub async fn shutdown() -> SymbioteResult<()> {
    tracing::info!("Shutting down Symbiote core");
    
    // Shutdown in reverse order
    events::shutdown_global_hooks().await?;
    metrics::shutdown_metrics()?;
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_core_initialization() {
        let result = initialize().await;
        assert!(result.is_ok());
        
        let shutdown_result = shutdown().await;
        assert!(shutdown_result.is_ok());
    }
}
