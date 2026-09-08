use super::*;

/// Registers a project plus one Ready task (with objective origin) for
/// dependency tests. Returns (project, task) identities.
fn register_task(store: &mut Store, tag: &str) -> (ProjectId, TaskId) {
    let (project, roots, roles) = records(&format!("dep-{tag}"));
    store
        .register_project(
            id!(CommandId, &format!("register-{tag}")),
            project.clone(),
            roots,
            roles,
        )
        .unwrap();
    let (task, _) = task_records(tag);
    // task_records shares one project; build the task against this test's
    // project identity directly.
    let task = Task::new(
        task.id().clone(),
        project.id.clone(),
        project.roots.iter().next().unwrap().clone(),
        project.lead.clone(),
        task.stream_id().clone(),
        VersionedTaskContract {
            id: id!(TaskContractId, &format!("contract-{tag}")),
            revision: Revision(1),
        },
    );
    let stream = ChangeStream::new(NewChangeStream {
        id: id!(ChangeStreamId, &format!("stream-{tag}")),
        project_id: project.id.clone(),
        root_id: project.roots.iter().next().unwrap().clone(),
        tasks: BTreeSet::from([task.id().clone()]),
        originating_chat: id!(ChatId, &format!("chat-{tag}")),
        worktree: id!(WorktreeId, &format!("worktree-{tag}")),
        branch: format!("symbiote/dep/{tag}"),
        lineage: StreamLineage::Independent,
        base: sha('a'),
        target: sha('b'),
    })
    .unwrap();
    let origin = register_work_target(store, &task).unwrap();
    store
        .create_task(id!(CommandId, &format!("task-{tag}")), task, stream, origin)
        .unwrap();
    (project.id.clone(), id!(TaskId, &format!("task-{tag}")))
}

fn edge(kind: TaskDependencyKind, project: &str, task: &str) -> TaskDependencyEdge {
    TaskDependencyEdge {
        kind,
        target: TaskDependencyTarget {
            project_id: ProjectId::new(project).unwrap(),
            task_id: TaskId::new(task).unwrap(),
        },
    }
}

fn set_edges(
    store: &mut Store,
    command: &str,
    project: &ProjectId,
    task: &TaskId,
    edges: &[TaskDependencyEdge],
    at: u64,
) -> Result<Receipt> {
    store.set_task_dependencies(
        id!(CommandId, command),
        project.clone(),
        task.clone(),
        edges.iter().cloned().collect(),
        id!(UserId, "owner"),
        Timestamp(at),
    )
}

#[test]
fn dependency_sets_replace_replay_and_survive_restart() {
    let temp = Temporary::new();
    let mut store = Store::open(temp.database()).unwrap();
    let (project, task) = register_task(&mut store, "one");
    let (other_project, other_task) = register_task(&mut store, "two");
    let receipt = set_edges(
        &mut store,
        "dep-initial",
        &project,
        &task,
        &[edge(
            TaskDependencyKind::Requires,
            other_project.as_str(),
            other_task.as_str(),
        )],
        11,
    )
    .unwrap();
    assert!(!receipt.replayed);
    let replay = set_edges(
        &mut store,
        "dep-initial",
        &project,
        &task,
        &[edge(
            TaskDependencyKind::Requires,
            other_project.as_str(),
            other_task.as_str(),
        )],
        11,
    )
    .unwrap();
    assert!(replay.replayed);
    assert_eq!(replay.sequence, receipt.sequence);
    assert_eq!(store.task_dependencies(&project, &task).unwrap().len(), 1);
    // Replacement supersedes; the journal keeps the original.
    set_edges(
        &mut store,
        "dep-replace",
        &project,
        &task,
        &[edge(
            TaskDependencyKind::Reviews,
            other_project.as_str(),
            other_task.as_str(),
        )],
        12,
    )
    .unwrap();
    assert_eq!(store.task_dependencies(&project, &task).unwrap().len(), 1);
    drop(store);
    let store = Store::open(temp.database()).unwrap();
    let edges = store.task_dependencies(&project, &task).unwrap();
    assert!(matches!(
        edges.iter().next().map(|e| e.kind),
        Some(TaskDependencyKind::Reviews)
    ));
    assert_eq!(
        store
            .dependency_command_timestamp(&id!(CommandId, "dep-initial"))
            .unwrap(),
        Some(Timestamp(11))
    );
}

