use super::*;

/// One Project's task, in the shared `project-one` fixture, with its own
/// stream. `register` owns the Project and its Role; these rows are the DAG
/// the read answers from.
fn graph_task(store: &mut Store, suffix: &str) -> TaskId {
    let (task, stream) = task_records(suffix);
    let id = task.id().clone();
    store
        .create_fixture_task(id!(CommandId, &format!("create-{suffix}")), task, stream)
        .unwrap();
    id
}

fn set_edges(
    store: &mut Store,
    command: &str,
    project: &ProjectId,
    task: &TaskId,
    edges: impl IntoIterator<Item = TaskDependencyEdge>,
) -> Result<Receipt> {
    store.set_task_dependencies(
        id!(CommandId, command),
        project.clone(),
        task.clone(),
        edges.into_iter().collect(),
        id!(UserId, "owner"),
        Timestamp(11),
    )
}

fn requires(project: &ProjectId, task: &TaskId) -> TaskDependencyEdge {
    TaskDependencyEdge {
        kind: TaskDependencyKind::Requires,
        target: TaskDependencyTarget {
            project_id: project.clone(),
            task_id: task.clone(),
        },
    }
}

/// The DAG half of the one public scheduling surface. Every case here reads
/// it the way a caller does rather than through a private door.
fn graph(store: &Store, project: &ProjectId) -> Result<TaskGraphReport> {
    store
        .scheduling_projection(project, Timestamp(30))
        .map(|p| p.dag)
}

fn blocks(project: &ProjectId, task: &TaskId) -> TaskDependencyEdge {
    TaskDependencyEdge {
        kind: TaskDependencyKind::Blocks,
        target: TaskDependencyTarget {
            project_id: project.clone(),
            task_id: task.clone(),
        },
    }
}

fn task_command(command: &str, expected_revision: Revision, action: TaskAction) -> TaskCommand {
    TaskCommand {
        id: id!(CommandId, command),
        expected_revision,
        actor: Actor::Host(id!(HostId, "host")),
        at: Timestamp(12),
        action,
    }
}

#[test]
fn the_dag_report_counts_canonical_rows_and_names_nothing_else() {
    let temp = Temporary::new();
    let mut store = Store::open(temp.database()).unwrap();
    let (project, _, _) = register(&mut store, "one");
    let a = graph_task(&mut store, "graph-a");
    let b = graph_task(&mut store, "graph-b");
    let c = graph_task(&mut store, "graph-c");
    // b requires c; c is recorded against an a that no longer exists as work.
    set_edges(
        &mut store,
        "graph-dep",
        &project.id,
        &b,
        [requires(&project.id, &c)],
    )
    .unwrap();
    store
        .apply_task(
            &a,
            task_command(
                "graph-cancel",
                Revision(0),
                TaskAction::Cancel {
                    reason: "withdrawn".into(),
                },
            ),
        )
        .unwrap();
    store
        .apply_task(
            &c,
            task_command("graph-queue", Revision(0), TaskAction::Queue),
        )
        .unwrap();

    let journaled = |store: &Store| store.events(&project.id, 0, 100).unwrap().events.len();
    let before_reads = journaled(&store);
    let report = graph(&store, &project.id).unwrap();
    assert_eq!(report.project_id, project.id);
    // Every number is counted: three rows, one of them closed by cancellation.
    assert_eq!(report.progress.total, 3);
    assert_eq!(report.progress.considered, 3);
    assert_eq!(report.progress.closed, 1);
    assert_eq!(report.progress.open, 2);
    assert_eq!(
        report
            .progress
            .states
            .iter()
            .map(|entry| (entry.state.clone(), entry.tasks))
            .collect::<Vec<_>>(),
        vec![
            (TaskState::Ready, 1),
            (TaskState::Queued, 1),
            (TaskState::Cancelled, 1),
        ]
    );
    // The remaining closure is the open work, in canonical id order.
    assert_eq!(
        report
            .remaining
            .iter()
            .map(|task| task.task_id.as_str().to_string())
            .collect::<Vec<_>>(),
        vec![b.as_str(), c.as_str()]
    );
    // b waits on c, and c's canonical state travels with the gate.
    assert_eq!(report.blockers.len(), 1);
    let blocked = &report.blockers[0];
    assert_eq!(blocked.task.task_id, b);
    assert_eq!(blocked.gates[0].kind, TaskDependencyKind::Requires);
    assert_eq!(blocked.gates[0].target.task_id, c);
    assert_eq!(blocked.gates[0].target_state, TaskState::Queued);
    assert_eq!(report.critical_path.length, 2);
    assert_eq!(report.critical_path.open, 2);

    // The answer is read, not cached: the next recorded transition moves it.
    store
        .apply_task(
            &c,
            task_command(
                "graph-block",
                Revision(1),
                TaskAction::Block {
                    reason: "dependency unresolved".into(),
                },
            ),
        )
        .unwrap();
    let after = graph(&store, &project.id).unwrap();
    assert_eq!(after.blockers[0].gates[0].target_state, TaskState::Blocked);
    assert_eq!(after.progress.open, 2);
    // A read writes nothing: the journal is exactly what was recorded before
    // the reads, and the reads are not in it.
    assert_eq!(journaled(&store), before_reads + 1);
}

