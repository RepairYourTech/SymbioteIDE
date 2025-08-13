//! Comprehensive error handling system for Symbiote

use std::fmt::{self, Display};
use std::time::Duration;
use thiserror::Error;

pub mod core;
pub mod chain;
pub mod context;

pub use core::*;
pub use chain::*;
pub use context::*;

/// Main error type for the Symbiote ecosystem
#[derive(Debug, Error)]
pub enum SymbioteError {
    /// Configuration-related errors
    #[error("Configuration error: {message}")]
    Configuration { message: String },

    /// Input validation errors
    #[error("Validation error in field '{field}': {message}")]
    Validation { field: String, message: String },

    /// Service-related errors
    #[error("Service error in '{service}': {message}")]
    Service { service: String, message: String },

    /// I/O errors
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization/deserialization errors
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Database errors
    #[error("Database error: {0}")]
    Database(String),

    /// Network errors
    #[error("Network error: {0}")]
    Network(String),

    /// Timeout errors
    #[error("Operation timed out after {duration:?}")]
    Timeout { duration: Duration },

    /// Cancellation errors
    #[error("Operation cancelled: {reason}")]
    Cancelled { reason: String },

    /// Permission/authorization errors
    #[error("Permission denied: {action} on {resource}")]
    PermissionDenied { action: String, resource: String },

    /// Resource not found errors
    #[error("Resource not found: {resource_type} with id '{id}'")]
    NotFound { resource_type: String, id: String },

    /// Resource already exists errors
    #[error("Resource already exists: {resource_type} with id '{id}'")]
    AlreadyExists { resource_type: String, id: String },

    /// Rate limiting errors
    #[error("Rate limit exceeded: {limit} requests per {window:?}")]
    RateLimitExceeded { limit: u32, window: Duration },

    /// Internal system errors
    #[error("Internal error: {message}")]
    Internal { message: String },

    /// Custom errors for extensibility
    #[error("Custom error: {error_type}: {message}")]
    Custom { error_type: String, message: String },
}

impl SymbioteError {
    /// Add context to an error
    pub fn context<T: Display>(self, context: T) -> Self {
        match self {
            SymbioteError::Internal { message } => SymbioteError::Internal {
                message: format!("{}: {}", context, message),
            },
            other => other,
        }
    }

    /// Check if the error is retryable
    pub fn is_retryable(&self) -> bool {
        match self {
            SymbioteError::Network(_) => true,
            SymbioteError::Timeout { .. } => true,
            SymbioteError::Database(_) => true,
            SymbioteError::RateLimitExceeded { .. } => true,
            SymbioteError::Internal { .. } => false,
            SymbioteError::Configuration { .. } => false,
            SymbioteError::Validation { .. } => false,
            SymbioteError::PermissionDenied { .. } => false,
            SymbioteError::NotFound { .. } => false,
            SymbioteError::AlreadyExists { .. } => false,
            SymbioteError::Io(_) => true,
            SymbioteError::Serialization(_) => false,
            SymbioteError::Service { .. } => true,
            SymbioteError::Cancelled { .. } => false,
            SymbioteError::Custom { .. } => false,
        }
    }

    /// Get error code for API responses
    pub fn error_code(&self) -> &'static str {
        match self {
            SymbioteError::Configuration { .. } => "CONFIGURATION_ERROR",
            SymbioteError::Validation { .. } => "VALIDATION_ERROR",
            SymbioteError::Service { .. } => "SERVICE_ERROR",
            SymbioteError::Io(_) => "IO_ERROR",
            SymbioteError::Serialization(_) => "SERIALIZATION_ERROR",
            SymbioteError::Database(_) => "DATABASE_ERROR",
            SymbioteError::Network(_) => "NETWORK_ERROR",
            SymbioteError::Timeout { .. } => "TIMEOUT_ERROR",
            SymbioteError::Cancelled { .. } => "CANCELLED_ERROR",
            SymbioteError::PermissionDenied { .. } => "PERMISSION_DENIED",
            SymbioteError::NotFound { .. } => "NOT_FOUND",
            SymbioteError::AlreadyExists { .. } => "ALREADY_EXISTS",
            SymbioteError::RateLimitExceeded { .. } => "RATE_LIMIT_EXCEEDED",
            SymbioteError::Internal { .. } => "INTERNAL_ERROR",
            SymbioteError::Custom { .. } => "CUSTOM_ERROR",
        }
    }

    /// Get HTTP status code for web APIs
    pub fn http_status_code(&self) -> u16 {
        match self {
            SymbioteError::Configuration { .. } => 500,
            SymbioteError::Validation { .. } => 400,
            SymbioteError::Service { .. } => 503,
            SymbioteError::Io(_) => 500,
            SymbioteError::Serialization(_) => 400,
            SymbioteError::Database(_) => 500,
            SymbioteError::Network(_) => 502,
            SymbioteError::Timeout { .. } => 504,
            SymbioteError::Cancelled { .. } => 499,
            SymbioteError::PermissionDenied { .. } => 403,
            SymbioteError::NotFound { .. } => 404,
            SymbioteError::AlreadyExists { .. } => 409,
            SymbioteError::RateLimitExceeded { .. } => 429,
            SymbioteError::Internal { .. } => 500,
            SymbioteError::Custom { .. } => 500,
        }
    }
}

/// Convenience functions for creating common errors
impl SymbioteError {
    /// Create a configuration error
    pub fn config<T: Display>(message: T) -> Self {
        Self::Configuration {
            message: message.to_string(),
        }
    }

    /// Create a validation error
    pub fn validation<T: Display, U: Display>(field: T, message: U) -> Self {
        Self::Validation {
            field: field.to_string(),
            message: message.to_string(),
        }
    }

    /// Create a service error
    pub fn service<T: Display, U: Display>(service: T, message: U) -> Self {
        Self::Service {
            service: service.to_string(),
            message: message.to_string(),
        }
    }

    /// Create a not found error
    pub fn not_found<T: Display, U: Display>(resource_type: T, id: U) -> Self {
        Self::NotFound {
            resource_type: resource_type.to_string(),
            id: id.to_string(),
        }
    }

    /// Create an internal error
    pub fn internal<T: Display>(message: T) -> Self {
        Self::Internal {
            message: message.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let error = SymbioteError::config("Invalid configuration");
        assert_eq!(error.error_code(), "CONFIGURATION_ERROR");
        assert_eq!(error.http_status_code(), 500);
        assert!(!error.is_retryable());
    }

    #[test]
    fn test_error_context() {
        let error = SymbioteError::internal("Something went wrong");
        let with_context = error.context("During startup");
        
        if let SymbioteError::Internal { message } = with_context {
            assert!(message.contains("During startup"));
            assert!(message.contains("Something went wrong"));
        } else {
            panic!("Expected Internal error");
        }
    }

    #[test]
    fn test_retryable_errors() {
        assert!(SymbioteError::Network("Connection failed".to_string()).is_retryable());
        assert!(SymbioteError::Timeout { duration: Duration::from_secs(30) }.is_retryable());
        assert!(!SymbioteError::Validation { field: "name".to_string(), message: "Required".to_string() }.is_retryable());
    }
}
