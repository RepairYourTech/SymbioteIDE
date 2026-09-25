//! Canonical DAG answers: progress, the remaining closure, per-task blockers,
//! the critical path, same-stream overlap, and the bounds one answer is held to.
use std::collections::{BTreeMap, BTreeSet};
use symbiote_domain::*;

const ALPHA: &str = "alpha";
const BETA: &str = "beta";

fn project(name: &str) -> ProjectId {
    ProjectId::new(name).unwrap()
}
fn task(name: &str) -> TaskId {
    TaskId::new(name).unwrap()
}
fn stream(name: &str) -> ChangeStreamId {
    ChangeStreamId::new(name).unwrap()
}
fn reference(project_name: &str, task_name: &str) -> GraphTaskRef {
    GraphTaskRef {
        project_id: project(project_name),
        task_id: task(task_name),
    }
}
fn row(project_name: &str, task_name: &str, stream_name: &str, state: TaskState) -> GraphTask {
    GraphTask {
        project_id: project(project_name),
        task_id: task(task_name),
        stream_id: stream(stream_name),
        state,
    }
}
fn edge(kind: TaskDependencyKind, project_name: &str, task_name: &str) -> TaskDependencyEdge {
    TaskDependencyEdge {
        kind,
        target: TaskDependencyTarget {
            project_id: project(project_name),
            task_id: task(task_name),
        },
    }
}
fn owned(
    owner: &str,
    edges: impl IntoIterator<Item = TaskDependencyEdge>,
) -> (TaskId, BTreeSet<TaskDependencyEdge>) {
    (task(owner), edges.into_iter().collect())
}
fn inputs(
    tasks: Vec<GraphTask>,
    outgoing: impl IntoIterator<Item = (TaskId, BTreeSet<TaskDependencyEdge>)>,
    incoming: impl IntoIterator<Item = (TaskId, BTreeSet<TaskDependencyEdge>)>,
    referenced: impl IntoIterator<Item = (GraphTaskRef, TaskState)>,
) -> GraphInputs {
    let total = tasks.len();
    GraphInputs {
        project_id: project(ALPHA),
        total,
        tasks,
        outgoing: outgoing.into_iter().collect(),
        incoming: incoming.into_iter().collect(),
        referenced: referenced.into_iter().collect(),
    }
}

#[test]
fn progress_is_counted_from_canonical_states_and_carries_no_percentage() {
    let report = task_graph_report(&inputs(
        vec![
            row(ALPHA, "a", "s1", TaskState::Completed),
            row(ALPHA, "b", "s1", TaskState::Completed),
            row(ALPHA, "c", "s2", TaskState::Running),
            row(ALPHA, "d", "s2", TaskState::Ready),
            row(ALPHA, "e", "s3", TaskState::Cancelled),
            row(ALPHA, "f", "s3", TaskState::Failed),
        ],
        Vec::new(),
        Vec::new(),
        Vec::new(),
    ))
    .unwrap();
    assert_eq!(report.progress.total, 6);
    assert_eq!(report.progress.considered, 6);
    assert_eq!(report.progress.closed, 3);
    assert_eq!(report.progress.open, 3);
    // The whole vocabulary that is present, in canonical lifecycle order.
    let counts: Vec<(TaskState, usize)> = report
        .progress
        .states
        .iter()
        .map(|entry| (entry.state.clone(), entry.tasks))
        .collect();
    assert_eq!(
        counts,
        vec![
            (TaskState::Ready, 1),
            (TaskState::Running, 1),
            (TaskState::Completed, 2),
            // Failed is not closed: the Host recovers it to Ready.
            (TaskState::Failed, 1),
            (TaskState::Cancelled, 1),
        ]
    );
    // A cancelled task is closed, and the remaining closure is the open work.
    let remaining: Vec<String> = report
        .remaining
        .iter()
        .map(|task| task.task_id.as_str().to_string())
        .collect();
    assert_eq!(remaining, vec!["c", "d", "f"]);
    // The document cannot express a percentage at all: progress is counts.
    let serialized = serde_json::to_value(&report.progress).unwrap();
    let object = serialized.as_object().unwrap();
    assert!(!object.contains_key("percent") && !object.contains_key("percentage"));
    assert!(!serialized.to_string().contains('%'));
}

