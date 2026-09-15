//! The workspace roots a record is written against.
//!
//! Two questions, one file. [`workspace_root`] is the base a record's relative
//! locators are spelled from, and [`workspace_roots`] is every root a build of a
//! package could have read its resolution in — the candidates a walk keeps
//! rather than choosing between, because which one a build read is not visible
//! from the tree. [`resolution_roots`] is that union over a whole closure,
//! including the workspaces a build could have been invoked in.
//!
//! A `[patch]` table is read only from the roots that are their own workspace
//! ([`patch_directories`]): measured, cargo ignores a member's whole, so a fork a
//! member patches is a fork no build of this package compiles.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use crate::manifest;

use super::paths::canonical;

/// The directory holding the workspace's packages: the root the package names
/// with `package.workspace`, or else the nearest ancestor manifest that
/// declares `[workspace]`. The base every locator in a record is spelled
/// against, and the directory [`crate::changed_sources`] resolves them from.
///
/// A name first, because that is the root cargo reads. Measured: a package whose
/// manifest says `workspace = "../root"` resolves to that root even when an
/// ancestor manifest declares `[workspace]` — `cargo metadata` reports the named
/// directory as `workspace_root`, and replacing the ancestor manifest with text
/// that is not TOML leaves the build succeeding, so the ancestor is not parsed
/// at all. A name whose root does not list the package stops the build
/// (`current package believes it's in a workspace when it's not`), so no record
/// is written for one.
pub(crate) fn workspace_root(manifest_dir: &Path) -> Result<PathBuf, String> {
    let package = canonical(manifest_dir);
    if let Some(target) = manifest::package_workspace(
        &std::fs::read_to_string(package.join("Cargo.toml")).unwrap_or_default(),
    ) {
        return Ok(canonical(&package.join(target)));
    }
    declaring_roots(&package)
        .into_iter()
        .next()
        .ok_or_else(|| format!("no [workspace] manifest above {}", manifest_dir.display()))
}

/// Every workspace root cargo can resolve for the package at `manifest_dir`:
/// the one root a `package.workspace` key names, or else each ancestor manifest
/// that declares `[workspace]` and the package's own directory.
///
/// A named root alone, because a name settles it: measured, cargo resolves such
/// a package to the named directory even where an ancestor manifest declares
/// `[workspace]`, the named root is where an inherited `workspace = true` path
/// is read from (one it does not declare stops the build), and a build whose
/// workspace both nests the package and claims it as a member stops with
/// `member of the wrong workspace`. So no ancestor of a package that names its
/// workspace is a root a build of it can read, and recording one would refuse a
/// binary for a change nothing compiled.
///
/// Without a name, not the nearest ancestor alone, because that is not cargo's
/// rule. An invoked workspace lists members that may sit below another
/// `[workspace]` manifest, and then *the invoked root* is the one whose
/// `[workspace.package]` values a member inherits; the manifest passed on the
/// way down is never loaded, so nothing in the package's own directory tree says
/// which of the two applied. Measured on two such roots that each patch the same
/// name and each list the member: invoked at the outer one `serde` resolves to
/// the outer root's fork, invoked at the nearer one to the nearer root's fork —
/// so both are roots a build of this package could have read, and a record that
/// named only one would report a binary clean after the other's fork changed. A
/// package its workspace `exclude`s is a workspace of one — measured, cargo
/// reads the excluding manifest, walks past it, and resolves the package to
/// itself — which is why its own directory is a root whether or not anything
/// above it excludes it. Recording every candidate is the superset of what cargo
/// can read for a package; naming an input a build of the closure did not read
/// costs a needless refusal, and the opposite costs the guarantee.
pub(crate) fn workspace_roots(manifest_dir: &Path) -> Vec<PathBuf> {
    let package = canonical(manifest_dir);
    if let Some(target) = manifest::package_workspace(
        &std::fs::read_to_string(package.join("Cargo.toml")).unwrap_or_default(),
    ) {
        return vec![canonical(&package.join(target))];
    }
    let mut roots = vec![package.clone()];
    roots.extend(declaring_roots(&package));
    roots.sort();
    roots.dedup();
    roots
}

