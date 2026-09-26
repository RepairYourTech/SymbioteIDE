//! Durable task leases with monotonic fencing tokens. A lease binds one
//! Running task's dispatch to a host session so a partitioned worker cannot
//! mutate canonical state after a newer owner exists. Leases authorize
//! nothing by themselves: dispatch contracts and Host authorization stay
//! upstream. This slice covers acquisition/renewal/expiry/stale recovery of
//! leases and the scheduler-ready projection; readiness events, collision
//! consumption and dead-letter delivery remain pending #95 acceptance.
use crate::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const LEASE_VERSION: u32 = 1;
/// Lease durations are bounded so a partition cannot hold a task forever.
pub const MIN_LEASE_MS: u64 = 1_000;
pub const MAX_LEASE_MS: u64 = 3_600_000;
/// Window used when a Start transition acquires the governing lease.
pub const DEFAULT_START_LEASE_MS: u64 = 300_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum LeaseState {
    Held,
    Expired,
    Released,
    Fenced,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TaskLease {
    pub version: u32,
    pub project_id: ProjectId,
    pub task_id: TaskId,
    pub stream_id: ChangeStreamId,
    pub dispatch_id: DispatchId,
    pub host_id: HostId,
    /// Monotonic per-task token: every acquisition increments it, and a
    /// worker presenting an older token is fenced out of later mutations.
    pub fencing_token: u64,
    pub state: LeaseState,
    pub acquired_at: Timestamp,
    pub expires_at: Timestamp,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LeaseError {
    InvalidRequest,
    TaskState,
    RevisionConflict,
    StillHeld,
    NotHeld,
    Fenced,
    ResourceLimit,
}
impl std::fmt::Display for LeaseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "lease rejected: {self:?}")
    }
}
impl std::error::Error for LeaseError {}

/// A scheduler query result: one startable pre-dispatch task (`Ready`, or
/// `Assigned` once the Host bound its canonical Role) whose blocking dependency
/// targets are all Completed and whose Change Stream is Active, with an
/// explanation of every constraint considered. Scheduling decisions must be
/// explainable from recorded state, so the reason is part of the record, not a
/// log line.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SchedulableTask {
    pub project_id: ProjectId,
    pub task_id: TaskId,
    pub stream_id: ChangeStreamId,
    pub role_id: RoleId,
    pub reason: SchedulableReason,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SchedulableReason {
    /// Ready with no dependency edges at all.
    NoBlockingDependencies,
    /// Ready and every blocking dependency target is Completed.
    DependenciesSatisfied,
}

/// What one considered Task is waiting on before it can run, from canonical
/// rows. This is the readiness answer: the gates that are not satisfied yet,
/// each naming the other side and the state it is in. A start candidate with an
/// empty list has nothing holding it, which is why the entry is published even
/// when the list is empty — "nothing is in the way" is an answer, not an
/// absence.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TaskReadiness {
    pub task: GraphTaskRef,
    /// The canonical state the task is in now, so a reader can tell a start
    /// candidate from one still queued or held.
    pub state: TaskState,
    /// Unsatisfied gates holding this task, in canonical order. Empty when
    /// nothing holds it.
    pub waiting_on: Vec<TaskGate>,
    /// Gates recorded for this task in total, satisfied or not. Published
    /// because "no gates at all" and "every gate satisfied" are different
    /// reasons to be schedulable, and only the count tells them apart.
    pub recorded_gates: usize,
}

/// The one scheduling surface for a Project: what can start now and why, what
/// every task is waiting on, and the DAG answers — progress, the remaining
/// closure, the tasks each open task waits on, and the critical path — beside
/// the readiness list rather than behind a second read.
///
/// The two gate lists answer different questions and are not duplicates.
/// `readiness` is the evidence: every considered task and what holds it.
/// `blocked` is the scheduling verdict for start candidates, naming the one
/// constraint that decides (stream state, a held lease, an unresolved
/// dependency) so a scheduler needs one field to branch on.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SchedulingProjection {
    /// The Project this answer is for.
    pub project_id: ProjectId,
    /// The instant the lease half of this answer was judged at, so a caller
    /// can tell a stale `stream_leased` from a current one.
    pub considered_at: Timestamp,
    pub schedulable: Vec<SchedulableTask>,
    /// Start candidates that cannot run, each with the stated blocker.
    pub blocked: Vec<BlockedTask>,
    /// Every considered task and the gates holding it. This is the one place
    /// the per-task gates are published: what an open task is blocked on is its
    /// `waiting_on` here, so the same gate is never carried twice.
    pub readiness: Vec<TaskReadiness>,
    /// Progress counted from canonical states, the remaining closure and the
    /// critical path. Carried here so the DAG answers are reachable from the
    /// scheduling surface, and `dag.progress` is the only progress the answer
    /// publishes.
    pub dag: TaskGraphReport,
}

