//! Dependency injection system for Symbiote

use crate::{SymbioteError, SymbioteResult};
use async_trait::async_trait;
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub mod container;
pub mod registry;
pub mod lifecycle;

pub use container::*;
pub use registry::*;
pub use lifecycle::*;

/// Service trait that all injectable services must implement
#[async_trait]
pub trait Service: Send + Sync + 'static {
    /// Get the service name
    fn name(&self) -> &'static str;

    /// Get the list of service dependencies
    fn dependencies(&self) -> Vec<&'static str> {
        Vec::new()
    }

    /// Initialize the service
    async fn initialize(&mut self) -> SymbioteResult<()> {
        Ok(())
    }

    /// Shutdown the service
    async fn shutdown(&mut self) -> SymbioteResult<()> {
        Ok(())
    }

    /// Health check for the service
    async fn health_check(&self) -> SymbioteResult<ServiceHealth> {
        Ok(ServiceHealth::Healthy)
    }
}

/// Service health status
#[derive(Debug, Clone, PartialEq)]
pub enum ServiceHealth {
    /// Service is healthy and operational
    Healthy,
    /// Service is degraded but functional
    Degraded(String),
    /// Service is unhealthy
    Unhealthy(String),
}

/// Service lifecycle state
#[derive(Debug, Clone, PartialEq)]
pub enum ServiceState {
    /// Service is registered but not initialized
    Registered,
    /// Service is being initialized
    Initializing,
    /// Service is initialized and ready
    Ready,
    /// Service is being shut down
    ShuttingDown,
    /// Service has been shut down
    Shutdown,
    /// Service failed during initialization or operation
    Failed(String),
}

/// Service container for dependency injection
pub struct ServiceContainer {
    services: RwLock<HashMap<TypeId, Arc<dyn Any + Send + Sync>>>,
    singletons: RwLock<HashMap<TypeId, Arc<dyn Any + Send + Sync>>>,
    factories: RwLock<HashMap<TypeId, Box<dyn Fn() -> Box<dyn Any + Send + Sync> + Send + Sync>>>,
    states: RwLock<HashMap<TypeId, ServiceState>>,
    dependency_graph: RwLock<HashMap<TypeId, Vec<TypeId>>>,
}

impl ServiceContainer {
    /// Create a new service container
    pub fn new() -> Self {
        Self {
            services: RwLock::new(HashMap::new()),
            singletons: RwLock::new(HashMap::new()),
            factories: RwLock::new(HashMap::new()),
            states: RwLock::new(HashMap::new()),
            dependency_graph: RwLock::new(HashMap::new()),
        }
    }

    /// Register a service instance
    pub async fn register<T: Service>(&self, service: T) -> SymbioteResult<()> {
        let type_id = TypeId::of::<T>();
        let service_arc = Arc::new(service);

        {
            let mut services = self.services.write().await;
            services.insert(type_id, service_arc.clone());
        }

        {
            let mut states = self.states.write().await;
            states.insert(type_id, ServiceState::Registered);
        }

        tracing::debug!("Registered service: {}", std::any::type_name::<T>());
        Ok(())
    }

    /// Register a singleton service
    pub async fn register_singleton<T: Service>(&self, service: T) -> SymbioteResult<()> {
        let type_id = TypeId::of::<T>();
        let service_arc = Arc::new(service);

        {
            let mut singletons = self.singletons.write().await;
            singletons.insert(type_id, service_arc.clone());
        }

        {
            let mut states = self.states.write().await;
            states.insert(type_id, ServiceState::Registered);
        }

        tracing::debug!("Registered singleton service: {}", std::any::type_name::<T>());
        Ok(())
    }

    /// Register a service factory
    pub async fn register_factory<T: Service, F>(&self, factory: F) -> SymbioteResult<()>
    where
        F: Fn() -> T + Send + Sync + 'static,
    {
        let type_id = TypeId::of::<T>();
        let factory_fn = Box::new(move || -> Box<dyn Any + Send + Sync> {
            Box::new(factory())
        });

        {
            let mut factories = self.factories.write().await;
            factories.insert(type_id, factory_fn);
        }

        {
            let mut states = self.states.write().await;
            states.insert(type_id, ServiceState::Registered);
        }

        tracing::debug!("Registered factory for service: {}", std::any::type_name::<T>());
        Ok(())
    }

