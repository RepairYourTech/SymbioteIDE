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
