use super::*;

pub(super) const MIGRATION_V7: &str = "CREATE TABLE task_dependencies (
 task_id TEXT NOT NULL REFERENCES tasks(id),
 project_id TEXT NOT NULL REFERENCES projects(id),
 target_project TEXT NOT NULL,
 target_task TEXT NOT NULL,
 kind TEXT NOT NULL,
 body TEXT NOT NULL CHECK(json_valid(body)),
 PRIMARY KEY(project_id, task_id, kind, target_project, target_task)
) STRICT;
CREATE INDEX task_dependencies_target ON task_dependencies(target_project, target_task);
PRAGMA user_version=7;";

pub(super) const MAX_DEPENDENCY_TASKS: usize = 16_384;

fn event(
    project: &ProjectId,
    task: &TaskId,
    edges: &BTreeSet<TaskDependencyEdge>,
    actor: &UserId,
    at: Timestamp,
) -> EventPayload {
    EventPayload::TaskDependenciesSet {
        task_id: task.clone(),
        project_id: project.clone(),
        edges: edges.iter().cloned().collect(),
        actor: actor.clone(),
        at,
    }
}

type TaskKey = (ProjectId, TaskId);

fn read(connection: &Connection, key: &TaskKey) -> Result<BTreeSet<TaskDependencyEdge>> {
    let mut statement = connection.prepare(
        "SELECT kind,target_project,target_task,body FROM task_dependencies WHERE project_id=?1 AND task_id=?2 ORDER BY kind,target_project,target_task",
    )?;
    let rows = statement.query_map(params![key.0.as_str(), key.1.as_str()], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
            r.get::<_, String>(3)?,
        ))
    })?;
    let mut edges = BTreeSet::new();
    for row in rows {
        let (kind, target_project, target_task, body) = row?;
        let edge: TaskDependencyEdge = serde_json::from_str(&body)?;
        // Indexed columns are tamper-evident projections of the body.
        let indexed_ok = serde_json::to_string(&edge.kind)
            .map(|k| k == kind)
            .unwrap_or(false)
            && edge.target.project_id.as_str() == target_project
            && edge.target.task_id.as_str() == target_task;
        if !indexed_ok {
            return Err(StoreError::Integrity(
                "dependency indexed state differs from body".into(),
            ));
        }
        edges.insert(edge);
    }
    Ok(edges)
}

fn edges_of(connection: &Connection) -> Result<BTreeMap<TaskKey, TaskDependencies>> {
    let mut statement =
        connection.prepare("SELECT DISTINCT project_id, task_id FROM task_dependencies")?;
    let rows = statement.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
    let mut graph = BTreeMap::new();
    for row in rows {
        let (project, task) = row?;
        let project = ProjectId::new(project)
            .map_err(|_| StoreError::Integrity("bad dependency project".into()))?;
        let task =
            TaskId::new(task).map_err(|_| StoreError::Integrity("bad dependency task".into()))?;
        let key = (project.clone(), task.clone());
        let edges = read(connection, &key)?;
        graph.insert(
            key,
            TaskDependencies {
                project_id: project,
                task_id: task,
                edges,
            },
        );
    }
    Ok(graph)
}

fn validate_graph(connection: &Connection) -> Result<()> {
    let graph = edges_of(connection)?;
    validate_task_dependency_graph(&graph, |project, task| {
        connection
            .query_row(
                "SELECT 1 FROM tasks WHERE id=?1 AND project_id=?2",
                params![task.as_str(), project.as_str()],
                |_| Ok(()),
            )
            .optional()
            .unwrap_or(None)
            .is_some()
    })?;
    Ok(())
}

impl Store {
    /// Replaces the dependency edge set of one Task. Callers authenticate the
    /// actor and authorize ManageWork; this never advances task state itself.
    /// All edges are validated as one global DAG inside the write transaction.
    pub fn set_task_dependencies(
        &mut self,
        command_id: CommandId,
        project: ProjectId,
        task: TaskId,
        edges: BTreeSet<TaskDependencyEdge>,
        actor: UserId,
        at: Timestamp,
    ) -> Result<Receipt> {
        let dependencies = TaskDependencies {
            project_id: project.clone(),
            task_id: task.clone(),
            edges,
        };
        dependencies
            .validate()
            .map_err(|_| StoreError::InvalidDependency)?;
        if at.0 > i64::MAX as u64 {
            return Err(StoreError::InvalidDependency);
        }
        let payload = event(&project, &task, &dependencies.edges, &actor, at);
        let request = serde_json::to_string(&payload)?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(receipt) = replay(&transaction, &command_id, &request)? {
            return Ok(receipt);
        }
        let task_row = transaction
            .query_row(
                "SELECT 1 FROM tasks WHERE id=?1 AND project_id=?2",
                params![task.as_str(), project.as_str()],
                |_| Ok(()),
            )
            .optional()?;
        if task_row.is_none() {
            return Err(StoreError::NotFound);
        }
        for edge in &dependencies.edges {
            let target_row = transaction
                .query_row(
                    "SELECT 1 FROM tasks WHERE id=?1 AND project_id=?2",
                    params![
                        edge.target.task_id.as_str(),
                        edge.target.project_id.as_str()
                    ],
                    |_| Ok(()),
                )
                .optional()?;
            if target_row.is_none() {
                return Err(StoreError::RelationshipMismatch);
            }
        }
        let count: usize = transaction.query_row(
            "SELECT count(DISTINCT project_id || ':' || task_id) FROM task_dependencies",
            [],
            |r| sql_usize(r, 0),
        )?;
        let has_rows = transaction
            .query_row(
                "SELECT 1 FROM task_dependencies WHERE project_id=?1 AND task_id=?2",
                params![project.as_str(), task.as_str()],
                |_| Ok(()),
            )
            .optional()?
            .is_some();
        if !has_rows && count >= MAX_DEPENDENCY_TASKS {
            return Err(StoreError::ResourceExhausted);
        }
        transaction.execute(
            "DELETE FROM task_dependencies WHERE project_id=?1 AND task_id=?2",
            params![project.as_str(), task.as_str()],
        )?;
        for edge in &dependencies.edges {
            let body = serde_json::to_string(edge)?;
            transaction.execute(
                "INSERT INTO task_dependencies(task_id,project_id,target_project,target_task,kind,body) VALUES (?1,?2,?3,?4,?5,?6)",
                params![
                    task.as_str(),
                    project.as_str(),
                    edge.target.project_id.as_str(),
                    edge.target.task_id.as_str(),
                    serde_json::to_string(&edge.kind)?,
                    body
                ],
            )?;
        }
        validate_graph(&transaction)?;
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

    pub fn task_dependencies(
        &self,
        project: &ProjectId,
        task: &TaskId,
    ) -> Result<BTreeSet<TaskDependencyEdge>> {
        read(&self.connection, &(project.clone(), task.clone()))
    }

    pub fn dependency_command_timestamp(&self, id: &CommandId) -> Result<Option<Timestamp>> {
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
                EventPayload::TaskDependenciesSet { at, .. } => Ok(Some(at)),
                _ => Err(StoreError::IdempotencyConflict),
            },
        }
    }

    /// Completion gate: the task may complete only when every completion-
    /// blocking dependency target is already Completed. Incoming `blocks`
    /// edges are stored on the blocking task and read here as incoming.
    pub fn dependencies_satisfied_for_completion(
        &self,
        project: &ProjectId,
        task: &TaskId,
    ) -> Result<()> {
        dependency::completion_gate(&self.connection, project, task)
    }
}

