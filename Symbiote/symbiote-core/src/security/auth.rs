//! Authentication abstractions

use crate::{Result, UserId};

#[derive(Debug, Clone)]
pub struct AuthToken {
    pub token: String,
    pub user_id: UserId,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

/// Authentication provider trait
#[async_trait::async_trait]
pub trait AuthProvider: Send + Sync {
    async fn authenticate(&self, credentials: &str) -> Result<AuthToken>;
    async fn validate_token(&self, token: &str) -> Result<UserId>;
    async fn revoke_token(&self, token: &str) -> Result<()>;
}
