//! # Security Audit Logging System for Symbiote IDE
//! 
//! Comprehensive security event logging and monitoring system with real-time
//! threat detection, compliance reporting, and forensic analysis capabilities.

use crate::{Result, ProjectId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::IpAddr;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;
use tracing::{info, warn, error};

/// Security event severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SecuritySeverity {
    /// Informational events (successful logins, etc.)
    Info,
    /// Low-risk events (failed login attempts)
    Low,
    /// Medium-risk events (privilege escalation attempts)
    Medium,
    /// High-risk events (data access violations)
    High,
    /// Critical security incidents (system compromise)
    Critical,
}

impl SecuritySeverity {
    /// Get numeric priority for sorting
    pub fn priority(&self) -> u8 {
        match self {
            Self::Info => 1,
            Self::Low => 2,
            Self::Medium => 3,
            Self::High => 4,
            Self::Critical => 5,
        }
    }

    /// Check if severity requires immediate attention
    pub fn requires_immediate_attention(&self) -> bool {
        matches!(self, Self::High | Self::Critical)
    }
}

/// Types of security events
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SecurityEventType {
    // Authentication Events
    LoginSuccess,
    LoginFailure,
    LogoutSuccess,
    PasswordChange,
    AccountLockout,
    
    // Authorization Events
    AccessGranted,
    AccessDenied,
    PrivilegeEscalation,
    RoleChange,
    
    // Data Access Events
    DataRead,
    DataWrite,
    DataDelete,
    DataExport,
    UnauthorizedDataAccess,
    
    // System Events
    SystemAccess,
    ConfigurationChange,
    ServiceStart,
    ServiceStop,
    
    // Security Events
    SecurityPolicyViolation,
    SuspiciousActivity,
    MalwareDetection,
    IntrusionAttempt,
    
    // API Events
    ApiKeyUsage,
    RateLimitExceeded,
    InvalidApiRequest,
    
    // Custom Events
    Custom(String),
}

/// Security audit event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityEvent {
    /// Unique event ID
    pub id: Uuid,
    
    /// Event timestamp
    pub timestamp: u64,
    
    /// Event type
    pub event_type: SecurityEventType,
    
    /// Event severity
    pub severity: SecuritySeverity,
    
    /// User ID associated with the event
    pub user_id: Option<String>,
    
    /// Session ID if applicable
    pub session_id: Option<String>,
    
    /// Project ID if applicable
    pub project_id: Option<ProjectId>,
    
    /// Source IP address
    pub source_ip: Option<IpAddr>,
    
    /// User agent string
    pub user_agent: Option<String>,
    
    /// Resource being accessed
    pub resource: Option<String>,
    
    /// Action performed
    pub action: String,
    
    /// Event outcome (success/failure)
    pub outcome: EventOutcome,
    
    /// Additional event details
    pub details: HashMap<String, String>,
    
    /// Risk score (0-100)
    pub risk_score: u8,
    
    /// Whether this event triggered an alert
    pub alert_triggered: bool,
}

/// Event outcome
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventOutcome {
    Success,
    Failure,
    Blocked,
    Unknown,
}

/// Audit log query parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditQuery {
    /// Start time for query range
    pub start_time: Option<u64>,
    
    /// End time for query range
    pub end_time: Option<u64>,
    
    /// Filter by event types
    pub event_types: Option<Vec<SecurityEventType>>,
    
    /// Filter by severity levels
    pub severities: Option<Vec<SecuritySeverity>>,
    
    /// Filter by user ID
    pub user_id: Option<String>,
    
    /// Filter by project ID
    pub project_id: Option<ProjectId>,
    
    /// Filter by source IP
    pub source_ip: Option<IpAddr>,
    
    /// Filter by outcome
    pub outcome: Option<EventOutcome>,
    
    /// Minimum risk score
    pub min_risk_score: Option<u8>,
    
    /// Maximum number of results
    pub limit: Option<usize>,
    
    /// Offset for pagination
    pub offset: Option<usize>,
}

/// Audit statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditStats {
    /// Total number of events
    pub total_events: u64,
    
    /// Events by severity
    pub events_by_severity: HashMap<SecuritySeverity, u64>,
    
    /// Events by type
    pub events_by_type: HashMap<SecurityEventType, u64>,
    
    /// Events by outcome
    pub events_by_outcome: HashMap<EventOutcome, u64>,
    
    /// High-risk events count
    pub high_risk_events: u64,
    
    /// Failed authentication attempts
    pub failed_auth_attempts: u64,
    
    /// Unique users involved
    pub unique_users: u64,
    
    /// Unique IP addresses
    pub unique_ips: u64,
    
    /// Time range covered
    pub time_range_start: u64,
    pub time_range_end: u64,
}

