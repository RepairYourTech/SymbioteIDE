# Security - Permissions & Audit System Plan

## Goals & Vision

The `security` crate provides comprehensive security controls and audit capabilities for Symbiote. It offers:

- **Permission Profiles**: Configurable security profiles (ZeroTrust, HiL, Full Auto)
- **Policy Engine**: Fine-grained permission policies and enforcement
- **Audit System**: Comprehensive audit logging and compliance reporting
- **Sandboxing**: Secure execution environments for untrusted code
- **Privacy Controls**: Data privacy and anonymization features
- **Compliance**: GDPR, SOC2, and enterprise compliance frameworks
- **Threat Detection**: Real-time security monitoring and alerting

This crate ensures Symbiote operates securely across all environments and use cases.

## UI Design Specifications

### Security Dashboard Layout

#### Main Security Dashboard
```
┌─────────────────────────────────────────────────────────────────────────────┐
│ 🔒 Security Center                           [🔄] [⚙️] [📊] [🚨]           │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│ 🎯 Security Overview                                                        │
│ ┌─────────────┬─────────────┬─────────────┬─────────────┬─────────────┐     │
│ │ 🟢 Status   │ 🔐 Profile  │ 🚨 Alerts   │ 📋 Audits   │ 🛡️ Threats  │     │
│ │   Secure    │ Zero Trust  │      3      │    1,247    │      0      │     │
│ └─────────────┴─────────────┴─────────────┴─────────────┴─────────────┘     │
│                                                                             │
│ 🔐 Active Permission Profile                                                │
│ ┌─────────────────────────────────────────────────────────────────────────┐ │
│ │ Zero Trust Mode                                        [Change Profile] │ │
│ │ ├─ All actions require explicit approval                               │ │
│ │ ├─ Network access restricted                                           │ │
│ │ ├─ File system access sandboxed                                        │ │
│ │ └─ Real-time monitoring enabled                                        │ │
│ │                                                                         │ │
│ │ Recent Approvals:                                                       │ │
│ │ • 2 min ago: Deploy container to staging (Approved)                   │ │
│ │ • 5 min ago: Access production database (Pending)                     │ │
│ │ • 8 min ago: Install npm package (Approved)                           │ │
│ └─────────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
│ 🚨 Security Alerts                                                          │
│ ┌─────────────────────────────────────────────────────────────────────────┐ │
│ │ 🟡 Medium: Unusual API access pattern detected                [Investigate]│ │
│ │ 🟠 High: Failed authentication attempts (5x)                 [Block IP]  │ │
│ │ 🔴 Critical: Potential data exfiltration attempt             [Emergency] │ │
│ └─────────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
│ 📊 Security Metrics (Last 24h)                                             │
│ ┌─────────────────────────────────────────────────────────────────────────┐ │
│ │ Permission Requests: 1,247 (↑12%)    Approvals: 1,156 (93%)           │ │
│ │ Denied Requests: 91 (7%)              Audit Events: 15,432             │ │
│ │ Threat Detections: 23 (↓5%)          Incidents: 0                     │ │
│ └─────────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
│ 🔗 Quick Actions                                                            │
│ [🔐 Change Profile] [📋 View Audit Log] [🚨 Security Report] [⚙️ Policies] │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

#### Permission Profile Configuration
```
┌─────────────────────────────────────────────────────────────────────────────┐
│ 🔐 Permission Profile Configuration                                   [✕] │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│ Choose Security Profile:                                                    │
│                                                                             │
│ ┌─────────────────────────────────────────────────────────────────────────┐ │
│ │ ● Zero Trust (Recommended for Production)                              │ │
│ │   ├─ All actions require explicit approval                             │ │
│ │   ├─ Maximum security, minimal automation                              │ │
│ │   ├─ Real-time monitoring and alerting                                 │ │
│ │   └─ Compliance: SOC2, GDPR, HIPAA ready                              │ │
│ │                                                                         │ │
│ │ ○ Human-in-the-Loop (Balanced)                                         │ │
│ │   ├─ Critical actions require approval                                 │ │
│ │   ├─ Routine operations automated                                      │ │
│ │   ├─ Configurable approval thresholds                                  │ │
│ │   └─ Compliance: SOC2, GDPR compliant                                 │ │
│ │                                                                         │ │
│ │ ○ Full Automation (Development Only)                                   │ │
│ │   ├─ All actions automated                                             │ │
│ │   ├─ Minimal security restrictions                                     │ │
│ │   ├─ Audit logging only                                                │ │
│ │   └─ Compliance: Basic audit trail                                     │ │
│ │                                                                         │ │
│ │ ○ Custom Profile                                                       │ │
│ │   ├─ Define your own security rules                                    │ │
│ │   ├─ Granular permission control                                       │ │
│ │   ├─ Advanced policy configuration                                     │ │
│ │   └─ Compliance: Configurable                                          │ │
│ └─────────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
│ Profile Details:                                                            │
│ ┌─────────────────────────────────────────────────────────────────────────┐ │
│ │ Network Access: ● Restricted  ○ Monitored  ○ Open                     │ │
│ │ File System:    ● Sandboxed   ○ Monitored  ○ Full Access              │ │
│ │ API Calls:      ● Approved    ○ Logged     ○ Unrestricted             │ │
│ │ Data Access:    ● Encrypted   ○ Monitored  ○ Direct                   │ │
│ │ Code Execution: ● Sandboxed   ○ Monitored  ○ Native                   │ │
│ └─────────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
│                                    [Cancel] [Apply Profile]                │
└─────────────────────────────────────────────────────────────────────────────┘
```

#### Approval Workflow Interface
```
┌─────────────────────────────────────────────────────────────────────────────┐
│ 🔐 Pending Approvals                                    [🔄] [⚙️] [📋]    │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│ ⏰ Urgent Approvals (3)                                                     │
│ ┌─────────────────────────────────────────────────────────────────────────┐ │
│ │ 🔴 HIGH PRIORITY                                                        │ │
│ │ Request: Deploy hotfix to production                                    │ │
│ │ User: alice@company.com                                                 │ │
│ │ Time: 2 minutes ago                                                     │ │
│ │ Risk: High (Production deployment)                                      │ │
│ │ Details: Critical security patch for authentication                     │ │
│ │ [🔍 Review] [✅ Approve] [❌ Deny] [⏸️ Request Info]                   │ │
│ ├─────────────────────────────────────────────────────────────────────────┤ │
│ │ 🟡 MEDIUM PRIORITY                                                      │ │
│ │ Request: Access customer database                                       │ │
│ │ User: bob@company.com                                                   │ │
│ │ Time: 5 minutes ago                                                     │ │
│ │ Risk: Medium (Data access)                                              │ │
│ │ Details: Debug customer issue #12345                                    │ │
│ │ [🔍 Review] [✅ Approve] [❌ Deny] [⏸️ Request Info]                   │ │
│ └─────────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
│ 📋 Standard Approvals (7)                                                   │
│ ┌─────────────────────────────────────────────────────────────────────────┐ │
│ │ • Install npm package 'lodash@4.17.21' (carol@company.com)            │ │
│ │ • Create new API endpoint (dave@company.com)                           │ │
│ │ • Modify database schema (eve@company.com)                             │ │
│ │ • Deploy to staging environment (frank@company.com)                    │ │
│ │ • Access external API (grace@company.com)                              │ │
│ │ [View All] [Bulk Actions]                                              │ │
│ └─────────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
│ ⚙️ Approval Settings                                                        │
│ ┌─────────────────────────────────────────────────────────────────────────┐ │
│ │ Auto-approve: ☑️ Package installs from trusted sources                 │ │
│ │              ☑️ Staging deployments                                    │ │
│ │              ☐ Database read operations                                │ │
│ │                                                                         │ │
│ │ Require 2FA: ☑️ Production deployments                                 │ │
│ │             ☑️ Data exports                                            │ │
│ │             ☑️ User management                                         │ │
│ │                                                                         │ │
│ │ Notification: 📧 Email + 📱 Slack + 🔔 Desktop                        │ │
│ └─────────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