#[test]
fn blockers_name_the_canonical_tasks_an_open_task_waits_on() {
    // d requires c; c consumes from b (in another Project); a blocks d; and d
    // records a review of e, a kind the store does not enforce yet.
    let report = task_graph_report(&inputs(
        vec![
            row(ALPHA, "a", "s1", TaskState::Completed),
            row(ALPHA, "c", "s1", TaskState::Running),
            row(ALPHA, "d", "s1", TaskState::Ready),
            row(ALPHA, "e", "s1", TaskState::Ready),
        ],
        vec![
            owned(
                "c",
                [edge(TaskDependencyKind::ConsumesContractFrom, BETA, "b")],
            ),
            owned(
                "d",
                [
                    edge(TaskDependencyKind::Requires, ALPHA, "c"),
                    edge(TaskDependencyKind::Reviews, ALPHA, "e"),
                ],
            ),
        ],
        vec![owned("d", [edge(TaskDependencyKind::Blocks, ALPHA, "a")])],
        vec![
            (reference(ALPHA, "a"), TaskState::Completed),
            (reference(ALPHA, "c"), TaskState::Running),
            (reference(ALPHA, "e"), TaskState::Ready),
            (reference(BETA, "b"), TaskState::Ready),
        ],
    ))
    .unwrap();
    // Two open tasks wait on something: d on c, and c on another Project's b.
    assert_eq!(report.blockers.len(), 2);
    let blocked = &report.blockers[1];
    assert_eq!(blocked.task, reference(ALPHA, "d"));
    // The satisfied `blocks` gate is history, not a blocker; the unenforced
    // `reviews` kind is not a gate at all.
    assert_eq!(
        blocked
            .gates
            .iter()
            .map(|gate| (gate.kind, gate.target.clone(), gate.target_state.clone()))
            .collect::<Vec<_>>(),
        vec![(
            TaskDependencyKind::Requires,
            reference(ALPHA, "c"),
            TaskState::Running
        )]
    );
    // c waits on a task in another Project, named with the state it is in.
    let c_blockers = &report.blockers[0];
    assert_eq!(
        c_blockers.gates[0].target,
        reference(BETA, "b"),
        "a cross-Project gate is named, not hidden"
    );
}

#[test]
fn a_cancelled_prerequisite_still_gates_its_dependent() {
    let report = task_graph_report(&inputs(
        vec![
            row(ALPHA, "a", "s1", TaskState::Cancelled),
            row(ALPHA, "b", "s1", TaskState::Ready),
        ],
        vec![owned("b", [edge(TaskDependencyKind::Requires, ALPHA, "a")])],
        Vec::new(),
        vec![(reference(ALPHA, "a"), TaskState::Cancelled)],
    ))
    .unwrap();
    assert!(a_state_is_closed_but_not_completed());
    assert_eq!(report.blockers.len(), 1);
    assert_eq!(
        report.blockers[0].gates[0].target_state,
        TaskState::Cancelled
    );
    // The closed prerequisite is out of the closure; the dependent is not out of
    // it, and it is still on the blocker list.
    assert_eq!(report.remaining, vec![reference(ALPHA, "b")]);
}

fn a_state_is_closed_but_not_completed() -> bool {
    TaskState::Cancelled.is_closed() && !TaskState::Cancelled.is_completed()
}

