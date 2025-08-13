# Vault - Security & Secrets Management Plan

## Goals & Vision

The `vault` crate provides secure storage and management of sensitive data for Symbiote. It offers:

- **OS Keychain Integration**: Native keychain support for Windows, macOS, and Linux
- **Encrypted Storage**: AES-256-GCM encryption for sensitive data at rest
- **Secret Management**: API keys, tokens, passwords, and certificates
- **SecretHandle System**: Opaque handles preventing raw secret exposure
- **Vault Daemon**: Privileged, isolated process for credential brokerage
- **Egress Proxy**: Policy-enforcing HTTP client with credential injection
- **UseCredRequest API**: Secure credential request/response system
- **Hardware Wallet Integration**: Ledger/Trezor support for crypto operations
- **MFA & Ephemeral Grants**: Multi-factor authentication with time-limited access
- **Prompt/Response Scrubbing**: Memory and logging hygiene for AI safety
- **Access Control**: Role-based access with audit logging
- **Key Rotation**: Automatic key rotation and versioning
- **Secure Communication**: TLS/mTLS for network communications
- **Hardware Security**: TPM and hardware security module support

This crate ensures all sensitive data in Symbiote is protected with enterprise-grade security and prevents credential leakage to AI models.

## Architecture & Design

### Core Modules

```
vault/
├── src/
│   ├── lib.rs                 # Public API exports
│   ├── daemon/               # Vault Daemon (privileged process)
│   │   ├── mod.rs
│   │   ├── server.rs         # Daemon server implementation
│   │   ├── isolation.rs      # Process isolation and sandboxing
│   │   ├── ipc.rs            # Inter-process communication
│   │   └── lifecycle.rs      # Daemon lifecycle management
│   ├── handles/              # SecretHandle system
│   │   ├── mod.rs
│   │   ├── secret_handle.rs  # Opaque secret handles
│   │   ├── handle_store.rs   # Handle-to-secret mapping
│   │   ├── expiration.rs     # Handle expiration management
│   │   └── scoping.rs        # Least-privilege scoping
│   ├── egress/               # Egress Proxy
│   │   ├── mod.rs
│   │   ├── proxy.rs          # HTTP proxy implementation
│   │   ├── policies.rs       # Request policies and allowlists
│   │   ├── injection.rs      # Credential injection
│   │   └── monitoring.rs     # Request monitoring and logging
│   ├── api/                  # UseCredRequest API
│   │   ├── mod.rs
│   │   ├── requests.rs       # Credential request types
│   │   ├── responses.rs      # Credential response types
│   │   ├── validation.rs     # Request validation
│   │   └── routing.rs        # Request routing logic
│   ├── keychain/             # OS keychain integration
│   │   ├── mod.rs
│   │   ├── windows.rs        # Windows Credential Manager
│   │   ├── macos.rs          # macOS Keychain Services
│   │   ├── linux.rs          # Linux Secret Service
│   │   └── fallback.rs       # Encrypted file fallback
│   ├── hardware_wallets/     # Hardware wallet integration
│   │   ├── mod.rs
│   │   ├── ledger.rs         # Ledger device support
│   │   ├── trezor.rs         # Trezor device support
│   │   ├── signing.rs        # Transaction signing
│   │   └── discovery.rs      # Device discovery and connection
│   ├── mfa/                  # Multi-factor authentication
│   │   ├── mod.rs
│   │   ├── totp.rs           # Time-based OTP
│   │   ├── webauthn.rs       # WebAuthn/FIDO2 support
│   │   ├── sms.rs            # SMS-based MFA
│   │   └── grants.rs         # Ephemeral grant management
│   ├── scrubbing/            # Prompt/response scrubbing
│   │   ├── mod.rs
│   │   ├── memory.rs         # Memory scrubbing utilities
│   │   ├── logging.rs        # Log sanitization
│   │   ├── patterns.rs       # Secret pattern detection
│   │   └── hygiene.rs        # Memory hygiene enforcement
│   ├── api_keys/             # API key management (NEW)
│   │   ├── mod.rs
│   │   ├── manager.rs        # Intelligent API key manager
│   │   ├── rotation.rs       # Key rotation and lifecycle
│   │   ├── usage_tracker.rs  # Usage tracking and analytics
│   │   ├── quota_manager.rs  # Quota management and enforcement
│   │   ├── failover.rs       # Failover key management
│   │   ├── cost_tracker.rs   # Cost tracking per key
│   │   └── optimizer.rs      # Key usage optimization
│   ├── crypto/               # Cryptographic operations
│   │   ├── mod.rs
│   │   ├── hkdf.rs           # HKDF key derivation
│   │   ├── signing.rs        # Digital signatures
│   │   ├── trading.rs        # Crypto trading signatures
│   │   └── derivation.rs     # API key derivation
│   ├── encryption/           # Encryption and decryption
│   │   ├── mod.rs
│   │   ├── aes.rs            # AES-256-GCM implementation
│   │   ├── key_derivation.rs # PBKDF2/Argon2 key derivation
│   │   ├── random.rs         # Cryptographically secure random
│   │   └── certificates.rs   # X.509 certificate handling
│   ├── storage/              # Secure storage backends
│   │   ├── mod.rs
│   │   ├── memory.rs         # In-memory secure storage
│   │   ├── file.rs           # Encrypted file storage
│   │   └── database.rs       # Database-backed storage
│   ├── access/               # Access control and auditing
│   │   ├── mod.rs
│   │   ├── rbac.rs           # Role-based access control
│   │   ├── policies.rs       # Security policies
│   │   ├── allowlists.rs     # Per-destination allowlists
│   │   └── audit.rs          # Audit logging
│   ├── rotation/             # Key rotation and versioning
│   │   ├── mod.rs
│   │   ├── scheduler.rs      # Rotation scheduling
│   │   ├── versioning.rs     # Key versioning
│   │   └── migration.rs      # Key migration
│   ├── hardware/             # Hardware security modules
│   │   ├── mod.rs
│   │   ├── tpm.rs            # TPM integration
│   │   └── hsm.rs            # HSM support
│   └── types/                # Security-related types
│       ├── mod.rs
│       ├── secrets.rs        # Secret value types
│       ├── handles.rs        # SecretHandle types
│       ├── keys.rs           # Cryptographic keys
│       ├── policies.rs       # Security policy types
│       └── threats.rs        # Threat model types
├── tests/
│   ├── integration/
│   └── unit/
└── examples/
    ├── basic_secrets.rs
    └── keychain_integration.rs
```

### Key Design Principles

