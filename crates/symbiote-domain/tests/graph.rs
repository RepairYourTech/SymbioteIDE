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
        dropped_gates: 0,
    }
}

/// The DAG half of the one answer. Both halves come from a single
/// `project_answer` call, so a case reading them cannot pass by computing two
/// different things.
fn graph(inputs: GraphInputs) -> Result<TaskGraphReport, DomainError> {
    project_answer(&inputs).map(|answer| answer.graph)
}

/// The whole answer, for a case that reads the gates off the readiness list and
/// the rest off the DAG block.
fn answer(inputs: GraphInputs) -> ProjectAnswer {
    project_answer(&inputs).expect("the answer is computable")
}

/// The readiness half: what each considered task is waiting on.
fn readiness(inputs: GraphInputs) -> Vec<TaskReadiness> {
    project_answer(&inputs).unwrap().readiness
}

/// The gates one task is waiting on, read off an answer the way a caller reads
/// them. The readiness list is the one place the answer publishes them, so this
/// is not a private door — it is the only door.
fn waiting_on(answer: &ProjectAnswer, task_id: &str) -> Vec<TaskGate> {
    answer
        .readiness
        .iter()
        .find(|entry| entry.task.task_id.as_str() == task_id)
        .map(|entry| entry.waiting_on.clone())
        .unwrap_or_default()
}

#[test]
fn an_unenforced_edge_kind_cannot_become_a_gate() {
    // The five kinds the store records with provenance but does not enforce.
    // A gate is a scheduling decision, and a decision built on one of these
    // would enforce a policy the owning slice has not written.
    for kind in [
        TaskDependencyKind::Reviews,
        TaskDependencyKind::Verifies,
        TaskDependencyKind::Supersedes,
        TaskDependencyKind::ConflictsWith,
        TaskDependencyKind::FollowUpTo,
    ] {
        let report = answer(inputs(
            vec![row(ALPHA, "a", "s1", TaskState::Ready)],
            vec![owned("a", [edge(kind, ALPHA, "b")])],
            Vec::new(),
            vec![(reference(ALPHA, "b"), TaskState::Ready)],
        ));
        assert!(
            waiting_on(&report, "a").is_empty(),
            "{kind:?} must not gate anything"
        );
        assert!(
            report.graph.remaining.len() == 1,
            "{kind:?} must not gate anything"
        );
        assert_eq!(
            report.graph.critical_path.length, 1,
            "{kind:?} must not gate anything"
        );
    }
    // `blocks` is the one kind that reaches a task from the other side, and it
    // does gate: the rule is a relation per kind, not a blanket rule.
    let blocked = answer(inputs(
        vec![row(ALPHA, "b", "s1", TaskState::Ready)],
        Vec::new(),
        vec![(
            task("b"),
            std::iter::once(TaskDependencyEdge {
                kind: TaskDependencyKind::Blocks,
                target: TaskDependencyTarget {
                    project_id: project(ALPHA),
                    task_id: task("a"),
                },
            })
            .collect(),
        )],
        vec![(reference(ALPHA, "a"), TaskState::Ready)],
    ));
    assert_eq!(waiting_on(&blocked, "b").len(), 1);
    assert_eq!(
        waiting_on(&blocked, "b")[0].kind,
        TaskDependencyKind::Blocks
    );
}

