//! What this repository's `context-credential-resolution.md` states about this crate, held to it.
use symbiote_context::{MAX_ITEM_BYTES, MAX_LIST_ITEMS, MAX_LIST_TOTAL_BYTES};
use symbiote_contract_read::{bytes, figure, region};

/// The three bounds `context-credential-resolution.md` states are the ones this crate enforces: the
/// per-item byte bound, the list item count and the list's total byte budget. Each figure is read
/// from the sentence it is written in, so a document that states a bound this crate has moved past
/// fails here by name rather than in prose nobody reads.
///
/// What it does not read: the prose around the figures — the visible truncation marker, the typed
/// `Overbound` refusal, the credential-broker boundary — which the crate's own cases drive, and the
/// Host-side resolution path the document also describes.
#[test]
fn the_contract_states_the_bounds_this_crate_enforces() {
    let contract = include_str!("../../../docs/contracts/context-credential-resolution.md");

    let stated_item: usize = bytes(region(contract, "bounded per item (", " with a"));
    assert_eq!(
        stated_item, MAX_ITEM_BYTES,
        "the contract states a {stated_item}-byte item bound, and this crate bounds {MAX_ITEM_BYTES}"
    );

    let stated_items: usize = figure(region(contract, "per list (", " items"));
    assert_eq!(
        stated_items, MAX_LIST_ITEMS,
        "the contract states a {stated_items}-item list bound, and this crate bounds {MAX_LIST_ITEMS}"
    );

    let stated_total: usize = bytes(region(contract, "items, ", " total)"));
    assert_eq!(
        stated_total, MAX_LIST_TOTAL_BYTES,
        "the contract states a {stated_total}-byte list budget, and this crate bounds {MAX_LIST_TOTAL_BYTES}"
    );
}