#### Audit Log Viewer
```
┌─────────────────────────────────────────────────────────────────────────────┐
│ 📋 Security Audit Log                        [🔍] [📥] [📊] [⚙️]          │
├─────────────────────────────────────────────────────────────────────────────┤
│ Filters: [🟢 All] [🔐 Auth] [📁 File] [🌐 Network] [🔧 Admin]             │
│ Time: [Last 24h ▼] User: [All users ▼] Risk: [All levels ▼]               │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│ 2024-01-15 14:23:45 [🔴 HIGH] alice@company.com                           │
│ Action: Production deployment initiated                                     │
│ Resource: /api/v1/deploy/production                                         │
│ Result: ✅ Approved (2FA verified)                                         │
│ Details: Deployed version 1.2.3 with security patches                     │
│ [🔍 Details] [📋 Related Events]                                          │
│                                                                             │
│ 2024-01-15 14:22:15 [🟡 MEDIUM] bob@company.com                           │
│ Action: Database query executed                                             │
│ Resource: customers.personal_data                                           │
│ Result: ✅ Approved (Manager approval)                                     │
│ Details: SELECT * FROM customers WHERE id = 12345                         │
│ [🔍 Details] [📋 Related Events]                                          │
│                                                                             │
│ 2024-01-15 14:21:30 [🟢 LOW] carol@company.com                            │
│ Action: Package installation                                                │
│ Resource: npm:lodash@4.17.21                                               │
│ Result: ✅ Auto-approved (Trusted source)                                 │
│ Details: Installed via npm install lodash                                  │
│ [🔍 Details] [📋 Related Events]                                          │
│                                                                             │
│ 2024-01-15 14:20:45 [🔴 HIGH] system                                      │
│ Action: Failed authentication attempt                                       │
│ Resource: /api/auth/login                                                   │
│ Result: ❌ Blocked (Rate limit exceeded)                                   │
│ Details: 5 failed attempts from IP 192.168.1.100                         │
│ [🔍 Details] [🚨 Create Incident]                                         │
│                                                                             │
│ 2024-01-15 14:19:12 [🟡 MEDIUM] dave@company.com                          │
│ Action: File system access                                                  │
│ Resource: /etc/passwd                                                       │
│ Result: ❌ Denied (Insufficient permissions)                               │
│ Details: Attempted to read system password file                           │
│ [🔍 Details] [⚠️ Security Review]                                         │
│                                                                             │
│ ● Live monitoring... (127 events/min)                                      │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Architecture & Design

### Core Modules

```
security/
├── src/
│   ├── lib.rs                 # Public API exports
│   ├── profiles/              # Permission profiles
│   │   ├── mod.rs
│   │   ├── zero_trust.rs     # Zero Trust profile
│   │   ├── human_in_loop.rs  # Human-in-the-Loop profile
│   │   ├── full_auto.rs      # Full Automation profile
│   │   └── custom.rs         # Custom profile builder
│   ├── policies/             # Policy engine
│   │   ├── mod.rs
│   │   ├── engine.rs         # Policy evaluation engine
│   │   ├── rules.rs          # Policy rule definitions
│   │   ├── conditions.rs     # Conditional logic
│   │   └── actions.rs        # Policy actions
│   ├── audit/                # Audit system
│   │   ├── mod.rs
│   │   ├── logger.rs         # Audit event logging
│   │   ├── storage.rs        # Audit log storage
│   │   ├── analysis.rs       # Log analysis and reporting
│   │   └── compliance.rs     # Compliance reporting
│   ├── sandbox/              # Sandboxing and isolation
│   │   ├── mod.rs
│   │   ├── process.rs        # Process sandboxing
│   │   ├── network.rs        # Network isolation
│   │   ├── filesystem.rs     # File system restrictions
│   │   └── resources.rs      # Resource limits
│   ├── privacy/              # Privacy controls
│   │   ├── mod.rs
│   │   ├── anonymization.rs  # Data anonymization
│   │   ├── encryption.rs     # Privacy-preserving encryption
│   │   ├── retention.rs      # Data retention policies
│   │   └── consent.rs        # Consent management
│   ├── ai_security/          # AI-specific security (NEW)
│   │   ├── mod.rs
│   │   ├── firewall.rs       # LLM firewall and threat detection
│   │   ├── prompt_injection.rs # Prompt injection detection
│   │   ├── output_sanitizer.rs # Output sanitization and filtering
│   │   ├── jailbreak_detector.rs # Jailbreaking prevention
│   │   ├── context_validator.rs # Context integrity validation
│   │   ├── content_filter.rs # Content filtering and safety
│   │   ├── threat_classifier.rs # AI threat classification
│   │   └── behavioral_monitor.rs # AI behavior monitoring
│   ├── monitoring/           # Security monitoring
│   │   ├── mod.rs
│   │   ├── detector.rs       # Threat detection
│   │   ├── alerts.rs         # Security alerting
│   │   ├── metrics.rs        # Security metrics
│   │   └── incidents.rs      # Incident management
│   └── types/                # Security types
│       ├── mod.rs
│       ├── permissions.rs    # Permission types
│       ├── events.rs         # Audit event types
│       ├── threats.rs        # Threat model types
│       └── ai_threats.rs     # AI-specific threat types (NEW)
├── tests/
│   ├── integration/
│   └── unit/
└── examples/
    ├── permission_profiles.rs
    └── audit_logging.rs
