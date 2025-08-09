//! # User Models for Symbiote IDE
//!
//! Comprehensive user data models with authentication, preferences, and relationships.
//! Following Week 5-6 Database & Storage Systems implementation plan.

use crate::{UserId, ProjectId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Core user entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    /// Unique user identifier
    pub id: UserId,

    /// User's email address (unique)
    pub email: String,

    /// Display name
    pub name: String,

    /// Optional username (unique)
    pub username: Option<String>,

    /// Profile avatar URL
    pub avatar_url: Option<String>,

    /// User's timezone
    pub timezone: Option<String>,

    /// User's preferred language
    pub language: Option<String>,

    /// Account status
    pub status: UserStatus,

    /// User role
    pub role: UserRole,

    /// Account creation timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,

    /// Last update timestamp
    pub updated_at: chrono::DateTime<chrono::Utc>,

    /// Last login timestamp
    pub last_login_at: Option<chrono::DateTime<chrono::Utc>>,

    /// Email verification status
    pub email_verified: bool,

    /// Two-factor authentication enabled
    pub two_factor_enabled: bool,
}

/// User account status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum UserStatus {
    Active,
    Inactive,
    Suspended,
    PendingVerification,
    Deleted,
}

/// User role in the system
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum UserRole {
    Admin,
    Developer,
    Viewer,
    Guest,
}

/// User authentication information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserAuth {
    /// User ID
    pub user_id: UserId,

    /// Password hash (if using password auth)
    pub password_hash: Option<String>,

    /// Salt for password hashing
    pub password_salt: Option<String>,

    /// OAuth provider information
    pub oauth_providers: Vec<OAuthProvider>,

    /// Two-factor authentication secret
    pub totp_secret: Option<String>,

    /// Recovery codes for 2FA
    pub recovery_codes: Vec<String>,

    /// Failed login attempts
    pub failed_attempts: u32,

    /// Account locked until
    pub locked_until: Option<chrono::DateTime<chrono::Utc>>,

    /// Last password change
    pub password_changed_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// OAuth provider information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthProvider {
    /// Provider name (github, google, etc.)
    pub provider: String,

    /// Provider user ID
    pub provider_user_id: String,

    /// Provider email
    pub provider_email: Option<String>,

    /// Access token (encrypted)
    pub access_token: Option<String>,

    /// Refresh token (encrypted)
    pub refresh_token: Option<String>,

    /// Token expiration
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,

    /// When this provider was linked
    pub linked_at: chrono::DateTime<chrono::Utc>,
}

/// User preferences and settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPreferences {
    /// User ID
    pub user_id: UserId,

    /// IDE theme preference
    pub theme: String,

    /// Font family
    pub font_family: String,

    /// Font size
    pub font_size: u32,

    /// Editor preferences
    pub editor_preferences: EditorPreferences,

    /// AI assistant preferences
    pub ai_preferences: AIPreferences,

    /// Notification preferences
    pub notification_preferences: NotificationPreferences,

    /// Custom key-value settings
    pub custom_settings: HashMap<String, serde_json::Value>,

    /// Last updated
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Editor-specific preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorPreferences {
    /// Tab size
    pub tab_size: u32,

    /// Use spaces instead of tabs
    pub use_spaces: bool,

    /// Word wrap enabled
    pub word_wrap: bool,

    /// Show line numbers
    pub show_line_numbers: bool,

    /// Show minimap
    pub show_minimap: bool,

    /// Auto-save enabled
    pub auto_save: bool,

    /// Auto-save delay in seconds
    pub auto_save_delay: u32,

    /// Vim mode enabled
    pub vim_mode: bool,
}

/// AI assistant preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIPreferences {
    /// Default AI model
    pub default_model: String,

    /// AI aggressiveness level (0-100)
    pub aggressiveness: u32,

    /// Auto-suggestions enabled
    pub auto_suggestions: bool,

    /// Code completion enabled
    pub code_completion: bool,

    /// AI explanations enabled
    pub explanations: bool,

    /// Preferred AI providers
    pub preferred_providers: Vec<String>,
}

/// Notification preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationPreferences {
    /// Email notifications enabled
    pub email_enabled: bool,

    /// Push notifications enabled
    pub push_enabled: bool,

    /// Desktop notifications enabled
    pub desktop_enabled: bool,

    /// Notification types to receive
    pub notification_types: Vec<String>,

    /// Quiet hours start
    pub quiet_hours_start: Option<chrono::NaiveTime>,

    /// Quiet hours end
    pub quiet_hours_end: Option<chrono::NaiveTime>,
}

