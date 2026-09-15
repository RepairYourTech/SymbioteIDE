//! The refusals this repository's decision ledger must survive.
//!
//! Each rule exists because the ledger could otherwise claim something the tree
//! does not support: that a record is still what it was accepted from, that a
//! workspace member needs no decision, that a choice under proof is settled,
//! that an unresolved choice has an owner, or that a dependency a provisional
//! choice names reached implementation unrecorded.
//!
//! A rule that cannot see what it is checking refuses rather than passes: a
//! record whose file is missing, a member cargo could not report and a fact
//! whose validity lapsed are findings, not silence.

use crate::ledger::{DecisionRecord, Ledger};
use crate::repository::{Workspace, hash};
use crate::{DecisionRegistry, DecisionState, GateStatus};
use std::path::Path;

/// One thing this ledger must not claim, and what is wrong with it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Problem {
    pub subject: String,
    pub detail: String,
}

impl Problem {
    fn new(subject: impl Into<String>, detail: impl Into<String>) -> Problem {
        Problem {
            subject: subject.into(),
            detail: detail.into(),
        }
    }
}

/// Every refusal the committed ledger meets, in ledger order. An empty result
/// is the only passing answer.
pub fn problems(ledger: &Ledger, workspace: &Workspace, root: &Path, now: u64) -> Vec<Problem> {
    let registry = match ledger.registry() {
        Ok(registry) => registry,
        Err(error) => {
            return vec![Problem::new(
                "the decision ledger",
                format!("it does not replay through the policy registry: {error}"),
            )];
        }
    };
    let mut problems = Vec::new();
    for record in &ledger.decisions {
        record_problems(record, root, &mut problems);
        owner_problems(record, &mut problems);
    }
    for fact in &ledger.facts {
        if now >= fact.expires_at {
            problems.push(Problem::new(
                format!("compatibility fact {}", fact.id),
                format!(
                    "its validity lapsed at {} and it has not been refreshed",
                    fact.expires_at
                ),
            ));
        }
    }
    status_problems(&registry, ledger, now, &mut problems);
    coverage_problems(ledger, workspace, &mut problems);
    problems
}

/// A record is the text it was accepted from, and its evidence is the bytes it
/// names.
fn record_problems(record: &DecisionRecord, root: &Path, problems: &mut Vec<Problem>) {
    let id = record.draft.id.as_str();
    match hash(root, &record.record) {
        None => problems.push(Problem::new(
            format!("{id}: {}", record.record),
            "the accepted text this decision records is not in the tree",
        )),
        Some(actual) if actual != record.record_sha256 => problems.push(Problem::new(
            format!("{id}: {}", record.record),
            format!(
                "the record has changed since this decision cited it: it is {actual}, the ledger recorded {}",
                record.record_sha256
            ),
        )),
        Some(_) => {}
    }
    for evidence in &record.draft.evidence {
        if evidence.artifact.starts_with("https://") {
            continue;
        }
        match hash(root, &evidence.artifact) {
            None => problems.push(Problem::new(
                format!("{id}: {}", evidence.artifact),
                "the evidence this decision cites is not in the tree",
            )),
            Some(actual) if actual != evidence.sha256 => problems.push(Problem::new(
                format!("{id}: {}", evidence.artifact),
                format!(
                    "the evidence has changed since this decision cited it: it is {actual}, the ledger recorded {}",
                    evidence.sha256
                ),
            )),
            Some(_) => {}
        }
    }
}

/// A choice that is still open must name the issue that owns its proof, and a
/// record that is decided — accepted, superseded, rejected or retired — must not
/// carry an owner it no longer waits on.
fn owner_problems(record: &DecisionRecord, problems: &mut Vec<Problem>) {
    let id = record.draft.id.as_str();
    let open = matches!(
        record.published.state,
        DecisionState::Proposed | DecisionState::Investigating
    );
    if let Some(issue) = record.blocking_issue.filter(|_| !open) {
        problems.push(Problem::new(
            format!("{id}: blocking issue"),
            format!(
                "the record publishes {} and cannot still be blocked by #{issue}",
                status_name(&record.published.state),
            ),
        ));
    }
    if open && record.blocking_issue.is_none() {
        problems.push(Problem::new(
            format!("{id}: blocking issue"),
            format!(
                "the record publishes {} and names no issue that owns what decides it",
                status_name(&record.published.state)
            ),
        ));
    }
}

/// The status the ledger publishes for an artifact is the one the registry
/// reaches, and an artifact that is not settled names what keeps it so.
fn status_problems(
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
                    status_name(&artifact.status),
                    status_name(&reached.status),
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
                    status_name(&artifact.status)
                ),
            ));
        }
    }
}

/// Workspace membership both ways, and the reach of a provisional choice: a
/// member that declares a dependency a decision names must pin that decision,
/// and a recorded pin must still be visible in the member's own dependencies.
fn coverage_problems(ledger: &Ledger, workspace: &Workspace, problems: &mut Vec<Problem>) {
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

/// The vocabulary the ledger file and the gate result share, for messages.
fn status_name<T: serde::Serialize>(status: &T) -> String {
    serde_json::to_value(status)
        .ok()
        .and_then(|value| value.as_str().map(str::to_string))
        .unwrap_or_default()
}
