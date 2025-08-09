//! Database migration system

use crate::Result;

/// Migration trait
#[async_trait::async_trait]
pub trait Migration: Send + Sync {
    fn version(&self) -> u64;
    fn description(&self) -> &str;
    async fn up(&self) -> Result<()>;
    async fn down(&self) -> Result<()>;
}
