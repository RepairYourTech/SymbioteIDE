//! What the spike contracts, their obligations and their runs must show.
//!
//! A contract is predeclared and owned by the issues its decision names; every
//! obligation belongs to an owner and every owner is addressed; and a decided
//! choice stands on a complete, in-threshold run set that accounts for every
//! platform the contract applies to, cited as the record's own evidence.

use super::Problem;
use crate::ledger::{DecisionRecord, Ledger};
use crate::policy::DecisionState;
use crate::repository::hash;
use crate::spike::{Contracts, Results, SpikeContract, fingerprint};
use std::collections::BTreeSet;
use std::path::Path;

/// A contract is predeclared, owned by the issues the decision names, and
/// carries the run that settles it. A decision published accepted stands on a
/// complete in-threshold run of that contract, cited as its own evidence.
pub(super) fn contract_problems(
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
                        record.published.state
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
    for cited in results.runs.iter().flat_map(|run| &run.artifacts) {
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
