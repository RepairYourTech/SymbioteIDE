//! Durable elevation leases (#269): one decided lease per row, journaled
//! like every other mutation, with expiry enforced at every read (the
//! automatic revocation) and sticky manual revocation.
use super::*;

pub(super) const MIGRATION_V11: &str = "CREATE TABLE elevations (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id),
    dispatch_id TEXT NOT NULL,
    permission TEXT NOT NULL,
    body TEXT NOT NULL CHECK(json_valid(body)),
    UNIQUE(id, project_id)
) STRICT;
PRAGMA user_version = 11;";

pub(super) const MIGRATION_V12: &str = "CREATE TABLE elevation_requests (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id),
    dispatch_id TEXT NOT NULL,
    permission TEXT NOT NULL,
    body TEXT NOT NULL CHECK(json_valid(body)),
    UNIQUE(id, project_id)
) STRICT;
PRAGMA user_version = 12;";

/// Whether an elevation record may attach to `task`'s dispatch. Two states
/// qualify, and the second is what keeps an answered ask from dying with the
/// run that filed it:
///
/// - `Running` under the named dispatch — the original rule. A live run is
///   the attribution every lease was first written against.
/// - `Failed` with the SAME dispatch still attached — the run refused and
///   `file_run_failure` filed the failure without dropping the binding, so
///   the dispatch the ask names is still the task's own. Its id is minted
///   deterministically per task (`disp_{task_id}`), so a recovered task's
///   next run re-mints the identical id: a decision recorded here attaches
///   to the dispatch that comes back, which is the only way an ask a run
///   filed can ever be answered by anything but a hand-written file. The
///   dispatch ids that do NOT come back — after an abandon, a recover of a
///   task whose binding is gone, a foreign id — name no attachable work
///   and are refused exactly as before.
///
/// Every other state refuses: `Ready`/`Queued`/`Assigned`/`Blocked` have no
/// dispatch at all, and the post-completion states are closed.
pub(crate) fn elevation_attaches(task: &Task, dispatch_id: &DispatchId) -> bool {
    match (task.state(), task.current_dispatch()) {
        (TaskState::Running, Some(dispatch)) => dispatch.id() == dispatch_id,
        (TaskState::Failed, Some(dispatch)) => dispatch.id() == dispatch_id,
        _ => false,
    }
}

