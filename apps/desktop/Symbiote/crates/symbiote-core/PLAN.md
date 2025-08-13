# Symbiote Core - Foundation Crate Plan

## Goals & Vision

The `symbiote-core` crate serves as the foundational layer for the entire Symbiote ecosystem. It provides:

- **Unified Type System**: Common types, traits, and abstractions used across all crates
- **Error Handling**: Comprehensive error types and result patterns
- **Dependency Injection**: Service container and dependency resolution
- **Configuration**: Core configuration structures and validation
- **Async Runtime**: Tokio-based async primitives and utilities
- **Logging & Tracing**: Structured logging and observability foundations

This crate establishes the architectural patterns and conventions that all other Symbiote components follow.

## Architecture & Design

### Core Modules

```
symbiote-core/
├── src/
│   ├── lib.rs                 # Public API exports
│   ├── types/                 # Common types and traits
│   │   ├── mod.rs
│   │   ├── id.rs             # Typed IDs (AgentId, WorkflowId, etc.)
│   │   ├── result.rs         # Result types and error handling
│   │   ├── config.rs         # Configuration traits and types
│   │   └── metadata.rs       # Common metadata structures
│   ├── error/                # Error handling system
│   │   ├── mod.rs
│   │   ├── core.rs           # Core error types
│   │   ├── chain.rs          # Error chaining utilities
│   │   └── context.rs        # Error context and debugging
│   ├── di/                   # Dependency injection
│   │   ├── mod.rs
│   │   ├── container.rs      # Service container
│   │   ├── registry.rs       # Service registry
│   │   └── lifecycle.rs      # Service lifecycle management
│   ├── async_utils/          # Async utilities
│   │   ├── mod.rs
│   │   ├── runtime.rs        # Runtime management
│   │   ├── channels.rs       # Channel utilities
│   │   └── timeout.rs        # Timeout and cancellation
│   ├── logging/              # Logging infrastructure
│   │   ├── mod.rs
│   │   ├── structured.rs     # Structured logging
│   │   ├── filters.rs        # Log filtering
│   │   └── formatters.rs     # Log formatting
│   ├── validation/           # Input validation
│   │   ├── mod.rs
│   │   ├── rules.rs          # Validation rules
│   │   └── sanitization.rs   # Input sanitization
│   ├── events/              # Event system
│   │   ├── mod.rs
│   │   ├── bus.rs            # Event bus implementation
│   │   ├── handlers.rs       # Event handler registry
│   │   └── types.rs          # Event type definitions
│   ├── metrics/             # Core metrics
│   │   ├── mod.rs
│   │   ├── registry.rs       # Metrics registry
│   │   ├── collectors.rs     # Metric collectors
│   │   └── exporters.rs      # Metric exporters
│   ├── security/            # Core security
│   │   ├── mod.rs
│   │   ├── auth.rs           # Authentication traits
│   │   ├── crypto.rs         # Cryptographic utilities
│   │   └── permissions.rs    # Permission system
│   └── plugins/             # Plugin system
│       ├── mod.rs
│       ├── loader.rs         # Plugin loader
│       ├── registry.rs       # Plugin registry
│       └── api.rs            # Plugin API definitions
├── tests/
│   ├── integration/
│   └── unit/
└── examples/
    ├── basic_usage.rs
    └── dependency_injection.rs
```

### Key Design Principles

1. **Zero-Cost Abstractions**: Compile-time optimizations where possible
2. **Type Safety**: Leverage Rust's type system for correctness
3. **Async-First**: Built for async/await patterns throughout
4. **Observability**: Comprehensive logging and tracing support
5. **Testability**: Easy mocking and testing infrastructure

## APIs & Interfaces

### Core Types

