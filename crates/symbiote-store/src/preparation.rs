use super::*;

pub(super) const MIGRATION_V10: &str = "CREATE TABLE dispatch_preparations (
 task_id TEXT PRIMARY KEY REFERENCES tasks(id),
 project_id TEXT NOT NULL REFERENCES projects(id),
 outcome TEXT NOT NULL CHECK(outcome IN ('ready','refused')),
 role_id TEXT,
 compiled_at INTEGER NOT NULL CHECK(compiled_at>=0),
 body TEXT NOT NULL CHECK(json_valid(body))
) STRICT;
PRAGMA user_version=10;";

pub(super) fn event(
    preparation: &DispatchPreparation,
    actor: &UserId,
    at: Timestamp,
) -> EventPayload {
    EventPayload::DispatchPrepared {
        preparation: Box::new(preparation.clone()),
        actor: actor.clone(),
        at,
    }
}

pub(super) fn read(connection: &Connection, task: &TaskId) -> Result<Option<DispatchPreparation>> {
    let row: Option<(String, String, u64, String)> = connection
        .query_row(
            "SELECT project_id,outcome,compiled_at,body FROM dispatch_preparations WHERE task_id=?1",
            [task.as_str()],
            |r| Ok((r.get(0)?, r.get(1)?, sql_u64(r, 2)?, r.get(3)?)),
        )
        .optional()?;
    row.map(|(project, outcome, compiled, body)| {
        let preparation: DispatchPreparation = serde_json::from_str(&body)?;
        preparation
            .validate()
            .map_err(|_| StoreError::InvalidPreparation)?;
        let indexed_ok = preparation.project_id.as_str() == project
            && preparation.compiled_at.0 == compiled
            && matches!(
                (outcome.as_str(), &preparation.outcome),
                ("ready", PreparationOutcome::Ready) | ("refused", PreparationOutcome::Refused)
            );
        if !indexed_ok {
            return Err(StoreError::Integrity(
                "preparation indexed state differs from body".into(),
            ));
        }
        Ok(preparation)
    })
    .transpose()
}

impl Store {
    /// Composes and durably records a dispatch preparation for one task from
    /// authoritative storage state: the scheduling projection's scheduling
    /// decision, the latest route decision's resolved Role, the task's held
    /// lease, the stream's reserved worktree identity, and the provider
    /// binding resolved from the registry for the task's dispatch profile.
    /// Refused compositions are recorded like ready ones — refusals are
    /// evidence, not errors. Callers authorize the owner policy.
    pub fn prepare_dispatch(
        &mut self,
        command_id: CommandId,
        task_id: TaskId,
        actor: UserId,
        at: Timestamp,
    ) -> Result<(Receipt, DispatchPreparation)> {
        if at.0 > i64::MAX as u64 {
            return Err(StoreError::InvalidPreparation);
        }
        self.record_preparation(command_id, task_id, actor, at)
    }

    fn record_preparation(
        &mut self,
        command_id: CommandId,
        task_id: TaskId,
        actor: UserId,
        at: Timestamp,
    ) -> Result<(Receipt, DispatchPreparation)> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let task = read_task(&transaction, &task_id)?;
        let project = task.project_id().clone();
        let stream = read_stream(&transaction, task.stream_id())?;
        // Routing: latest route decision for the task's origin work identity.
        let routing: Option<symbiote_workforce::RouteDecision> =
            match work::read_origin(&transaction, &project, &task_id) {
                Ok(Some(origin)) => {
                    route::read(&transaction, &route::work_key(&origin.reference().id))?
                }
                _ => None,
            };
        // Lease: current lease row for this task, held and unexpired.
        let lease = lease::read(&transaction, &task_id)?
            .filter(|lease| {
                lease.state == symbiote_domain::LeaseState::Held && lease.expires_at.0 > at.0
            })
            .map(|lease| (lease.dispatch_id.clone(), lease.fencing_token));
        // Provider: resolve the binding from the registry for the dispatch
        // profile the task would start with.
        let provider_resolution = resolve_profile_provider(&transaction, &task)?;
        let preparation = DispatchPreparation::compose(
            &task,
            routing
                .as_ref()
                .and_then(|decision| decision.resolved.clone()),
            lease.as_ref().map(|(dispatch, token)| (dispatch, *token)),
            &stream,
            provider_resolution
                .as_ref()
                .map(|(connection, model)| (connection, model)),
            at,
        );
        preparation
            .validate()
            .map_err(|_| StoreError::InvalidPreparation)?;
        let payload = event(&preparation, &actor, at);
        let request = serde_json::to_string(&payload)?;
        if let Some(receipt) = replay(&transaction, &command_id, &request)? {
            let existing = read(&transaction, &task_id)?.ok_or(StoreError::NotFound)?;
            return Ok((receipt, existing));
        }
        let body = serde_json::to_string(&preparation)?;
        transaction.execute(
            "INSERT INTO dispatch_preparations(task_id,project_id,outcome,role_id,compiled_at,body) VALUES (?1,?2,?3,?4,?5,?6)
             ON CONFLICT(task_id) DO UPDATE SET outcome=excluded.outcome, role_id=excluded.role_id, compiled_at=excluded.compiled_at, body=excluded.body",
            params![
                preparation.task_id.as_str(),
                preparation.project_id.as_str(),
                match preparation.outcome {
                    PreparationOutcome::Ready => "ready",
                    PreparationOutcome::Refused => "refused",
                },
                preparation.role_id.as_ref().map(|r| r.as_str()),
                at.0 as i64,
                body
            ],
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
        Ok((receipt, preparation))
    }