impl Store {
    /// Records the authority's decision: an approval that licenses the
    /// permission for exactly one dispatch until its window closes, or an
    /// explicit denial (journaled evidence, licenses nothing). The lease
    /// must attribute a dispatch that can come back — the Running one, or
    /// a Failed task whose dispatch binding still names it, because the
    /// dispatch id is deterministic and the recovered run re-mints it —
    /// must not re-grant a permission the dispatch's binding access
    /// already carries, and must stay within the Team's access ceiling.
    pub fn decide_elevation(
        &mut self,
        command_id: CommandId,
        lease: ElevationLease,
    ) -> Result<Receipt> {
        lease.validate().map_err(|_| StoreError::InvalidElevation)?;
        if lease.id != command_id {
            return Err(StoreError::InvalidElevation);
        }
        let payload = EventPayload::ElevationDecided {
            lease: Box::new(lease.clone()),
        };
        let request = serde_json::to_string(&payload)?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(receipt) = replay(&transaction, &command_id, &request)? {
            return Ok(receipt);
        }
        // Attribution: the elevated dispatch must be one that can run — the
        // task's Running dispatch, or the dispatch a Failed task still
        // carries (whose id the recovered run re-mints). Elevation attaches
        // to work that exists, never to a hypothetical dispatch identity.
        let task = read_task(&transaction, &lease.task_id)?;
        if task.project_id() != &lease.project_id {
            return Err(StoreError::RelationshipMismatch);
        }
        let dispatch = task.current_dispatch().ok_or(StoreError::NotFound)?.clone();
        if !elevation_attaches(&task, &lease.dispatch_id) {
            return Err(StoreError::RelationshipMismatch);
        }
        // An approval licenses nothing when its dispatch will never run
        // again, and the checks below ask exactly that: the consumer
        // allowlist and the runtime rule want a NATIVE dispatch, and both
        // are satisfied identically whether the run is live now or comes
        // back after a recover. A Failed task whose stream was abandoned,
        // or whose dispatch binding was dropped, is refused here by the
        // same predicate — there is no run for the license to attach to.
        // Everything below this line asks whether a GRANT would license
        // anything, and a denial licenses nothing at all. So it is recorded
        // on attribution alone: "the authority saw this ask and refused it"
        // is evidence worth keeping precisely when the permission could never
        // have been granted, and refusing to record that refusal is how a
        // Project ends up with an ask on its ledger that nobody ever answered
        // and no record of anyone having said so.
        if !lease.approved {
            let payload = EventPayload::ElevationDecided {
                lease: Box::new(lease.clone()),
            };
            let request = serde_json::to_string(&payload)?;
            transaction.execute(
                "INSERT INTO elevations(id,project_id,dispatch_id,permission,body) VALUES (?1,?2,?3,?4,?5)",
                params![
                    lease.id.as_str(),
                    lease.project_id.as_str(),
                    lease.dispatch_id.as_str(),
                    elevation_permission_name(&lease.permission),
                    serde_json::to_string(&lease)?
                ],
            )?;
            let receipt = append(
                &transaction,
                &lease.project_id,
                &command_id,
                Revision(0),
                &request,
                &payload,
            )?;
            transaction.commit()?;
            return Ok(receipt);
        }
        // Elevation, not re-grant: a permission the dispatch's binding
        // access already carries needs no lease — answered before the
        // consumer rules, because "nothing to elevate" is more specific
        // than "no consumer for it" and true whichever consumer set exists.
        let binding_body: String = transaction
            .query_row(
                "SELECT body FROM workforce_bindings WHERE project_id=?1 AND role_id=?2",
                params![lease.project_id.as_str(), task.role_id().as_str()],
                |r| r.get(0),
            )
            .optional()?
            .ok_or(StoreError::RelationshipMismatch)?;
        let binding: symbiote_workforce::BindingConfiguration =
            serde_json::from_str(&binding_body).map_err(|_| StoreError::InvalidBinding)?;
        if binding.binding.access.grants.contains(&lease.permission) {
            return Err(StoreError::ElevationRegrant);
        }
        // Enforcement consumers so far (#269): the broker's UseCredential
        // gate (#217) and the native shell-tool path's ExecuteProcess gate
        // — both consult `active_elevation` at the run's clock. Deciding a
        // permission with no enforcement consumer would journal an
        // approval that licenses nothing and could mislead an auditor.
        // Widen as each further permission's enforcement point lands.
        if !matches!(
            lease.permission,
            Permission::UseCredential | Permission::ExecuteProcess
        ) {
            return Err(StoreError::ElevationUnconsumed);
        }
        // Runtime-aware consumer rule: every consumer gate lives on the
        // NATIVE loop. The external harness never consults
        // `active_elevation` — its escalations are refused by the driver
        // regardless of any lease — so a decision on an
        // EXTERNAL_HARNESS dispatch would journal an approval that
        // licenses nothing and could mislead an auditor, whichever
        // permission it names. Expressed as a NATIVE allowlist (not an
        // ExternalHarness rejection) so a future third runtime kind
        // fails closed, exactly like the permission allowlist above.
        if dispatch.contract().profile().runtime != symbiote_domain::RuntimeKind::NativeSymbiote {
            return Err(StoreError::ElevationUnenforceableRuntime);
        }
        // The Team's access ceiling is law: no decision elevates beyond it.
        let team =
            team::read(&transaction, &lease.project_id)?.ok_or(StoreError::RelationshipMismatch)?;
        if !team.0.access_ceiling.grants.contains(&lease.permission) {
            return Err(StoreError::ElevationCeiling);
        }
        transaction.execute(
            "INSERT INTO elevations(id,project_id,dispatch_id,permission,body) VALUES (?1,?2,?3,?4,?5)",
            params![
                lease.id.as_str(),
                lease.project_id.as_str(),
                lease.dispatch_id.as_str(),
                elevation_permission_name(&lease.permission),
                serde_json::to_string(&lease)?
            ],
        )?;
        let receipt = append(
            &transaction,
            &lease.project_id,
            &command_id,
            Revision(0),
            &request,
            &payload,
        )?;
        transaction.commit()?;
        Ok(receipt)
    }