#[test]
fn the_critical_path_is_the_longest_recorded_chain_into_open_work() {
    // a <- b <- c <- d, with d open and the others done but recorded.
    let chain = vec![
        row(ALPHA, "a", "s1", TaskState::Completed),
        row(ALPHA, "b", "s1", TaskState::Completed),
        row(ALPHA, "c", "s1", TaskState::Running),
        row(ALPHA, "d", "s1", TaskState::Ready),
    ];
    let report = task_graph_report(&inputs(
        chain.clone(),
        vec![
            owned("b", [edge(TaskDependencyKind::Requires, ALPHA, "a")]),
            owned("c", [edge(TaskDependencyKind::Requires, ALPHA, "b")]),
            owned("d", [edge(TaskDependencyKind::Requires, ALPHA, "c")]),
        ],
        Vec::new(),
        vec![
            (reference(ALPHA, "a"), TaskState::Completed),
            (reference(ALPHA, "b"), TaskState::Completed),
            (reference(ALPHA, "c"), TaskState::Running),
        ],
    ))
    .unwrap();
    assert_eq!(report.critical_path.length, 4);
    assert_eq!(report.critical_path.open, 2);
    assert!(!report.critical_path.truncated);
    assert_eq!(
        report
            .critical_path
            .chain
            .iter()
            .map(|task| task.task_id.as_str().to_string())
            .collect::<Vec<_>>(),
        vec!["a", "b", "c", "d"],
        "the chain is reported upstream first"
    );
    // The same state always names the same chain: a second read is identical.
    let again = task_graph_report(&inputs(
        chain,
        vec![
            owned("b", [edge(TaskDependencyKind::Requires, ALPHA, "a")]),
            owned("c", [edge(TaskDependencyKind::Requires, ALPHA, "b")]),
            owned("d", [edge(TaskDependencyKind::Requires, ALPHA, "c")]),
        ],
        Vec::new(),
        vec![
            (reference(ALPHA, "a"), TaskState::Completed),
            (reference(ALPHA, "b"), TaskState::Completed),
            (reference(ALPHA, "c"), TaskState::Running),
        ],
    ))
    .unwrap();
    assert_eq!(again.critical_path, report.critical_path);
    // A tie is resolved by canonical id order, never by read order.
    let tied = task_graph_report(&inputs(
        vec![
            row(ALPHA, "a", "s1", TaskState::Completed),
            row(ALPHA, "b", "s1", TaskState::Ready),
            row(ALPHA, "c", "s1", TaskState::Ready),
        ],
        vec![
            owned("b", [edge(TaskDependencyKind::Requires, ALPHA, "a")]),
            owned("c", [edge(TaskDependencyKind::Requires, ALPHA, "a")]),
        ],
        Vec::new(),
        vec![(reference(ALPHA, "a"), TaskState::Completed)],
    ))
    .unwrap();
    assert_eq!(tied.critical_path.length, 2);
    assert_eq!(
        tied.critical_path.chain.last(),
        Some(&reference(ALPHA, "b"))
    );
}

#[test]
fn an_incoming_block_is_read_as_a_dependency_of_the_task_it_blocks() {
    // `blocks` is stored on the blocking task, so the blocked task sees it as an
    // incoming edge: this is the inverted direction, read the one way the
    // store's completion gate reads it.
    let report = task_graph_report(&inputs(
        vec![row(ALPHA, "blocked", "s1", TaskState::Ready)],
        Vec::new(),
        vec![owned(
            "blocked",
            [edge(TaskDependencyKind::Blocks, ALPHA, "blocker")],
        )],
        vec![(reference(ALPHA, "blocker"), TaskState::Ready)],
    ))
    .unwrap();
    assert_eq!(report.blockers.len(), 1);
    assert_eq!(report.blockers[0].gates[0].kind, TaskDependencyKind::Blocks);
    assert_eq!(
        report.blockers[0].gates[0].target,
        reference(ALPHA, "blocker")
    );
    assert_eq!(report.critical_path.length, 2);
    assert_eq!(
        report.critical_path.chain,
        vec![reference(ALPHA, "blocker"), reference(ALPHA, "blocked")]
    );
}

#[test]
fn a_task_with_no_recorded_gate_is_still_a_chain_of_one() {
    let report = task_graph_report(&inputs(
        vec![row(ALPHA, "a", "s1", TaskState::Ready)],
        Vec::new(),
        Vec::new(),
        Vec::new(),
    ))
    .unwrap();
    assert_eq!(report.critical_path.length, 1);
    assert_eq!(report.critical_path.chain, vec![reference(ALPHA, "a")]);
    assert_eq!(report.critical_path.open, 1);
}

