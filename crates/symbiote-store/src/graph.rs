use super::*;
use std::collections::btree_map::Entry;

/// The #94 DAG and readiness rows, resolved from the store's own rows:
/// progress, the remaining closure, per-task blockers, the critical path and
/// what each task is waiting on. Nothing is derived from a cached column, a
/// report an agent produced or a caller-supplied number, so the answer cannot
/// disagree with the state it names.
///
/// The bound is applied by taking the Project's tasks in canonical id order
/// until either the task or the gate bound would be exceeded, so the answer is
/// exact over what it considered and `progress.partial` says when it is not.
/// Cross-Project targets are read from their own rows: the graph is global, and
/// a report that hid the other side of a gate would be a guess.
impl Store {
    pub fn project_answer(&self, project: &ProjectId) -> Result<ProjectAnswer> {
        let total: usize = self.connection.query_row(
            "SELECT count(*) FROM tasks WHERE project_id=?1",
            params![project.as_str()],
            |r| sql_usize(r, 0),
        )?;
        if total == 0 {
            // A Project the store does not hold is a refusal, not an empty
            // answer: an empty graph is a real answer for a real Project.
            let known: usize = self.connection.query_row(
                "SELECT count(*) FROM projects WHERE id=?1",
                params![project.as_str()],
                |r| sql_usize(r, 0),
            )?;
            if known == 0 {
                return Err(StoreError::NotFound);
            }
        }

        // The subject's tasks, in canonical id order, within both bounds.
        let mut statement = self
            .connection
            .prepare("SELECT id, stream_id, body FROM tasks WHERE project_id=?1 ORDER BY id")?;
        let rows = statement
            .query_map(params![project.as_str()], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                ))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let mut tasks: Vec<GraphTask> = Vec::new();
        let mut outgoing: BTreeMap<TaskId, BTreeSet<TaskDependencyEdge>> = BTreeMap::new();
        let mut incoming: BTreeMap<TaskId, BTreeSet<TaskDependencyEdge>> = BTreeMap::new();
        let mut referenced: BTreeMap<GraphTaskRef, TaskState> = BTreeMap::new();
        let mut gates = 0usize;
        for (task, stream, body) in rows {
            let record: Task = serde_json::from_str(&body)?;
            let task_id =
                TaskId::new(&task).map_err(|_| StoreError::Integrity("bad task id".into()))?;
            let stream_id = ChangeStreamId::new(&stream)
                .map_err(|_| StoreError::Integrity("bad task stream".into()))?;
            let owned = self.owned_edges(project, &task_id)?;
            let blocked = self.incoming_edges(project, &task_id)?;
            // Both bounds are checked before the row is taken, so the answer the
            // domain computes is always inside the bounds this module states,
            // and the whole count travels beside the considered one.
            let with_this_row = gates + owned.len() + blocked.len();
            if !tasks.is_empty()
                && (tasks.len() >= MAX_GRAPH_REPORT_TASKS || with_this_row > MAX_GRAPH_REPORT_GATES)
            {
                break;
            }
            gates = with_this_row;
            for edge in owned.iter().chain(blocked.iter()) {
                let target = GraphTaskRef {
                    project_id: edge.target.project_id.clone(),
                    task_id: edge.target.task_id.clone(),
                };
                if let Entry::Vacant(slot) = referenced.entry(target) {
                    let state = self.task_state(&edge.target)?;
                    slot.insert(state);
                }
            }
            outgoing.insert(task_id.clone(), owned);
            incoming.insert(task_id.clone(), blocked);
            tasks.push(GraphTask {
                project_id: project.clone(),
                task_id,
                stream_id,
                state: record.state().clone(),
            });
        }
        Ok(symbiote_domain::project_answer(&GraphInputs {
            project_id: project.clone(),
            total,
            tasks,
            outgoing,
            incoming,
            referenced,
        })?)
    }

    /// The edges one task owns, with the indexed columns checked against each
    /// body exactly as the per-task read does: an index that disagrees with the
    /// record it projects is refused, never trusted.
    fn owned_edges(
        &self,
        project: &ProjectId,
        task: &TaskId,
    ) -> Result<BTreeSet<TaskDependencyEdge>> {
        let mut statement = self.connection.prepare(
            "SELECT kind,target_project,target_task,body FROM task_dependencies WHERE project_id=?1 AND task_id=?2 ORDER BY kind,target_project,target_task",
        )?;
        let rows = statement.query_map(params![project.as_str(), task.as_str()], |r| {
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
            edges.insert(dependency_edge(
                &kind,
                &target_project,
                &target_task,
                &body,
            )?);
        }
        Ok(edges)
    }

    /// The edges other tasks own that name this one. Only `blocks` is read as
    /// an incoming dependency (the store enforces exactly that one this way);
    /// the other kinds are recorded provenance and are fetched so the domain
    /// can leave them out of the answer explicitly.
    ///
    /// Read from the blocked task's side, each edge names the *owner* of the
    /// row — the task whose record the `blocks` edge was written on — because
    /// that is the upstream work. Handing back the stored target would name
    /// the blocked task as its own upstream, and that self-edge is precisely
    /// what the reader refuses as a cycle.
    fn incoming_edges(
        &self,
        project: &ProjectId,
        task: &TaskId,
    ) -> Result<BTreeSet<TaskDependencyEdge>> {
        let mut statement = self.connection.prepare(
            "SELECT project_id,task_id,kind,target_project,target_task,body FROM task_dependencies WHERE target_project=?1 AND target_task=?2 ORDER BY project_id,task_id,kind",
        )?;
        let rows = statement.query_map(params![project.as_str(), task.as_str()], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, String>(4)?,
                r.get::<_, String>(5)?,
            ))
        })?;
        let mut edges = BTreeSet::new();
        for row in rows {
            let (owner_project, owner_task, kind, target_project, target_task, body) = row?;
            // The index is still checked against the record it projects, so a
            // row whose columns disagree with its body is refused, never read.
            let stored = dependency_edge(&kind, &target_project, &target_task, &body)?;
            edges.insert(TaskDependencyEdge {
                kind: stored.kind,
                target: TaskDependencyTarget {
                    project_id: ProjectId::new(&owner_project)
                        .map_err(|_| StoreError::Integrity("bad dependency owner".into()))?,
                    task_id: TaskId::new(&owner_task)
                        .map_err(|_| StoreError::Integrity("bad dependency owner".into()))?,
                },
            });
        }
        Ok(edges)
    }

    /// One task's canonical state, read from its own row. A referenced task
    /// that is not there is a relationship failure, not an unknown state: the
    /// report may not guess what a task it names is doing.
    fn task_state(&self, target: &TaskDependencyTarget) -> Result<TaskState> {
        let body: Option<String> = self
            .connection
            .query_row(
                "SELECT body FROM tasks WHERE id=?1 AND project_id=?2",
                params![target.task_id.as_str(), target.project_id.as_str()],
                |r| r.get(0),
            )
            .optional()?;
        let body = body.ok_or(StoreError::RelationshipMismatch)?;
        Ok(serde_json::from_str::<Task>(&body)?.state().clone())
    }
}

/// One stored edge, checked against the columns that project it.
fn dependency_edge(
    kind: &str,
    target_project: &str,
    target_task: &str,
    body: &str,
) -> Result<TaskDependencyEdge> {
    let edge: TaskDependencyEdge = serde_json::from_str(body)?;
    let indexed_ok = serde_json::to_string(&edge.kind)
        .map(|serialized| serialized == kind)
        .unwrap_or(false)
        && edge.target.project_id.as_str() == target_project
        && edge.target.task_id.as_str() == target_task;
    if !indexed_ok {
        return Err(StoreError::Integrity(
            "dependency indexed state differs from body".into(),
        ));
    }
    Ok(edge)
}