impl SchedulingProjection {
    /// Every Project this answer names besides its own — the set a caller must
    /// be able to read before the answer is served, so a record with an unnamed
    /// other side is a reason the reader cannot act on.
    ///
    /// It covers the whole answer, not the graph half: the remaining closure
    /// and the critical path name some Projects, and the readiness list names
    /// others, because a task that has left the work can still be waiting on
    /// work in another Project and the chain does not walk closed tasks. A set
    /// built from the graph alone would drop exactly those gates, and the Host
    /// authorizes over this one so there is nowhere to look by accident.
    pub fn referenced_projects(&self) -> BTreeSet<ProjectId> {
        let mut projects = self.dag.referenced_projects();
        let mut note = |task: &GraphTaskRef| {
            if task.project_id != self.project_id {
                projects.insert(task.project_id.clone());
            }
        };
        for entry in &self.readiness {
            note(&entry.task);
            for gate in &entry.waiting_on {
                note(&gate.target);
            }
        }
        projects
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BlockedTask {
    pub project_id: ProjectId,
    pub task_id: TaskId,
    pub stream_id: ChangeStreamId,
    pub reason: BlockedReason,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum BlockedReason {
    /// A blocking dependency target is not Completed.
    DependencyUnresolved,
    /// The owning Change Stream is not Active (collided, integrated...).
    StreamUnsafe,
    /// Another task holds a lease on this task's stream — serialized by policy.
    StreamLeased,
    /// The bounded answer did not read this task, so nothing can be said about
    /// its gates. It is reported rather than dropped because a start candidate
    /// that vanishes from a scheduling answer reads as work that is not
    /// waiting, and this answer does not know. `progress.partial` is true
    /// whenever this reason appears.
    NotConsidered,
}

/// Pure lease arithmetic shared by the store and any future scheduler loop.
pub fn lease_expiry(acquired_at: Timestamp, duration_ms: u64) -> Result<Timestamp, LeaseError> {
    if !(MIN_LEASE_MS..=MAX_LEASE_MS).contains(&duration_ms) {
        return Err(LeaseError::InvalidRequest);
    }
    let expires = acquired_at
        .0
        .checked_add(duration_ms)
        .ok_or(LeaseError::InvalidRequest)?;
    Ok(Timestamp(expires))
}

impl TaskLease {
    /// Structural validity: version, nonzero token, and a window inside the
    /// declared bounds. Replay relies on this, so a journal payload with a
    /// window acquisition itself would reject fails here too.
    pub fn validate_shape(&self) -> Result<(), LeaseError> {
        if self.version != LEASE_VERSION
            || self.fencing_token == 0
            || lease_expiry(
                self.acquired_at,
                self.expires_at.0.saturating_sub(self.acquired_at.0),
            )
            .is_err()
        {
            return Err(LeaseError::InvalidRequest);
        }
        Ok(())
    }
}

/// True when the lease is held and unexpired at `now`. Fenced/released/expired
/// leases never hold.
pub fn lease_holds(lease: &TaskLease, now: Timestamp) -> bool {
    lease.state == LeaseState::Held && lease.expires_at.0 > now.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lease_expiry_bounds_are_enforced() {
        let now = Timestamp(10_000);
        assert_eq!(
            lease_expiry(now, MIN_LEASE_MS - 1),
            Err(LeaseError::InvalidRequest)
        );
        assert_eq!(
            lease_expiry(now, MAX_LEASE_MS + 1),
            Err(LeaseError::InvalidRequest)
        );
        assert_eq!(lease_expiry(now, MIN_LEASE_MS), Ok(Timestamp(11_000)));
        assert_eq!(
            lease_expiry(now, MAX_LEASE_MS),
            Ok(Timestamp(10_000 + MAX_LEASE_MS))
        );
    }

    #[test]
    fn only_held_unexpired_leases_hold() {
        let held = TaskLease {
            version: LEASE_VERSION,
            project_id: ProjectId::new("p").unwrap(),
            task_id: TaskId::new("t").unwrap(),
            stream_id: ChangeStreamId::new("s").unwrap(),
            dispatch_id: DispatchId::new("d").unwrap(),
            host_id: HostId::new("h").unwrap(),
            fencing_token: 1,
            state: LeaseState::Held,
            acquired_at: Timestamp(1_000),
            expires_at: Timestamp(2_000),
        };
        assert!(lease_holds(&held, Timestamp(1_999)));
        assert!(!lease_holds(&held, Timestamp(2_000)));
        let expired = TaskLease {
            state: LeaseState::Expired,
            ..held
        };
        assert!(!lease_holds(&expired, Timestamp(1_500)));
    }
}
