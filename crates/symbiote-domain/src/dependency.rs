//! Typed Task dependency edges. Edge records are separate first-class state:
//! a Task's pure `apply` cannot validate a graph spanning other aggregates,
//! and dependency edits are Lead/user operations, not lifecycle commands.
use crate::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const MAX_TASK_EDGES: usize = 64;
/// Upper bound for whole-graph validation, mirroring the work-hierarchy cap.
const MAX_GRAPH_TASKS: usize = 16_384;

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum TaskDependencyKind {
    /// This task cannot start until the target completes.
    Requires,
    /// This task consumes a versioned contract produced by the target.
    ConsumesContractFrom,
    /// This task blocks the target (stored on the blocking task).
    Blocks,
    /// This task independently reviews the target. Recorded, not yet enforced.
    Reviews,
    /// This task independently verifies the target. Recorded, not yet enforced.
    Verifies,
    /// This task supersedes the target. Recorded, not yet enforced.
    Supersedes,
    /// This task conflicts with the target. Recorded, not yet enforced.
    ConflictsWith,
    /// This task follows up on the target. Recorded, not yet enforced.
    FollowUpTo,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TaskDependencyTarget {
    pub project_id: ProjectId,
    pub task_id: TaskId,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TaskDependencyEdge {
    pub kind: TaskDependencyKind,
    pub target: TaskDependencyTarget,
}

/// The full edge set of one owning task. Replacement semantics: a later
/// `set` supersedes the previous set while the journal keeps history.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TaskDependencies {
    pub project_id: ProjectId,
    pub task_id: TaskId,
    pub edges: BTreeSet<TaskDependencyEdge>,
}

/// Kinds whose target must complete before the owning task may complete.
/// `blocks` is inverted: the edge is stored on the blocking task, so the
/// blocking check reads it as an incoming dependency of the target.
pub fn completion_blocking(kind: &TaskDependencyKind) -> bool {
    matches!(
        kind,
        TaskDependencyKind::Requires
            | TaskDependencyKind::ConsumesContractFrom
            | TaskDependencyKind::Blocks
    )
}

impl TaskDependencies {
    pub fn validate(&self) -> Result<(), DomainError> {
        if self.edges.len() > MAX_TASK_EDGES {
            return Err(DomainError::ResourceLimit);
        }
        for edge in &self.edges {
            if edge.target.task_id == self.task_id && edge.target.project_id == self.project_id {
                return Err(DomainError::Cycle);
            }
        }
        Ok(())
    }
}

/// Validates the global dependency graph: no self-edges, no dangling targets
/// (checked through `task_exists`), no cycles. Cross-Project edges are one
/// global graph, matching the work-hierarchy treatment of references.
pub fn validate_task_dependency_graph(
    edges: &BTreeMap<(ProjectId, TaskId), TaskDependencies>,
    task_exists: impl Fn(&ProjectId, &TaskId) -> bool,
) -> Result<(), DomainError> {
    if edges.len() > MAX_GRAPH_TASKS {
        return Err(DomainError::ResourceLimit);
    }
    // Adjacency: an edge is a waiting-on relation. For most kinds the owner
    // waits on the target; for `blocks` the target waits on the owner.
    let mut adjacency: BTreeMap<(ProjectId, TaskId), BTreeSet<(ProjectId, TaskId)>> =
        BTreeMap::new();
    let mut nodes: BTreeSet<(ProjectId, TaskId)> = BTreeSet::new();
    for ((project, task), dependencies) in edges {
        dependencies.validate()?;
        let owner = (project.clone(), task.clone());
        nodes.insert(owner.clone());
        for edge in &dependencies.edges {
            let target = (edge.target.project_id.clone(), edge.target.task_id.clone());
            if !task_exists(&target.0, &target.1) {
                return Err(DomainError::MissingReference);
            }
            nodes.insert(target.clone());
            let waiter = if matches!(edge.kind, TaskDependencyKind::Blocks) {
                target.clone()
            } else {
                owner.clone()
            };
            let blocked = if matches!(edge.kind, TaskDependencyKind::Blocks) {
                owner.clone()
            } else {
                target
            };
            adjacency.entry(waiter).or_default().insert(blocked);
        }
    }
    // Kahn's algorithm over the waiting-on edges.
    let mut indegree: BTreeMap<(ProjectId, TaskId), usize> = BTreeMap::new();
    for node in &nodes {
        indegree.entry(node.clone()).or_insert(0);
    }
    for blocked in adjacency.values() {
        for node in blocked {
            *indegree.entry(node.clone()).or_insert(0) += 1;
        }
    }
    let mut ready: Vec<_> = indegree
        .iter()
        .filter(|(_, degree)| **degree == 0)
        .map(|(node, _)| node.clone())
        .collect();
    let mut visited = 0;
    while let Some(node) = ready.pop() {
        visited += 1;
        if let Some(neighbors) = adjacency.get(&node) {
            for neighbor in neighbors {
                let degree = indegree.get_mut(neighbor).expect("adjacent node tracked");
                *degree -= 1;
                if *degree == 0 {
                    ready.push(neighbor.clone());
                }
            }
        }
    }
    if visited != nodes.len() {
        return Err(DomainError::Cycle);
    }
    Ok(())
}
