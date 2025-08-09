//! Configuration management for Symbiote IDE
//! 
//! Comprehensive configuration system as defined in the plan.

use crate::{Result, SymbioteError};
use serde::{Deserialize, Serialize};

/// Main application configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AppConfig {
    pub database: DatabaseConfig,
    pub ai_providers: AIProvidersConfig,
    pub security: SecurityConfig,
    pub performance: PerformanceConfig,
    pub features: FeatureFlags,
}

impl AppConfig {
    /// Load configuration from file
    pub fn load_from_file<P: AsRef<std::path::Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| SymbioteError::config(format!("Failed to read config file: {}", e)))?;
        
        let config: AppConfig = toml::from_str(&content)
            .map_err(|e| SymbioteError::config(format!("Failed to parse config: {}", e)))?;
        
        config.validate()?;
        Ok(config)
    }

    /// Load configuration from environment and defaults
    pub fn load() -> Result<Self> {
        let settings = config::Config::builder()
            .add_source(config::Environment::with_prefix("SYMBIOTE"))
            .add_source(config::File::with_name("symbiote.toml").required(false))
            .add_source(config::File::with_name("config/symbiote.toml").required(false))
            .build()
            .map_err(|e| SymbioteError::config(format!("Failed to build config: {}", e)))?;

        let config: AppConfig = settings.try_deserialize()
            .map_err(|e| SymbioteError::config(format!("Failed to deserialize config: {}", e)))?;

        config.validate()?;
        Ok(config)
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        self.database.validate()?;
        self.ai_providers.validate()?;
        self.security.validate()?;
        self.performance.validate()?;
        Ok(())
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            database: DatabaseConfig::default(),
            ai_providers: AIProvidersConfig::default(),
            security: SecurityConfig::default(),
            performance: PerformanceConfig::default(),
            features: FeatureFlags::default(),
        }
    }
}

/// Database configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DatabaseConfig {
    pub sqlite_path: String,
    pub neo4j_uri: String,
    pub neo4j_user: String,
    pub neo4j_password: String,
    pub qdrant_uri: String,
    pub qdrant_api_key: Option<String>,
}

impl DatabaseConfig {
    pub fn validate(&self) -> Result<()> {
        if self.sqlite_path.is_empty() {
            return Err(SymbioteError::config("SQLite path cannot be empty"));
        }
        if self.neo4j_uri.is_empty() {
            return Err(SymbioteError::config("Neo4j URI cannot be empty"));
        }
        if self.qdrant_uri.is_empty() {
            return Err(SymbioteError::config("Qdrant URI cannot be empty"));
        }
        Ok(())
    }
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            sqlite_path: "symbiote.db".to_string(),
            neo4j_uri: "bolt://localhost:7687".to_string(),
            neo4j_user: "neo4j".to_string(),
            neo4j_password: "password".to_string(),
            qdrant_uri: "http://localhost:6334".to_string(),
            qdrant_api_key: None,
        }
    }
}

/// AI providers configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AIProvidersConfig {
    pub openrouter_api_key: Option<String>,
    pub openai_api_key: Option<String>,
    pub anthropic_api_key: Option<String>,
    pub google_api_key: Option<String>,
    pub default_provider: String,
    pub default_model: String,
    pub max_tokens: u32,
    pub temperature: f32,
}

impl AIProvidersConfig {
    pub fn validate(&self) -> Result<()> {
        if self.default_provider.is_empty() {
            return Err(SymbioteError::config("Default provider cannot be empty"));
        }
        if self.default_model.is_empty() {
            return Err(SymbioteError::config("Default model cannot be empty"));
        }
        if self.max_tokens == 0 {
            return Err(SymbioteError::config("Max tokens must be greater than 0"));
        }
        if !(0.0..=2.0).contains(&self.temperature) {
            return Err(SymbioteError::config("Temperature must be between 0.0 and 2.0"));
        }
        Ok(())
    }
}

impl Default for AIProvidersConfig {
    fn default() -> Self {
        Self {
            openrouter_api_key: None,
            openai_api_key: None,
            anthropic_api_key: None,
            google_api_key: None,
            default_provider: "openrouter".to_string(),
            default_model: "anthropic/claude-3.5-sonnet".to_string(),
            max_tokens: 4096,
            temperature: 0.7,
        }
    }
}

/// Security configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SecurityConfig {
    pub jwt_secret: String,
    pub session_timeout_hours: u32,
    pub max_login_attempts: u32,
    pub enable_2fa: bool,
    pub encryption_key: Option<String>,
}

impl SecurityConfig {
    pub fn validate(&self) -> Result<()> {
        if self.jwt_secret.len() < 32 {
            return Err(SymbioteError::config("JWT secret must be at least 32 characters"));
        }
        if self.session_timeout_hours == 0 {
            return Err(SymbioteError::config("Session timeout must be greater than 0"));
        }
        if self.max_login_attempts == 0 {
            return Err(SymbioteError::config("Max login attempts must be greater than 0"));
        }
        Ok(())
    }
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            jwt_secret: "your-super-secret-jwt-key-change-this-in-production".to_string(),
            session_timeout_hours: 24,
            max_login_attempts: 5,
            enable_2fa: false,
            encryption_key: None,
        }
    }
}

/// Performance configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PerformanceConfig {
    pub max_concurrent_requests: u32,
    pub request_timeout_seconds: u32,
    pub cache_size_mb: u32,
    pub worker_threads: Option<u32>,
    pub enable_metrics: bool,
}

impl PerformanceConfig {
    pub fn validate(&self) -> Result<()> {
        if self.max_concurrent_requests == 0 {
            return Err(SymbioteError::config("Max concurrent requests must be greater than 0"));
        }
        if self.request_timeout_seconds == 0 {
            return Err(SymbioteError::config("Request timeout must be greater than 0"));
        }
        Ok(())
    }
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            max_concurrent_requests: 1000,
            request_timeout_seconds: 30,
            cache_size_mb: 512,
            worker_threads: None, // Use default
            enable_metrics: true,
        }
    }
}

/// Feature flags for enabling/disabling functionality
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FeatureFlags {
    pub enable_ai_features: bool,
    pub enable_collaboration: bool,
    pub enable_git_integration: bool,
    pub enable_terminal: bool,
    pub enable_notebook: bool,
    pub enable_testing: bool,
    pub enable_performance_monitoring: bool,
    pub enable_extensions: bool,
}

impl Default for FeatureFlags {
    fn default() -> Self {
        Self {
            enable_ai_features: true,
            enable_collaboration: true,
            enable_git_integration: true,
            enable_terminal: true,
            enable_notebook: true,
            enable_testing: true,
            enable_performance_monitoring: true,
            enable_extensions: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_validation() {
        let config = AppConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_invalid_ai_config() {
        let mut config = AIProvidersConfig::default();
        config.temperature = 3.0; // Invalid temperature
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_invalid_security_config() {
        let mut config = SecurityConfig::default();
        config.jwt_secret = "short".to_string(); // Too short
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_serialization() {
        let config = AppConfig::default();
        let serialized = toml::to_string(&config).unwrap();
        let deserialized: AppConfig = toml::from_str(&serialized).unwrap();
        assert!(deserialized.validate().is_ok());
    }
}
