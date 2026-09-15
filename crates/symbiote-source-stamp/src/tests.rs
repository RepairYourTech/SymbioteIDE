//! The record itself: what `build_stamp` writes, and what `changed_sources`
//! reports for the records a test builds by hand.
//!
//! The manifest reader, the scan and the walk keep their own tests beside them
//! (`manifest::tests`, `scan::tests`, `walk::tests`); the fixture helpers here
//! are the ones they share.

use super::*;

use std::collections::BTreeSet;

use sha2::{Digest, Sha256};

use crate::manifest::{DependencyEdge, read};
use crate::wire::wire;

pub(crate) fn fixture(name: &str) -> PathBuf {
    let directory =
        std::env::temp_dir().join(format!("symbiote-stamp-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).expect("a temporary directory");
    directory
}

/// A binary carrying a record over exactly `sources` — the shape
/// `build_stamp` produces, so a test can check what the walk recorded
/// rather than re-listing what it should have found.
pub(crate) fn record_over(root: &Path, sources: &BTreeSet<PathBuf>) -> PathBuf {
    let mut record = String::new();
    for source in sources {
        let content = std::fs::read(source).expect("a recorded source");
        record.push_str(&format!(
            "{:x}\t{}\n",
            Sha256::digest(&content),
            relative_to(root, source).display()
        ));
    }
    let binary = root.join("symbiote-example");
    std::fs::write(
        &binary,
        format!(
            "binary bytes before {}{record}{} and after",
            wire().start,
            wire().end
        ),
    )
    .expect("the binary");
    binary
}

pub(crate) fn record_for(root: &Path, files: &[(&str, &str)]) -> PathBuf {
    std::fs::write(
        root.join("Cargo.toml"),
        "[workspace]\n[package]\nname = \"symbiote-example\"\n",
    )
    .expect("the root manifest");
    let mut sources = BTreeSet::new();
    for (path, content) in files {
        let file = root.join(path);
        std::fs::create_dir_all(file.parent().expect("a parent")).expect("directories");
        std::fs::write(&file, content).expect("the source");
        sources.insert(file);
    }
    record_over(root, &sources)
}

/// The content a record spells for a file it recorded, in the shape the wire file
/// names for a hash — not filler, which the file's rule reads as neither an input
/// nor a mark.
fn recorded_hash() -> String {
    "0".repeat(wire().hash_length)
}

/// A fixture workspace holding one package, for a test that writes a binary with a
/// record by hand: the directory to write the binary in, and the package whose
/// manifest the record's locators are spelled against.
fn workspace_package(name: &str) -> (PathBuf, PathBuf) {
    let root = fixture(name);
    let package = root.join("ws");
    std::fs::create_dir_all(package.join("src")).expect("the package");
    std::fs::write(package.join("Cargo.toml"), "[workspace]\nmembers = []\n")
        .expect("the root manifest");
    std::fs::write(package.join("src/lib.rs"), "pub fn nothing() {}\n").expect("the source");
    (root, package)
}

#[test]
fn a_record_marked_with_an_unnamed_resolution_is_refused() {
    // Where no reading establishes the workspace cargo resolved in, the record
    // says so and the check refuses it: a record that may be missing the
    // `[patch]` fork the binary compiled cannot be shown to match any tree.
    let (root, workspace) = workspace_package("marked-record");
    let wire = wire();
    let record = format!(
        "{}{}\tsrc/lib.rs\n{}{}",
        wire.start,
        recorded_hash(),
        wire.mark_line("unnamed-resolution"),
        wire.end
    );
    let binary = root.join("binary");
    std::fs::write(&binary, record).expect("the binary carrying the record");
    let problem = changed_sources(&binary, &workspace)
        .expect_err("a marked record is refused rather than compared");
    assert!(
        problem.contains("could not be established") && problem.contains("`PWD`"),
        "the refusal names what is unknown and what to do about it: {problem}"
    );
}

#[test]
fn a_record_marked_by_an_earlier_version_is_refused_by_its_shape() {
    // A mark the wire does not name is read by its content rather than by its
    // name, which the file's rule states: measured, a record carrying the locator
    // an earlier version wrote was compared against a missing file, whose content
    // is the wire's `unset` too, matched it, and was reported clean.
    let (root, workspace) = workspace_package("earlier-mark");
    let wire = wire();
    let earlier = format!(
        "{}{}\tsrc/lib.rs\n{}\trun-directory-not-named\n{}",
        wire.start,
        recorded_hash(),
        wire.unset,
        wire.end
    );
    let binary = root.join("binary");
    std::fs::write(&binary, earlier).expect("the binary carrying the record");
    let problem = changed_sources(&binary, &workspace)
        .expect_err("a record carrying a marker this version does not know is refused");
    assert!(
        problem.contains("run-directory-not-named") && problem.contains(wire.unknown_remedy),
        "the refusal names the line and carries the wire's own remedy for one it does not \
         name: {problem}"
    );
}

#[test]
fn the_first_mark_the_record_spells_is_the_one_reported_where_the_wire_names_none() {
    // The wire names the order the mark reported is read in, and it is the
    // record's own, so the answer cannot depend on how a reader happens to key
    // them — measured, this crate used to report the alphabetically first and
    // `planning/integrity/source_record.py` the first in the record.
    let (root, workspace) = workspace_package("unknown-marks");
    let wire = wire();
    let record = format!(
        "{}{}\tsrc/lib.rs\n{}\tzzz-old\n{}\taaa-old\n{}",
        wire.start,
        recorded_hash(),
        wire.unset,
        wire.unset,
        wire.end
    );
    let binary = root.join("binary");
    std::fs::write(&binary, record).expect("the binary carrying the record");
    let problem = changed_sources(&binary, &workspace)
        .expect_err("a marked record is refused rather than compared");
    assert!(
        problem.contains("zzz-old") && !problem.contains("aaa-old"),
        "the mark reported is the first the record itself spells, whatever its name: {problem}"
    );
}

#[test]
fn a_record_marked_where_cargo_could_not_be_asked_is_refused() {
    // A build whose `CARGO` names a program that cannot be started cannot ask the
    // two questions only cargo answers: the workspace a run directory names, and
    // the closure cargo resolves for the package there. Reading either failure as
    // "nothing here" would write a root list and a closure that nothing checked,
    // so the build marks the record and the check refuses it, naming the remedy
    // that clears the mark rather than a list of differences.
    let (root, workspace) = workspace_package("unaskable-cargo-record");
    let wire = wire();
    let record = format!(
        "{}{}\tsrc/lib.rs\n{}{}",
        wire.start,
        recorded_hash(),
        wire.mark_line("unasked-cargo"),
        wire.end
    );
    let binary = root.join("binary");
    std::fs::write(&binary, record).expect("the binary carrying the record");
    let problem = changed_sources(&binary, &workspace)
        .expect_err("a marked record is refused rather than compared");
    assert!(
        problem.contains("could not be run") && problem.contains("cargo-not-asked"),
        "the refusal names what could not be established and the mark that says so: {problem}"
    );
    assert!(
        problem.contains("a cargo that can be started"),
        "and the rebuild that clears the mark: {problem}"
    );
}

#[test]
fn the_mark_that_blocks_the_rebuild_is_reported_where_a_record_carries_more_than_one() {
    // A record can hold both marks, and they are not equally blocking: the cargo
    // that ran the build is one of the readings that can name the workspace its
    // resolution was read in, so losing it can lose the resolution with it.
    // Reporting the resolution would send the rebuild down the remedy that leaves
    // the cargo unaskable, and the rebuild would be refused again; the cargo is
    // the fact that has to be cleared first, so the refusal names it — whatever
    // order the record's lines are in.
    let (root, workspace) = workspace_package("two-marks");
    let wire = wire();
    // Written in the record's own order — the resolution first — so the mark the
    // refusal names is the wire's choice rather than the first line's.
    let record = format!(
        "{}{}\tsrc/lib.rs\n{}{}{}",
        wire.start,
        recorded_hash(),
        wire.mark_line("unnamed-resolution"),
        wire.mark_line("unasked-cargo"),
        wire.end
    );
    let binary = root.join("binary");
    std::fs::write(&binary, record).expect("the binary carrying the record");
    let problem = changed_sources(&binary, &workspace)
        .expect_err("a marked record is refused rather than compared");
    assert!(
        problem.contains("cargo-not-asked") && problem.contains("a cargo that can be started"),
        "the refusal reports the mark that blocks the rebuild: {problem}"
    );
    assert!(
        !problem.contains("resolution-not-named"),
        "and not the mark whose remedy leaves the cargo unaskable: {problem}"
    );
}

#[test]
fn a_line_the_wire_names_is_read_as_that_mark_whatever_its_content() {
    // A mark is the line's *locator*, not its content, which the file's rule
    // states: measured, the checker read the same line as a file the record had
    // recorded and reported the record clean.
    let (root, workspace) = workspace_package("mark-under-a-hash");
    let wire = wire();
    let record = format!(
        "{}{}\tsrc/lib.rs\n{}\t{}\n{}",
        wire.start,
        recorded_hash(),
        recorded_hash(),
        wire.mark("unasked-cargo").locator,
        wire.end
    );
    let binary = root.join("binary");
    std::fs::write(&binary, record).expect("the binary carrying the record");
    let problem = changed_sources(&binary, &workspace)
        .expect_err("a record naming a mark cannot be stood behind, whatever its content");
    assert!(
        problem.contains("cargo-not-asked") && problem.contains("a cargo that can be started"),
        "the refusal names the mark the line's locator does: {problem}"
    );
}

#[test]
fn a_line_the_wire_does_not_read_is_a_malformed_record() {
    // Neither an input nor a mark: no locator, a blank line, or a content that is
    // neither the wire's `unset` nor a hash. The refusal is the wire's own remedy
    // for a line it cannot read, with the line named, so a record that lost a line
    // is refused rather than read short.
    for (name, line) in [
        ("no-tab", "no tab at all".to_owned()),
        ("blank", String::new()),
        ("no-locator", format!("{}\t", wire().unset)),
        ("not-a-content", "abc\tsrc/lib.rs".to_owned()),
        // A CR is not part of a locator: measured, a CRLF record was read as inputs
        // whose locators each ended in a CR, and a missing file was then compared
        // against the line rather than the line being refused.
        (
            "carriage-return",
            format!("{}\tsrc/lib.rs\r", recorded_hash()),
        ),
    ] {
        let (root, workspace) = workspace_package(name);
        let wire = wire();
        let record = format!(
            "{}{}\tsrc/lib.rs\n{line}\n{}",
            wire.start,
            recorded_hash(),
            wire.end
        );
        let binary = root.join("binary");
        std::fs::write(&binary, record).expect("the binary carrying the record");
        let problem = changed_sources(&binary, &workspace)
            .expect_err("a record with a line the wire does not read cannot be checked");
        assert!(
            problem.contains(wire.malformed_remedy) && problem.contains(&format!("{line:?}")),
            "the refusal is the wire's own remedy for a line it cannot read, naming it: {problem}"
        );
    }
}

#[test]
fn a_malformed_line_is_refused_ahead_of_a_mark_the_record_also_carries() {
    // The precedence the wire states: a record the wire cannot read is one whose
    // marks cannot be stood behind either, so the line is what is named rather
    // than a mark the record also spells. The checker reads every line and
    // reports the line before the mark it also found, and this reader stops at
    // the line as it reads, so a record carrying both is refused one way.
    let (root, workspace) = workspace_package("malformed-before-mark");
    let wire = wire();
    let record = format!(
        "{}{}\tsrc/lib.rs\n{}\t{}\nno tab at all\n{}",
        wire.start,
        recorded_hash(),
        recorded_hash(),
        wire.mark("unasked-cargo").locator,
        wire.end
    );
    let binary = root.join("binary");
    std::fs::write(&binary, record).expect("the binary carrying the record");
    let problem = changed_sources(&binary, &workspace)
        .expect_err("a record the wire cannot read cannot be stood behind");
    assert!(
        problem.contains(wire.malformed_remedy) && problem.contains("no tab at all"),
        "the line is refused rather than the mark the record also carries: {problem}"
    );
    assert!(
        !problem.contains("cargo-not-asked"),
        "the mark beside the line is not what is reported: {problem}"
    );
}

#[test]
fn an_environment_input_that_differs_is_named() {
    let root = fixture("environment");
    std::fs::write(
        root.join("Cargo.toml"),
        "[workspace]\n[package]\nname = \"symbiote-example\"\n",
    )
    .expect("the root manifest");
    let binary = root.join("symbiote-example");
    // A variable no test sets: a record that names it as set is a
    // difference, and a record that names it `unset` is not — which is
    // also the difference between set-and-empty and not set at all.
    let wire = wire();
    for (hash, expected) in [("0123456789abcdef", 1), (wire.unset, 0), ("", 1)] {
        std::fs::write(
            &binary,
            format!(
                "bytes {}{hash}\t{}SYMBIOTE_STAMP_PROBE_VARIABLE\n{} bytes",
                wire.start, wire.prefix, wire.end
            ),
        )
        .expect("the binary");
        assert_eq!(
            changed_sources(&binary, &root)
                .expect("a readable record")
                .len(),
            expected,
            "a record holding {hash:?} for an unset variable"
        );
    }
    assert_eq!(
        changed_sources(&binary, &root).expect("a readable record"),
        [Input::Environment(
            "SYMBIOTE_STAMP_PROBE_VARIABLE".to_owned()
        )]
    );
}

#[test]
fn a_recorded_source_whose_content_differs_is_named() {
    let root = fixture("changed");
    let binary = record_for(&root, &[("src/lib.rs", "one\n")]);
    assert!(
        changed_sources(&binary, &root)
            .expect("a readable record")
            .is_empty()
    );

    std::fs::write(root.join("src/lib.rs"), "two\n").expect("the changed source");
    assert_eq!(
        changed_sources(&binary, &root).expect("a readable record"),
        [Input::File(root.join("src/lib.rs"))]
    );

    std::fs::remove_file(root.join("src/lib.rs")).expect("the removed source");
    assert_eq!(
        changed_sources(&binary, &root).expect("a readable record"),
        [Input::File(root.join("src/lib.rs"))]
    );
}

#[test]
fn a_binary_without_a_record_cannot_be_checked() {
    let root = fixture("no-record");
    let binary = record_for(&root, &[("src/lib.rs", "one\n")]);
    std::fs::write(&binary, "no record here").expect("a binary without a record");
    let problem = changed_sources(&binary, &root).expect_err("no record");
    assert!(problem.contains("carries no build record"), "{problem}");

    let missing = root.join("symbioted");
    let problem = changed_sources(&missing, &root).expect_err("no binary");
    assert!(problem.contains("cannot read the binary"), "{problem}");
}

#[test]
fn the_generated_static_carries_the_record_verbatim() {
    let record = "aa\tcrates/one/Cargo.toml\nbb\tCargo.lock\n";
    let generated = record_static(record);
    assert!(generated.contains(RECORD_STATIC), "{generated}");
    assert!(
        generated.lines().count() == 3,
        "the record must be one escaped literal, not raw lines: {generated}"
    );
    assert!(
        !generated.contains("Cargo.lock\nbb"),
        "a raw newline would end the literal: {generated}"
    );

    // The check reads what a binary holds, which is what that literal
    // compiles to: the record, framed by the markers.
    let binary = format!("noise{}{record}{}noise", wire().start, wire().end);
    assert_eq!(
        embedded_record(binary.as_bytes()).expect("the wire frames and decodes it"),
        Some(record)
    );
}

#[test]
fn a_record_whose_bytes_are_not_the_encoding_is_refused() {
    // The wire states what a record's bytes are, so bytes that are not it are a
    // refusal rather than a binary taken for one carrying no record — measured,
    // this crate reported a binary without one while the checker raised over the
    // same artifact.
    let (root, workspace) = workspace_package("undecodable");
    let wire = wire();
    let hash = recorded_hash();
    // Inside a line, so the record is framed by the rule and the bytes are what it
    // cannot be read for: the wire's `end` follows the LF the last line ends with.
    let prefix = format!("{hash}\tsrc/lib.rs\n{hash}\tsrc/");
    let mut binary = format!("{}{prefix}", wire.start).into_bytes();
    binary.extend_from_slice(&[0xff, 0xfe]);
    binary.extend_from_slice(format!("rs\n{}", wire.end).as_bytes());
    let path = root.join("binary");
    std::fs::write(&path, binary).expect("the binary carrying the record");
    let problem = changed_sources(&path, &workspace)
        .expect_err("a record the wire cannot decode cannot be checked");
    // The whole text is the wire's, offset and encoding filled in by the file's own
    // `{byte}` and `{encoding}` rather than appended by this crate.
    assert!(
        problem.contains(&wire.undecodable(prefix.len()))
            && problem.contains("carries a record that cannot be read"),
        "the refusal is the wire's own text for bytes it cannot decode, with the byte they stop \
         being the encoding at filled in: {problem}"
    );
}

#[test]
fn a_record_whose_own_bytes_spell_a_marker_is_refused_rather_than_compared() {
    // The framing the wire states: the record is the bytes between the markers and
    // holds neither, and the `end` that ends it follows the LF its last line ends
    // with. Measured, a locator legally named after the end marker was framed at
    // that inner marker by both readers, which read a record missing every line
    // after it and reported a binary as matching a tree it was not built from.
    let (root, workspace) = workspace_package("framing");
    let wire = wire();
    let hash = recorded_hash();
    let body = format!("{hash}\tsrc/lib.rs{}\n{hash}\tsrc/other.rs\n", wire.end);
    let path = root.join("binary");
    std::fs::write(&path, format!("{}{body}{}", wire.start, wire.end)).expect("the binary");
    let problem = changed_sources(&path, &workspace)
        .expect_err("a record that spells a marker is not one the wire frames");
    assert!(
        problem.contains(wire.unframed_remedy) && problem.contains("cannot be read"),
        "the refusal is the wire's own remedy for a record it cannot frame: {problem}"
    );

    // And an `end` that ends no line ends no record, so the binary carries none —
    // rather than an empty record, which a reader would compare and call clean.
    std::fs::write(
        &path,
        format!("{}{hash}\tsrc/lib.rs{}", wire.start, wire.end),
    )
    .expect("the binary");
    let problem = changed_sources(&path, &workspace).expect_err("nothing the wire frames");
    assert!(
        problem.contains("carries no build record"),
        "a binary no `end` closes a line of carries no record: {problem}"
    );
}

#[test]
fn a_workspace_whose_root_is_not_a_workspace_is_an_error() {
    let root = fixture("no-workspace");
    let problem = workspace_root(&root).expect_err("no workspace");
    assert!(problem.contains("no [workspace] manifest"), "{problem}");
}
/// The paths of a manifest's edges, for a test that reads which directories
/// the walk would enter rather than how each edge is classified.
pub(crate) fn paths(manifest: &str) -> Vec<String> {
    read(manifest)
        .edges
        .into_iter()
        .map(|edge| edge.path)
        .collect()
}

/// The dependency edges a manifest states, each with whether the walk follows it
/// only where the directory is there.
pub(crate) fn edges(manifest: &str) -> Vec<(String, bool)> {
    stated(read(manifest).edges)
}

/// The `[patch]` edges a manifest states, which cargo reads from a root's
/// manifest only.
pub(crate) fn patches(manifest: &str) -> Vec<(String, bool)> {
    stated(read(manifest).patches)
}

pub(crate) fn stated(edges: Vec<DependencyEdge>) -> Vec<(String, bool)> {
    edges
        .into_iter()
        .map(|edge| (edge.path, edge.conditional))
        .collect()
}
