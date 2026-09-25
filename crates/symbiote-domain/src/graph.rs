//! Canonical DAG answers: progress, the remaining closure, the tasks each
//! open Task waits on, the critical path, and same-stream overlap. Every field
//! is counted from recorded task rows and dependency edges — there is no
//! caller-supplied or agent-reported percentage anywhere in this module, and
//! nothing here is cached, so an answer cannot drift from the state it names.
//!
//! Readiness (which task may start now, and what the scheduler's projection
//! refuses) is answered by that projection; this module answers the rest of
//! the #94 DAG surface and does not restate it.
use crate::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Tasks one DAG answer considers, in canonical id order. A Project holding
/// more is answered partially and says so: `progress.total` keeps the whole
/// count beside the considered one.
pub const MAX_GRAPH_REPORT_TASKS: usize = 256;
/// Gates (dependency edges that order work) one DAG answer reads. The bound
/// keeps the answer inside the protocol's own response limit rather than
/// letting a wide graph produce a document the daemon cannot serve.
pub const MAX_GRAPH_REPORT_GATES: usize = 2_048;
/// Chain members one critical path lists. `length` is always the true chain
/// length in tasks; `truncated` says whether the list is the whole chain.
pub const MAX_CRITICAL_PATH_TASKS: usize = 64;

/// One canonical task row, as a DAG answer needs it: identity, the stream it
/// mutates, and the state it is in. Nothing else about a Task can change an
/// answer here.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GraphTask {
    pub project_id: ProjectId,
    pub task_id: TaskId,
    pub stream_id: ChangeStreamId,
    pub state: TaskState,
}

/// One task an answer names, wherever it lives. Blockers and chain members may
/// name a task in another Project, which is why this is a project/task pair
/// rather than a bare task id.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GraphTaskRef {
    pub project_id: ProjectId,
    pub task_id: TaskId,
}

impl GraphTaskRef {
    pub fn of(task: &GraphTask) -> Self {
        Self {
            project_id: task.project_id.clone(),
            task_id: task.task_id.clone(),
        }
    }
}

/// The rows one DAG answer is computed from, resolved by the caller (the store)
/// from authoritative storage: `total` is the Project's own task count,
/// `tasks` the rows the answer considers, `outgoing` the edges those tasks own,
/// `incoming` the edges other tasks own that name them, and `referenced` the
/// canonical state of every task those edges name, in any Project.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphInputs {
    pub project_id: ProjectId,
    pub total: usize,
    pub tasks: Vec<GraphTask>,
    pub outgoing: BTreeMap<TaskId, BTreeSet<TaskDependencyEdge>>,
    pub incoming: BTreeMap<TaskId, BTreeSet<TaskDependencyEdge>>,
    pub referenced: BTreeMap<GraphTaskRef, TaskState>,
}

/// How many tasks sit in one canonical state.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TaskStateCount {
    pub state: TaskState,
    pub tasks: usize,
}

/// Progress, counted from canonical states. There is no percentage here and no
/// way to express one: a caller that wants a fraction divides these counts
/// itself, from states, rather than being handed a number it reported.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TaskProgress {
    /// Every task the Project holds, whether or not the answer considered it.
    pub total: usize,
    /// Task rows the answer read. Below `total` the answer is exact only over
    /// the considered tasks, which are the first in canonical id order.
    pub considered: usize,
    /// Gates read for the considered tasks.
    pub considered_gates: usize,
    /// The states present, in canonical lifecycle order. A state no task is in
    /// is absent rather than reported as a guessed zero.
    pub states: Vec<TaskStateCount>,
    /// Considered tasks that will never need scheduling again: `Completed` or
    /// `Cancelled`. `Failed` and `Interrupted` are not closed.
    pub closed: usize,
    /// Considered tasks that are not closed — the size of the remaining
    /// closure.
    pub open: usize,
}

/// One gate a task waits on: the edge kind that orders the work, the canonical
/// task on the other side, and the state that task is in now.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TaskGate {
    pub kind: TaskDependencyKind,
    pub target: GraphTaskRef,
    pub target_state: TaskState,
}

