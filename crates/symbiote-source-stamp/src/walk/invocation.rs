//! The workspace a build resolved in, from the readings that can name it.
//!
//! Cargo resolves a build in the workspace of the directory it was run in, or of
//! the manifest its command line named, and its own scan starts at a directory
//! and looks for the manifest above it. None of that is visible in the package's
//! own directory, so the readings here are the invoking process
//! ([`invoking_process`]: its working directory and its command line), the
//! manifest that command line named, the alias a cargo configuration expands it
//! to, the build directory the output was written into, and cargo's own answer
//! for a directory ([`cargo_workspace`]).
//!
//! They are candidates, not a choice — and one of them decides whether the
//! record can be stood behind at all: the workspace the resolution was read in
//! ([`Invocation::resolution_named`]).

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use crate::manifest;

use super::asking::{Answer, asked_cargo};
use super::paths::canonical;
use super::roots::{declares_workspace, declaring_roots, resolution_root};

/// The workspaces a build could have been invoked in, and the two facts that
/// decide whether a record written from them can be stood behind.
pub(crate) struct Invocation {
    /// Every root a reading named, sorted and without duplicates.
    pub(crate) roots: Vec<PathBuf>,
    /// Whether a reading establishes the workspace cargo resolved in: the
    /// invoking process's command line named the manifest it built, or named
    /// none and cargo's own scan starts in the directory it was run in. Where
    /// this is false the record is marked and the guard refuses it rather than
    /// reporting it clean.
    pub(crate) resolution_named: bool,
    /// Whether every question put to cargo about this build's roots could be
    /// asked at all ([`Answer`]). Where this is false the record is marked and
    /// the guard refuses it: a root the answer would have named is missing from
    /// the record, and a change to what it holds would be reported clean.
    pub(crate) cargo_unasked: bool,
}

