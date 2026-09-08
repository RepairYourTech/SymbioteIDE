use super::*;

pub(super) const MIGRATION_V5: &str = "CREATE TABLE workforce_bindings (
 id TEXT PRIMARY KEY,
 project_id TEXT NOT NULL REFERENCES projects(id),
 role_id TEXT NOT NULL,
 revision INTEGER NOT NULL CHECK(revision>=0),
 actor TEXT NOT NULL,
 updated_at INTEGER NOT NULL CHECK(updated_at>=0),
 body TEXT NOT NULL CHECK(json_valid(body)),
 UNIQUE(project_id,role_id),
 FOREIGN KEY(role_id,project_id) REFERENCES roles(id,project_id)
) STRICT;
PRAGMA user_version=5;";

fn event(
    configuration: &BindingConfiguration,
    expected: Option<Revision>,
    actor: &UserId,
    at: Timestamp,
) -> EventPayload {
    EventPayload::BindingReplaced {
        configuration: Box::new(configuration.clone()),
        expected_revision: expected,
        actor: actor.clone(),
        at,
    }
}
fn validate_revision(
    configuration: &BindingConfiguration,
    expected: Option<Revision>,
) -> Result<()> {
    configuration
        .validate()
        .map_err(|_| StoreError::InvalidBinding)?;
    let next = match expected {
        None => 0,
        Some(old) => old
            .0
            .checked_add(1)
            .ok_or(StoreError::BindingRevisionConflict)?,
    };
    if configuration.binding.revision.0 != next || next > i64::MAX as u64 {
        return Err(StoreError::BindingRevisionConflict);
    }
    Ok(())
}
type Record = (BindingConfiguration, UserId, Timestamp);
/// Read desired state without qualifying it against today's Team. A later Team
/// edit may legitimately make this intent stale while preserving its history.
fn read(connection: &Connection, id: &BindingId) -> Result<Option<Record>> {
    let row: Option<(String,String,u64,String,u64,String)> = connection.query_row(
        "SELECT project_id,role_id,revision,actor,updated_at,body FROM workforce_bindings WHERE id=?1", [id.as_str()],
        |r| Ok((r.get(0)?,r.get(1)?,sql_u64(r,2)?,r.get(3)?,sql_u64(r,4)?,r.get(5)?))).optional()?;
    row.map(|(project, role, revision, actor, at, body)| {
        let configuration: BindingConfiguration = serde_json::from_str(&body)?;
        configuration
            .validate()
            .map_err(|_| StoreError::InvalidBinding)?;
        let binding = &configuration.binding;
        if &binding.id != id
            || binding.project_id.as_str() != project
            || binding.role_id.as_str() != role
            || binding.revision.0 != revision
        {
            return Err(StoreError::Integrity(
                "binding indexed state differs from body".into(),
            ));
        }
        Ok((
            configuration,
            UserId::new(actor).map_err(|_| StoreError::InvalidBinding)?,
            Timestamp(at),
        ))
    })
    .transpose()
}
impl Store {
    /// Trusted internal desired intent; callers authenticate and authorize it.
    /// This operation never activates a runtime or constitutes qualification.
    pub fn replace_binding(
        &mut self,
        command_id: CommandId,
        expected_revision: Option<Revision>,
        configuration: BindingConfiguration,
        actor: UserId,
        at: Timestamp,
    ) -> Result<Receipt> {
        validate_revision(&configuration, expected_revision)?;
        if at.0 > i64::MAX as u64 {
            return Err(StoreError::InvalidBinding);
        }
        let payload = event(&configuration, expected_revision, &actor, at);
        let request = serde_json::to_string(&payload)?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(receipt) = replay(&transaction, &command_id, &request)? {
            return Ok(receipt);
        }
        let binding = &configuration.binding;
        let team_body: String = transaction
            .query_row(
                "SELECT body FROM team_configurations WHERE project_id=?1",
                [binding.project_id.as_str()],
                |r| r.get(0),
            )
            .optional()?
            .ok_or(StoreError::NotFound)?;
        let team: TeamConfiguration = serde_json::from_str(&team_body)?;
        configuration
            .validate_team(&team)
            .map_err(|_| StoreError::InvalidBinding)?;
        let previous = read(&transaction, &binding.id)?;
        if previous.as_ref().map(|(c, _, _)| c.binding.revision) != expected_revision {
            return Err(StoreError::BindingRevisionConflict);
        }
        if previous.as_ref().is_some_and(|(c, _, old)| {
            c.binding.project_id != binding.project_id
                || c.binding.role_id != binding.role_id
                || at.0 < old.0
        }) {
            return Err(StoreError::InvalidBinding);
        }
        let occupied: Option<String> = transaction
            .query_row(
                "SELECT id FROM workforce_bindings WHERE project_id=?1 AND role_id=?2",
                params![binding.project_id.as_str(), binding.role_id.as_str()],
                |r| r.get(0),
            )
            .optional()?;
        if occupied
            .as_deref()
            .is_some_and(|id| id != binding.id.as_str())
        {
            return Err(StoreError::AlreadyExists);
        }
        let body = serde_json::to_string(&configuration)?;
        let changed = if let Some(old) = expected_revision {
            transaction.execute("UPDATE workforce_bindings SET revision=?1,actor=?2,updated_at=?3,body=?4 WHERE id=?5 AND revision=?6",params![sql_revision(binding.revision)?,actor.as_str(),at.0 as i64,body,binding.id.as_str(),sql_revision(old)?])?
        } else {
            transaction.execute("INSERT INTO workforce_bindings(id,project_id,role_id,revision,actor,updated_at,body) VALUES (?1,?2,?3,0,?4,?5,?6)",params![binding.id.as_str(),binding.project_id.as_str(),binding.role_id.as_str(),actor.as_str(),at.0 as i64,body])?
        };
        if changed != 1 {
            return Err(StoreError::BindingRevisionConflict);
        }
        let receipt = append(
            &transaction,
            &binding.project_id,
            &command_id,
            binding.revision,
            &request,
            &payload,
        )?;
        transaction.commit()?;
        Ok(receipt)
    }
    pub fn get_binding(&self, project: &ProjectId, id: &BindingId) -> Result<BindingConfiguration> {
        read(&self.connection, id)?
            .filter(|(c, _, _)| &c.binding.project_id == project)
            .map(|(c, _, _)| c)
            .ok_or(StoreError::NotFound)
    }
    pub fn binding_command_timestamp(&self, id: &CommandId) -> Result<Option<Timestamp>> {
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
                EventPayload::BindingReplaced { at, .. } => Ok(Some(at)),
                _ => Err(StoreError::IdempotencyConflict),
            },
        }
    }
}
#[derive(Default)]
pub(super) struct Audit {
    records: BTreeMap<BindingId, Record>,
    roles: BTreeMap<(ProjectId, RoleId), BindingId>,
}
impl Audit {
    pub(super) fn replaced(
        &mut self,
        configuration: &BindingConfiguration,
        expected: Option<Revision>,
        actor: &UserId,
        at: Timestamp,
        journal: (&str, u64, &str),
        teams: &team::Audit,
    ) -> Result<()> {
        validate_revision(configuration, expected)?;
        let binding = &configuration.binding;
        let current_team = teams
            .current(&binding.project_id)
            .ok_or(StoreError::NotFound)?;
        configuration
            .validate_team(current_team)
            .map_err(|_| StoreError::InvalidBinding)?;
        let (project, revision, request) = journal;
        if binding.project_id.as_str() != project
            || binding.revision.0 != revision
            || at.0 > i64::MAX as u64
            || request != serde_json::to_string(&event(configuration, expected, actor, at))?
        {
            return Err(StoreError::Integrity(
                "invalid binding journal lineage".into(),
            ));
        }
        let previous = self.records.get(&binding.id);
        if previous.map(|(c, _, _)| c.binding.revision) != expected {
            return Err(StoreError::BindingRevisionConflict);
        }
        if previous.is_some_and(|(c, _, old)| {
            c.binding.project_id != binding.project_id
                || c.binding.role_id != binding.role_id
                || at.0 < old.0
        }) {
            return Err(StoreError::InvalidBinding);
        }
        let role = (binding.project_id.clone(), binding.role_id.clone());
        if self.roles.get(&role).is_some_and(|id| id != &binding.id) {
            return Err(StoreError::AlreadyExists);
        }
        self.roles.insert(role, binding.id.clone());
        self.records.insert(
            binding.id.clone(),
            (configuration.clone(), actor.clone(), at),
        );
        Ok(())
    }
    pub(super) fn finish(self, connection: &Connection) -> Result<()> {
        let count = connection.query_row("SELECT count(*) FROM workforce_bindings", [], |r| {
            sql_usize(r, 0)
        })?;
        if count != self.records.len() {
            return Err(StoreError::Integrity(
                "binding count differs from journal".into(),
            ));
        }
        for (id, expected) in self.records {
            if read(connection, &id)?.as_ref() != Some(&expected) {
                return Err(StoreError::Integrity(
                    "binding state differs from journal".into(),
                ));
            }
        }
        Ok(())
    }
}