/// Security alert configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertRule {
    /// Rule ID
    pub id: Uuid,
    
    /// Rule name
    pub name: String,
    
    /// Rule description
    pub description: String,
    
    /// Event types to monitor
    pub event_types: Vec<SecurityEventType>,
    
    /// Minimum severity to trigger
    pub min_severity: SecuritySeverity,
    
    /// Minimum risk score to trigger
    pub min_risk_score: u8,
    
    /// Time window for aggregation (seconds)
    pub time_window: u64,
    
    /// Threshold count within time window
    pub threshold: u32,
    
    /// Whether rule is active
    pub enabled: bool,
    
    /// Alert destinations
    pub alert_destinations: Vec<AlertDestination>,
}

/// Alert destination configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertDestination {
    Email(String),
    Webhook(String),
    Slack(String),
    Discord(String),
    Log,
}

/// Security audit logger
pub struct SecurityAuditLogger {
    /// Configuration
    config: AuditConfig,
    
    /// Alert rules
    alert_rules: Vec<AlertRule>,
    
    /// Event buffer for batch processing
    event_buffer: Vec<SecurityEvent>,
}

/// Audit configuration
#[derive(Debug, Clone)]
pub struct AuditConfig {
    /// Whether audit logging is enabled
    pub enabled: bool,
    
    /// Maximum events to buffer before flushing
    pub buffer_size: usize,
    
    /// Flush interval in seconds
    pub flush_interval: u64,
    
    /// Retention period in days
    pub retention_days: u32,
    
    /// Whether to enable real-time alerting
    pub real_time_alerts: bool,
    
    /// Whether to log successful events
    pub log_success_events: bool,
    
    /// Whether to log info-level events
    pub log_info_events: bool,
    
    /// Database connection for persistence
    pub database_url: Option<String>,
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            buffer_size: 1000,
            flush_interval: 60, // 1 minute
            retention_days: 90, // 3 months
            real_time_alerts: true,
            log_success_events: true,
            log_info_events: false, // Reduce noise
            database_url: None,
        }
    }
}

impl SecurityAuditLogger {
    /// Create a new security audit logger
    pub fn new(config: AuditConfig) -> Self {
        Self {
            config,
            alert_rules: Vec::new(),
            event_buffer: Vec::new(),
        }
    }

    /// Log a security event
    pub async fn log_event(&mut self, mut event: SecurityEvent) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        // Skip info events if not configured to log them
        if event.severity == SecuritySeverity::Info && !self.config.log_info_events {
            return Ok(());
        }

        // Skip successful events if not configured to log them
        if event.outcome == EventOutcome::Success && !self.config.log_success_events {
            return Ok(());
        }

        // Set timestamp if not provided
        if event.timestamp == 0 {
            event.timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
        }

        // Calculate risk score if not set
        if event.risk_score == 0 {
            event.risk_score = self.calculate_risk_score(&event);
        }

        // Check alert rules
        if self.config.real_time_alerts {
            event.alert_triggered = self.check_alert_rules(&event).await?;
        }

        // Log to tracing system
        match event.severity {
            SecuritySeverity::Info => info!(
                event_type = ?event.event_type,
                user_id = ?event.user_id,
                action = %event.action,
                "Security event logged"
            ),
            SecuritySeverity::Low => info!(
                event_type = ?event.event_type,
                user_id = ?event.user_id,
                action = %event.action,
                risk_score = event.risk_score,
                "Low-risk security event"
            ),
            SecuritySeverity::Medium => warn!(
                event_type = ?event.event_type,
                user_id = ?event.user_id,
                action = %event.action,
                risk_score = event.risk_score,
                "Medium-risk security event"
            ),
            SecuritySeverity::High => error!(
                event_type = ?event.event_type,
                user_id = ?event.user_id,
                action = %event.action,
                risk_score = event.risk_score,
                "High-risk security event"
            ),
            SecuritySeverity::Critical => error!(
                event_type = ?event.event_type,
                user_id = ?event.user_id,
                action = %event.action,
                risk_score = event.risk_score,
                alert_triggered = event.alert_triggered,
                "CRITICAL security event"
            ),
        }

        // Add to buffer
        self.event_buffer.push(event);