/// The gates currently holding one open task, and nothing else: a gate is
/// satisfied by completion alone, so a cancelled prerequisite keeps the
/// dependent on this list rather than reading as delivered work. Edge kinds the
/// store does not enforce (`reviews`, `verifies`, `supersedes`,
/// `conflicts_with`, `follow_up_to`) are deliberately absent, because reporting
/// them as blockers would enforce a policy their owning slice has not written
/// yet.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TaskBlockers {
    pub task: GraphTaskRef,
    pub gates: Vec<TaskGate>,
}

/// The longest recorded chain of gating edges into the Project's open work.
/// With no effort or duration data recorded, length is measured in tasks, not
/// time: this names the dependency chain that decides when the work behind it
/// can start, and says how much of it is already closed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CriticalPath {
    /// The true chain length in tasks, always.
    pub length: usize,
    /// How many of the chain's members are not closed.
    pub open: usize,
    /// At most [`MAX_CRITICAL_PATH_TASKS`] members, upstream first.
    pub chain: Vec<GraphTaskRef>,
    /// True when `chain` is a prefix of a longer chain.
    pub truncated: bool,
}

/// Startable tasks that share one Change Stream. Same-stream work is serialized
/// by policy, so two startable tasks in one stream cannot both run; a held
/// lease on the stream is the dynamic half of that fact and is refused by the
/// scheduler projection as `stream_leased`, not repeated here.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StreamOverlap {
    pub stream_id: ChangeStreamId,
    pub tasks: Vec<TaskId>,
}

/// The #94 DAG answer for one Project: what is left, what each open task waits
/// on, the chain that decides the rest, and where startable work contends for
/// the same stream.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TaskGraphReport {
    pub project_id: ProjectId,
    pub progress: TaskProgress,
    /// The remaining closure: every considered task that is not closed, in
    /// canonical id order.
    pub remaining: Vec<GraphTaskRef>,
    pub blockers: Vec<TaskBlockers>,
    pub critical_path: CriticalPath,
    pub overlaps: Vec<StreamOverlap>,
}

impl TaskGraphReport {
    /// Every Project this answer names besides its own: the other side of a
    /// blocker, of the remaining closure, or of the critical path. A caller must
    /// hold a read on each of them before the answer is served, because a gate
    /// with an unnamed other side is a reason the reader cannot act on.
    pub fn referenced_projects(&self) -> BTreeSet<ProjectId> {
        let mut projects: BTreeSet<ProjectId> = BTreeSet::new();
        let mut note = |task: &GraphTaskRef| {
            if task.project_id != self.project_id {
                projects.insert(task.project_id.clone());
            }
        };
        for task in &self.remaining {
            note(task);
        }
        for entry in &self.blockers {
            note(&entry.task);
            for gate in &entry.gates {
                note(&gate.target);
            }
        }
        for task in &self.critical_path.chain {
            note(task);
        }
        projects
    }
}

