//! What the ledger says about its own records and facts.
//!
//! The record is the text it was accepted from and its evidence is the bytes it
//! names, so a record whose file moved or changed is a finding; and a
//! compatibility fact is only as good as the window it was observed in.

use super::Problem;
use crate::ledger::{DecisionRecord, Ledger};
use crate::policy::DecisionState;
use crate::repository::hash;
use std::path::Path;

/// A record is the text it was accepted from, and its evidence is the bytes it
/// names.
pub(super) fn record_problems(record: &DecisionRecord, root: &Path, problems: &mut Vec<Problem>) {
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
pub(super) fn owner_problems(record: &DecisionRecord, problems: &mut Vec<Problem>) {
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
                record.published.state,
            ),
        ));
    }
    if open && record.blocking_issue.is_none() {
        problems.push(Problem::new(
            format!("{id}: blocking issue"),
            format!(
                "the record publishes {} and names no issue that owns what decides it",
                record.published.state
            ),
        ));
    }
}

/// A compatibility fact that outlived its observation window is a claim the
/// repository has not refreshed.
pub(super) fn fact_problems(ledger: &Ledger, now: u64, problems: &mut Vec<Problem>) {
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
}