#[test]
fn an_incoming_block_is_read_as_a_dependency_of_the_task_it_blocks() {
    let temp = Temporary::new();
    let mut store = Store::open(temp.database()).unwrap();
    let (project, _, _) = register(&mut store, "one");
    let a = graph_task(&mut store, "block-a");
    let b = graph_task(&mut store, "block-b");
    // `blocks` is recorded on the blocking task, so the row that names b is
    // owned by a. Reading b's graph has to find the gate on the other end of
    // that row, never the gate b already knows: naming the task as its own
    // blocker would be a self-edge, and the honest answer for that is `Cycle`.
    set_edges(
        &mut store,
        "block-ab",
        &project.id,
        &a,
        [blocks(&project.id, &b)],
    )
    .unwrap();

    let report = graph(&store, &project.id).unwrap();
    assert_eq!(report.blockers.len(), 1);
    let blocked = &report.blockers[0];
    assert_eq!(blocked.task.task_id, b);
    assert_eq!(blocked.gates[0].kind, TaskDependencyKind::Blocks);
    assert_eq!(blocked.gates[0].target.task_id, a);
    assert_eq!(blocked.gates[0].target_state, TaskState::Ready);
    // The chain runs upstream first, from the blocking task to the blocked one.
    assert_eq!(report.critical_path.length, 2);
    assert_eq!(
        report
            .critical_path
            .chain
            .iter()
            .map(|task| task.task_id.clone())
            .collect::<Vec<_>>(),
        vec![a.clone(), b]
    );
    assert_eq!(report.critical_path.open, 2);
    // The blocking task's own answer names no blocker: it is not waiting on
    // anything, it is the thing being waited on.
    let mine = store.task_dependencies(&project.id, &a).unwrap();
    assert_eq!(mine.len(), 1);
    assert!(
        mine.iter()
            .all(|edge| edge.kind == TaskDependencyKind::Blocks),
        "the row is still stored on the blocking task, unrewritten: {mine:?}"
    );
}

#[test]
fn an_incoming_edge_whose_index_disagrees_with_its_body_is_refused() {
    let temp = Temporary::new();
    let mut store = Store::open(temp.database()).unwrap();
    let (project, _, _) = register(&mut store, "one");
    let (other, _, _) = register(&mut store, "two");
    let b = graph_task(&mut store, "index-b");
    // The blocking task lives in the other Project, so the only reader of its
    // row is the incoming read. A check the owned read would happen to catch
    // cannot stand in for the one being proved here.
    let (their_task, their_stream) = task_records("index-theirs");
    let their_task = Task::new(
        their_task.id().clone(),
        other.id.clone(),
        other.roots.iter().next().unwrap().clone(),
        id!(RoleId, "role-two"),
        their_task.stream_id().clone(),
        VersionedTaskContract {
            id: id!(TaskContractId, "task-contract-index-theirs"),
            revision: Revision(1),
        },
    );
    let their_stream = ChangeStream::new(NewChangeStream {
        id: their_stream.id().clone(),
        project_id: other.id.clone(),
        root_id: other.roots.iter().next().unwrap().clone(),
        tasks: BTreeSet::from([their_task.id().clone()]),
        originating_chat: id!(ChatId, "chat-index-theirs"),
        worktree: id!(WorktreeId, "worktree-index-theirs"),
        branch: "symbiote/index-theirs".into(),
        lineage: StreamLineage::Independent,
        base: sha('a'),
        target: sha('b'),
    })
    .unwrap();
    let origin = register_work_target(&mut store, &their_task).unwrap();
    store
        .create_task(
            id!(CommandId, "create-index-theirs"),
            their_task.clone(),
            their_stream,
            origin,
        )
        .unwrap();
    set_edges(
        &mut store,
        "index-blocks",
        &other.id,
        their_task.id(),
        [blocks(&project.id, &b)],
    )
    .unwrap();
    let honest = graph(&store, &project.id).unwrap();
    assert_eq!(honest.blockers.len(), 1);
    assert_eq!(honest.blockers[0].gates[0].kind, TaskDependencyKind::Blocks);

    // The row's index says `blocks`; the record it projects says `requires`.
    // The kind is the part the answer is built from, so a read that skipped
    // this check would take a tampered row as provenance, drop b's only gate,
    // and report that nothing holds it back.
    let retag = |store: &mut Store, body: String| {
        store
            .connection
            .execute(
                "UPDATE task_dependencies SET body=?1 WHERE project_id=?2 AND task_id=?3",
                params![body, other.id.as_str(), their_task.id().as_str()],
            )
            .unwrap();
    };
    retag(
        &mut store,
        format!(
            "{{\"kind\":\"requires\",\"target\":{{\"project_id\":\"{}\",\"task_id\":\"{}\"}}}}",
            project.id.as_str(),
            b.as_str()
        ),
    );
    assert!(matches!(
        graph(&store, &project.id),
        Err(StoreError::Integrity(reason)) if reason.contains("dependency")
    ));

    // The target is checked the same way, even though the emitted edge takes
    // its other end from the row owner: a row whose body names a third task is
    // still a row nothing vouches for.
    retag(
        &mut store,
        format!(
            "{{\"kind\":\"blocks\",\"target\":{{\"project_id\":\"{}\",\"task_id\":\"task-index-c\"}}}}",
            project.id.as_str()
        ),
    );
    assert!(matches!(
        graph(&store, &project.id),
        Err(StoreError::Integrity(reason)) if reason.contains("dependency")
    ));
}

