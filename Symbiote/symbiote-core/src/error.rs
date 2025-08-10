//! Error handling system for Symbiote IDE
//! 
//! Comprehensive error types covering all system components as defined in the plan.

use thiserror::Error;

/// Main error type for the Symbiote system
#[derive(Error, Debug, Clone)]
pub enum SymbioteError {
    #[error("File system error: {0}")]
    FileSystem(String),

    #[error("Database error: {0}")]
    Database(String),

    #[error("AI provider error: {0}")]
    AIProvider(String),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Security error: {0}")]
    Security(String),

    #[error("Context error: {0}")]
    Context(String),

    #[error("Context corruption: {0}")]
    ContextCorruption(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Authentication error: {0}")]
    Authentication(String),

    #[error("Authorization error: {0}")]
    Authorization(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("External service error: {service}: {message}")]
    ExternalService { service: String, message: String },

    #[error("Timeout error: {0}")]
    Timeout(String),

    #[error("Rate limit exceeded: {0}")]
    RateLimit(String),

    #[error("Resource not found: {0}")]
    NotFound(String),

    #[error("Resource already exists: {0}")]
    AlreadyExists(String),

    #[error("Operation not supported: {0}")]
    NotSupported(String),

    #[error("Concurrent modification error: {0}")]
    ConcurrentModification(String),

    #[error("Quota exceeded: {0}")]
    QuotaExceeded(String),

    #[error("Service unavailable: {0}")]
    ServiceUnavailable(String),

    #[error("External error: {0}")]
    External(String),

    #[error("Not implemented: {0}")]
    NotImplemented(String),

    #[error("Execution error: {0}")]
    Execution(String),
}

impl SymbioteError {
    /// Create an AI provider error
    pub fn ai_provider<S: Into<String>>(message: S) -> Self {
        Self::AIProvider(message.into())
    }

    /// Create a parse error
    pub fn parse<S: Into<String>>(message: S) -> Self {
        Self::Parse(message.into())
    }

    /// Create a configuration error
    pub fn config<S: Into<String>>(message: S) -> Self {
        Self::Config(message.into())
    }

    /// Create a security error
    pub fn security<S: Into<String>>(message: S) -> Self {
        Self::Security(message.into())
    }

    pub fn file_system<S: Into<String>>(message: S) -> Self {
        Self::FileSystem(message.into())
    }

    pub fn context<S: Into<String>>(message: S) -> Self {
        Self::Context(message.into())
    }

    pub fn database<S: Into<String>>(message: S) -> Self {
        Self::Database(message.into())
    }

    pub fn serialization<S: Into<String>>(message: S) -> Self {
        Self::Serialization(message.into())
    }

    /// Create an authentication error
    pub fn authentication<S: Into<String>>(message: S) -> Self {
        Self::Authentication(message.into())
    }

    /// Create an authorization error
    pub fn authorization<S: Into<String>>(message: S) -> Self {
        Self::Authorization(message.into())
    }

    /// Create a validation error
    pub fn validation<S: Into<String>>(message: S) -> Self {
        Self::Validation(message.into())
    }

    /// Create an internal error
    pub fn internal<S: Into<String>>(message: S) -> Self {
        Self::Internal(message.into())
    }

    /// Create an external service error
    pub fn external_service<S: Into<String>>(service: S, message: S) -> Self {
        Self::ExternalService {
            service: service.into(),
            message: message.into(),
        }
    }

    /// Create a timeout error
    pub fn timeout<S: Into<String>>(message: S) -> Self {
        Self::Timeout(message.into())
    }

    /// Create a rate limit error
    pub fn rate_limit<S: Into<String>>(message: S) -> Self {
        Self::RateLimit(message.into())
    }

    /// Create a not found error
    pub fn not_found<S: Into<String>>(message: S) -> Self {
        Self::NotFound(message.into())
    }

    /// Create an already exists error
    pub fn already_exists<S: Into<String>>(message: S) -> Self {
        Self::AlreadyExists(message.into())
    }

    /// Create a not supported error
    pub fn not_supported<S: Into<String>>(message: S) -> Self {
        Self::NotSupported(message.into())
    }

    /// Create a concurrent modification error
    pub fn concurrent_modification<S: Into<String>>(message: S) -> Self {
        Self::ConcurrentModification(message.into())
    }

    /// Create a quota exceeded error
    pub fn quota_exceeded<S: Into<String>>(message: S) -> Self {
        Self::QuotaExceeded(message.into())
    }

