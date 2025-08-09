//! Service registry for managing system services

use super::Service;
use crate::Result;
use std::collections::HashMap;

/// Registry for managing system services
#[derive(Default)]
pub struct ServiceRegistry {
    services: HashMap<String, Box<dyn Service>>,
}

impl ServiceRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, service: Box<dyn Service>) {
        self.services.insert(service.name().to_string(), service);
    }

    pub async fn start_all(&mut self) -> Result<()> {
        for service in self.services.values_mut() {
            service.start().await?;
        }
        Ok(())
    }

    pub async fn stop_all(&mut self) -> Result<()> {
        for service in self.services.values_mut() {
            service.stop().await?;
        }
        Ok(())
    }

    pub async fn health_check_all(&self) -> Result<HashMap<String, bool>> {
        let mut results = HashMap::new();
        for (name, service) in &self.services {
            let healthy = service.health_check().await?;
            results.insert(name.clone(), healthy);
        }
        Ok(results)
    }
}