#[test]
fn a_cycle_in_the_rows_is_diagnosed_by_name_rather_than_walked() {
    // Writes reject cycles, so this is what corrupt stored rows look like to a
    // reader. The answer is a named refusal, never a walk of the loop.
    let report = task_graph_report(&inputs(
        vec![
            row(ALPHA, "a", "s1", TaskState::Ready),
            row(ALPHA, "b", "s1", TaskState::Ready),
        ],
        vec![
            owned("a", [edge(TaskDependencyKind::Requires, ALPHA, "b")]),
            owned("b", [edge(TaskDependencyKind::Requires, ALPHA, "a")]),
        ],
        Vec::new(),
        vec![
            (reference(ALPHA, "a"), TaskState::Ready),
            (reference(ALPHA, "b"), TaskState::Ready),
        ],
    ));
    assert_eq!(report, Err(DomainError::Cycle));
}

#[test]
fn an_edge_naming_a_task_that_is_not_there_is_refused() {
    let report = task_graph_report(&inputs(
        vec![row(ALPHA, "a", "s1", TaskState::Ready)],
        vec![owned(
            "a",
            [edge(TaskDependencyKind::Requires, ALPHA, "ghost")],
        )],
        Vec::new(),
        vec![],
    ));
    assert_eq!(report, Err(DomainError::MissingReference));
}

#[test]
fn overlap_names_startable_tasks_that_share_one_change_stream() {
    let report = task_graph_report(&inputs(
        vec![
            // Two startable tasks in one stream contend for it.
            row(ALPHA, "a", "s1", TaskState::Ready),
            row(ALPHA, "b", "s1", TaskState::Assigned),
            // A queued task is not a candidate, so it is not contention.
            row(ALPHA, "c", "s1", TaskState::Queued),
            // A startable task alone in its stream is not contention either.
            row(ALPHA, "d", "s2", TaskState::Ready),
        ],
        Vec::new(),
        Vec::new(),
        Vec::new(),
    ))
    .unwrap();
    assert_eq!(
        report.overlaps,
        vec![StreamOverlap {
            stream_id: stream("s1"),
            tasks: vec![task("a"), task("b")],
        }]
    );
}

#[test]
fn an_answer_is_bounded_and_says_when_it_is_partial() {
    let many: Vec<GraphTask> = (0..MAX_GRAPH_REPORT_TASKS + 5)
        .map(|index| row(ALPHA, &format!("task-{index:04}"), "s1", TaskState::Ready))
        .collect();
    // The store bounds the rows it reads; the domain refuses an answer that was
    // handed more than its own bound allows.
    assert_eq!(
        task_graph_report(&GraphInputs {
            project_id: project(ALPHA),
            total: 300,
            tasks: many,
            outgoing: BTreeMap::new(),
            incoming: BTreeMap::new(),
            referenced: BTreeMap::new(),
        })
        .err(),
        Some(DomainError::ResourceLimit)
    );
    // Within the bound, the caller's total stays beside the considered count,
    // so a partial answer is visible instead of silently complete.
    let partial = task_graph_report(&GraphInputs {
        project_id: project(ALPHA),
        total: 300,
        tasks: (0..2)
            .map(|index| row(ALPHA, &format!("task-{index:04}"), "s1", TaskState::Ready))
            .collect(),
        outgoing: BTreeMap::new(),
        incoming: BTreeMap::new(),
        referenced: BTreeMap::new(),
    })
    .unwrap();
    assert_eq!(partial.progress.total, 300);
    assert_eq!(partial.progress.considered, 2);
    // A total below the rows it answered is not an answer.
    assert_eq!(
        task_graph_report(&GraphInputs {
            project_id: project(ALPHA),
            total: 0,
            tasks: vec![row(ALPHA, "a", "s1", TaskState::Ready)],
            outgoing: BTreeMap::new(),
            incoming: BTreeMap::new(),
            referenced: BTreeMap::new(),
        })
        .err(),
        Some(DomainError::InvalidStream)
    );
}