    /// Create a service unavailable error
    pub fn service_unavailable<S: Into<String>>(message: S) -> Self {
        Self::ServiceUnavailable(message.into())
    }

    /// Create an external error
    pub fn external<S: Into<String>>(message: S) -> Self {
        Self::External(message.into())
    }

    /// Create a not implemented error
    pub fn not_implemented<S: Into<String>>(message: S) -> Self {
        Self::NotImplemented(message.into())
    }

    /// Check if this error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::Network(_)
                | Self::Timeout(_)
                | Self::ServiceUnavailable(_)
                | Self::ExternalService { .. }
        )
    }

    /// Check if this error is a client error (4xx equivalent)
    pub fn is_client_error(&self) -> bool {
        matches!(
            self,
            Self::Authentication(_)
                | Self::Authorization(_)
                | Self::Validation(_)
                | Self::NotFound(_)
                | Self::AlreadyExists(_)
                | Self::NotSupported(_)
        )
    }

    /// Check if this error is a server error (5xx equivalent)
    pub fn is_server_error(&self) -> bool {
        matches!(
            self,
            Self::Internal(_)
                | Self::Database(_)
                | Self::ServiceUnavailable(_)
                | Self::ConcurrentModification(_)
        )
    }

    /// Get error category for logging and metrics
    pub fn category(&self) -> &'static str {
        match self {
            Self::FileSystem(_) => "filesystem",
            Self::Database(_) => "database",
            Self::AIProvider(_) => "ai_provider",
            Self::Parse(_) => "parse",
            Self::Config(_) => "config",
            Self::Security(_) => "security",
            Self::Context(_) => "context",
            Self::ContextCorruption(_) => "context_corruption",
            Self::Network(_) => "network",
            Self::Serialization(_) => "serialization",
            Self::Authentication(_) => "authentication",
            Self::Authorization(_) => "authorization",
            Self::Validation(_) => "validation",
            Self::Internal(_) => "internal",
            Self::ExternalService { .. } => "external_service",
            Self::Timeout(_) => "timeout",
            Self::RateLimit(_) => "rate_limit",
            Self::NotFound(_) => "not_found",
            Self::AlreadyExists(_) => "already_exists",
            Self::NotSupported(_) => "not_supported",
            Self::ConcurrentModification(_) => "concurrent_modification",
            Self::QuotaExceeded(_) => "quota_exceeded",
            Self::ServiceUnavailable(_) => "service_unavailable",
            Self::External(_) => "external",
            Self::NotImplemented(_) => "not_implemented",
            Self::Execution(_) => "execution",
        }
    }
}

/// Result type alias for convenience
pub type Result<T> = std::result::Result<T, SymbioteError>;

// Additional From implementations for error types not covered by thiserror
impl From<anyhow::Error> for SymbioteError {
    fn from(err: anyhow::Error) -> Self {
        Self::Internal(err.to_string())
    }
}

impl From<std::io::Error> for SymbioteError {
    fn from(err: std::io::Error) -> Self {
        SymbioteError::FileSystem(err.to_string())
    }
}

impl From<sqlx::Error> for SymbioteError {
    fn from(err: sqlx::Error) -> Self {
        SymbioteError::Database(err.to_string())
    }
}

impl From<reqwest::Error> for SymbioteError {
    fn from(err: reqwest::Error) -> Self {
        SymbioteError::Network(err.to_string())
    }
}

impl From<serde_json::Error> for SymbioteError {
    fn from(err: serde_json::Error) -> Self {
        SymbioteError::Serialization(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let err = SymbioteError::ai_provider("Test error");
        assert_eq!(err.to_string(), "AI provider error: Test error");
        assert_eq!(err.category(), "ai_provider");
    }

    #[test]
    fn test_error_retryable() {
        let timeout_err = SymbioteError::timeout("Request timeout");
        assert!(timeout_err.is_retryable());

        let validation_err = SymbioteError::validation("Invalid input");
        assert!(!validation_err.is_retryable());
    }

    #[test]
    fn test_error_categories() {
        let auth_err = SymbioteError::authentication("Invalid token");
        assert!(auth_err.is_client_error());
        assert!(!auth_err.is_server_error());

        let internal_err = SymbioteError::internal("Something went wrong");
        assert!(!internal_err.is_client_error());
        assert!(internal_err.is_server_error());
    }
}
