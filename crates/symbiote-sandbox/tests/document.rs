//! The bounds `docs/security/linux-sandbox.md` states are the ones this crate enforces.
//!
//! The document is a security boundary, and a bound stated in a security document that the code
//! has moved past is the worst kind of prose to have drifted: a reader trusts it. Each figure below
//! is read from the sentence it is written in — the traversal entry count and depth from the
//! preflight paragraph, the retained diagnostic lines and their byte length from the setup-failure
//! paragraph — and held against the constant that enforces it, so a document that moves without
//! this crate failing is refused here by name rather than discovered in a review.
//!
//! What it does not read: the prose around the figures — what a bound refuses and why — which the
//! behavioural cases in `process.rs` drive, and the alias proof under `docs/proofs/`, which is
//! captured evidence from one recorded run rather than a bound this crate publishes.
#![cfg(target_os = "linux")]

use symbiote_contract_read::{figure, region};
use symbiote_sandbox::{
    MAX_SETUP_DIAGNOSTIC_BYTES, MAX_SETUP_DIAGNOSTIC_LINES, MAX_TREE_DEPTH, MAX_TREE_ENTRIES,
};

#[test]
fn the_document_states_the_traversal_and_diagnostic_bounds_this_crate_enforces() {
    let document = include_str!("../../../docs/security/linux-sandbox.md");

    // The preflight sentence carries both traversal figures: "It limits traversal to 100,000
    // entries and 64 levels." Each is read from its own narrow slice, because the shared reading
    // takes every digit in the region it is given rather than the first figure in it.
    for (label, stated, enforced) in [
        (
            "entries",
            figure::<usize>(region(document, "It limits traversal to ", " entries")),
            MAX_TREE_ENTRIES,
        ),
        (
            "levels",
            figure::<usize>(region(document, " entries and", " levels")),
            MAX_TREE_DEPTH,
        ),
    ] {
        assert_eq!(
            stated, enforced,
            "the document states a {label} bound of {stated}, and this crate refuses at {enforced}: \
             the preflight walks a worktree before it launches anything under it"
        );
    }

    // The setup-failure sentence spells the line count as a word and the byte length as a figure.
    for (label, stated, enforced) in [
        (
            "diagnostic lines",
            figure::<usize>(region(
                document,
                "Setup failures retain at most ",
                " diagnostic lines",
            )),
            MAX_SETUP_DIAGNOSTIC_LINES,
        ),
        (
            "bytes each",
            figure::<usize>(region(
                document,
                "diagnostic lines of ",
                " UTF-8 bytes each",
            )),
            MAX_SETUP_DIAGNOSTIC_BYTES,
        ),
    ] {
        assert_eq!(
            stated, enforced,
            "the document retains {stated} {label} on a setup failure, and this crate retains \
             {enforced}: a refusal a caller cannot read is not a refusal it can act on"
        );
    }
}