    /// Sticky manual revocation: a revoked lease is dead regardless of its
    /// window, and a revocation can never be undone.
    pub fn revoke_elevation(
        &mut self,
        command_id: CommandId,
        project_id: &ProjectId,
        elevation_id: &CommandId,
        revoked_by: &UserId,
        at: Timestamp,
    ) -> Result<Receipt> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(receipt) = replay(
            &transaction,
            &command_id,
            &elevation_revocation_request(project_id, elevation_id, revoked_by, at)?,
        )? {
            return Ok(receipt);
        }
        let body: String = {
            let row: Option<String> = transaction
                .query_row(
                    "SELECT body FROM elevations WHERE id=?1 AND project_id=?2",
                    params![elevation_id.as_str(), project_id.as_str()],
                    |r| r.get(0),
                )
                .optional()?;
            row.ok_or(StoreError::NotFound)?
        };
        let mut lease: ElevationLease = serde_json::from_str(&body)?;
        if lease.revoked_at.is_some() {
            return Err(StoreError::InvalidElevation);
        }
        lease.revoked_at = Some(at);
        lease.validate().map_err(|_| StoreError::InvalidElevation)?;
        let count = transaction.execute(
            "UPDATE elevations SET body=?1 WHERE id=?2 AND project_id=?3",
            params![
                serde_json::to_string(&lease)?,
                elevation_id.as_str(),
                project_id.as_str()
            ],
        )?;
        if count != 1 {
            return Err(StoreError::NotFound);
        }
        let payload = EventPayload::ElevationRevoked {
            lease: Box::new(lease),
            revoked_by: revoked_by.clone(),
        };
        let receipt = append(
            &transaction,
            project_id,
            &command_id,
            Revision(1),
            &elevation_revocation_request(project_id, elevation_id, revoked_by, at)?,
            &payload,
        )?;
        transaction.commit()?;
        Ok(receipt)
    }

    /// The decision's durable timestamp, rebuilt for identical retries.
    ///
    /// The Host stamps the decision clock rather than accepting one on the
    /// wire, so this is what lets a retry replay: the second call rebuilds
    /// the same lease the first one wrote, and the journal's byte-for-byte
    /// comparison answers a replay instead of a conflict. A command id that
    /// was used for something else is an idempotency conflict, never a
    /// silent second clock.
    pub fn elevation_decision_timestamp(
        &self,
        command_id: &CommandId,
    ) -> Result<Option<Timestamp>> {
        let body: Option<String> = self
            .connection
            .query_row(
                "SELECT payload FROM journal WHERE command_id=?1",
                [command_id.as_str()],
                |r| r.get(0),
            )
            .optional()?;
        match body {
            None => Ok(None),
            Some(body) => match serde_json::from_str::<EventPayload>(&body)? {
                EventPayload::ElevationDecided { lease } => Ok(Some(lease.decided_at)),
                _ => Err(StoreError::IdempotencyConflict),
            },
        }
    }

    /// One lease by its own identity, or nothing. The revocation names a
    /// lease the operator has to be able to find first, and a refusal that
    /// cannot say which ids exist is a refusal that cannot be acted on.
    pub fn elevation(
        &self,
        project_id: &ProjectId,
        elevation_id: &CommandId,
    ) -> Result<Option<ElevationLease>> {
        let body: Option<String> = self
            .connection
            .query_row(
                "SELECT body FROM elevations WHERE id=?1 AND project_id=?2",
                params![elevation_id.as_str(), project_id.as_str()],
                |r| r.get(0),
            )
            .optional()?;
        match body {
            None => Ok(None),
            Some(body) => Ok(Some(serde_json::from_str(&body)?)),
        }
    }

    /// Everything one Project was asked for and everything it decided,
    /// read at `at`. Both lists are ordered by their own clock and then by
    /// id, so two reads of the same records answer in the same order and a
    /// caller paging nothing is still deterministic.
    pub fn elevations(
        &self,
        project_id: &ProjectId,
        at: Timestamp,
    ) -> Result<symbiote_domain::ElevationLedger> {
        let mut asks = self
            .connection
            .prepare("SELECT body FROM elevation_requests WHERE project_id=?1 ORDER BY json_extract(body, '$.requested_at'), id")?;
        let requests = asks
            .query_map(params![project_id.as_str()], |r| r.get::<_, String>(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?
            .into_iter()
            .map(|body| serde_json::from_str::<ElevationRequest>(&body).map_err(Into::into))
            .collect::<Result<Vec<_>>>()?;
        let mut decided = self
            .connection
            .prepare("SELECT body FROM elevations WHERE project_id=?1 ORDER BY json_extract(body, '$.decided_at'), id")?;
        let leases: Vec<symbiote_domain::DecidedElevation> = decided
            .query_map(params![project_id.as_str()], |r| r.get::<_, String>(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?
            .into_iter()
            .map(|body| {
                serde_json::from_str::<ElevationLease>(&body)
                    .map(|lease| symbiote_domain::DecidedElevation::read(lease, at))
                    .map_err(Into::into)
            })
            .collect::<Result<Vec<_>>>()?;
        // The asks are correlated with the decisions AFTER both are read, so
        // the join sees every decision on this Project rather than only the
        // ones that happened to be written before it.
        let requests = requests
            .into_iter()
            .map(|request| symbiote_domain::AnsweredRequest::read(request, &leases))
            .collect();
        Ok(symbiote_domain::ElevationLedger {
            project_id: project_id.clone(),
            read_at: at,
            requests,
            leases,
        })
    }

    /// The revocation's durable timestamp, rebuilt for identical retries —
    /// the third of the three clocks this file stamps on the Host's behalf.
    pub fn elevation_revocation_timestamp(
        &self,
        command_id: &CommandId,
    ) -> Result<Option<Timestamp>> {
        let body: Option<String> = self
            .connection
            .query_row(
                "SELECT payload FROM journal WHERE command_id=?1",
                [command_id.as_str()],
                |r| r.get(0),
            )
            .optional()?;
        match body {
            None => Ok(None),
            Some(body) => match serde_json::from_str::<EventPayload>(&body)? {
                EventPayload::ElevationRevoked { lease, .. } => Ok(Some(
                    lease.revoked_at.ok_or(StoreError::IdempotencyConflict)?,
                )),
                _ => Err(StoreError::IdempotencyConflict),
            },
        }
    }

    /// Journals a worker's elevation ask. Observation only: this NEVER
    /// inserts an elevations row and NEVER licenses a permission. Only
    /// [`Self::decide_elevation`] can. Attribution is a Running dispatch.
    pub fn request_elevation(
        &mut self,
        command_id: CommandId,
        ask: ElevationRequest,
    ) -> Result<Receipt> {
        ask.validate().map_err(|_| StoreError::InvalidElevation)?;
        if ask.id != command_id {
            return Err(StoreError::InvalidElevation);
        }
        let payload = EventPayload::ElevationRequested {
            ask: Box::new(ask.clone()),
        };
        let request = serde_json::to_string(&payload)?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(receipt) = replay(&transaction, &command_id, &request)? {
            return Ok(receipt);
        }
        let task = read_task(&transaction, &ask.task_id)?;
        if task.project_id() != &ask.project_id {
            return Err(StoreError::RelationshipMismatch);
        }
        // The same predicate a decision reads: a run that refused and failed
        // its task still names its dispatch, and evidence filed for that
        // dispatch must stay answerable after the failure — otherwise the
        // ask's own record would outlive the only state that could answer it.
        if !elevation_attaches(&task, &ask.dispatch_id) {
            return Err(StoreError::RelationshipMismatch);
        }
        transaction.execute(
            "INSERT INTO elevation_requests(id,project_id,dispatch_id,permission,body) VALUES (?1,?2,?3,?4,?5)",
            params![
                ask.id.as_str(),
                ask.project_id.as_str(),
                ask.dispatch_id.as_str(),
                elevation_permission_name(&ask.permission),
                serde_json::to_string(&ask)?
            ],
        )?;
        let receipt = append(
            &transaction,
            &ask.project_id,
            &command_id,
            Revision(0),
            &request,
            &payload,
        )?;
        transaction.commit()?;
        Ok(receipt)
    }

    /// The request's durable timestamp, rebuilt for identical retries.
    pub fn elevation_request_timestamp(&self, command_id: &CommandId) -> Result<Option<Timestamp>> {
        let body: Option<String> = self
            .connection
            .query_row(
                "SELECT payload FROM journal WHERE command_id=?1",
                [command_id.as_str()],
                |r| r.get(0),
            )
            .optional()?;
        match body {
            None => Ok(None),
            Some(body) => match serde_json::from_str::<EventPayload>(&body)? {
                EventPayload::ElevationRequested { ask } => Ok(Some(ask.requested_at)),
                _ => Err(StoreError::IdempotencyConflict),
            },
        }
    }

    /// Whether an approved, unrevoked, unexpired elevation licenses
    /// `permission` for this dispatch at the reading clock. This IS the
    /// automatic revocation: no sweeper runs, the window is simply checked
    /// every time enforcement asks.
    pub fn active_elevation(
        &self,
        dispatch_id: &DispatchId,
        permission: &Permission,
        at: Timestamp,
    ) -> Result<bool> {
        let mut statement = self
            .connection
            .prepare("SELECT body FROM elevations WHERE dispatch_id=?1 AND permission=?2")?;
        let rows = statement.query_map(
            params![dispatch_id.as_str(), elevation_permission_name(permission)],
            |r| r.get::<_, String>(0),
        )?;
        for row in rows {
            let lease: ElevationLease = serde_json::from_str(&row?)?;
            if lease.licenses(at) {
                return Ok(true);
            }
        }
        Ok(false)
    }
}

pub(super) fn elevation_revocation_request(
    project: &ProjectId,
    elevation: &CommandId,
    user: &UserId,
    at: Timestamp,
) -> Result<String> {
    Ok(serde_json::to_string(&(
        "revoke_elevation",
        project,
        elevation,
        user,
        at,
    ))?)
}

fn elevation_permission_name(permission: &Permission) -> &'static str {
    match permission {
        Permission::ReadRoot => "read_root",
        Permission::MutateStream => "mutate_stream",
        Permission::ExecuteProcess => "execute_process",
        Permission::Network => "network",
        Permission::UseCredential => "use_credential",
    }
}
