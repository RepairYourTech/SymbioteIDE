//! Canonical DAG answers: progress, the remaining closure, the tasks each
//! open Task waits on, and the critical path. Every field is counted from
//! recorded task rows and dependency edges — there is no caller-supplied or
//! agent-reported percentage anywhere in this module, and nothing here is
//! cached, so an answer cannot drift from the state it names.
//!
//! Readiness — which task may start now and what it waits on first — is
//! answered by the scheduling projection, which carries this module's answers
//! rather than a second copy of them. See [`TaskGate`] for the one rule about
//! what satisfies a gate.
use crate::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Tasks one DAG answer considers, in canonical id order. A Project holding
/// more is answered partially and says so: `progress.partial` is the flag, and
/// `progress.total` keeps the whole count beside the considered one.
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
    /// True when the answer did not read every task the Project holds. The
    /// flag rather than an arithmetic comparison a caller has to remember to
    /// make: an answer that dropped work must say so in the answer, and
    /// `critical_path.truncated` is about the chain, not about this.
    pub partial: bool,
}

/// The one rule about what satisfies a gate: completion alone. A cancelled
/// prerequisite is closed but produced nothing the dependent required, so the
/// dependent still waits; a `Failed` or `Interrupted` prerequisite is recovered
/// to `Ready` and still waits. Every reader of a gate — this module, the
/// scheduling projection, the write-time blocking check — asks this rather than
/// re-deriving it, so a change to what counts as delivered lands once.
pub fn gate_satisfied(target_state: &TaskState) -> bool {
    target_state.is_completed()
}

/// One gate a task waits on: the edge kind that orders the work, the canonical
/// task on the other side, and the state that task is in now. A gate is
/// satisfied by [`gate_satisfied`] alone.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TaskGate {
    pub kind: TaskDependencyKind,
    pub target: GraphTaskRef,
    pub target_state: TaskState,
}

/// The longest recorded chain of gating edges into the Project's open work.
/// With no effort or duration data recorded, length is measured in tasks, not
/// time. `length` counts every member of the chain and `open` those that are
/// not closed; a member that is closed is still named, because a closed link
/// in the recorded chain is history rather than a gap. The chain mixes the two
/// enforced relations on purpose: a `blocks` link holds the target's
/// *completion* rather than its start, so it lengthens the chain to delivery
/// without stopping the target from being started.
///
/// A Project holding no work that still needs scheduling has no critical path
/// to report, and `no_open_work` says so rather than leaving `length` at zero
/// to be read as a measurement of a chain that was found and found empty.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CriticalPath {
    /// The true chain length in tasks, always.
    pub length: usize,
    /// How many of the chain's members are not closed.
    pub open: usize,
    /// At most [`MAX_CRITICAL_PATH_TASKS`] members, upstream first.
    pub chain: Vec<GraphTaskRef>,
    /// True when `chain` is a prefix of a longer chain. This is about the
    /// chain alone; whether the answer read the whole Project is
    /// `progress.partial`.
    pub truncated: bool,
    /// True when every task the answer read is closed, so there is no open work
    /// for a chain to run through and `length` is zero because there is none,
    /// not because one was measured. Every other field is zero or empty when
    /// this is true, and a chain is named exactly when it is false.
    pub no_open_work: bool,
}

/// The #94 DAG answer for one Project: what is left and the chain that decides
/// the rest. The scheduling projection serves this beside the readiness answer,
/// which is the one place the per-task gates are published, so there is one
/// surface to read and each fact is carried once. A caller asking what blocks an
/// open task reads it off that task's `waiting_on` in the readiness list; the
/// gate is the same record either way, so publishing it twice would be two
/// copies to keep honest.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TaskGraphReport {
    pub project_id: ProjectId,
    pub progress: TaskProgress,
    /// The remaining closure: every considered task that is not closed, in
    /// canonical id order.
    pub remaining: Vec<GraphTaskRef>,
    pub critical_path: CriticalPath,
}