```rust
// Typed IDs for type safety
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgentId(Uuid);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkflowId(Uuid);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ToolId(Uuid);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UserId(Uuid);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(Uuid);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProjectId(Uuid);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkspaceId(Uuid);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ContainerId(Uuid);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TradeId(Uuid);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ModelId(Uuid);

// Result types
pub type SymbioteResult<T> = Result<T, SymbioteError>;
pub type AsyncResult<T> = Pin<Box<dyn Future<Output = SymbioteResult<T>> + Send>>;

// Configuration trait
pub trait Configuration: Send + Sync + 'static {
    fn validate(&self) -> SymbioteResult<()>;
    fn merge(&mut self, other: Self) -> SymbioteResult<()>;
}

// Service trait for dependency injection
pub trait Service: Send + Sync + 'static {
    fn name(&self) -> &'static str;
    fn dependencies(&self) -> Vec<&'static str>;
    async fn initialize(&mut self) -> SymbioteResult<()>;
    async fn shutdown(&mut self) -> SymbioteResult<()>;
}
```

### Error Handling

```rust
#[derive(Debug, thiserror::Error)]
pub enum SymbioteError {
    #[error("Configuration error: {message}")]
    Configuration { message: String },
    
    #[error("Validation error: {field}: {message}")]
    Validation { field: String, message: String },
    
    #[error("Service error: {service}: {message}")]
    Service { service: String, message: String },
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
    
    #[error("Timeout error: operation timed out after {duration:?}")]
    Timeout { duration: Duration },
    
    #[error("Cancelled: {reason}")]
    Cancelled { reason: String },
    
    #[error("Internal error: {message}")]
    Internal { message: String },
}

impl SymbioteError {
    pub fn context<T: Display>(self, context: T) -> Self {
        // Add context to error chain
    }
    
    pub fn is_retryable(&self) -> bool {
        // Determine if error is retryable
    }
    
    pub fn error_code(&self) -> &'static str {
        // Return error code for API responses
    }
}
```

### Dependency Injection

```rust
pub struct ServiceContainer {
    services: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
    singletons: HashMap<TypeId, Arc<dyn Any + Send + Sync>>,
    factories: HashMap<TypeId, Box<dyn Fn() -> Box<dyn Any + Send + Sync> + Send + Sync>>,
}

impl ServiceContainer {
    pub fn new() -> Self;
    
    pub fn register<T: Service>(&mut self, service: T) -> SymbioteResult<()>;
    
    pub fn register_singleton<T: Service>(&mut self, service: T) -> SymbioteResult<()>;
    
    pub fn register_factory<T: Service, F>(&mut self, factory: F) -> SymbioteResult<()>
    where
        F: Fn() -> T + Send + Sync + 'static;
    
    pub fn resolve<T: Service>(&self) -> SymbioteResult<Arc<T>>;
    
    pub async fn initialize_all(&mut self) -> SymbioteResult<()>;
    
    pub async fn shutdown_all(&mut self) -> SymbioteResult<()>;
}
```

## Implementation Details

### Technology Stack

- **Rust Edition**: 2021
- **Async Runtime**: Tokio 1.x with full features
- **Serialization**: Serde with JSON, TOML, YAML support
- **Error Handling**: thiserror for error definitions
- **Logging**: tracing with structured logging
- **UUID Generation**: uuid crate with v4 and v7 support
- **Time Handling**: chrono with timezone support
- **Validation**: validator crate with custom rules

### Key Dependencies

```toml
[dependencies]
tokio = { version = "1.0", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
thiserror = "1.0"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
uuid = { version = "1.0", features = ["v4", "v7", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
validator = { version = "0.16", features = ["derive"] }
anyhow = "1.0"
once_cell = "1.0"
parking_lot = "0.12"
dashmap = "5.0"
```

### Performance Considerations

1. **Memory Management**: Use Arc/Rc for shared data, avoid unnecessary clones
2. **Async Efficiency**: Minimize context switches, use appropriate buffer sizes
3. **Error Handling**: Zero-cost error propagation with Result types
4. **Logging**: Conditional compilation for debug logs
5. **Serialization**: Efficient binary formats for internal communication

## Testing Strategy

### Unit Tests

- **Type Safety**: Test typed ID generation and validation
- **Error Handling**: Test error creation, chaining, and context
- **Dependency Injection**: Test service registration and resolution
- **Validation**: Test input validation and sanitization
- **Async Utilities**: Test timeout, cancellation, and channel operations

### Integration Tests

