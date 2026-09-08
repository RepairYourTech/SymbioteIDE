use super::*;

/// Reserved command-ID namespace for system sweeps. Callers may not lease,
/// release, or otherwise journal under it, so a poisoned ID can never pre-
/// commit bytes that block the sweep.
pub(super) const RESERVED_LEASE_PREFIX: &str = "symbiote-sweep-";

pub(super) const MIGRATION_V8: &str = "CREATE TABLE task_leases (
 task_id TEXT PRIMARY KEY REFERENCES tasks(id),
 project_id TEXT NOT NULL REFERENCES projects(id),
 stream_id TEXT NOT NULL,
 dispatch_id TEXT NOT NULL,
 host_id TEXT NOT NULL,
 fencing_token INTEGER NOT NULL CHECK(fencing_token>0),
 state TEXT NOT NULL CHECK(state IN ('held','expired','released','fenced')),
 acquired_at INTEGER NOT NULL CHECK(acquired_at>=0),
 expires_at INTEGER NOT NULL CHECK(expires_at>=0),
 body TEXT NOT NULL CHECK(json_valid(body))
) STRICT;
CREATE INDEX task_leases_stream ON task_leases(stream_id);
PRAGMA user_version=8;";

pub(super) fn event(lease: &TaskLease, actor: &UserId, at: Timestamp) -> EventPayload {
    EventPayload::TaskLeased {
        lease: Box::new(lease.clone()),
        actor: actor.clone(),
        at,
    }
}

fn parse_state(text: &str) -> Option<LeaseState> {
    match text {
        "held" => Some(LeaseState::Held),
        "expired" => Some(LeaseState::Expired),
        "released" => Some(LeaseState::Released),
        "fenced" => Some(LeaseState::Fenced),
        _ => None,
    }
}

pub(super) type LeaseRow = (
    String,
    String,
    String,
    String,
    u64,
    String,
    u64,
    u64,
    String,
);

pub(super) fn read(connection: &Connection, task: &TaskId) -> Result<Option<TaskLease>> {
    let row: Option<LeaseRow> = connection
        .query_row(
            "SELECT project_id,stream_id,dispatch_id,host_id,fencing_token,state,acquired_at,expires_at,body FROM task_leases WHERE task_id=?1",
            [task.as_str()],
            |r| {
                Ok((
                    r.get(0)?,
                    r.get(1)?,
                    r.get(2)?,
                    r.get(3)?,
                    sql_u64(r, 4)?,
                    r.get(5)?,
                    sql_u64(r, 6)?,
                    sql_u64(r, 7)?,
                    r.get(8)?,
                ))
            },
        )
        .optional()?;
    row.map(
        |(project, stream, dispatch, host, token, state, acquired, expires, body)| {
            let lease: TaskLease = serde_json::from_str(&body)?;
            let indexed_ok = lease.project_id.as_str() == project
                && lease.stream_id.as_str() == stream
                && lease.dispatch_id.as_str() == dispatch
                && lease.host_id.as_str() == host
                && lease.fencing_token == token
                && parse_state(&state) == Some(lease.state)
                && lease.acquired_at.0 == acquired
                && lease.expires_at.0 == expires;
            if !indexed_ok {
                return Err(StoreError::Integrity(
                    "lease indexed state differs from body".into(),
                ));
            }
            Ok(lease)
        },
    )
    .transpose()
}

