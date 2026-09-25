//! Typed Task dependency edges: kinds, replacement sets, and global DAG checks.
use std::collections::{BTreeMap, BTreeSet};
use symbiote_domain::*;

fn id_set(edges: &[(TaskDependencyKind, &str, &str)]) -> TaskDependencies {
    TaskDependencies {
        project_id: ProjectId::new("alpha").unwrap(),
        task_id: TaskId::new("owner").unwrap(),
        edges: edges
            .iter()
            .map(|(kind, project, task)| TaskDependencyEdge {
                kind: *kind,
                target: TaskDependencyTarget {
                    project_id: ProjectId::new((*project).to_string()).unwrap(),
                    task_id: TaskId::new((*task).to_string()).unwrap(),
                },
            })
            .collect(),
    }
}

fn graph(entries: &[TaskDependencies]) -> BTreeMap<(ProjectId, TaskId), TaskDependencies> {
    entries
        .iter()
        .map(|d| ((d.project_id.clone(), d.task_id.clone()), d.clone()))
        .collect()
}

fn exists(project: &ProjectId, task: &TaskId) -> bool {
    matches!(
        (project.as_str(), task.as_str()),
        ("alpha", "a" | "b" | "c" | "owner") | ("beta", "b1" | "b2")
    )
}

#[test]
fn all_kinds_round_trip_serde() {
    for kind in [
        TaskDependencyKind::Requires,
        TaskDependencyKind::ConsumesContractFrom,
        TaskDependencyKind::Blocks,
        TaskDependencyKind::Reviews,
        TaskDependencyKind::Verifies,
        TaskDependencyKind::Supersedes,
        TaskDependencyKind::ConflictsWith,
        TaskDependencyKind::FollowUpTo,
    ] {
        let edge = TaskDependencyEdge {
            kind,
            target: TaskDependencyTarget {
                project_id: ProjectId::new("alpha").unwrap(),
                task_id: TaskId::new("a").unwrap(),
            },
        };
        let serialized = serde_json::to_string(&edge).unwrap();
        assert_eq!(
            serde_json::from_str::<TaskDependencyEdge>(&serialized).unwrap(),
            edge
        );
    }
    // Completion blocking is a policy statement, tested per kind.
    assert!(completion_blocking(&TaskDependencyKind::Requires));
    assert!(completion_blocking(
        &TaskDependencyKind::ConsumesContractFrom
    ));
    assert!(completion_blocking(&TaskDependencyKind::Blocks));
    assert!(!completion_blocking(&TaskDependencyKind::Reviews));
    assert!(!completion_blocking(&TaskDependencyKind::FollowUpTo));
}

#[test]
fn self_edges_are_cycles() {
    let mut dependencies = id_set(&[(TaskDependencyKind::Requires, "alpha", "other")]);
    assert!(dependencies.validate().is_ok());
    dependencies.task_id = TaskId::new("other").unwrap();
    assert_eq!(dependencies.validate(), Err(DomainError::Cycle));
}

#[test]
fn edge_cap_is_enforced() {
    let edges: Vec<_> = (0..65)
        .map(|n| {
            let name = format!("t{n}");
            TaskDependencyEdge {
                kind: TaskDependencyKind::Requires,
                target: TaskDependencyTarget {
                    project_id: ProjectId::new("alpha").unwrap(),
                    task_id: TaskId::new(&name).unwrap(),
                },
            }
        })
        .collect();
    let dependencies = TaskDependencies {
        project_id: ProjectId::new("alpha").unwrap(),
        task_id: TaskId::new("owner").unwrap(),
        edges: edges.into_iter().collect(),
    };
    assert_eq!(dependencies.validate(), Err(DomainError::ResourceLimit));
}

#[test]
fn linear_diamond_and_cross_project_graphs_are_valid() {
    // owner -> a -> b (chain)
    let chain = graph(&[
        id_set(&[(TaskDependencyKind::Requires, "alpha", "a")]),
        TaskDependencies {
            project_id: ProjectId::new("alpha").unwrap(),
            task_id: TaskId::new("a").unwrap(),
            edges: BTreeSet::from([TaskDependencyEdge {
                kind: TaskDependencyKind::Requires,
                target: TaskDependencyTarget {
                    project_id: ProjectId::new("alpha").unwrap(),
                    task_id: TaskId::new("b").unwrap(),
                },
            }]),
        },
    ]);
    assert!(validate_task_dependency_graph(&chain, exists).is_ok());
    // Diamond: owner requires a and b; both require c.
    let diamond = graph(&[
        id_set(&[
            (TaskDependencyKind::Requires, "alpha", "a"),
            (TaskDependencyKind::Requires, "alpha", "c"),
        ]),
        TaskDependencies {
            project_id: ProjectId::new("alpha").unwrap(),
            task_id: TaskId::new("a").unwrap(),
            edges: BTreeSet::from([TaskDependencyEdge {
                kind: TaskDependencyKind::Requires,
                target: TaskDependencyTarget {
                    project_id: ProjectId::new("alpha").unwrap(),
                    task_id: TaskId::new("c").unwrap(),
                },
            }]),
        },
    ]);
    assert!(validate_task_dependency_graph(&diamond, exists).is_ok());
    // Cross-Project edge is part of one global graph.
    let cross = graph(&[id_set(&[(
        TaskDependencyKind::ConsumesContractFrom,
        "beta",
        "b1",
    )])]);
    assert!(validate_task_dependency_graph(&cross, exists).is_ok());
}