```

### Key Design Principles

1. **Principle of Least Privilege**: Minimal permissions by default
2. **Defense in Depth**: Multiple security layers
3. **Fail Secure**: Secure defaults and safe failure modes
4. **Transparency**: Comprehensive audit trails
5. **Flexibility**: Configurable security policies

## Error Handling

### Security Error Types

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SecurityError {
    #[error("Permission denied: {reason}")]
    PermissionDenied { reason: String },

    #[error("Authentication failed: {details}")]
    AuthenticationFailed { details: String },

    #[error("Authorization failed for user {user_id}: {action}")]
    AuthorizationFailed { user_id: String, action: String },

    #[error("Security policy violation: {policy_name} - {violation}")]
    PolicyViolation { policy_name: String, violation: String },

    #[error("Threat detected: {threat_type} - {description}")]
    ThreatDetected { threat_type: String, description: String },

    #[error("Security incident creation failed: {reason}")]
    IncidentCreationFailed { reason: String },

    #[error("Audit logging failed: {error}")]
    AuditLogFailed { error: String },

    #[error("Encryption operation failed: {operation}")]
    EncryptionFailed { operation: String },

    #[error("Decryption operation failed: {operation}")]
    DecryptionFailed { operation: String },

    #[error("Key management error: {operation} - {details}")]
    KeyManagementError { operation: String, details: String },

    #[error("Sandbox execution failed: {reason}")]
    SandboxExecutionFailed { reason: String },

    #[error("Vulnerability scan failed: {scan_type} - {error}")]
    VulnerabilityScanFailed { scan_type: String, error: String },

    #[error("Compliance check failed: {framework} - {requirement}")]
    ComplianceCheckFailed { framework: String, requirement: String },

    #[error("MFA verification failed: {method} - {reason}")]
    MFAVerificationFailed { method: String, reason: String },

    #[error("Role assignment failed: {user_id} -> {role} - {reason}")]
    RoleAssignmentFailed { user_id: String, role: String, reason: String },

    #[error("Network security violation: {rule} - {details}")]
    NetworkSecurityViolation { rule: String, details: String },

    #[error("Container security violation: {container_id} - {violation}")]
    ContainerSecurityViolation { container_id: String, violation: String },

    #[error("Security configuration error: {component} - {error}")]
    ConfigurationError { component: String, error: String },

    #[error("Database error: {operation} - {error}")]
    DatabaseError { operation: String, error: String },

    #[error("IO error: {operation} - {error}")]
    IoError { operation: String, error: String },

    #[error("Serialization error: {data_type} - {error}")]
    SerializationError { data_type: String, error: String },

    #[error("Timeout error: {operation} timed out after {duration_ms}ms")]
    TimeoutError { operation: String, duration_ms: u64 },

    #[error("Resource limit exceeded: {resource} - {limit}")]
    ResourceLimitExceeded { resource: String, limit: String },

    #[error("Invalid input: {field} - {reason}")]
    InvalidInput { field: String, reason: String },

    #[error("Security service unavailable: {service}")]
    ServiceUnavailable { service: String },

    #[error("Internal security error: {details}")]
    InternalError { details: String },
}

pub type SecurityResult<T> = Result<T, SecurityError>;

impl From<sqlx::Error> for SecurityError {
    fn from(err: sqlx::Error) -> Self {
        SecurityError::DatabaseError {
            operation: "database_operation".to_string(),
            error: err.to_string(),
        }
    }
}

impl From<std::io::Error> for SecurityError {
    fn from(err: std::io::Error) -> Self {
        SecurityError::IoError {
            operation: "io_operation".to_string(),
            error: err.to_string(),
        }
    }
}

impl From<serde_json::Error> for SecurityError {
    fn from(err: serde_json::Error) -> Self {
        SecurityError::SerializationError {
            data_type: "json".to_string(),
            error: err.to_string(),
        }
    }
}
```

## APIs & Interfaces

### Permission Profiles

```rust
pub trait PermissionProfile: Send + Sync + Clone {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    
    fn evaluate_permission(&self, request: &PermissionRequest) -> PermissionDecision;
    fn requires_approval(&self, action: &Action) -> bool;
    fn approval_workflow(&self, action: &Action) -> Option<ApprovalWorkflow>;
    
    fn allowed_operations(&self) -> Vec<Operation>;
    fn restricted_resources(&self) -> Vec<ResourcePattern>;
    fn time_restrictions(&self) -> Option<TimeRestrictions>;
}

#[derive(Debug, Clone)]
pub struct ZeroTrustProfile {
    config: ZeroTrustConfig,
}

impl PermissionProfile for ZeroTrustProfile {
    fn name(&self) -> &'static str { "ZeroTrust" }
    
    fn evaluate_permission(&self, request: &PermissionRequest) -> PermissionDecision {
        // Always require explicit approval
        PermissionDecision::RequiresApproval {
            reason: "Zero Trust policy requires approval for all actions".to_string(),
            approvers: self.config.required_approvers.clone(),
            timeout: self.config.approval_timeout,
        }
    }
}

#[derive(Debug, Clone)]
pub struct HumanInLoopProfile {
    config: HiLConfig,
    risk_assessor: RiskAssessor,
}

impl PermissionProfile for HumanInLoopProfile {
    fn evaluate_permission(&self, request: &PermissionRequest) -> PermissionDecision {
        let risk_level = self.risk_assessor.assess_risk(request);
        
        match risk_level {
            RiskLevel::Low => PermissionDecision::Allow,
            RiskLevel::Medium => PermissionDecision::RequiresApproval {
                reason: format!("Medium risk action requires approval: {}", risk_level.reason()),
                approvers: vec![ApproverType::User],
                timeout: Duration::from_secs(300), // 5 minutes
            },
            RiskLevel::High => PermissionDecision::RequiresApproval {
                reason: format!("High risk action requires approval: {}", risk_level.reason()),
                approvers: vec![ApproverType::User, ApproverType::Admin],
                timeout: Duration::from_secs(3600), // 1 hour
            },
            RiskLevel::Critical => PermissionDecision::Deny {
                reason: "Critical risk actions are not permitted".to_string(),
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct FullAutoProfile {
    config: FullAutoConfig,
    safety_checks: Vec<SafetyCheck>,
}

impl PermissionProfile for FullAutoProfile {
    fn evaluate_permission(&self, request: &PermissionRequest) -> PermissionDecision {
        // Run safety checks
        for check in &self.safety_checks {
            if let Err(reason) = check.validate(request) {
                return PermissionDecision::Deny { reason };
            }
        }
        
        // Allow if all safety checks pass
        PermissionDecision::Allow
    }
}

pub struct SecurityManager {
    permission_engine: PermissionEngine,
    audit_logger: AuditLogger,
    threat_detector: ThreatDetector,
    policy_engine: PolicyEngine,
    incident_manager: IncidentManager,
    compliance_manager: ComplianceManager,
    alert_manager: AlertManager,
    config: SecurityConfig,

    // Unified Security Architecture (from master plan)
    encryption_manager: EncryptionManager,
    vulnerability_scanner: VulnerabilityScanner,
    penetration_tester: PenetrationTester,
    wasm_sandbox: WasmSandbox,

    // AI Security Components (NEW)
    ai_security_firewall: AISecurityFirewall,
    prompt_injection_detector: PromptInjectionDetector,
    output_sanitizer: OutputSanitizer,
    jailbreak_detector: JailbreakDetector,
}

impl SecurityManager {
    pub async fn new(config: SecurityConfig) -> SecurityResult<Self>;

    pub async fn set_permission_profile(&mut self, profile: Box<dyn PermissionProfile>) -> SecurityResult<()>;

    pub fn get_current_profile(&self) -> &dyn PermissionProfile;

    pub async fn request_permission(&self, request: PermissionRequest) -> SecurityResult<PermissionResponse>;

    pub async fn approve_request(&mut self, request_id: RequestId, approver: UserId, decision: ApprovalDecision) -> SecurityResult<()>;

    pub async fn deny_request(&mut self, request_id: RequestId, approver: UserId, reason: String) -> SecurityResult<()>;

    pub fn list_pending_requests(&self) -> Vec<PendingRequest>;

    pub async fn log_audit_event(&self, event: AuditEvent) -> SecurityResult<()>;

    pub fn get_audit_events(&self, filter: AuditFilter) -> SecurityResult<Vec<AuditEvent>>;

    pub async fn create_security_policy(&mut self, policy: SecurityPolicy) -> SecurityResult<PolicyId>;

    pub async fn update_security_policy(&mut self, policy_id: PolicyId, policy: SecurityPolicy) -> SecurityResult<()>;

    pub async fn delete_security_policy(&mut self, policy_id: PolicyId) -> SecurityResult<()>;

    pub fn list_security_policies(&self) -> Vec<SecurityPolicyInfo>;

    pub async fn test_security_policy(&self, policy: &SecurityPolicy, test_cases: Vec<PermissionRequest>) -> SecurityResult<PolicyTestResult>;

    pub async fn export_security_config(&self) -> SecurityResult<String>;

    pub async fn import_security_config(&mut self, config_json: &str) -> SecurityResult<ImportResult>;

    pub async fn backup_security_data(&self, backup_path: &str) -> SecurityResult<BackupInfo>;

    pub async fn restore_security_data(&mut self, backup_path: &str) -> SecurityResult<RestoreResult>;

    pub async fn get_security_metrics(&self, timeframe: TimeRange) -> SecurityResult<SecurityMetrics>;

    pub async fn generate_compliance_report(&self, standards: Vec<ComplianceStandard>) -> SecurityResult<ComplianceReport>;

    pub async fn scan_for_vulnerabilities(&self, scope: ScanScope) -> SecurityResult<VulnerabilityReport>;

    pub async fn create_security_incident(&mut self, incident: SecurityIncident) -> SecurityResult<IncidentId>;

    pub async fn update_incident_status(&mut self, incident_id: IncidentId, status: IncidentStatus) -> SecurityResult<()>;

    pub async fn get_incident_details(&self, incident_id: IncidentId) -> SecurityResult<IncidentDetails>;

    pub fn list_active_incidents(&self) -> Vec<IncidentSummary>;

    pub async fn create_threat_rule(&mut self, rule: ThreatDetectionRule) -> SecurityResult<RuleId>;

    pub async fn update_threat_rule(&mut self, rule_id: RuleId, rule: ThreatDetectionRule) -> SecurityResult<()>;

    pub async fn delete_threat_rule(&mut self, rule_id: RuleId) -> SecurityResult<()>;

    pub fn list_threat_rules(&self) -> Vec<ThreatRuleInfo>;

    pub async fn test_threat_rule(&self, rule: &ThreatDetectionRule, test_data: Vec<SecurityEvent>) -> SecurityResult<RuleTestResult>;

    pub async fn get_threat_intelligence(&self) -> SecurityResult<ThreatIntelligence>;

    pub async fn update_threat_intelligence(&mut self) -> SecurityResult<UpdateResult>;

    pub async fn create_security_alert(&mut self, alert: SecurityAlert) -> SecurityResult<AlertId>;

    pub async fn acknowledge_alert(&mut self, alert_id: AlertId, user_id: UserId) -> SecurityResult<()>;

    pub async fn resolve_alert(&mut self, alert_id: AlertId, resolution: AlertResolution) -> SecurityResult<()>;

    pub fn list_active_alerts(&self) -> Vec<AlertSummary>;

    pub async fn configure_alert_channels(&mut self, channels: Vec<AlertChannel>) -> SecurityResult<()>;

    pub async fn test_alert_channel(&self, channel: &AlertChannel) -> SecurityResult<TestResult>;

    pub async fn get_security_dashboard_data(&self) -> SecurityResult<DashboardData>;

    pub async fn generate_security_report(&self, report_type: ReportType, timeframe: TimeRange) -> SecurityResult<SecurityReport>;

    pub async fn schedule_security_scan(&mut self, scan_config: ScanConfig) -> SecurityResult<ScanJobId>;

    pub async fn get_scan_results(&self, scan_job_id: ScanJobId) -> SecurityResult<ScanResults>;

    pub fn list_scheduled_scans(&self) -> Vec<ScheduledScanInfo>;

    pub async fn create_security_baseline(&mut self, baseline: SecurityBaseline) -> SecurityResult<BaselineId>;

    pub async fn compare_to_baseline(&self, baseline_id: BaselineId) -> SecurityResult<BaselineComparison>;

    pub async fn update_security_baseline(&mut self, baseline_id: BaselineId, baseline: SecurityBaseline) -> SecurityResult<()>;

    pub fn list_security_baselines(&self) -> Vec<BaselineInfo>;

    // AI Security Methods (NEW)
    pub async fn validate_ai_input(&self, input: &str, context: &AIContext) -> SecurityResult<ValidationResult>;

    pub async fn sanitize_ai_output(&self, output: &str, context: &AIContext) -> SecurityResult<String>;

    pub async fn detect_prompt_injection(&self, prompt: &str) -> SecurityResult<ThreatLevel>;

    pub async fn detect_jailbreak(&self, conversation: &[Message]) -> SecurityResult<JailbreakRisk>;

    pub async fn validate_ai_context(&self, context: &AIContext) -> SecurityResult<ContextIntegrity>;

    pub async fn filter_ai_content(&self, content: &str, filter_type: ContentFilterType) -> SecurityResult<FilterResult>;

    pub async fn classify_ai_threat(&self, input: &str, output: &str) -> SecurityResult<AIThreatClassification>;

    pub async fn monitor_ai_behavior(&self, session_id: SessionId, behavior: AIBehavior) -> SecurityResult<BehaviorAnalysis>;
}
```

