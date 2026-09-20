//! What a decision's links into the issue program must resolve to.
//!
//! `issue_refs` and `requirement_refs` are how a decision is queryable from the
//! issue-generation side, and a `blocking_issue` is how a record says who owns
//! what it waits on. A link the program does not carry resolves to nothing, a
//! link to a reference entry names a pointer rather than the work, and a record
//! waiting on an issue the snapshot records closed is waiting on nothing.

use super::Problem;
use crate::ledger::Ledger;
use crate::roadmap::{REGISTRY_PATH, Roadmap};
use std::path::Path;

pub(super) fn linkage_problems(ledger: &Ledger, root: &Path, problems: &mut Vec<Problem>) {
    let roadmap = match Roadmap::read(&root.join(REGISTRY_PATH)) {
        Ok(roadmap) => roadmap,
        Err(error) => {
            problems.push(Problem::new(REGISTRY_PATH, error.0));
            return;
        }
    };
    for record in &ledger.decisions {
        let id = record.draft.id.as_str();
        for issue in &record.draft.issue_refs {
            match roadmap.issue(*issue) {
                None => problems.push(Problem::new(
                    format!("{id}: issue #{issue}"),
                    "the issue-generation registry carries no such issue, so the link names nothing the program owns",
                )),
                Some(found) if !roadmap.canonical(found) => problems.push(Problem::new(
                    format!("{id}: issue #{issue}"),
                    format!(
                        "the registry carries it as a {} pointing at {:?}, so a decision links the canonical issue rather than the pointer",
                        found.kind, found.canonical_issue
                    ),
                )),
                Some(_) => {}
            }
        }
        for key in &record.draft.requirement_refs {
            match roadmap.requirement(key) {
                None => problems.push(Problem::new(
                    format!("{id}: requirement {key}"),
                    "the issue-generation registry carries no such requirement, so the link names nothing the program owns",
                )),
                Some(found) if !roadmap.canonical(found) => problems.push(Problem::new(
                    format!("{id}: requirement {key}"),
                    format!(
                        "the registry carries it as a {} pointing at {:?}, so a decision links the canonical requirement",
                        found.kind, found.canonical_issue
                    ),
                )),
                Some(_) => {}
            }
        }
        waiting(
            &format!("{id}: blocking issue"),
            record.blocking_issue,
            &roadmap,
            problems,
        );
    }
    for artifact in &ledger.artifacts {
        waiting(
            artifact.pins.artifact.as_str(),
            artifact.blocking_issue,
            &roadmap,
            problems,
        );
    }
}

/// A record that waits on an issue the program does not carry, or on one the
/// snapshot records closed, is waiting on nothing.
fn waiting(subject: &str, issue: Option<u64>, roadmap: &Roadmap, problems: &mut Vec<Problem>) {
    let Some(issue) = issue else {
        return;
    };
    match roadmap.closed(issue) {
        None => problems.push(Problem::new(
            subject,
            format!("it waits on #{issue}, which the issue-generation registry does not carry"),
        )),
        Some(true) => problems.push(Problem::new(
            subject,
            format!(
                "it waits on #{issue}, which the issue-generation snapshot records closed, so it owns nothing that is still waited on"
            ),
        )),
        Some(false) => {}
    }
}