/// Every workspace the build a build script belongs to could have been invoked
/// in, from the two things that can say so: the build directory the output was
/// written to, and the directory cargo was run in.
///
/// A package another workspace compiles needs them because nothing in the
/// package's own directory names that root. Measured: `cargo build -p <path
/// dependency>` from a sibling workspace builds that package's binary with the
/// sibling's `[patch]` — the binary prints the sibling's fork — while its build
/// script's environment differs from its own build's in nothing but `OUT_DIR`,
/// the sibling's build directory; and the sibling's manifest and lockfile are
/// what that resolution read.
///
/// Cargo writes a build script's output to `<build directory>/[<target
/// triple>/]<profile>/build/<package>-<hash>/out`, and *which* of the two levels
/// above the profile is the build directory is not decidable from the path: with
/// `--target` the triple sits between them, and without it a directory may
/// merely be named like a triple (measured: `CARGO_TARGET_DIR=<root>/
/// x86_64-unknown-linux-gnu` with no `--target` names the host triple and inserts
/// no level). So both readings are candidates rather than a choice, each taken
/// only where the directory above it declares `[workspace]`. A candidate the
/// build did not read costs a refusal no rebuild clears; choosing wrong costs a
/// record that omits a fork the binary compiled, which the guard then reports
/// clean — the failure this crate exists to prevent — so the union is the safe
/// side of that trade.
///
/// The build directory is not the only thing that can say where the build was
/// invoked. Cargo finds the workspace to build by scanning up from the directory
/// it was run in, so that directory names a candidate too: measured, a sibling
/// workspace that pointed `CARGO_TARGET_DIR` at another workspace's target
/// directory — `CARGO_TARGET_DIR=<abs>/ws2/target`, run in `ws1` — builds with
/// `ws1`'s fork while leaving no other trace of `ws1` anywhere, and the target
/// directory it wrote into belongs to `ws2`. Every directory [`invoking_process`]
/// and the caller's `PWD` name is read that way, and the candidate is cargo's own
/// answer for it rather than a second implementation of the same scan: measured,
/// `cargo locate-project --workspace` run there follows a `package.workspace`
/// name and answers the root cargo would load for a build started in that
/// directory.
///
/// Neither of those readings is enough where cargo was told which manifest to
/// build, because the resolution is then read in *that* manifest's workspace
/// while the directory cargo was run in names another one. Measured: `cd third &&
/// CARGO_TARGET_DIR=<outside every workspace> cargo build --manifest-path
/// ../sibling/member/Cargo.toml` compiles the sibling's `[patch]` fork — the
/// `serde` it links prints the sibling's name — while the build directory names
/// no workspace at all, `third` is the only run directory, and the record it
/// wrote named `third`'s fork and missed the sibling's. So where the invoking
/// process can be read, its command line is read too and the manifest it names is
/// placed the way cargo places it: the root that manifest's package belongs to,
/// which cargo itself reports for the same question (`cargo metadata
/// --manifest-path ../sibling/member/Cargo.toml` answers
/// `"workspace_root": "…/sibling"`, and a manifest naming its own root answers
/// `…/own`). That root is a candidate, and its `[patch]` tables seed the closure.
///
/// Every root a reading names is a candidate, and no reading is narrowed by
/// asking cargo whether it could have built the package: measured, cargo's own
/// answer to that question is a false negative for a workspace that reaches the
/// package through an optional dependency its build selected (see
/// [`resolved_packages`]), so narrowing on it would trade a refusal a rebuild
/// clears for a record that omits a compiled fork.
///
/// A build that no reading places stops instead of guessing: with no reading
/// naming a workspace at all, the `[patch]` tables the build read are unknown, so
/// a record written here could omit the fork the binary compiled and a proof
/// would pass against it — the failure this crate exists to prevent. Measured,
/// that is a build whose output directory is outside every workspace, whose run
/// directory names none, and whose command line names no manifest at all: `cd
/// /tmp && CARGO_TARGET_DIR=/tmp/out cargo build`. A manifest no `[workspace]`
/// manifest owns does not reach it — it is its own root ([`resolution_root`]), so
/// the build records rather than failing. Where the reading exists, the command
/// line places the `--manifest-path` arrangements this used to refuse, and their
/// records became correct rather than absent; on a platform without the reading,
/// they are still what reaches this panic.
///
/// Where a root *is* named but the workspace the resolution was read in is not
/// established, the record is written carrying that fact
/// ([`Invocation::resolution_named`]) and the guard refuses it rather than
/// reporting it clean: a record that may be missing the `[patch]` fork the binary
/// compiled cannot be stood behind. On a platform that keeps the reading that
/// cannot happen, because the command line or the directory places the build; it
/// happens where the reading exists and fails — the parent process is not the
/// cargo `CARGO` names, or the kernel hides it. Where the platform keeps no
/// reading at all `PWD` is all there is, and whether it names the resolution's
/// workspace is exactly what cannot be told: measured, a launcher that sets
/// cargo's working directory and leaves `PWD` inherited left the record naming
/// the wrong workspace's `[patch]` fork while a change to the fork the binary was
/// compiled with was reported clean. There the record is written as it was before
/// the reading existed, and the crate documentation states the limit.
pub(crate) fn invocation_roots(
    out_dir: &Path,
    run_from: &[PathBuf],
    invoked: Option<&Invoking>,
) -> Invocation {
    let mut roots = Vec::new();
    // `out` under `<package>-<hash>` under `build` is cargo's layout. Anything
    // else is not a build script's output directory and names no build
    // directory to read a root above.
    let shape = out_dir.file_name() == Some(OsStr::new("out"))
        && out_dir.ancestors().nth(2).and_then(Path::file_name) == Some(OsStr::new("build"));
    if shape {
        // The level above the profile is the build directory, or the triple
        // above it; the root sits above each reading.
        if let Some(level) = out_dir.ancestors().nth(3).and_then(Path::parent) {
            let readings = [level.parent(), level.parent().and_then(Path::parent)];
            for root in readings.into_iter().flatten() {
                if declares_workspace(root) {
                    roots.push(canonical(root));
                }
            }
        }
    }
    let mut named = !roots.is_empty();
    // The same directory reaches here twice — once as the shell's `PWD`, once as
    // where the parent process was started — and two spellings of it are one
    // directory, so the queries are asked once per resolved path.
    let mut directories: Vec<PathBuf> = run_from
        .iter()
        .map(|directory| canonical(directory))
        .collect();
    directories.sort();
    directories.dedup();
    let mut run_directory_named = false;
    let mut cargo_unasked = false;
    for directory in &directories {
        match cargo_workspace(directory) {
            Answer::Given(Some(root)) => {
                named = true;
                run_directory_named = true;
                roots.push(root);
            }
            // Cargo ran and named no workspace for this directory.
            Answer::Given(None) => {}
            // The question could not be put, so whether this directory names a
            // workspace is unknown rather than absent, and the root a
            // `package.workspace` name points at cannot be reached by the ancestor
            // scan the answer would have been narrowed to: the record is marked.
            Answer::Unasked => cargo_unasked = true,
        }
    }
    // The workspace the resolution was read in, where the invoking process says
    // which it is: cargo scans up from the directory it was run in unless its
    // command line named a manifest, and then that manifest's workspace is the
    // root — one rule for both, because that is the rule cargo applies (measured
    // above), and a manifest no `[workspace]` manifest owns is its own root
    // ([`resolution_root`]).
    let resolution_named = match invoked {
        Some(invoked) => {
            let directory = invoked
                .manifest
                .as_ref()
                .and_then(|manifest| manifest.parent())
                .map(Path::to_path_buf)
                .unwrap_or_else(|| invoked.directory.clone());
            match resolution_root(&directory) {
                Some(root) => {
                    named = true;
                    roots.push(root);
                    true
                }
                None => false,
            }
        }
        // No reading of the invoking process on this platform: `PWD` names a
        // directory, but not the fact that the resolution was read in it — see
        // the documentation above for that limit.
        None => !READS_INVOKING_PROCESS && run_directory_named,
    };
    if !named {
        panic!(
            "cannot record the sources under test: this build's workspace cannot be named — {} \
             is not inside one, the directory cargo was run in names no workspace (or was not \
             reported in `PWD`), and the command line names no manifest that belongs to one — so \
             the `[patch]` tables the build read are unknown and a record written here could omit \
             the fork it compiled. Run cargo from the workspace, or build with the target \
             directory inside it.",
            out_dir.display()
        );
    }
    roots.sort();
    roots.dedup();
    Invocation {
        roots,
        resolution_named,
        cargo_unasked,
    }
}

