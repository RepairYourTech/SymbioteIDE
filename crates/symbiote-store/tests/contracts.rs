//! What this repository's `storage.md` states about this crate, held to the crate.

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

/// The figure a statement writes, as digits or as a spelled word, commas and surrounding
/// punctuation trimmed off.
fn figure(text: &str) -> u64 {
    let trimmed = text.trim();
    for (word, value) in [
        ("one", 1),
        ("two", 2),
        ("three", 3),
        ("four", 4),
        ("five", 5),
        ("six", 6),
        ("seven", 7),
        ("eight", 8),
        ("nine", 9),
        ("ten", 10),
    ] {
        if trimmed.starts_with(word) {
            return value;
        }
    }
    let number: String = trimmed
        .trim_matches(|c: char| !c.is_ascii_digit() && c != ',')
        .replace(',', "");
    number
        .parse()
        .unwrap_or_else(|_| panic!("a figure, not {text:?}"))
}

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

    let stated = figure(region(
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