/// Answers the #94 DAG questions for one Project from canonical rows alone.
///
/// The waiting-on relation is the one the store already enforces: `requires`
/// and `consumes_contract_from` mean the owner waits on the target, and
/// `blocks` is stored on the blocking task and read as an incoming dependency
/// of the blocked target. The other five kinds are recorded with provenance but
/// are not ordering facts yet, so they cannot appear in a blocker, a chain or a
/// closure.
///
/// A graph that contains a cycle is refused by name (`Cycle`) rather than
/// walked: writes reject cycles, so a cycle read here means the stored rows and
/// the journal disagree, and any answer built on that would be a guess.
pub fn task_graph_report(inputs: &GraphInputs) -> Result<TaskGraphReport, DomainError> {
    let GraphInputs {
        project_id,
        total,
        tasks,
        outgoing,
        incoming,
        referenced,
    } = inputs;
    if tasks.len() > MAX_GRAPH_REPORT_TASKS
        || gate_count(outgoing, incoming) > MAX_GRAPH_REPORT_GATES
    {
        return Err(DomainError::ResourceLimit);
    }
    // The caller's own count has to hold the rows it supplied, or the report
    // would state a total below the work it just answered.
    if *total < tasks.len() {
        return Err(DomainError::InvalidStream);
    }
    // A subject Project's tasks, in canonical id order.
    let mut ordered: Vec<&GraphTask> = tasks.iter().collect();
    ordered.sort_by(|a, b| a.task_id.cmp(&b.task_id));
    if ordered.iter().any(|task| task.project_id != *project_id) {
        return Err(DomainError::LineageMismatch);
    }
    let own: BTreeMap<&TaskId, &TaskState> = ordered
        .iter()
        .map(|task| (&task.task_id, &task.state))
        .collect();
    // A considered row is the authority for a subject task; a task the answer
    // did not read but the caller resolved for a gate is still usable, because
    // the store read that row from its own Project.
    let state_of = |task: &GraphTaskRef| -> Option<TaskState> {
        if task.project_id == *project_id {
            own.get(&task.task_id)
                .map(|state| (*state).clone())
                .or_else(|| referenced.get(task).cloned())
        } else {
            referenced.get(task).cloned()
        }
    };

    // The gate on each subject task: the edge, the upstream task it waits on,
    // and that task's canonical state now. Both directions of the enforced
    // kinds, in canonical edge order.
    let mut gates: BTreeMap<TaskId, Vec<TaskGate>> = BTreeMap::new();
    let mut considered_gates = 0usize;
    for task in &ordered {
        let mut found: Vec<TaskGate> = Vec::new();
        let gate = |kind: TaskDependencyKind, target: TaskDependencyTarget| {
            let reference = GraphTaskRef {
                project_id: target.project_id,
                task_id: target.task_id,
            };
            let target_state = state_of(&reference).ok_or(DomainError::MissingReference)?;
            Ok(TaskGate {
                kind,
                target: reference,
                target_state,
            })
        };
        if let Some(edges) = outgoing.get(&task.task_id) {
            for edge in edges {
                // `blocks` is stored on the blocking task: it gates the target's
                // completion, not this task's start.
                if !orders_start(&edge.kind) {
                    continue;
                }
                found.push(gate(edge.kind, edge.target.clone())?);
            }
        }
        if let Some(edges) = incoming.get(&task.task_id) {
            for edge in edges {
                // Only an incoming `blocks` gates this task; the other kinds
                // name it as something other than work it waits on.
                if !matches!(edge.kind, TaskDependencyKind::Blocks) {
                    continue;
                }
                found.push(gate(edge.kind, edge.target.clone())?);
            }
        }
        if !found.is_empty() {
            considered_gates += found.len();
            gates.insert(task.task_id.clone(), found);
        }
    }

    // Upstream -> downstream over every gate, including the satisfied ones: the
    // chain a Project is inside is part of the record even where it is done.
    // Every subject task is a node, so a task with no gates is still a chain of
    // one rather than a missing answer.
    let mut nodes: BTreeSet<GraphTaskRef> =
        ordered.iter().map(|task| GraphTaskRef::of(task)).collect();
    let mut upstream_of: BTreeMap<GraphTaskRef, Vec<GraphTaskRef>> = BTreeMap::new();
    for task in &ordered {
        let downstream = GraphTaskRef::of(task);
        let Some(task_gates) = gates.get(&task.task_id) else {
            continue;
        };
        let mut upstreams: Vec<GraphTaskRef> =
            task_gates.iter().map(|gate| gate.target.clone()).collect();
        upstreams.sort();
        upstreams.dedup();
        for upstream in &upstreams {
            nodes.insert(upstream.clone());
        }
        upstream_of.insert(downstream, upstreams);
    }
    let order = topological_order(&nodes, &upstream_of)?;
    let depth = chain_depths(&upstream_of, &order);

    let mut states: BTreeMap<u8, (TaskState, usize)> = BTreeMap::new();
    let mut closed = 0usize;
    for task in &ordered {
        let count = states
            .entry(task.state.lifecycle_order())
            .or_insert_with(|| (task.state.clone(), 0));
        count.1 += 1;
        if task.state.is_closed() {
            closed += 1;
        }
    }

    let remaining: Vec<GraphTaskRef> = ordered
        .iter()
        .filter(|task| !task.state.is_closed())
        .map(|task| GraphTaskRef::of(task))
        .collect();

    let mut blockers: Vec<TaskBlockers> = Vec::new();
    for task in &remaining {
        let Some(task_gates) = gates.get(&task.task_id) else {
            continue;
        };
        let mut open: Vec<TaskGate> = task_gates
            .iter()
            .filter(|gate| !gate.target_state.is_completed())
            .cloned()
            .collect();
        if open.is_empty() {
            continue;
        }
        open.sort();
        blockers.push(TaskBlockers {
            task: task.clone(),
            gates: open,
        });
    }

    let critical_path = critical_path_of(&ordered, &upstream_of, &depth, &state_of);
    let mut streams: BTreeMap<ChangeStreamId, Vec<TaskId>> = BTreeMap::new();
    for task in &ordered {
        if task.state.is_startable() {
            streams
                .entry(task.stream_id.clone())
                .or_default()
                .push(task.task_id.clone());
        }
    }

    Ok(TaskGraphReport {
        project_id: project_id.clone(),
        progress: TaskProgress {
            total: *total,
            considered: ordered.len(),
            considered_gates,
            states: states
                .into_values()
                .map(|(state, tasks)| TaskStateCount { state, tasks })
                .collect(),
            closed,
            open: ordered.len() - closed,
        },
        remaining,
        blockers,
        critical_path,
        overlaps: streams
            .into_iter()
            .filter(|(_, tasks)| tasks.len() > 1)
            .map(|(stream_id, tasks)| StreamOverlap { stream_id, tasks })
            .collect(),
    })
}

