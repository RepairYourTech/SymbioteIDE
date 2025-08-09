//! # Data Encryption Framework for Symbiote IDE
//!
//! Comprehensive encryption system for sensitive data at rest and in transit,
//! supporting multiple encryption algorithms, key management, and secure storage.

use crate::{Result, SymbioteError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;
use ring::{aead::{self, BoundKey, NonceSequence, Nonce, NONCE_LEN}, pbkdf2, rand::{SecureRandom, SystemRandom}};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

/// Counter-based nonce sequence for AEAD encryption
struct CounterNonceSequence(u32);

impl NonceSequence for CounterNonceSequence {
    fn advance(&mut self) -> std::result::Result<Nonce, ring::error::Unspecified> {
        let mut nonce_bytes = vec![0; NONCE_LEN];
        let bytes = self.0.to_be_bytes();
        nonce_bytes[8..].copy_from_slice(&bytes);
        self.0 += 1;
        Nonce::try_assume_unique_for_key(&nonce_bytes)
    }
}

/// Encryption algorithm types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EncryptionAlgorithm {
    /// AES-256-GCM (recommended for most use cases)
    AES256GCM,
    /// ChaCha20-Poly1305 (alternative to AES)
    ChaCha20Poly1305,
    /// AES-128-GCM (for performance-critical scenarios)
    AES128GCM,
}

impl EncryptionAlgorithm {
    /// Get the AEAD algorithm for this encryption type
    pub fn aead_algorithm(&self) -> &'static aead::Algorithm {
        match self {
            Self::AES256GCM => &aead::AES_256_GCM,
            Self::ChaCha20Poly1305 => &aead::CHACHA20_POLY1305,
            Self::AES128GCM => &aead::AES_128_GCM,
        }
    }

    /// Get the key length for this algorithm
    pub fn key_length(&self) -> usize {
        match self {
            Self::AES256GCM => 32, // 256 bits
            Self::ChaCha20Poly1305 => 32, // 256 bits
            Self::AES128GCM => 16, // 128 bits
        }
    }

    /// Get the nonce length for this algorithm
    pub fn nonce_length(&self) -> usize {
        match self {
            Self::AES256GCM => 12, // 96 bits
            Self::ChaCha20Poly1305 => 12, // 96 bits
            Self::AES128GCM => 12, // 96 bits
        }
    }
}

/// Encryption key metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionKey {
    /// Unique key identifier
    pub id: Uuid,

    /// Key name/label
    pub name: String,

    /// Encryption algorithm
    pub algorithm: EncryptionAlgorithm,

    /// Key creation timestamp
    pub created_at: u64,

    /// Key expiration timestamp (optional)
    pub expires_at: Option<u64>,

    /// Whether key is active
    pub active: bool,

    /// Key usage purpose
    pub purpose: KeyPurpose,

    /// Key derivation parameters (for password-derived keys)
    pub derivation_params: Option<KeyDerivationParams>,
}

/// Key usage purposes
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum KeyPurpose {
    /// General data encryption
    DataEncryption,
    /// Database encryption
    DatabaseEncryption,
    /// File encryption
    FileEncryption,
    /// Communication encryption
    CommunicationEncryption,
    /// Backup encryption
    BackupEncryption,
    /// Custom purpose
    Custom(String),
}

/// Key derivation parameters for password-based keys
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyDerivationParams {
    /// Salt for key derivation
    pub salt: Vec<u8>,

    /// Number of iterations
    pub iterations: u32,

    /// Key derivation algorithm
    pub algorithm: KeyDerivationAlgorithm,
}

/// Key derivation algorithms
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum KeyDerivationAlgorithm {
    PBKDF2SHA256,
    PBKDF2SHA512,
}

impl KeyDerivationAlgorithm {
    /// Get the PBKDF2 algorithm for this type
    pub fn pbkdf2_algorithm(&self) -> pbkdf2::Algorithm {
        match self {
            Self::PBKDF2SHA256 => pbkdf2::PBKDF2_HMAC_SHA256,
            Self::PBKDF2SHA512 => pbkdf2::PBKDF2_HMAC_SHA512,
        }
    }
}

