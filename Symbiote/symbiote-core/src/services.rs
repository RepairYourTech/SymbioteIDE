//! Service layer for Symbiote IDE

pub mod service_registry;

pub use service_registry::ServiceRegistry;

/// Service trait for all system services
#[async_trait::async_trait]
pub trait Service: Send + Sync {
    /// Service name
    fn name(&self) -> &str;
    
    /// Start the service
    async fn start(&mut self) -> crate::Result<()>;
    
    /// Stop the service
    async fn stop(&mut self) -> crate::Result<()>;
    
    /// Check if service is healthy
    async fn health_check(&self) -> crate::Result<bool>;
}
