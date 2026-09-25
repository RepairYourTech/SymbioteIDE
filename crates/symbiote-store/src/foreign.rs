use super::*;

/// Migration v13: the foreign runtime links of one canonical Task. The indexed
/// columns are a projection of `body`, exactly as `task_dependencies` is, so a
/// row edited outside the store is caught against its own record rather than
/// served. `foreign_session_id` is NOT NULL because a primary key column
/// cannot be null, and an absent session is stored as the empty string — which
/// no foreign identifier can be, because the domain refuses a blank one, so the
/// encoding cannot collide with a real value.
pub(super) const MIGRATION_V13: &str = "CREATE TABLE task_foreign_links (
 project_id TEXT NOT NULL REFERENCES projects(id),
 task_id TEXT NOT NULL REFERENCES tasks(id),
 system TEXT NOT NULL,
 item_kind TEXT NOT NULL,
 foreign_id TEXT NOT NULL,
 foreign_session_id TEXT NOT NULL,
 body TEXT NOT NULL CHECK(json_valid(body)),
 PRIMARY KEY(project_id, task_id, system, item_kind, foreign_id, foreign_session_id)
) STRICT;
PRAGMA user_version=13;";

fn event(
    project: &ProjectId,
    task: &TaskId,
    links: &BTreeSet<ForeignTaskLink>,
    actor: &UserId,
    at: Timestamp,
) -> EventPayload {
    EventPayload::TaskForeignLinksSet {
        task_id: task.clone(),
        project_id: project.clone(),
        links: links.iter().cloned().collect(),
        actor: actor.clone(),
        at,
    }
}

type TaskKey = (ProjectId, TaskId);

/// The indexed columns of one link: the identity the row is keyed by, with the
/// optional foreign session encoded as the empty string. Status and
/// observation time stay in `body` alone — nothing queries them by index, and
/// the journal audit compares the whole record, so an edited body is caught
/// there rather than by a projection that would only half cover it.
fn indexed(link: &ForeignTaskLink) -> Result<(String, String, String, String)> {
    Ok((
        serde_json::to_string(&link.system)?,
        serde_json::to_string(&link.kind)?,
        link.foreign_id.clone(),
        link.foreign_session_id.clone().unwrap_or_default(),
    ))
}

fn read(connection: &Connection, key: &TaskKey) -> Result<BTreeSet<ForeignTaskLink>> {
    let mut statement = connection.prepare(
        "SELECT system,item_kind,foreign_id,foreign_session_id,body FROM task_foreign_links WHERE project_id=?1 AND task_id=?2 ORDER BY system,item_kind,foreign_id,foreign_session_id",
    )?;
    let rows = statement.query_map(params![key.0.as_str(), key.1.as_str()], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
            r.get::<_, String>(3)?,
            r.get::<_, String>(4)?,
        ))
    })?;
    let mut links = BTreeSet::new();
    for row in rows {
        let (system, kind, foreign_id, foreign_session, body) = row?;
        let link: ForeignTaskLink = serde_json::from_str(&body)?;
        link.validate()?;
        // The indexed columns are tamper-evident projections of the body.
        let (projected_system, projected_kind, projected_id, projected_session) = indexed(&link)?;
        if projected_system != system
            || projected_kind != kind
            || projected_id != foreign_id
            || projected_session != foreign_session
        {
            return Err(StoreError::Integrity(
                "foreign link indexed state differs from body".into(),
            ));
        }
        links.insert(link);
    }
    Ok(links)
}

/// Whether the Task named by this key exists in the Project it is asked for. A
/// link set is always a Task's, so a read or a write for a Task this Project
/// does not hold is refused rather than answered with an empty set.
fn task_exists(connection: &Connection, key: &TaskKey) -> Result<bool> {
    Ok(connection
        .query_row(
            "SELECT 1 FROM tasks WHERE id=?1 AND project_id=?2",
            params![key.1.as_str(), key.0.as_str()],
            |_| Ok(()),
        )
        .optional()?
        .is_some())
}