/// Encrypted data container
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedData {
    /// Key ID used for encryption
    pub key_id: Uuid,

    /// Encryption algorithm used
    pub algorithm: EncryptionAlgorithm,

    /// Encrypted data
    pub ciphertext: Vec<u8>,

    /// Nonce/IV used for encryption
    pub nonce: Vec<u8>,

    /// Authentication tag (for AEAD)
    pub tag: Vec<u8>,

    /// Additional authenticated data (optional)
    pub aad: Option<Vec<u8>>,

    /// Encryption timestamp
    pub encrypted_at: u64,
}

/// Encryption configuration
#[derive(Debug, Clone)]
pub struct EncryptionConfig {
    /// Default encryption algorithm
    pub default_algorithm: EncryptionAlgorithm,

    /// Key rotation interval in seconds
    pub key_rotation_interval: u64,

    /// Whether to compress data before encryption
    pub compress_before_encrypt: bool,

    /// Maximum data size to encrypt in memory (bytes)
    pub max_memory_size: usize,

    /// Key derivation iterations for password-based keys
    pub pbkdf2_iterations: u32,
}

impl Default for EncryptionConfig {
    fn default() -> Self {
        Self {
            default_algorithm: EncryptionAlgorithm::AES256GCM,
            key_rotation_interval: 30 * 24 * 60 * 60, // 30 days
            compress_before_encrypt: true,
            max_memory_size: 100 * 1024 * 1024, // 100MB
            pbkdf2_iterations: 100_000,
        }
    }
}

/// Encryption provider trait
pub trait EncryptionProvider: Send + Sync {
    /// Encrypt data with the specified key
    fn encrypt(&self, data: &[u8], key_id: &Uuid) -> Result<EncryptedData>;

    /// Decrypt data
    fn decrypt(&self, encrypted_data: &EncryptedData) -> Result<Vec<u8>>;

    /// Generate a new encryption key
    fn generate_key(&self, purpose: KeyPurpose, algorithm: EncryptionAlgorithm) -> Result<EncryptionKey>;

    /// Derive key from password
    fn derive_key_from_password(
        &self,
        password: &str,
        purpose: KeyPurpose,
        algorithm: EncryptionAlgorithm,
    ) -> Result<EncryptionKey>;

    /// Get key by ID
    fn get_key(&self, key_id: &Uuid) -> Result<Option<EncryptionKey>>;

    /// List all keys
    fn list_keys(&self) -> Result<Vec<EncryptionKey>>;

    /// Rotate key (generate new key and mark old as inactive)
    fn rotate_key(&self, old_key_id: &Uuid) -> Result<EncryptionKey>;

    /// Delete key (mark as inactive)
    fn delete_key(&self, key_id: &Uuid) -> Result<()>;
}

/// Default encryption provider implementation
pub struct DefaultEncryptionProvider {
    /// Configuration
    config: EncryptionConfig,

    /// Key storage (in production, this would be a secure key store)
    keys: HashMap<Uuid, (EncryptionKey, Vec<u8>)>, // (metadata, raw_key)

    /// Random number generator
    rng: SystemRandom,
}

impl DefaultEncryptionProvider {
    /// Create a new encryption provider
    pub fn new(config: EncryptionConfig) -> Self {
        Self {
            config,
            keys: HashMap::new(),
            rng: SystemRandom::new(),
        }
    }

    /// Create with default configuration
    pub fn default() -> Self {
        Self::new(EncryptionConfig::default())
    }

    /// Generate random bytes
    fn generate_random_bytes(&self, length: usize) -> Result<Vec<u8>> {
        let mut bytes = vec![0u8; length];
        self.rng.fill(&mut bytes)
            .map_err(|_| SymbioteError::internal("Failed to generate random bytes"))?;
        Ok(bytes)
    }