### Unified Policy Engine (Cross-Cutting)

```rust
/// Unified policy engine for cross-cutting governance
pub struct UnifiedPolicyEngine {
    policies: Vec<Policy>,
    evaluator: PolicyEvaluator,
    cache: PolicyCache,

    // Domain-specific governance
    code_quality_governor: CodeQualityGovernor,
    trading_risk_governor: TradingRiskGovernor,
    privacy_actions_governor: PrivacyActionsGovernor,

    // Enforcement and audit
    enforcement_engine: EnforcementEngine,
    audit_logger: PolicyAuditLogger,
    approval_workflow: ApprovalWorkflow,
}

impl UnifiedPolicyEngine {
    pub fn new() -> Self;

    /// Add policy rule with domain-specific governance
    pub fn add_policy(&mut self, policy: Policy) -> SymbioteResult<()>;

    /// Remove policy by ID
    pub fn remove_policy(&mut self, policy_id: &str) -> SymbioteResult<()>;

    /// Evaluate policy with cross-domain context
    pub fn evaluate(&self, request: &PermissionRequest) -> SymbioteResult<PolicyDecision>;

    /// Enforce policy decision with audit
    pub fn enforce_policy(&self, decision: &PolicyDecision, context: &SecurityContext) -> SymbioteResult<EnforcementResult>;

    /// Handle approval workflow for policies requiring approval
    pub fn handle_approval_workflow(&self, policy_rule: &PolicyRule, context: &SecurityContext) -> SymbioteResult<ApprovalResult>;

    /// Govern code quality actions
    pub fn govern_code_quality(&self, action: &CodeQualityAction) -> SymbioteResult<GovernanceDecision>;

    /// Govern trading risk actions
    pub fn govern_trading_risk(&self, action: &TradingRiskAction) -> SymbioteResult<GovernanceDecision>;

    /// Govern privacy actions
    pub fn govern_privacy_actions(&self, action: &PrivacyAction) -> SymbioteResult<GovernanceDecision>;

    /// Explain policy decision
    pub fn explain_decision(&self, request: &PermissionRequest) -> SymbioteResult<DecisionExplanation>;

    /// Validate policies for conflicts and consistency
    pub fn validate_policies(&self) -> SymbioteResult<Vec<PolicyValidationError>>;

    /// Update policy dynamically
    pub fn update_policy(&mut self, policy_id: &str, updates: &PolicyUpdates) -> SymbioteResult<()>;

    /// Resolve policy conflicts
    pub fn resolve_conflicts(&self, conflicting_policies: &[String]) -> SymbioteResult<ConflictResolution>;

    /// Generate compliance report
    pub fn generate_compliance_report(&self, time_range: TimeRange) -> SymbioteResult<ComplianceReport>;
}

/// Legacy PolicyEngine for backward compatibility
pub struct PolicyEngine {
    policies: Vec<Policy>,
    evaluator: PolicyEvaluator,
    cache: PolicyCache,
}

impl PolicyEngine {
    pub fn new() -> Self;

    pub fn add_policy(&mut self, policy: Policy) -> SymbioteResult<()>;

    pub fn remove_policy(&mut self, policy_id: &str) -> SymbioteResult<()>;

    pub fn evaluate(&self, request: &PermissionRequest) -> SymbioteResult<PolicyDecision>;

    pub fn explain_decision(&self, request: &PermissionRequest) -> SymbioteResult<DecisionExplanation>;

    pub fn validate_policies(&self) -> SymbioteResult<Vec<PolicyValidationError>>;
}

#[derive(Debug, Clone)]
pub struct Policy {
    pub id: String,
    pub name: String,
    pub description: String,
    pub rules: Vec<PolicyRule>,
    pub priority: u32,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct PolicyRule {
    pub id: String,
    pub domain: String,
    pub condition: String,
    pub action: String,
    pub severity: String,
    pub requires_approval: bool,

    // Legacy fields for backward compatibility
    pub condition_obj: Option<Condition>,
    pub action_obj: Option<PolicyAction>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub enum Condition {
    Always,
    Never,
    And(Vec<Condition>),
    Or(Vec<Condition>),
    Not(Box<Condition>),
    ResourceMatch(ResourcePattern),
    ActionMatch(ActionPattern),
    PrincipalMatch(PrincipalPattern),
    TimeMatch(TimePattern),
    RiskLevel(RiskLevel),
    Custom(String, serde_json::Value),
}

#[derive(Debug, Clone)]
pub enum PolicyAction {
    Allow,
    Deny,
    RequireApproval {
        approvers: Vec<ApproverType>,
        timeout: Duration,
    },
    Log {
        level: LogLevel,
        message: String,
    },
    Alert {
        severity: AlertSeverity,
        recipients: Vec<String>,
    },
    Throttle {
        rate_limit: RateLimit,
    },
}
```

