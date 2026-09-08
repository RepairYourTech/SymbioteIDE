//! Exact resource-consent checks over trusted Host records. This crate neither
//! establishes user identity nor grants an execution permit or OS sandbox.
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use symbiote_domain::{
    AccessSnapshot, CommandId, HostId, ProjectId, RoleId, RuntimeProfileId, Timestamp, UserId,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrustError {
    InvalidFingerprint,
    InvalidResourceReference,
    InvalidAccess,
    InvalidTime,
    InvalidDocument,
    SnapshotChanged,
    WrongHost,
    Expired,
    Revoked,
    NotYetValid,
    PolicyChanged,
    AccessDenied,
}
impl std::fmt::Display for TrustError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("resource consent check failed")
    }
}
impl std::error::Error for TrustError {}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[serde(try_from = "String", into = "String")]
pub struct Fingerprint(String);
impl Fingerprint {
    pub fn new(value: impl Into<String>) -> Result<Self, TrustError> {
        let value = value.into();
        if value.len() != 64
            || !value
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(TrustError::InvalidFingerprint);
        }
        Ok(Self(value))
    }
    /// Fingerprints exact supplied bytes; does not retain them or resolve dependencies.
    pub fn of(bytes: &[u8]) -> Self {
        Self(format!("{:x}", Sha256::digest(bytes)))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl TryFrom<String> for Fingerprint {
    type Error = TrustError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}
impl From<Fingerprint> for String {
    fn from(value: Fingerprint) -> Self {
        value.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(try_from = "SnapshotWire")]
pub struct ResourceSnapshot {
    pub project_id: ProjectId,
    pub role_id: RoleId,
    pub profile_id: RuntimeProfileId,
    pub host_id: HostId,
    pub resource_ref: String,
    pub fingerprint: Fingerprint,
    pub access: AccessSnapshot,
}
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct SnapshotWire {
    project_id: ProjectId,
    role_id: RoleId,
    profile_id: RuntimeProfileId,
    host_id: HostId,
    resource_ref: String,
    fingerprint: Fingerprint,
    access: AccessSnapshot,
}
impl TryFrom<SnapshotWire> for ResourceSnapshot {
    type Error = TrustError;
    fn try_from(v: SnapshotWire) -> Result<Self, Self::Error> {
        let value = Self {
            project_id: v.project_id,
            role_id: v.role_id,
            profile_id: v.profile_id,
            host_id: v.host_id,
            resource_ref: v.resource_ref,
            fingerprint: v.fingerprint,
            access: v.access,
        };
        value.validate()?;
        Ok(value)
    }
}
impl ResourceSnapshot {
    pub fn validate(&self) -> Result<(), TrustError> {
        if self.resource_ref.is_empty()
            || self.resource_ref.len() > 128
            || !self
                .resource_ref
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.'))
        {
            return Err(TrustError::InvalidResourceReference);
        }
        if self.access.project_id != self.project_id || self.access.roots.len() > 256 {
            return Err(TrustError::InvalidAccess);
        }
        Ok(())
    }
    pub fn parse(input: &str) -> Result<Self, TrustError> {
        parse(input)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(try_from = "ConsentWire")]
pub struct ResourceConsent {
    pub id: CommandId,
    pub snapshot: ResourceSnapshot,
    pub user_id: UserId,
    pub issued_at: Timestamp,
    pub expires_at: Timestamp,
    pub revoked_at: Option<Timestamp>,
}
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ConsentWire {
    id: CommandId,
    snapshot: ResourceSnapshot,
    user_id: UserId,
    issued_at: Timestamp,
    expires_at: Timestamp,
    revoked_at: Option<Timestamp>,
}
impl TryFrom<ConsentWire> for ResourceConsent {
    type Error = TrustError;
    fn try_from(v: ConsentWire) -> Result<Self, Self::Error> {
        let value = Self {
            id: v.id,
            snapshot: v.snapshot,
            user_id: v.user_id,
            issued_at: v.issued_at,
            expires_at: v.expires_at,
            revoked_at: v.revoked_at,
        };
        value.validate()?;
        Ok(value)
    }
}
impl ResourceConsent {
    pub fn validate(&self) -> Result<(), TrustError> {
        self.snapshot.validate()?;
        if self.issued_at.0 >= self.expires_at.0
            || self.expires_at.0 == u64::MAX
            || self.revoked_at.is_some_and(|at| at.0 < self.issued_at.0)
        {
            return Err(TrustError::InvalidTime);
        }
        Ok(())
    }
    pub fn parse(input: &str) -> Result<Self, TrustError> {
        parse(input)
    }
}

fn parse<T: serde::de::DeserializeOwned>(input: &str) -> Result<T, TrustError> {
    if input.len() > 65_536 {
        return Err(TrustError::InvalidDocument);
    }
    // All objects are strict typed structs: serde rejects duplicate and unknown
    // fields directly, including escaped duplicate names and nested AccessSnapshot.
    serde_json::from_str(input).map_err(|_| TrustError::InvalidDocument)
}

/// `consent` must originate from an authenticated, persisted Host decision;
/// `snapshot` must describe the bytes about to load, not a self-asserted prompt.
/// Rechecking here does not make a later unchecked load safe against replacement.
pub fn authorize_load(
    consent: &ResourceConsent,
    snapshot: &ResourceSnapshot,
    current_policy: &AccessSnapshot,
    host: &HostId,
    at: Timestamp,
) -> Result<(), TrustError> {
    consent.validate()?;
    snapshot.validate()?;
    if consent.snapshot != *snapshot {
        return Err(TrustError::SnapshotChanged);
    }
    if snapshot.host_id != *host {
        return Err(TrustError::WrongHost);
    }
    if consent.revoked_at.is_some() {
        return Err(TrustError::Revoked);
    }
    if at.0 < consent.issued_at.0 {
        return Err(TrustError::NotYetValid);
    }
    if at.0 >= consent.expires_at.0 {
        return Err(TrustError::Expired);
    }
    if current_policy.project_id != snapshot.project_id
        || current_policy.policy_revision != snapshot.access.policy_revision
    {
        return Err(TrustError::PolicyChanged);
    }
    if current_policy.roots.len() > 256
        || !snapshot.access.roots.is_subset(&current_policy.roots)
        || !snapshot.access.grants.is_subset(&current_policy.grants)
    {
        return Err(TrustError::AccessDenied);
    }
    Ok(())
}