- **Service Lifecycle**: Test complete service initialization and shutdown
- **Configuration Loading**: Test configuration from various sources
- **Error Propagation**: Test error handling across module boundaries
- **Logging Integration**: Test structured logging output

### Benchmarks

- **Service Resolution**: Benchmark DI container performance
- **Error Creation**: Benchmark error allocation and formatting
- **Serialization**: Benchmark common type serialization
- **Async Operations**: Benchmark async utility performance

## Integration Points

### Upstream Dependencies

- **Tokio Runtime**: Shared async runtime across all crates
- **Tracing Subscriber**: Global logging configuration
- **Service Container**: Shared DI container instance

### Downstream Consumers

- **All Symbiote Crates**: Use core types, errors, and utilities
- **Storage Crate**: Uses configuration and error types
- **AI Crate**: Uses async utilities and service traits
- **Agent Framework**: Uses DI container and core types
- **UI Applications**: Uses error types for user feedback

### External Integrations

- **Operating System**: File system, environment variables
- **Network Stack**: HTTP clients, WebSocket connections
- **Database Systems**: Connection pooling, transaction management
- **Monitoring Systems**: Metrics collection, distributed tracing

## Acceptance Criteria

### Functional Requirements

- [ ] Typed ID system with UUID v4/v7 support
- [ ] Comprehensive error handling with context and chaining
- [ ] Dependency injection container with lifecycle management
- [ ] Configuration loading from files, environment, and CLI
- [ ] Structured logging with filtering and formatting
- [ ] Input validation with custom rules
- [ ] Async utilities for timeout and cancellation

### Non-Functional Requirements

- [ ] Zero-cost abstractions where possible
- [ ] Memory-safe with no unsafe code
- [ ] Thread-safe for concurrent access
- [ ] Comprehensive documentation with examples
- [ ] 95%+ test coverage
- [ ] Sub-millisecond service resolution
- [ ] Graceful error handling without panics

### Quality Gates

- [ ] All tests pass on CI/CD pipeline
- [ ] No clippy warnings or errors
- [ ] Documentation builds successfully
- [ ] Benchmarks meet performance targets
- [ ] Security audit passes
- [ ] Memory leak detection passes

## Dependencies & Prerequisites

### Build Dependencies

- **Rust Toolchain**: 1.70+ with async/await support
- **Cargo**: Latest stable version
- **Development Tools**: clippy, rustfmt, cargo-audit

### Runtime Dependencies

- **Operating System**: Windows 10+, macOS 10.15+, Linux (Ubuntu 20.04+)
- **Memory**: Minimum 100MB heap allocation
- **CPU**: Single-core sufficient, multi-core recommended

### Development Prerequisites

- **IDE Support**: rust-analyzer for LSP integration
- **Testing Framework**: Built-in Rust testing with tokio-test
- **Documentation**: rustdoc with custom CSS themes
- **Continuous Integration**: GitHub Actions with cross-platform testing

## UI Specifications

### Core System Dashboard