    /// Resolve a service instance
    pub async fn resolve<T: Service>(&self) -> SymbioteResult<Arc<T>> {
        let type_id = TypeId::of::<T>();

        // Check singletons first
        {
            let singletons = self.singletons.read().await;
            if let Some(service) = singletons.get(&type_id) {
                return service
                    .clone()
                    .downcast::<T>()
                    .map_err(|_| SymbioteError::service("ServiceContainer", "Failed to downcast singleton service"));
            }
        }

        // Check regular services
        {
            let services = self.services.read().await;
            if let Some(service) = services.get(&type_id) {
                return service
                    .clone()
                    .downcast::<T>()
                    .map_err(|_| SymbioteError::service("ServiceContainer", "Failed to downcast service"));
            }
        }

        // Try factory
        {
            let factories = self.factories.read().await;
            if let Some(factory) = factories.get(&type_id) {
                let instance = factory();
                return instance
                    .downcast::<T>()
                    .map(|boxed| Arc::new(*boxed))
                    .map_err(|_| SymbioteError::service("ServiceContainer", "Failed to downcast factory service"));
            }
        }

        Err(SymbioteError::not_found("Service", std::any::type_name::<T>()))
    }

    /// Initialize all registered services
    pub async fn initialize_all(&self) -> SymbioteResult<()> {
        tracing::info!("Initializing all services");

        // TODO: Implement proper dependency resolution order
        // For now, initialize in registration order

        let service_ids: Vec<TypeId> = {
            let states = self.states.read().await;
            states.keys().cloned().collect()
        };

        for type_id in service_ids {
            self.initialize_service(type_id).await?;
        }

        tracing::info!("All services initialized successfully");
        Ok(())
    }

    /// Shutdown all services
    pub async fn shutdown_all(&self) -> SymbioteResult<()> {
        tracing::info!("Shutting down all services");

        let service_ids: Vec<TypeId> = {
            let states = self.states.read().await;
            states.keys().cloned().collect()
        };

        // Shutdown in reverse order
        for type_id in service_ids.into_iter().rev() {
            self.shutdown_service(type_id).await?;
        }

        tracing::info!("All services shut down successfully");
        Ok(())
    }

    async fn initialize_service(&self, type_id: TypeId) -> SymbioteResult<()> {
        {
            let mut states = self.states.write().await;
            states.insert(type_id, ServiceState::Initializing);
        }

        // TODO: Implement actual service initialization
        // This would require storing mutable references or using interior mutability

        {
            let mut states = self.states.write().await;
            states.insert(type_id, ServiceState::Ready);
        }

        Ok(())
    }

    async fn shutdown_service(&self, type_id: TypeId) -> SymbioteResult<()> {
        {
            let mut states = self.states.write().await;
            states.insert(type_id, ServiceState::ShuttingDown);
        }

        // TODO: Implement actual service shutdown

        {
            let mut states = self.states.write().await;
            states.insert(type_id, ServiceState::Shutdown);
        }

        Ok(())
    }
}

impl Default for ServiceContainer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestService {
        name: String,
    }

    #[async_trait]
    impl Service for TestService {
        fn name(&self) -> &'static str {
            "TestService"
        }

        async fn initialize(&mut self) -> SymbioteResult<()> {
            tracing::debug!("Initializing {}", self.name);
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_service_registration() {
        let container = ServiceContainer::new();
        let service = TestService {
            name: "test".to_string(),
        };

        let result = container.register(service).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_service_resolution() {
        let container = ServiceContainer::new();
        let service = TestService {
            name: "test".to_string(),
        };

        container.register(service).await.unwrap();
        let resolved = container.resolve::<TestService>().await;
        assert!(resolved.is_ok());
    }
}