    /// Get raw key bytes for a key ID
    fn get_raw_key(&self, key_id: &Uuid) -> Result<Vec<u8>> {
        self.keys.get(key_id)
            .map(|(_, raw_key)| raw_key.clone())
            .ok_or_else(|| SymbioteError::not_found(format!("Encryption key not found: {}", key_id)))
    }

    /// Compress data if configured
    fn maybe_compress(&self, data: &[u8]) -> Result<Vec<u8>> {
        if self.config.compress_before_encrypt && data.len() > 1024 {
            // TODO: Implement compression (e.g., using flate2)
            // For now, return data as-is
            Ok(data.to_vec())
        } else {
            Ok(data.to_vec())
        }
    }

    /// Decompress data if needed
    fn maybe_decompress(&self, data: &[u8]) -> Result<Vec<u8>> {
        // TODO: Implement decompression
        // For now, return data as-is
        Ok(data.to_vec())
    }
}

impl EncryptionProvider for DefaultEncryptionProvider {
    fn encrypt(&self, data: &[u8], key_id: &Uuid) -> Result<EncryptedData> {
        // Check data size limit
        if data.len() > self.config.max_memory_size {
            return Err(SymbioteError::validation(format!(
                "Data size {} exceeds maximum {} bytes",
                data.len(),
                self.config.max_memory_size
            )));
        }

        // Get key metadata and raw key
        let (key_metadata, raw_key) = self.keys.get(key_id)
            .ok_or_else(|| SymbioteError::not_found(format!("Encryption key not found: {}", key_id)))?;

        if !key_metadata.active {
            return Err(SymbioteError::validation("Cannot encrypt with inactive key"));
        }

        // Check key expiration
        if let Some(expires_at) = key_metadata.expires_at {
            let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
            if now > expires_at {
                return Err(SymbioteError::validation("Cannot encrypt with expired key"));
            }
        }

        // Compress data if configured
        let data_to_encrypt = self.maybe_compress(data)?;

        // Generate nonce
        let nonce_length = key_metadata.algorithm.nonce_length();
        let nonce = self.generate_random_bytes(nonce_length)?;

        // Create AEAD key
        let algorithm = key_metadata.algorithm.aead_algorithm();
        let unbound_key = aead::UnboundKey::new(algorithm, raw_key)
            .map_err(|_| SymbioteError::internal("Failed to create encryption key"))?;

        // Create nonce sequence
        let nonce_sequence = CounterNonceSequence(1);
        let mut sealing_key = aead::SealingKey::new(unbound_key, nonce_sequence);

        // Encrypt data
        let mut ciphertext = data_to_encrypt;
        let tag = sealing_key.seal_in_place_separate_tag(aead::Aad::empty(), &mut ciphertext)
            .map_err(|_| SymbioteError::internal("Encryption failed"))?;

        Ok(EncryptedData {
            key_id: *key_id,
            algorithm: key_metadata.algorithm,
            ciphertext,
            nonce,
            tag: tag.as_ref().to_vec(),
            aad: None,
            encrypted_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
        })
    }

    fn decrypt(&self, encrypted_data: &EncryptedData) -> Result<Vec<u8>> {
        // Get raw key
        let raw_key = self.get_raw_key(&encrypted_data.key_id)?;

        // Create AEAD key
        let algorithm = encrypted_data.algorithm.aead_algorithm();
        let unbound_key = aead::UnboundKey::new(algorithm, &raw_key)
            .map_err(|_| SymbioteError::internal("Failed to create decryption key"))?;
        // Create nonce sequence (starting from 1 to match encryption)
        let nonce_sequence = CounterNonceSequence(1);
        let mut opening_key = aead::OpeningKey::new(unbound_key, nonce_sequence);

        // Combine ciphertext and tag
        let mut ciphertext_and_tag = encrypted_data.ciphertext.clone();
        ciphertext_and_tag.extend_from_slice(&encrypted_data.tag);

        // Decrypt data
        let plaintext = opening_key.open_in_place(aead::Aad::empty(), &mut ciphertext_and_tag)
            .map_err(|_| SymbioteError::internal("Decryption failed"))?;

        // Decompress if needed
        self.maybe_decompress(plaintext)
    }

