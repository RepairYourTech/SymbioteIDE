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
