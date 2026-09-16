//! What the ledger says about its own records and facts.
//!
//! The record is the text it was accepted from and its evidence is the bytes it
//! names, so a record whose file moved or changed is a finding; a choice that
//! names the proof it waits on has to name where that proof's bar lives, in its
//! own accepted text; and a compatibility fact is only as good as the window it
//! was observed in.

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

/// A choice's proof has one owner on each side: the choice names the section of
/// its own accepted text that states what a proof of it must do, and the
/// contract answers that section's clauses. A section belongs to the choice
/// rather than to the contract, so a contract cannot pick the clauses it is
/// settled against out of the record belonging to the choice; and a section no
/// contract answers is a bar nothing is held to.
pub(super) fn proof_problems(record: &DecisionRecord, problems: &mut Vec<Problem>) {
    let id = record.draft.id.as_str();
    match (&record.proof_contract, &record.proof_section) {
        (Some(_), None) => problems.push(Problem::new(
            format!("{id}: proof section"),
            "it names a proof contract and no section of its accepted text that says what that proof must do, so the contract would state its own bar",
        )),
        (None, Some(section)) => problems.push(Problem::new(
            format!("{id}: proof section"),
            format!(
                "it names the {section:?} section of its accepted text as the bar its proof must answer, and no contract answers it"
            ),
        )),
        _ => {}
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