```
┌─────────────────────────────────────────────────────────────────────────────┐
│ ⚙️ Symbiote Core System Monitor                       [🔄] [⚙️] [📊] [🔒] [🚨] │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│ 🏗️ System Overview                                                          │
│ ┌─────────────────────────────────────────────────────────────────────────┐ │
│ │ Runtime: Tokio 1.35.1        │ Services: 23 active                      │ │
│ │ Uptime: 2d 14h 32m           │ Memory: 245MB / 1GB                      │ │
│ │ CPU: 12.3%                   │ Threads: 47 active                       │ │
│ │ Status: 🟢 Healthy           │ Errors: 0 in last hour                   │ │
│ └─────────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
│ 🔧 Service Container                                                        │
│ ┌─────────────────────────────────────────────────────────────────────────┐ │
│ │ Service                │ Status    │ Dependencies │ Health │ Actions     │ │
│ │ StorageManager         │ 🟢 Active │ 0           │ 100%   │ [🔄][⏹️][📊] │ │
│ │ SecurityManager        │ 🟢 Active │ 1           │ 98%    │ [🔄][⏹️][📊] │ │
│ │ TelemetryManager       │ 🟢 Active │ 2           │ 95%    │ [🔄][⏹️][📊] │ │
│ │ VaultManager           │ 🟢 Active │ 1           │ 100%   │ [🔄][⏹️][📊] │ │
│ │ AIClient               │ 🟡 Warn   │ 3           │ 87%    │ [🔄][⏹️][📊] │ │
│ │ [➕ Register Service] [🔄 Restart All] [📊 Dependencies Graph]           │ │
│ └─────────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
│ 🚨 System Events                                                            │
│ ┌─────────────────────────────────────────────────────────────────────────┐ │
│ │ Time     │ Event Type        │ Source          │ Status │ Details        │ │
│ │ 14:32:15 │ ServiceStarted    │ StorageManager  │ ✅     │ Initialized    │ │
│ │ 14:32:14 │ ConfigLoaded      │ Core            │ ✅     │ All modules    │ │
│ │ 14:32:12 │ HookRegistered    │ SecurityManager │ ✅     │ Auth hooks     │ │
│ │ 14:32:10 │ DependencyResolved│ VaultManager    │ ✅     │ Security deps  │ │
│ │ [📋 View All Events] [🔍 Filter] [📤 Export] [🔔 Alerts]                │ │
│ └─────────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
│ 🔧 Quick Actions                                                            │
│ │ [🔄 Restart Core] [📊 Performance Report] [🧹 Cleanup] [⚙️ Configuration] │ │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Error Monitoring Dashboard

```
┌─────────────────────────────────────────────────────────────────────────────┐
│ 🚨 Error Monitoring & Diagnostics                     [🔄] [⚙️] [📊] [🔒] [🚨] │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│ 📊 Error Summary                                                            │
│ ┌─────────────────────────────────────────────────────────────────────────┐ │
│ │ Total Errors: 47          │ Critical: 2    │ High: 8    │ Medium: 37    │ │
│ │ Error Rate: 0.3%          │ Recovery: 94%  │ Retries: 156│ Timeouts: 12  │ │
│ │ Last 24h: ↓ 23% decrease │ MTTR: 2.3min   │ MTBF: 4.2h │ SLA: 99.7%    │ │
│ └─────────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
│ 🔍 Recent Errors                                                            │
│ ┌─────────────────────────────────────────────────────────────────────────┐ │
│ │ Time     │ Severity │ Component      │ Error Type        │ Status        │ │
│ │ 14:28:45 │ 🔴 Crit  │ SecurityMgr    │ AuthenticationFail│ 🔄 Retrying   │ │
│ │ 14:27:32 │ 🟡 Med   │ StorageManager │ ConnectionTimeout │ ✅ Resolved   │ │
│ │ 14:26:18 │ 🟠 High  │ VaultManager   │ EncryptionFailed  │ 🔄 Retrying   │ │
│ │ 14:25:04 │ 🟡 Med   │ TelemetryMgr   │ MetricExportFail  │ ✅ Resolved   │ │
│ │ [📋 View All] [🔍 Filter] [📊 Analytics] [🚨 Alerts]                     │ │
│ └─────────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
│ 🛠️ Error Details                                                            │
│ │ Error ID: ERR-2024-001234                                                 │ │
│ │ Component: SecurityManager                                                │ │
│ │ Type: AuthenticationFailed                                                │ │
│ │ Message: "Invalid JWT token signature"                                    │ │
│ │ Stack Trace: [📋 View Full] [📤 Export] [🔄 Retry] [🔧 Fix]               │ │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Database Schema

### Core System Persistence

