//! What `docs/contracts/client-sdk.md` states about this crate, held to the crate.
//!
//! Both halves are read from the declarations rather than restated: the variants `ClientError`
//! writes, the type names this module declares or imports, and the names the document's
//! classification bullet writes inside backticks. A terminal outcome the document invents fails
//! here by name, and a variant this crate adds without naming it in the document fails here too.
//!
//! What it does not read: the count the bullet used to state beside the list ("Exactly four").
//! The four outcomes are not the file's six variants — three are build-time refusals — so the
//! count is not a figure any declaration carries; the list itself is the statement, and the
//! numeral is gone.

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

/// The leading identifier of every backticked name in `text`, in order: `` `Ok(body)` `` writes
/// `Ok`, `` `ResponseBody` `` writes `ResponseBody`.
fn backticked_names(text: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut rest = text;
    while let Some(open) = rest.find('`') {
        let after = &rest[open + 1..];
        let Some(close) = after.find('`') else { break };
        rest = &after[close + 1..];
        let name: String = after[..close]
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
            .collect();
        if !name.is_empty() {
            names.push(name);
        }
    }
    names
}

/// Every capitalized identifier in `text` that is not part of a longer word.
fn capitalized_names(text: &str) -> Vec<String> {
    text.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .filter(|piece| piece.starts_with(|c: char| c.is_ascii_uppercase()) && !piece.is_empty())
        .map(str::to_string)
        .collect()
}

/// The variant names `enum ClientError` declares, read from the declaration itself: the variant
/// is the identifier a declaration line opens with, not the payload type beside it.
fn declared_error_variants() -> Vec<String> {
    let source = include_str!("../src/lib.rs");
    let body = region(source, "pub enum ClientError {", "\n}\n");
    body.lines()
        .map(str::trim_start)
        .filter(|line| {
            line.starts_with(|c: char| c.is_ascii_uppercase()) && !line.starts_with("//")
        })
        .map(|variant| {
            variant
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect()
        })
        .collect()
}

/// The type names this module carries: what it declares (`pub struct`, `pub enum`) and what it
/// imports (every `use` statement).
fn declared_type_names() -> Vec<String> {
    let source = include_str!("../src/lib.rs");
    let mut names = Vec::new();

    let mut rest = source;
    while let Some(at) = rest.find("use ") {
        let after = &rest[at + "use ".len()..];
        let Some(end) = after.find(';') else { break };
        rest = &after[end..];
        names.extend(capitalized_names(&after[..end]));
    }

    for keyword in ["pub struct ", "pub enum ", "pub fn "] {
        let mut rest = source;
        while let Some(at) = rest.find(keyword) {
            let after = &rest[at + keyword.len()..];
            rest = &after[1..];
            let name: String = after
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect();
            if !name.is_empty() {
                names.push(name);
            }
        }
    }

    names
}

/// Every outcome this crate classifies is named by the contract, and every outcome the contract's
/// classification bullet names is one this crate carries. The bullet's names are read from the
/// sentence; the set they must belong to is read from the declarations, so neither a variant
/// added to the enum nor an outcome invented in prose can pass silently.
#[test]
fn the_contract_names_every_outcome_this_crate_classifies_and_none_it_does_not() {
    let contract = include_str!("../../../docs/contracts/client-sdk.md");

    let variants = declared_error_variants();
    let types = declared_type_names();
    assert!(
        variants.len() > 1 && types.len() > 3,
        "the crate must declare its variants and its types, and this read found {variants:?} and {types:?}"
    );

    for variant in &variants {
        assert!(
            contract.contains(variant.as_str()),
            "this crate classifies {variant}, and the contract does not name it"
        );
    }

    let bullet = region(
        contract,
        "**Response classification.**",
        "A disconnect is a transport fact",
    );
    let named = backticked_names(bullet);
    assert!(
        named.len() > 1,
        "the classification bullet must name its outcomes, and this read found {named:?}"
    );
    for name in &named {
        assert!(
            name == "Ok" || variants.contains(name) || types.contains(name),
            "the contract names a terminal outcome {name}, and this crate carries no such variant or type"
        );
    }
}