### Audit System

```rust
pub struct AuditLogger {
    storage: Box<dyn AuditStorage>,
    formatter: AuditFormatter,
    filters: Vec<AuditFilter>,
    encryption: Option<AuditEncryption>,
}

impl AuditLogger {
    pub fn new(config: AuditConfig) -> SymbioteResult<Self>;
    
    pub async fn log_event(&self, event: AuditEvent) -> SymbioteResult<()>;
    
    pub async fn query_events(&self, query: AuditQuery) -> SymbioteResult<Vec<AuditEvent>>;
    
    pub async fn generate_report(&self, report_type: ReportType, period: TimePeriod) -> SymbioteResult<AuditReport>;
    
    pub async fn verify_integrity(&self, period: TimePeriod) -> SymbioteResult<IntegrityReport>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub event_type: AuditEventType,
    pub principal: Principal,
    pub resource: Resource,
    pub action: Action,
    pub outcome: Outcome,
    pub metadata: HashMap<String, serde_json::Value>,
    pub risk_score: Option<f64>,
    pub session_id: Option<String>,
    pub correlation_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditEventType {
    Authentication,
    Authorization,
    DataAccess,
    DataModification,
    SystemAccess,
    ConfigurationChange,
    SecurityEvent,
    ComplianceEvent,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Outcome {
    Success,
    Failure { reason: String },
    Partial { details: String },
    Cancelled { reason: String },
}
```

### Sandboxing

```rust
pub struct Sandbox {
    config: SandboxConfig,
    process_manager: ProcessManager,
    network_isolator: NetworkIsolator,
    filesystem_restrictor: FilesystemRestrictor,
    resource_limiter: ResourceLimiter,
}

impl Sandbox {
    pub fn new(config: SandboxConfig) -> SymbioteResult<Self>;
    
    pub async fn execute<T>(&self, task: T) -> SymbioteResult<T::Output>
    where
        T: SandboxedTask + Send + 'static,
        T::Output: Send + 'static;
    
    pub fn create_isolated_environment(&self) -> SymbioteResult<IsolatedEnvironment>;
    
    pub fn set_resource_limits(&mut self, limits: ResourceLimits) -> SymbioteResult<()>;
    
    pub fn add_network_restriction(&mut self, restriction: NetworkRestriction) -> SymbioteResult<()>;
    
    pub fn add_filesystem_restriction(&mut self, restriction: FilesystemRestriction) -> SymbioteResult<()>;
}

pub trait SandboxedTask {
    type Output;
    
    fn execute(&self, environment: &IsolatedEnvironment) -> SymbioteResult<Self::Output>;
    
    fn required_permissions(&self) -> Vec<Permission>;
    
    fn resource_requirements(&self) -> ResourceRequirements;
    
    fn timeout(&self) -> Option<Duration>;
}

#[derive(Debug, Clone)]
pub struct ResourceLimits {
    pub max_memory: Option<u64>,
    pub max_cpu_time: Option<Duration>,
    pub max_wall_time: Option<Duration>,
    pub max_file_descriptors: Option<u32>,
    pub max_network_connections: Option<u32>,
    pub max_disk_usage: Option<u64>,
}
```

## Database Schema

### Security Data Persistence

