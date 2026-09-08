use super::*;

pub(super) const MIGRATION_V6: &str = "CREATE TABLE work_routes (
 work_key TEXT PRIMARY KEY,
 project_id TEXT NOT NULL REFERENCES projects(id),
 role_id TEXT,
 resolved INTEGER NOT NULL CHECK(resolved IN (0,1)),
 updated_at INTEGER NOT NULL CHECK(updated_at>=0),
 decision TEXT NOT NULL CHECK(json_valid(decision)),
 FOREIGN KEY(role_id,project_id) REFERENCES roles(id,project_id)
) STRICT;
CREATE INDEX work_routes_project ON work_routes(project_id);
PRAGMA user_version=6;";

/// Latest decision per canonical work item. A later decision replaces the
/// earlier one; the journal retains the full history for provenance.
pub(super) const MAX_ROUTES: usize = 16_384;

fn event(decision: &RouteDecision, actor: &UserId, at: Timestamp) -> EventPayload {
    EventPayload::WorkRouted {
        decision: Box::new(decision.clone()),
        actor: actor.clone(),
        at,
    }
}

fn work_key(work: &WorkId) -> String {
    work.key()
}

fn read(connection: &Connection, key: &str) -> Result<Option<RouteDecision>> {
    let row: Option<(String, Option<String>, i64, String)> = connection
        .query_row(
            "SELECT project_id,role_id,resolved,decision FROM work_routes WHERE work_key=?1",
            [key],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .optional()?;
    row.map(|(project, role, resolved, body)| {
        let decision: RouteDecision = serde_json::from_str(&body)?;
        decision.validate().map_err(|_| StoreError::InvalidRoute)?;
        // Indexed columns are tamper-evident projections of the body, matching
        // the binding/team/work convention.
        let indexed_ok = decision.project_id.as_str() == project
            && decision.resolved.as_ref().map(|r| r.as_str()) == role.as_deref()
            && i64::from(decision.resolved.is_some()) == resolved;
        if !indexed_ok {
            return Err(StoreError::Integrity(
                "route indexed state differs from body".into(),
            ));
        }
        Ok(decision)
    })
    .transpose()
}

impl Store {
    /// Records a routing decision against a canonical work item. Callers
    /// authenticate the actor and authorize ManageWork; this never activates a
    /// runtime or re-staffs an existing dispatch. Re-routing the same work item
    /// supersedes the previous decision while the journal keeps its provenance.
    pub fn record_route(
        &mut self,
        command_id: CommandId,
        decision: RouteDecision,
        actor: UserId,
        at: Timestamp,
    ) -> Result<Receipt> {
        decision.validate().map_err(|_| StoreError::InvalidRoute)?;
        if at.0 > i64::MAX as u64 {
            return Err(StoreError::InvalidRoute);
        }
        let payload = event(&decision, &actor, at);
        let request = serde_json::to_string(&payload)?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(receipt) = replay(&transaction, &command_id, &request)? {
            return Ok(receipt);
        }
        let key = work_key(&decision.work_id);
        let work_project: Option<String> = transaction
            .query_row(
                "SELECT project_id FROM work_items WHERE id=?1",
                [&key],
                |r| r.get(0),
            )
            .optional()?;
        if work_project.as_deref() != Some(decision.project_id.as_str()) {
            return Err(StoreError::RelationshipMismatch);
        }
        if decision.resolved.is_some() {
            // A routed Role must be a current Team member of this Project.
            let team_body: String = transaction
                .query_row(
                    "SELECT body FROM team_configurations WHERE project_id=?1",
                    [decision.project_id.as_str()],
                    |r| r.get(0),
                )
                .optional()?
                .ok_or(StoreError::NotFound)?;
            let team: TeamConfiguration = serde_json::from_str(&team_body)?;
            if !team
                .members
                .iter()
                .any(|m| Some(&m.role_id) == decision.resolved.as_ref())
            {
                return Err(StoreError::InvalidRoute);
            }
        }
        // Replacement decisions never move backward in time, matching the
        // team/binding replacement convention.
        let previous_at: Option<u64> = transaction
            .query_row(
                "SELECT updated_at FROM work_routes WHERE work_key=?1",
                [&key],
                |r| sql_u64(r, 0),
            )
            .optional()?;
        if previous_at.is_some_and(|old| at.0 < old) {
            return Err(StoreError::InvalidRoute);
        }
        let exists = transaction
            .query_row(
                "SELECT 1 FROM work_routes WHERE work_key=?1",
                [&key],
                |_| Ok(()),
            )
            .optional()?;
        if exists.is_none() {
            let count: usize =
                transaction
                    .query_row("SELECT count(*) FROM work_routes", [], |r| sql_usize(r, 0))?;
            if count >= MAX_ROUTES {
                return Err(StoreError::ResourceExhausted);
            }
        }
        let body = serde_json::to_string(&decision)?;
        transaction.execute(
            "INSERT INTO work_routes(work_key,project_id,role_id,resolved,updated_at,decision) VALUES (?1,?2,?3,?4,?5,?6)
             ON CONFLICT(work_key) DO UPDATE SET role_id=excluded.role_id, resolved=excluded.resolved, updated_at=excluded.updated_at, decision=excluded.decision",
            params![
                key,
                decision.project_id.as_str(),
                decision.resolved.as_ref().map(|r| r.as_str()),
                i64::from(decision.resolved.is_some()),
                at.0 as i64,
                body
            ],
        )?;
        let receipt = append(
            &transaction,
            &decision.project_id,
            &command_id,
            Revision(0),
            &request,
            &payload,
        )?;
        transaction.commit()?;
        Ok(receipt)
    }

    pub fn get_route(&self, project: &ProjectId, work_id: &WorkId) -> Result<RouteDecision> {
        read(&self.connection, &work_key(work_id))?
            .filter(|d| &d.project_id == project)
            .ok_or(StoreError::NotFound)
    }

    pub fn route_command_timestamp(&self, id: &CommandId) -> Result<Option<Timestamp>> {
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
                EventPayload::WorkRouted { at, .. } => Ok(Some(at)),
                _ => Err(StoreError::IdempotencyConflict),
            },
        }
    }
}