1. **Defense in Depth**: Multiple layers of security protection
2. **Zero Trust**: Verify all access requests and operations
3. **Least Privilege**: Minimal access rights by default
4. **Audit Everything**: Comprehensive logging of all operations
5. **Fail Secure**: Secure defaults and graceful failure modes

## APIs & Interfaces

### Vault Daemon (Privileged Process)

```rust
/// Main vault daemon that runs as a privileged, isolated process
pub struct VaultDaemon {
    keychain: Box<dyn KeychainProvider>,
    storage: Box<dyn SecureStorage>,
    handle_store: SecretHandleStore,
    egress_proxy: EgressProxy,
    access_control: AccessController,
    mfa_manager: MfaManager,
    audit_logger: AuditLogger,
    scrubber: PromptResponseScrubber,
    config: VaultConfig,
}

impl VaultDaemon {
    pub async fn new(config: VaultConfig) -> VaultResult<Self>;

    /// Start the daemon server
    pub async fn start(&self) -> VaultResult<()>;

    /// Handle UseCredRequest from agent runtime
    pub async fn handle_cred_request(&self, request: UseCredRequest) -> VaultResult<UseCredResponse>;

    /// Create opaque secret handle (never exposes raw secrets)
    pub async fn create_secret_handle(&self, secret_id: &str, scope: AccessScope) -> VaultResult<SecretHandle>;

    /// Revoke secret handle
    pub async fn revoke_handle(&self, handle: &SecretHandle) -> VaultResult<()>;

    /// Process egress HTTP request with credential injection
    pub async fn process_egress_request(&self, request: EgressRequest) -> VaultResult<EgressResponse>;

    /// Perform MFA challenge for sensitive operations
    pub async fn mfa_challenge(&self, user_id: &str, operation: &str) -> VaultResult<MfaChallenge>;

    /// Verify MFA response and create ephemeral grant
    pub async fn verify_mfa_and_grant(&self, challenge_id: &str, response: MfaResponse) -> VaultResult<EphemeralGrant>;
}

### Agent Runtime (Unprivileged Process)

```rust
/// Agent runtime that communicates with vault daemon via IPC
pub struct AgentVaultClient {
    daemon_client: VaultDaemonClient,
    handle_cache: HandleCache,
    scrubber: MemoryScrubber,
}

impl AgentVaultClient {
    pub async fn new() -> VaultResult<Self>;

    /// Request credential use (returns opaque handle, never raw secret)
    pub async fn use_credential(&self, request: UseCredRequest) -> VaultResult<SecretHandle>;

    /// Make HTTP request through egress proxy
    pub async fn egress_request(&self, request: EgressRequest) -> VaultResult<EgressResponse>;

    /// Sign crypto/trading request
    pub async fn sign_trading_request(&self, request: TradingRequest) -> VaultResult<SignedRequest>;

    /// Derive API key using HKDF
    pub async fn derive_api_key(&self, base_key_handle: SecretHandle, context: &[u8]) -> VaultResult<SecretHandle>;
}

### Vault Manager (Legacy Interface)

```rust
pub struct VaultManager {
    keychain: Box<dyn KeychainProvider>,
    storage: Box<dyn SecureStorage>,
    access_control: AccessController,
    audit_logger: AuditLogger,
    config: VaultConfig,
}

impl VaultManager {
    pub async fn new(config: VaultConfig) -> VaultResult<Self>;
    
    pub async fn store_secret(&self, key: &str, secret: SecretValue) -> VaultResult<()>;
    
    pub async fn retrieve_secret(&self, key: &str) -> VaultResult<Option<SecretValue>>;
    
    pub async fn delete_secret(&self, key: &str) -> VaultResult<()>;
    
    pub async fn list_secrets(&self) -> VaultResult<Vec<SecretMetadata>>;
    
    pub async fn rotate_key(&self, key: &str) -> VaultResult<()>;
    
    pub async fn backup_vault(&self, path: &Path) -> VaultResult<BackupInfo>;

    pub async fn restore_vault(&self, backup_path: &Path) -> VaultResult<()>;

    pub async fn create_access_token(&self, permissions: Vec<Permission>) -> VaultResult<AccessToken>;

    pub async fn revoke_access_token(&self, token_id: &str) -> VaultResult<()>;

    pub async fn audit_access(&self, filters: AuditFilters) -> VaultResult<Vec<AuditEntry>>;

    pub async fn set_policy(&self, key: &str, policy: AccessPolicy) -> VaultResult<()>;

    pub async fn encrypt_data(&self, data: &[u8], key_id: &str) -> VaultResult<EncryptedData>;

    pub async fn decrypt_data(&self, encrypted: &EncryptedData) -> VaultResult<Vec<u8>>;

    pub async fn generate_key(&self, key_type: KeyType, metadata: KeyMetadata) -> VaultResult<KeyId>;

    pub async fn import_key(&self, key_data: &[u8], key_type: KeyType) -> VaultResult<KeyId>;

    pub async fn export_key(&self, key_id: &KeyId, format: KeyFormat) -> VaultResult<Vec<u8>>;

    pub async fn sign_data(&self, data: &[u8], key_id: &KeyId) -> VaultResult<Signature>;

    pub async fn verify_signature(&self, data: &[u8], signature: &Signature, key_id: &KeyId) -> VaultResult<bool>;
}

/// Intelligent API Key Manager with rotation and quota management (NEW)
pub struct APIKeyManager {
    key_storage: SecureKeyStorage,
    rotation_manager: KeyRotationManager,
    usage_tracker: KeyUsageTracker,
    quota_manager: QuotaManager,
    failover_manager: FailoverKeyManager,
    cost_tracker: KeyCostTracker,
    optimizer: KeyUsageOptimizer,
    vault_client: AgentVaultClient,
}

impl APIKeyManager {
    pub async fn new(config: APIKeyConfig) -> VaultResult<Self>;

    /// Get best available key for provider and request type
    pub async fn get_optimal_key(&self, provider: ProviderId, request_type: RequestType) -> VaultResult<SecretHandle>;

    /// Rotate keys based on usage patterns and security policies
    pub async fn rotate_keys(&self, rotation_policy: RotationPolicy) -> VaultResult<RotationReport>;

    /// Track usage and enforce quotas
    pub async fn track_usage(&self, key_id: KeyId, usage: Usage) -> VaultResult<()>;

    /// Handle key failures and failover
    pub async fn handle_key_failure(&self, key_id: KeyId, failure_reason: FailureReason) -> VaultResult<SecretHandle>;

    /// Optimize costs across multiple keys
    pub async fn optimize_key_usage(&self, cost_targets: CostTargets) -> VaultResult<OptimizationPlan>;

    /// Get key usage analytics
    pub async fn get_usage_analytics(&self, timeframe: TimeRange) -> VaultResult<UsageAnalytics>;

    /// Add new API key with metadata
    pub async fn add_api_key(&self, provider: ProviderId, key: APIKey, metadata: KeyMetadata) -> VaultResult<KeyId>;

    /// Remove API key
    pub async fn remove_api_key(&self, key_id: KeyId) -> VaultResult<()>;

    /// List all keys for a provider
    pub async fn list_keys(&self, provider: ProviderId) -> VaultResult<Vec<KeyInfo>>;
}
```

