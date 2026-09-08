use super::*;

pub(super) const MIGRATION_V4: &str = "CREATE TABLE team_configurations (
 project_id TEXT PRIMARY KEY REFERENCES projects(id),
 revision INTEGER NOT NULL CHECK(revision>=0),
 actor TEXT NOT NULL,
 updated_at INTEGER NOT NULL CHECK(updated_at>=0),
 body TEXT NOT NULL CHECK(json_valid(body))
) STRICT;
PRAGMA user_version=4;";

fn event(
    team: &TeamConfiguration,
    expected: Option<Revision>,
    actor: &UserId,
    at: Timestamp,
) -> EventPayload {
    EventPayload::TeamReplaced {
        team: Box::new(team.clone()),
        expected_revision: expected,
        actor: actor.clone(),
        at,
    }
}
fn validate_revision(team: &TeamConfiguration, expected: Option<Revision>) -> Result<()> {
    let next = match expected {
        None => 0,
        Some(old) => old
            .0
            .checked_add(1)
            .ok_or(StoreError::TeamRevisionConflict)?,
    };
    if team.revision.0 != next || team.revision.0 > i64::MAX as u64 {
        return Err(StoreError::TeamRevisionConflict);
    }
    Ok(())
}
fn validate_relationships(connection: &Connection, team: &TeamConfiguration) -> Result<()> {
    team.validate().map_err(|_| StoreError::InvalidTeam)?;
    let lead: Option<String> = connection
        .query_row(
            "SELECT lead_id FROM projects WHERE id=?1",
            [team.project_id.as_str()],
            |r| r.get(0),
        )
        .optional()?;
    if lead.as_deref() != Some(team.lead_role_id.as_str()) {
        return Err(StoreError::RelationshipMismatch);
    }
    let mut query = connection.prepare("SELECT body FROM roles WHERE project_id=?1")?;
    let roles = query
        .query_map([team.project_id.as_str()], |r| r.get::<_, String>(0))?
        .map(|row| Ok(serde_json::from_str::<Role>(&row?)?))
        .collect::<Result<Vec<_>>>()?;
    team.validate_roles(&roles)
        .map_err(|_| StoreError::RelationshipMismatch)?;
    // Access roots are canonical Project roots, not merely well-formed identifiers.
    for root in team
        .access_ceiling
        .roots
        .iter()
        .chain(team.members.iter().flat_map(|m| m.access.roots.iter()))
    {
        if connection
            .query_row(
                "SELECT 1 FROM roots WHERE id=?1 AND project_id=?2",
                params![root.as_str(), team.project_id.as_str()],
                |_| Ok(()),
            )
            .optional()?
            .is_none()
        {
            return Err(StoreError::RelationshipMismatch);
        }
    }
    Ok(())
}
fn read(
    connection: &Connection,
    project: &ProjectId,
) -> Result<Option<(TeamConfiguration, UserId, Timestamp)>> {
    let row: Option<(u64, String, u64, String)> = connection
        .query_row(
            "SELECT revision,actor,updated_at,body FROM team_configurations WHERE project_id=?1",
            [project.as_str()],
            |r| Ok((sql_u64(r, 0)?, r.get(1)?, sql_u64(r, 2)?, r.get(3)?)),
        )
        .optional()?;
    row.map(|(revision, actor, at, body)| {
        let team: TeamConfiguration = serde_json::from_str(&body)?;
        if &team.project_id != project || team.revision.0 != revision {
            return Err(StoreError::Integrity(
                "team indexed state differs from body".into(),
            ));
        }
        validate_relationships(connection, &team)?;
        let actor = UserId::new(actor).map_err(|_| StoreError::InvalidTeam)?;
        Ok((team, actor, Timestamp(at)))
    })
    .transpose()
}
impl Store {
    /// Internal trusted intent; the caller must authenticate actor and authorize
    /// configuration changes. A valid Team is not a qualified runtime workforce.
    pub fn replace_team(
        &mut self,
        command_id: CommandId,
        expected_revision: Option<Revision>,
        team: TeamConfiguration,
        actor: UserId,
        at: Timestamp,
    ) -> Result<Receipt> {
        team.validate().map_err(|_| StoreError::InvalidTeam)?;
        validate_revision(&team, expected_revision)?;
        if at.0 > i64::MAX as u64 {
            return Err(StoreError::InvalidTeam);
        }
        let payload = event(&team, expected_revision, &actor, at);
        let request = serde_json::to_string(&payload)?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(receipt) = replay(&transaction, &command_id, &request)? {
            return Ok(receipt);
        }
        validate_relationships(&transaction, &team)?;
        let previous = read(&transaction, &team.project_id)?;
        if previous.as_ref().map(|(t, _, _)| t.revision) != expected_revision {
            return Err(StoreError::TeamRevisionConflict);
        }
        if previous.as_ref().is_some_and(|(_, _, old)| at.0 < old.0) {
            return Err(StoreError::InvalidTeam);
        }
        let body = serde_json::to_string(&team)?;
        let changed = if let Some(old) = expected_revision {
            transaction.execute("UPDATE team_configurations SET revision=?1,actor=?2,updated_at=?3,body=?4 WHERE project_id=?5 AND revision=?6",params![sql_revision(team.revision)?,actor.as_str(),at.0 as i64,body,team.project_id.as_str(),sql_revision(old)?])?
        } else {
            transaction.execute("INSERT INTO team_configurations(project_id,revision,actor,updated_at,body) VALUES (?1,0,?2,?3,?4)",params![team.project_id.as_str(),actor.as_str(),at.0 as i64,body])?
        };
        if changed != 1 {
            return Err(StoreError::TeamRevisionConflict);
        }
        let receipt = append(
            &transaction,
            &team.project_id,
            &command_id,
            team.revision,
            &request,
            &payload,
        )?;
        transaction.commit()?;
        Ok(receipt)
    }
    pub fn get_team(&self, project: &ProjectId) -> Result<TeamConfiguration> {
        read(&self.connection, project)?
            .map(|(team, _, _)| team)
            .ok_or(StoreError::NotFound)
    }
    pub fn team_command_timestamp(&self, id: &CommandId) -> Result<Option<Timestamp>> {
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
                EventPayload::TeamReplaced { at, .. } => Ok(Some(at)),
                _ => Err(StoreError::IdempotencyConflict),
            },
        }
    }
}

