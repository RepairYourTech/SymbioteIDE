//! Database connection management

use crate::Result;

/// Database connection trait
#[async_trait::async_trait]
pub trait DatabaseConnection: Send + Sync {
    async fn connect(&mut self) -> Result<()>;
    async fn disconnect(&mut self) -> Result<()>;
    async fn health_check(&self) -> Result<bool>;
}
