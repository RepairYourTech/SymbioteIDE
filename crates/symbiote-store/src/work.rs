use super::*;

pub(super) const MIGRATION_V3: &str = "
CREATE TABLE work_items (
 id TEXT PRIMARY KEY, project_id TEXT NOT NULL REFERENCES projects(id),
 role_id TEXT NOT NULL, revision INTEGER NOT NULL CHECK(revision>=0),
 body TEXT NOT NULL CHECK(json_valid(body)),
 FOREIGN KEY(role_id,project_id) REFERENCES roles(id,project_id)
) STRICT;
CREATE TABLE task_origins (
 task_id TEXT PRIMARY KEY REFERENCES tasks(id),
 project_id TEXT NOT NULL REFERENCES projects(id),
 body TEXT NOT NULL CHECK(json_valid(body))
) STRICT;
PRAGMA user_version=3;";
const MAX_WORK_ITEMS: usize = 4096;
fn validate_graph(graph: &[WorkItem]) -> Result<()> {
    validate_work_graph(graph)?;
    Ok(())
}

fn items(connection: &Connection) -> Result<Vec<WorkItem>> {
    let count: usize =
        connection.query_row("SELECT count(*) FROM work_items", [], |r| sql_usize(r, 0))?;
    if count > MAX_WORK_ITEMS {
        return Err(StoreError::InvalidInitialState);
    }
    let mut query = connection
        .prepare("SELECT id,project_id,role_id,revision,body FROM work_items ORDER BY id")?;
    let rows = query.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
            sql_u64(r, 3)?,
            r.get::<_, String>(4)?,
        ))
    })?;
    let mut result = Vec::new();
    for row in rows {
        let (id, project, role, revision, body) = row?;
        let item: WorkItem = serde_json::from_str(&body)?;
        if item.id().key() != id
            || item.project_id().as_str() != project
            || item.role_id().as_str() != role
            || item.revision().0 != revision
        {
            return Err(StoreError::Integrity(
                "work indexed state differs from body".into(),
            ));
        }
        result.push(item);
    }
    validate_graph(&result)?;
    Ok(result)
}
fn validate_role(connection: &Connection, item: &WorkItem) -> Result<()> {
    if connection
        .query_row(
            "SELECT 1 FROM roles WHERE id=?1 AND project_id=?2",
            params![item.role_id().as_str(), item.project_id().as_str()],
            |_| Ok(()),
        )
        .optional()?
        .is_none()
    {
        return Err(StoreError::RelationshipMismatch);
    }
    Ok(())
}
fn read_item(connection: &Connection, project: &ProjectId, id: &WorkId) -> Result<WorkItem> {
    items(connection)?
        .into_iter()
        .find(|item| item.id() == id && item.project_id() == project)
        .ok_or(StoreError::NotFound)
}
pub(super) fn validate_origin(
    connection: &Connection,
    project: &ProjectId,
    origin: &TaskOrigin,
) -> Result<()> {
    if &origin.work_ref().project_id != project {
        return Err(StoreError::RelationshipMismatch);
    }
    origin.validate_target(&read_item(connection, project, &origin.work_ref().id)?)?;
    Ok(())
}
pub(super) fn insert_origin(
    connection: &Connection,
    task: &TaskId,
    project: &ProjectId,
    origin: &TaskOrigin,
) -> Result<()> {
    connection.execute(
        "INSERT INTO task_origins(task_id,project_id,body) VALUES (?1,?2,?3)",
        params![
            task.as_str(),
            project.as_str(),
            serde_json::to_string(origin)?
        ],
    )?;
    Ok(())
}
pub(super) fn read_origin(
    connection: &Connection,
    project: &ProjectId,
    task: &TaskId,
) -> Result<Option<TaskOrigin>> {
    let current = read_task(connection, task)?;
    if current.project_id() != project {
        return Err(StoreError::NotFound);
    }
    let row: Option<(String, String)> = connection
        .query_row(
            "SELECT project_id,body FROM task_origins WHERE task_id=?1",
            [task.as_str()],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    row.map(|(owner, body)| {
        if owner != project.as_str() {
            return Err(StoreError::RelationshipMismatch);
        }
        let origin: TaskOrigin = serde_json::from_str(&body)?;
        validate_origin(connection, project, &origin)?;
        Ok(origin)
    })
    .transpose()
}
fn mutation(project: &ProjectId, id: &WorkId, command: &WorkCommand) -> Result<String> {
    Ok(serde_json::to_string(&(
        "apply_work_command",
        project,
        id,
        command,
    ))?)
}

impl Store {
    pub fn create_work_item(&mut self, command_id: CommandId, item: WorkItem) -> Result<Receipt> {
        let payload = EventPayload::WorkItemCreated {
            item: Box::new(item.clone()),
        };
        let request = serde_json::to_string(&payload)?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(receipt) = replay(&transaction, &command_id, &request)? {
            return Ok(receipt);
        }
        if item.revision() != Revision(0) || !item.history().is_empty() {
            return Err(StoreError::InvalidInitialState);
        }
        validate_role(&transaction, &item)?;
        let mut graph = items(&transaction)?;
        if graph.iter().any(|other| other.id() == item.id()) {
            return Err(StoreError::AlreadyExists);
        }
        if graph.len() >= MAX_WORK_ITEMS {
            return Err(StoreError::InvalidInitialState);
        }
        graph.push(item.clone());
        validate_graph(&graph)?;
        transaction.execute(
            "INSERT INTO work_items(id,project_id,role_id,revision,body) VALUES (?1,?2,?3,0,?4)",
            params![
                item.id().key(),
                item.project_id().as_str(),
                item.role_id().as_str(),
                serde_json::to_string(&item)?
            ],
        )?;
        let receipt = append(
            &transaction,
            item.project_id(),
            &command_id,
            item.revision(),
            &request,
            &payload,
        )?;
        transaction.commit()?;
        Ok(receipt)
    }
    pub fn apply_work_command(
        &mut self,
        project: &ProjectId,
        id: &WorkId,
        command: WorkCommand,
    ) -> Result<Receipt> {
        let request = mutation(project, id, &command)?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(receipt) = replay(&transaction, &command.id, &request)? {
            return Ok(receipt);
        }
        let mut graph = items(&transaction)?;
        let item = graph
            .iter_mut()
            .find(|i| i.id() == id && i.project_id() == project)
            .ok_or(StoreError::NotFound)?;
        let previous = item.revision();
        item.apply(command.clone())?;
        let updated = item.clone();
        validate_graph(&graph)?;
        let mut query = transaction.prepare("SELECT body FROM task_origins")?;
        for row in query.query_map([], |r| r.get::<_, String>(0))? {
            let origin: TaskOrigin = serde_json::from_str(&row?)?;
            origin.validate_target(
                graph
                    .iter()
                    .find(|i| {
                        i.id() == &origin.work_ref().id
                            && i.project_id() == &origin.work_ref().project_id
                    })
                    .ok_or(StoreError::NotFound)?,
            )?;
        }
        drop(query);
        let changed=transaction.execute("UPDATE work_items SET revision=?1,body=?2 WHERE id=?3 AND project_id=?4 AND revision=?5",params![sql_revision(updated.revision())?,serde_json::to_string(&updated)?,id.key(),project.as_str(),sql_revision(previous)?])?;
        if changed != 1 {
            return Err(StoreError::RelationshipMismatch);
        }
        let payload = EventPayload::WorkItemChanged {
            project_id: project.clone(),
            work_id: id.clone(),
            command: Box::new(command.clone()),
            item: Box::new(updated.clone()),
        };
        let receipt = append(
            &transaction,
            project,
            &command.id,
            updated.revision(),
            &request,
            &payload,
        )?;
        transaction.commit()?;
        Ok(receipt)
    }
    pub fn work_item(&self, project: &ProjectId, id: &WorkId) -> Result<WorkItem> {
        read_item(&self.connection, project, id)
    }
    pub fn work_command_timestamp(&self, id: &CommandId) -> Result<Option<Timestamp>> {
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
                EventPayload::WorkItemCreated { item } => Ok(Some(item.created_at())),
                EventPayload::WorkItemChanged { command, .. } => Ok(Some(command.at)),
                EventPayload::TaskOriginAssigned { at, .. } => Ok(Some(at)),
                _ => Err(StoreError::IdempotencyConflict),
            },
        }
    }
    pub fn task_origin(&self, project: &ProjectId, task: &TaskId) -> Result<Option<TaskOrigin>> {
        read_origin(&self.connection, project, task)
    }
    pub fn assign_task_origin(
        &mut self,
        command_id: CommandId,
        project: &ProjectId,
        task: &TaskId,
        origin: TaskOrigin,
        actor: UserId,
        at: Timestamp,
    ) -> Result<Receipt> {
        let payload = EventPayload::TaskOriginAssigned {
            task_id: task.clone(),
            project_id: project.clone(),
            origin: origin.clone(),
            actor,
            at,
        };
        let request = serde_json::to_string(&payload)?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(receipt) = replay(&transaction, &command_id, &request)? {
            return Ok(receipt);
        }
        if read_origin(&transaction, project, task)?.is_some() {
            return Err(StoreError::AlreadyExists);
        }
        validate_origin(&transaction, project, &origin)?;
        insert_origin(&transaction, task, project, &origin)?;
        let receipt = append(
            &transaction,
            project,
            &command_id,
            read_task(&transaction, task)?.revision(),
            &request,
            &payload,
        )?;
        transaction.commit()?;
        Ok(receipt)
    }
}