#[derive(Default)]
pub(super) struct Audit {
    records: BTreeMap<ProjectId, (TeamConfiguration, UserId, Timestamp)>,
}
impl Audit {
    pub(super) fn current(&self, project: &ProjectId) -> Option<&TeamConfiguration> {
        self.records.get(project).map(|(team, _, _)| team)
    }
    pub(super) fn replaced(
        &mut self,
        team: &TeamConfiguration,
        expected: Option<Revision>,
        actor: &UserId,
        at: Timestamp,
        journal: (&str, u64, &str),
        projects: &BTreeMap<ProjectId, (Project, Vec<Root>, Vec<Role>)>,
    ) -> Result<()> {
        let (project, revision, request) = journal;
        let (canonical, roots, roles) =
            projects.get(&team.project_id).ok_or(StoreError::NotFound)?;
        team.validate().map_err(|_| StoreError::InvalidTeam)?;
        team.validate_roles(roles)
            .map_err(|_| StoreError::RelationshipMismatch)?;
        validate_revision(team, expected)?;
        if canonical.lead != team.lead_role_id
            || team.project_id.as_str() != project
            || team.revision.0 != revision
            || request != serde_json::to_string(&event(team, expected, actor, at))?
            || at.0 > i64::MAX as u64
        {
            return Err(StoreError::Integrity("invalid team journal lineage".into()));
        }
        let root_ids: BTreeSet<_> = roots.iter().map(|r| r.id.clone()).collect();
        if !team.access_ceiling.roots.is_subset(&root_ids) {
            return Err(StoreError::RelationshipMismatch);
        }
        let previous = self.records.get(&team.project_id);
        if previous.map(|(t, _, _)| t.revision) != expected
            || previous.is_some_and(|(_, _, old)| at.0 < old.0)
        {
            return Err(StoreError::TeamRevisionConflict);
        }
        self.records
            .insert(team.project_id.clone(), (team.clone(), actor.clone(), at));
        Ok(())
    }
    pub(super) fn finish(self, connection: &Connection) -> Result<()> {
        let count = connection.query_row("SELECT count(*) FROM team_configurations", [], |r| {
            sql_usize(r, 0)
        })?;
        if count != self.records.len() {
            return Err(StoreError::Integrity(
                "team count differs from journal".into(),
            ));
        }
        for (project, expected) in self.records {
            if read(connection, &project)?.as_ref() != Some(&expected) {
                return Err(StoreError::Integrity(
                    "team materialization differs from journal".into(),
                ));
            }
        }
        Ok(())
    }
}