impl Store {
    /// Acquires or renews the lease for a Running task's dispatch. The
    /// fencing token increments monotonically per task; a caller renewing
    /// with a stale token is fenced. Callers authenticate the Host actor;
    /// dispatch validity is checked against the stored task.
    #[allow(clippy::too_many_arguments)]
    pub fn acquire_lease(
        &mut self,
        command_id: CommandId,
        task: TaskId,
        dispatch_id: DispatchId,
        host_id: HostId,
        duration_ms: u64,
        actor: UserId,
        at: Timestamp,
    ) -> Result<Receipt> {
        let expires_at = lease_expiry(at, duration_ms).map_err(|_| StoreError::InvalidLease)?;
        if at.0 > i64::MAX as u64 {
            return Err(StoreError::InvalidLease);
        }
        if command_id.as_str().starts_with(RESERVED_LEASE_PREFIX) {
            return Err(StoreError::InvalidLease);
        }
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let task_record = read_task(&transaction, &task)?;
        let stored_dispatch = task_record
            .current_dispatch()
            .ok_or(StoreError::InvalidLease)?
            .clone();
        if stored_dispatch.id() != &dispatch_id || stored_dispatch.contract().host_id() != &host_id
        {
            return Err(StoreError::InvalidLease);
        }
        // Only actively dispatched work may hold a lease.
        if task_record.state() != &TaskState::Running {
            return Err(StoreError::LeaseConflict(LeaseError::TaskState));
        }
        let existing = read(&transaction, &task)?;
        let fencing_token = match &existing {
            None => 1,
            Some(previous) => {
                // A held, unexpired lease can only be renewed by its own
                // dispatch on the same host with the same token generation.
                if lease_holds(previous, at) {
                    if previous.dispatch_id != dispatch_id || previous.host_id != host_id {
                        return Err(StoreError::LeaseConflict(LeaseError::StillHeld));
                    }
                    previous.fencing_token
                } else {
                    previous
                        .fencing_token
                        .checked_add(1)
                        .ok_or(StoreError::LeaseConflict(LeaseError::ResourceLimit))?
                }
            }
        };
        let stream_id = stored_dispatch.contract().stream_id().clone();
        let lease = TaskLease {
            version: LEASE_VERSION,
            project_id: task_record.project_id().clone(),
            task_id: task.clone(),
            stream_id,
            dispatch_id,
            host_id,
            fencing_token,
            state: LeaseState::Held,
            acquired_at: at,
            expires_at,
        };
        lease
            .validate_shape()
            .map_err(|_| StoreError::InvalidLease)?;
        let payload = event(&lease, &actor, at);
        let request = serde_json::to_string(&payload)?;
        if let Some(receipt) = replay(&transaction, &command_id, &request)? {
            return Ok(receipt);
        }
        let body = serde_json::to_string(&lease)?;
        transaction.execute(
            "INSERT INTO task_leases(task_id,project_id,stream_id,dispatch_id,host_id,fencing_token,state,acquired_at,expires_at,body) VALUES (?1,?2,?3,?4,?5,?6,'held',?7,?8,?9)
             ON CONFLICT(task_id) DO UPDATE SET dispatch_id=excluded.dispatch_id, host_id=excluded.host_id, fencing_token=excluded.fencing_token, state='held', acquired_at=excluded.acquired_at, expires_at=excluded.expires_at, body=excluded.body",
            params![
                lease.task_id.as_str(),
                lease.project_id.as_str(),
                lease.stream_id.as_str(),
                lease.dispatch_id.as_str(),
                lease.host_id.as_str(),
                lease.fencing_token as i64,
                at.0 as i64,
                expires_at.0 as i64,
                body
            ],
        )?;
        let receipt = append(
            &transaction,
            task_record.project_id(),
            &command_id,
            Revision(0),
            &request,
            &payload,
        )?;
        transaction.commit()?;
        Ok(receipt)
    }

    /// Releases a held lease. Only the holding dispatch on its host may
    /// release; the state transition is terminal for that token generation.
    pub fn release_lease(
        &mut self,
        command_id: CommandId,
        task: TaskId,
        dispatch_id: DispatchId,
        fencing_token: u64,
        actor: UserId,
        at: Timestamp,
    ) -> Result<Receipt> {
        if at.0 > i64::MAX as u64 {
            return Err(StoreError::InvalidLease);
        }
        if command_id.as_str().starts_with(RESERVED_LEASE_PREFIX) {
            return Err(StoreError::InvalidLease);
        }
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let existing = read(&transaction, &task)?.ok_or(StoreError::NotFound)?;
        let project = existing.project_id.clone();
        if existing.dispatch_id != dispatch_id || existing.fencing_token != fencing_token {
            return Err(StoreError::LeaseConflict(LeaseError::Fenced));
        }
        if existing.state != LeaseState::Held || !lease_holds(&existing, at) {
            // An expired lease is terminal for its generation: the sweep owns
            // its expiration record, and a late release must not erase it.
            // A retry of the original release replays below first.
            let mut replayed_lease = existing;
            replayed_lease.state = LeaseState::Released;
            let payload = event(&replayed_lease, &actor, at);
            let request = serde_json::to_string(&payload)?;
            if let Some(receipt) = replay(&transaction, &command_id, &request)? {
                return Ok(receipt);
            }
            return Err(StoreError::LeaseConflict(LeaseError::NotHeld));
        }
        let mut lease = existing;
        lease.state = LeaseState::Released;
        let payload = event(&lease, &actor, at);
        let request = serde_json::to_string(&payload)?;
        if let Some(receipt) = replay(&transaction, &command_id, &request)? {
            return Ok(receipt);
        }
        transaction.execute(
            "UPDATE task_leases SET state='released', body=?2 WHERE task_id=?1",
            params![task.as_str(), serde_json::to_string(&lease)?],
        )?;
        let receipt = append(
            &transaction,
            &project,
            &command_id,
            Revision(0),
            &request,
            &payload,
        )?;
        transaction.commit()?;
        Ok(receipt)
    }

