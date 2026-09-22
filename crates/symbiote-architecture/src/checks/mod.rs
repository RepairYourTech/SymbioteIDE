//! The refusals this repository's ledger, workspace and contracts must survive.
//!
//! The entry points — [`problems`] for the ledger, the workspace and the
//! contracts, and [`governance`] for the repository's own archival record — and
//! one subject per file: the
//! ledger's own records and facts (`records`), the workspace artifacts and the
//! pins they carry (`artifacts`), the spike contracts and the runs that would
//! settle their choices (`contracts`), the transitions the states were reached
//! through (`history`), the immutable copy of what they rest on (`store`), and
//! the issue links they carry (`linkage`). Each rule exists because the tree
//! could otherwise claim something it does not support: that a record is still
//! what it was accepted from, that a workspace member needs no decision, that a
//! choice under proof is settled, that an unresolved choice has an owner, that a
//! dependency a provisional choice names reached implementation unrecorded, or
//! that a choice was settled by a run that never met the contract it was
//! measured against.
//!
//! A rule that cannot see what it is checking refuses rather than passes: a
//! record whose file is missing, a member cargo could not report, a fact whose
//! validity lapsed, a choice that names a proof without saying what it must
//! prove and a run that cannot be read are findings, not silence.

use crate::ledger::Ledger;
use crate::repository::Workspace;
use crate::spike::Contracts;
use std::path::Path;

mod artifacts;
mod contracts;
mod history;
mod linkage;
mod records;
mod store;

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
        records::record_problems(record, root, &mut problems);
        records::owner_problems(record, &mut problems);
        records::proof_problems(record, &mut problems);
    }
    records::fact_problems(ledger, now, &mut problems);
    artifacts::status_problems(&registry, ledger, now, &mut problems);
    artifacts::coverage_problems(ledger, workspace, &mut problems);
    contracts::contract_problems(ledger, contracts, root, &mut problems);
    problems
}

/// What this repository's own archival record must show: the history its states
/// were reached through, the immutable copy of what they rest on, and the issue
/// links that make them queryable from the roadmap side. The second entry point
/// for the same reason there are subjects at all: these read three files of this
/// repository that the ledger, workspace and contract rules do not, and a
/// fixture driving one of those rules carries no history to check.
pub fn governance(ledger: &Ledger, root: &Path) -> Vec<Problem> {
    let mut problems = Vec::new();
    history::history_problems(ledger, root, &mut problems);
    store::store_problems(ledger, root, &mut problems);
    linkage::linkage_problems(ledger, root, &mut problems);
    problems
}