/// User session information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSession {
    /// Session ID
    pub id: String,

    /// User ID
    pub user_id: UserId,

    /// Session token
    pub token: String,

    /// IP address
    pub ip_address: String,

    /// User agent
    pub user_agent: String,

    /// Session created at
    pub created_at: chrono::DateTime<chrono::Utc>,

    /// Session expires at
    pub expires_at: chrono::DateTime<chrono::Utc>,

    /// Last activity
    pub last_activity: chrono::DateTime<chrono::Utc>,

    /// Session is active
    pub active: bool,
}

/// User project membership
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProjectMembership {
    /// User ID
    pub user_id: UserId,

    /// Project ID
    pub project_id: ProjectId,

    /// Role in the project
    pub role: ProjectRole,

    /// Permissions
    pub permissions: Vec<String>,

    /// When user joined the project
    pub joined_at: chrono::DateTime<chrono::Utc>,

    /// Invitation status
    pub status: MembershipStatus,
}

/// Project-specific roles
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProjectRole {
    Owner,
    Admin,
    Developer,
    Viewer,
}

/// Membership status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MembershipStatus {
    Active,
    Invited,
    Suspended,
    Left,
}

impl Default for UserPreferences {
    fn default() -> Self {
        Self {
            user_id: UserId::new(),
            theme: "dark".to_string(),
            font_family: "JetBrains Mono".to_string(),
            font_size: 14,
            editor_preferences: EditorPreferences::default(),
            ai_preferences: AIPreferences::default(),
            notification_preferences: NotificationPreferences::default(),
            custom_settings: HashMap::new(),
            updated_at: chrono::Utc::now(),
        }
    }
}

impl Default for EditorPreferences {
    fn default() -> Self {
        Self {
            tab_size: 4,
            use_spaces: true,
            word_wrap: false,
            show_line_numbers: true,
            show_minimap: true,
            auto_save: true,
            auto_save_delay: 5,
            vim_mode: false,
        }
    }
}

impl Default for AIPreferences {
    fn default() -> Self {
        Self {
            default_model: "gpt-4".to_string(),
            aggressiveness: 50,
            auto_suggestions: true,
            code_completion: true,
            explanations: true,
            preferred_providers: vec!["openai".to_string()],
        }
    }
}

impl Default for NotificationPreferences {
    fn default() -> Self {
        Self {
            email_enabled: true,
            push_enabled: true,
            desktop_enabled: true,
            notification_types: vec![
                "project_updates".to_string(),
                "security_alerts".to_string(),
                "system_notifications".to_string(),
            ],
            quiet_hours_start: None,
            quiet_hours_end: None,
        }
    }
}

impl User {
    /// Create a new user
    pub fn new(email: String, name: String) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: UserId::new(),
            email,
            name,
            username: None,
            avatar_url: None,
            timezone: None,
            language: None,
            status: UserStatus::PendingVerification,
            role: UserRole::Developer,
            created_at: now,
            updated_at: now,
            last_login_at: None,
            email_verified: false,
            two_factor_enabled: false,
        }
    }

    /// Check if user is active
    pub fn is_active(&self) -> bool {
        self.status == UserStatus::Active
    }

    /// Check if user is admin
    pub fn is_admin(&self) -> bool {
        self.role == UserRole::Admin
    }

    /// Update last login time
    pub fn update_last_login(&mut self) {
        self.last_login_at = Some(chrono::Utc::now());
        self.updated_at = chrono::Utc::now();
    }
}

impl UserAuth {
    /// Create new user auth record
    pub fn new(user_id: UserId) -> Self {
        Self {
            user_id,
            password_hash: None,
            password_salt: None,
            oauth_providers: Vec::new(),
            totp_secret: None,
            recovery_codes: Vec::new(),
            failed_attempts: 0,
            locked_until: None,
            password_changed_at: None,
        }
    }

    /// Check if account is locked
    pub fn is_locked(&self) -> bool {
        if let Some(locked_until) = self.locked_until {
            chrono::Utc::now() < locked_until
        } else {
            false
        }
    }

    /// Increment failed attempts
    pub fn increment_failed_attempts(&mut self) {
        self.failed_attempts += 1;

        // Lock account after 5 failed attempts for 30 minutes
        if self.failed_attempts >= 5 {
            self.locked_until = Some(chrono::Utc::now() + chrono::Duration::minutes(30));
        }
    }

    /// Reset failed attempts
    pub fn reset_failed_attempts(&mut self) {
        self.failed_attempts = 0;
        self.locked_until = None;
    }
}
