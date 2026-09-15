//! The files a package's build reads.
//!
//! This module answers one question: given the package a build script is
//! running in, which files does its build read? It walks the package closure —
//! the package and every package reached through normal and build dependency
//! edges, read by [`crate::manifest`], including the dependencies a member
//! inherits from a workspace root by name and the `[patch]` replacements a root
//! declares — takes every file under each closure
//! package that its binaries compile, follows what those sources include or
//! declare through [`crate::scan`], and adds the manifests, lockfiles and
//! configuration cargo reads for the same packages.
//!
//! It owns what is *not* an input (test, example and bench targets, build
//! output, registry dependencies, dev edges), by the rule the wire file states:
//! its `exclusion` lines name the directory names a walk skips and where it skips
//! them, so what a name means is not this module's to decide; which roots a
//! record's locators are spelled against is [`roots`]' business, and which
//! workspace a build actually resolved in is [`invocation`]'s.
//!
//! ## Structure
//!
//! * `roots` owns the roots a record is written against — the base every
//!   locator is spelled from, and every candidate root a build of a package
//!   could have read, with the `[patch]` and inherited tables each contributes.
//! * `invocation` owns the readings that replace guessing at which workspace a
//!   build resolved in: the directory the invoking cargo was started in, the
//!   manifest its command line named, the build directory, the alias a cargo
//!   configuration expands, and cargo's own answer for a directory.
//! * `asking` owns the two questions only cargo answers, the gate the second one
//!   closes, and the fact that a question could not be put at all.
//! * `paths` owns how a path is spelled in a record.
//! * this module owns the walk itself: the closure, the files under each
//!   package, the includes and `#[path]` modules those files pull in, and the
//!   configuration cargo reads to build any of them.

pub(crate) mod asking;
pub(crate) mod invocation;
pub(crate) mod paths;
pub(crate) mod roots;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use crate::manifest::{self, DependencyEdge, Manifest};
use crate::scan::followed_inputs;
use crate::wire::wire;

use asking::{Answer, asked_packages, gate_against_cargo};
use paths::{canonical, is_rust};
use roots::{patch_directories, resolution_roots, workspace_roots};

/// The configuration files cargo reads for a build of any closure package,
/// relative to the directory it is looked for in.
const CONFIGURATION_FILES: [&str; 4] = [
    ".cargo/config.toml",
    ".cargo/config",
    "rust-toolchain.toml",
    "rust-toolchain",
];