#[test]
fn readiness_names_what_every_task_waits_on_including_the_ones_waiting_on_nothing() {
    // a <- b, with a cancelled, plus c and d holding no gate at all.
    let rows = readiness(inputs(
        vec![
            row(ALPHA, "a", "s1", TaskState::Cancelled),
            row(ALPHA, "b", "s1", TaskState::Ready),
            row(ALPHA, "c", "s1", TaskState::Queued),
            row(ALPHA, "d", "s1", TaskState::Completed),
        ],
        vec![owned("b", [edge(TaskDependencyKind::Requires, ALPHA, "a")])],
        Vec::new(),
        vec![(reference(ALPHA, "a"), TaskState::Cancelled)],
    ));
    // Every considered task is answered, not only the held ones: "nothing is in
    // the way" is the answer for a task, and publishing only the held tasks
    // would make a caller infer that from a missing entry.
    assert_eq!(
        rows.iter()
            .map(|entry| entry.task.task_id.clone())
            .collect::<Vec<_>>(),
        vec![task("a"), task("b"), task("c"), task("d")],
        "readiness answers for every considered task, in canonical order"
    );
    let by_task = |name: &str| {
        rows.iter()
            .find(|entry| entry.task.task_id == task(name))
            .unwrap()
    };
    assert_eq!(by_task("b").state, TaskState::Ready);
    assert_eq!(by_task("b").recorded_gates, 1);
    assert_eq!(by_task("b").waiting_on.len(), 1);
    assert_eq!(by_task("b").waiting_on[0].target.task_id, task("a"));
    assert_eq!(
        by_task("b").waiting_on[0].target_state,
        TaskState::Cancelled
    );
    // Nothing holds c or d, and the count says why their list is empty: c never
    // had a gate, d's chain is complete.
    assert!(by_task("c").waiting_on.is_empty());
    assert_eq!(by_task("c").recorded_gates, 0);
    assert!(by_task("d").waiting_on.is_empty());
    // A closed task is answered too, with nothing holding it: it has left the
    // work rather than being absent from the record.
    assert_eq!(by_task("a").state, TaskState::Cancelled);
    assert!(by_task("a").waiting_on.is_empty());
}

/// The whole answer, the way the Host serves it: the DAG block beside the
/// readiness list, with the lease half empty because this case asks about what
/// the answer names rather than what it schedules.
fn served(answer: ProjectAnswer) -> SchedulingProjection {
    SchedulingProjection {
        project_id: project(ALPHA),
        considered_at: Timestamp(30),
        schedulable: Vec::new(),
        blocked: Vec::new(),
        readiness: answer.readiness,
        dag: answer.graph,
    }
}

#[test]
fn a_gate_on_a_finished_task_still_names_the_project_it_waits_on() {
    // The Host refuses a scheduling answer that names a Project the caller
    // cannot read, so the set it refuses over has to cover every Project the
    // answer names. A task that has left the work is the case that slips: it is
    // in neither the remaining closure nor the critical path, and the gate it is
    // still waiting on is the only place its other Project is written down.
    let answer = answer(inputs(
        vec![row(ALPHA, "a", "s1", TaskState::Completed)],
        vec![owned(
            "a",
            [edge(TaskDependencyKind::Requires, BETA, "beta")],
        )],
        Vec::new(),
        vec![(reference(BETA, "beta"), TaskState::Running)],
    ));
    assert_eq!(waiting_on(&answer, "a").len(), 1, "the gate is published");
    // The graph half names nothing: the closure is empty and there is no chain.
    assert!(answer.graph.remaining.is_empty());
    assert!(answer.graph.critical_path.chain.is_empty());
    assert!(
        answer.graph.referenced_projects().is_empty(),
        "the graph half alone names no other Project, which is why it is not the set to authorize over"
    );
    // The whole answer does, and it is the whole answer the Host asks about.
    assert_eq!(
        served(answer).referenced_projects(),
        BTreeSet::from([project(BETA)]),
        "a gate on a finished task still names the Project it waits on"
    );
}

