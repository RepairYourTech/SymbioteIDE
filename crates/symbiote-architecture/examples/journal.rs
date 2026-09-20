//! The only writer of this repository's committed history (#173).
//!
//! A decision that is proposed, revised or accepted is recorded by appending
//! one entry to the chain, naming the content the ledger holds for that
//! transition. Nothing here rewrites an entry or derives the history from the
//! ledger: an append is visible in the diff as the transition it records, and a
//! chain that no longer verifies is refused before anything is added.
//!
//! ```sh
//! cargo run -p symbiote-architecture --example journal -- proposed ADR-0001/SHELL 0
//! cargo run -p symbiote-architecture --example journal -- revised ADR-0001/SHELL 1
//! cargo run -p symbiote-architecture --example journal -- accepted ADR-0001/SHELL 2
//! ```

use std::process::ExitCode;
use symbiote_architecture::journal::{JOURNAL_PATH, Journal, Transition};
use symbiote_architecture::ledger::{LEDGER_PATH, Ledger};
use symbiote_architecture::workspace_root;

fn main() -> ExitCode {
    let root = workspace_root();
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [transition, subject, from] = args.as_slice() else {
        eprintln!(
            "usage: journal <proposed|revised|accepted> <decision> <from-revision>\n\
             a transition is recorded by appending one entry; the history is authored, not regenerated"
        );
        return ExitCode::FAILURE;
    };
    let transition = match transition.as_str() {
        "proposed" => Transition::Proposed,
        "revised" => Transition::Revised,
        "accepted" => Transition::Accepted,
        other => {
            eprintln!("{other} is not a transition this repository records");
            return ExitCode::FAILURE;
        }
    };
    let Ok(from) = from.parse::<u64>() else {
        eprintln!("{from:?} is not a revision");
        return ExitCode::FAILURE;
    };
    let ledger = match Ledger::read(&root.join(LEDGER_PATH)) {
        Ok(ledger) => ledger,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
    };
    let path = root.join(JOURNAL_PATH);
    let mut journal = match Journal::read(&path) {
        Ok(journal) => journal,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
    };
    let entry = match journal.append(&ledger, transition, subject, from) {
        Ok(entry) => entry,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
    };
    if let Err(error) = journal.replay(&ledger) {
        eprintln!("the history would not recover what the ledger publishes: {error}");
        return ExitCode::FAILURE;
    }
    if let Err(error) = journal.write(&path) {
        eprintln!("{error}");
        return ExitCode::FAILURE;
    }
    println!(
        "entry {} {} {} from {} to {} {}",
        entry.seq, entry.subject, entry.transition, entry.from, entry.to, entry.sha256
    );
    ExitCode::SUCCESS
}