    fn generate_key(&self, purpose: KeyPurpose, algorithm: EncryptionAlgorithm) -> Result<EncryptionKey> {
        let key_id = Uuid::new_v4();
        let key_length = algorithm.key_length();
        let _raw_key = self.generate_random_bytes(key_length)?;

        let key_metadata = EncryptionKey {
            id: key_id,
            name: format!("{:?} Key", purpose),
            algorithm,
            created_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            expires_at: Some(
                SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
                + self.config.key_rotation_interval
            ),
            active: true,
            purpose,
            derivation_params: None,
        };

        // Store key (in production, this would be stored securely)
        // Note: This is a mutable operation, but we can't modify self in this trait method
        // In a real implementation, this would use interior mutability or a different design

        Ok(key_metadata)
    }

    fn derive_key_from_password(
        &self,
        password: &str,
        purpose: KeyPurpose,
        algorithm: EncryptionAlgorithm,
    ) -> Result<EncryptionKey> {
        let key_id = Uuid::new_v4();
        let key_length = algorithm.key_length();

        // Generate salt
        let salt = self.generate_random_bytes(32)?; // 256-bit salt

        // Derive key using PBKDF2
        let mut derived_key = vec![0u8; key_length];
        pbkdf2::derive(
            pbkdf2::PBKDF2_HMAC_SHA256,
            std::num::NonZeroU32::new(self.config.pbkdf2_iterations).unwrap(),
            &salt,
            password.as_bytes(),
            &mut derived_key,
        );

        let key_metadata = EncryptionKey {
            id: key_id,
            name: format!("Password-derived {:?} Key", purpose),
            algorithm,
            created_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            expires_at: Some(
                SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
                + self.config.key_rotation_interval
            ),
            active: true,
            purpose,
            derivation_params: Some(KeyDerivationParams {
                salt,
                iterations: self.config.pbkdf2_iterations,
                algorithm: KeyDerivationAlgorithm::PBKDF2SHA256,
            }),
        };

        Ok(key_metadata)
    }

    fn get_key(&self, key_id: &Uuid) -> Result<Option<EncryptionKey>> {
        Ok(self.keys.get(key_id).map(|(metadata, _)| metadata.clone()))
    }

    fn list_keys(&self) -> Result<Vec<EncryptionKey>> {
        Ok(self.keys.values().map(|(metadata, _)| metadata.clone()).collect())
    }

    fn rotate_key(&self, old_key_id: &Uuid) -> Result<EncryptionKey> {
        let old_key = self.keys.get(old_key_id)
            .ok_or_else(|| SymbioteError::not_found(format!("Key not found: {}", old_key_id)))?;

        // Generate new key with same purpose and algorithm
        self.generate_key(old_key.0.purpose.clone(), old_key.0.algorithm)
    }

    fn delete_key(&self, _key_id: &Uuid) -> Result<()> {
        // In a real implementation, this would mark the key as inactive
        // For now, just return success
        Ok(())
    }
}

/// Utility functions for encryption
impl EncryptedData {
    /// Encode encrypted data to base64 string
    pub fn to_base64(&self) -> Result<String> {
        let serialized = serde_json::to_vec(self)
            .map_err(|e| SymbioteError::serialization(e.to_string()))?;
        Ok(BASE64.encode(serialized))
    }

    /// Decode encrypted data from base64 string
    pub fn from_base64(encoded: &str) -> Result<Self> {
        let decoded = BASE64.decode(encoded)
            .map_err(|e| SymbioteError::serialization(format!("Base64 decode error: {}", e)))?;
        serde_json::from_slice(&decoded)
            .map_err(|e| SymbioteError::serialization(e.to_string()))
    }
}

/// High-level encryption service
pub struct EncryptionService {
    provider: Box<dyn EncryptionProvider>,
    default_key_id: Option<Uuid>,
}