```sql
-- Security incidents
CREATE TABLE security_incidents (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    incident_type VARCHAR(100) NOT NULL,
    severity VARCHAR(20) NOT NULL, -- 'low', 'medium', 'high', 'critical'
    title VARCHAR(255) NOT NULL,
    description TEXT,
    status VARCHAR(50) DEFAULT 'open', -- 'open', 'investigating', 'resolved', 'closed'
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW(),
    resolved_at TIMESTAMP,
    assigned_to VARCHAR(255),
    metadata JSONB
);

-- Security policies
CREATE TABLE security_policies (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    policy_name VARCHAR(255) NOT NULL,
    policy_type VARCHAR(100) NOT NULL,
    policy_rules JSONB NOT NULL,
    enabled BOOLEAN DEFAULT true,
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW(),
    created_by VARCHAR(255),
    version INTEGER DEFAULT 1
);

-- Permission requests
CREATE TABLE permission_requests (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id VARCHAR(255) NOT NULL,
    action_type VARCHAR(100) NOT NULL,
    resource_path VARCHAR(500),
    request_data JSONB,
    status VARCHAR(50) DEFAULT 'pending', -- 'pending', 'approved', 'denied', 'expired'
    requested_at TIMESTAMP DEFAULT NOW(),
    approved_at TIMESTAMP,
    approved_by VARCHAR(255),
    denial_reason TEXT,
    expires_at TIMESTAMP
);

-- Audit events
CREATE TABLE audit_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_type VARCHAR(100) NOT NULL,
    user_id VARCHAR(255),
    action VARCHAR(255) NOT NULL,
    resource VARCHAR(500),
    outcome VARCHAR(50) NOT NULL, -- 'success', 'failure', 'denied'
    timestamp TIMESTAMP DEFAULT NOW(),
    ip_address INET,
    user_agent TEXT,
    session_id VARCHAR(255),
    metadata JSONB
);

-- Threat detection rules
CREATE TABLE threat_rules (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    rule_name VARCHAR(255) NOT NULL,
    rule_type VARCHAR(100) NOT NULL,
    pattern TEXT NOT NULL,
    severity VARCHAR(20) NOT NULL,
    enabled BOOLEAN DEFAULT true,
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW(),
    last_triggered TIMESTAMP,
    trigger_count INTEGER DEFAULT 0
);

-- Security alerts
CREATE TABLE security_alerts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    alert_type VARCHAR(100) NOT NULL,
    severity VARCHAR(20) NOT NULL,
    title VARCHAR(255) NOT NULL,
    description TEXT,
    status VARCHAR(50) DEFAULT 'active', -- 'active', 'acknowledged', 'resolved'
    created_at TIMESTAMP DEFAULT NOW(),
    acknowledged_at TIMESTAMP,
    acknowledged_by VARCHAR(255),
    resolved_at TIMESTAMP,
    metadata JSONB
);

-- User roles and permissions
CREATE TABLE user_roles (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id VARCHAR(255) NOT NULL,
    role_name VARCHAR(100) NOT NULL,
    granted_at TIMESTAMP DEFAULT NOW(),
    granted_by VARCHAR(255),
    expires_at TIMESTAMP,
    active BOOLEAN DEFAULT true
);

-- Security baselines
CREATE TABLE security_baselines (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    baseline_name VARCHAR(255) NOT NULL,
    baseline_type VARCHAR(100) NOT NULL,
    configuration JSONB NOT NULL,
    created_at TIMESTAMP DEFAULT NOW(),
    created_by VARCHAR(255),
    is_active BOOLEAN DEFAULT false
);

-- Vulnerability scan results
CREATE TABLE vulnerability_scans (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    scan_type VARCHAR(100) NOT NULL,
    target_system VARCHAR(255),
    scan_status VARCHAR(50) DEFAULT 'running',
    started_at TIMESTAMP DEFAULT NOW(),
    completed_at TIMESTAMP,
    vulnerabilities_found INTEGER DEFAULT 0,
    critical_count INTEGER DEFAULT 0,
    high_count INTEGER DEFAULT 0,
    medium_count INTEGER DEFAULT 0,
    low_count INTEGER DEFAULT 0,
    scan_results JSONB
);

-- Indexes for performance
CREATE INDEX idx_security_incidents_status ON security_incidents(status);
CREATE INDEX idx_security_incidents_severity ON security_incidents(severity);
CREATE INDEX idx_permission_requests_user ON permission_requests(user_id);
CREATE INDEX idx_permission_requests_status ON permission_requests(status);
CREATE INDEX idx_audit_events_user ON audit_events(user_id);
CREATE INDEX idx_audit_events_timestamp ON audit_events(timestamp);
CREATE INDEX idx_security_alerts_status ON security_alerts(status);
CREATE INDEX idx_user_roles_user ON user_roles(user_id);
CREATE INDEX idx_vulnerability_scans_status ON vulnerability_scans(scan_status);
```

## Implementation Details

### Technology Stack

- **Sandboxing**: Linux namespaces, Windows Job Objects, macOS sandbox
- **Process Isolation**: tokio process management with resource limits
- **Network Security**: iptables/netfilter integration for Linux
- **Audit Storage**: SQLite with encryption for audit logs
- **Policy Engine**: Custom rule engine with JSON/YAML configuration
- **Monitoring**: Real-time threat detection with machine learning
- **Compliance**: Built-in templates for GDPR, SOC2, HIPAA

### Key Dependencies

```toml
[dependencies]
tokio = { version = "1.0", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
serde_yaml = "0.9"
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1.0", features = ["v4", "serde"] }
thiserror = "1.0"
tracing = "0.1"
regex = "1.0"
dashmap = "5.0"
parking_lot = "0.12"
ring = "0.16"
symbiote-core = { path = "../symbiote-core" }
symbiote-vault = { path = "../vault" }
symbiote-storage = { path = "../storage" }

[target.'cfg(unix)'.dependencies]
nix = "0.26"
libc = "0.2"

[target.'cfg(windows)'.dependencies]
windows = { version = "0.48", features = ["Win32_System_JobObjects", "Win32_Security"] }

[dev-dependencies]
tempfile = "3.0"
tokio-test = "0.4"
```

### Security Hardening

1. **Input Validation**: Strict validation of all policy inputs
2. **Output Sanitization**: Sanitize audit log outputs
3. **Memory Safety**: Use secure memory allocation for sensitive data
4. **Time-Based Attacks**: Constant-time operations for security decisions
5. **Side-Channel Protection**: Prevent information leakage through timing

## Testing Strategy

### Unit Tests

- **Permission Evaluation**: Test all permission profile logic
- **Policy Engine**: Test policy rule evaluation and combinations
- **Audit Logging**: Test audit event creation and storage
- **Sandboxing**: Test process isolation and resource limits
- **Privacy Controls**: Test data anonymization and encryption

### Integration Tests

- **End-to-End Security**: Test complete security workflows
- **Cross-Platform**: Test sandboxing on all supported platforms
- **Performance**: Test security overhead and latency
- **Compliance**: Test compliance reporting and audit trails
- **Threat Simulation**: Test against simulated security threats

### Security Tests

- **Penetration Testing**: Test against common attack vectors
- **Privilege Escalation**: Test sandbox escape attempts
- **Audit Integrity**: Test audit log tampering detection
- **Policy Bypass**: Test attempts to bypass security policies

## Integration Points

### Upstream Dependencies

- **symbiote-core**: Uses error types, configuration, and DI
- **vault**: Uses secure storage for sensitive security data
- **storage**: Uses database for audit log persistence

### Downstream Consumers

- **All Symbiote Components**: Security enforcement and audit logging
- **Agent Framework**: Permission checking for agent actions
- **Workflow Engine**: Security policies for workflow execution
- **Trading System**: Risk assessment and approval workflows
- **UI Applications**: User permission management and audit display

### External Integrations

- **SIEM Systems**: Security information and event management
- **Identity Providers**: External authentication and authorization
- **Compliance Tools**: Automated compliance reporting
- **Monitoring Systems**: Security metrics and alerting

## Acceptance Criteria

### Functional Requirements

- [ ] Configurable permission profiles (ZeroTrust, HiL, Full Auto)
- [ ] Flexible policy engine with rule-based evaluation
- [ ] Comprehensive audit logging with integrity verification
- [ ] Process sandboxing with resource limits
- [ ] Privacy controls with data anonymization
- [ ] Real-time threat detection and alerting
- [ ] Compliance reporting for major frameworks

### Non-Functional Requirements

- [ ] Sub-millisecond permission evaluation
- [ ] 99.99% audit log integrity
- [ ] Zero privilege escalation vulnerabilities
- [ ] Comprehensive security coverage
- [ ] Minimal performance overhead (<5%)
- [ ] Cross-platform compatibility

### Quality Gates

- [ ] Security audit by external firm passes
- [ ] Penetration testing shows no critical vulnerabilities
- [ ] Compliance certification for target frameworks
- [ ] Performance benchmarks meet targets
- [ ] All security tests pass
- [ ] Documentation includes security best practices

## Dependencies & Prerequisites

### Build Dependencies

- **Rust Toolchain**: 1.70+ with security features
- **Platform SDKs**: Native security API access
- **Security Tools**: Static analysis and vulnerability scanning

### Runtime Dependencies

- **Operating System**: Native security and sandboxing support
- **Privileges**: Appropriate permissions for sandboxing operations
- **Storage**: Secure storage for audit logs and policies
- **Network**: Secure communication channels

### Development Prerequisites

- **Security Testing Tools**: Penetration testing and vulnerability assessment
- **Compliance Knowledge**: Understanding of relevant compliance frameworks
- **Threat Modeling**: Security threat analysis and mitigation planning
- **Audit Tools**: Log analysis and integrity verification tools

