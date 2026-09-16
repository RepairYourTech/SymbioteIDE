//! The published decision map of this repository (#173): every recorded
//! decision, every artifact that pins one, every predeclared spike contract and
//! the run that would settle the choice it belongs to, and every refusal the
//! ledger meets.
//!
//! ```sh
//! cargo run -p symbiote-architecture --example decisions
//! ```
//!
//! The exit status is the verdict, so the same read that a maintainer inspects
//! is also the check: a refused ledger prints the refusals and fails.

use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};
use symbiote_architecture::checks::problems;
use symbiote_architecture::ledger::{LEDGER_PATH, Ledger};
use symbiote_architecture::repository::Workspace;
use symbiote_architecture::spike::{CONTRACTS_PATH, Contracts};
use symbiote_architecture::workspace_root;

fn main() -> ExitCode {
    let root = workspace_root();
    let ledger = match Ledger::read(&root.join(LEDGER_PATH)) {
        Ok(ledger) => ledger,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
    };
    let registry = match ledger.registry() {
        Ok(registry) => registry,
        Err(error) => {
            eprintln!("the decision ledger does not replay: {error}");
            return ExitCode::FAILURE;
        }
    };
    let contracts = match Contracts::read(&root.join(CONTRACTS_PATH)) {
        Ok(contracts) => contracts,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
    };
    let workspace = match Workspace::read(&root) {
        Ok(workspace) => workspace,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
    };
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or(0);

    for record in &ledger.decisions {
        let issues = record
            .draft
            .issue_refs
            .iter()
            .map(|issue| format!("#{issue}"))
            .collect::<Vec<_>>()
            .join(" ");
        println!(
            "decision {} {} rev {} {}{} record {}",
            record.draft.id,
            name(&record.published.state),
            record.published.revision,
            issues,
            blocking(record.blocking_issue, " blocking"),
            record.record,
        );
    }
    for artifact in &ledger.artifacts {
        let reached = registry.gate(&artifact.pins.artifact, now);
        let pins = artifact
            .pins
            .decisions
            .iter()
            .map(|pin| format!("{}@{}", pin.id, pin.revision))
            .collect::<Vec<_>>()
            .join(", ");
        let reasons = if reached.reasons.is_empty() {
            String::new()
        } else {
            format!(" reasons {}", reached.reasons.join("; "))
        };
        println!(
            "artifact {} {} pins {}{}{}",
            artifact.pins.artifact,
            name(&artifact.status),
            pins,
            blocking(artifact.blocking_issue, " blocking"),
            reasons,
        );
    }

    for contract in &contracts.contracts {
        let run = if root.join(&contract.result_artifact).exists() {
            "recorded"
        } else {
            "not run"
        };
        println!(
            "contract {} settles {}: {} predeclared measurements, {} stop conditions, run {} {run}",
            contract.id,
            contract.decision,
            contract.measurements.len(),
            contract.stop_conditions.len(),
            contract.result_artifact,
        );
    }

    let found = problems(&ledger, &contracts, &workspace, &root, now);
    for problem in &found {
        println!("refused {}: {}", problem.subject, problem.detail);
    }
    println!(
        "{}, {}, {}, {} workspace members, {} refusals",
        counted(ledger.decisions.len(), "decision"),
        counted(ledger.artifacts.len(), "artifact"),
        counted(contracts.contracts.len(), "contract"),
        workspace.paths().count(),
        found.len(),
    );
    if found.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn counted(count: usize, noun: &str) -> String {
    if count == 1 {
        format!("1 {noun}")
    } else {
        format!("{count} {noun}s")
    }
}

fn blocking(issue: Option<u64>, prefix: &str) -> String {
    issue.map_or_else(String::new, |issue| format!("{prefix} #{issue}"))
}

fn name<T: serde::Serialize>(state: &T) -> String {
    serde_json::to_value(state)
        .ok()
        .and_then(|value| value.as_str().map(str::to_string))
        .unwrap_or_default()
}
