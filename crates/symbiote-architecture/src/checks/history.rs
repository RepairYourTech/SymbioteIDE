//! What the repository's own history must show for what it publishes.
//!
//! A published state with no recorded transition is a state nobody can audit; a
//! transition for a decision the ledger does not hold is a history of something
//! else; an acceptance with no recorded acceptance is a state whose authority
//! cannot be read back; and a journal that does not rebuild what the ledger
//! publishes has recovered nothing. Each is a finding rather than silence, and
//! the chain itself — order, continuity, each entry's own digest — is enforced
//! where the journal is read.

use super::Problem;
use crate::journal::{JOURNAL_PATH, Journal, Transition};
use crate::ledger::Ledger;
use crate::policy::DecisionState;
use std::path::Path;

pub(super) fn history_problems(ledger: &Ledger, root: &Path, problems: &mut Vec<Problem>) {
    let journal = match Journal::read(&root.join(JOURNAL_PATH)) {
        Ok(journal) => journal,
        Err(error) => {
            problems.push(Problem::new(JOURNAL_PATH, error.0));
            return;
        }
    };
    for record in &ledger.decisions {
        let id = record.draft.id.as_str();
        let subject = format!("{id}: {JOURNAL_PATH}");
        if !journal.records(id, Transition::Proposed) {
            problems.push(Problem::new(
                subject.clone(),
                format!(
                    "the record publishes {} and the history records no transition that proposed it",
                    record.published.state
                ),
            ));
        }
        if matches!(
            record.published.state,
            DecisionState::Accepted | DecisionState::Superseded
        ) && !journal.records(id, Transition::Accepted)
        {
            problems.push(Problem::new(
                subject,
                format!(
                    "the record publishes {} and the history records no acceptance that reached it",
                    record.published.state
                ),
            ));
        }
    }
    for entry in &journal.entries {
        if ledger.decision(&entry.subject).is_none() {
            problems.push(Problem::new(
                format!("entry {}: {}", entry.seq, entry.subject),
                "the journal records a transition for a decision the ledger does not hold",
            ));
        }
    }
    if let Err(error) = journal.replay(ledger) {
        problems.push(Problem::new(
            JOURNAL_PATH,
            format!("the journal does not recover what the ledger publishes: {error}"),
        ));
    }
}