/// Every root a build of `packages` could have read the resolution in: the
/// candidate roots of each package ([`workspace_roots`]) and the workspaces the
/// build itself could have been invoked in ([`invocation_roots`]).
///
/// One owner, because the walk asks this twice — once for the `[patch]` tables
/// that seed the closure, once for the manifests and lockfiles of everything the
/// closure reached — and two answers would record a resolution that never
/// happened.
pub(crate) fn resolution_roots(packages: &[PathBuf], invocation: &[PathBuf]) -> Vec<PathBuf> {
    let mut roots = BTreeSet::new();
    for package in packages {
        roots.extend(workspace_roots(package));
    }
    roots.extend(invocation.iter().cloned());
    roots.into_iter().collect()
}

/// The `[patch]` replacement directories the resolution roots declare, each
/// followed only where it is there.
///
/// A patch cargo does not apply — because the fork does not satisfy the
/// requirement, which only resolution knows — is followed anyway: see the crate
/// documentation for that limit. A fork that is not there stops the build before
/// a record is written, so one that is absent is skipped.
pub(crate) fn patch_directories(roots: &[PathBuf]) -> Vec<PathBuf> {
    let mut forks = Vec::new();
    for root in roots {
        // A `[patch]` table belongs to the manifest of a directory cargo resolves
        // a build in at that same directory: measured, a member's patch is
        // ignored whole (`patch for the non root package will be ignored`) even
        // where the fork it names is there, while a package no `[workspace]`
        // manifest owns is its own root and its patch applies to a build of it.
        // [`resolution_root`] is that question, asked of the manifest rather than
        // of the directory's name.
        if resolution_root(root).as_deref() != Some(root.as_path()) {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(root.join("Cargo.toml")) else {
            continue;
        };
        for patch in manifest::read(&text).patches {
            let fork = root.join(patch.path);
            if fork.join("Cargo.toml").is_file() {
                forks.push(fork);
            }
        }
    }
    forks
}

/// The workspace roots `directory` and its ancestors declare, nearest first:
/// the directories cargo scans up through to place a manifest kept below them.
pub(crate) fn declaring_roots(directory: &Path) -> Vec<PathBuf> {
    let mut roots = Vec::new();
    let mut candidate = Some(canonical(directory));
    while let Some(directory) = candidate {
        if declares_workspace(&directory) {
            roots.push(directory.clone());
        }
        candidate = directory.parent().map(Path::to_path_buf);
    }
    roots
}

/// Whether the manifest in `directory` declares the directory a workspace root.
pub(crate) fn declares_workspace(directory: &Path) -> bool {
    std::fs::read_to_string(directory.join("Cargo.toml"))
        .is_ok_and(|manifest| manifest::declares_workspace(&manifest))
}

/// The workspace a build resolves in when cargo is run in — or told to build a
/// manifest in — `directory`: cargo's own search up for a manifest, then the root
/// that manifest's package belongs to, or the manifest's own directory where no
/// `[workspace]` manifest owns it.
///
/// A package no workspace declares is its own root, and its `[patch]` tables apply
/// to a build of it: measured, `cargo locate-project --workspace` in such a
/// directory answers its own manifest, `cargo metadata` reports that directory as
/// `workspace_root`, and the build links the fork the `[patch]` table beside it
/// names. Reading only the nearest `[workspace]` manifest instead refused that
/// build — no record at all — while cargo had named the root itself.
pub(crate) fn resolution_root(directory: &Path) -> Option<PathBuf> {
    let package = nearest_manifest(directory)?;
    match workspace_root(&package) {
        Ok(root) => Some(root),
        Err(_) => Some(package),
    }
}

/// The nearest directory at or above `directory` holding a `Cargo.toml`, which is
/// where cargo's own search for the manifest of a build started there stops.
fn nearest_manifest(directory: &Path) -> Option<PathBuf> {
    let mut candidate = Some(canonical(directory));
    while let Some(directory) = candidate {
        if directory.join("Cargo.toml").is_file() {
            return Some(directory);
        }
        candidate = directory.parent().map(Path::to_path_buf);
    }
    None
}