impl Store {
    /// Replaces the foreign runtime link set of one Task. Callers authenticate
    /// the actor and authorize ManageWork. This records what foreign runtimes
    /// said about their own work; it never advances task state, never creates
    /// a Task, and never satisfies a completion gate — a `complete` status a
    /// harness reported is stored as that harness's claim and read back as it.
    pub fn set_task_foreign_links(
        &mut self,
        command_id: CommandId,
        project: ProjectId,
        task: TaskId,
        links: BTreeSet<ForeignTaskLink>,
        actor: UserId,
        at: Timestamp,
    ) -> Result<Receipt> {
        let key = (project.clone(), task.clone());
        let set = ForeignTaskLinks {
            project_id: project.clone(),
            task_id: task.clone(),
            links,
        };
        set.validate().map_err(|_| StoreError::InvalidForeignLink)?;
        if at.0 > i64::MAX as u64 {
            return Err(StoreError::InvalidForeignLink);
        }
        let payload = event(&project, &task, &set.links, &actor, at);
        let request = serde_json::to_string(&payload)?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(receipt) = replay(&transaction, &command_id, &request)? {
            return Ok(receipt);
        }
        if !task_exists(&transaction, &key)? {
            return Err(StoreError::NotFound);
        }
        transaction.execute(
            "DELETE FROM task_foreign_links WHERE project_id=?1 AND task_id=?2",
            params![project.as_str(), task.as_str()],
        )?;
        for link in &set.links {
            let (system, kind, foreign_id, foreign_session) = indexed(link)?;
            let body = serde_json::to_string(link)?;
            transaction.execute(
                "INSERT INTO task_foreign_links(project_id,task_id,system,item_kind,foreign_id,foreign_session_id,body) VALUES (?1,?2,?3,?4,?5,?6,?7)",
                params![
                    project.as_str(),
                    task.as_str(),
                    system,
                    kind,
                    foreign_id,
                    foreign_session,
                    body
                ],
            )?;
        }
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

    /// The foreign runtime link set of one Task. A Task this Project does not
    /// hold is `NotFound`; a Task with no links is an empty set, because "this
    /// Task has no foreign references" is a real answer and is not a refusal.
    pub fn task_foreign_links(
        &self,
        project: &ProjectId,
        task: &TaskId,
    ) -> Result<BTreeSet<ForeignTaskLink>> {
        let key = (project.clone(), task.clone());
        if !task_exists(&self.connection, &key)? {
            return Err(StoreError::NotFound);
        }
        read(&self.connection, &key)
    }

    pub fn foreign_link_command_timestamp(&self, id: &CommandId) -> Result<Option<Timestamp>> {
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
                EventPayload::TaskForeignLinksSet { at, .. } => Ok(Some(at)),
                _ => Err(StoreError::IdempotencyConflict),
            },
        }
    }
}

/// Journal audit: the link set of one Task is whatever its last set event
/// says, recomputed from the journal and compared to the stored rows in both
/// directions — a set that differs from the journal, and a row with no journal
/// provenance, are both integrity failures.
#[derive(Default)]
pub(super) struct Audit {
    links: BTreeMap<TaskKey, BTreeSet<ForeignTaskLink>>,
}
impl Audit {
    pub(super) fn set(
        &mut self,
        task: &TaskId,
        declared_project: &Option<ProjectId>,
        links: &[ForeignTaskLink],
        actor: &UserId,
        at: Timestamp,
        journal: (&str, u64, &str),
    ) -> Result<()> {
        let (project_key, revision, request) = journal;
        // The payload's declared project is part of authenticated intent: a
        // payload naming another project while its rows and request stay tied
        // to the journal row is corruption, not a valid relocation.
        let declared = ProjectId::new(
            declared_project
                .as_ref()
                .ok_or(StoreError::Integrity(
                    "foreign link event without project".into(),
                ))?
                .as_str(),
        )
        .map_err(|_| StoreError::Integrity("bad foreign link project".into()))?;
        let project = ProjectId::new(project_key)
            .map_err(|_| StoreError::Integrity("bad foreign link project".into()))?;
        if declared != project {
            return Err(StoreError::Integrity(
                "foreign link event project differs from journal".into(),
            ));
        }
        let set = ForeignTaskLinks {
            project_id: project.clone(),
            task_id: task.clone(),
            links: links.iter().cloned().collect(),
        };
        set.validate().map_err(|_| StoreError::InvalidForeignLink)?;
        if at.0 > i64::MAX as u64
            || revision != 0
            || project.as_str() != project_key
            || request != serde_json::to_string(&event(&project, task, &set.links, actor, at))?
        {
            return Err(StoreError::Integrity(
                "invalid foreign link journal lineage".into(),
            ));
        }
        self.links.insert((project, task.clone()), set.links);
        Ok(())
    }

    pub(super) fn finish(self, connection: &Connection) -> Result<()> {
        // Owners whose final set is empty have no rows; the per-key
        // comparison below is the authoritative check, so a count-based
        // rejection would wrongly refuse a legitimately cleared task. The
        // inverse is still corruption: rows with no journal provenance must
        // fail, so every materialized owner must appear in the journal.
        let mut statement =
            connection.prepare("SELECT DISTINCT project_id, task_id FROM task_foreign_links")?;
        let rows =
            statement.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
        for row in rows {
            let (project, task) = row?;
            let project = ProjectId::new(project)
                .map_err(|_| StoreError::Integrity("bad foreign link project".into()))?;
            let task = TaskId::new(task)
                .map_err(|_| StoreError::Integrity("bad foreign link task".into()))?;
            if !self.links.contains_key(&(project, task)) {
                return Err(StoreError::Integrity(
                    "foreign link owner lacks journal provenance".into(),
                ));
            }
        }
        for (key, expected) in self.links {
            if read(connection, &key)? != expected {
                return Err(StoreError::Integrity(
                    "foreign link state differs from journal".into(),
                ));
            }
        }
        Ok(())
    }
}