#[test]
fn cycles_and_missing_targets_are_rejected() {
    // Two-cycle through requires.
    let two = graph(&[
        id_set(&[(TaskDependencyKind::Requires, "alpha", "a")]),
        TaskDependencies {
            project_id: ProjectId::new("alpha").unwrap(),
            task_id: TaskId::new("a").unwrap(),
            edges: BTreeSet::from([TaskDependencyEdge {
                kind: TaskDependencyKind::Requires,
                target: TaskDependencyTarget {
                    project_id: ProjectId::new("alpha").unwrap(),
                    task_id: TaskId::new("owner").unwrap(),
                },
            }]),
        },
    ]);
    assert_eq!(
        validate_task_dependency_graph(&two, exists),
        Err(DomainError::Cycle)
    );
    // Three-cycle.
    let three = graph(&[
        id_set(&[(TaskDependencyKind::Requires, "alpha", "a")]),
        TaskDependencies {
            project_id: ProjectId::new("alpha").unwrap(),
            task_id: TaskId::new("a").unwrap(),
            edges: BTreeSet::from([TaskDependencyEdge {
                kind: TaskDependencyKind::Requires,
                target: TaskDependencyTarget {
                    project_id: ProjectId::new("alpha").unwrap(),
                    task_id: TaskId::new("b").unwrap(),
                },
            }]),
        },
        TaskDependencies {
            project_id: ProjectId::new("alpha").unwrap(),
            task_id: TaskId::new("b").unwrap(),
            edges: BTreeSet::from([TaskDependencyEdge {
                kind: TaskDependencyKind::Requires,
                target: TaskDependencyTarget {
                    project_id: ProjectId::new("alpha").unwrap(),
                    task_id: TaskId::new("owner").unwrap(),
                },
            }]),
        },
    ]);
    assert_eq!(
        validate_task_dependency_graph(&three, exists),
        Err(DomainError::Cycle)
    );
    // Dangling target.
    let dangling = graph(&[id_set(&[(TaskDependencyKind::Requires, "alpha", "ghost")])]);
    assert_eq!(
        validate_task_dependency_graph(&dangling, exists),
        Err(DomainError::MissingReference)
    );
}

#[test]
fn the_domain_owns_which_kinds_are_enforced_and_what_satisfies_one() {
    // The one table: what each kind means, and what a state has to be before it
    // satisfies a gate. The store's write path and the DAG answer both read
    // these and neither writes them out, so a change here lands in both at
    // once; the store's own cases watch them from the other end.
    for kind in [
        TaskDependencyKind::Requires,
        TaskDependencyKind::ConsumesContractFrom,
    ] {
        assert_eq!(
            dependency_relation(&kind),
            DependencyRelation::Start,
            "{kind:?} holds its owner back from starting"
        );
        assert!(orders_start(&kind), "{kind:?} is a start relation");
        assert!(
            !blocks_completion(&kind),
            "{kind:?} says nothing about the target's completion"
        );
        assert!(
            completion_blocking(&kind),
            "{kind:?} gates from either side"
        );
    }
    assert_eq!(
        dependency_relation(&TaskDependencyKind::Blocks),
        DependencyRelation::Completion,
        "a blocks edge holds the target back from completing"
    );
    assert!(blocks_completion(&TaskDependencyKind::Blocks));
    assert!(
        !orders_start(&TaskDependencyKind::Blocks),
        "a blocks edge does not hold its own owner back"
    );
    assert!(completion_blocking(&TaskDependencyKind::Blocks));
    for kind in [
        TaskDependencyKind::Reviews,
        TaskDependencyKind::Verifies,
        TaskDependencyKind::Supersedes,
        TaskDependencyKind::ConflictsWith,
        TaskDependencyKind::FollowUpTo,
    ] {
        assert_eq!(
            dependency_relation(&kind),
            DependencyRelation::Provenance,
            "{kind:?} records why two tasks are related and gates nothing"
        );
        assert!(!orders_start(&kind) && !blocks_completion(&kind));
        assert!(!completion_blocking(&kind), "{kind:?} must not gate");
    }
    // Completion alone satisfies a gate. Cancelled is closed but delivered
    // nothing the dependent required, so the dependent still waits.
    assert!(gate_satisfied(&TaskState::Completed));
    for state in [
        TaskState::Ready,
        TaskState::Queued,
        TaskState::Assigned,
        TaskState::Blocked,
        TaskState::Running,
        TaskState::CompletionRequested,
        TaskState::Verifying,
        TaskState::Failed,
        TaskState::Cancelled,
        TaskState::Interrupted,
    ] {
        assert!(
            !gate_satisfied(&state),
            "{state:?} has not satisfied a gate"
        );
    }
}

#[test]
fn blocks_edges_wait_in_the_inverse_direction() {
    // owner blocks a: a waits on owner. A blocks-cycle is still a cycle.
    let blocking = graph(&[
        id_set(&[(TaskDependencyKind::Blocks, "alpha", "a")]),
        TaskDependencies {
            project_id: ProjectId::new("alpha").unwrap(),
            task_id: TaskId::new("a").unwrap(),
            edges: BTreeSet::from([TaskDependencyEdge {
                kind: TaskDependencyKind::Blocks,
                target: TaskDependencyTarget {
                    project_id: ProjectId::new("alpha").unwrap(),
                    task_id: TaskId::new("owner").unwrap(),
                },
            }]),
        },
    ]);
    assert_eq!(
        validate_task_dependency_graph(&blocking, exists),
        Err(DomainError::Cycle)
    );
    // One-directional blocking is valid.
    let one_way = graph(&[id_set(&[(TaskDependencyKind::Blocks, "alpha", "a")])]);
    assert!(validate_task_dependency_graph(&one_way, exists).is_ok());
}