/// Every dependency edge the two edge maps hold, the bound's input.
fn gate_count(
    outgoing: &BTreeMap<TaskId, BTreeSet<TaskDependencyEdge>>,
    incoming: &BTreeMap<TaskId, BTreeSet<TaskDependencyEdge>>,
) -> usize {
    outgoing.values().map(BTreeSet::len).sum::<usize>()
        + incoming.values().map(BTreeSet::len).sum::<usize>()
}

/// The longest recorded gating chain into the Project's open work: the open
/// task with the deepest recorded chain, ties resolved by canonical id order so
/// the same state always names the same chain. `length` counts every member,
/// `open` the members that are not closed, and the listed chain is bounded and
/// says whether it is whole.
fn critical_path_of(
    ordered: &[&GraphTask],
    upstream_of: &BTreeMap<GraphTaskRef, Vec<GraphTaskRef>>,
    depth: &BTreeMap<GraphTaskRef, usize>,
    state_of: &impl Fn(&GraphTaskRef) -> Option<TaskState>,
) -> CriticalPath {
    let deepest = |candidate: &GraphTask| {
        (
            depth
                .get(&GraphTaskRef::of(candidate))
                .copied()
                .unwrap_or(0),
            // Reversed, so the canonically smallest id wins the max.
            std::cmp::Reverse(candidate.task_id.clone()),
        )
    };
    let Some(leaf) = ordered
        .iter()
        .filter(|task| !task.state.is_closed())
        .max_by(|a, b| deepest(a).cmp(&deepest(b)))
    else {
        return CriticalPath {
            length: 0,
            open: 0,
            chain: Vec::new(),
            truncated: false,
        };
    };
    let mut chain: Vec<GraphTaskRef> = Vec::new();
    let mut cursor = GraphTaskRef::of(leaf);
    loop {
        chain.push(cursor.clone());
        if chain.len() == MAX_CRITICAL_PATH_TASKS {
            break;
        }
        let Some(upstreams) = upstream_of.get(&cursor) else {
            break;
        };
        let next = upstreams
            .iter()
            .max_by(|a, b| {
                (
                    depth.get(*a).copied().unwrap_or(0),
                    std::cmp::Reverse((*a).clone()),
                )
                    .cmp(&(
                        depth.get(*b).copied().unwrap_or(0),
                        std::cmp::Reverse((*b).clone()),
                    ))
            })
            .cloned();
        match next {
            Some(next) => cursor = next,
            None => break,
        }
    }
    // The chain was walked downstream-first; report it upstream first.
    chain.reverse();
    let length = depth.get(&GraphTaskRef::of(leaf)).copied().unwrap_or(0);
    CriticalPath {
        length,
        open: chain
            .iter()
            .filter(|task| state_of(task).is_some_and(|state| !state.is_closed()))
            .count(),
        truncated: length > chain.len(),
        chain,
    }
}

