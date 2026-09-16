//! The refusals this repository's decision ledger must survive.
//!
//! Each rule exists because the ledger could otherwise claim something the tree
//! does not support: that a record is still what it was accepted from, that a
//! workspace member needs no decision, that a choice under proof is settled,
//! that an unresolved choice has an owner, that a dependency a provisional
//! choice names reached implementation unrecorded, or that a choice was settled
//! by a run that never met the contract it was measured against.
//!
//! A rule that cannot see what it is checking refuses rather than passes: a
//! record whose file is missing, a member cargo could not report, a fact whose
//! validity lapsed and a run that cannot be read are findings, not silence.

use crate::ledger::{DecisionRecord, Ledger};
use crate::repository::{Workspace, hash};
use crate::spike::{Contracts, Results, fingerprint};
use crate::{DecisionRegistry, DecisionState, GateStatus, SpikeContract};
use std::collections::BTreeSet;
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

/// Every refusal the committed ledger meets, in ledger order, then every
/// refusal its committed spike contracts meet. An empty result is the only
/// passing answer.
pub fn problems(
    ledger: &Ledger,
    contracts: &Contracts,
    workspace: &Workspace,
    root: &Path,
    now: u64,
) -> Vec<Problem> {
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
    contract_problems(ledger, contracts, root, &mut problems);
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

/// A contract is predeclared, owned by the issues the decision names, and
/// carries the run that settles it. A decision published accepted stands on a
/// complete in-threshold run of that contract, cited as its own evidence.
fn contract_problems(
    ledger: &Ledger,
    contracts: &Contracts,
    root: &Path,
    problems: &mut Vec<Problem>,
) {
    for contract in &contracts.contracts {
        let subject = format!("contract {}", contract.id);
        let Some(record) = ledger.decision(&contract.decision) else {
            problems.push(Problem::new(
                subject,
                format!(
                    "it settles {}, which this ledger does not hold, so nothing owns the choice it is written for",
                    contract.decision
                ),
            ));
            continue;
        };
        if record.proof_contract.as_deref() != Some(contract.id.as_str()) {
            problems.push(Problem::new(
                subject.clone(),
                format!(
                    "{} does not name this contract as its proof, so nothing would settle it by this run",
                    record.draft.id
                ),
            ));
        }
        clause_problems(contract, record, &subject, problems);
        run_problems(contract, record, root, &subject, problems);
    }
    for record in &ledger.decisions {
        let Some(id) = &record.proof_contract else {
            continue;
        };
        if !contracts.holds(id) {
            problems.push(Problem::new(
                format!("{}: proof contract", record.draft.id),
                format!("it is settled by {id}, which this repository does not hold"),
            ));
        }
    }
}

/// Every obligation a contract records names the issue it belongs to, and every
/// issue the decision names is addressed by one: an obligation nobody owns, or
/// an owner the contract never addresses, is the same gap one level down.
fn clause_problems(
    contract: &SpikeContract,
    record: &DecisionRecord,
    subject: &str,
    problems: &mut Vec<Problem>,
) {
    let owned: BTreeSet<String> = record
        .draft
        .issue_refs
        .iter()
        .map(|issue| format!("#{issue}"))
        .collect();
    let mut cited: BTreeSet<&str> = BTreeSet::new();
    for (kind, entry) in contract
        .workload
        .iter()
        .map(|entry| ("workload", entry))
        .chain(
            contract
                .stop_conditions
                .iter()
                .map(|entry| ("stop condition", entry)),
        )
    {
        match entry.split_once(':').map(|(tag, _)| tag.trim()) {
            Some(tag) if owned.contains(tag) => {
                cited.insert(tag);
            }
            _ => problems.push(Problem::new(
                subject,
                format!(
                    "the {kind} {entry:?} names no clause of {}, so nothing owns it (it belongs to one of {})",
                    record.draft.id,
                    owned.iter().cloned().collect::<Vec<_>>().join(", ")
                ),
            )),
        }
    }
    for issue in &owned {
        if !cited.contains(issue.as_str()) {
            problems.push(Problem::new(
                subject,
                format!(
                    "the contract addresses no obligation to {issue}, which {} names as its own",
                    record.draft.id
                ),
            ));
        }
    }
}

/// The run a contract points at: the committed artifact is the one it names,
/// and while the choice is not decided its identity alone holds. A decided
/// choice needs the run to be there, complete, within every predeclared
/// threshold, with nothing left untested — and cited as the record's evidence.
fn run_problems(
    contract: &SpikeContract,
    record: &DecisionRecord,
    root: &Path,
    subject: &str,
    problems: &mut Vec<Problem>,
) {
    let decided = matches!(
        record.published.state,
        DecisionState::Accepted | DecisionState::Superseded
    );
    if decided
        && !record
            .draft
            .evidence
            .iter()
            .any(|evidence| evidence.artifact == contract.result_artifact)
    {
        problems.push(Problem::new(
            subject,
            format!(
                "the record settles the choice on {} and does not cite it as its own evidence",
                contract.result_artifact
            ),
        ));
    }
    let committed = root.join(&contract.result_artifact);
    let results = match Results::read(&committed) {
        Ok(results) => Some(results),
        Err(error) => {
            if decided {
                problems.push(Problem::new(
                    subject,
                    format!(
                        "the choice is {} and no run settles it: {error}",
                        status_name(&record.published.state)
                    ),
                ));
            } else if committed.exists() {
                problems.push(Problem::new(
                    subject,
                    format!(
                        "the run it points at, {}, cannot be read: {error}",
                        contract.result_artifact
                    ),
                ));
            }
            None
        }
    };
    let Some(results) = results else {
        return;
    };
    let fingerprint = fingerprint(contract);
    let unmet = if decided {
        results.unmet(contract, &fingerprint)
    } else {
        results.identity_unmet(contract, &fingerprint)
    };
    for detail in unmet {
        problems.push(Problem::new(subject, detail));
    }
    for cited in &results.artifacts {
        match hash(root, &cited.artifact) {
            None => problems.push(Problem::new(
                subject,
                format!(
                    "the run cites raw artifact {}, which is not in the tree",
                    cited.artifact
                ),
            )),
            Some(actual) if actual != cited.sha256 => problems.push(Problem::new(
                subject,
                format!(
                    "the raw artifact {} has changed since the run cited it: it is {actual}, the result recorded {}",
                    cited.artifact, cited.sha256
                ),
            )),
            Some(_) => {}
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