/// Every file the build of the package at `manifest_dir` reads — the walk over
/// each closure package's directory, every file those sources include or
/// declare as a module from wherever it lives, the workspace manifest and
/// lockfile of each closure package's own workspace together with those of the
/// workspaces the build could have been invoked in, and the configuration files
/// cargo reads
/// for the build — together with the include sites the scan cannot follow, each
/// one a reason this build must stop. Followed to a fixed point over the Rust
/// sources, so an included `.rs` file that includes another is recorded too, and
/// a non-Rust file pulled in by `include!` is recorded but not scanned for
/// includes of its own.
///
/// `invocation` is every workspace the build could have been invoked in
/// ([`invocation_roots`]), which for a package another workspace builds is not
/// one the tree names. Every one of them is recorded, even where cargo's own
/// resolution does not reach this package from it ([`resolved_packages`]); what
/// cargo answers is used only to check the closure ([`gate_against_cargo`]).
///
/// The third value is whether every question this walk put to cargo could be
/// asked at all: false where one could not, in which case the closure above was
/// never checked and the build must mark its record rather than write one it
/// cannot stand behind ([`crate::build_stamp`]).
pub(crate) fn recorded_sources(
    manifest_dir: &Path,
    workspace: &Path,
    invocation: &[PathBuf],
) -> (BTreeSet<PathBuf>, Vec<String>, bool) {
    let (asked, cargo_unasked) = match asked_packages(manifest_dir, invocation) {
        Answer::Given(asked) => (asked, false),
        // No question could be put, so nothing was collected to gate with: the walk
        // still records the closure it reached from the manifests, and the mark the
        // caller writes is what says the gate did not run.
        Answer::Unasked => (Vec::new(), true),
    };
    let packages = closure_directories(manifest_dir, invocation);
    gate_against_cargo(&asked, &packages);
    let mut sources = BTreeSet::new();
    for directory in &packages {
        sources.extend(package_sources(directory));
    }

    let mut scanned = BTreeSet::new();
    let mut unfollowed = Vec::new();
    loop {
        let pending: Vec<PathBuf> = sources
            .iter()
            .filter(|source| is_rust(source) && !scanned.contains(*source))
            .cloned()
            .collect();
        if pending.is_empty() {
            break;
        }
        for source in pending {
            scanned.insert(source.clone());
            // The package a file belongs to is the value
            // `env!("CARGO_MANIFEST_DIR")` has in it, which is what a path built
            // from that variable is resolved against.
            let package = enclosing_package(&source, &packages);
            match followed_inputs(&source, package.as_deref()) {
                Ok(included) => {
                    for file in included {
                        let file = canonical(&file);
                        if file.is_file() {
                            sources.insert(file);
                        }
                    }
                }
                Err(problem) => unfollowed.push(problem),
            }
        }
    }

    // A workspace manifest and its lockfile sit above every package of that
    // workspace, so the walk over package directories reaches neither. The
    // lockfile pins the registry dependencies; the manifest configures the
    // packages — a member says `edition.workspace = true`, which makes the
    // edition its code is compiled under a property of a file kept outside it.
    // Every closure package is asked, not only the one being built, and every
    // root cargo can resolve for it rather than the nearest one: see
    // [`workspace_roots`].
    let mut roots = BTreeSet::from([workspace.to_path_buf()]);
    roots.extend(resolution_roots(&packages, invocation));
    for root in roots {
        for shared in ["Cargo.toml", "Cargo.lock"] {
            let file = root.join(shared);
            if file.is_file() {
                sources.insert(file);
            }
        }
    }

    // The files cargo reads to configure a build rather than to find one: a
    // `rustflags` entry in a `.cargo/config.toml`, or the channel a
    // `rust-toolchain.toml` selects, changes what the compiler produces while
    // every source file stays byte-identical. Only those that exist are
    // recorded — the module documentation says why the absent ones are not.
    sources.extend(configuration_files(&packages, workspace));
    (sources, unfollowed, cargo_unasked)
}

/// The configuration files cargo reads for a build of a package in this
/// workspace: each of [`CONFIGURATION_FILES`] in every closure package's own
/// directory and in the workspace root, in every directory above those, and
/// beside the registry cache — whichever of them exist.
///
/// Above a package, because cargo reads configuration from its working
/// directory upwards whichever directory a build was started in; and beside
/// `CARGO_HOME`, because that is where a machine-wide configuration lives.
fn configuration_files(packages: &[PathBuf], workspace: &Path) -> Vec<PathBuf> {
    let mut directories: BTreeSet<PathBuf> = BTreeSet::new();
    for start in packages.iter().map(PathBuf::as_path).chain([workspace]) {
        let mut directory = Some(canonical(start));
        while let Some(candidate) = directory {
            directories.insert(candidate.clone());
            directory = candidate.parent().map(Path::to_path_buf);
        }
    }
    if let Some(cache) = std::env::var_os("CARGO_HOME") {
        directories.insert(PathBuf::from(cache));
    }
    directories
        .into_iter()
        .flat_map(|directory| {
            CONFIGURATION_FILES
                .iter()
                .map(move |name| directory.join(name))
        })
        .filter(|file| file.is_file())
        .map(|file| canonical(&file))
        .collect()
}

/// Every workspace package directory reachable from `manifest_dir` through
/// normal and build dependency edges, the package itself included, and every
/// `[patch]` replacement this build's resolution applies.
fn closure_directories(manifest_dir: &Path, invocation: &[PathBuf]) -> Vec<PathBuf> {
    let mut pending = vec![manifest_dir.to_path_buf()];
    // A patch enters the closure from the roots the *resolution* could have been
    // made in, not from the package that requires the name. Measured: a patch in
    // an ancestor manifest applies to a member's build, while one in a workspace
    // that merely takes a package as a path dependency does not apply to a build
    // of that package by itself, and its own root's patch does not apply to the
    // build that takes it.
    pending.extend(patch_directories(&resolution_roots(
        &[manifest_dir.to_path_buf()],
        invocation,
    )));
    let mut reached = BTreeSet::new();
    while let Some(directory) = pending.pop() {
        let directory = canonical(&directory);
        if !reached.insert(directory.clone()) {
            continue;
        }
        let text = std::fs::read_to_string(directory.join("Cargo.toml"))
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", directory.display()));
        let Manifest {
            mut edges,
            inherited,
            ..
        } = manifest::read(&text);
        // A member that inherits a dependency names no path of its own, and the
        // path is read from the roots *it* can resolve itself in: measured, a
        // package that belongs to a workspace of its own takes `dep.workspace =
        // true` from that root's table even where another workspace's build
        // compiles it.
        edges.append(&mut inherited_edges(&directory, &inherited));
        for edge in edges {
            let dependency = directory.join(&edge.path);
            // An edge cargo reads only where it applies is not an input when the
            // directory is not there. A dependency edge is not conditional, and
            // a directory that cannot be read stops the walk.
            if edge.conditional && !dependency.join("Cargo.toml").is_file() {
                continue;
            }
            pending.push(dependency);
        }
    }
    reached.into_iter().collect()
}

