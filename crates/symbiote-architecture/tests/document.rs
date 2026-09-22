//! What this repository's documents claim about its contract (#173), held to the
//! crate that owns the facts: every name `docs/contracts/architecture.md` writes in a
//! code span is something this crate has, the surface that document writes for the join
//! is the surface `checks/mod.rs` defines, and every ceiling `docs/proofs/linux-shell.md`
//! states for a measurement is the maximum the committed contract predeclares. A rename,
//! or a threshold the contract moves, fails a case here rather than leaving a document
//! citing a name nobody can run or a bar the machine no longer enforces — the bar's
//! content is written in three prose copies, and the copy that states its numbers is
//! what this file compares with the contract.
//!
//! This lived in `spike_contracts.rs`, which is about #38's proof contract and
//! has nothing to do with reading a document. It is its own target so a
//! maintainer looking for the rules finds them, and so neither file grows the
//! other's concern.

use std::path::Path;
use symbiote_architecture::spike::{CONTRACTS_PATH, Contracts, Results, SpikeContract};
use symbiote_architecture::workspace_root;

/// The proof record of the shell spike (#38): the run's figures and the bar it states
/// beside them, which is the copy of the bar this file holds to the contract.
const PROOF_RECORD: &str = "docs/proofs/linux-shell.md";

/// One statement a paragraph of the record makes about the bar: a measurement it
/// names in a code span, or a ceiling it writes as `of a 3.0 ceiling`.
#[derive(Debug, PartialEq)]
enum BarStatement {
    Name(String),
    Ceiling(f64),
}

/// The ceiling phrase at the start of `text` — `of a 3.0 ceiling`, `of 0 ceilings` —
/// with the number it states and how far the phrase reaches. `None` for the word `of`
/// in any other sentence, so the record's prose about a method is read as prose and a
/// figure of its own is not mistaken for a threshold.
fn ceiling_phrase(text: &str) -> Option<(f64, usize)> {
    let rest = text
        .strip_prefix("of a ")
        .or_else(|| text.strip_prefix("of "))?;
    let digits: String = rest
        .chars()
        .take_while(|character| character.is_ascii_digit() || *character == '.')
        .collect();
    if digits.is_empty() || digits == "." {
        return None;
    }
    let number: f64 = digits.parse().ok()?;
    let tail = &rest[digits.len()..];
    let after = tail
        .strip_prefix(" ceilings")
        .or_else(|| tail.strip_prefix(" ceiling"))?;
    if after.starts_with(|character: char| character.is_ascii_alphanumeric() || character == '_') {
        return None;
    }
    Some((number, text.len() - after.len()))
}

/// What a paragraph states, in the order it writes it: every measurement name it
/// writes in a code span, and every ceiling phrase it writes. A code span that is not
/// a lower-case identifier — a path, a command, a type, a sentence — is not a name
/// here, and only a ceiling phrase is a threshold, so neither the record's prose nor
/// its figures are read as statements of the bar.
fn statements(paragraph: &str) -> Vec<BarStatement> {
    let mut found = Vec::new();
    let mut rest = paragraph;
    while !rest.is_empty() {
        if let Some(after) = rest.strip_prefix('`') {
            if let Some(end) = after.find('`') {
                let span = &after[..end];
                if is_identifier_name(span) {
                    found.push(BarStatement::Name(span.to_string()));
                }
                rest = &after[end + 1..];
                continue;
            }
        }
        if let Some((number, reached)) = ceiling_phrase(rest) {
            found.push(BarStatement::Ceiling(number));
            rest = &rest[reached..];
            continue;
        }
        let step = rest.chars().next().map_or(1, char::len_utf8);
        rest = &rest[step..];
    }
    found
}