### SecretHandle System (Opaque Handles)

```rust
/// Opaque handle that never exposes raw secrets to agent runtime
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SecretHandle {
    id: HandleId,
    scope: AccessScope,
    expiration: Option<DateTime<Utc>>,
    // Raw secret data is NEVER included in this struct
}

impl SecretHandle {
    /// Check if handle is still valid
    pub fn is_valid(&self) -> bool;

    /// Get handle scope (what operations are allowed)
    pub fn scope(&self) -> &AccessScope;

    /// Get expiration time
    pub fn expires_at(&self) -> Option<DateTime<Utc>>;
}

/// UseCredRequest API for requesting credential access
#[derive(Debug, Clone)]
pub struct UseCredRequest {
    pub credential_id: String,
    pub operation: CredentialOperation,
    pub destination: Option<String>,
    pub scope: AccessScope,
    pub justification: String,
    pub mfa_token: Option<String>,
}

#[derive(Debug, Clone)]
pub struct UseCredResponse {
    pub handle: SecretHandle,
    pub expires_at: DateTime<Utc>,
    pub allowed_operations: Vec<CredentialOperation>,
}

#[derive(Debug, Clone)]
pub enum CredentialOperation {
    HttpRequest { method: String, url: String },
    ApiCall { service: String, endpoint: String },
    CryptoSigning { transaction_type: String },
    DatabaseAccess { database: String, operation: String },
    OAuth { provider: String, scopes: Vec<String> },
}

### Egress Proxy System

```rust
/// Policy-enforcing HTTP client that injects credentials
pub struct EgressProxy {
    policies: PolicyEngine,
    allowlists: DestinationAllowlists,
    credential_injector: CredentialInjector,
    monitor: RequestMonitor,
}

#[derive(Debug, Clone)]
pub struct EgressRequest {
    pub method: String,
    pub url: String,
    pub headers: HashMap<String, String>,
    pub body: Option<Vec<u8>>,
    pub credential_handle: Option<SecretHandle>,
    pub policy_context: PolicyContext,
}

#[derive(Debug, Clone)]
pub struct EgressResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
    pub scrubbed: bool, // Whether response was scrubbed of secrets
}

### Hardware Wallet Integration

```rust
/// Hardware wallet manager for crypto operations
pub struct HardwareWalletManager {
    ledger_client: Option<LedgerClient>,
    trezor_client: Option<TrezorClient>,
    discovery: DeviceDiscovery,
}

impl HardwareWalletManager {
    pub async fn discover_devices(&self) -> VaultResult<Vec<WalletDevice>>;

    pub async fn sign_transaction(&self, device: &WalletDevice, tx: &Transaction) -> VaultResult<Signature>;

    pub async fn get_public_key(&self, device: &WalletDevice, path: &DerivationPath) -> VaultResult<PublicKey>;

    pub async fn verify_device(&self, device: &WalletDevice) -> VaultResult<bool>;
}

### Secret Types

```rust
#[derive(Debug, Clone)]
pub struct SecretValue {
    data: SecureBytes,
    metadata: SecretMetadata,
}

impl SecretValue {
    pub fn new(data: impl Into<SecureBytes>) -> Self;
    pub fn from_string(s: String) -> Self;
    pub fn from_bytes(bytes: Vec<u8>) -> Self;
    
    pub fn expose_secret(&self) -> &[u8];
    pub fn expose_string(&self) -> VaultResult<&str>;
    
    pub fn metadata(&self) -> &SecretMetadata;
}

#[derive(Debug, Clone)]
pub struct SecretMetadata {
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub secret_type: SecretType,
    pub tags: HashMap<String, String>,
    pub access_policy: AccessPolicy,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SecretType {
    ApiKey,
    Password,
    Token,
    Certificate,
    PrivateKey,
    DatabaseCredentials,
    Custom(String),
}
```

### Keychain Integration

```rust
pub trait KeychainProvider: Send + Sync {
    async fn store(&self, service: &str, account: &str, secret: &[u8]) -> VaultResult<()>;
    
    async fn retrieve(&self, service: &str, account: &str) -> VaultResult<Option<Vec<u8>>>;
    
    async fn delete(&self, service: &str, account: &str) -> VaultResult<()>;
    
    async fn list(&self, service: &str) -> VaultResult<Vec<String>>;
    
    fn is_available(&self) -> bool;
    
    fn provider_name(&self) -> &'static str;
}

#[cfg(target_os = "windows")]
pub struct WindowsCredentialManager {
    // Windows Credential Manager implementation
}

#[cfg(target_os = "macos")]
pub struct MacOSKeychain {
    // macOS Keychain Services implementation
}

#[cfg(target_os = "linux")]
pub struct LinuxSecretService {
    // Linux Secret Service implementation
}

pub struct EncryptedFileKeychain {
    // Fallback encrypted file implementation
}
```

### Encryption Services

```rust
pub struct EncryptionService {
    master_key: MasterKey,
    cipher: AesGcm<Aes256>,
    key_derivation: KeyDerivation,
}

impl EncryptionService {
    pub fn new(master_key: MasterKey) -> VaultResult<Self>;
    
    pub fn encrypt(&self, plaintext: &[u8]) -> VaultResult<EncryptedData>;
    
    pub fn decrypt(&self, encrypted: &EncryptedData) -> VaultResult<Vec<u8>>;
    
    pub fn derive_key(&self, password: &str, salt: &[u8]) -> VaultResult<DerivedKey>;
    
    pub fn generate_salt(&self) -> [u8; 32];
    
    pub fn generate_nonce(&self) -> [u8; 12];
}

#[derive(Debug, Clone)]
pub struct EncryptedData {
    pub ciphertext: Vec<u8>,
    pub nonce: [u8; 12],
    pub tag: [u8; 16],
    pub algorithm: EncryptionAlgorithm,
    pub key_version: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EncryptionAlgorithm {
    Aes256Gcm,
    ChaCha20Poly1305,
}
```

### Access Control

```rust
pub struct AccessController {
    policies: HashMap<String, AccessPolicy>,
    roles: HashMap<String, Role>,
    audit_logger: AuditLogger,
}

impl AccessController {
    pub fn new() -> Self;
    
