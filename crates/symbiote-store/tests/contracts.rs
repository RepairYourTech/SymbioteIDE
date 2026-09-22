//! What this repository's `storage.md` states about this crate, held to the crate.
use symbiote_contract_read::{figure, region};

/// The busy timeout `storage.md` states is the one this crate's writer sets. The figure is read from
/// the sentence it is written in and the pragma's value from the call that sets it, so a document
/// that states a timeout this crate has moved past fails here by name rather than in prose nobody
/// reads.
///
/// What it does not read: the prose around it — `BEGIN IMMEDIATE`, WAL, `synchronous=FULL`, the
/// revision-conflict and rollback behaviour — which the crate's own cases drive against real
/// databases.
#[test]
fn the_contract_states_the_writer_timeout_this_crate_sets() {
    let contract = include_str!("../../../docs/contracts/storage.md");
    let source = include_str!("../src/lib.rs");

    let stated: u64 = figure(region(
        contract,
        "Writer transactions use SQLite `BEGIN IMMEDIATE`, foreign-key constraints, a ",
        " busy timeout",
    ));
    let enforced = figure(region(source, "busy_timeout(Duration::from_secs(", "))"));
    assert_eq!(
        stated, enforced,
        "the contract states a {stated}-second busy timeout, and this crate sets {enforced}"
    );
}