```sql
-- Service registry and lifecycle management
CREATE TABLE core_services (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    service_name VARCHAR(255) NOT NULL UNIQUE,
    service_type VARCHAR(100) NOT NULL, -- 'singleton', 'transient', 'scoped'
    status VARCHAR(50) DEFAULT 'inactive', -- 'active', 'inactive', 'failed', 'starting', 'stopping'
    dependencies JSONB, -- Array of service names this service depends on
    configuration JSONB, -- Service-specific configuration
    health_score DECIMAL(5,2) DEFAULT 100.00,
    last_health_check TIMESTAMP,
    started_at TIMESTAMP,
    stopped_at TIMESTAMP,
    restart_count INTEGER DEFAULT 0,
    error_count INTEGER DEFAULT 0,
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW(),
    metadata JSONB
);

-- System events and audit trail
CREATE TABLE core_system_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_id VARCHAR(255) NOT NULL UNIQUE,
    event_type VARCHAR(100) NOT NULL, -- 'service_started', 'service_stopped', 'error_occurred', etc.
    source_service VARCHAR(255),
    event_data JSONB NOT NULL,
    severity VARCHAR(20), -- 'info', 'warning', 'error', 'critical'
    correlation_id VARCHAR(255), -- For tracing related events
    user_id VARCHAR(255),
    session_id VARCHAR(255),
    occurred_at TIMESTAMP DEFAULT NOW(),
    processed_at TIMESTAMP,
    metadata JSONB
);

-- Error tracking and analysis
CREATE TABLE core_errors (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    error_id VARCHAR(255) NOT NULL UNIQUE,
    error_type VARCHAR(100) NOT NULL,
    error_message TEXT NOT NULL,
    error_context JSONB, -- Additional context about the error
    stack_trace TEXT,
    source_service VARCHAR(255),
    source_function VARCHAR(255),
    severity VARCHAR(20), -- 'low', 'medium', 'high', 'critical'
    is_recoverable BOOLEAN DEFAULT false,
    recovery_action VARCHAR(255),
    retry_count INTEGER DEFAULT 0,
    max_retries INTEGER DEFAULT 3,
    first_occurred TIMESTAMP DEFAULT NOW(),
    last_occurred TIMESTAMP DEFAULT NOW(),
    resolved_at TIMESTAMP,
    resolution_notes TEXT,
    occurrence_count INTEGER DEFAULT 1,
    metadata JSONB
);

-- Configuration management
CREATE TABLE core_configurations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    config_key VARCHAR(500) NOT NULL UNIQUE,
    config_value JSONB NOT NULL,
    config_type VARCHAR(100), -- 'system', 'service', 'user', 'environment'
    is_encrypted BOOLEAN DEFAULT false,
    is_sensitive BOOLEAN DEFAULT false,
    validation_schema JSONB, -- JSON Schema for validation
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW(),
    created_by VARCHAR(255),
    updated_by VARCHAR(255),
    version INTEGER DEFAULT 1,
    metadata JSONB
);

-- Global hooks and event handlers
CREATE TABLE core_hooks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    hook_id VARCHAR(255) NOT NULL UNIQUE,
    hook_name VARCHAR(255) NOT NULL,
    event_types JSONB NOT NULL, -- Array of event types this hook handles
    handler_service VARCHAR(255) NOT NULL,
    handler_function VARCHAR(255) NOT NULL,
    priority INTEGER DEFAULT 100, -- Lower numbers = higher priority
    is_active BOOLEAN DEFAULT true,
    execution_timeout_ms INTEGER DEFAULT 5000,
    retry_policy JSONB, -- Retry configuration
    filter_conditions JSONB, -- Conditions for when hook should execute
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW(),
    last_executed TIMESTAMP,
    execution_count INTEGER DEFAULT 0,
    success_count INTEGER DEFAULT 0,
    failure_count INTEGER DEFAULT 0,
    metadata JSONB
);

-- Dependency injection container state
CREATE TABLE core_dependencies (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    service_name VARCHAR(255) NOT NULL,
    dependency_name VARCHAR(255) NOT NULL,
    dependency_type VARCHAR(100), -- 'required', 'optional', 'circular'
    resolution_order INTEGER,
    is_resolved BOOLEAN DEFAULT false,
    resolved_at TIMESTAMP,
    resolution_time_ms INTEGER,
    created_at TIMESTAMP DEFAULT NOW(),
    metadata JSONB,
    UNIQUE(service_name, dependency_name)
);

-- Performance metrics and monitoring
CREATE TABLE core_metrics (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    metric_name VARCHAR(255) NOT NULL,
    metric_type VARCHAR(100), -- 'counter', 'gauge', 'histogram', 'timer'
    metric_value DECIMAL(20,6) NOT NULL,
    labels JSONB, -- Key-value pairs for metric labels
    source_service VARCHAR(255),
    recorded_at TIMESTAMP DEFAULT NOW(),
    metadata JSONB
);

-- Indexes for performance
CREATE INDEX idx_core_services_status ON core_services(status);
CREATE INDEX idx_core_services_name ON core_services(service_name);
CREATE INDEX idx_core_system_events_type ON core_system_events(event_type);
CREATE INDEX idx_core_system_events_source ON core_system_events(source_service);
CREATE INDEX idx_core_system_events_occurred_at ON core_system_events(occurred_at);
CREATE INDEX idx_core_errors_type ON core_errors(error_type);
CREATE INDEX idx_core_errors_severity ON core_errors(severity);
CREATE INDEX idx_core_errors_source ON core_errors(source_service);
CREATE INDEX idx_core_errors_occurred ON core_errors(first_occurred);
CREATE INDEX idx_core_configurations_key ON core_configurations(config_key);
CREATE INDEX idx_core_configurations_type ON core_configurations(config_type);
CREATE INDEX idx_core_hooks_event_types ON core_hooks USING GIN(event_types);
CREATE INDEX idx_core_hooks_active ON core_hooks(is_active);
CREATE INDEX idx_core_dependencies_service ON core_dependencies(service_name);
CREATE INDEX idx_core_dependencies_resolved ON core_dependencies(is_resolved);
CREATE INDEX idx_core_metrics_name ON core_metrics(metric_name);
CREATE INDEX idx_core_metrics_recorded_at ON core_metrics(recorded_at);
```