    /// Starts a task from its durable `Ready` preparation: compiles the live
    /// `WorkforceRuntimeContract`/`Dispatch` against the caller-supplied Host
    /// record (the Host's own identity and current enforcement claims —
    /// never client-supplied), applies the domain `Start` transition, and
    /// verifies the resulting dispatch matches the preparation's lease and
    /// routed Role. The command id carries the fencing token implicitly
    /// through the preparation's compiled state; exact idempotency follows
    /// the store convention.
    #[allow(clippy::too_many_arguments)]
    pub fn start_prepared_task(
        &mut self,
        command_id: CommandId,
        task_id: TaskId,
        dispatch_id: DispatchId,
        contract_id: RuntimeContractId,
        host: &Host,
        actor: UserId,
        at: Timestamp,
    ) -> Result<(Receipt, Dispatch)> {
        if at.0 > i64::MAX as u64 {
            return Err(StoreError::InvalidPreparation);
        }
        // Caller-chosen ids may not occupy the system sweep namespace.
        if command_id
            .as_str()
            .starts_with(lease::RESERVED_LEASE_PREFIX)
        {
            return Err(StoreError::InvalidLease);
        }
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        // Idempotent retry: consult the journal BEFORE any state validation
        // so a lost response after a committed start replays its receipt
        // (P1 from PR #494 review).
        // Idempotent retry: a committed start under this command id returns
        // its recorded receipt. The request bytes cannot be re-derived (the
        // Host's claim windows move with time), so the retry validates by
        // payload kind and task identity instead of byte equality.
        let existing_row: Option<(u64, u64, String)> = transaction
            .query_row(
                "SELECT sequence,revision,payload FROM journal WHERE command_id=?1",
                [command_id.as_str()],
                |r| Ok((sql_u64(r, 0)?, sql_u64(r, 1)?, r.get(2)?)),
            )
            .optional()?;
        if let Some((sequence, revision, payload)) = &existing_row {
            let parsed: EventPayload = serde_json::from_str(payload)?;
            match parsed {
                EventPayload::TaskChanged {
                    task_id: recorded,
                    task,
                    ..
                } => {
                    if recorded != task_id {
                        return Err(StoreError::IdempotencyConflict);
                    }
                    let dispatch = task
                        .current_dispatch()
                        .ok_or(StoreError::IdempotencyConflict)?
                        .clone();
                    return Ok((
                        Receipt {
                            sequence: *sequence,
                            revision: Revision(*revision),
                            replayed: true,
                        },
                        dispatch,
                    ));
                }
                _ => return Err(StoreError::IdempotencyConflict),
            }
        }
        let preparation = read(&transaction, &task_id)?.ok_or(StoreError::NotFound)?;
        if preparation.outcome != PreparationOutcome::Ready {
            return Err(StoreError::PreparationRefused);
        }
        // The preparation's recorded decisions bind the start: the routed
        // Role must still hold. A live lease from a previous generation is
        // refused; first-time starts acquire the lease below, transactionally
        // after the domain Start transition succeeds.
        let routed_role = preparation.role_id.clone().ok_or(StoreError::NotFound)?;
        if let Some(existing) = lease::read(&transaction, &task_id)? {
            if lease_holds(&existing, at) {
                return Err(StoreError::LeaseConflict(
                    symbiote_domain::LeaseError::StillHeld,
                ));
            }
        }
        let task = read_task(&transaction, &task_id)?;
        if task.project_id() != &preparation.project_id {
            return Err(StoreError::RelationshipMismatch);
        }
        // Same explicit origin guard as apply_task's Start path.
        if work::read_origin(&transaction, &preparation.project_id, &task_id)?.is_none() {
            return Err(StoreError::UnclassifiedTask);
        }
        // Start-time scheduling gates (P1 from PR #494 review): unresolved
        // completion-blocking dependencies and unsafe streams refuse the
        // start — the same conditions the #95 projection encodes.
        dependency::completion_gate(&transaction, &preparation.project_id, &task_id)?;
        {
            let stream_body: String = transaction.query_row(
                "SELECT body FROM streams WHERE id=?1",
                [task.stream_id().as_str()],
                |r| r.get(0),
            )?;
            let stream: ChangeStream = serde_json::from_str(&stream_body)?;
            if stream.state() != &StreamState::Active {
                return Err(StoreError::PreparationRefused);
            }
        }
        // The routed Role must still be the Team's member record.
        let role_body: String = transaction
            .query_row(
                "SELECT body FROM roles WHERE id=?1 AND project_id=?2",
                params![routed_role.as_str(), preparation.project_id.as_str()],
                |r| r.get(0),
            )
            .optional()?
            .ok_or(StoreError::RelationshipMismatch)?;
        let role: Role = serde_json::from_str(&role_body)?;
        // The lease's dispatch was compiled against a profile bound to the
        // routed Role's workforce binding; that binding is the profile source.
        let binding_body: String = transaction
            .query_row(
                "SELECT body FROM workforce_bindings WHERE project_id=?1 AND role_id=?2",
                params![preparation.project_id.as_str(), routed_role.as_str()],
                |r| r.get(0),
            )
            .optional()?
            .ok_or(StoreError::RelationshipMismatch)?;
        let binding: symbiote_workforce::BindingConfiguration =
            serde_json::from_str(&binding_body).map_err(|_| StoreError::InvalidPreparation)?;
        let profile = binding.primary.profile.clone();
        if !profile.eligible_hosts.contains(&host.id) {
            return Err(StoreError::RelationshipMismatch);
        }
        let dispatch = Dispatch::compile(
            dispatch_id,
            contract_id,
            DispatchInputs {
                task: &task,
                role: &role,
                binding: &binding.binding,
                profile: &profile,
                host,
                minimum_enforcement: &binding.policies.minimum_enforcement,
                now: at,
            },
        )
        .map_err(StoreError::Domain)?;
        let command = TaskCommand {
            id: command_id.clone(),
            expected_revision: task.revision(),
            actor: Actor::Host(host.id.clone()),
            at,
            action: TaskAction::Start {
                dispatch: Box::new(dispatch.clone()),
            },
        };
        let receipt = {
            let mut mutable_task = task.clone();
            mutable_task
                .apply(command.clone())
                .map_err(StoreError::Domain)?;
            let updated = transaction.execute(
                "UPDATE tasks SET revision=?1,body=?2 WHERE id=?3 AND revision=?4",
                params![
                    sql_revision(mutable_task.revision())?,
                    serde_json::to_string(&mutable_task)?,
                    task_id.as_str(),
                    sql_revision(task.revision())?
                ],
            )?;
            if updated != 1 {
                return Err(StoreError::Domain(DomainError::RevisionConflict));
            }
            let payload = EventPayload::TaskChanged {
                task_id: task_id.clone(),
                command: Box::new(command.clone()),
                task: Box::new(mutable_task.clone()),
            };
            // The journal's request bytes must match what the audit's
            // TaskChanged arm re-derives: mutation_request format, not the
            // payload serialization (P0 from PR #494 review).
            let request = mutation_request(&task_id, &command)?;
            if let Some(receipt) = replay(&transaction, &command_id, &request)? {
                return Ok((receipt, dispatch));
            }
            append(
                &transaction,
                &preparation.project_id,
                &command_id,
                mutable_task.revision(),
                &request,
                &payload,
            )?
        };
        // Acquire the governing lease for the now-Running dispatch in the
        // same transaction: the fencing token is minted here and any later
        // generation supersedes it.
        let lease_command = CommandId::new(format!("{}-lease", command_id.as_str()))
            .map_err(|_| StoreError::InvalidPreparation)?;
        let expires_at = lease_expiry(at, DEFAULT_START_LEASE_MS)
            .map_err(|_| StoreError::LeaseConflict(symbiote_domain::LeaseError::InvalidRequest))?;
        let fencing_token = lease::read(&transaction, &task_id)?
            .map(|previous| {
                previous
                    .fencing_token
                    .checked_add(1)
                    .ok_or(StoreError::LeaseConflict(
                        symbiote_domain::LeaseError::ResourceLimit,
                    ))
            })
            .transpose()?
            .unwrap_or(1);
        let lease = symbiote_domain::TaskLease {
            version: symbiote_domain::LEASE_VERSION,
            project_id: preparation.project_id.clone(),
            task_id: task_id.clone(),
            stream_id: dispatch.contract().stream_id().clone(),
            dispatch_id: dispatch.id().clone(),
            host_id: host.id.clone(),
            fencing_token,
            state: symbiote_domain::LeaseState::Held,
            acquired_at: at,
            expires_at,
        };
        lease
            .validate_shape()
            .map_err(|_| StoreError::InvalidLease)?;
        let lease_payload = EventPayload::TaskLeased {
            lease: Box::new(lease.clone()),
            actor: actor.clone(),
            at,
        };
        let lease_request = serde_json::to_string(&lease_payload)?;
        let lease_body = serde_json::to_string(&lease)?;
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
                lease_body
            ],
        )?;
        append(
            &transaction,
            &preparation.project_id,
            &lease_command,
            Revision(0),
            &lease_request,
            &lease_payload,
        )?;
        transaction.commit()?;
        Ok((receipt, dispatch))
    }

    pub fn dispatch_preparation(&self, task: &TaskId) -> Result<DispatchPreparation> {
        read(&self.connection, task)?.ok_or(StoreError::NotFound)
    }

    pub fn preparation_command_timestamp(&self, id: &CommandId) -> Result<Option<Timestamp>> {
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
                EventPayload::DispatchPrepared { at, .. } => Ok(Some(at)),
                _ => Err(StoreError::IdempotencyConflict),
            },
        }
    }
}