#[test]
fn a_cross_project_gate_is_named_with_the_state_it_is_in() {
    let temp = Temporary::new();
    let mut store = Store::open(temp.database()).unwrap();
    let (project, _, _) = register(&mut store, "one");
    let (other, _, _) = register(&mut store, "two");
    let mine = graph_task(&mut store, "cross-mine");
    // The other Project's task row, created against its own fixture identity.
    let (their_task, their_stream) = task_records("cross-theirs");
    let their_task = Task::new(
        their_task.id().clone(),
        other.id.clone(),
        other.roots.iter().next().unwrap().clone(),
        id!(RoleId, "role-two"),
        their_task.stream_id().clone(),
        VersionedTaskContract {
            id: id!(TaskContractId, "task-contract-theirs"),
            revision: Revision(1),
        },
    );
    let their_stream = ChangeStream::new(NewChangeStream {
        id: their_stream.id().clone(),
        project_id: other.id.clone(),
        root_id: other.roots.iter().next().unwrap().clone(),
        tasks: BTreeSet::from([their_task.id().clone()]),
        originating_chat: id!(ChatId, "chat-theirs"),
        worktree: id!(WorktreeId, "worktree-theirs"),
        branch: "symbiote/theirs".into(),
        lineage: StreamLineage::Independent,
        base: sha('a'),
        target: sha('b'),
    })
    .unwrap();
    let origin = register_work_target(&mut store, &their_task).unwrap();
    store
        .create_task(
            id!(CommandId, "create-theirs"),
            their_task.clone(),
            their_stream,
            origin,
        )
        .unwrap();
    set_edges(
        &mut store,
        "cross-dep",
        &project.id,
        &mine,
        [requires(&other.id, their_task.id())],
    )
    .unwrap();

    let report = graph(&store, &project.id).unwrap();
    // The other side of the gate is named with its own project, never folded
    // into this Project's rows or hidden from the reader that must know why the
    // work waits.
    assert_eq!(report.blockers.len(), 1);
    let gate = &report.blockers[0].gates[0];
    assert_eq!(gate.target.project_id, other.id);
    assert_eq!(gate.target.task_id, *their_task.id());
    assert_eq!(gate.target_state, TaskState::Ready);
    // The other Project's own answer does not mention this Project.
    let theirs = graph(&store, &other.id).unwrap();
    assert!(theirs.blockers.is_empty());
    assert_eq!(theirs.progress.total, 1);
}