/// The block of lines a paragraph is, so a statement is read beside the names its own
/// paragraph writes rather than beside a neighbouring bullet's.
fn paragraphs(record: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut current: Vec<&str> = Vec::new();
    for line in record.lines() {
        if line.trim().is_empty() {
            if !current.is_empty() {
                found.push(current.join("\n"));
                current.clear();
            }
        } else {
            current.push(line);
        }
    }
    if !current.is_empty() {
        found.push(current.join("\n"));
    }
    found
}

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
const NOT_CASES: [&str; 10] = [
    "blocking_issue",
    "contract_sha256",
    "deployment_refs",
    "graph_refs",
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

/// The entry points `checks/mod.rs` defines, from the module's own source: a `pub fn`
/// written at the top level, which is how the join exposes one. The subjects it
/// composes are private modules, so nothing else in that file is part of its surface.
fn entry_points_of(source: &str) -> Vec<String> {
    source
        .lines()
        .filter_map(|line| line.strip_prefix("pub fn "))
        .map(|rest| rest.split(['(', '<']).next().unwrap_or_default().to_owned())
        .filter(|name| !name.is_empty())
        .collect()
}

/// The subjects the join composes, from the same source: a `mod x;` declaration.
fn subjects_of(source: &str) -> Vec<String> {
    source
        .lines()
        .filter_map(|line| line.strip_prefix("mod "))
        .filter_map(|rest| rest.strip_suffix(';'))
        .map(str::to_owned)
        .collect()
}

/// The names a module's own doc writes between `open`/`close` around a code span —
/// `[\u{60}problems\u{60}]` for an entry point, `(\u{60}records\u{60})` for a subject. The two
/// forms are how that doc marks the two lists, so each is read as the list it is.
fn doc_names(doc: &str, open: char, close: char) -> Vec<String> {
    let mut names = Vec::new();
    let mut rest = doc;
    while let Some(at) = rest.find(open) {
        let after = &rest[at + open.len_utf8()..];
        if let Some(span) = after.strip_prefix('`') {
            if let Some(end) = span.find('`') {
                if let Some(tail) = span[end + 1..].strip_prefix(close) {
                    names.push(span[..end].to_owned());
                    rest = tail;
                    continue;
                }
            }
        }
        rest = after;
    }
    names
}

/// A lower-case bare name: how a row writes an entry point, as opposed to the path of
/// the module the row is about or a type it names.
fn is_bare_name(span: &str) -> bool {
    !span.is_empty()
        && span
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
}

/// The document's account of the join is the module's own surface: every entry point
/// `checks/mod.rs` defines is named in the row this document writes for that file and in
/// the module's own doc, every subject it composes is named in the module's own doc and
/// has a row of its own here, and neither copy names an entry point or a subject the
/// module does not define. The two lists are derived from the source, so a third entry
/// point or a seventh subject fails this case by name rather than leaving a list a reader
/// would take for the whole of the surface — which is what the numeral both copies used to
/// carry in front of those lists could not do, and why neither states one now.
///
/// What it does not read: a signature, a body, whether an entry point is public for
/// another reason, or which entry point asks which subject — the row's own words say that,
/// and this case holds the names it needs to say it about.
#[test]
fn the_document_names_the_surface_the_join_defines() {
    let root = workspace_root();
    let module = "crates/symbiote-architecture/src/checks/mod.rs";
    let source = std::fs::read_to_string(root.join(module)).expect("the join module");
    let entry_points = entry_points_of(&source);
    let subjects = subjects_of(&source);
    assert!(
        entry_points.len() >= 2 && subjects.len() >= 2,
        "the join exposes entry points and composes subjects: {entry_points:?}, {subjects:?}"
    );

    let module_doc: String = source
        .lines()
        .filter(|line| line.trim_start().starts_with("//!"))
        .collect::<Vec<&str>>()
        .join("\n");
    let document = std::fs::read_to_string(root.join("docs/contracts/architecture.md"))
        .expect("the contract document");
    let row = document
        .lines()
        .find(|line| line.starts_with("| `checks/mod.rs` |"))
        .expect("the document writes a row for the join");
    let listed = doc_names(&module_doc, '[', ']');
    let composed = doc_names(&module_doc, '(', ')');

    for name in &entry_points {
        assert!(
            row.contains(name.as_str()),
            "the document's row for {module} names every entry point it defines, and it does not name {name}"
        );
        assert!(
            listed.contains(name),
            "the module's own doc lists every entry point it defines, and it does not list {name}"
        );
    }
    for subject in &subjects {
        assert!(
            composed.contains(subject),
            "the module's own doc names every subject it composes, and it does not name {subject}"
        );
        assert!(
            document.contains(&format!("`checks/{subject}.rs`")),
            "the document writes a row for every subject the join composes, and it has none for checks/{subject}.rs"
        );
    }

    for named in &listed {
        assert!(
            entry_points.contains(named),
            "the module's own doc lists {named} as an entry point, and the join defines no such one"
        );
    }
    for named in &composed {
        assert!(
            subjects.contains(named),
            "the module's own doc names {named} as a subject, and the join composes no such module"
        );
    }
    let stated: Vec<&str> = row
        .split_once("entry point")
        .map(|(_, tail)| tail.split('`').skip(1).step_by(2).collect())
        .unwrap_or_default();
    assert!(
        !stated.is_empty(),
        "the join's row says which of the names it writes are entry points: {row}"
    );
    for span in stated {
        if !is_bare_name(span) {
            continue;
        }
        assert!(
            entry_points.iter().any(|name| name.as_str() == span),
            "the join's row writes {span} as an entry point, and the join defines no such one"
        );
    }
}

/// The bar the proof record states is the contract's own data (#173): every ceiling it
/// writes is the predeclared maximum of the measurements named before it in the same
/// paragraph, every measurement the committed run observed has its ceiling stated where
/// a reader looks, and no ceiling is stated for a name the contract does not predeclare.
///
/// The bar's content is written in prose in three copies, and this is the one that states
/// numbers: the record's ceiling for each measurement is answered to the contract rather
/// than kept in step by hand, so a maximum that moves fails a case here instead of leaving
/// a reader with a bar the machine no longer enforces, and a measurement the record stops
/// stating is caught by the run that observed it.
///
/// The contract is the one with the committed run — the contract whose own
/// `result_artifact` reads back a dossier naming it — because that is the run the record
/// describes. A document holding two such runs fails here rather than comparing the
/// record against a contract it may not be about.
///
/// The form it reads, and what that form costs: a ceiling is stated after the
/// measurements it belongs to, so a paragraph that names something else before it — a
/// field, a path — is refused rather than read past, and a ceiling written as bare prose
/// with no measurement before it is refused too. What it does not read is a name written
/// outside a code span, and the figures themselves: those are the run's data, and the
/// artifact the run cites is what holds them.
#[test]
fn the_proof_record_states_the_contracts_own_bar() {
    let root = workspace_root();
    let record = std::fs::read_to_string(root.join(PROOF_RECORD)).expect("the proof record");
    let contracts =
        Contracts::read(&root.join(CONTRACTS_PATH)).expect("the committed contract document");
    let with_a_run: Vec<&SpikeContract> = contracts
        .contracts
        .iter()
        .filter(|contract| {
            Results::read(&root.join(&contract.result_artifact))
                .is_ok_and(|results| results.contract == contract.id)
        })
        .collect();
    assert_eq!(
        with_a_run.len(),
        1,
        "the record describes the one contract with a committed run, and this document holds {} of them",
        with_a_run.len()
    );
    let contract = with_a_run[0];
    let results = Results::read(&root.join(&contract.result_artifact)).expect("the committed run");
    let maximum = |name: &str| {
        contract
            .measurements
            .iter()
            .find(|measurement| measurement.name == name)
            .map(|measurement| measurement.maximum)
    };

    let mut stated: Vec<String> = Vec::new();
    for paragraph in paragraphs(&record) {
        let mut named: Vec<String> = Vec::new();
        for statement in statements(&paragraph) {
            match statement {
                BarStatement::Name(name) => named.push(name),
                BarStatement::Ceiling(number) => {
                    assert!(
                        !named.is_empty(),
                        "the record states a ceiling of {number} with no measurement named before it in the same paragraph, so nothing can be compared with the contract"
                    );
                    for name in named.drain(..) {
                        let predeclared = maximum(&name).unwrap_or_else(|| {
                            panic!(
                                "the record states a {number} ceiling for {name}, and the committed contract predeclares no such measurement"
                            )
                        });
                        assert_eq!(
                            number, predeclared,
                            "the record states a {number} ceiling for {name}, and the committed contract predeclares {predeclared}"
                        );
                        stated.push(name);
                    }
                }
            }
        }
    }
    assert!(
        !stated.is_empty(),
        "the record states the bar the committed run was measured against"
    );
    for run in &results.runs {
        for observation in &run.observations {
            assert!(
                stated.contains(&observation.measurement),
                "the committed run observed {}, and the record states no ceiling for it, so the bar it was measured against is not stated where a reader looks",
                observation.measurement
            );
        }
    }
}