impl TaskGraphReport {
    /// Every Project this answer names besides its own: the other side of the
    /// remaining closure or of the critical path. A caller must hold a read on
    /// each of them before the answer is served, because a record with an
    /// unnamed other side is a reason the reader cannot act on. The readiness
    /// list names the same other sides and the Host authorizes over both.
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
        for task in &self.critical_path.chain {
            note(task);
        }
        projects
    }
}

/// The DAG and readiness answers for one Project, computed together because
/// they read the same gates: splitting them would mean walking the rows twice
/// and risking two answers to one question.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectAnswer {
    pub graph: TaskGraphReport,
    pub readiness: Vec<TaskReadiness>,
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
pub fn project_answer(inputs: &GraphInputs) -> Result<ProjectAnswer, DomainError> {
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
                // Read from the owner: `blocks` is stored on the blocking task
                // and holds the target's completion, not this task's start.
                if !orders_start(&edge.kind) {
                    continue;
                }
                found.push(gate(edge.kind, edge.target.clone())?);
            }
        }
        if let Some(edges) = incoming.get(&task.task_id) {
            for edge in edges {
                // Read from the target: the only edge that reaches a task this
                // way is the one holding its completion, and it is stored on
                // the blocking task.
                if !blocks_completion(&edge.kind) {
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

    let critical_path = critical_path_of(&ordered, &upstream_of, &depth, &state_of);

    // Readiness: every considered task, with the gates that are not satisfied
    // yet. This is the one place the per-task gates are published — a task that
    // is waiting on work is answered here rather than in a second list that
    // would carry the same gates. Published for closed tasks too, with an empty
    // list, because "this one is finished and nothing holds it" is the answer
    // for a task that has left the work — and publishing only the held ones
    // would make a caller infer the difference from a missing entry.
    let readiness: Vec<TaskReadiness> = ordered
        .iter()
        .map(|task| {
            let mut waiting_on: Vec<TaskGate> = gates
                .get(&task.task_id)
                .map(|task_gates| {
                    task_gates
                        .iter()
                        .filter(|gate| !gate_satisfied(&gate.target_state))
                        .cloned()
                        .collect()
                })
                .unwrap_or_default();
            waiting_on.sort();
            TaskReadiness {
                task: GraphTaskRef::of(task),
                state: task.state.clone(),
                recorded_gates: gates.get(&task.task_id).map_or(0, Vec::len),
                waiting_on,
            }
        })
        .collect();

    Ok(ProjectAnswer {
        graph: TaskGraphReport {
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
                // The flag, not a comparison the caller has to remember to make.
                partial: ordered.len() < *total,
            },
            remaining,
            critical_path,
        },
        readiness,
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
        // Nothing is open: there is no chain to report, and saying so is the
        // answer. A zero length here would read as a measurement.
        return CriticalPath {
            length: 0,
            open: 0,
            chain: Vec::new(),
            truncated: false,
            no_open_work: true,
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
        no_open_work: false,
    }
}

/// Kahn's algorithm over the waiting-on edges, in canonical order, refusing a
/// cycle by name instead of walking one. Every node is seeded, so a task with
/// no gates is a root rather than a missing node, and every gate target is
/// seeded as a node by the caller before this runs, so the two sets agree.
fn topological_order(
    nodes: &BTreeSet<GraphTaskRef>,
    upstream_of: &BTreeMap<GraphTaskRef, Vec<GraphTaskRef>>,
) -> Result<Vec<GraphTaskRef>, DomainError> {
    let mut outgoing: BTreeMap<GraphTaskRef, Vec<GraphTaskRef>> = BTreeMap::new();
    let mut indegree: BTreeMap<&GraphTaskRef, usize> =
        nodes.iter().map(|node| (node, 0usize)).collect();
    // Every node in `upstream_of` is one the caller seeded from a gate target,
    // so the two sets already agree and an edge cannot name a node that is not
    // there. A disagreement would leave a downstream node unpopped, and the
    // length check below names that as a cycle rather than walking it.
    for (downstream, upstreams) in upstream_of {
        for upstream in upstreams {
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
