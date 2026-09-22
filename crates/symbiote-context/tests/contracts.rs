//! What this repository's `context-credential-resolution.md` states about this crate, held to it.
use symbiote_context::{MAX_ITEM_BYTES, MAX_LIST_ITEMS, MAX_LIST_TOTAL_BYTES};

/// The slice `text` writes between `from` and the next `to` after it.
fn region<'a>(text: &'a str, from: &str, to: &str) -> &'a str {
    let start = text
        .find(from)
        .unwrap_or_else(|| panic!("the text must state {from:?}"))
        + from.len();
    let rest = &text[start..];
    let end = rest
        .find(to)
        .unwrap_or_else(|| panic!("the text must state {to:?}"));
    &rest[..end]
}

/// The figure a statement writes, commas and any surrounding punctuation trimmed off.
fn figure(text: &str) -> usize {
    let number: String = text
        .trim_matches(|c: char| !c.is_ascii_digit() && c != ',')
        .replace(',', "");
    number
        .parse()
        .unwrap_or_else(|_| panic!("a figure, not {text:?}"))
}

/// The byte figure a statement writes: `8 KiB`.
fn bytes(text: &str) -> usize {
    let (number, unit) = text.trim().split_once(' ').expect("a figure and a unit");
    let number = figure(number);
    match unit {
        "KiB" => number * 1024,
        "MiB" => number * 1024 * 1024,
        _ => number,
    }
}

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

    let stated_item = bytes(region(contract, "bounded per item (", " with a"));
    assert_eq!(
        stated_item, MAX_ITEM_BYTES,
        "the contract states a {stated_item}-byte item bound, and this crate bounds {MAX_ITEM_BYTES}"
    );

    let stated_items = figure(region(contract, "per list (", " items"));
    assert_eq!(
        stated_items, MAX_LIST_ITEMS,
        "the contract states a {stated_items}-item list bound, and this crate bounds {MAX_LIST_ITEMS}"
    );

    let stated_total = bytes(region(contract, "items, ", " total)"));
    assert_eq!(
        stated_total, MAX_LIST_TOTAL_BYTES,
        "the contract states a {stated_total}-byte list budget, and this crate bounds {MAX_LIST_TOTAL_BYTES}"
    );
}