/// Resolves the provider identity for the task's routed Role from the
/// workforce binding bound to that Role. The task has no dispatch yet at
/// preparation time; the binding's primary candidate carries the runtime
/// profile — including provider connection and model identities — directly.
fn resolve_profile_provider(
    transaction: &Transaction<'_>,
    task: &Task,
) -> Result<Option<(ProviderConnectionId, ModelId)>> {
    let binding_id: Option<String> = transaction
        .query_row(
            "SELECT id FROM workforce_bindings WHERE project_id=?1 AND role_id=?2",
            params![task.project_id().as_str(), task.role_id().as_str()],
            |r| r.get(0),
        )
        .optional()?;
    let Some(binding_id) = binding_id else {
        return Ok(None);
    };
    let body: String = transaction.query_row(
        "SELECT body FROM workforce_bindings WHERE id=?1",
        [binding_id],
        |r| r.get(0),
    )?;
    let binding: symbiote_workforce::BindingConfiguration =
        serde_json::from_str(&body).map_err(|_| StoreError::InvalidPreparation)?;
    let profile = &binding.primary.profile;
    match provider::read(transaction, profile.provider.as_str())? {
        Some(connection) => Ok(Some((connection.id.clone(), profile.model.clone()))),
        None => Ok(None),
    }
}

pub(super) fn audit_finish(
    connection: &Connection,
    preparations: &BTreeMap<TaskId, DispatchPreparation>,
) -> Result<()> {
    for (task, expected) in preparations {
        if read(connection, task)?.as_ref() != Some(expected) {
            return Err(StoreError::Integrity(
                "preparation state differs from journal".into(),
            ));
        }
    }
    let materialized: usize =
        connection.query_row("SELECT count(*) FROM dispatch_preparations", [], |r| {
            sql_usize(r, 0)
        })?;
    if materialized != preparations.len() {
        return Err(StoreError::Integrity(
            "dispatch_preparations differs from journal".into(),
        ));
    }
    Ok(())
}