        // Flush if buffer is full
        if self.event_buffer.len() >= self.config.buffer_size {
            self.flush_events().await?;
        }

        Ok(())
    }

    /// Calculate risk score for an event
    fn calculate_risk_score(&self, event: &SecurityEvent) -> u8 {
        let mut score = 0u8;

        // Base score from severity
        score += match event.severity {
            SecuritySeverity::Info => 10,
            SecuritySeverity::Low => 25,
            SecuritySeverity::Medium => 50,
            SecuritySeverity::High => 75,
            SecuritySeverity::Critical => 90,
        };

        // Adjust based on event type
        score += match event.event_type {
            SecurityEventType::LoginFailure => 5,
            SecurityEventType::AccessDenied => 10,
            SecurityEventType::PrivilegeEscalation => 20,
            SecurityEventType::UnauthorizedDataAccess => 25,
            SecurityEventType::SecurityPolicyViolation => 15,
            SecurityEventType::SuspiciousActivity => 20,
            SecurityEventType::MalwareDetection => 30,
            SecurityEventType::IntrusionAttempt => 35,
            _ => 0,
        };

        // Adjust based on outcome
        if event.outcome == EventOutcome::Failure {
            score += 5;
        }

        // Cap at 100
        score.min(100)
    }

    /// Check if event triggers any alert rules
    async fn check_alert_rules(&self, event: &SecurityEvent) -> Result<bool> {
        for rule in &self.alert_rules {
            if !rule.enabled {
                continue;
            }

            // Check if event matches rule criteria
            if rule.event_types.contains(&event.event_type)
                && event.severity.priority() >= rule.min_severity.priority()
                && event.risk_score >= rule.min_risk_score
            {
                // For now, trigger alert immediately
                // In production, would implement time window aggregation
                self.trigger_alert(rule, event).await?;
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Trigger an alert
    async fn trigger_alert(&self, rule: &AlertRule, event: &SecurityEvent) -> Result<()> {
        for destination in &rule.alert_destinations {
            match destination {
                AlertDestination::Log => {
                    error!(
                        rule_name = %rule.name,
                        event_id = %event.id,
                        event_type = ?event.event_type,
                        severity = ?event.severity,
                        risk_score = event.risk_score,
                        "Security alert triggered"
                    );
                }
                AlertDestination::Email(email) => {
                    // TODO: Implement email alerting
                    info!("Would send email alert to: {}", email);
                }
                AlertDestination::Webhook(url) => {
                    // TODO: Implement webhook alerting
                    info!("Would send webhook alert to: {}", url);
                }
                AlertDestination::Slack(channel) => {
                    // TODO: Implement Slack alerting
                    info!("Would send Slack alert to: {}", channel);
                }
                AlertDestination::Discord(channel) => {
                    // TODO: Implement Discord alerting
                    info!("Would send Discord alert to: {}", channel);
                }
            }
        }

        Ok(())
    }

    /// Flush buffered events to storage
    async fn flush_events(&mut self) -> Result<()> {
        if self.event_buffer.is_empty() {
            return Ok(());
        }

        // TODO: Implement database persistence
        info!("Flushing {} security events to storage", self.event_buffer.len());
        
        self.event_buffer.clear();
        Ok(())
    }

    /// Add an alert rule
    pub fn add_alert_rule(&mut self, rule: AlertRule) {
        self.alert_rules.push(rule);
    }

    /// Remove an alert rule
    pub fn remove_alert_rule(&mut self, rule_id: &Uuid) {
        self.alert_rules.retain(|rule| rule.id != *rule_id);
    }

    /// Query audit events
    pub async fn query_events(&self, _query: AuditQuery) -> Result<Vec<SecurityEvent>> {
        // TODO: Implement database query
        // For now, return empty results
        Ok(Vec::new())
    }

    /// Get audit statistics
    pub async fn get_statistics(&self, query: AuditQuery) -> Result<AuditStats> {
        // TODO: Implement statistics calculation from database
        Ok(AuditStats {
            total_events: 0,
            events_by_severity: HashMap::new(),
            events_by_type: HashMap::new(),
            events_by_outcome: HashMap::new(),
            high_risk_events: 0,
            failed_auth_attempts: 0,
            unique_users: 0,
            unique_ips: 0,
            time_range_start: query.start_time.unwrap_or(0),
            time_range_end: query.end_time.unwrap_or(
                SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
            ),
        })
    }
}

/// Helper functions for creating common security events
impl SecurityEvent {
    /// Create a login success event
    pub fn login_success(user_id: String, source_ip: Option<IpAddr>) -> Self {
        Self {
            id: Uuid::new_v4(),
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            event_type: SecurityEventType::LoginSuccess,
            severity: SecuritySeverity::Info,
            user_id: Some(user_id),
            session_id: None,
            project_id: None,
            source_ip,
            user_agent: None,
            resource: None,
            action: "User login".to_string(),
            outcome: EventOutcome::Success,
            details: HashMap::new(),
            risk_score: 0,
            alert_triggered: false,
        }
    }

    /// Create a login failure event
    pub fn login_failure(user_id: Option<String>, source_ip: Option<IpAddr>, reason: String) -> Self {
        let mut details = HashMap::new();
        details.insert("failure_reason".to_string(), reason);

        Self {
            id: Uuid::new_v4(),
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            event_type: SecurityEventType::LoginFailure,
            severity: SecuritySeverity::Low,
            user_id,
            session_id: None,
            project_id: None,
            source_ip,
            user_agent: None,
            resource: None,
            action: "Failed login attempt".to_string(),
            outcome: EventOutcome::Failure,
            details,
            risk_score: 0,
            alert_triggered: false,
        }
    }

    /// Create an unauthorized access event
    pub fn unauthorized_access(
        user_id: Option<String>,
        resource: String,
        source_ip: Option<IpAddr>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            event_type: SecurityEventType::AccessDenied,
            severity: SecuritySeverity::Medium,
            user_id,
            session_id: None,
            project_id: None,
            source_ip,
            user_agent: None,
            resource: Some(resource),
            action: "Unauthorized access attempt".to_string(),
            outcome: EventOutcome::Blocked,
            details: HashMap::new(),
            risk_score: 0,
            alert_triggered: false,
        }
    }

    /// Create a data access event
    pub fn data_access(
        user_id: String,
        project_id: ProjectId,
        resource: String,
        action: String,
        outcome: EventOutcome,
    ) -> Self {
        let severity = match outcome {
            EventOutcome::Success => SecuritySeverity::Info,
            EventOutcome::Failure => SecuritySeverity::Low,
            EventOutcome::Blocked => SecuritySeverity::Medium,
            EventOutcome::Unknown => SecuritySeverity::Low,
        };

        Self {
            id: Uuid::new_v4(),
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            event_type: SecurityEventType::DataRead,
            severity,
            user_id: Some(user_id),
            session_id: None,
            project_id: Some(project_id),
            source_ip: None,
            user_agent: None,
            resource: Some(resource),
            action,
            outcome,
            details: HashMap::new(),
            risk_score: 0,
            alert_triggered: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_severity_priority() {
        assert!(SecuritySeverity::Critical.priority() > SecuritySeverity::High.priority());
        assert!(SecuritySeverity::High.priority() > SecuritySeverity::Medium.priority());
        assert!(SecuritySeverity::Medium.priority() > SecuritySeverity::Low.priority());
        assert!(SecuritySeverity::Low.priority() > SecuritySeverity::Info.priority());
    }

    #[test]
    fn test_security_event_creation() {
        let event = SecurityEvent::login_success("user123".to_string(), None);
        assert_eq!(event.event_type, SecurityEventType::LoginSuccess);
        assert_eq!(event.severity, SecuritySeverity::Info);
        assert_eq!(event.outcome, EventOutcome::Success);
        assert_eq!(event.user_id, Some("user123".to_string()));
    }

    #[tokio::test]
    async fn test_audit_logger() {
        let config = AuditConfig::default();
        let mut logger = SecurityAuditLogger::new(config);

        let event = SecurityEvent::login_success("test_user".to_string(), None);
        let result = logger.log_event(event).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_risk_score_calculation() {
        let config = AuditConfig::default();
        let logger = SecurityAuditLogger::new(config);

        let critical_event = SecurityEvent {
            id: Uuid::new_v4(),
            timestamp: 0,
            event_type: SecurityEventType::IntrusionAttempt,
            severity: SecuritySeverity::Critical,
            user_id: None,
            session_id: None,
            project_id: None,
            source_ip: None,
            user_agent: None,
            resource: None,
            action: "Test".to_string(),
            outcome: EventOutcome::Failure,
            details: HashMap::new(),
            risk_score: 0,
            alert_triggered: false,
        };

        let score = logger.calculate_risk_score(&critical_event);
        assert!(score > 90); // Should be high risk
    }
}