#[test]
fn readiness_and_the_blocker_list_come_from_one_computation_and_cannot_disagree() {
    let inputs = inputs(
        vec![
            row(ALPHA, "a", "s1", TaskState::Ready),
            row(ALPHA, "b", "s1", TaskState::Ready),
        ],
        vec![owned("b", [edge(TaskDependencyKind::Requires, ALPHA, "a")])],
        Vec::new(),
        vec![(reference(ALPHA, "a"), TaskState::Ready)],
    );
    let answer = project_answer(&inputs).unwrap();
    // The gate list is published once. What an open task is blocked on is its
    // `waiting_on` off the readiness list, so there is no second copy of the
    // same gate that could be built from a different walk.
    let from_readiness: Vec<(TaskId, TaskId, TaskDependencyKind)> = answer
        .readiness
        .iter()
        .flat_map(|entry| {
            entry.waiting_on.iter().map(move |gate| {
                (
                    entry.task.task_id.clone(),
                    gate.target.task_id.clone(),
                    gate.kind,
                )
            })
        })
        .collect();
    assert_eq!(
        from_readiness,
        vec![(task("b"), task("a"), TaskDependencyKind::Requires)]
    );
    // And the graph block carries no second copy of it: no gate at all, and no
    // field that would publish one.
    assert!(!format!("{:?}", answer.graph).contains("TaskGate"));
    let wire = serde_json::to_string(&answer.graph).unwrap();
    assert!(
        !wire.contains("\"blockers\""),
        "the gate list is published once, off the readiness list: {wire}"
    );
}

