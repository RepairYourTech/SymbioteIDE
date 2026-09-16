//! What the workspace must show for what the ledger publishes.
//!
//! A published status is the one the registry reaches and an artifact that is
//! not settled names what keeps it so; membership holds in both directions; and
//! a dependency a provisional choice names cannot reach a member that never
//! pinned that choice.

use super::Problem;
use crate::ledger::Ledger;
use crate::policy::{DecisionRegistry, GateStatus};
use crate::repository::Workspace;

/// The status the ledger publishes for an artifact is the one the registry
/// reaches, and an artifact that is not settled names what keeps it so.
pub(super) fn status_problems(
    registry: &DecisionRegistry,
    ledger: &Ledger,
    now: u64,
    problems: &mut Vec<Problem>,
) {
    for artifact in &ledger.artifacts {
        let name = artifact.pins.artifact.as_str();
        let reached = registry.gate(name, now);
        if reached.status != artifact.status {
            problems.push(Problem::new(
                name,
                format!(
                    "the ledger publishes {} and the registry reaches {} ({})",
                    artifact.status,
                    reached.status,
                    reached.reasons.join("; ")
                ),
            ));
        }
        if artifact.status == GateStatus::Settled && artifact.blocking_issue.is_some() {
            problems.push(Problem::new(
                name,
                "a settled artifact cannot still name a blocking issue",
            ));
        }
        if artifact.status != GateStatus::Settled && artifact.blocking_issue.is_none() {
            problems.push(Problem::new(
                name,
                format!(
                    "the artifact is {} and names no issue that owns what stops it being ready",
                    artifact.status
                ),
            ));
        }
    }
}

/// Workspace membership both ways, and the reach of a provisional choice: a
/// member that declares a dependency a decision names must pin that decision,
/// and a recorded pin must still be visible in the member's own dependencies.
pub(super) fn coverage_problems(
    ledger: &Ledger,
    workspace: &Workspace,
    problems: &mut Vec<Problem>,
) {
    for member in workspace.paths() {
        if !ledger.artifacts.iter().any(|a| a.pins.artifact == member) {
            problems.push(Problem::new(
                member,
                "this workspace member pins no architecture decision; the ledger must record what it depends on",
            ));
        }
    }
    for artifact in &ledger.artifacts {
        let member = artifact.pins.artifact.as_str();
        if !workspace.contains(member) {
            problems.push(Problem::new(
                member,
                "the ledger pins an artifact cargo does not report as a workspace member",
            ));
        }
    }
    for record in &ledger.decisions {
        let id = record.draft.id.as_str();
        if record.proposed_dependencies.is_empty() {
            continue;
        }
        let names = record.proposed_dependencies.join(", ");
        for member in workspace.paths() {
            let declared = workspace.declares(member, &record.proposed_dependencies);
            if !declared.is_empty() && !pins(ledger, member, id) {
                problems.push(Problem::new(
                    member,
                    format!(
                        "it declares {}, which {} names as its own, so it must pin that decision",
                        declared.join(", "),
                        id
                    ),
                ));
            }
        }
        for artifact in &ledger.artifacts {
            let member = artifact.pins.artifact.as_str();
            if !pins(ledger, member, id) {
                continue;
            }
            if workspace
                .declares(member, &record.proposed_dependencies)
                .is_empty()
            {
                problems.push(Problem::new(
                    member,
                    format!(
                        "the ledger pins {id}, whose dependency names ({names}) this member does not declare"
                    ),
                ));
            }
        }
    }
}

fn pins(ledger: &Ledger, member: &str, id: &str) -> bool {
    ledger
        .artifacts
        .iter()
        .filter(|artifact| artifact.pins.artifact == member)
        .flat_map(|artifact| &artifact.pins.decisions)
        .any(|pin| pin.id == id)
}
