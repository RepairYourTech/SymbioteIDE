//! The repository channel: facts of the tree itself, such as the absence of an
//! Electron dependency or the workspace-wide `unsafe_code = "forbid"` lint.
//!
//! Every check here reads the tree rather than a document, and each fails
//! rather than passes when it finds nothing to read: `no manifest was found to
//! check` is not the same finding as `no manifest declares Electron`.

use crate::catalog::{Fact, INVARIANTS, Invariant};
use crate::report::Outcome;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// The repository facts an invariant declares.
pub fn outcomes(invariant: &Invariant, root: &Path) -> Vec<Outcome> {
    invariant
        .facts
        .iter()
        .map(|fact| fact_outcome(*fact, root))
        .collect()
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

/// The checks this repository actually runs, as a coverage row may cite them.
///
/// A row's check half is only evidence if what it names exists here, so the
/// names come from the tree and the ledger rather than from the sentence: the
/// jobs the workflows define, the invariants the ledger defines, and the paths
/// and test names of the bindings the ledger claims the harness runs. That last
/// part is not circular — `harness` proves those bindings are compiled and run,
/// and `document` uses that answer here.
///
/// What this cannot resolve is an issue reference: there is no network in this
/// check, so an owner such as `#387` is verified only as being an issue other
/// than the one being accounted for. Whether the named issue owns the work is a
/// judgement recorded in the map, not something this ledger verifies.
#[derive(Debug, Default)]
pub struct Checks {
    names: BTreeSet<String>,
}

impl Checks {
    /// Every check this tree provides.
    ///
    /// Fails rather than passing when the workflows cannot be read: a resolver
    /// with nothing to resolve against has not resolved anything, which is the
    /// same failure mode the facts refuse.
    pub fn read(root: &Path) -> Result<Self, String> {
        let mut names = workflow_jobs(root);
        if names.is_empty() {
            return Err("no workflow job was found to resolve against".into());
        }
        for invariant in INVARIANTS {
            names.insert(invariant.id.to_string());
            for binding in invariant.tests {
                if let Some((path, name)) = binding.split_once("::") {
                    names.insert(path.to_string());
                    names.insert(name.to_string());
                }
            }
        }
        Ok(Self { names })
    }

    /// Whether this repository provides the named check.
    pub fn knows(&self, name: &str) -> bool {
        self.names.contains(name)
    }
}

/// The job names the workflows define: a key indented one level under a
/// top-level `jobs:` block, which is the shape every workflow here uses.
fn workflow_jobs(root: &Path) -> BTreeSet<String> {
    let mut jobs = BTreeSet::new();
    let Ok(entries) = std::fs::read_dir(root.join(".github/workflows")) else {
        return jobs;
    };
    let mut files: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| {
            matches!(
                path.extension().and_then(|e| e.to_str()),
                Some("yml" | "yaml")
            )
        })
        .collect();
    files.sort();
    for file in files {
        let Ok(text) = std::fs::read_to_string(&file) else {
            continue;
        };
        let mut in_jobs = false;
        for line in text.lines() {
            if line.starts_with("jobs:") {
                in_jobs = true;
                continue;
            }
            if !in_jobs {
                continue;
            }
            // A blank line separates jobs rather than ending the block.
            if line.trim().is_empty() {
                continue;
            }
            let Some(under_jobs) = line.strip_prefix("  ") else {
                in_jobs = false;
                continue;
            };
            // Only the job keys themselves, not the keys inside a job.
            if under_jobs.starts_with(' ') {
                continue;
            }
            let Some((key, _)) = under_jobs.split_once(':') else {
                continue;
            };
            let key = key.trim();
            if !key.is_empty()
                && key
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
            {
                jobs.insert(key.to_string());
            }
        }
    }
    jobs
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
