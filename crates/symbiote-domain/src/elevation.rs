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

/// A worker's ask for one permission beyond its binding. Observation
/// only: filing this never licenses anything. Only a later
/// [`ElevationLease`] decided by the owner can.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ElevationRequest {
    /// The requesting command's id: the ask's durable identity.
    pub id: CommandId,
    pub project_id: ProjectId,
    pub task_id: TaskId,
    pub dispatch_id: DispatchId,
    pub permission: Permission,
    /// Why the worker needs this. Never a secret or an unbounded path.
    pub reason: String,
    pub requested_at: Timestamp,
}

impl ElevationRequest {
    pub fn validate(&self) -> Result<(), ElevationError> {
        if self.reason.trim().is_empty()
            || self.reason.len() > 1024
            || self.reason.contains('\0')
            || self.reason.chars().any(char::is_control)
        {
            return Err(ElevationError::InvalidLease);
        }
        Ok(())
    }
}

/// What one decided elevation is doing at a reading clock. Standing is not
/// stored: it is the lease read at the clock the ledger was served, so a
/// lease that was live a moment ago and is dead now says so without anyone
/// having to run a sweeper to discover it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum ElevationStanding {
    /// Approved, unrevoked, and inside its window: this lease licenses its
    /// permission for its dispatch at the reading clock.
    Live,
    /// Approved, and the window has closed. Expiry IS the automatic
    /// revocation, and this is the fact that says it happened.
    Expired,
    /// Manually revoked. Dead regardless of any window, and never undone.
    Revoked,
    /// A denial: evidence the authority saw the ask and refused it. It never
    /// licensed anything and it never will.
    Denied,
}

/// One decided elevation as a reader meets it: the whole lease, and what it is
/// doing right now.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DecidedElevation {
    pub lease: ElevationLease,
    pub standing: ElevationStanding,
}

impl DecidedElevation {
    /// The lease read at `at`. The order matters: a denial is a denial
    /// whether or not it was revoked, and a revocation outranks a window
    /// because a revoked lease is dead however long its window ran.
    pub fn read(lease: ElevationLease, at: Timestamp) -> Self {
        let standing = if !lease.approved {
            ElevationStanding::Denied
        } else if lease.revoked_at.is_some() {
            ElevationStanding::Revoked
        } else if lease.licenses(at) {
            ElevationStanding::Live
        } else {
            ElevationStanding::Expired
        };
        Self { lease, standing }
    }
}

/// Everything one Project has asked for and everything it has been granted:
/// the one read an operator needs before answering an ask, and the one read
/// they need before taking an answer back.
///
/// It publishes evidence and decisions, never authority. A live lease here
/// says a permission is licensed for one dispatch; it does not widen a
/// binding, and it grants nothing to whoever reads it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ElevationLedger {
    pub project_id: ProjectId,
    /// The clock the standings were judged at, carried so a reader can tell
    /// how old this answer is rather than re-deriving it from its own.
    pub read_at: Timestamp,
    /// The asks, in the order they were filed. An ask licenses nothing at
    /// any standing; it is here so the decision beside it can be read next
    /// to the request it answers.
    pub requests: Vec<ElevationRequest>,
    pub leases: Vec<DecidedElevation>,
}

#[cfg(test)]
mod tests {
    /// The standing is a READ at a clock, not a fact the record carries: the
    /// same lease is live, expired or revoked depending only on when it is
    /// asked about, and the two orderings that could be got wrong are a
    /// revocation outranking a window and a denial outranking both.
    #[test]
    fn a_standing_is_read_at_a_clock_and_never_stored() {
        let live = lease(true);
        assert_eq!(
            DecidedElevation::read(live.clone(), Timestamp(1_000)).standing,
            ElevationStanding::Live
        );
        assert_eq!(
            DecidedElevation::read(live.clone(), Timestamp(1_000 + 299_999)).standing,
            ElevationStanding::Live
        );
        assert_eq!(
            DecidedElevation::read(live.clone(), Timestamp(301_000)).standing,
            ElevationStanding::Expired
        );
        // A lease read BEFORE it was decided is not live: the window has not
        // opened yet, and saying "expired" is the honest half of that — it
        // licenses nothing at that clock either way.
        assert_eq!(
            DecidedElevation::read(live.clone(), Timestamp(999)).standing,
            ElevationStanding::Expired
        );
        // A revocation outranks a window: dead at once, and dead inside the
        // window it was granted for, which is the whole point of one.
        let mut revoked = lease(true);
        revoked.revoked_at = Some(Timestamp(1_500));
        assert_eq!(
            DecidedElevation::read(revoked.clone(), Timestamp(1_500)).standing,
            ElevationStanding::Revoked
        );
        assert_eq!(
            DecidedElevation::read(revoked, Timestamp(1_600)).standing,
            ElevationStanding::Revoked
        );
        // A denial is a denial whatever else is true of it.
        let mut denied = lease(false);
        denied.revoked_at = Some(Timestamp(1_200));
        assert_eq!(
            DecidedElevation::read(denied, Timestamp(9_999)).standing,
            ElevationStanding::Denied
        );
        // The lease itself is carried whole beside the standing, so a reader
        // never has to re-derive a fact the record already holds.
        assert_eq!(
            DecidedElevation::read(live, Timestamp(1_000))
                .lease
                .expires_at,
            Timestamp(1_000 + 300_000)
        );
    }

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

    #[test]
    fn an_elevation_request_is_evidence_and_has_no_license() {
        let request = ElevationRequest {
            id: CommandId::new("ask-1").unwrap(),
            project_id: ProjectId::new("project").unwrap(),
            task_id: TaskId::new("task").unwrap(),
            dispatch_id: DispatchId::new("dispatch").unwrap(),
            permission: Permission::UseCredential,
            reason: "the run must read the operator's configured secret".into(),
            requested_at: Timestamp(1_000),
        };
        assert!(request.validate().is_ok());
        // There is no licenses() on a request: only a decided lease can.
        let mut empty = request.clone();
        empty.reason.clear();
        assert!(matches!(
            empty.validate(),
            Err(ElevationError::InvalidLease)
        ));
        let mut control = request;
        control.reason = "line\nbreak".into();
        assert!(matches!(
            control.validate(),
            Err(ElevationError::InvalidLease)
        ));
    }
}