/// Shared gate implementation; called inside the apply_task transaction and
/// by the read-only query above.
pub(super) fn completion_gate(
    connection: &Connection,
    project: &ProjectId,
    task: &TaskId,
) -> Result<()> {
    {
        let mut statement = connection.prepare(
            "SELECT kind,target_project,target_task FROM task_dependencies WHERE project_id=?1 AND task_id=?2",
        )?;
        let outgoing = statement.query_map(params![project.as_str(), task.as_str()], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
            ))
        })?;
        let mut blocking: Vec<(String, String)> = Vec::new();
        for row in outgoing {
            let (kind, target_project, target_task) = row?;
            let kind: TaskDependencyKind = serde_json::from_str(&kind)?;
            if completion_blocking(&kind) {
                blocking.push((target_project, target_task));
            }
        }
        let mut statement = connection.prepare(
            "SELECT kind,project_id,task_id FROM task_dependencies WHERE target_project=?1 AND target_task=?2",
        )?;
        let incoming = statement.query_map(params![project.as_str(), task.as_str()], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
            ))
        })?;
        for row in incoming {
            let (kind, owner_project, owner_task) = row?;
            let kind: TaskDependencyKind = serde_json::from_str(&kind)?;
            if matches!(kind, TaskDependencyKind::Blocks) {
                blocking.push((owner_project, owner_task));
            }
        }
        for (target_project, target_task) in blocking {
            let body: String = connection
                .query_row(
                    "SELECT body FROM tasks WHERE id=?1 AND project_id=?2",
                    params![target_task, target_project],
                    |r| r.get(0),
                )
                .optional()?
                .ok_or(StoreError::RelationshipMismatch)?;
            let target: Task = serde_json::from_str(&body)?;
            if target.state() != &TaskState::Completed {
                return Err(StoreError::DependenciesUnresolved);
            }
        }
        Ok(())
    }
}

#[derive(Default)]
pub(super) struct Audit {
    graph: BTreeMap<TaskKey, TaskDependencies>,
}
impl Audit {
    pub(super) fn set(
        &mut self,
        task: &TaskId,
        _project: &ProjectId,
        edges: &[TaskDependencyEdge],
        actor: &UserId,
        at: Timestamp,
        journal: (&str, u64, &str),
    ) -> Result<()> {
        let (project_key, revision, request) = journal;
        let project = ProjectId::new(project_key)
            .map_err(|_| StoreError::Integrity("bad dependency project".into()))?;
        let dependencies = TaskDependencies {
            project_id: project.clone(),
            task_id: task.clone(),
            edges: edges.iter().cloned().collect(),
        };
        dependencies
            .validate()
            .map_err(|_| StoreError::InvalidDependency)?;
        if at.0 > i64::MAX as u64
            || revision != 0
            || project.as_str() != project_key
            || request
                != serde_json::to_string(&event(&project, task, &dependencies.edges, actor, at))?
        {
            return Err(StoreError::Integrity(
                "invalid dependency journal lineage".into(),
            ));
        }
        self.graph.insert((project, task.clone()), dependencies);
        Ok(())
    }

    pub(super) fn validate_graph(
        &self,
        exists: impl Fn(&ProjectId, &TaskId) -> bool,
    ) -> Result<()> {
        validate_task_dependency_graph(&self.graph, exists)?;
        Ok(())
    }

    pub(super) fn finish(self, connection: &Connection) -> Result<()> {
        let count: usize = connection.query_row(
            "SELECT count(DISTINCT project_id || ':' || task_id) FROM task_dependencies",
            [],
            |r| sql_usize(r, 0),
        )?;
        if count != self.graph.len() {
            return Err(StoreError::Integrity(
                "dependency task count differs from journal".into(),
            ));
        }
        for (key, expected) in self.graph {
            if read(connection, &key)? != expected.edges {
                return Err(StoreError::Integrity(
                    "dependency state differs from journal".into(),
                ));
            }
        }
        Ok(())
    }
}
