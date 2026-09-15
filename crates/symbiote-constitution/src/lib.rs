//! The constitution's non-negotiable invariants, as machine-checked data (#170).
//!
//! Prose cannot fail a build, so an invariant that lives only in a paragraph is
//! a promise rather than a property. This crate turns every non-negotiable
//! invariant named by the [product constitution](../../../docs/architecture/product-constitution.md)
//! and by issue #170 into a record with a stable identifier, the exact normative
//! text it depends on, and the checks that must pass for it to hold. Each
//! invariant is evaluated through three channels, and a channel that finds
//! nothing to check is a failure rather than a pass:
//!
//! * **document** — [`document`]: a required clause of the constitution must be
//!   present, and a forbidden authorization absent. The document is embedded
//!   with `include_str!`, so the checks read the text compiled into the binary,
//!   not whatever a later run finds on disk. The same module parses the
//!   constitution's own coverage map, whose every row must cite a check this
//!   repository provides or an issue other than the one being accounted for.
//! * **repository** — [`repository`]: a fact of the tree itself, such as the
//!   absence of an Electron dependency or the workspace-wide
//!   `unsafe_code = "forbid"` lint. The same reading answers [`repository::Checks`],
//!   the checks this tree actually runs, which a coverage row's check names
//!   resolve against rather than being taken at their word.
//! * **test** — [`harness`]: a named test that must be a check the harness
//!   actually compiles and runs, carry `#[test]` (or `def` for the Python
//!   maintenance suites), and not be `#[ignore]`d. [`Harness`] reads cargo's own
//!   target list and the workflow's discovery patterns, so a binding cannot cite
//!   a file the harness never compiles.
//!
//! [`report::evaluate`] produces the per-invariant report published as evidence
//! against the issue, committed under [`REPORT_PATH`] and diffed against a fresh
//! run so the record cannot drift from the tree.
//!
//! Each concern has one owner: [`catalog`] holds the inventory and nothing else,
//! [`document`], [`repository`] and [`harness`] each own one channel's rules,
//! and [`report`] owns the verdict vocabulary, the assembly of the three
//! channels and the committed encoding. A channel depends on [`report`] for the
//! verdict type and never on another channel, so a change to one channel's rules
//! lands in that channel's file. Where an invariant is an integration obligation
//! owned by another canonical issue, the catalog names that owner instead of
//! claiming a behavior this crate does not implement.

pub mod catalog;
pub mod document;
pub mod harness;
pub mod report;
pub mod repository;

pub use catalog::{EXPLANATIONS, Fact, INVARIANTS, Invariant};
pub use document::CONSTITUTION;
pub use harness::Harness;
pub use report::{Coverage, Outcome, REPORT_PATH, Report, evaluate, report_json, report_to_json};

use std::path::{Path, PathBuf};

/// The workspace root, derived from this crate's manifest so a test or example
/// never depends on the process's current directory. It is resolved through the
/// filesystem, because the manifest directory reaches it through `..` and a
/// path that still holds those components does not compare equal to the
/// absolute paths cargo reports.
pub fn workspace_root() -> PathBuf {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    root.canonicalize().unwrap_or(root)
}