/// Whether this platform keeps a process's working directory and command line,
/// which [`invoking_process`] reads. On one that does not, `PWD` is the only
/// reading a build has, and the crate documentation states what that leaves
/// uncovered.
#[cfg(target_os = "linux")]
pub(crate) const READS_INVOKING_PROCESS: bool = true;

/// [`READS_INVOKING_PROCESS`] on a platform that keeps neither.
#[cfg(not(target_os = "linux"))]
pub(crate) const READS_INVOKING_PROCESS: bool = false;

/// The cargo that ran this build, as its own process records it: the directory
/// it was started in and the manifest its command line told it to build.
///
/// `PWD` is the shell's record of where cargo was started, and cargo passes it
/// through rather than setting it, so it can name a directory cargo never
/// resolved in: measured, a launcher that sets cargo's working directory and
/// leaves `PWD` inherited — `PWD` naming one workspace while cargo ran in
/// another — leaves the record naming the first workspace's `[patch]` fork and
/// missing the one the binary was compiled with, which the guard then reported
/// clean. The process that spawned this build script *is* that cargo (measured:
/// `/proc/<parent>/exe` is `env!("CARGO")` and `/proc/<parent>/cwd` is the
/// directory cargo was run in, in a default build, a `--target` build, one with
/// a redirected target directory, and one where `PWD` was absent), so where the
/// platform keeps a process's working directory it is the fact `PWD` is meant to
/// carry.
///
/// Its command line carries the other half of the same fact: cargo resolves the
/// build in the workspace of the manifest `--manifest-path` named, which need not
/// be the workspace of the directory it was run in. Measured, the build script of
/// a package a sibling workspace builds sees
/// `argv=["…/bin/cargo", "build", "--offline", "--manifest-path",
/// "../sibling/member/Cargo.toml"]` with `cwd` the third workspace. Read from
/// `/proc` and only where the parent's executable is a cargo, which is what makes
/// it an answer rather than a guess; everywhere else this reading does not exist
/// and `PWD` is all there is.
pub(crate) struct Invoking {
    /// The directory cargo was run in.
    pub(crate) directory: PathBuf,
    /// The manifest the command line named, resolved against that directory,
    /// where it named one.
    pub(crate) manifest: Option<PathBuf>,
}

