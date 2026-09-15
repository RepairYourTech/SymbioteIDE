//! The repository channel: facts of the tree itself, such as the absence of an
//! Electron dependency or the workspace-wide `unsafe_code = "forbid"` lint.
//!
//! Every check here reads the tree rather than a document, and each fails
//! rather than passes when it finds nothing to read: `no manifest was found to
//! check` is not the same finding as `no manifest declares Electron`.

use crate::catalog::{Fact, Invariant};
use crate::report::Outcome;
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
