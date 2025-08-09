//! Encryption utilities

use crate::Result;

/// Encryption provider trait
pub trait EncryptionProvider: Send + Sync {
    fn encrypt(&self, data: &[u8]) -> Result<Vec<u8>>;
    fn decrypt(&self, encrypted_data: &[u8]) -> Result<Vec<u8>>;
}
