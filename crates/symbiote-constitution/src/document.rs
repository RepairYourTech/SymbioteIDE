//! The document channel: what the constitution itself must say.
//!
//! Required clauses must appear verbatim and forbidden authorizations must be
//! absent. The document is embedded, so a check reads the text compiled into
//! this binary and never a different file a later run might find.

use crate::catalog::Invariant;
use crate::report::Outcome;

/// The constitution, embedded so that a check can never read a different file
/// than the one this build compiled.
pub const CONSTITUTION: &str = include_str!("../../../docs/architecture/product-constitution.md");

/// Clause checks against a constitution's text, so a test can also run them
/// against a deliberately weakened copy rather than only against the real file.
pub fn outcomes(invariant: &Invariant, document: &str) -> Vec<Outcome> {
    let mut outcomes = Vec::new();
    for clause in invariant.document {
        let subject = format!("requires: {clause}");
        outcomes.push(if document.contains(clause) {
            Outcome::pass("document", subject, "clause present")
        } else {
            Outcome::fail(
                "document",
                subject,
                "clause is missing from the constitution",
            )
        });
    }
    for clause in invariant.forbidden {
        let subject = format!("forbids: {clause}");
        let present = document.contains(clause);
        outcomes.push(if present {
            Outcome::fail("document", subject, "a forbidden authorization is present")
        } else {
            Outcome::pass("document", subject, "authorization is absent")
        });
    }
    outcomes
}

/// The issue this record accounts for. A coverage row that points back at it
/// accounts for nothing: it names the claim being accounted for, not a check
/// that runs or an owner of the integration.
pub const RECORDED_ISSUE: u64 = 170;

/// The coverage map's header, which doubles as the marker that the map exists.
const COVERAGE_HEADER: &str = "| Named coverage item | Machine check or canonical owner |";

/// One row of the constitution's coverage map: a named coverage item and what
/// accounts for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CoverageRow<'a> {
    /// The named coverage item.
    pub item: &'a str,
    /// The machine check that runs it, or the canonical issue that owns it.
    pub accounting: &'a str,
}

/// The rows of a constitution's coverage map, or `None` when it has none.
///
/// A missing map is not a pass: the map is this record's account of its own
/// coverage, so its absence is refused rather than read as nothing to check.
pub fn coverage_rows(document: &str) -> Option<Vec<CoverageRow<'_>>> {
    let mut lines = document.lines();
    lines.find(|line| line.trim() == COVERAGE_HEADER)?;
    let mut rows = Vec::new();
    for line in lines {
        let line = line.trim();
        if !line.starts_with('|') {
            break;
        }
        let cells: Vec<&str> = line.trim_matches('|').split('|').map(str::trim).collect();
        if cells.len() < 2 || cells.iter().all(|cell| cell.starts_with("---")) {
            continue;
        }
        rows.push(CoverageRow {
            item: cells[0],
            accounting: cells[1],
        });
    }
    Some(rows)
}

/// The backticked spans of a line — the checks a coverage row cites — or `None`
/// when its backticks do not pair.
pub fn check_names(accounting: &str) -> Option<Vec<&str>> {
    let mut names = Vec::new();
    let mut rest = accounting;
    while let Some(opening) = rest.find('`') {
        let after_opening = &rest[opening + 1..];
        let closing = after_opening.find('`')?;
        names.push(&after_opening[..closing]);
        rest = &after_opening[closing + 1..];
    }
    Some(names)
}

/// Whether a coverage row accounts for itself.
///
/// `is_known_check` is the repository's own answer to whether a named check is
/// one this tree provides, so a row cannot cite a job or a test that does not
/// exist here. Naming is resolved rather than merely shaped: every check a row
/// names must resolve, because citing one real check beside an invented one is
/// not accounting for the row. An owner is an issue reference other than
/// [`RECORDED_ISSUE`] — naming the issue this record accounts for is the claim
/// itself, not an owner of it — so a row must name a resolving check or such an
/// owner.
///
/// What this does not verify: an issue number cannot be resolved offline, so the
/// owner half proves only that the row names some issue other than the one being
/// accounted for. Whether that issue actually owns the work is a human judgement
/// recorded in the map, not something this ledger checks.
pub fn accounts_for(accounting: &str, is_known_check: impl Fn(&str) -> bool) -> bool {
    let Some(names) = check_names(accounting) else {
        return false;
    };
    names.iter().all(|name| is_known_check(name))
        && (!names.is_empty() || owners(accounting).any(|issue| issue != RECORDED_ISSUE))
}

/// Every `#N` issue reference in a row.
fn owners(accounting: &str) -> impl Iterator<Item = u64> + '_ {
    accounting.split('#').skip(1).filter_map(|rest| {
        let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        digits.parse::<u64>().ok()
    })
}