#[derive(Default)]
pub(super) struct Audit {
    items: BTreeMap<WorkId, WorkItem>,
    origins: BTreeMap<TaskId, TaskOrigin>,
}
impl Audit {
    pub(super) fn item(&self, id: &WorkId) -> Option<&WorkItem> {
        self.items.get(id)
    }
}
impl Audit {
    pub(super) fn created(
        &mut self,
        item: &WorkItem,
        project: &str,
        revision: u64,
        request: &str,
        event: &EventPayload,
        projects: &BTreeMap<ProjectId, (Project, Vec<Root>, Vec<Role>)>,
    ) -> Result<()> {
        if item.project_id().as_str() != project
            || revision != 0
            || item.revision() != Revision(0)
            || !item.history().is_empty()
            || request != serde_json::to_string(event)?
            || !projects
                .get(item.project_id())
                .is_some_and(|(_, _, roles)| roles.iter().any(|r| &r.id == item.role_id()))
            || self.items.contains_key(item.id())
        {
            return Err(StoreError::Integrity(
                "invalid work creation lineage".into(),
            ));
        }
        self.items.insert(item.id().clone(), item.clone());
        self.graph()
    }
    fn graph(&self) -> Result<()> {
        if self.items.len() > MAX_WORK_ITEMS {
            return Err(StoreError::InvalidInitialState);
        }
        validate_graph(&self.items.values().cloned().collect::<Vec<_>>())?;
        for origin in self.origins.values() {
            origin.validate_target(
                self.items
                    .get(&origin.work_ref().id)
                    .ok_or(StoreError::NotFound)?,
            )?;
        }
        Ok(())
    }
    pub(super) fn changed(
        &mut self,
        project: &ProjectId,
        id: &WorkId,
        command: &WorkCommand,
        item: &WorkItem,
        journal: (&str, &str, u64, &str),
    ) -> Result<()> {
        let (project_key, command_key, revision, request) = journal;
        let previous = self.items.get_mut(id).ok_or(StoreError::NotFound)?;
        if previous.project_id() != project
            || project.as_str() != project_key
            || command.id.as_str() != command_key
            || request != mutation(project, id, command)?
        {
            return Err(StoreError::RelationshipMismatch);
        }
        previous.apply(command.clone())?;
        if previous != item || item.revision().0 != revision {
            return Err(StoreError::Integrity(
                "work event differs from replay".into(),
            ));
        }
        self.graph()
    }
    pub(super) fn origin(&mut self, task: &Task, origin: &TaskOrigin) -> Result<()> {
        let reference = origin.work_ref();
        if &reference.project_id != task.project_id() || self.origins.contains_key(task.id()) {
            return Err(StoreError::RelationshipMismatch);
        }
        origin.validate_target(self.items.get(&reference.id).ok_or(StoreError::NotFound)?)?;
        self.origins.insert(task.id().clone(), origin.clone());
        Ok(())
    }
    pub(super) fn finish(self, connection: &Connection) -> Result<()> {
        let actual = items(connection)?;
        if actual.len() != self.items.len()
            || actual
                .iter()
                .any(|item| self.items.get(item.id()) != Some(item))
        {
            return Err(StoreError::Integrity(
                "work materialization differs from journal".into(),
            ));
        }
        let count: usize =
            connection.query_row("SELECT count(*) FROM task_origins", [], |r| sql_usize(r, 0))?;
        if count != self.origins.len() {
            return Err(StoreError::Integrity(
                "origin count differs from journal".into(),
            ));
        }
        for (task, origin) in self.origins {
            if read_origin(connection, &origin.work_ref().project_id, &task)?.as_ref()
                != Some(&origin)
            {
                return Err(StoreError::Integrity("origin differs from journal".into()));
            }
        }
        Ok(())
    }
}