#[derive(Default)]
pub(super) struct Audit {
    decisions: BTreeMap<String, (RouteDecision, Timestamp)>,
}
impl Audit {
    pub(super) fn routed(
        &mut self,
        decision: &RouteDecision,
        actor: &UserId,
        at: Timestamp,
        journal: (&str, u64, &str),
        work: &work::Audit,
        teams: &team::Audit,
    ) -> Result<()> {
        decision.validate().map_err(|_| StoreError::InvalidRoute)?;
        let (project_key, revision, request) = journal;
        if at.0 > i64::MAX as u64
            || revision != 0
            || decision.project_id.as_str() != project_key
            || request != serde_json::to_string(&event(decision, actor, at))?
        {
            return Err(StoreError::Integrity(
                "invalid route journal lineage".into(),
            ));
        }
        let item = work.item(&decision.work_id).ok_or(StoreError::NotFound)?;
        if item.project_id() != &decision.project_id {
            return Err(StoreError::RelationshipMismatch);
        }
        if decision.resolved.is_some() {
            let team = teams
                .current(&decision.project_id)
                .ok_or(StoreError::NotFound)?;
            if !team
                .members
                .iter()
                .any(|m| Some(&m.role_id) == decision.resolved.as_ref())
            {
                return Err(StoreError::InvalidRoute);
            }
        }
        let key = work_key(&decision.work_id);
        if let Some((_, old)) = self.decisions.get(&key) {
            if at.0 < old.0 {
                return Err(StoreError::InvalidRoute);
            }
        }
        self.decisions.insert(key, (decision.clone(), at));
        Ok(())
    }

    pub(super) fn finish(self, connection: &Connection) -> Result<()> {
        let count: usize =
            connection.query_row("SELECT count(*) FROM work_routes", [], |r| sql_usize(r, 0))?;
        if count != self.decisions.len() {
            return Err(StoreError::Integrity(
                "route count differs from journal".into(),
            ));
        }
        for (key, (expected, _)) in self.decisions {
            if read(connection, &key)?.as_ref() != Some(&expected) {
                return Err(StoreError::Integrity(
                    "route state differs from journal".into(),
                ));
            }
        }
        Ok(())
    }
}