#[test]
fn cycles_missing_targets_and_unknown_tasks_are_rejected() {
    let mut store = Store::memory().unwrap();
    let (project, task) = register_task(&mut store, "cyc");
    let (other_project, other_task) = register_task(&mut store, "cyc2");
    set_edges(
        &mut store,
        "dep-a",
        &project,
        &task,
        &[edge(
            TaskDependencyKind::Requires,
            other_project.as_str(),
            other_task.as_str(),
        )],
        11,
    )
    .unwrap();
    // Closing the loop across projects is rejected and leaves no partial rows.
    assert!(matches!(
        set_edges(
            &mut store,
            "dep-b",
            &other_project,
            &other_task,
            &[edge(
                TaskDependencyKind::Requires,
                project.as_str(),
                task.as_str()
            )],
            12,
        ),
        Err(StoreError::Domain(DomainError::Cycle))
    ));
    assert!(
        store
            .task_dependencies(&other_project, &other_task)
            .unwrap()
            .is_empty()
    );
    // Unknown target task.
    assert!(matches!(
        set_edges(
            &mut store,
            "dep-ghost",
            &project,
            &task,
            &[edge(
                TaskDependencyKind::Requires,
                project.as_str(),
                "ghost"
            )],
            13,
        ),
        Err(StoreError::RelationshipMismatch)
    ));
    // Unknown owning task.
    assert!(matches!(
        set_edges(
            &mut store,
            "dep-nowhere",
            &project,
            &id!(TaskId, "ghost"),
            &[edge(
                TaskDependencyKind::Requires,
                other_project.as_str(),
                other_task.as_str()
            )],
            14,
        ),
        Err(StoreError::NotFound)
    ));
    // Self-edge rejected by domain validation.
    assert!(matches!(
        set_edges(
            &mut store,
            "dep-self",
            &project,
            &task,
            &[edge(
                TaskDependencyKind::Requires,
                project.as_str(),
                task.as_str()
            )],
            15,
        ),
        Err(StoreError::InvalidDependency)
    ));
}

#[test]
fn completion_blocks_on_unresolved_dependencies_and_incoming_blocks() {
    let mut store = Store::memory().unwrap();
    let (project, task) = register_task(&mut store, "block");
    let (other_project, other_task) = register_task(&mut store, "block2");
    set_edges(
        &mut store,
        "dep-block",
        &project,
        &task,
        &[edge(
            TaskDependencyKind::Requires,
            other_project.as_str(),
            other_task.as_str(),
        )],
        11,
    )
    .unwrap();
    // The target is Ready; completion of the owner must be refused at the
    // gate even before lifecycle evidence is considered.
    assert!(matches!(
        store.dependencies_satisfied_for_completion(&project, &task),
        Err(StoreError::DependenciesUnresolved)
    ));
    // Incoming blocks: other_task blocking completion of task.
    set_edges(
        &mut store,
        "dep-incoming",
        &other_project,
        &other_task,
        &[edge(
            TaskDependencyKind::Blocks,
            project.as_str(),
            task.as_str(),
        )],
        12,
    )
    .unwrap();
    assert!(matches!(
        store.dependencies_satisfied_for_completion(&project, &task),
        Err(StoreError::DependenciesUnresolved)
    ));
    // Remove the owner's outgoing requires via replacement; the incoming
    // blocks edge from other_task must still gate.
    set_edges(&mut store, "dep-clear", &project, &task, &[], 13).unwrap();
    assert!(matches!(
        store.dependencies_satisfied_for_completion(&project, &task),
        Err(StoreError::DependenciesUnresolved)
    ));
    // Non-blocking kinds never gate completion: replace the blocker's edges.
    set_edges(
        &mut store,
        "dep-followup",
        &other_project,
        &other_task,
        &[edge(
            TaskDependencyKind::FollowUpTo,
            project.as_str(),
            task.as_str(),
        )],
        14,
    )
    .unwrap();
    assert!(
        store
            .dependencies_satisfied_for_completion(&project, &task)
            .is_ok()
    );
}

#[test]
fn tampered_dependency_rows_are_refused_on_reopen() {
    let temp = Temporary::new();
    let mut store = Store::open(temp.database()).unwrap();
    let (project, task) = register_task(&mut store, "tamper");
    let (other_project, other_task) = register_task(&mut store, "tamper2");
    set_edges(
        &mut store,
        "dep-tamper",
        &project,
        &task,
        &[edge(
            TaskDependencyKind::Requires,
            other_project.as_str(),
            other_task.as_str(),
        )],
        11,
    )
    .unwrap();
    drop(store);
    // Desynchronize an indexed column from the body.
    let connection = Connection::open(temp.database()).unwrap();
    connection
        .execute(
            "UPDATE task_dependencies SET target_task='ghost' WHERE project_id=?1",
            params![project.as_str()],
        )
        .unwrap();
    drop(connection);
    assert!(matches!(
        Store::open(temp.database()),
        Err(StoreError::Integrity(_))
    ));
}