#[cfg(target_os = "linux")]
pub(crate) fn invoking_process() -> Option<Invoking> {
    let parent = std::os::unix::process::parent_id();
    let cargo = std::env::var_os("CARGO").map(PathBuf::from)?;
    let executable = std::fs::read_link(format!("/proc/{parent}/exe")).ok()?;
    let same_program = executable
        .file_name()
        .zip(cargo.file_name())
        .is_some_and(|(parent, cargo)| parent == cargo);
    if !same_program {
        return None;
    }
    let directory = std::fs::read_link(format!("/proc/{parent}/cwd"))
        .ok()
        .filter(|directory| directory.is_dir())?;
    let manifest = std::fs::read(format!("/proc/{parent}/cmdline"))
        .ok()
        .and_then(|line| {
            let arguments: Vec<String> = line
                .split(|byte| *byte == 0)
                .filter(|argument| !argument.is_empty())
                .map(|argument| String::from_utf8_lossy(argument).into_owned())
                .collect();
            manifest_path(&arguments, &directory)
        });
    Some(Invoking {
        directory,
        manifest,
    })
}

/// [`invoking_process`], on a platform that keeps no such answer: `PWD` is all
/// there is, so nothing here is read and the record's mark rests on `PWD` alone.
#[cfg(not(target_os = "linux"))]
pub(crate) fn invoking_process() -> Option<Invoking> {
    None
}

/// The manifest the cargo command line names, resolved against the directory
/// cargo was run in: `--manifest-path <path>` or `--manifest-path=<path>`, both
/// of which cargo builds the same workspace for — in the command line itself, or
/// in the arguments an alias expands to ([`expanded_arguments`]).
#[cfg(target_os = "linux")]
fn manifest_path(arguments: &[String], directory: &Path) -> Option<PathBuf> {
    let expanded = expanded_arguments(arguments, directory);
    let mut arguments = expanded.iter();
    while let Some(argument) = arguments.next() {
        if argument == "--manifest-path" {
            return arguments
                .next()
                .map(|path| canonical(&directory.join(path)));
        }
        if let Some(path) = argument.strip_prefix("--manifest-path=") {
            return Some(canonical(&directory.join(path)));
        }
    }
    None
}

/// The arguments a cargo command line expands to: the alias cargo's configuration
/// gives its subcommand, followed by the rest of the command line, or the command
/// line itself where it names no alias.
///
/// A cargo alias replaces the subcommand, so a build reached through one carries
/// no manifest on its own command line — measured, `[alias] b = "build
/// --manifest-path ../sibling/member/Cargo.toml"` runs the build from a parent
/// whose command line reads `["…/bin/cargo", "b", "--offline"]` while that
/// manifest's workspace is what the resolution was read in, so a reading that
/// stops at the command line names the wrong root. The expansion is read where
/// cargo reads it ([`configuration_alias`]).
#[cfg(target_os = "linux")]
fn expanded_arguments(arguments: &[String], directory: &Path) -> Vec<String> {
    let Some(index) = subcommand_index(arguments) else {
        return arguments.to_vec();
    };
    let Some(expansion) = configuration_alias(arguments, index, &arguments[index], directory)
    else {
        return arguments.to_vec();
    };
    let mut expanded = expansion;
    expanded.extend(arguments.iter().skip(index + 1).cloned());
    expanded
}

/// The index of the subcommand in a cargo command line: the executable at 0,
/// then the options before it — `--color` and `--config` take a value, and a
/// `+toolchain` argument is a value of its own rather than an option with one —
/// until the first argument that is neither.
///
/// `+toolchain` is the rustup shim's directive, and the cargo whose command line
/// this reads is the one the shim runs: measured, `cargo +stable build` spawns a
/// build script's parent with `argv=["…/bin/cargo", "build"]`, the directive
/// consumed before it, and that toolchain's own cargo refuses the directive
/// outright (`error: no such command: '+stable'`, whose help says to invoke
/// cargo through rustup). A line that carries one is still read as one argument,
/// because reading it as an option that takes a value reads the subcommand as
/// that value: `cargo +nightly b --offline`, where `b` is an alias whose
/// expansion names a manifest, would then name no subcommand at all — no
/// expansion is read, the line names no manifest, and the record names the run
/// directory's workspace while the fork the alias made the build compile goes
/// unnamed and a change to it is reported clean.
#[cfg(target_os = "linux")]
fn subcommand_index(arguments: &[String]) -> Option<usize> {
    let mut index = 1;
    while let Some(argument) = arguments.get(index) {
        if argument == "--color" || argument == "--config" {
            index += 2;
            continue;
        }
        if argument.starts_with('-') || argument.starts_with('+') {
            index += 1;
            continue;
        }
        return Some(index);
    }
    None
}