/// Kahn's algorithm over the waiting-on edges, in canonical order, refusing a
/// cycle by name instead of walking one. Every node is seeded, so a task with
/// no gates is a root rather than a missing node.
fn topological_order(
    nodes: &BTreeSet<GraphTaskRef>,
    upstream_of: &BTreeMap<GraphTaskRef, Vec<GraphTaskRef>>,
) -> Result<Vec<GraphTaskRef>, DomainError> {
    let mut outgoing: BTreeMap<GraphTaskRef, Vec<GraphTaskRef>> = BTreeMap::new();
    let mut indegree: BTreeMap<&GraphTaskRef, usize> =
        nodes.iter().map(|node| (node, 0usize)).collect();
    for (downstream, upstreams) in upstream_of {
        for upstream in upstreams {
            if !nodes.contains(upstream) || !nodes.contains(downstream) {
                return Err(DomainError::MissingReference);
            }
            outgoing
                .entry(upstream.clone())
                .or_default()
                .push(downstream.clone());
            *indegree.get_mut(downstream).expect("a seeded node") += 1;
        }
    }
    let mut ready: Vec<GraphTaskRef> = indegree
        .iter()
        .filter(|(_, degree)| **degree == 0)
        .map(|(node, _)| (*node).clone())
        .collect();
    let mut order: Vec<GraphTaskRef> = Vec::with_capacity(nodes.len());
    while let Some(node) = ready.pop() {
        if let Some(targets) = outgoing.get(&node) {
            for target in targets {
                let degree = indegree.get_mut(target).expect("a seeded node");
                *degree -= 1;
                if *degree == 0 {
                    ready.push(target.clone());
                }
            }
        }
        order.push(node);
    }
    if order.len() != nodes.len() {
        return Err(DomainError::Cycle);
    }
    Ok(order)
}

/// Longest chain ending at each node, in tasks, computed over the topological
/// order and preferring the canonically smallest predecessor on a tie so the
/// same state always names the same chain. A node with no recorded gate is a
/// chain of one.
fn chain_depths(
    upstream_of: &BTreeMap<GraphTaskRef, Vec<GraphTaskRef>>,
    order: &[GraphTaskRef],
) -> BTreeMap<GraphTaskRef, usize> {
    let deepest = |candidates: &[GraphTaskRef], depth: &BTreeMap<GraphTaskRef, usize>| {
        candidates
            .iter()
            .max_by(|a, b| {
                (
                    depth.get(*a).copied().unwrap_or(0),
                    std::cmp::Reverse((*a).clone()),
                )
                    .cmp(&(
                        depth.get(*b).copied().unwrap_or(0),
                        std::cmp::Reverse((*b).clone()),
                    ))
            })
            .cloned()
    };
    let mut depth: BTreeMap<GraphTaskRef, usize> = BTreeMap::new();
    for node in order {
        let upstream = upstream_of
            .get(node)
            .map(|upstream| deepest(upstream, &depth));
        depth.insert(
            node.clone(),
            match upstream.flatten() {
                Some(upstream) => depth.get(&upstream).copied().unwrap_or(0) + 1,
                None => 1,
            },
        );
    }
    depth
}