## Security Model

### Core Security Framework

```rust
pub struct CoreSecurityManager {
    access_control: CoreAccessControl,
    service_security: ServiceSecurityManager,
    audit_logger: CoreAuditLogger,
    crypto_manager: CoreCryptographyManager,
}

impl CoreSecurityManager {
    /// Validate core system access permissions
    pub async fn validate_core_access(&self, user_id: &str, operation: CoreOperation) -> SymbioteResult<AccessDecision>;

    /// Secure service registration and lifecycle
    pub async fn secure_service_registration(&self, service: &dyn Service) -> SymbioteResult<SecureService>;

    /// Validate dependency injection security
    pub async fn validate_dependency_security(&self, service_name: &str, dependency: &str) -> SymbioteResult<DependencySecurityResult>;

    /// Enforce security policies on core operations
    pub async fn enforce_security_policies(&self, operation: &CoreOperation, policies: &[SecurityPolicy]) -> SymbioteResult<PolicyEnforcement>;

    /// Log core system operations for audit and compliance
    pub async fn log_core_operation(&self, operation: &CoreOperation, user_id: &str, result: &OperationResult) -> SymbioteResult<()>;

    /// Handle sensitive data in core configurations
    pub async fn handle_sensitive_core_data(&self, config_data: &ConfigurationData) -> SymbioteResult<SanitizedConfiguration>;

    /// Manage core system encryption and key rotation
    pub async fn manage_core_encryption(&self, data_type: &str, encryption_policy: &EncryptionPolicy) -> SymbioteResult<EncryptionResult>;

    /// Validate service communication security
    pub async fn validate_service_communication(&self, from_service: &str, to_service: &str, message: &ServiceMessage) -> SymbioteResult<CommunicationSecurityResult>;

    /// Monitor core system for security violations
    pub async fn monitor_security_violations(&self, system_events: &[SystemEvent]) -> SymbioteResult<Vec<SecurityViolation>>;
}

#[derive(Debug, Clone)]
pub enum CoreOperation {
    ServiceRegistration { service_name: String, service_type: String },
    ServiceInitialization { service_name: String, dependencies: Vec<String> },
    ConfigurationAccess { config_key: String, operation_type: String },
    HookRegistration { hook_name: String, event_types: Vec<String> },
    EventEmission { event_type: String, source_service: String },
    DependencyResolution { service_name: String, dependency_name: String },
    ErrorHandling { error_type: String, source_service: String },
    MetricsCollection { metric_name: String, source_service: String },
}

#[derive(Debug, Clone)]
pub struct ServiceSecurityProfile {
    pub service_name: String,
    pub permission_level: PermissionLevel,
    pub allowed_dependencies: Vec<String>,
    pub blocked_dependencies: Vec<String>,
    pub sandbox_enabled: bool,
    pub network_access: NetworkAccess,
    pub file_system_access: FileSystemAccess,
    pub encryption_required: bool,
    pub audit_level: AuditLevel,
}

#[derive(Debug, Clone)]
pub enum PermissionLevel {
    Minimal,      // Basic core operations only
    Standard,     // Standard service operations
    Elevated,     // Extended system access
    Administrative, // Full core system access
    Custom { permissions: Vec<CorePermission> },
}
```