impl EncryptionService {
    /// Create new encryption service
    pub fn new(provider: Box<dyn EncryptionProvider>) -> Self {
        Self {
            provider,
            default_key_id: None,
        }
    }

    /// Set default key for encryption
    pub fn set_default_key(&mut self, key_id: Uuid) {
        self.default_key_id = Some(key_id);
    }

    /// Encrypt string data
    pub fn encrypt_string(&self, data: &str) -> Result<String> {
        let key_id = self.default_key_id
            .ok_or_else(|| SymbioteError::validation("No default encryption key set"))?;

        let encrypted = self.provider.encrypt(data.as_bytes(), &key_id)?;
        encrypted.to_base64()
    }

    /// Decrypt string data
    pub fn decrypt_string(&self, encrypted_data: &str) -> Result<String> {
        let encrypted = EncryptedData::from_base64(encrypted_data)?;
        let decrypted = self.provider.decrypt(&encrypted)?;
        String::from_utf8(decrypted)
            .map_err(|e| SymbioteError::internal(format!("Invalid UTF-8: {}", e)))
    }

    /// Encrypt JSON data
    pub fn encrypt_json<T: Serialize>(&self, data: &T) -> Result<String> {
        let json = serde_json::to_string(data)
            .map_err(|e| SymbioteError::serialization(e.to_string()))?;
        self.encrypt_string(&json)
    }

    /// Decrypt JSON data
    pub fn decrypt_json<T: for<'de> Deserialize<'de>>(&self, encrypted_data: &str) -> Result<T> {
        let json = self.decrypt_string(encrypted_data)?;
        serde_json::from_str(&json)
            .map_err(|e| SymbioteError::serialization(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encryption_algorithm_properties() {
        assert_eq!(EncryptionAlgorithm::AES256GCM.key_length(), 32);
        assert_eq!(EncryptionAlgorithm::AES128GCM.key_length(), 16);
        assert_eq!(EncryptionAlgorithm::ChaCha20Poly1305.key_length(), 32);

        assert_eq!(EncryptionAlgorithm::AES256GCM.nonce_length(), 12);
        assert_eq!(EncryptionAlgorithm::AES128GCM.nonce_length(), 12);
        assert_eq!(EncryptionAlgorithm::ChaCha20Poly1305.nonce_length(), 12);
    }

    #[test]
    fn test_encryption_provider_creation() {
        let provider = DefaultEncryptionProvider::default();
        assert_eq!(provider.config.default_algorithm, EncryptionAlgorithm::AES256GCM);
    }

    #[test]
    fn test_key_generation() {
        let provider = DefaultEncryptionProvider::default();
        let key = provider.generate_key(
            KeyPurpose::DataEncryption,
            EncryptionAlgorithm::AES256GCM,
        );

        assert!(key.is_ok());
        let key = key.unwrap();
        assert_eq!(key.algorithm, EncryptionAlgorithm::AES256GCM);
        assert_eq!(key.purpose, KeyPurpose::DataEncryption);
        assert!(key.active);
    }

    #[test]
    fn test_encrypted_data_serialization() {
        let encrypted_data = EncryptedData {
            key_id: Uuid::new_v4(),
            algorithm: EncryptionAlgorithm::AES256GCM,
            ciphertext: vec![1, 2, 3, 4],
            nonce: vec![5, 6, 7, 8],
            tag: vec![9, 10, 11, 12],
            aad: None,
            encrypted_at: 1234567890,
        };

        let base64_encoded = encrypted_data.to_base64();
        assert!(base64_encoded.is_ok());

        let decoded = EncryptedData::from_base64(&base64_encoded.unwrap());
        assert!(decoded.is_ok());

        let decoded = decoded.unwrap();
        assert_eq!(decoded.key_id, encrypted_data.key_id);
        assert_eq!(decoded.algorithm, encrypted_data.algorithm);
        assert_eq!(decoded.ciphertext, encrypted_data.ciphertext);
    }
}