/// The expansion cargo's configuration gives `name`, where it gives one: a
/// `--config` value among the arguments before the subcommand first, because
/// cargo applies those after every file, then the configuration files of
/// `directory` and its ancestors, nearest first, then beside the registry cache.
#[cfg(target_os = "linux")]
fn configuration_alias(
    arguments: &[String],
    subcommand: usize,
    name: &str,
    directory: &Path,
) -> Option<Vec<String>> {
    let mut index = 1;
    while index < subcommand {
        let argument = &arguments[index];
        let value = if argument == "--config" {
            index += 1;
            arguments.get(index).map(String::as_str)
        } else {
            argument.strip_prefix("--config=")
        };
        if let Some(expansion) = value.and_then(|value| configured_alias(value, name)) {
            return Some(expansion);
        }
        index += 1;
    }
    for file in configuration_candidates(directory) {
        if let Ok(text) = std::fs::read_to_string(file) {
            if let Some(expansion) = manifest::alias(&text, name) {
                return Some(expansion);
            }
        }
    }
    None
}

/// The alias a `--config` value states: `alias.<name> = <value>`, the value read
/// the way the same declaration in a file is.
#[cfg(target_os = "linux")]
fn configured_alias(value: &str, name: &str) -> Option<Vec<String>> {
    let (key, declared) = value.split_once('=')?;
    if key.trim() != format!("alias.{name}") {
        return None;
    }
    manifest::alias(&format!("[alias]\n{name} = {declared}\n"), name)
}

/// The configuration files cargo reads for a build invoked in `directory`, in the
/// order cargo applies them: the one under `.cargo` in the directory and in each
/// ancestor, nearest first, then beside the registry cache.
#[cfg(target_os = "linux")]
fn configuration_candidates(directory: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut candidate = Some(canonical(directory));
    while let Some(directory) = candidate {
        files.push(directory.join(".cargo/config.toml"));
        files.push(directory.join(".cargo/config"));
        candidate = directory.parent().map(Path::to_path_buf);
    }
    if let Some(cache) = std::env::var_os("CARGO_HOME") {
        files.push(PathBuf::from(&cache).join("config.toml"));
        files.push(PathBuf::from(&cache).join("config"));
    }
    files
}

/// The workspace root of the directory cargo was run in, as cargo answers it:
/// measured, `cargo locate-project --workspace` run there reports the root cargo
/// would load for a build started in that directory, following a
/// `package.workspace` name an ancestor `[workspace]` manifest would otherwise
/// hide. A non-zero exit is an answer too — `cargo locate-project` fails in a
/// directory no workspace contains — and the ancestor scan is read then, because
/// it can only add a candidate.
///
/// [`Answer::Unasked`] where cargo could not be run at all, which the ancestor
/// scan does **not** stand in for: measured, a root a `package.workspace` key
/// names is invisible to that scan, so a fallback that looks like an answer
/// would let a record miss the fork the binary compiled. The caller marks the
/// record instead.
fn cargo_workspace(directory: &Path) -> Answer<Option<PathBuf>> {
    let args = ["locate-project", "--workspace", "--message-format", "plain"];
    match asked_cargo(directory, &args) {
        Answer::Given(output) => Answer::Given(
            output
                .status
                .success()
                .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
                .and_then(|manifest| Path::new(&manifest).parent().map(canonical))
                .filter(|root| declares_workspace(root))
                .or_else(|| declaring_roots(directory).into_iter().next()),
        ),
        Answer::Unasked => Answer::Unasked,
    }
}

#[cfg(test)]
mod tests;
