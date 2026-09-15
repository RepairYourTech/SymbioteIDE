//! The questions only cargo can answer about a build.
//!
//! Two of them: the workspace a run directory belongs to ([`cargo_workspace`],
//! which the invocation readings ask), and the packages cargo resolves for this
//! one in a root ([`resolved_packages`]) — the second is what gates the closure
//! the walk read out of the manifests ([`gate_against_cargo`]), because those
//! roots are reached through their `[patch]` and inherited tables rather than by
//! walking their own.
//!
//! [`Answer::Unasked`] is the third fact here and not an answer: the cargo that
//! is running this build could not be run at all, so the question was never put,
//! and a caller must pass that on rather than read it as "nothing here". A build
//! marks its record for it ([`crate::build_stamp`]).

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::manifest;

use super::paths::canonical;
use super::roots::workspace_roots;

/// Cargo's answer to a question about a build, where the question could be put
/// at all.
///
/// `Unasked` is not an answer. It means the cargo running this build could not
/// be run — `CARGO` names no program, or one that cannot be started — so nothing
/// was asked, and a walk that read it as "nothing here" would write a record
/// that is quietly short in exactly the two places cargo is asked: the root a
/// run directory names ([`cargo_workspace`]) and the closure gate
/// ([`asked_packages`]). Both callers pass it on instead, and the build marks its
/// record ([`crate::build_stamp`]) so the check refuses it rather than comparing
/// it.
pub(crate) enum Answer<T> {
    /// Cargo ran, and this is what it said — whether or not it succeeded: a
    /// failed query is an answer (`cargo locate-project` fails in a directory no
    /// workspace holds), and so is an empty one.
    Given(T),
    /// The cargo running this build could not be run to be asked.
    Unasked,
}

/// Cargo's output when it is run in `directory`, or [`Answer::Unasked`] when it
/// could not be run at all. The cargo that is building this build script is the
/// one asked, by the path cargo itself sets, so the answer describes the
/// toolchain in use rather than whichever cargo is first on `PATH`.
pub(crate) fn asked_cargo(directory: &Path, args: &[&str]) -> Answer<std::process::Output> {
    let Some(cargo) = std::env::var_os("CARGO") else {
        return Answer::Unasked;
    };
    Command::new(cargo)
        .args(args)
        .current_dir(directory)
        .output()
        .map_or(Answer::Unasked, Answer::Given)
}

/// The name the package at `manifest_dir` declares, which is what cargo
/// selects a package by.
fn package_name(manifest_dir: &Path) -> Option<String> {
    let text = std::fs::read_to_string(manifest_dir.join("Cargo.toml")).ok()?;
    manifest::package_name(&text)
}

/// The packages cargo resolves for the package at `manifest_dir` in `root`,
/// where it resolves it there at all; the empty set where cargo answered that it
/// resolves the package nowhere here, or ran not at all —
/// [`Answer::Unasked`] is the second case, which the caller must pass on rather
/// than read as the first.
///
/// Measured: `cargo tree -p <name>` run in a workspace that has the package —
/// as a member or through a path dependency of one, optional or not — prints
/// that package's subtree with the paths its resolution reads (`path` forks of
/// the workspace's `[patch]` tables included). It answers `package ID
/// specification … did not match any packages` when the features and target it
/// selected leave the package out of the graph, which is **not** the fact that
/// the workspace cannot build it: measured, a workspace whose member takes the
/// package as an *optional* dependency answers that way under the default
/// features a build elsewhere selected while that build compiles the package
/// and that workspace's own fork of it. So only the positive answer is used as
/// one, and it is asked with `--offline`, so a build in progress is never sent to
/// the network for this.
pub(crate) fn resolved_packages(root: &Path, manifest_dir: &Path) -> Answer<BTreeSet<PathBuf>> {
    let Some(name) = package_name(manifest_dir) else {
        // Nothing to ask about: a manifest that names no package gives the gate no
        // package to require of this root.
        return Answer::Given(BTreeSet::new());
    };
    let args = [
        "tree",
        "-p",
        &name,
        "--offline",
        "--edges",
        "normal,build",
        "--prefix",
        "none",
    ];
    match asked_cargo(root, &args) {
        // A non-zero exit is cargo's answer that it resolves the package nowhere in
        // this root, which gates nothing: the empty set rather than a shorter list,
        // so that a caller cannot read a refusal to answer as an answer.
        Answer::Given(output) => {
            let resolved = if output.status.success() {
                paths_in(&String::from_utf8_lossy(&output.stdout))
            } else {
                BTreeSet::new()
            };
            Answer::Given(resolved)
        }
        Answer::Unasked => Answer::Unasked,
    }
}

/// Cargo's own answer for the workspaces the package is not part of: for each,
/// the packages cargo resolves for this one there, where it resolves it there
/// at all.
///
/// It is used to check the closure, never to drop a root — see
/// [`resolved_packages`] for why a failure of the query is not evidence of
/// anything. The roots the package's own manifests name are not asked about:
/// their closure is what the walk reads out of those manifests.
pub(crate) fn asked_packages(
    manifest_dir: &Path,
    invocation: &[PathBuf],
) -> Answer<Vec<(PathBuf, BTreeSet<PathBuf>)>> {
    let own = workspace_roots(manifest_dir);
    let mut asked = Vec::new();
    for root in invocation.iter().filter(|root| !own.contains(root)) {
        match resolved_packages(root, manifest_dir) {
            Answer::Given(packages) => asked.push((root.clone(), packages)),
            // One question that could not be put makes the gate below unusable: a
            // root it would have answered for goes unchecked, so the caller marks
            // the record instead of writing one no gate stood behind.
            Answer::Unasked => return Answer::Unasked,
        }
    }
    Answer::Given(asked)
}

/// Stops the build when a package cargo resolved for this one in a workspace the
/// build could have been invoked in is not among `packages` — the closure the
/// walk reached through the manifests.
///
/// Those roots are reached through their `[patch]` and inherited tables rather
/// than by walking their closure, so this is where a short record could hide;
/// one that omits an input lets a proof pass against a binary built from other
/// sources, which is the failure this crate exists to prevent.
pub(crate) fn gate_against_cargo(asked: &[(PathBuf, BTreeSet<PathBuf>)], packages: &[PathBuf]) {
    for (root, resolved) in asked {
        for package in resolved {
            if !packages.contains(package) {
                panic!(
                    "cannot record the sources under test: cargo resolves {} for a build of this \
                     package in {}, and the walk over the manifests did not reach it. A record \
                     that does not name every input a binary compiles would let a proof pass \
                     against a binary built from other sources.",
                    package.display(),
                    root.display(),
                );
            }
        }
    }
}

/// The package directories in `cargo tree --prefix none` output: cargo writes a
/// package it resolved from a directory as `name v1.2.3 (/directory)` — the
/// directory is there when the package is one of this tree's, whatever its
/// `[patch]` table replaced — and a package it resolved from the registry names
/// no directory at all, while one from a git or registry *source* names a URL
/// there.
pub(crate) fn paths_in(tree: &str) -> BTreeSet<PathBuf> {
    let mut directories = BTreeSet::new();
    for line in tree.lines() {
        let Some((_, rest)) = line.split_once(" (") else {
            continue;
        };
        let Some(end) = rest.find(')') else {
            continue;
        };
        let directory = Path::new(&rest[..end]);
        if directory.is_absolute() && directory.is_dir() {
            directories.insert(canonical(directory));
        }
    }
    directories
}
