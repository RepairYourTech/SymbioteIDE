//! Explicit capability elevation (#269): a bounded, attributable, durable
//! lease that licenses ONE permission beyond a dispatch's binding access —
//! never beyond the Team's access ceiling — with a reason, an approving
//! authority, a hard expiry (automatic revocation) and sticky manual
//! revocation. Observation is not enforcement; neither is a request: only
//! a decided lease counts, and it elevates exactly one dispatch.
use crate::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// The longest elevation window the Host accepts. Short leases keep the
/// blast radius bounded; longer needs are re-requested.
pub const MAX_ELEVATION_WINDOW_MS: u64 = 900_000;

/// One decided elevation: the request (what was asked and why) plus the
/// authority's decision. `approved: false` records a DENIAL — evidence
/// that the authority saw and refused the ask. Only `approved` leases
/// with no revocation and an unexpired window license anything.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ElevationLease {
    /// The deciding command's id: the lease's durable identity.
    pub id: CommandId,
    pub project_id: ProjectId,
    /// The elevated dispatch and its task — every active lease is
    /// attributable to exactly one dispatch.
    pub task_id: TaskId,
    pub dispatch_id: DispatchId,
    pub permission: Permission,
    /// Why the worker needs this, supplied with the request and recorded
    /// with the decision. Never a secret or an unbounded path.
    pub reason: String,
    /// False records an explicit denial; the remaining fields still
    /// journal who decided and when.
    pub approved: bool,
    pub approved_by: UserId,
    pub decided_at: Timestamp,
    /// For an approval: the hard window edge (automatic revocation —
    /// every check compares against the reading clock, so an expired
    /// lease licenses nothing without any sweeper running). For a
    /// denial: equals `decided_at`.
    pub expires_at: Timestamp,
    /// Sticky manual revocation: once set, the lease is dead regardless
    /// of its window.
    pub revoked_at: Option<Timestamp>,
}

#[derive(Debug)]
pub enum ElevationError {
    InvalidLease,
    WindowExhausted,
}
impl std::fmt::Display for ElevationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for ElevationError {}

impl ElevationLease {
    pub fn validate(&self) -> Result<(), ElevationError> {
        if self.reason.trim().is_empty()
            || self.reason.len() > 1024
            || self.reason.contains('\0')
            || self.reason.chars().any(char::is_control)
        {
            return Err(ElevationError::InvalidLease);
        }
        if self.approved {
            // The window must be positive and within the Host's bound.
            if self.expires_at.0 <= self.decided_at.0
                || self.expires_at.0 - self.decided_at.0 > MAX_ELEVATION_WINDOW_MS
            {
                return Err(ElevationError::WindowExhausted);
            }
        } else if self.expires_at != self.decided_at {
            return Err(ElevationError::InvalidLease);
        }
        if let Some(revoked_at) = self.revoked_at {
            if revoked_at < self.decided_at {
                return Err(ElevationError::InvalidLease);
            }
        }
        Ok(())
    }

    /// Whether this lease licenses its permission at the reading clock.
    /// Automatic revocation is this check: an expired or manually revoked
    /// lease never licenses anything, approved or not.
    pub fn licenses(&self, at: Timestamp) -> bool {
        self.approved && self.revoked_at.is_none() && self.decided_at <= at && at < self.expires_at
    }

    /// The permissions this lease could ever license: exactly its own.
    pub fn elevates(&self) -> BTreeSet<Permission> {
        BTreeSet::from([self.permission.clone()])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lease(approved: bool) -> ElevationLease {
        ElevationLease {
            id: CommandId::new("elevate-1").unwrap(),
            project_id: ProjectId::new("project").unwrap(),
            task_id: TaskId::new("task").unwrap(),
            dispatch_id: DispatchId::new("dispatch").unwrap(),
            permission: Permission::UseCredential,
            reason: "the run must read the operator's configured secret".into(),
            approved,
            approved_by: UserId::new("owner").unwrap(),
            decided_at: Timestamp(1_000),
            expires_at: Timestamp(if approved { 1_000 + 300_000 } else { 1_000 }),
            revoked_at: None,
        }
    }

    #[test]
    fn elevation_windows_are_bounded_and_expiry_is_the_auto_revocation() {
        let approved = lease(true);
        assert!(approved.validate().is_ok());
        assert!(approved.licenses(Timestamp(1_000)));
        assert!(approved.licenses(Timestamp(300_999)));
        // Hard expiry: automatic revocation without any sweeper.
        assert!(!approved.licenses(Timestamp(301_000)));
        // A window past the Host's bound is refused outright.
        let mut overlong = lease(true);
        overlong.expires_at = Timestamp(1_000 + MAX_ELEVATION_WINDOW_MS + 1);
        assert!(matches!(
            overlong.validate(),
            Err(ElevationError::WindowExhausted)
        ));
        // An empty window is not an approval.
        let mut empty = lease(true);
        empty.expires_at = Timestamp(1_000);
        assert!(matches!(
            empty.validate(),
            Err(ElevationError::WindowExhausted)
        ));
    }

    #[test]
    fn denials_license_nothing_and_revocation_is_sticky() {
        let denial = lease(false);
        assert!(denial.validate().is_ok());
        assert!(!denial.licenses(Timestamp(1_001)));
        let mut revoked = lease(true);
        revoked.revoked_at = Some(Timestamp(2_000));
        assert!(revoked.validate().is_ok());
        assert!(!revoked.licenses(Timestamp(2_000)));
        assert!(!revoked.licenses(Timestamp(1_500)));
        // Revocation cannot precede the decision.
        let mut backwards = lease(true);
        backwards.revoked_at = Some(Timestamp(999));
        assert!(matches!(
            backwards.validate(),
            Err(ElevationError::InvalidLease)
        ));
        // A denial with a real window is malformed.
        let mut denying_window = lease(false);
        denying_window.expires_at = Timestamp(2_000);
        assert!(matches!(
            denying_window.validate(),
            Err(ElevationError::InvalidLease)
        ));
    }
}
