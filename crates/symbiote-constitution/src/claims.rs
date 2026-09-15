//! The conformance contract's own claims about the report, read as data (#170).
//!
//! A document cannot be trusted to stay true of an artifact it does not read.
//! Three times in this record's history the contract doc asserted something the
//! committed report did not show — a coverage row claiming evidence that did not
//! exist, a test presented as a channel that was never bound, and a sentence
//! describing one earlier pass's diff — and each time every check stayed green.
//! So the names the doc presents as report channels are taken from the doc
//! itself and resolved against the artifact, rather than restated here where
//! they could drift with it.
//!
//! The limit, stated once: a claim is found by the phrasing this document uses
//! to make it, so a claim written another way is not seen by this check, and a
//! document that makes no such claim at all fails rather than passing vacuously.
//! The point is not a circular guarantee; it is that the sentences which are
//! written this way cannot go false while every check stays green.

/// Where the conformance contract is written down.
pub const CONTRACT_PATH: &str = "docs/contracts/constitution.md";

/// The phrase with which the contract doc presents a name as a report channel.
const CLAIM_MARKER: &str = "in the report";

/// A test the contract doc presents as a channel of the report, with the line
/// that claims it, so a failure can quote the claim and not only the name.
#[derive(Debug, PartialEq, Eq)]
pub struct Claim<'a> {
    /// The name the document presents as a report channel.
    pub name: &'a str,
    /// The line making the claim.
    pub line: &'a str,
}

/// The test names the contract doc presents as channels of the report.
///
/// Prose here is one paragraph per line, so a line is the sentence a reader
/// would point at when the claim turns out to be untrue.
pub fn claimed_channels(document: &str) -> Vec<Claim<'_>> {
    let mut claims = Vec::new();
    for line in document.lines() {
        if !line.contains(CLAIM_MARKER) {
            continue;
        }
        for name in crate::document::check_names(line).unwrap_or_default() {
            if is_test_name(name) {
                claims.push(Claim { name, line });
            }
        }
    }
    claims
}

/// Whether a backticked token names a test rather than an invariant id or a
/// path: a bare identifier, so `CN-22` and `docs/contracts/…` are not candidates.
fn is_test_name(name: &str) -> bool {
    !name.is_empty()
        && name.chars().any(|c| c.is_ascii_alphabetic())
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}