This security system provides the comprehensive protection and compliance capabilities that enable Symbiote to operate safely in enterprise environments while maintaining user trust and regulatory compliance.

## Unified Security Architecture (Master Plan Features)

### Enhanced Security Components

```rust
/// Encryption manager for comprehensive encryption support
pub struct EncryptionManager {
    chacha20_cipher: ChaCha20Poly1305Cipher,
    key_manager: KeyManager,
    encryption_policies: EncryptionPolicies,
}

impl EncryptionManager {
    /// API Key Encryption with ChaCha20Poly1305
    pub async fn encrypt_api_key(&self, api_key: &str) -> SymbioteResult<EncryptedApiKey>;

    pub async fn decrypt_api_key(&self, encrypted_key: &EncryptedApiKey) -> SymbioteResult<String>;

    /// Encryption at Rest for all stored data
    pub async fn encrypt_at_rest(&self, data: &[u8]) -> SymbioteResult<EncryptedData>;

    pub async fn decrypt_at_rest(&self, encrypted_data: &EncryptedData) -> SymbioteResult<Vec<u8>>;

    /// Encryption in Transit with TLS/mTLS
    pub async fn setup_tls_config(&self) -> SymbioteResult<TLSConfig>;

    pub async fn setup_mtls_config(&self, client_cert: &Certificate) -> SymbioteResult<mTLSConfig>;

    /// Key rotation and management
    pub async fn rotate_keys(&mut self) -> SymbioteResult<KeyRotationResult>;

    pub async fn backup_keys(&self, backup_location: &str) -> SymbioteResult<()>;
}

/// WASM-based sandbox for secure code execution
pub struct WasmSandbox {
    runtime: WasmRuntime,
    security_policies: WasmSecurityPolicies,
    resource_limiter: WasmResourceLimiter,
}

impl WasmSandbox {
    /// Sandboxed Execution with WASM-based code execution
    pub async fn execute_code(&self, code: &str, context: &ExecutionContext) -> SymbioteResult<ExecutionResult>;

    pub async fn create_secure_context(&self, permissions: &Permissions) -> SymbioteResult<SecureContext>;

    pub async fn validate_code_safety(&self, code: &str) -> SymbioteResult<SafetyAssessment>;

    /// Resource limits enforcement
    pub async fn enforce_memory_limits(&self, context: &SecureContext, limit: u64) -> SymbioteResult<()>;

    pub async fn enforce_cpu_limits(&self, context: &SecureContext, limit: Duration) -> SymbioteResult<()>;

    pub async fn enforce_network_limits(&self, context: &SecureContext, allowlist: &[String]) -> SymbioteResult<()>;
}

/// Vulnerability scanner for security assessment
pub struct VulnerabilityScanner {
    scanner_engine: ScannerEngine,
    vulnerability_db: VulnerabilityDatabase,
    scan_policies: ScanPolicies,
}

impl VulnerabilityScanner {
    /// Vulnerability Management with automated scanning
    pub async fn scan_system(&self) -> SymbioteResult<VulnerabilityScanResult>;

    pub async fn scan_dependencies(&self, dependencies: &[Dependency]) -> SymbioteResult<DependencyScanResult>;

    pub async fn scan_configuration(&self, config: &SystemConfig) -> SymbioteResult<ConfigScanResult>;

    /// Continuous monitoring
    pub async fn start_continuous_scan(&self, interval: Duration) -> SymbioteResult<ScanJobId>;

    pub async fn get_scan_results(&self, scan_id: &ScanJobId) -> SymbioteResult<ScanResults>;

    /// Remediation recommendations
    pub async fn generate_remediation_plan(&self, vulnerabilities: &[Vulnerability]) -> SymbioteResult<RemediationPlan>;

    pub async fn apply_auto_remediation(&self, plan: &RemediationPlan) -> SymbioteResult<RemediationResult>;
}

/// Penetration tester for security validation
pub struct PenetrationTester {
    test_engine: PenTestEngine,
    attack_scenarios: AttackScenarios,
    reporting_engine: ReportingEngine,
}

impl PenetrationTester {
    /// Penetration Testing with automated security testing
    pub async fn run_penetration_test(&self, target: &TestTarget) -> SymbioteResult<PenTestResult>;

    pub async fn test_authentication(&self, auth_endpoints: &[AuthEndpoint]) -> SymbioteResult<AuthTestResult>;

    pub async fn test_authorization(&self, protected_resources: &[Resource]) -> SymbioteResult<AuthzTestResult>;

    pub async fn test_input_validation(&self, input_endpoints: &[InputEndpoint]) -> SymbioteResult<InputTestResult>;

    /// Attack simulation
    pub async fn simulate_sql_injection(&self, endpoints: &[DatabaseEndpoint]) -> SymbioteResult<SQLInjectionResult>;

    pub async fn simulate_xss_attack(&self, web_endpoints: &[WebEndpoint]) -> SymbioteResult<XSSTestResult>;

    pub async fn simulate_privilege_escalation(&self, user_contexts: &[UserContext]) -> SymbioteResult<PrivEscResult>;

    /// Security reporting
    pub async fn generate_security_report(&self, test_results: &[TestResult]) -> SymbioteResult<SecurityReport>;

    pub async fn generate_compliance_report(&self, framework: ComplianceFramework) -> SymbioteResult<ComplianceReport>;
}

/// Multi-Factor Authentication support
pub struct MFAManager {
    providers: HashMap<String, Box<dyn MFAProvider>>,
    policies: MFAPolicies,
    session_manager: MFASessionManager,
}

impl MFAManager {
    /// Multi-Factor Authentication with comprehensive MFA support
    pub async fn setup_mfa(&self, user_id: &UserId, method: MFAMethod) -> SymbioteResult<MFASetupResult>;

    pub async fn verify_mfa(&self, user_id: &UserId, token: &str) -> SymbioteResult<MFAVerificationResult>;

    pub async fn require_mfa_for_action(&self, action: &SecurityAction) -> SymbioteResult<bool>;

    /// MFA methods support
    pub async fn setup_totp(&self, user_id: &UserId) -> SymbioteResult<TOTPSetup>;

    pub async fn setup_sms(&self, user_id: &UserId, phone: &str) -> SymbioteResult<SMSSetup>;

    pub async fn setup_email(&self, user_id: &UserId, email: &str) -> SymbioteResult<EmailSetup>;

    pub async fn setup_hardware_key(&self, user_id: &UserId, key_info: &HardwareKeyInfo) -> SymbioteResult<HardwareKeySetup>;
}

/// Role-Based Access Control system
pub struct RBACManager {
    roles: HashMap<RoleId, Role>,
    permissions: HashMap<PermissionId, Permission>,
    assignments: UserRoleAssignments,
}

impl RBACManager {
    /// Role-Based Access Control with granular permissions
    pub async fn create_role(&mut self, role: &Role) -> SymbioteResult<RoleId>;

    pub async fn assign_role(&mut self, user_id: &UserId, role_id: &RoleId) -> SymbioteResult<()>;

    pub async fn check_permission(&self, user_id: &UserId, permission: &Permission) -> SymbioteResult<bool>;

    /// Role hierarchy and inheritance
    pub async fn create_role_hierarchy(&mut self, parent: &RoleId, child: &RoleId) -> SymbioteResult<()>;

    pub async fn get_effective_permissions(&self, user_id: &UserId) -> SymbioteResult<Vec<Permission>>;

    /// Dynamic role assignment
    pub async fn assign_temporary_role(&mut self, user_id: &UserId, role_id: &RoleId, duration: Duration) -> SymbioteResult<()>;

    pub async fn revoke_role(&mut self, user_id: &UserId, role_id: &RoleId) -> SymbioteResult<()>;
}

/// Network security manager
pub struct NetworkSecurityManager {
    firewall_rules: FirewallRules,
    network_monitor: NetworkMonitor,
    intrusion_detector: IntrusionDetector,
}

impl NetworkSecurityManager {
    /// Network Security with firewall and monitoring
    pub async fn configure_firewall(&mut self, rules: &[FirewallRule]) -> SymbioteResult<()>;

    pub async fn monitor_network_traffic(&self) -> SymbioteResult<NetworkTrafficReport>;

    pub async fn detect_intrusions(&self) -> SymbioteResult<Vec<IntrusionEvent>>;

    /// Network isolation
    pub async fn create_network_isolation(&self, context: &IsolationContext) -> SymbioteResult<NetworkIsolation>;

    pub async fn enforce_network_policies(&self, policies: &[NetworkPolicy]) -> SymbioteResult<()>;
}

/// Container security manager
pub struct ContainerSecurityManager {
    image_scanner: ImageScanner,
    runtime_monitor: RuntimeMonitor,
    policy_enforcer: PolicyEnforcer,
}

impl ContainerSecurityManager {
    /// Container Security with image scanning and runtime protection
    pub async fn scan_container_image(&self, image: &ContainerImage) -> SymbioteResult<ImageScanResult>;

    pub async fn monitor_container_runtime(&self, container_id: &ContainerId) -> SymbioteResult<RuntimeSecurityReport>;

    pub async fn enforce_security_policies(&self, container_id: &ContainerId, policies: &[SecurityPolicy]) -> SymbioteResult<()>;

    /// Secure container execution
    pub async fn create_secure_container(&self, config: &SecureContainerConfig) -> SymbioteResult<SecureContainer>;

    pub async fn validate_container_security(&self, container_id: &ContainerId) -> SymbioteResult<SecurityValidationResult>;
}

/// Security metrics and KPIs
pub struct SecurityMetricsManager {
    metrics_collector: MetricsCollector,
    kpi_calculator: KPICalculator,
    dashboard_generator: DashboardGenerator,
}

impl SecurityMetricsManager {
    /// Security Metrics with comprehensive KPIs and dashboards
    pub async fn collect_security_metrics(&self) -> SymbioteResult<SecurityMetrics>;

    pub async fn calculate_security_kpis(&self, time_range: &TimeRange) -> SymbioteResult<SecurityKPIs>;

    pub async fn generate_security_dashboard(&self) -> SymbioteResult<SecurityDashboard>;

    /// Trend analysis
    pub async fn analyze_security_trends(&self, metrics: &[SecurityMetrics]) -> SymbioteResult<TrendAnalysis>;

    pub async fn predict_security_risks(&self, historical_data: &[SecurityEvent]) -> SymbioteResult<RiskPrediction>;

    /// Alerting and notifications
    pub async fn setup_security_alerts(&self, alert_rules: &[AlertRule]) -> SymbioteResult<()>;

    pub async fn send_security_notification(&self, notification: &SecurityNotification) -> SymbioteResult<()>;
}

/// Compliance framework manager
pub struct ComplianceFrameworkManager {
    frameworks: HashMap<String, ComplianceFramework>,
    assessor: ComplianceAssessor,
    reporter: ComplianceReporter,
}

impl ComplianceFrameworkManager {
    /// Compliance Framework with SOC2, GDPR, HIPAA support
    pub async fn assess_compliance(&self, framework: &str) -> SymbioteResult<ComplianceAssessment>;

    pub async fn generate_compliance_report(&self, framework: &str, time_range: &TimeRange) -> SymbioteResult<ComplianceReport>;

    pub async fn track_compliance_metrics(&self, framework: &str) -> SymbioteResult<ComplianceMetrics>;

    /// Framework-specific implementations
    pub async fn assess_soc2_compliance(&self) -> SymbioteResult<SOC2Assessment>;

    pub async fn assess_gdpr_compliance(&self) -> SymbioteResult<GDPRAssessment>;

    pub async fn assess_hipaa_compliance(&self) -> SymbioteResult<HIPAAAssessment>;

    /// Remediation and improvement
    pub async fn generate_remediation_plan(&self, assessment: &ComplianceAssessment) -> SymbioteResult<RemediationPlan>;

    pub async fn track_remediation_progress(&self, plan: &RemediationPlan) -> SymbioteResult<RemediationProgress>;
}

/// AI Security types and enums (NEW)
#[derive(Debug, Clone)]
pub struct AIContext {
    pub session_id: SessionId,
    pub user_id: UserId,
    pub conversation_history: Vec<Message>,
    pub security_level: SecurityLevel,
    pub provider: ProviderId,
}

#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub is_safe: bool,
    pub threat_level: ThreatLevel,
    pub detected_threats: Vec<AIThreat>,
    pub recommendations: Vec<SecurityRecommendation>,
}

#[derive(Debug, Clone)]
pub enum ThreatLevel {
    None,
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone)]
pub enum AIThreat {
    PromptInjection { pattern: String, confidence: f32 },
    Jailbreaking { technique: String, severity: ThreatLevel },
    CredentialLeakage { credential_type: String, location: String },
    ContextPollution { source: String, impact: String },
    AdversarialInput { attack_type: String, payload: String },
}

#[derive(Debug, Clone)]
pub enum JailbreakRisk {
    None,
    Low { indicators: Vec<String> },
    Medium { techniques: Vec<String> },
    High { attack_vectors: Vec<String> },
    Critical { immediate_threats: Vec<String> },
}

#[derive(Debug, Clone)]
pub struct ContextIntegrity {
    pub is_valid: bool,
    pub tampering_detected: bool,
    pub integrity_score: f32,
    pub violations: Vec<IntegrityViolation>,
}

#[derive(Debug, Clone)]
pub enum ContentFilterType {
    Profanity,
    Violence,
    SexualContent,
    Hate,
    SelfHarm,
    Illegal,
    Misinformation,
    Spam,
}

#[derive(Debug, Clone)]
pub struct FilterResult {
    pub is_allowed: bool,
    pub filtered_content: String,
    pub violations: Vec<ContentViolation>,
    pub confidence: f32,
}

#[derive(Debug, Clone)]
pub struct AIThreatClassification {
    pub threat_type: AIThreatType,
    pub severity: ThreatLevel,
    pub confidence: f32,
    pub mitigation_actions: Vec<MitigationAction>,
}

#[derive(Debug, Clone)]
pub enum AIThreatType {
    PromptInjection,
    Jailbreaking,
    DataExfiltration,
    ModelManipulation,
    ContextPoisoning,
    OutputManipulation,
}

/// Security types and enums
#[derive(Debug, Clone)]
pub struct EncryptedApiKey {
    pub ciphertext: Vec<u8>,
    pub nonce: Vec<u8>,
    pub key_id: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MFAMethod {
    TOTP,
    SMS,
    Email,
    HardwareKey,
    Biometric,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ComplianceFramework {
    SOC2,
    GDPR,
    HIPAA,
    PCI_DSS,
    ISO27001,
    NIST,
}

#[derive(Debug, Clone)]
pub struct SecurityKPIs {
    pub threat_detection_rate: f64,
    pub incident_response_time: Duration,
    pub vulnerability_remediation_time: Duration,
    pub compliance_score: f64,
    pub security_training_completion: f64,
}
```
