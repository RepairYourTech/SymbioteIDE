//! The reading the contract cases share.
//!
//! Every crate that compares a `docs/contracts/` document with the declaration that enforces it
//! needs the same three readings: the slice a document writes between two anchors, the figure a
//! statement writes, and that figure where it carries a unit. Each case used to carry its own copy —
//! eighteen of them, four dialects of the same three functions — so the reading is one owner now.
//! What stays with the case: which anchors to read between, and what the slice means.
//!
//! This is a dev-dependency only. No crate depends on it outside a test target, and it adds nothing
//! to a product or published surface.

use std::fmt::Debug;

/// The slice `text` writes from `from` to the next `to` after it.
pub fn region<'a>(text: &'a str, from: &str, to: &str) -> &'a str {
    let start = text
        .find(from)
        .unwrap_or_else(|| panic!("the text must write {from:?}"))
        + from.len();
    let rest = &text[start..];
    let end = rest
        .find(to)
        .unwrap_or_else(|| panic!("the text must write {to:?} after {from:?}"));
    &rest[..end]
}

/// The figure the statement writes: the digits it carries, with `,` and `_` separators read as
/// separators and the words `one` to `ten` read where a document spells a count out.
///
/// A statement, or the crate's own slice of source, may write the figure with punctuation around it
/// (`"a 512 bound"`, `"const MAX: usize = 1_048_576;"`) or as a spelled word, so the reading takes
/// the digits and scales nothing.
pub fn figure<T: TryFrom<u64>>(text: &str) -> T
where
    T::Error: Debug,
{
    convert::<T>(number(text), text)
}

/// The byte figure the statement writes: `16 KiB`, `1 MiB`, or a bare count of bytes.
pub fn bytes<T: TryFrom<u64>>(text: &str) -> T
where
    T::Error: Debug,
{
    let scale = if text.contains("KiB") {
        1024
    } else if text.contains("MiB") {
        1024 * 1024
    } else {
        1
    };
    convert::<T>(number(text) * scale, text)
}

fn number(text: &str) -> u64 {
    const WORDS: [(&str, u64); 10] = [
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
    ];
    let trimmed = text.trim();
    if let Some((_, value)) = WORDS.iter().find(|(word, _)| trimmed.starts_with(word)) {
        return *value;
    }
    let digits: String = trimmed.chars().filter(char::is_ascii_digit).collect();
    digits
        .parse()
        .unwrap_or_else(|_| panic!("a figure, not {text:?}"))
}

fn convert<T: TryFrom<u64>>(value: u64, text: &str) -> T
where
    T::Error: Debug,
{
    T::try_from(value)
        .unwrap_or_else(|error| panic!("{text:?} writes {value}, which does not fit: {error:?}"))
}
