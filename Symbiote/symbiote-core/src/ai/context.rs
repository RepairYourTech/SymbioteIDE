//! Context management for AI

use crate::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Context {
    pub id: String,
    pub content: String,
    pub metadata: std::collections::HashMap<String, String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Context manager trait
#[async_trait::async_trait]
pub trait ContextManager: Send + Sync {
    async fn store_context(&self, context: Context) -> Result<()>;
    async fn retrieve_context(&self, id: &str) -> Result<Option<Context>>;
    async fn search_context(&self, query: &str) -> Result<Vec<Context>>;
}
