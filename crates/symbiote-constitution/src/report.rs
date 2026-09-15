//! What the ledger reports: the verdict vocabulary, the assembly of the three
//! channels into one report, and the committed encoding of that report.
//!
//! The channels depend on this module for [`Outcome`] and never on one another,
//! so a change to one channel's rules lands in that channel's file.

use crate::catalog::INVARIANTS;
use crate::harness::Harness;
use crate::{document, repository};
use serde::Serialize;
use std::fmt::Write as _;
use std::path::Path;

/// One channel's verdict for one invariant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Outcome {
    /// `document`, `repository` or `test`.
    pub channel: &'static str,
    /// What was checked: the clause, the fact, or the binding.
    pub subject: String,
    pub ok: bool,
    /// Why it passed, or what is missing when it did not.
    pub detail: String,
}

impl Outcome {
    pub(crate) fn pass(channel: &'static str, subject: String, detail: &str) -> Self {
        Self {
            channel,
            subject,
            ok: true,
            detail: detail.into(),
        }
    }

    pub(crate) fn fail(channel: &'static str, subject: String, detail: &str) -> Self {
        Self {
            channel,
            subject,
            ok: false,
            detail: detail.into(),
        }
    }
}

/// Everything checked for one invariant, in catalog order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Coverage {
    pub id: &'static str,
    pub requirement: &'static str,
    pub statement: &'static str,
    pub owners: Vec<u64>,
    pub outcomes: Vec<Outcome>,
}

impl Coverage {
    /// Whether every channel passed and at least one channel was checked.
    pub fn is_proven(&self) -> bool {
        !self.outcomes.is_empty() && self.outcomes.iter().all(|outcome| outcome.ok)
    }
}

/// The whole report, as published against the issue.
#[derive(Debug, Clone, Serialize)]
pub struct Report {
    pub invariants: Vec<Coverage>,
    /// Total channels checked across every invariant.
    pub checked: usize,
    /// One line per failed channel; empty when the constitution conforms.
    pub failures: Vec<String>,
}

/// Where the report is committed, relative to the workspace root. The example
/// writes it and the conformance suite diffs it, so the record on the issue
/// cannot diverge from the tree that produced it.
pub const REPORT_PATH: &str = "docs/contracts/constitution-report.json";

/// Evaluate every invariant against the embedded constitution and the tree.
///
/// The harness is discovered once, from cargo and the workflow's own patterns.
/// A harness that cannot be discovered fails every `test` channel with the
/// reason rather than reporting a channel that was never checked: an appraiser
/// with nothing to compare against has not passed the apprisal.
pub fn evaluate(root: &Path) -> Report {
    let harness = Harness::discover(root);
    let mut invariants = Vec::with_capacity(INVARIANTS.len());
    let mut failures = Vec::new();
    let mut checked = 0;
    for invariant in INVARIANTS {
        let mut outcomes = document::outcomes(invariant, document::CONSTITUTION);
        outcomes.extend(repository::outcomes(invariant, root));
        match &harness {
            Ok(harness) => outcomes.extend(harness.outcomes(invariant)),
            Err(error) => outcomes.extend(
                invariant
                    .tests
                    .iter()
                    .map(|binding| Outcome::fail("test", binding.to_string(), error)),
            ),
        }
        checked += outcomes.len();
        for outcome in outcomes.iter().filter(|outcome| !outcome.ok) {
            let mut line = String::new();
            let _ = write!(
                line,
                "{} {} [{}]: {}",
                invariant.id, outcome.subject, outcome.channel, outcome.detail
            );
            failures.push(line);
        }
        invariants.push(Coverage {
            id: invariant.id,
            requirement: invariant.requirement,
            statement: invariant.statement,
            owners: invariant.owners.to_vec(),
            outcomes,
        });
    }
    Report {
        invariants,
        checked,
        failures,
    }
}

/// The report exactly as the committed artifact spells it, so the example that
/// regenerates it and the test that diffs it agree byte for byte.
pub fn report_to_json(report: &Report) -> String {
    let mut json = serde_json::to_string_pretty(report).expect("report serialization");
    json.push('\n');
    json
}

/// Evaluate the tree and render the report in the committed encoding.
pub fn report_json(root: &Path) -> String {
    report_to_json(&evaluate(root))
}
