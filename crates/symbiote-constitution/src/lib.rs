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
//! * **document** — a required clause of the constitution must be present, and a
//!   forbidden authorization absent. The document is embedded with
//!   [`include_str!`], so the checks read the text compiled into the binary, not
//!   whatever a later run finds on disk.
//! * **repository** — a fact of the tree itself, such as the absence of an
//!   Electron dependency or the workspace-wide `unsafe_code = "forbid"` lint.
//! * **test** — a named test in this workspace that must exist, carry `#[test]`
//!   (or `def` for the Python maintenance suites), and not be `#[ignore]`d. The
//!   workspace test run executes it; the binding is what keeps the ledger from
//!   drifting away from the test it points at.
//!
//! [`evaluate`] produces the per-invariant report that is published as evidence
//! against the issue. Where an invariant is an integration obligation owned by
//! another canonical issue, the record names that owner instead of claiming a
//! behavior this change does not implement.

use serde::Serialize;
use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

/// The constitution, embedded so that a check can never read a different file
/// than the one this build compiled.
pub const CONSTITUTION: &str = include_str!("../../../docs/architecture/product-constitution.md");

/// A fact of the repository tree rather than of a document or a test.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fact {
    /// No manifest declares Electron, whose use the constitution forbids.
    NoElectron,
    /// No Go source or module exists: Go is not a second core language.
    NoGoCore,
    /// The workspace forbids unsafe Rust and every crate inherits that lint.
    ForbidUnsafeRust,
    /// The workbench is strict TypeScript and its build type-checks the tree.
    StrictTypeScript,
}

impl Fact {
    pub fn name(self) -> &'static str {
        match self {
            Fact::NoElectron => "no_electron",
            Fact::NoGoCore => "no_go_core",
            Fact::ForbidUnsafeRust => "forbid_unsafe_rust",
            Fact::StrictTypeScript => "strict_typescript",
        }
    }
}

/// One non-negotiable invariant with the evidence that must exist for it to
/// hold. The fields are `&'static` so the catalog is a compile-time constant
/// that no runtime input can weaken.
#[derive(Debug, Clone, Copy)]
pub struct Invariant {
    /// Stable identifier quoted by the report and by `docs/contracts/constitution.md`.
    pub id: &'static str,
    /// The requirement this invariant satisfies, quoted from #170 or the constitution.
    pub requirement: &'static str,
    /// The invariant in one normative sentence.
    pub statement: &'static str,
    /// Clauses that must appear verbatim in the constitution.
    pub document: &'static [&'static str],
    /// Authorizations that must not appear; the constitution's non-goals are
    /// machine-enforced as absences, since prose that permits a forbidden
    /// choice reads as policy.
    pub forbidden: &'static [&'static str],
    /// `relative/path.rs::test_name` bindings that the workspace test run executes.
    pub tests: &'static [&'static str],
    /// Facts of the tree that must hold.
    pub facts: &'static [Fact],
    /// Canonical issues that own the integration this invariant does not implement.
    pub owners: &'static [u64],
}

/// One channel's verdict for one invariant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Outcome {
    /// `document`, `repository` or `test`.
    pub channel: &'static str,
    /// What was checked: the clause, the fact, or the binding.
    pub subject: String,
    pub ok: bool,
    /// Why it passed, or what is missing when it did not.
    pub detail: String,
}

/// Everything checked for one invariant, in catalog order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Coverage {
    pub id: &'static str,
    pub requirement: &'static str,
    pub statement: &'static str,
    pub owners: Vec<u64>,
    pub outcomes: Vec<Outcome>,
}

impl Coverage {
    /// Whether every channel passed and at least one channel was checked.
    pub fn is_proven(&self) -> bool {
        !self.outcomes.is_empty() && self.outcomes.iter().all(|outcome| outcome.ok)
    }
}

/// The whole report, as published against the issue.
#[derive(Debug, Clone, Serialize)]
pub struct Report {
    pub invariants: Vec<Coverage>,
    /// Total channels checked across every invariant.
    pub checked: usize,
    /// One line per failed channel; empty when the constitution conforms.
    pub failures: Vec<String>,
}

/// The workspace root, derived from this crate's manifest so a test or example
/// never depends on the process's current directory.
pub fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn pass(channel: &'static str, subject: String, detail: &str) -> Outcome {
    Outcome {
        channel,
        subject,
        ok: true,
        detail: detail.into(),
    }
}

fn fail(channel: &'static str, subject: String, detail: &str) -> Outcome {
    Outcome {
        channel,
        subject,
        ok: false,
        detail: detail.into(),
    }
}