#[test]
fn a_long_chain_is_listed_up_to_its_bound_and_says_it_was_cut() {
    let depth = MAX_CRITICAL_PATH_TASKS + 3;
    let mut tasks: Vec<GraphTask> = (0..depth)
        .map(|index| {
            row(
                ALPHA,
                &format!("task-{index:04}"),
                "s1",
                if index + 1 == depth {
                    TaskState::Ready
                } else {
                    TaskState::Completed
                },
            )
        })
        .collect();
    tasks.sort_by(|a, b| a.task_id.cmp(&b.task_id));
    let mut outgoing: Vec<(TaskId, BTreeSet<TaskDependencyEdge>)> = Vec::new();
    let mut referenced: Vec<(GraphTaskRef, TaskState)> = Vec::new();
    for index in 1..depth {
        outgoing.push(owned(
            &format!("task-{index:04}"),
            [edge(
                TaskDependencyKind::Requires,
                ALPHA,
                &format!("task-{:04}", index - 1),
            )],
        ));
    }
    for index in 0..depth {
        referenced.push((
            reference(ALPHA, &format!("task-{index:04}")),
            if index + 1 == depth {
                TaskState::Ready
            } else {
                TaskState::Completed
            },
        ));
    }
    let report = task_graph_report(&inputs(tasks, outgoing, Vec::new(), referenced)).unwrap();
    assert_eq!(report.critical_path.length, depth);
    assert_eq!(report.critical_path.chain.len(), MAX_CRITICAL_PATH_TASKS);
    assert!(report.critical_path.truncated);
    assert_eq!(report.critical_path.open, 1);
}

#[test]
fn the_dag_answer_bounds_are_the_ones_the_contract_states() {
    use symbiote_contract_read::{figure, region};

    let hierarchy = include_str!("../../../docs/contracts/work-hierarchy.md");
    for (label, stated, held) in [
        (
            "tasks in a DAG answer",
            figure::<u64>(region(hierarchy, "DAG answers are bounded to ", " tasks")),
            MAX_GRAPH_REPORT_TASKS as u64,
        ),
        (
            "gates in a DAG answer",
            figure::<u64>(region(hierarchy, " tasks and ", " gates")),
            MAX_GRAPH_REPORT_GATES as u64,
        ),
        (
            "members in a listed critical path",
            figure::<u64>(region(hierarchy, "itself capped at ", " members")),
            MAX_CRITICAL_PATH_TASKS as u64,
        ),
    ] {
        assert_eq!(
            stated, held,
            "the contract states {stated} {label}, and this crate bounds {held}"
        );
    }
}

#[test]
fn the_dag_contract_states_what_it_counts_and_what_it_never_invents() {
    let hierarchy = include_str!("../../../docs/contracts/work-hierarchy.md");
    for phrase in [
        // The derivation, stated as the absence of a reported number. Each
        // phrase is read from one line of the contract, never across a wrap.
        "percentage field and no way to express one",
        "from canonical state, rather than reporting a",
        // The five answers, each named.
        "**Progress**",
        "**Remaining closure**",
        "**Blockers**",
        "**Critical path**",
        "**Overlap**",
        // The three refusals a reader needs to trust it.
        "refuses by name (`cycle`) rather than walking it",
        "`not_found`",
        "a partial answer says so instead of presenting a",
        // The honesty rules this answer is held to.
        "cancelled prerequisite keeps its dependent on this list",
        "are deliberately",
        "it is computed, not unimplemented",
        "writes nothing: no",
    ] {
        assert!(
            hierarchy.contains(phrase),
            "the work-hierarchy contract does not state: {phrase}"
        );
    }
    // The bound the constant holds is the bound the document states, read by
    // the case above; this one holds the vocabulary to the same document.
    assert!(
        hierarchy.contains("`get_task_graph(project_id)`"),
        "the contract does not name the read that serves this answer"
    );
}

#[test]
fn the_report_round_trips_and_refuses_unknown_fields() {
    let report = task_graph_report(&inputs(
        vec![
            row(ALPHA, "a", "s1", TaskState::Ready),
            row(ALPHA, "b", "s1", TaskState::Running),
        ],
        vec![owned("b", [edge(TaskDependencyKind::Requires, ALPHA, "a")])],
        Vec::new(),
        vec![(reference(ALPHA, "a"), TaskState::Ready)],
    ))
    .unwrap();
    let serialized = serde_json::to_string(&report).unwrap();
    assert_eq!(
        serde_json::from_str::<TaskGraphReport>(&serialized).unwrap(),
        report
    );
    let extra = serialized.replace(
        "\"project_id\":\"alpha\"",
        "\"project_id\":\"alpha\",\"progress_percent\":80",
    );
    assert!(serde_json::from_str::<TaskGraphReport>(&extra).is_err());
}