    pub fn check_access(&self, principal: &Principal, resource: &str, action: Action) -> VaultResult<bool>;
    
    pub fn grant_permission(&mut self, principal: &Principal, resource: &str, permissions: Vec<Permission>) -> VaultResult<()>;
    
    pub fn revoke_permission(&mut self, principal: &Principal, resource: &str, permissions: Vec<Permission>) -> VaultResult<()>;
    
    pub fn create_role(&mut self, name: &str, permissions: Vec<Permission>) -> VaultResult<()>;
    
    pub fn assign_role(&mut self, principal: &Principal, role: &str) -> VaultResult<()>;
}

#[derive(Debug, Clone)]
pub struct Principal {
    pub id: String,
    pub principal_type: PrincipalType,
    pub attributes: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PrincipalType {
    User,
    Service,
    Agent,
    System,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    Read,
    Write,
    Delete,
    List,
    Rotate,
    Backup,
    Restore,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Permission {
    Allow(Action),
    Deny(Action),
}
```

### Multi-Factor Authentication & Ephemeral Grants

```rust
/// MFA manager for sensitive operations
pub struct MfaManager {
    totp_provider: TotpProvider,
    webauthn_provider: WebAuthnProvider,
    sms_provider: Option<SmsProvider>,
    grant_store: EphemeralGrantStore,
}

impl MfaManager {
    pub async fn create_challenge(&self, user_id: &str, operation: &str) -> VaultResult<MfaChallenge>;

    pub async fn verify_response(&self, challenge_id: &str, response: MfaResponse) -> VaultResult<bool>;

    pub async fn create_ephemeral_grant(&self, user_id: &str, scope: AccessScope, duration: Duration) -> VaultResult<EphemeralGrant>;

    pub async fn validate_grant(&self, grant: &EphemeralGrant) -> VaultResult<bool>;

    pub async fn revoke_grant(&self, grant_id: &str) -> VaultResult<()>;
}

#[derive(Debug, Clone)]
pub struct MfaChallenge {
    pub id: String,
    pub challenge_type: MfaChallengeType,
    pub challenge_data: Vec<u8>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub enum MfaChallengeType {
    Totp,
    WebAuthn { credential_ids: Vec<String> },
    Sms { phone_number_hint: String },
}

#[derive(Debug, Clone)]
pub struct EphemeralGrant {
    pub id: String,
    pub user_id: String,
    pub scope: AccessScope,
    pub granted_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub used_count: u32,
    pub max_uses: Option<u32>,
}

### Prompt/Response Scrubbing & Memory Hygiene

```rust
/// Scrubber for preventing credential leakage to AI models
pub struct PromptResponseScrubber {
    secret_patterns: SecretPatternMatcher,
    memory_scrubber: MemoryScrubber,
    log_sanitizer: LogSanitizer,
}

impl PromptResponseScrubber {
    pub fn new() -> Self;

    /// Scrub secrets from prompt before sending to AI
    pub fn scrub_prompt(&self, prompt: &str) -> ScrubResult<String>;

    /// Scrub secrets from AI response
    pub fn scrub_response(&self, response: &str) -> ScrubResult<String>;

    /// Scrub memory regions containing sensitive data
    pub fn scrub_memory(&self, ptr: *mut u8, len: usize);

    /// Sanitize log entries
    pub fn sanitize_log_entry(&self, entry: &str) -> String;

    /// Add custom secret pattern
    pub fn add_pattern(&mut self, pattern: SecretPattern);
}

#[derive(Debug, Clone)]
pub struct SecretPattern {
    pub name: String,
    pub regex: String,
    pub replacement: String,
    pub confidence: f64,
}

#[derive(Debug, Clone)]
pub struct ScrubResult<T> {
    pub data: T,
    pub secrets_found: u32,
    pub patterns_matched: Vec<String>,
    pub scrubbed: bool,
}

### HKDF Key Derivation & Crypto Operations

```rust
/// HKDF-based key derivation for API keys
pub struct HkdfKeyDerivation {
    master_key: SecretHandle,
    info_context: Vec<u8>,
}

impl HkdfKeyDerivation {
    pub fn new(master_key: SecretHandle, context: &[u8]) -> Self;

    /// Derive API key from master key and context
    pub async fn derive_api_key(&self, service: &str, user_id: &str) -> VaultResult<SecretHandle>;

    /// Derive signing key for crypto operations
    pub async fn derive_signing_key(&self, purpose: &str, key_path: &[u32]) -> VaultResult<SecretHandle>;
}

/// Crypto/Trading request signing
pub struct CryptoSigner {
    hardware_wallets: HardwareWalletManager,
    software_keys: SoftwareKeyStore,
}

impl CryptoSigner {
    pub async fn sign_transaction(&self, tx: &Transaction, key_handle: SecretHandle) -> VaultResult<Signature>;

    pub async fn sign_trading_request(&self, request: &TradingRequest, key_handle: SecretHandle) -> VaultResult<SignedTradingRequest>;

    pub async fn verify_signature(&self, data: &[u8], signature: &Signature, public_key: &PublicKey) -> VaultResult<bool>;
}

### Per-Destination Allowlists

```rust
/// Destination allowlist manager
pub struct DestinationAllowlists {
    allowlists: HashMap<String, DestinationAllowlist>,
    default_policy: AllowlistPolicy,
}

impl DestinationAllowlists {
    pub fn new(default_policy: AllowlistPolicy) -> Self;

    /// Check if destination is allowed for credential
    pub fn is_allowed(&self, credential_id: &str, destination: &str) -> bool;

    /// Add destination to allowlist
    pub fn add_destination(&mut self, credential_id: &str, destination: &str) -> VaultResult<()>;

    /// Remove destination from allowlist
    pub fn remove_destination(&mut self, credential_id: &str, destination: &str) -> VaultResult<()>;

    /// Get all allowed destinations for credential
    pub fn get_allowed_destinations(&self, credential_id: &str) -> Vec<String>;
}

#[derive(Debug, Clone)]
pub struct DestinationAllowlist {
    pub credential_id: String,
    pub allowed_domains: Vec<String>,
    pub allowed_ips: Vec<IpAddr>,
    pub allowed_patterns: Vec<String>,
    pub policy: AllowlistPolicy,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AllowlistPolicy {
    AllowAll,
    DenyAll,
    AllowListed,
    DenyListed,
}
```

## Implementation Details

### Technology Stack

- **Encryption**: AES-256-GCM with ring or RustCrypto
- **Key Derivation**: Argon2id for password-based keys
- **Random Generation**: OsRng for cryptographically secure randomness
- **Keychain Integration**: keyring-rs for cross-platform support
- **Certificates**: rustls and x509-parser for TLS/X.509
- **Hardware Security**: tpm-rs for TPM integration
- **Audit Logging**: Structured logging with tamper detection

### Key Dependencies

```toml
[dependencies]
ring = "0.16"
aes-gcm = "0.10"
argon2 = "0.5"
keyring = "2.0"
rustls = "0.21"
x509-parser = "0.15"
rand = "0.8"
zeroize = { version = "1.6", features = ["derive"] }
secrecy = "0.8"
tokio = { version = "1.0", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1.0", features = ["v4", "serde"] }
thiserror = "1.0"
tracing = "0.1"
symbiote-core = { path = "../symbiote-core" }

[target.'cfg(windows)'.dependencies]
windows = { version = "0.48", features = ["Win32_Security_Credentials"] }

[target.'cfg(target_os = "macos")'.dependencies]
security-framework = "2.9"

[target.'cfg(target_os = "linux")'.dependencies]
secret-service = "3.0"

[dev-dependencies]
tempfile = "3.0"
tokio-test = "0.4"
```

### Security Considerations

1. **Memory Protection**: Use zeroize to clear sensitive data from memory
2. **Side-Channel Resistance**: Constant-time operations for cryptographic functions
3. **Key Management**: Secure key generation, storage, and rotation
4. **Audit Trail**: Immutable audit logs with integrity verification
5. **Error Handling**: Avoid information leakage in error messages

### Platform-Specific Implementation

```rust
#[cfg(target_os = "windows")]
impl KeychainProvider for WindowsCredentialManager {
    async fn store(&self, service: &str, account: &str, secret: &[u8]) -> VaultResult<()> {
        use windows::Win32::Security::Credentials::*;
        // Windows Credential Manager API implementation
    }
}

#[cfg(target_os = "macos")]
impl KeychainProvider for MacOSKeychain {
    async fn store(&self, service: &str, account: &str, secret: &[u8]) -> VaultResult<()> {
        use security_framework::passwords::*;
        // macOS Keychain Services implementation
    }
}

#[cfg(target_os = "linux")]
impl KeychainProvider for LinuxSecretService {
    async fn store(&self, service: &str, account: &str, secret: &[u8]) -> VaultResult<()> {
        use secret_service::*;
        // Linux Secret Service implementation
    }
}
```

## Testing Strategy

### Unit Tests

- **Encryption/Decryption**: Test all cryptographic operations
- **Key Derivation**: Test password-based key generation
- **Access Control**: Test permission checking and role management
- **Keychain Integration**: Test platform-specific keychain operations
- **Secret Management**: Test secret lifecycle operations

### Integration Tests

- **Cross-Platform**: Test on Windows, macOS, and Linux
- **Hardware Security**: Test TPM and HSM integration
- **Performance**: Test encryption/decryption performance
- **Concurrent Access**: Test thread safety and concurrent operations
- **Error Scenarios**: Test failure modes and recovery

### Security Tests

- **Penetration Testing**: Test against common attack vectors
- **Side-Channel Analysis**: Test for timing and power analysis vulnerabilities
- **Memory Analysis**: Test for sensitive data leakage in memory
- **Audit Verification**: Test audit log integrity and completeness

## Integration Points

### Upstream Dependencies

- **symbiote-core**: Uses error types, configuration, and logging
- **Operating System**: Native keychain and security services
- **Hardware**: TPM and HSM for hardware-backed security

### Downstream Consumers

- **AI Crate**: Stores API keys and authentication tokens
- **Agent Framework**: Stores agent credentials and certificates
- **Workflow Engine**: Stores service credentials and secrets
- **Trading System**: Stores exchange API keys and wallet credentials
- **Settings System**: Stores encrypted configuration values

### External Integrations

- **Cloud Key Management**: AWS KMS, Azure Key Vault, Google Cloud KMS
- **Hardware Security Modules**: PKCS#11 compatible HSMs
- **Certificate Authorities**: Automated certificate provisioning
- **Identity Providers**: OAuth, SAML, and OpenID Connect integration

## Acceptance Criteria

### Functional Requirements

- [ ] Cross-platform keychain integration (Windows, macOS, Linux)
- [ ] AES-256-GCM encryption for data at rest
- [ ] Role-based access control with audit logging
- [ ] Automatic key rotation and versioning
- [ ] Hardware security module support
- [ ] Secure backup and restore functionality
- [ ] Certificate and private key management

### Non-Functional Requirements

- [ ] Sub-millisecond encryption/decryption operations
- [ ] Zero memory leakage of sensitive data
- [ ] 99.99% availability for secret retrieval
- [ ] FIPS 140-2 Level 2 compliance where applicable
- [ ] Resistance to side-channel attacks
- [ ] Comprehensive audit trail for all operations

### Quality Gates

- [ ] Security audit by external firm
- [ ] Penetration testing passes
- [ ] Memory safety verification
- [ ] Performance benchmarks meet targets
- [ ] Cross-platform compatibility verified
- [ ] Documentation includes security best practices

## Dependencies & Prerequisites

### Build Dependencies

- **Rust Toolchain**: 1.70+ with crypto feature support
- **Platform SDKs**: Windows SDK, Xcode, Linux development packages
- **Cryptographic Libraries**: OpenSSL or native crypto libraries

### Runtime Dependencies

- **Operating System**: Native keychain services
- **Hardware**: TPM 2.0 for hardware-backed security (optional)
- **Certificates**: Root CA certificates for TLS verification
- **Memory**: Secure memory allocation support

### Development Prerequisites

- **Security Tools**: Static analysis tools for cryptographic code
- **Testing Hardware**: TPM-enabled devices for testing
- **Certificate Management**: Test certificate authority for development
- **Audit Tools**: Log analysis and integrity verification tools

## UI Specifications

### Vault Management Dashboard

```
┌─────────────────────────────────────────────────────────────────────────────┐
│ 🔐 Vault Security Center                           [🔄] [⚙️] [📊] [🔒] [🚨] │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│ 🛡️ Security Overview                                                        │
│ ┌─────────────────────────────────────────────────────────────────────────┐ │
│ │ Vault Status: 🟢 Secure       │ Secrets Stored: 47                      │ │
│ │ Last Backup: 2h ago           │ Active Sessions: 3                       │ │
│ │ Key Rotation: ✅ Up to date   │ Failed Attempts: 0 today                │ │
│ │ Hardware Security: 🟢 TPM     │ Audit Events: 156 today                 │ │
│ └─────────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
│ 🔑 Secret Management                                                        │
│ ┌─────────────────────────────────────────────────────────────────────────┐ │
│ │ Secret Type          │ Count │ Status    │ Last Rotation │ Actions       │ │
│ │ API Keys             │ 23    │ 🟢 Secure │ 7 days ago    │ [🔄][👁️][🗑️] │ │
│ │ Database Credentials │ 8     │ 🟢 Secure │ 14 days ago   │ [🔄][👁️][🗑️] │ │
│ │ Certificates         │ 12    │ 🟡 Expiring│ 30 days ago   │ [🔄][👁️][🗑️] │ │
│ │ Trading Keys         │ 4     │ 🟢 Secure │ 3 days ago    │ [🔄][👁️][🗑️] │ │
│ │ [➕ Add Secret] [📥 Import] [📤 Export] [🔄 Rotate All]                  │ │
│ └─────────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
│ 🔐 Access Control                                                           │
│ ┌─────────────────────────────────────────────────────────────────────────┐ │
│ │ User/Service         │ Role      │ Permissions │ Last Access │ Status    │ │
│ │ ai-service           │ AI Agent  │ Read API    │ 5 min ago   │ 🟢 Active │ │
│ │ trading-bot          │ Trader    │ Sign Txns   │ 12 min ago  │ 🟢 Active │ │
│ │ workflow-engine      │ Workflow  │ Read Creds  │ 1 hour ago  │ 🟢 Active │ │
│ │ admin-user           │ Admin     │ Full Access │ 2 hours ago │ 🟢 Active │ │
│ │ [➕ Add User] [👥 Manage Roles] [🔒 Revoke Access] [📊 Audit Log]        │ │
│ └─────────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
│ 🚨 Security Alerts                                                          │
│ │ • Certificate for api.example.com expires in 7 days                     │ │
│ │ • Unusual access pattern detected for trading-bot service               │ │
│ │ • Key rotation recommended for database credentials                      │ │
│ │ • Hardware security module health check passed                          │ │
│ │ [📋 View All Alerts] [🔔 Alert Settings] [📊 Security Report]            │ │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Secret Creation Interface

```
┌─────────────────────────────────────────────────────────────────────────────┐
│ ➕ Add New Secret                                                    [✕]   │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│ 📝 Secret Details                                                           │
│ ┌─────────────────────────────────────────────────────────────────────────┐ │
│ │ Secret Name: [OpenAI API Key                                    ]        │ │
│ │ Secret Type: [API Key ▼]                                                │ │
│ │ Category:    [AI Services ▼]                                            │ │
│ │ Description: [API key for OpenAI GPT-4 access                  ]        │ │
│ └─────────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
│ 🔐 Secret Value                                                             │
│ ┌─────────────────────────────────────────────────────────────────────────┐ │
│ │ Value: [••••••••••••••••••••••••••••••••••••••••••••••••••••••••]       │ │
│ │ ☑️ Generate secure value  ☐ Import from file  ☐ Manual entry           │ │
│ │                                                                         │ │
│ │ Strength: [████████████████████████████████████████] 256-bit           │ │
│ │ Entropy:  [████████████████████████████████████████] Excellent         │ │
│ └─────────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
│ 🛡️ Security Settings                                                        │
│ ┌─────────────────────────────────────────────────────────────────────────┐ │
│ │ Access Control:                                                         │ │
│ │ ├─ Allowed Services: [ai-service, workflow-engine] [+ Add]              │ │
│ │ ├─ Allowed Destinations: [api.openai.com] [+ Add]                       │ │
│ │ ├─ Rotation Policy: [Every 90 days ▼]                                  │ │
│ │ └─ Backup Policy: [Include in backups ☑️]                              │ │
│ │                                                                         │ │
│ │ Hardware Security:                                                      │ │
│ │ ├─ ☑️ Store in TPM                                                      │ │
│ │ ├─ ☑️ Require hardware authentication                                   │ │
│ │ └─ ☐ Use hardware wallet for signing                                   │ │
│ └─────────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
│ 📋 Metadata                                                                 │
│ ┌─────────────────────────────────────────────────────────────────────────┐ │
│ │ Tags: [openai, gpt-4, production] [+ Add Tag]                          │ │
│ │ Owner: [admin-user ▼]                                                   │ │
│ │ Expiration: [Never ▼] or [Custom Date: ___________]                     │ │
│ │ Notes: [Production API key with $100/month limit               ]        │ │
│ └─────────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
│ [💾 Save Secret] [🧪 Test Connection] [📋 Generate Handle] [❌ Cancel]      │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Database Schema

### Vault Data Persistence

```sql
-- Secret storage and metadata
CREATE TABLE vault_secrets (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    secret_id VARCHAR(255) NOT NULL UNIQUE,
    secret_name VARCHAR(255) NOT NULL,
    secret_type VARCHAR(100) NOT NULL, -- 'api_key', 'password', 'certificate', 'token'
    category VARCHAR(100), -- 'ai_services', 'databases', 'trading', 'infrastructure'
    description TEXT,
    encrypted_value BYTEA NOT NULL, -- AES-256-GCM encrypted secret value
    encryption_key_id VARCHAR(255) NOT NULL, -- Reference to encryption key
    nonce BYTEA NOT NULL, -- Encryption nonce/IV
    value_hash VARCHAR(255) NOT NULL, -- Hash for integrity verification
    metadata JSONB, -- Additional secret metadata
    tags JSONB, -- Array of tags for categorization
    created_by VARCHAR(255) NOT NULL,
    owned_by VARCHAR(255) NOT NULL,
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW(),
    last_accessed TIMESTAMP,
    access_count INTEGER DEFAULT 0,
    is_active BOOLEAN DEFAULT true,
    expires_at TIMESTAMP,
    rotation_policy JSONB, -- Rotation configuration
    last_rotated TIMESTAMP,
    backup_included BOOLEAN DEFAULT true,
    hardware_backed BOOLEAN DEFAULT false,
    tpm_sealed BOOLEAN DEFAULT false
);

-- Secret access control and permissions
CREATE TABLE vault_access_control (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    secret_id VARCHAR(255) REFERENCES vault_secrets(secret_id),
    principal_id VARCHAR(255) NOT NULL, -- User, service, or role ID
    principal_type VARCHAR(50) NOT NULL, -- 'user', 'service', 'role'
    permission_type VARCHAR(50) NOT NULL, -- 'read', 'write', 'rotate', 'delete'
    granted_by VARCHAR(255) NOT NULL,
    granted_at TIMESTAMP DEFAULT NOW(),
    expires_at TIMESTAMP,
    conditions JSONB, -- Access conditions (time, location, etc.)
    is_active BOOLEAN DEFAULT true,
    metadata JSONB
);

-- Secret handles and opaque references
CREATE TABLE vault_secret_handles (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    handle_id VARCHAR(255) NOT NULL UNIQUE,
    secret_id VARCHAR(255) REFERENCES vault_secrets(secret_id),
    created_for VARCHAR(255) NOT NULL, -- Service or user that requested the handle
    scope VARCHAR(255) NOT NULL, -- Access scope for the handle
    created_at TIMESTAMP DEFAULT NOW(),
    expires_at TIMESTAMP NOT NULL,
    last_used TIMESTAMP,
    use_count INTEGER DEFAULT 0,
    max_uses INTEGER, -- Optional use limit
    is_revoked BOOLEAN DEFAULT false,
    revoked_at TIMESTAMP,
    revoked_by VARCHAR(255),
    metadata JSONB
);

-- Audit trail for all vault operations
CREATE TABLE vault_audit_log (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_id VARCHAR(255) NOT NULL UNIQUE,
    event_type VARCHAR(100) NOT NULL, -- 'secret_created', 'secret_accessed', 'handle_created', etc.
    secret_id VARCHAR(255),
    handle_id VARCHAR(255),
    principal_id VARCHAR(255) NOT NULL,
    principal_type VARCHAR(50) NOT NULL,
    operation VARCHAR(100) NOT NULL,
    resource VARCHAR(255),
    result VARCHAR(50), -- 'success', 'failure', 'denied'
    error_message TEXT,
    source_ip INET,
    user_agent TEXT,
    session_id VARCHAR(255),
    request_id VARCHAR(255),
    occurred_at TIMESTAMP DEFAULT NOW(),
    metadata JSONB,
    integrity_hash VARCHAR(255) -- For audit log integrity
);

-- Encryption keys and key management
CREATE TABLE vault_encryption_keys (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    key_id VARCHAR(255) NOT NULL UNIQUE,
    key_type VARCHAR(50) NOT NULL, -- 'master', 'derived', 'hardware'
    algorithm VARCHAR(50) NOT NULL, -- 'AES-256-GCM', 'ChaCha20-Poly1305'
    encrypted_key BYTEA, -- Encrypted key material (if software-based)
    key_derivation JSONB, -- Key derivation parameters
    hardware_key_ref VARCHAR(255), -- Reference to hardware key (TPM, HSM)
    created_at TIMESTAMP DEFAULT NOW(),
    expires_at TIMESTAMP,
    is_active BOOLEAN DEFAULT true,
    rotation_schedule JSONB,
    last_rotated TIMESTAMP,
    version INTEGER DEFAULT 1,
    metadata JSONB
);

-- Hardware security module and TPM integration
CREATE TABLE vault_hardware_security (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    device_id VARCHAR(255) NOT NULL UNIQUE,
    device_type VARCHAR(50) NOT NULL, -- 'tpm', 'hsm', 'hardware_wallet'
    device_name VARCHAR(255),
    manufacturer VARCHAR(255),
    model VARCHAR(255),
    firmware_version VARCHAR(100),
    is_available BOOLEAN DEFAULT true,
    last_health_check TIMESTAMP,
    health_status VARCHAR(50), -- 'healthy', 'warning', 'error'
    capabilities JSONB, -- Supported operations and algorithms
    configuration JSONB,
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW(),
    metadata JSONB
);

-- Backup and recovery information
CREATE TABLE vault_backups (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    backup_id VARCHAR(255) NOT NULL UNIQUE,
    backup_type VARCHAR(50) NOT NULL, -- 'full', 'incremental', 'differential'
    backup_location VARCHAR(500) NOT NULL,
    encryption_key_id VARCHAR(255) REFERENCES vault_encryption_keys(key_id),
    backup_size BIGINT,
    secret_count INTEGER,
    checksum VARCHAR(255),
    created_at TIMESTAMP DEFAULT NOW(),
    created_by VARCHAR(255),
    expires_at TIMESTAMP,
    is_encrypted BOOLEAN DEFAULT true,
    compression_type VARCHAR(50),
    metadata JSONB
);

-- MFA challenges and ephemeral grants
CREATE TABLE vault_mfa_challenges (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    challenge_id VARCHAR(255) NOT NULL UNIQUE,
    user_id VARCHAR(255) NOT NULL,
    challenge_type VARCHAR(50) NOT NULL, -- 'totp', 'sms', 'email', 'hardware'
    operation VARCHAR(255) NOT NULL, -- Operation requiring MFA
    challenge_data JSONB, -- Challenge-specific data
    created_at TIMESTAMP DEFAULT NOW(),
    expires_at TIMESTAMP NOT NULL,
    attempts INTEGER DEFAULT 0,
    max_attempts INTEGER DEFAULT 3,
    is_verified BOOLEAN DEFAULT false,
    verified_at TIMESTAMP,
    metadata JSONB
);

-- Ephemeral access grants
CREATE TABLE vault_ephemeral_grants (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    grant_id VARCHAR(255) NOT NULL UNIQUE,
    user_id VARCHAR(255) NOT NULL,
    scope VARCHAR(255) NOT NULL, -- Access scope granted
    permissions JSONB NOT NULL, -- Array of granted permissions
    created_at TIMESTAMP DEFAULT NOW(),
    expires_at TIMESTAMP NOT NULL,
    is_revoked BOOLEAN DEFAULT false,
    revoked_at TIMESTAMP,
    revoked_by VARCHAR(255),
    mfa_challenge_id VARCHAR(255) REFERENCES vault_mfa_challenges(challenge_id),
    metadata JSONB
);

-- Destination allowlists for credential usage
CREATE TABLE vault_destination_allowlists (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    secret_id VARCHAR(255) REFERENCES vault_secrets(secret_id),
    destination_pattern VARCHAR(500) NOT NULL, -- URL pattern or hostname
    destination_type VARCHAR(50), -- 'hostname', 'url_pattern', 'ip_range'
    is_allowed BOOLEAN DEFAULT true,
    created_by VARCHAR(255) NOT NULL,
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW(),
    metadata JSONB,
    UNIQUE(secret_id, destination_pattern)
);

-- Indexes for performance
CREATE INDEX idx_vault_secrets_type ON vault_secrets(secret_type);
CREATE INDEX idx_vault_secrets_category ON vault_secrets(category);
CREATE INDEX idx_vault_secrets_owner ON vault_secrets(owned_by);
CREATE INDEX idx_vault_secrets_active ON vault_secrets(is_active);
CREATE INDEX idx_vault_secrets_expires ON vault_secrets(expires_at);
CREATE INDEX idx_vault_secrets_tags ON vault_secrets USING GIN(tags);
CREATE INDEX idx_vault_access_control_secret ON vault_access_control(secret_id);
CREATE INDEX idx_vault_access_control_principal ON vault_access_control(principal_id);
CREATE INDEX idx_vault_access_control_active ON vault_access_control(is_active);
CREATE INDEX idx_vault_secret_handles_secret ON vault_secret_handles(secret_id);
CREATE INDEX idx_vault_secret_handles_created_for ON vault_secret_handles(created_for);
CREATE INDEX idx_vault_secret_handles_expires ON vault_secret_handles(expires_at);
CREATE INDEX idx_vault_secret_handles_revoked ON vault_secret_handles(is_revoked);
CREATE INDEX idx_vault_audit_log_event_type ON vault_audit_log(event_type);
CREATE INDEX idx_vault_audit_log_principal ON vault_audit_log(principal_id);
CREATE INDEX idx_vault_audit_log_occurred_at ON vault_audit_log(occurred_at);
CREATE INDEX idx_vault_audit_log_secret ON vault_audit_log(secret_id);
CREATE INDEX idx_vault_encryption_keys_active ON vault_encryption_keys(is_active);
CREATE INDEX idx_vault_encryption_keys_expires ON vault_encryption_keys(expires_at);
CREATE INDEX idx_vault_hardware_security_type ON vault_hardware_security(device_type);
CREATE INDEX idx_vault_hardware_security_available ON vault_hardware_security(is_available);
CREATE INDEX idx_vault_backups_created_at ON vault_backups(created_at);
CREATE INDEX idx_vault_backups_expires ON vault_backups(expires_at);
CREATE INDEX idx_vault_mfa_challenges_user ON vault_mfa_challenges(user_id);
CREATE INDEX idx_vault_mfa_challenges_expires ON vault_mfa_challenges(expires_at);
CREATE INDEX idx_vault_mfa_challenges_verified ON vault_mfa_challenges(is_verified);
CREATE INDEX idx_vault_ephemeral_grants_user ON vault_ephemeral_grants(user_id);
CREATE INDEX idx_vault_ephemeral_grants_expires ON vault_ephemeral_grants(expires_at);
CREATE INDEX idx_vault_ephemeral_grants_revoked ON vault_ephemeral_grants(is_revoked);
CREATE INDEX idx_vault_destination_allowlists_secret ON vault_destination_allowlists(secret_id);
```

## Error Handling

### Vault Error Types

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum VaultError {
    #[error("Secret storage failed: {secret_id} - {reason}")]
    SecretStorageFailed { secret_id: String, reason: String },

    #[error("Secret retrieval failed: {secret_id} - {error}")]
    SecretRetrievalFailed { secret_id: String, error: String },

    #[error("Access denied: {principal} - {operation} - {resource}")]
    AccessDenied { principal: String, operation: String, resource: String },

    #[error("Encryption failed: {key_id} - {reason}")]
    EncryptionFailed { key_id: String, reason: String },

    #[error("Decryption failed: {key_id} - {error}")]
    DecryptionFailed { key_id: String, error: String },

    #[error("Key derivation failed: {algorithm} - {reason}")]
    KeyDerivationFailed { algorithm: String, reason: String },

    #[error("Hardware security module error: {device} - {error}")]
    HardwareSecurityError { device: String, error: String },

    #[error("Keychain operation failed: {operation} - {platform} - {reason}")]
    KeychainOperationFailed { operation: String, platform: String, reason: String },

    #[error("Secret handle creation failed: {secret_id} - {error}")]
    HandleCreationFailed { secret_id: String, error: String },

    #[error("Secret handle expired: {handle_id}")]
    HandleExpired { handle_id: String },

    #[error("Secret handle revoked: {handle_id} - {reason}")]
    HandleRevoked { handle_id: String, reason: String },

    #[error("MFA challenge failed: {challenge_id} - {error}")]
    MfaChallengeFailed { challenge_id: String, error: String },

    #[error("Ephemeral grant creation failed: {user_id} - {reason}")]
    EphemeralGrantFailed { user_id: String, reason: String },

    #[error("Key rotation failed: {key_id} - {error}")]
    KeyRotationFailed { key_id: String, error: String },

    #[error("Backup operation failed: {backup_type} - {reason}")]
    BackupOperationFailed { backup_type: String, reason: String },

    #[error("Restore operation failed: {backup_id} - {error}")]
    RestoreOperationFailed { backup_id: String, error: String },

    #[error("Audit logging failed: {event_type} - {reason}")]
    AuditLoggingFailed { event_type: String, reason: String },

    #[error("Certificate operation failed: {operation} - {error}")]
    CertificateOperationFailed { operation: String, error: String },

    #[error("Hardware wallet operation failed: {device} - {operation} - {reason}")]
    HardwareWalletOperationFailed { device: String, operation: String, reason: String },

    #[error("Destination not allowed: {destination} - {secret_id}")]
    DestinationNotAllowed { destination: String, secret_id: String },

    #[error("Secret not found: {secret_id}")]
    SecretNotFound { secret_id: String },

    #[error("Invalid secret format: {secret_type} - {validation_error}")]
    InvalidSecretFormat { secret_type: String, validation_error: String },

    #[error("Configuration error: {setting} - {issue}")]
    ConfigurationError { setting: String, issue: String },

    #[error("Permission denied: {operation} - {resource}")]
    PermissionDenied { operation: String, resource: String },

    #[error("Database error: {operation} - {error}")]
    DatabaseError { operation: String, error: String },

    #[error("Serialization error: {data_type} - {error}")]
    SerializationError { data_type: String, error: String },

    #[error("External service error: {service} - {error}")]
    ExternalServiceError { service: String, error: String },

    #[error("Internal error: {details}")]
    InternalError { details: String },
}

pub type VaultResult<T> = Result<T, VaultError>;

impl From<std::io::Error> for VaultError {
    fn from(err: std::io::Error) -> Self {
        VaultError::InternalError {
            details: format!("IO error: {}", err),
        }
    }
}

impl From<serde_json::Error> for VaultError {
    fn from(err: serde_json::Error) -> Self {
        VaultError::SerializationError {
            data_type: "json".to_string(),
            error: err.to_string(),
        }
    }
}

impl From<ring::error::Unspecified> for VaultError {
    fn from(err: ring::error::Unspecified) -> Self {
        VaultError::EncryptionFailed {
            key_id: "unknown".to_string(),
            reason: format!("Cryptographic operation failed: {}", err),
        }
    }
}
```

This vault system provides the security foundation that enables Symbiote to safely handle sensitive data while maintaining enterprise-grade security standards and compliance requirements.