/// Clause checks against a constitution's text, so a test can also run them
/// against a deliberately weakened copy rather than only against the real file.
pub fn document_outcomes(invariant: &Invariant, document: &str) -> Vec<Outcome> {
    let mut outcomes = Vec::new();
    for clause in invariant.document {
        let subject = format!("requires: {clause}");
        outcomes.push(if document.contains(clause) {
            pass("document", subject, "clause present")
        } else {
            fail(
                "document",
                subject,
                "clause is missing from the constitution",
            )
        });
    }
    for clause in invariant.forbidden {
        let subject = format!("forbids: {clause}");
        let present = document.contains(clause);
        outcomes.push(if present {
            fail("document", subject, "a forbidden authorization is present")
        } else {
            pass("document", subject, "authorization is absent")
        });
    }
    outcomes
}

/// The repository facts an invariant declares.
pub fn repository_outcomes(invariant: &Invariant, root: &Path) -> Vec<Outcome> {
    invariant
        .facts
        .iter()
        .map(|fact| fact_outcome(*fact, root))
        .collect()
}

/// The named tests an invariant declares, checked for existence and for being
/// run rather than skipped.
pub fn test_outcomes(invariant: &Invariant, root: &Path) -> Vec<Outcome> {
    invariant
        .tests
        .iter()
        .map(|binding| binding_outcome(root, binding))
        .collect()
}

/// Check one `relative/path::name` test binding.
pub fn binding_outcome(root: &Path, binding: &str) -> Outcome {
    let subject = binding.to_string();
    let Some((path, name)) = binding.rsplit_once("::") else {
        return fail("test", subject, "binding is not `path::test_name`");
    };
    let Ok(source) = std::fs::read_to_string(root.join(path)) else {
        return fail("test", subject, "the named test file does not exist");
    };
    let python = path.ends_with(".py");
    let needle = if python {
        format!("def {name}(")
    } else {
        format!("fn {name}(")
    };
    let Some(at) = source.find(&needle) else {
        return fail(
            "test",
            subject,
            "no test by that name in the named file, so the evidence moved",
        );
    };
    let line_start = source[..at].rfind('\n').map_or(0, |end| end + 1);
    let attributes = attribute_block(&source[..line_start]);
    if !python && !attributes.contains("#[test]") {
        return fail(
            "test",
            subject,
            "the named item is not a test: no #[test] attribute",
        );
    }
    if attributes.contains("#[ignore") || attributes.contains("@unittest.skip") {
        return fail(
            "test",
            subject,
            "the named test is skipped, and a skipped check is not evidence",
        );
    }
    pass("test", subject, "exists and runs with the workspace tests")
}

/// The contiguous attribute/doc-comment block directly above an item, read so a
/// skipped or non-test item cannot be passed off as evidence.
fn attribute_block(before: &str) -> String {
    let mut block = Vec::new();
    for line in before.lines().rev() {
        let trimmed = line.trim();
        if trimmed.starts_with("#[")
            || trimmed.starts_with("///")
            || trimmed.starts_with("//!")
            || trimmed.starts_with('@')
        {
            block.push(trimmed);
        } else {
            break;
        }
    }
    block.join("\n")
}

/// Evaluate one repository fact, so a test can also assert it directly.
pub fn fact_outcome(fact: Fact, root: &Path) -> Outcome {
    let subject = fact.name().to_string();
    let (ok, detail) = match fact {
        Fact::NoElectron => electron_absence(root),
        Fact::NoGoCore => go_absence(root),
        Fact::ForbidUnsafeRust => forbid_unsafe(root),
        Fact::StrictTypeScript => strict_typescript(root),
    };
    Outcome {
        channel: "repository",
        subject,
        ok,
        detail,
    }
}

/// Every manifest in the tree, excluding build output and dependency caches.
fn manifests(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    walk(root, &mut |path| {
        if matches!(
            path.file_name().and_then(|name| name.to_str()),
            Some("Cargo.toml" | "package.json")
        ) {
            found.push(path.to_path_buf());
        }
    });
    found.sort();
    found
}

fn electron_absence(root: &Path) -> (bool, String) {
    let manifests = manifests(root);
    if manifests.is_empty() {
        // Nothing read is not the same as nothing declared.
        return (false, "no manifest was found to check".into());
    }
    let mut declared = Vec::new();
    let mut read = 0;
    for manifest in manifests {
        let Ok(text) = std::fs::read_to_string(&manifest) else {
            continue;
        };
        read += 1;
        let package = manifest
            .strip_prefix(root)
            .unwrap_or(&manifest)
            .display()
            .to_string();
        if text.lines().any(declares_electron) {
            declared.push(package);
        }
    }
    if read == 0 {
        return (false, "no manifest could be read".into());
    }
    if declared.is_empty() {
        return (true, format!("no manifest of {read} declares Electron"));
    }
    (
        false,
        format!("{} declares an Electron dependency", declared.join(", ")),
    )
}

/// Whether one manifest line is a dependency on Electron or a scoped Electron
/// package. Only the key position counts, so prose about Electron is not a
/// declaration.
fn declares_electron(line: &str) -> bool {
    let key = line
        .trim()
        .split(['=', ':'])
        .next()
        .unwrap_or("")
        .trim()
        .trim_matches('"');
    key == "electron" || key.starts_with("electron-") || key.starts_with("@electron/")
}