#[test]
fn a_cycle_is_refused_by_the_writer_and_diagnosed_by_the_reader() {
    let temp = Temporary::new();
    let mut store = Store::open(temp.database()).unwrap();
    let (project, _, _) = register(&mut store, "one");
    let a = graph_task(&mut store, "cycle-a");
    let b = graph_task(&mut store, "cycle-b");
    set_edges(
        &mut store,
        "cycle-ab",
        &project.id,
        &a,
        [requires(&project.id, &b)],
    )
    .unwrap();
    // The write path rejects the cycle, so no edge and no event is recorded.
    assert!(matches!(
        set_edges(
            &mut store,
            "cycle-ba",
            &project.id,
            &b,
            [requires(&project.id, &a)]
        ),
        Err(StoreError::Domain(DomainError::Cycle))
    ));
    assert!(store.task_dependencies(&project.id, &b).unwrap().is_empty());
    assert!(matches!(
        graph(&store, &project.id),
        Ok(TaskGraphReport { blockers, .. }) if blockers.len() == 1
    ));
    // Corrupt the rows directly — the shape a tampered database has — and the
    // read diagnoses the cycle by name instead of walking it.
    store
        .connection
        .execute(
            "INSERT INTO task_dependencies(task_id,project_id,target_project,target_task,kind,body) VALUES (?1,?2,?3,?4,?5,?6)",
            params![
                b.as_str(),
                project.id.as_str(),
                project.id.as_str(),
                a.as_str(),
                "\"requires\"",
                format!(
                    "{{\"kind\":\"requires\",\"target\":{{\"project_id\":\"{}\",\"task_id\":\"{}\"}}}}",
                    project.id.as_str(),
                    a.as_str()
                ),
            ],
        )
        .unwrap();
    assert!(matches!(
        graph(&store, &project.id),
        Err(StoreError::Domain(DomainError::Cycle))
    ));
    // The journal audit catches the same tampering by its own route: the row has
    // no recorded event behind it, which is what an injected edge looks like.
    assert!(matches!(
        store.integrity_check(),
        Err(StoreError::Integrity(reason)) if reason.contains("provenance")
    ));
}

#[test]
fn the_projection_is_the_one_surface_and_readiness_agrees_with_the_schedulable_verdict() {
    let temp = Temporary::new();
    let mut store = Store::open(temp.database()).unwrap();
    let (project, _, _) = register(&mut store, "one");
    let a = graph_task(&mut store, "ready-a");
    let b = graph_task(&mut store, "ready-b");
    let c = graph_task(&mut store, "ready-c");
    // b waits on a; a is cancelled, so b still waits. c waits on nothing.
    set_edges(
        &mut store,
        "ready-dep",
        &project.id,
        &b,
        [requires(&project.id, &a)],
    )
    .unwrap();
    store
        .apply_task(
            &a,
            task_command(
                "ready-cancel",
                Revision(0),
                TaskAction::Cancel {
                    reason: "withdrawn".into(),
                },
            ),
        )
        .unwrap();

    let projection = store
        .scheduling_projection(&project.id, Timestamp(30))
        .unwrap();
    // One read, and the Project is named on it.
    assert_eq!(projection.project_id, project.id);
    assert_eq!(projection.considered_at, Timestamp(30));
    assert_eq!(projection.progress.total, 3);
    // The four answers are all here, not split across two reads.
    assert_eq!(projection.dag.project_id, project.id);
    assert_eq!(projection.dag.critical_path.length, 2);
    assert_eq!(projection.readiness.len(), 3);

    let waiting = |name: &TaskId| {
        projection
            .readiness
            .iter()
            .find(|entry| &entry.task.task_id == name)
            .unwrap()
    };
    // b is waiting on a, and a is named with the state that keeps b waiting.
    assert_eq!(waiting(&b).waiting_on.len(), 1);
    assert_eq!(waiting(&b).waiting_on[0].target.task_id, a);
    assert_eq!(waiting(&b).waiting_on[0].target_state, TaskState::Cancelled);
    // a is cancelled and c is clear, so neither is waiting on anything.
    assert!(waiting(&a).waiting_on.is_empty());
    assert!(waiting(&c).waiting_on.is_empty());
    assert_eq!(waiting(&c).recorded_gates, 0);

    // The scheduling verdict and the readiness evidence cannot disagree,
    // because the store reads the verdict out of the readiness list rather
    // than re-deriving it from the rows.
    let blocked: Vec<TaskId> = projection
        .blocked
        .iter()
        .map(|entry| entry.task_id.clone())
        .collect();
    assert_eq!(blocked, vec![b.clone()]);
    assert_eq!(
        projection.blocked[0].reason,
        BlockedReason::DependencyUnresolved
    );
    // a is cancelled, so it is not a start candidate at all and appears in
    // neither list; c is Ready and clear, so it is offered.
    let schedulable: Vec<TaskId> = projection
        .schedulable
        .iter()
        .map(|entry| entry.task_id.clone())
        .collect();
    assert_eq!(schedulable, vec![c.clone()]);
    assert_eq!(
        projection.schedulable[0].reason,
        SchedulableReason::NoBlockingDependencies
    );
    // A queued task is not a start candidate but is still answered by readiness
    // and by the DAG answers, so "what is waiting" never loses a task.
    store
        .apply_task(
            &c,
            task_command("ready-queue", Revision(0), TaskAction::Queue),
        )
        .unwrap();
    let after = store
        .scheduling_projection(&project.id, Timestamp(31))
        .unwrap();
    assert!(after.schedulable.is_empty());
    // b is still Ready and still waiting on the cancelled a, so it stays in
    // blocked: queueing c did not unblock anything.
    assert_eq!(
        after
            .blocked
            .iter()
            .map(|entry| (entry.task_id.clone(), entry.reason))
            .collect::<Vec<_>>(),
        vec![(b, BlockedReason::DependencyUnresolved)]
    );
    assert_eq!(after.readiness.len(), 3);
    let c_entry = after
        .readiness
        .iter()
        .find(|entry| entry.task.task_id == c)
        .unwrap();
    assert_eq!(c_entry.state, TaskState::Queued);
    assert!(c_entry.waiting_on.is_empty());
    // The same read is a read: it wrote no journal event.
    let journaled = store.events(&project.id, 0, 100).unwrap().events.len();
    assert!(
        store
            .scheduling_projection(&project.id, Timestamp(32))
            .is_ok()
    );
    assert_eq!(
        store.events(&project.id, 0, 100).unwrap().events.len(),
        journaled
    );
}

