//! The contract document's own citations (#173): every name `docs/contracts/`
//! `architecture.md` writes in a code span is held to something this crate has,
//! so a rename fails a case here rather than leaving the document citing a name
//! nobody can run.
//!
//! This lived in `spike_contracts.rs`, which is about #38's proof contract and
//! has nothing to do with reading a document. It is its own target so a
//! maintainer looking for the rule finds it, and so neither file grows the
//! other's concern.

use std::path::Path;
use symbiote_architecture::workspace_root;

/// Whether the suite defines `name` as a case it runs: a `fn name(` whose
/// attribute block carries `#[test]`, rather than any function of that name.
/// A helper satisfies a name; it is not something a failure can be read from.
fn defines_a_case(suite: &str, name: &str) -> bool {
    let lines: Vec<&str> = suite.lines().collect();
    let needle = format!("fn {name}(");
    lines.iter().enumerate().any(|(at, line)| {
        if !line.trim_start().starts_with(&needle) {
            return false;
        }
        let mut above = at;
        while above > 0 {
            let previous = lines[above - 1].trim();
            if previous == "#[test]" {
                return true;
            }
            if previous.starts_with("#[") || previous.is_empty() {
                above -= 1;
                continue;
            }
            return false;
        }
        false
    })
}

/// A lower-case name carrying an underscore: what this document writes in a code
/// span when it names something, as opposed to a path, a command, a type or a
/// sentence. Digits are part of a name, so a citation cannot escape by carrying
/// one. This is the whole of what the check below reads, and the sentence it
/// serves says the same thing in the same words.
fn is_identifier_name(span: &str) -> bool {
    span.contains('_')
        && span
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}

/// Whether the crate's library sources carry `name` as a name: an occurrence
/// that is not part of a longer one, so the file that carries `blocking_issue`
/// does not carry `blocking_iss`. A `contains` test here would let the document
/// cite a name nothing defines, as long as it is a fragment of something that
/// exists. What it holds is that the crate's `src/` names it somewhere — a
/// definition, a string, a comment — not that it defines it: that reading is the
/// reader's, and `tests/`, `examples/` and `benches/` are not read, so a name
/// that lives only there is refused rather than excused.
fn carries_name(source: &str, name: &str) -> bool {
    let bytes = source.as_bytes();
    let inside =
        |byte: Option<u8>| byte.is_none_or(|byte| !(byte.is_ascii_alphanumeric() || byte == b'_'));
    source.match_indices(name).any(|(at, _)| {
        inside(at.checked_sub(1).map(|before| bytes[before]))
            && inside(bytes.get(at + name.len()).copied())
    })
}

/// The names the document writes in code spans that are not cases of this
/// suite: the crate's own fields, methods and functions, under which no case can
/// be run. A name here must be one the crate's `src/` carries — as a name, not as
/// a fragment — and must still be cited by the document, so the list cannot
/// excuse a name that exists nowhere and cannot outlive the sentence that used
/// it.
const NOT_CASES: [&str; 8] = [
    "blocking_issue",
    "contract_sha256",
    "proof_contract",
    "proof_section",
    "proposed_dependencies",
    "record_sha256",
    "require_ready",
    "workspace_root",
];

/// Every `.rs` file under a directory, so a name may live in a submodule.
fn rust_sources(directory: &Path) -> String {
    let mut text = String::new();
    let mut directories = vec![directory.to_path_buf()];
    while let Some(next) = directories.pop() {
        let entries = std::fs::read_dir(&next).expect("a source directory");
        for entry in entries.filter_map(|entry| entry.ok()) {
            let path = entry.path();
            if path.is_dir() {
                directories.push(path);
            } else if path.extension().is_some_and(|kind| kind == "rs") {
                text.push_str(&std::fs::read_to_string(&path).unwrap_or_default());
            }
        }
    }
    text
}

/// Names the contract document writes in code spans are names of this crate: a
/// citation is a claim, so every one of them is held either to a case the suite
/// actually runs or to the exception list above, which is held back to the crate
/// and to the document. A rename therefore fails a case here, a name nothing
/// defines cannot be written down, and a helper cannot stand in for a case. The
/// two names the constitution ledger's catalog also binds are held twice,
/// deliberately: that binding belongs to the catalog's evidence, this one to the
/// document's own citations. What it does not read is a name the document writes
/// outside a code span, or whether the case it finds is the one the citing
/// sentence needed: it holds the name, not the claim.
#[test]
fn the_contract_document_names_only_cases_this_crate_holds() {
    let document = std::fs::read_to_string(workspace_root().join("docs/contracts/architecture.md"))
        .expect("the contract document");
    assert_eq!(
        document.matches('`').count() % 2,
        0,
        "the document's code spans are balanced, so a citation can be read out of it"
    );
    let crate_root = workspace_root().join("crates/symbiote-architecture");
    let suite = rust_sources(&crate_root.join("tests"));
    let source = rust_sources(&crate_root.join("src"));
    let mut cases = 0;
    for (index, span) in document.split('`').enumerate() {
        if index % 2 == 0 || !is_identifier_name(span) {
            continue;
        }
        if defines_a_case(&suite, span) {
            cases += 1;
            continue;
        }
        assert!(
            NOT_CASES.contains(&span),
            "the contract document names {span} in a code span, and it is neither a case this crate runs nor one of the names listed as not cases"
        );
        assert!(
            carries_name(&source, span),
            "{span} is listed as not a case, and the crate's own src/ carries no such name"
        );
    }
    for name in NOT_CASES {
        assert!(
            document
                .split('`')
                .skip(1)
                .step_by(2)
                .any(|span| span == name),
            "{name} is listed as not a case, and the contract document no longer cites it"
        );
    }
    assert!(
        cases >= 5,
        "the document cites the cases its claims rest on: {cases}"
    );
}