This foundation crate establishes the architectural patterns and quality standards that all other Symbiote components will follow, ensuring consistency, reliability, and maintainability across the entire ecosystem.

## Global Hooks & Event System (SYSTEM-WIDE ARCHITECTURE)

### SymbioteCore with Global Hooks

```rust
/// Central Symbiote core with global hooks system
pub struct SymbioteCore {
    // Global Hooks & Event System (SYSTEM-WIDE)
    hooks_runtime: GlobalHooksRuntime,
    event_bus: SystemEventBus,
    slash_commands: SlashCommandsFramework,

    // Core subsystems (all integrate with hooks)
    vault: VaultManager,
    security: SecurityManager,
    telemetry: TelemetryManager,

    // All major subsystems register with hooks
    ide_engine: Option<IdeEngine>,
    trader_engine: Option<TraderEngine>,
    ai_provider_manager: Option<AIProviderManager>,
    workflow_engine: Option<WorkflowEngine>,
    assistant_hub: Option<AssistantHub>,
    browser_automation: Option<BrowserAutomationSystem>,
    container_manager: Option<ContainerManagementSystem>,

    // Configuration
    config: CoreConfig,

    // Runtime state
    runtime: RuntimeManager,
}

impl SymbioteCore {
    pub async fn new() -> SymbioteResult<Self>;

    /// Register subsystem with global hooks
    pub async fn register_subsystem<T: HookEmitter>(&mut self, subsystem: T) -> SymbioteResult<()>;

    /// Emit system-wide event
    pub async fn emit_event(&self, event: SystemEvent) -> SymbioteResult<()>;

    /// Register global hook
    pub async fn register_hook(&self, hook: GlobalHook) -> SymbioteResult<HookId>;

    /// Execute slash command
    pub async fn execute_slash_command(&self, command: &str, context: &CommandContext) -> SymbioteResult<CommandResult>;
}

/// Global hooks runtime for system-wide event handling
pub struct GlobalHooksRuntime {
    // Hook management
    hook_registry: HookRegistry,
    hook_executor: HookExecutor,
    hook_scheduler: HookScheduler,

    // Isolation and security
    worker_pool: WorkerPool,
    timeout_manager: TimeoutManager,
    retry_manager: RetryManager,
    idempotency_tracker: IdempotencyTracker,

    // Security integration
    permission_checker: PermissionChecker,
    egress_proxy: EgressProxy,
    secret_handler: SecretHandler,
}

/// Comprehensive system events across ALL subsystems
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SystemEvent {
    // IDE Events
    FileOpened { path: PathBuf, workspace_id: WorkspaceId, user_id: UserId },
    FileChanged { path: PathBuf, change_type: ChangeType, diff: String },
    CodeCompiled { project_id: ProjectId, success: bool, errors: Vec<String> },
    TestRun { test_id: String, result: TestResult, coverage: f32 },
    BuildStarted { build_id: String, config: BuildConfig },
    BuildFinished { build_id: String, success: bool, duration_ms: u64 },
    RefactorStarted { refactor_id: String, plan: RefactorPlan },
    RefactorCompleted { refactor_id: String, result: RefactorResult },

    // Trading Events
    OrderPlaced { order: Order, strategy_id: Option<String> },
    OrderFilled { order_id: String, fill_price: f64, quantity: f64 },
    OrderCancelled { order_id: String, reason: String },
    PositionOpened { position: Position, entry_price: f64 },
    PositionClosed { position_id: String, exit_price: f64, pnl: f64 },
    RiskLimitHit { risk_type: RiskType, current_value: f64, limit: f64 },
    StrategyStarted { strategy_id: String, parameters: StrategyParams },
    StrategyPaused { strategy_id: String, reason: String },

    // AI Events
    ModelCalled { provider: String, model: String, request: ModelRequest },
    ModelResponse { provider: String, model: String, response: ModelResponse, cost: f64, tokens: u32 },
    TokensConsumed { workspace_id: WorkspaceId, provider: String, tokens: u32, cost: f64 },
    BudgetExceeded { workspace_id: WorkspaceId, budget_type: BudgetType, current: f64, limit: f64 },
    ProviderFailover { from_provider: String, to_provider: String, reason: String },

    // Workflow Events
    WorkflowStarted { workflow_id: WorkflowId, trigger: WorkflowTrigger },
    WorkflowCompleted { workflow_id: WorkflowId, success: bool, duration_ms: u64 },
    NodeExecuted { node_id: String, workflow_id: WorkflowId, result: NodeResult },
    NodeFailed { node_id: String, workflow_id: WorkflowId, error: String },

    // Security Events
    PermissionRequested { user_id: UserId, resource: String, action: String },
    PermissionGranted { user_id: UserId, resource: String, action: String },
    PermissionDenied { user_id: UserId, resource: String, action: String, reason: String },
    VaultAccessed { user_id: UserId, secret_id: String, operation: VaultOperation },
    SecurityViolation { user_id: UserId, violation_type: SecurityViolationType, details: String },

    // Browser Automation Events
    BrowserSessionStarted { session_id: String, url: String },
    BrowserActionExecuted { session_id: String, action: BrowserAction, success: bool },
    FormFilled { session_id: String, form_data: FormData },
    PageNavigated { session_id: String, from_url: String, to_url: String },

    // Container Events
    ContainerStarted { container_id: ContainerId, image: String, config: ContainerConfig },
    ContainerStopped { container_id: ContainerId, exit_code: i32 },
    DeploymentStarted { deployment_id: String, environment: String },
    DeploymentCompleted { deployment_id: String, success: bool },

    // Assistant Events
    ThreadCreated { thread_id: String, user_id: UserId, workspace_id: WorkspaceId },
    RunStarted { run_id: String, thread_id: String, agent_id: AgentId },
    RunFinished { run_id: String, success: bool, cost: f64, duration_ms: u64 },
    ApprovalRequested { approval_id: String, action: String, context: ApprovalContext },
    MemoryAdded { memory_id: String, content: String, tags: Vec<String> },
    ContextPackUpdated { pack_id: String, size_tokens: u32, freshness_ts: i64 },

    // System Events
    SystemStarted { version: String, config: SystemConfig },
    SystemShutdown { reason: String },
    HealthCheckFailed { component: String, error: String },
    ConfigurationChanged { component: String, changes: ConfigChanges },

    // Custom Events (extensible)
    Custom { event_type: String, data: serde_json::Value, metadata: EventMetadata },
}

/// Trait for subsystems to emit events
pub trait HookEmitter {
    fn emit_event(&self, event: SystemEvent) -> SymbioteResult<()>;
    fn register_event_types(&self) -> Vec<SystemEventType>;
    fn get_subsystem_id(&self) -> String;
}

/// Slash commands framework
pub struct SlashCommandsFramework {
    command_parser: CommandParser,
    command_router: CommandRouter,
    command_executor: CommandExecutor,
    permission_checker: PermissionChecker,
}

/// Built-in slash commands
#[derive(Debug, Clone)]
pub enum BuiltInCommand {
    Plan(String),                    // /plan "auth + db + API"
    Scaffold(ScaffoldConfig),        // /scaffold app react-native expo-auth
    Stack(StackCommand),             // /stack suggest mobile
    DevServer(DevServerCommand),     // /devserver status|restart
    Approve(ApprovalCommand),        // /approve pending
    Permission(PermissionCommand),   // /perm set hil
    Mode(ModeCommand),              // /mode set vibe
    Workflow(WorkflowCommand),      // /workflow run {name} --input
    Agent(AgentCommand),            // /agent run tool:{name} --args
}
```