/// The edges a package takes from the roots it can resolve itself in rather than
/// from its own manifest: the path such a root declares for a dependency the
/// package inherits by name (`dep.workspace = true`).
///
/// Every root cargo could have read, not the nearest ancestor. Measured: a
/// member that sits below a nearer `[workspace]` manifest takes the invoked
/// root's entry, and the nearer manifest is never loaded, so which root the
/// build read is not visible from the tree — and the walk asks every candidate
/// [`workspace_roots`] names, the package's own directory among them because a
/// root package declares its own. A candidate's edge the build did not read
/// costs a needless refusal; missing the edge of the root it did read costs the
/// guarantee. A package that names its root has one candidate, so a root it
/// does not name contributes nothing.
fn inherited_edges(package: &Path, inherited: &BTreeSet<String>) -> Vec<DependencyEdge> {
    let mut edges = Vec::new();
    for root in workspace_roots(package) {
        let Ok(text) = std::fs::read_to_string(root.join("Cargo.toml")) else {
            continue;
        };
        for (name, path) in manifest::read(&text).shared {
            if inherited.contains(&name) {
                edges.push(from_root(&root, path));
            }
        }
    }
    edges
}

/// One edge a workspace root declares, as the closure reads it: the root's
/// directory joined to the path, and marked conditional.
///
/// The root's directory, because a root's path is relative to the root rather
/// than to the package that takes the edge — measured, a `path` a member writes
/// beside `workspace = true` is ignored in favour of the root's, in every
/// spelling. Conditional, because a root cargo read must hold the directory it
/// names while a root it never read may name one that is gone.
fn from_root(root: &Path, path: String) -> DependencyEdge {
    DependencyEdge {
        path: root.join(path).display().to_string(),
        conditional: true,
    }
}

/// The package directory a file belongs to: the deepest of `packages` that
/// contains it, which is the value `env!("CARGO_MANIFEST_DIR")` has in it.
fn enclosing_package(source: &Path, packages: &[PathBuf]) -> Option<PathBuf> {
    packages
        .iter()
        .filter(|package| source.starts_with(package))
        .max_by_key(|package| package.components().count())
        .cloned()
}

/// Every file under a package that its binaries compile: all of the package
/// except the directories the wire file's `exclusion` lines skip — at the package
/// root alone where cargo looks for a target directory of that name, wherever
/// they sit where the name is state a build writes, installs or keeps rather than
/// compiles a source from — whatever the file is called. Every file rather than
/// `.rs` alone, because a compile can read a file no extension announces
/// (`include_str!("schema.sql")`).
pub(crate) fn package_sources(root: &Path) -> Vec<PathBuf> {
    let wire = wire();
    let mut sources = BTreeSet::new();
    let mut directories = vec![root.to_path_buf()];
    while let Some(directory) = directories.pop() {
        // The package root is the one place cargo looks for target directories, so
        // it is the one place the file's `root` exclusions apply.
        let at_root = directory == root;
        for entry in std::fs::read_dir(&directory)
            .into_iter()
            .flatten()
            .flatten()
        {
            let name = entry.file_name().to_string_lossy().to_string();
            if wire.skips_directory(&name, at_root) {
                continue;
            }
            match entry.file_type() {
                Ok(kind) if kind.is_dir() => directories.push(entry.path()),
                Ok(_) => {
                    sources.insert(entry.path());
                }
                _ => {}
            }
        }
    }
    sources.into_iter().collect()
}

#[cfg(test)]
mod tests;
