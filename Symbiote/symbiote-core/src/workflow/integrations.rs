//! Integration Layer - External service connections

use crate::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Integration layer for external services
#[derive(Debug)]
pub struct IntegrationLayer {
    /// Available integrations
    integrations: HashMap<String, Integration>,
}

/// Integration definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Integration {
    pub integration_id: String,
    pub name: String,
    pub service_type: ServiceType,
    pub configuration: IntegrationConfig,
    pub status: IntegrationStatus,
}

/// Service types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServiceType {
    API,
    Database,
    MessageQueue,
    Storage,
    Authentication,
    Monitoring,
}

/// Integration configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationConfig {
    pub endpoint: String,
    pub authentication: AuthConfig,
    pub parameters: HashMap<String, serde_json::Value>,
}

/// Authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    pub auth_type: AuthType,
    pub credentials: HashMap<String, String>,
}

/// Authentication types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthType {
    None,
    ApiKey,
    OAuth2,
    Basic,
    Bearer,
}

/// Integration status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IntegrationStatus {
    Active,
    Inactive,
    Error(String),
}

impl IntegrationLayer {
    pub fn new() -> Self {
        Self {
            integrations: HashMap::new(),
        }
    }
}

impl Clone for IntegrationLayer {
    fn clone(&self) -> Self {
        IntegrationLayer::new()
    }
}
