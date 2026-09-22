//! What this repository's `host-inventory.md` states about this crate, held to the crate.

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

/// The figure a statement writes, as digits or as a spelled word.
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
        .trim_matches(|c: char| !c.is_ascii_digit() && c != '_')
        .replace('_', "");
    number
        .parse()
        .unwrap_or_else(|_| panic!("a figure, not {text:?}"))
}

/// The sample lifetime `host-inventory.md` states is the one this crate stamps onto a pulse, in
/// milliseconds. The rule this holds is that **every** `checked_add(<figure>)` in the crate's source
/// is that lifetime — the observation's own and the refresh's — because a pulse that outlived the
/// contract's figure would be a cached fact nobody re-observes. A document that states a lifetime
/// this crate has moved past, or a second lifetime added beside it, fails here by name.
///
/// What it does not read: the prose around the figure — the one-second read cache, the ceiling on
/// usable TTL, telemetry being off by default — which the crate's own cases drive.
#[test]
fn the_contract_states_the_sample_lifetime_this_crate_stamps() {
    let contract = include_str!("../../../docs/contracts/host-inventory.md");
    let source = include_str!("../src/lib.rs");

    let stated = figure(region(contract, "Samples expire after ", "."));
    let mut stamped = 0;
    for (index, _) in source.match_indices("checked_add(") {
        let rest = &source[index + "checked_add(".len()..];
        let value: String = rest
            .chars()
            .take_while(|c| c.is_ascii_digit() || *c == '_')
            .collect();
        if value.is_empty() {
            continue;
        }
        stamped += 1;
        assert_eq!(
            figure(&value),
            stated * 1_000,
            "the contract states a {stated}-second sample lifetime, and this crate stamps {value} milliseconds"
        );
    }
    assert!(
        stamped > 1,
        "the crate must stamp the contract's lifetime on the observation and its refresh, and this read found {stamped}"
    );
}