    /// Transitions every held-but-unexpired-at-write lease whose expiry has
    /// passed into `expired`, journaling each transition. Idempotent: already
    /// expired leases are left alone.
    pub fn expire_stale_leases(
        &mut self,
        actor: UserId,
        at: Timestamp,
    ) -> Result<Vec<(TaskId, u64)>> {
        if at.0 > i64::MAX as u64 {
            return Err(StoreError::InvalidLease);
        }
        let mut expired: Vec<(TaskId, u64)> = Vec::new();
        loop {
            let transaction = self
                .connection
                .transaction_with_behavior(TransactionBehavior::Immediate)?;
            let mut statement = transaction.prepare(
                "SELECT task_id, project_id, stream_id, dispatch_id, host_id, fencing_token, acquired_at, expires_at, body FROM task_leases WHERE state='held' AND expires_at<=?1 ORDER BY task_id",
            )?;
            let rows = statement.query_map(params![at.0 as i64], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, String>(4)?,
                    sql_u64(r, 5)?,
                    sql_u64(r, 6)?,
                    sql_u64(r, 7)?,
                    r.get::<_, String>(8)?,
                ))
            })?;
            let mut stale: Vec<TaskLease> = Vec::new();
            for row in rows {
                let (task, project, stream, dispatch, host, token, acquired, expires, body) = row?;
                let lease: TaskLease = serde_json::from_str(&body)?;
                let indexed_ok = lease.task_id.as_str() == task
                    && lease.project_id.as_str() == project
                    && lease.stream_id.as_str() == stream
                    && lease.dispatch_id.as_str() == dispatch
                    && lease.host_id.as_str() == host
                    && lease.fencing_token == token
                    && lease.acquired_at.0 == acquired
                    && lease.expires_at.0 == expires;
                if !indexed_ok {
                    return Err(StoreError::Integrity(
                        "lease indexed state differs from body".into(),
                    ));
                }
                stale.push(lease);
            }
            drop(statement);
            if stale.is_empty() {
                transaction.commit()?;
                break;
            }
            let mut committed_any = false;
            for lease in stale {
                let mut expired_lease = lease.clone();
                expired_lease.state = LeaseState::Expired;
                let payload = event(&expired_lease, &actor, at);
                let request = serde_json::to_string(&payload)?;
                let expiry_command = CommandId::new(format!(
                    "{}{}-{}",
                    RESERVED_LEASE_PREFIX,
                    lease.task_id.as_str(),
                    lease.fencing_token
                ))
                .map_err(|_| StoreError::InvalidLease)?;
                let replayed = replay(&transaction, &expiry_command, &request)?;
                if replayed.is_none() {
                    transaction.execute(
                        "UPDATE task_leases SET state='expired', body=?2 WHERE task_id=?1",
                        params![
                            lease.task_id.as_str(),
                            serde_json::to_string(&expired_lease)?
                        ],
                    )?;
                    append(
                        &transaction,
                        &lease.project_id,
                        &expiry_command,
                        Revision(0),
                        &request,
                        &payload,
                    )?;
                    expired.push((lease.task_id.clone(), lease.fencing_token));
                    committed_any = true;
                }
            }
            transaction.commit()?;
            if !committed_any {
                break;
            }
        }
        Ok(expired)
    }

    // Fencing an older generation is implicit: every post-terminal
    // acquisition takes the next token and the replayed state machine refuses
    // any transition that is not strictly increasing. There is deliberately no
    // externally callable fence mutation: unjournaled materialized changes
    // would fail startup audit (learned from the #490 review).

    pub fn lease_command_timestamp(&self, id: &CommandId) -> Result<Option<Timestamp>> {
        let body: Option<String> = self
            .connection
            .query_row(
                "SELECT payload FROM journal WHERE command_id=?1",
                [id.as_str()],
                |r| r.get(0),
            )
            .optional()?;
        match body {
            None => Ok(None),
            Some(body) => match serde_json::from_str::<EventPayload>(&body)? {
                EventPayload::TaskLeased { at, .. } => Ok(Some(at)),
                _ => Err(StoreError::IdempotencyConflict),
            },
        }
    }

    pub fn task_lease(&self, task: &TaskId) -> Result<TaskLease> {
        read(&self.connection, task)?.ok_or(StoreError::NotFound)
    }

    /// Reads the current fencing token for a task, if any lease row exists.
    /// Recovery flows use this to fence superseded generations.
    pub fn lease_fencing_token(&self, task: &TaskId) -> Result<Option<u64>> {
        Ok(read(&self.connection, task)?.map(|lease| lease.fencing_token))
    }

    /// The scheduler's explainable projection over Ready tasks: dependency
    /// edges (#94) and Change Stream state decide, and every task carries the
    /// reason it is schedulable or blocked. Bounded to the dependency cap.
    pub fn scheduling_projection(&self, now: Timestamp) -> Result<SchedulingProjection> {
        let mut statement = self
            .connection
            .prepare("SELECT id, project_id, stream_id, role_id, body FROM tasks WHERE id IN (SELECT id FROM tasks WHERE json_extract(body,'$.state')='ready') ORDER BY id")?;
        let rows = statement.query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, String>(4)?,
            ))
        })?;
        let mut schedulable = Vec::new();
        let mut blocked = Vec::new();
        for row in rows {
            let (task, project, stream, role, body) = row?;
            let record: Task = serde_json::from_str(&body)?;
            if record.state() != &TaskState::Ready {
                continue;
            }
            let project_id = ProjectId::new(&project)
                .map_err(|_| StoreError::Integrity("bad task project".into()))?;
            let task_id =
                TaskId::new(&task).map_err(|_| StoreError::Integrity("bad task id".into()))?;
            let stream_id = ChangeStreamId::new(&stream)
                .map_err(|_| StoreError::Integrity("bad task stream".into()))?;
            let role_id =
                RoleId::new(&role).map_err(|_| StoreError::Integrity("bad task role".into()))?;
            // Stream state decides before dependencies: an unsafe stream
            // blocks regardless of the task graph.
            let stream_body: String = self
                .connection
                .query_row(
                    "SELECT body FROM streams WHERE id=?1",
                    [stream.as_str()],
                    |r| r.get(0),
                )
                .optional()?
                .ok_or(StoreError::RelationshipMismatch)?;
            let stream_record: ChangeStream = serde_json::from_str(&stream_body)?;
            if stream_record.state() != &StreamState::Active {
                blocked.push(BlockedTask {
                    project_id: project_id.clone(),
                    task_id: task_id.clone(),
                    stream_id: stream_id.clone(),
                    reason: BlockedReason::StreamUnsafe,
                });
                continue;
            }
            // Held leases on the same stream serialize work by policy.
            let leased: Option<String> = self
                .connection
                .query_row(
                    "SELECT task_id FROM task_leases WHERE stream_id=?1 AND state='held' AND expires_at>?2 AND task_id<>?3 LIMIT 1",
                    params![stream.as_str(), now.0 as i64, task.as_str()],
                    |r| r.get(0),
                )
                .optional()?;
            if leased.is_some() {
                blocked.push(BlockedTask {
                    project_id: project_id.clone(),
                    task_id: task_id.clone(),
                    stream_id: stream_id.clone(),
                    reason: BlockedReason::StreamLeased,
                });
                continue;
            }
            // Dependency edges from #94 decide last.
            let mut statement = self.connection.prepare(
                "SELECT kind,target_project,target_task FROM task_dependencies WHERE project_id=?1 AND task_id=?2",
            )?;
            let outgoing = statement.query_map(params![project.as_str(), task.as_str()], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                ))
            })?;
            let mut unresolved = false;
            let mut has_edges = false;
            for row in outgoing {
                let (kind, target_project, target_task) = row?;
                let kind: TaskDependencyKind = serde_json::from_str(&kind)?;
                // Scheduler-relevant waits: requires/consumes_contract_from
                // order work; blocks is inverted (the target waits on this
                // task) and does not gate starting here.
                if !matches!(
                    kind,
                    TaskDependencyKind::Requires | TaskDependencyKind::ConsumesContractFrom
                ) {
                    continue;
                }
                has_edges = true;
                let target_body: String = self
                    .connection
                    .query_row(
                        "SELECT body FROM tasks WHERE id=?1 AND project_id=?2",
                        params![target_task, target_project],
                        |r| r.get(0),
                    )
                    .optional()?
                    .ok_or(StoreError::RelationshipMismatch)?;
                let target: Task = serde_json::from_str(&target_body)?;
                if target.state() != &TaskState::Completed {
                    unresolved = true;
                    break;
                }
            }
            if unresolved {
                blocked.push(BlockedTask {
                    project_id,
                    task_id,
                    stream_id,
                    reason: BlockedReason::DependencyUnresolved,
                });
            } else {
                schedulable.push(SchedulableTask {
                    project_id,
                    task_id,
                    stream_id,
                    role_id,
                    reason: if has_edges {
                        SchedulableReason::DependenciesSatisfied
                    } else {
                        SchedulableReason::NoBlockingDependencies
                    },
                });
            }
        }
        Ok(SchedulingProjection {
            schedulable,
            blocked,
        })
    }
}