#[test]
fn a_project_the_store_does_not_hold_is_refused_rather_than_answered_empty() {
    let temp = Temporary::new();
    let mut store = Store::open(temp.database()).unwrap();
    assert!(matches!(
        graph(&store, &id!(ProjectId, "ghost-project")),
        Err(StoreError::NotFound)
    ));
    // A registered Project with no tasks is a real answer, not a refusal.
    let (project, _, _) = register(&mut store, "one");
    let empty = graph(&store, &project.id).unwrap();
    assert_eq!(empty.progress.total, 0);
    assert_eq!(empty.progress.considered, 0);
    assert!(empty.remaining.is_empty());
    assert_eq!(empty.critical_path.length, 0);
}

#[test]
fn a_graph_wider_than_the_stated_bounds_is_answered_partially_and_says_so() {
    let temp = Temporary::new();
    let mut store = Store::open(temp.database()).unwrap();
    let (project, _, _) = register(&mut store, "one");
    let count = symbiote_domain::MAX_GRAPH_REPORT_TASKS + 5;
    for index in 0..count {
        graph_task(&mut store, &format!("wide-{index:04}"));
    }
    let report = graph(&store, &project.id).unwrap();
    assert_eq!(report.progress.total, count);
    assert_eq!(
        report.progress.considered,
        symbiote_domain::MAX_GRAPH_REPORT_TASKS
    );
    // The partial answer is the first tasks in canonical id order, and it says
    // it is partial rather than presenting a whole answer it did not read.
    assert_eq!(
        report.remaining.len(),
        symbiote_domain::MAX_GRAPH_REPORT_TASKS
    );
    assert_eq!(report.remaining[0].task_id, id!(TaskId, "task-wide-0000"));
    assert_eq!(
        report.remaining[report.remaining.len() - 1].task_id,
        id!(TaskId, "task-wide-0255")
    );
    assert!(report.progress.total > report.progress.considered);
}

#[test]
fn a_graph_wider_in_gates_than_the_bound_stops_before_it() {
    let temp = Temporary::new();
    let mut store = Store::open(temp.database()).unwrap();
    let (project, _, _) = register(&mut store, "one");
    // 64 gates per task (the per-task edge bound) over enough tasks that the
    // gate bound is reached before the task bound is.
    let targets: Vec<TaskId> = (0..64)
        .map(|index| graph_task(&mut store, &format!("gates-target-{index:02}")))
        .collect();
    for owner in 0..40 {
        let task = graph_task(&mut store, &format!("gates-owner-{owner:02}"));
        let result = set_edges(
            &mut store,
            &format!("gates-{owner:02}"),
            &project.id,
            &task,
            targets.iter().map(|target| requires(&project.id, target)),
        );
        assert!(result.is_ok(), "owner {owner} failed: {result:?}");
    }
    let report = graph(&store, &project.id).unwrap();
    assert!(
        report.progress.considered_gates <= symbiote_domain::MAX_GRAPH_REPORT_GATES,
        "the gate bound is what it says it is, got {}",
        report.progress.considered_gates
    );
    assert!(report.progress.considered < 40);
    assert_eq!(report.progress.total, 104);
}