fn go_absence(root: &Path) -> (bool, String) {
    let mut found = Vec::new();
    walk(root, &mut |path| {
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");
        if name == "go.mod"
            || (name.ends_with(".go") && path.extension().is_some_and(|ext| ext == "go"))
        {
            found.push(
                path.strip_prefix(root)
                    .unwrap_or(path)
                    .display()
                    .to_string(),
            );
        }
    });
    if found.is_empty() {
        return (true, "no Go source or module exists".into());
    }
    (false, format!("Go core files exist: {}", found.join(", ")))
}

fn forbid_unsafe(root: &Path) -> (bool, String) {
    let Ok(manifest) = std::fs::read_to_string(root.join("Cargo.toml")) else {
        return (false, "the workspace manifest is unreadable".into());
    };
    if !manifest.contains("unsafe_code = \"forbid\"") {
        return (false, "the workspace does not forbid unsafe Rust".into());
    }
    let members = crate_manifests(root);
    if members.is_empty() {
        return (false, "no crate manifest was found to check".into());
    }
    let mut missing = Vec::new();
    for member in members {
        let Ok(text) = std::fs::read_to_string(&member) else {
            missing.push(member.display().to_string());
            continue;
        };
        // A crate that does not inherit the workspace lints may still compile,
        // so the forbid must be claimed by every member, not only declared once.
        if !text.contains("[lints]") || !text.contains("workspace = true") {
            missing.push(
                member
                    .strip_prefix(root)
                    .unwrap_or(&member)
                    .display()
                    .to_string(),
            );
        }
    }
    if missing.is_empty() {
        return (
            true,
            "the workspace forbids unsafe Rust in every member".into(),
        );
    }
    (
        false,
        format!(
            "members not inheriting the workspace lints: {}",
            missing.join(", ")
        ),
    )
}

fn crate_manifests(root: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(root.join("crates")) else {
        return Vec::new();
    };
    let mut found: Vec<_> = entries
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path().join("Cargo.toml"))
        .filter(|path| path.is_file())
        .collect();
    found.sort();
    found
}

fn strict_typescript(root: &Path) -> (bool, String) {
    let app = root.join("crates/symbiote-desktop/app");
    let Ok(tsconfig) = std::fs::read_to_string(app.join("tsconfig.json")) else {
        return (false, "the workbench tsconfig is unreadable".into());
    };
    if !tsconfig.contains("\"strict\": true") {
        return (false, "the workbench compiler is not strict".into());
    }
    let Ok(package) = std::fs::read_to_string(app.join("package.json")) else {
        return (false, "the workbench package manifest is unreadable".into());
    };
    if !package.contains("\"typecheck\"") {
        return (false, "the workbench build does not type-check".into());
    }
    (
        true,
        "the workbench is strict TypeScript and type-checks".into(),
    )
}

/// A bounded, deterministic walk that skips build output, dependency caches and
/// version control rather than reporting their contents.
fn walk(dir: &Path, visit: &mut impl FnMut(&Path)) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut children: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok())
        .map(|e| e.path())
        .collect();
    children.sort();
    for path in children {
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");
        if path.is_dir() {
            if matches!(
                name,
                ".git" | "target" | "node_modules" | "dist" | ".freebuff"
            ) {
                continue;
            }
            walk(&path, visit);
        } else {
            visit(&path);
        }
    }
}

/// Evaluate every invariant against the embedded constitution and the tree.
pub fn evaluate(root: &Path) -> Report {
    let mut invariants = Vec::with_capacity(INVARIANTS.len());
    let mut failures = Vec::new();
    let mut checked = 0;
    for invariant in INVARIANTS {
        let mut outcomes = document_outcomes(invariant, CONSTITUTION);
        outcomes.extend(repository_outcomes(invariant, root));
        outcomes.extend(test_outcomes(invariant, root));
        checked += outcomes.len();
        for outcome in outcomes.iter().filter(|outcome| !outcome.ok) {
            let mut line = String::new();
            let _ = write!(
                line,
                "{} {} [{}]: {}",
                invariant.id, outcome.subject, outcome.channel, outcome.detail
            );
            failures.push(line);
        }
        invariants.push(Coverage {
            id: invariant.id,
            requirement: invariant.requirement,
            statement: invariant.statement,
            owners: invariant.owners.to_vec(),
            outcomes,
        });
    }
    Report {
        invariants,
        checked,
        failures,
    }
}

/// The identifiers the repository actually declares, so a caller can prove the
/// report covers the whole catalog rather than a subset.
pub fn identifiers() -> BTreeSet<&'static str> {
    INVARIANTS.iter().map(|invariant| invariant.id).collect()
}

include!("catalog.rs");