#[test]
fn progress_is_counted_from_canonical_states_and_carries_no_percentage() {
    let report = graph(inputs(
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
    let report = answer(inputs(
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
    ));
    // Two open tasks wait on something: d on c, and c on another Project's b.
    assert_eq!(waiting_on(&report, "d").len(), 1);
    // The satisfied `blocks` gate is history, not a blocker; the unenforced
    // `reviews` kind is not a gate at all.
    assert_eq!(
        waiting_on(&report, "d")
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
    assert_eq!(
        waiting_on(&report, "c")[0].target,
        reference(BETA, "b"),
        "a cross-Project gate is named, not hidden"
    );
}

#[test]
fn a_cancelled_prerequisite_still_gates_its_dependent() {
    let report = answer(inputs(
        vec![
            row(ALPHA, "a", "s1", TaskState::Cancelled),
            row(ALPHA, "b", "s1", TaskState::Ready),
        ],
        vec![owned("b", [edge(TaskDependencyKind::Requires, ALPHA, "a")])],
        Vec::new(),
        vec![(reference(ALPHA, "a"), TaskState::Cancelled)],
    ));
    assert!(a_state_is_closed_but_not_completed());
    assert_eq!(waiting_on(&report, "b").len(), 1);
    assert_eq!(
        waiting_on(&report, "b")[0].target_state,
        TaskState::Cancelled
    );
    // The closed prerequisite is out of the closure; the dependent is not out of
    // it, and it is still on the blocker list.
    assert_eq!(report.graph.remaining, vec![reference(ALPHA, "b")]);
}

fn a_state_is_closed_but_not_completed() -> bool {
    TaskState::Cancelled.is_closed() && !TaskState::Cancelled.is_completed()
}

#[test]
fn a_cancelled_chain_member_is_closed_and_never_counted_as_open_work() {
    // a <- b <- c, where a was cancelled and b finished anyway.
    let report = graph(inputs(
        vec![
            row(ALPHA, "a", "s1", TaskState::Cancelled),
            row(ALPHA, "b", "s1", TaskState::Completed),
            row(ALPHA, "c", "s1", TaskState::Ready),
        ],
        vec![
            owned("b", [edge(TaskDependencyKind::Requires, ALPHA, "a")]),
            owned("c", [edge(TaskDependencyKind::Requires, ALPHA, "b")]),
        ],
        Vec::new(),
        vec![
            (reference(ALPHA, "a"), TaskState::Cancelled),
            (reference(ALPHA, "b"), TaskState::Completed),
        ],
    ))
    .unwrap();
    assert_eq!(report.critical_path.length, 3);
    // "Not closed" is one fact across the whole report: the cancelled member is
    // out of the closure, out of `progress.open`, and out of the chain's `open`
    // count. Counting it open would answer that a withdrawn task is work still
    // to do, and would make the chain disagree with the progress beside it.
    assert_eq!(report.critical_path.open, 1);
    assert_eq!(report.progress.open, 1);
    assert_eq!(report.progress.closed, 2);
    assert_eq!(report.remaining, vec![reference(ALPHA, "c")]);
    // It is still named in the chain: a closed member of the longest recorded
    // chain is history, not a gap, and dropping it would shorten a real chain.
    assert_eq!(report.critical_path.chain.len(), 3);
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
    let report = graph(inputs(
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
    assert!(
        !report.critical_path.no_open_work,
        "a chain was measured, so this is not the no-open-work case"
    );
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
    let again = graph(inputs(
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
    let tied = graph(inputs(
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
fn a_project_with_nothing_open_reports_no_chain_rather_than_a_chain_of_zero() {
    // A finished Project. A chain cannot be measured here — there is no open
    // work for one to run through — so the answer has to say that rather than
    // report a length of zero, which reads as a measurement.
    let report = graph(inputs(
        vec![
            row(ALPHA, "a", "s1", TaskState::Completed),
            row(ALPHA, "b", "s1", TaskState::Cancelled),
        ],
        vec![owned("b", [edge(TaskDependencyKind::Requires, ALPHA, "a")])],
        Vec::new(),
        vec![(reference(ALPHA, "a"), TaskState::Completed)],
    ))
    .unwrap();
    assert_eq!(report.progress.open, 0);
    assert_eq!(report.progress.closed, 2);
    assert!(
        report.critical_path.no_open_work,
        "nothing open must be said, not measured as a zero-length chain"
    );
    assert_eq!(report.critical_path.length, 0);
    assert_eq!(report.critical_path.open, 0);
    assert!(report.critical_path.chain.is_empty());
    assert!(!report.critical_path.truncated);
    // A Project with open work says the opposite, so the flag is the fact and
    // not a constant.
    let open = graph(inputs(
        vec![row(ALPHA, "a", "s1", TaskState::Ready)],
        Vec::new(),
        Vec::new(),
        Vec::new(),
    ))
    .unwrap();
    assert!(
        !open.critical_path.no_open_work,
        "a lone open task is a chain of one, which is a measurement"
    );
    assert_eq!(open.critical_path.length, 1);
}

#[test]
fn an_incoming_block_is_read_as_a_dependency_of_the_task_it_blocks() {
    // `blocks` is stored on the blocking task, so the blocked task sees it as an
    // incoming edge: this is the inverted direction, read the one way the
    // store's completion gate reads it.
    let report = answer(inputs(
        vec![row(ALPHA, "blocked", "s1", TaskState::Ready)],
        Vec::new(),
        vec![owned(
            "blocked",
            [edge(TaskDependencyKind::Blocks, ALPHA, "blocker")],
        )],
        vec![(reference(ALPHA, "blocker"), TaskState::Ready)],
    ));
    assert_eq!(waiting_on(&report, "blocked").len(), 1);
    assert_eq!(
        waiting_on(&report, "blocked")[0].kind,
        TaskDependencyKind::Blocks
    );
    assert_eq!(
        waiting_on(&report, "blocked")[0].target,
        reference(ALPHA, "blocker")
    );
    assert_eq!(report.graph.critical_path.length, 2);
    assert_eq!(
        report.graph.critical_path.chain,
        vec![reference(ALPHA, "blocker"), reference(ALPHA, "blocked")]
    );
}

#[test]
fn a_task_with_no_recorded_gate_is_still_a_chain_of_one() {
    let report = graph(inputs(
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
    let report = graph(inputs(
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
    let report = graph(inputs(
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
fn an_answer_is_bounded_and_says_when_it_is_partial() {
    let many: Vec<GraphTask> = (0..MAX_GRAPH_REPORT_TASKS + 5)
        .map(|index| row(ALPHA, &format!("task-{index:04}"), "s1", TaskState::Ready))
        .collect();
    // The store bounds the rows it reads; the domain refuses an answer that was
    // handed more than its own bound allows.
    assert_eq!(
        graph(GraphInputs {
            project_id: project(ALPHA),
            total: 300,
            tasks: many,
            outgoing: BTreeMap::new(),
            incoming: BTreeMap::new(),
            referenced: BTreeMap::new(),
            dropped_gates: 0,
        })
        .err(),
        Some(DomainError::ResourceLimit)
    );
    // Within the bound, the caller's total stays beside the considered count,
    // so a partial answer is visible instead of silently complete.
    let partial = graph(GraphInputs {
        project_id: project(ALPHA),
        total: 300,
        tasks: (0..2)
            .map(|index| row(ALPHA, &format!("task-{index:04}"), "s1", TaskState::Ready))
            .collect(),
        outgoing: BTreeMap::new(),
        incoming: BTreeMap::new(),
        referenced: BTreeMap::new(),
        dropped_gates: 0,
    })
    .unwrap();
    assert_eq!(partial.progress.total, 300);
    assert_eq!(partial.progress.considered, 2);
    // The answer says so itself. Before the flag, a caller had to compare the
    // two counts to notice that four hundred of its tasks were missing, and the
    // one word "truncated" in the payload described the chain, not the report.
    assert!(
        partial.progress.partial,
        "an answer that read 2 of 300 rows must say it is partial"
    );
    assert!(
        !partial.critical_path.truncated && partial.progress.partial,
        "the chain bound and the report bound are different facts"
    );

    // A whole answer does not claim to be partial.
    let whole = graph(inputs(
        vec![row(ALPHA, "a", "s1", TaskState::Ready)],
        Vec::new(),
        Vec::new(),
        Vec::new(),
    ))
    .unwrap();
    assert!(!whole.progress.partial);
    assert_eq!(whole.progress.total, whole.progress.considered);
    // A total below the rows it answered is not an answer.
    assert_eq!(
        graph(GraphInputs {
            project_id: project(ALPHA),
            total: 0,
            tasks: vec![row(ALPHA, "a", "s1", TaskState::Ready)],
            outgoing: BTreeMap::new(),
            incoming: BTreeMap::new(),
            referenced: BTreeMap::new(),
            dropped_gates: 0,
        })
        .err(),
        Some(DomainError::InvalidStream)
    );
}

#[test]
fn a_gate_the_bound_cut_is_reported_by_the_answer_that_lost_it() {
    // Every row the Project holds was read, so the counts agree and the old
    // comparison — considered below total — says the answer is whole. It is
    // not: the caller could not fit every gate those rows hold, and the gates
    // it did not read are work the answer does not carry. The row itself
    // cannot be the tell either, because the one task whose gates were cut is
    // the first task, and taking it is what keeps the answer from being an
    // empty one.
    let report = graph(GraphInputs {
        project_id: project(ALPHA),
        total: 1,
        tasks: vec![row(ALPHA, "a", "s1", TaskState::Ready)],
        outgoing: BTreeMap::new(),
        incoming: BTreeMap::new(),
        referenced: BTreeMap::new(),
        dropped_gates: 5,
    })
    .unwrap();
    assert_eq!(report.progress.total, report.progress.considered);
    assert!(
        report.progress.partial,
        "an answer that could not read every gate it was given must say it is partial"
    );
    // The same answer with nothing cut says it is not partial, so the flag is
    // this answer's statement rather than a constant.
    let whole = graph(GraphInputs {
        project_id: project(ALPHA),
        total: 1,
        tasks: vec![row(ALPHA, "a", "s1", TaskState::Ready)],
        outgoing: BTreeMap::new(),
        incoming: BTreeMap::new(),
        referenced: BTreeMap::new(),
        dropped_gates: 0,
    })
    .unwrap();
    assert!(!whole.progress.partial);
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
    let report = graph(inputs(tasks, outgoing, Vec::new(), referenced)).unwrap();
    assert_eq!(report.critical_path.length, depth);
    assert_eq!(report.critical_path.chain.len(), MAX_CRITICAL_PATH_TASKS);
    assert!(report.critical_path.truncated);
    assert_eq!(report.critical_path.open, 1);
}

#[test]
fn the_dag_answer_bounds_are_the_ones_the_contract_states() {
    use symbiote_contract_read::{figure, region};

    let hierarchy = include_str!("../../../docs/contracts/work-hierarchy.md");
    // One sentence, read once, then split: anchoring the two figures on the
    // whole document matched an earlier "Tasks and" and read the wrong number.
    let bounds = region(hierarchy, "The answer is bounded to ", ":");
    for (label, stated, held) in [
        (
            "tasks in a DAG answer",
            figure::<u64>(region(bounds, "", " Tasks")),
            MAX_GRAPH_REPORT_TASKS as u64,
        ),
        (
            "gates in a DAG answer",
            figure::<u64>(region(bounds, " Tasks and ", " gates")),
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
        "divides these counts itself, from canonical state, rather than",
        // The four answers the request named, each served from one read.
        "**Readiness**",
        "**Schedulable and blocked**",
        "**Progress**",
        "**Remaining closure**",
        "**Blockers**",
        "**Critical path**",
        // The refusals and the honesty rules a reader needs to trust it.
        "refuses by name (`cycle`) rather than walking it",
        "`not_found`",
        "`progress.partial` says when the answer did not",
        "cancelled prerequisite keeps its dependent waiting",
        "are deliberately",
        "writes nothing: no",
        // One surface, one copy of each fact, and the reason the second read
        // and the second copy are gone.
        "is the one scheduling and DAG surface",
        "**Why one copy of each fact and not two.**",
        // The bound says which quantity it counts. Its measured cost is not
        // pinned here, and deliberately so: this crate cannot frame a response
        // body, and a case that only asserted the document said a number would
        // hold the sentence and not the fact. `symbiote-host`'s case builds that
        // shape through the real daemon and compares the figure stated there
        // against the bytes the transport actually framed.
        "the gate bound counts",
        "**gating edges**",
        // The one row the gate bound cuts rather than obeys, and why cutting it
        // still says the answer is partial rather than whole.
        "The first Task is the one row the gate",
        "its gates are cut to the budget",
        "not read every gate of the Tasks it did read",
        // No open work says so rather than reporting a measured zero.
        "`no_open_work` is true",
        // The removal is recorded rather than forgotten, and the removed field
        // is named as removed so a reader does not hunt for it.
        "**Same-stream overlap was removed, not deferred.**",
    ] {
        assert!(
            hierarchy.contains(phrase),
            "the work-hierarchy contract does not state: {phrase}"
        );
    }
    // The bound the constant holds is the bound the document states, read by
    // the case above; this one holds the vocabulary to the same document.
    assert!(
        hierarchy.contains("`get_scheduling_projection(project_id)`"),
        "the contract does not name the read that serves this answer"
    );
    // The removed read and the removed field are named as gone, so the
    // contract cannot be read as promising them.
    assert!(
        !hierarchy.contains("get_task_graph(project_id)` is"),
        "the contract still presents the removed read as the one that serves this"
    );
    // The duplicate gate list is named only as removed, so a reader who hunts
    // for `dag.blockers` finds the record of its deletion, not a promise.
    assert!(
        hierarchy.contains("v1.29 published a second `dag.blockers` list"),
        "the contract does not record the duplicate gate list as removed"
    );
    assert!(
        hierarchy.contains("the duplicate is gone"),
        "the contract does not say the duplicate was deleted rather than reconciled"
    );
}

#[test]
fn the_report_round_trips_and_refuses_unknown_fields() {
    let report = graph(inputs(
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
