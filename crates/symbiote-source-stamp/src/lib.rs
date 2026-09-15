//! A build-time record of the sources a workspace binary was compiled from,
//! carried by that binary, and the check that the binary still matches them.
//!
//! The end-to-end proofs drive workspace binaries, so a binary that does not
//! contain the sources under test would let a proof stay green while running
//! other code. Comparing timestamps answers "was something touched after the
//! build", which misses a real change whose timestamp is older than the binary
//! (a restored file, a checkout with preserved times, a clock skew) and
//! refuses a binary whose sources were only touched.
//!
//! This crate records the **content** instead and puts the record in the
//! binary, so no record can be read that describes a build other than the one
//! that produced the binary:
//!
//! * [`build_stamp`] is called from a `build.rs`. For the package and every
//!   workspace package it reaches through normal and build dependency edges
//!   (read from the dependency and `[patch]` tables of the manifest, in
//!   whichever of the spellings a manifest may give an edge, and never from a
//!   comment) it records every file the package's binaries compile — everything
//!   under the
//!   package that is not a test, example or bench target, whatever its
//!   extension, because a compile can read a file no manifest mentions
//!   (`include_str!("schema.sql")`) — together with the manifests and build
//!   scripts that name them and the workspace manifest and `Cargo.lock` that
//!   configure and pin each package's build, a path dependency in another
//!   workspace bringing that workspace's root manifest as well, and every file
//!   those sources pull
//!   in through an `include!`, `include_bytes!` or `include_str!` or a
//!   `#[path = "…"]` module attribute, wherever it lives:
//!   `symbiote-workflow` compiles fixtures kept at the workspace root.
//!   It hashes the content of all of them, tells cargo to rerun the build
//!   script when any of them changes, and writes the record into `OUT_DIR` as
//!   a `static`, which the binary includes so the record travels inside the
//!   binary itself.
//! * [`changed_sources`] is called by the proof. It reads the record out of
//!   the binary's own bytes, re-reads every input the record holds, and
//!   refuses a binary that carries no record — one built without the stamp —
//!   naming every input whose content differs. An input is a file, or a value
//!   of the build's environment ([`Input`]).
//!
//! What a compile produces depends on more than the source files, so the
//! record holds the other two kinds of input that can change it while every
//! file stays byte-identical: the configuration files cargo reads for a build
//! of any closure package — `.cargo/config.toml`, `.cargo/config`,
//! `rust-toolchain.toml`, `rust-toolchain`, in each package directory and
//! workspace root and every directory above them, plus the ones beside the
//! registry cache — and the one environment variable both a build script and a
//! proof see alike, the wire's `environment` (the cargo that ran the build),
//! which names the toolchain. The flags a build was given are covered through
//! the files that carry them rather than through `RUSTFLAGS`, which the two
//! sides do not see alike; the wire's own documentation says what is left
//! outside and why.
//!
//! What the walk covers is stated rather than claimed whole. Test, example and
//! bench targets are outside it because nothing a binary compiles comes from
//! them, so a change there must not refuse a current binary. Registry
//! dependencies are outside it because `Cargo.lock`, which is inside it, pins
//! them, and dev-dependency edges are outside it because a binary compiles
//! none of them. A dependency a member inherits from a workspace root
//! (`dep.workspace = true`) is inside it, although the member names no path: the
//! path is read from every root cargo could have resolved it in, because which
//! root that was is not visible from the tree, and an inherited path that is
//! not there is skipped, because a root cargo read must hold the directory or
//! the build stops before a record is written while a root it never read may
//! name one that is gone. One case the manifest itself settles: a package that
//! names its workspace (`workspace = "../root"`) makes that root the only root a
//! build of it can read — measured, cargo resolves such a package to the named
//! root even where an ancestor manifest declares `[workspace]`, reads an
//! inherited path from the named root and stops when that root does not declare
//! it, and stops with `member of the wrong workspace` for a build that both nests
//! the package and claims it as a member — so the named root's inherited paths
//! and patches are inside the record and an ancestor's are outside it. Where a
//! package names no root, two candidate roots can each patch the same name and
//! each list the member (measured: invoked at the outer one `serde` resolves to
//! the outer root's fork, invoked at the nearer one to the nearer root's fork),
//! and both are inside the record, because which invocation a build was cannot
//! be read from the tree — except through the two things that can say it: the
//! build directory the output was written to, and the directory cargo was run
//! in. A build script's output is written to `<build directory>/[<target
//! triple>/]<profile>/build/<package>-<hash>/out`, and *which* of the two levels
//! above the profile is the build directory is not decidable from that path: `--target`
//! puts the triple between them (measured on the host triple and on a foreign
//! one), while `CARGO_TARGET_DIR=<root>/x86_64-unknown-linux-gnu` with no
//! `--target` (measured) names the host triple and inserts no level at all, so a
//! rule that skipped a level by its name loses the root that is there. Both
//! readings are therefore candidates, each taken where the directory above it
//! declares `[workspace]`, and so is the root cargo answers for the directory it
//! was run in. That root is asked of cargo rather than scanned for here:
//! measured, `cargo locate-project --workspace` run in a package that names a
//! root (`workspace = "../root"`) answers that root even where an ancestor
//! manifest declares `[workspace]`. A package no `[workspace]` manifest owns is
//! its own root, which is cargo's answer for it too — `cargo locate-project
//! --workspace` in it names its own manifest and `cargo metadata` reports that
//! directory as `workspace_root` — so a build of it reads the `[patch]` table
//! beside its manifest and records rather than being refused for naming no
//! workspace.
//!
//! Every root a reading names is a candidate, and none is dropped because cargo
//! answers that it does not resolve the package there: measured, that answer is
//! a false negative for a workspace reaching the package through an *optional*
//! dependency the build selected — `cargo tree -p <name>` under the default
//! features answers `package ID specification … did not match any packages`
//! while the build compiles the package and that workspace's own fork of it, so
//! a record narrowed on that answer omits a fork the binary compiled and the
//! guard reports a change to it clean. A candidate the build did not read costs
//! a refusal a rebuild clears, and dropping one costs the guarantee: the union
//! is the side of that trade this record is on.
//!
//! A package
//! another workspace builds is inside the record against that root too, its
//! patches, manifest and lockfile: measured, `cargo build -p <path dependency>`
//! from a sibling workspace builds the package's binary with the sibling's fork,
//! while its build script's environment differs from its own build's in nothing
//! but `OUT_DIR`. The paths such a package inherits come from
//! the root *it* names rather than from the one that built it, because measured,
//! its `dep.workspace = true` resolves to its own root's directory even under
//! that build — and a patch in a dependency's own workspace does not apply to a
//! build that takes it. For such a root the walk follows its manifest rather
//! than its closure, so the record is gated there against cargo: every package
//! `cargo tree -p <name>` resolves in a root the package is not part of — where
//! it resolves the package there at all — must be inside the closure, and one
//! that is not stops the build rather than leaving a record a proof could pass
//! against. That answer is used in that direction only. A `[patch]` key is inside
//! it only from a root manifest, because cargo ignores a member's patch table whole (measured:
//! `patch for the non root package will be ignored`, exit 0, with the dependency
//! resolved from its own path even though the fork is there), so a fork a member
//! patches is outside it — a change there must not refuse a binary no rebuild
//! reads.
//!
//! A patch cargo does not apply is **inside it**, and that is a deliberate
//! limit rather than a claim: whether cargo applies one is not decidable from
//! the manifests this walk reads. Measured on one tree that depends on
//! `serde_json` and patches `serde`, naming no `serde` requirement anywhere: a
//! fork at `1.0.0` leaves the patch unused (`patch `serde v1.0.0 (…)` was not
//! used in the crate graph`) and `serde` comes from the registry, while the same
//! fork at `1.0.229` is compiled instead — the requirement that decides it lives
//! in `serde_json`'s registry manifest, which this crate does not read. So the
//! walk follows a root's patch wherever its directory is there, at the cost of a
//! needless refusal when cargo did not use it.
//!
//! No reading can always name the workspace a build read its patches in, and the
//! record says which it could establish. Cargo's default is the only relation it
//! guarantees — the build directory is the root's own
//! `target` — and every override moves it: measured accepted are
//! `CARGO_TARGET_DIR=../shared` (whose `OUT_DIR` keeps the `..`),
//! `CARGO_TARGET_DIR=/tmp/target`, the same through a symlink, a build directory
//! inside another workspace (`CARGO_TARGET_DIR=<abs>/ws2/target`, run in `ws1`,
//! which names that other workspace and not the one that read its patches), a
//! directory merely named like a triple, `build.target-dir` in
//! `.cargo/config.toml`, `--target-dir`, and `build.build-dir`, which moves the
//! build directory off the target directory altogether. Each of those was
//! measured to lose the sibling root a build read until the directory cargo is
//! run in joined the candidates — there, the binary compiled `ws1`'s fork while
//! the record named `ws2`'s, and a change to `ws1`'s fork was reported clean.
//! Nothing else names the root there: cargo passes no variable naming the
//! workspace root (measured: that build's environment adds `OUT_DIR`, and
//! `CARGO_TARGET_DIR` when the caller set one), and a build script's working
//! directory is its own package rather than the invocation's own.
//!
//! That directory is not enough either, and this is the arrangement the audits
//! kept finding: cargo may have been told which manifest to build, and the
//! resolution is then read in *that* manifest's workspace. Measured: `cd third &&
//! CARGO_TARGET_DIR=<outside every workspace> cargo build --manifest-path
//! ../sibling/member/Cargo.toml` links the sibling's `serde` fork while the run
//! directory names `third` and the build directory names nothing, and the record
//! named `third`'s fork and missed the sibling's — a change to the fork the
//! binary compiled was reported clean. So where the platform keeps the invoking
//! process, its command line is read too and the manifest it names is placed the
//! way cargo places it, and that workspace is a candidate root.
//!
//! A build that no reading places at all stops rather than writing a record it
//! cannot stand behind: measured before the invoking process was read, `env -u PWD
//! CARGO_TARGET_DIR=/tmp/target cargo build -p pkg` run in the sibling recorded
//! that target directory's workspace fork alone and left a change to the fork the
//! binary compiled reported clean. The process's own directory places it now;
//! what still reaches the stop is a build no reading places — its output
//! directory outside every workspace, its run directory naming none, and its
//! command line naming no manifest at all.
//!
//! `PWD` is the shell's record of where cargo was run, which cargo passes through
//! rather than setting — so it can be stale: measured, a launcher that sets
//! cargo's working directory and leaves `PWD` inherited, cargo run in one
//! workspace while `PWD` names another, left the record naming the named
//! workspace's patch fork and missing the one the binary was compiled with, and
//! the guard reported a change to it clean. Two readings replace it where the
//! platform keeps the invoking process: the directory that process was started in
//! (measured: `/proc/<parent>/exe` is `CARGO`, and `/proc/<parent>/cwd` is where
//! cargo was run, in a default build, a `--target` build, one with a redirected
//! target directory, and one where `PWD` was absent) and the manifest its command
//! line names (`walk::invocation::invoking_process` carries the measurements). A build
//! reached through a cargo alias names no manifest there at all, so the
//! configuration cargo applies is read as well: measured, `[alias] b = "build
//! --manifest-path ../sibling/member/Cargo.toml"` runs the build from a parent
//! whose command line reads `["…/bin/cargo", "b", "--offline"]` while the
//! sibling's `[patch]` fork is what links, and the alias is read from the
//! `--config` value or the `.cargo/config.toml` cargo would apply.
//!
//! Where a reading places the build — the directory it was run in, the manifest
//! its command line named, or the manifest an alias expands it to — the record
//! holds every root the tree names, every candidate the readings yield, and the
//! workspace that manifest belongs to: naming an input a build of the closure did
//! not read costs a needless refusal, and the opposite costs the guarantee. Where
//! none does, the record is written marked (`resolution-not-named`) and
//! [`changed_sources`] refuses it rather than reporting it clean, as
//! `planning/integrity/source_record.py` does. A second mark carries the same
//! verdict for a question the build could not put at all (`cargo-not-asked`): the
//! cargo that ran a build is asked the two things only cargo answers — the root a
//! run directory names, and the closure it resolves for this package there — and
//! a `CARGO` naming no program that can be started leaves both unknown, where
//! reading the failure as "nothing here" records a root list and a closure
//! nothing checked. A mark this crate no longer writes is refused too: a record
//! line is read by the one rule the wire file states (`wire::Line`), and under it
//! an `unset` content under a locator no mark is written with is a mark rather
//! than a file the record says it read.
//! Measured cost, so that a maintainer meets it here: a marked record declares the
//! variable whose change would clear its mark as a rerun input (`PWD` for the
//! resolution, `CARGO` for the cargo), so the rebuild the refusal asks for stamps
//! again, which measured it otherwise did not (the mark survived a `Finished in
//! 0.01s` rebuild), where an ordinary build declares nothing beyond its inputs.
//!
//! A platform that keeps neither the directory nor the command line of the
//! process that ran the build (this crate reads `/proc`, so Linux) has `PWD`
//! alone, and cannot tell a stale one from a correct one: the stale-`PWD` and
//! `--manifest-path` arrangements above are recorded unmarked there. That is a
//! stated limit of the platform rather than one this walk can close.
//!
//! A configuration file that did not exist when the binary was
//! built and appears afterwards is outside it: cargo reads no such file at
//! build time, and naming the absent location as a build-script input would
//! list a missing path, which makes cargo recompile every stamped crate on
//! every build (measured: `Dirty symbiote-host: the file ... is missing`, and
//! the binary relinked each time). A `rust-toolchain.toml` that appears is
//! still caught, because the toolchain it selects is named by the recorded
//! cargo. Files a package generates into its `OUT_DIR` are outside it
//! too: their content comes from the build script, which is inside it. The
//! include scan reads code, not text — a path named in a comment or a string
//! literal is not an input — and follows a literal path and a `concat!` of
//! literals and `env!("CARGO_MANIFEST_DIR")`, which is how a package names an
//! input it does not keep beside itself. It follows those to a fixed point
//! over Rust sources, so a non-Rust file pulled in by `include!` is recorded
//! but not itself scanned for includes. A `#[path = "…"]` module attribute is
//! followed the same way when it sits on a `mod name;` declaration, resolved
//! against the directory the compiler looks in: the file's own directory at the
//! top level of a file, and a directory inside it when the declaration sits in
//! an inline module block. A file does not say whether the compiler found it as
//! `mod name;` — which puts its modules under a directory of its own name — or
//! named it through a `#[path]`, as a crate root or as a `mod.rs`, which do not,
//! so inside an inline module the record keeps both directories rather than
//! choose: a record that is long is a rebuild, and a record that is short is
//! the failure this crate exists to prevent. A path built any other way, and a
//! `#[path]` on an inline module, is **not skipped**: the build stops and names
//! it.
//!
//! Cargo's dep-info is not read here because it is a private, versioned binary
//! format that is written *after* the build script that must write the record,
//! so it can neither populate a record on a first build nor be a completeness
//! check the documented rebuild could ever clear.
//!
//! ## Structure
//!
//! * `manifest` reads the facts the walk needs out of a manifest, as text: the
//!   dependency and `[patch]` edges, in whichever spelling a manifest gives
//!   one, the dependencies a member inherits from a workspace root, and the
//!   workspace a manifest declares or a package names for itself.
//! * `scan` reads the inputs one source pulls in — include macros and `#[path]`
//!   modules — with the tokenizer that tells code from text.
//! * `walk` owns what a binary's build covers: the package closure — a
//!   dependency a manifest inherits by name and the `[patch]` replacements a root
//!   declares included — the files under each package, the workspace roots a
//!   record's locators are spelled against, the workspaces a build could have
//!   been invoked in (asked of cargo, whose questions report that they could not
//!   be put rather than answering "nothing here"),
//!   and the configuration cargo reads for the same packages.
//! * `wire` owns the bytes a record is spelled with, what a line of one is, and
//!   the marks a build writes where it could not stand behind one, read from one
//!   file that `planning/integrity/source_record.py` reads too, so the crate and
//!   the check cannot spell the same record differently.
//! * this module owns the record itself: [`build_stamp`] turns the walk's
//!   inputs into the `static` a binary carries, and [`changed_sources`] reads
//!   that record back out of a binary and compares it with the tree.
//! * `tests` holds the record's own tests and the fixture helpers the others
//!   share; `walk`, `scan`, `manifest` and `wire` each keep theirs beside them
//!   (`walk::tests`, `scan::tests`, `manifest::tests`, `wire::tests`).
//!
//! Data flows one way — `walk` asks `manifest` for edges and `scan` for what a
//! source pulls in, and this module writes and checks the record — so a reader
//! can follow a single input from the manifest that names it to the refusal it
//! causes without reading the other four.

mod manifest;
mod scan;
mod walk;
mod wire;

#[cfg(test)]
pub(crate) mod tests;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use walk::invocation::{invocation_roots, invoking_process};
use walk::paths::relative_to;
use walk::recorded_sources;
use walk::roots::workspace_root;
use wire::{Line, embedded_record, record_lines, wire};

/// The file `build_stamp` writes into `OUT_DIR`, for the binary to `include!`.
pub const RECORD_FILE: &str = "source_record.rs";

/// The `static` that file defines: the record, between the wire's own markers.
pub const RECORD_STATIC: &str = "SOURCE_RECORD";

/// One input a record holds, as [`changed_sources`] reports it: a file the
/// build compiled, or a value of the environment it compiled under.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Input {
    /// A file, named as the record names it: relative to the workspace, or
    /// absolute when it lives outside it.
    File(PathBuf),
    /// An environment variable, named without the `env:` prefix the record
    /// spells it with.
    Environment(String),
}

impl std::fmt::Display for Input {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Input::File(path) => write!(formatter, "{}", path.display()),
            Input::Environment(name) => write!(formatter, "{}{name}", wire().prefix),
        }
    }
}

/// Records the content of the inputs this package's binaries are built from —
/// its sources, the files they compile, the manifests and configuration that
/// describe and shape the build, and the environment it runs under — as a
/// `static` in `OUT_DIR`, for the binary to include, and asks cargo to rerun
/// the build script when any of them changes. Call this from a build script's
/// `main`.
///
/// Where the readings cannot establish the workspace cargo resolved in, or the
/// cargo that is running this build cannot be asked the questions the record
/// rests on, the record is written marked (`resolution-not-named`,
/// `cargo-not-asked`) and [`changed_sources`] refuses it: a record that may be
/// missing the `[patch]` fork the binary compiled cannot be stood behind.
pub fn build_stamp() {
    let manifest_dir = PathBuf::from(environment("CARGO_MANIFEST_DIR"));
    let out_dir = PathBuf::from(environment("OUT_DIR"));
    let wire = wire();
    let workspace = workspace_root(&manifest_dir)
        .unwrap_or_else(|problem| panic!("cannot record the sources under test: {problem}"));

    // The cargo that ran this build, where the platform keeps it: the directory it
    // was started in and the manifest its command line told it to build. `PWD` is
    // the shell's record of the same directory and cargo passes it through rather
    // than setting it — so it can be stale, measured: a launcher that sets cargo's
    // working directory and leaves `PWD` inherited leaves `PWD` naming a workspace
    // the build never resolved in. Neither is trusted over the other: every root
    // either names is a candidate, and the workspace the resolution was read in is
    // established from the process where the process can be read.
    let invoked = invoking_process();
    let run_from: Vec<PathBuf> = std::env::var_os("PWD")
        .map(PathBuf::from)
        .into_iter()
        .chain(invoked.as_ref().map(|invoked| invoked.directory.clone()))
        .collect();
    let invocation = invocation_roots(&out_dir, &run_from, invoked.as_ref());
    let (sources, unfollowed, unasked_cargo) =
        recorded_sources(&manifest_dir, &workspace, &invocation.roots);
    let unasked_cargo = unasked_cargo || invocation.cargo_unasked;
    if let Some(problem) = unfollowed.first() {
        panic!(
            "cannot record the sources under test: {problem}. A record that cannot name every \
             input a binary compiles would let a proof pass against a binary built from other \
             sources, so this build stops here."
        );
    }

    let mut record = String::new();
    for source in &sources {
        let content = std::fs::read(source)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", source.display()));
        let locator = relative_to(&workspace, source).display().to_string();
        // No locator a record cannot read back is one a build stamps: the rule
        // the file states makes a line end at each LF, reads a CR in one as
        // malformed, and frames the record between two markers its own bytes
        // cannot hold — so a path holding any of them would be stamped into a
        // record neither reader could read, in a remedy asking for a rebuild that
        // reproduces it. Measured, both a path holding a line feed and one legally
        // named after the end marker built successfully and left an artifact both
        // readers refused. The walk found the file, so the build stops here naming
        // it rather than record it.
        if let Some(holds) = wire.unholdable(&locator) {
            panic!(
                "cannot record the sources under test: the path {locator} holds {holds}, so a \
                 record naming it could not be read back. Rename it."
            );
        }
        record.push_str(&format!("{:x}\t{locator}\n", Sha256::digest(&content)));
        println!("cargo:rerun-if-changed={}", source.display());
    }
    // The environment this compile runs under, held as an input of its own:
    // the cargo that ran it, which names the toolchain. Cargo re-runs this
    // build script when it changes, so the record describes the toolchain in
    // use rather than the one that built it first.
    let name = wire.environment;
    record.push_str(&format!(
        "{}\t{}{name}\n",
        environment_content(name),
        wire.prefix
    ));
    println!("cargo:rerun-if-env-changed={name}");

    // Each fact the readings left unestablished, held as a line of its own:
    // rather than record a list that may be missing its `[patch]` fork and let a
    // proof pass against it, the record says so and the check refuses it. The
    // locator is recognised by shape rather than by name, so an older spelling of
    // a mark is refused too.
    for (role, rerun) in [
        (!invocation.resolution_named).then_some(("unnamed-resolution", "PWD")),
        unasked_cargo.then_some(("unasked-cargo", "CARGO")),
    ]
    .into_iter()
    .flatten()
    {
        record.push_str(&wire.mark_line(role));
        // And the rebuild each refusal asks for has to stamp again: measured,
        // cargo watches no environment variable this script does not declare, so
        // without this a record marked by a build that reported no `PWD` keeps its
        // mark through the very rebuild that would clear it — and the same holds
        // for the cargo a build was run with, whose replacement is what makes an
        // unaskable one askable.
        println!("cargo:rerun-if-env-changed={rerun}");
    }

    let generated = out_dir.join(RECORD_FILE);
    std::fs::write(&generated, record_static(&record)).unwrap_or_else(|error| {
        panic!(
            "cannot write the source record at {}: {error}",
            generated.display()
        )
    });
}

/// The record as the Rust source of the `static` a binary includes. The record
/// is spelled as a string literal by `Debug`, which is defined to be a literal
/// with the same value, so a path holding a quote or a backslash survives.
fn record_static(record: &str) -> String {
    format!(
        "// Generated by symbiote-source-stamp: the content this binary was compiled from.\n\
         #[doc(hidden)]\n\
         static {RECORD_STATIC}: &str = {:?};\n",
        format!("{}{record}{}", wire().start, wire().end)
    )
}

/// Every file whose recorded content no longer matches what is on disk, or an
/// error naming why the binary at `binary` cannot be checked at all. An empty
/// list means the binary carries a record of exactly the content the tree
/// holds.
///
/// A record a build marked — where the workspace cargo resolved in could not be
/// established, or where the cargo that ran it could not be asked the questions
/// the record rests on — is one of those errors rather than a list: it may be
/// missing the `[patch]` fork the binary compiled, so no tree can be shown to
/// match it. `read_record` reads it by the rule `wire.txt` states, and every
/// refusal here prints the text that file holds for what was found there: the
/// remedy beside the mark the record carries, the file's remedy for a mark it
/// does not name, or its remedy for a line it cannot read at all — and a record
/// the wire cannot read is refused through its remedy for that, before any line is
/// read: one whose bytes are not the encoding the file states, or one whose own
/// bytes spell a marker of its own framing.
pub fn changed_sources(binary: &Path, manifest_dir: &Path) -> Result<Vec<Input>, String> {
    let bytes = std::fs::read(binary)
        .map_err(|error| format!("cannot read the binary at {} ({error})", binary.display()))?;
    let record = match embedded_record(&bytes) {
        Ok(Some(record)) => record,
        Ok(None) => {
            return Err(format!(
                "the binary at {} carries no build record, so a build of the sources under test \
                 did not produce it — rebuild it from those sources and check that build",
                binary.display()
            ));
        }
        // The wire states what the bytes it frames must be — the encoding it
        // decodes by, and no marker of its own framing among them — so a record
        // that breaks either is refused here rather than taken for a binary that
        // carries none or read up to a marker it spells. Measured, an undecodable
        // record was raised over by the checker while this crate reported a binary
        // carrying none, and a marker a record spelled cut the record short in both.
        Err(reason) => {
            return Err(format!(
                "the binary at {} carries a record that cannot be read: {reason}",
                binary.display()
            ));
        }
    };

    let workspace = workspace_root(manifest_dir)?;
    let recorded = read_record(record)?;
    let refused = |locator: &str, remedy: &str| {
        format!(
            "the binary at {} carries a record its own build could not stand behind — the record \
             says so at `{locator}`: {remedy}.",
            binary.display()
        )
    };
    // A mark names what its build could not establish, so the refusal says that
    // and what clears it rather than listing differences the record cannot be
    // trusted to hold. A record can hold more than one, and the wire orders the
    // marks it names, so the first found is the one a rebuild has to clear first.
    if let Some(mark) = wire()
        .marks
        .iter()
        .find(|mark| recorded.named.contains(&mark.locator))
    {
        return Err(refused(mark.locator, mark.remedy));
    }
    // A mark the wire does not name is read by its content — an `unset` under a
    // locator the file names nowhere — which matters because the names change:
    // measured, a record carrying the previous locator
    // (`unset\trun-directory-not-named`) compared that line against a missing
    // file, whose content is `unset` too, matched it, and was reported clean.
    // Where one record carries several, the wire names the order the one reported
    // is read in, and it is the record's own, which `read_record` kept: the
    // inputs, keyed, are sorted, which is how this crate and the Python checker
    // came to describe the same record differently.
    if let Some(locator) = recorded.unknown.first() {
        return Err(refused(locator, wire().unknown_remedy));
    }
    if recorded.inputs.is_empty() {
        return Err(format!(
            "the binary at {} carries an empty build record, which names no input to check — \
             rebuild it from the sources under test",
            binary.display()
        ));
    }
    Ok(recorded
        .inputs
        .iter()
        .filter_map(|(locator, hash)| {
            let (input, content) = input_at(locator, &workspace);
            (content != *hash).then_some(input)
        })
        .collect())
}

/// The input a locator names — with a relative file locator resolved against
/// the workspace — and the content it holds now, spelled as a record spells
/// content: a file's sha256, an environment variable's, or the wire's `unset`
/// when it is not there.
fn input_at(locator: &str, workspace: &Path) -> (Input, String) {
    match locator.strip_prefix(wire().prefix) {
        Some(name) => (
            Input::Environment(name.to_owned()),
            environment_content(name),
        ),
        None => {
            let path = workspace.join(locator);
            let content = content_hash(&path).unwrap_or_else(|| wire().unset.to_owned());
            (Input::File(path), content)
        }
    }
}

/// The content of an environment variable: its value's sha256, or the wire's
/// `unset` when it is not set at all, which is a different fact from set and
/// empty.
fn environment_content(name: &str) -> String {
    std::env::var(name)
        .map(|value| format!("{:x}", Sha256::digest(value.as_bytes())))
        .unwrap_or_else(|_| wire().unset.to_owned())
}

/// What a record says, read by the wire's own rule: the inputs it holds and the
/// marks it carries. A line the wire reads as neither is not here — it is an
/// error from [`read_record`], because a record neither reader can read is not a
/// shorter record.
struct Record {
    /// Every input the record holds: the locator it is spelled with, and the
    /// content the record gives it.
    inputs: BTreeMap<String, String>,
    /// The locators of the wire's marks the record carries, in the order the
    /// record spells them; which of them is reported is the wire's order.
    named: Vec<&'static str>,
    /// The locators of the marks the wire does not name, in the order the record
    /// spells them, which is the order the wire reports one in.
    unknown: Vec<String>,
}

/// The record a build wrote, divided and read line by line by the wire's own rule
/// — the rule the checker also implements, so a line cannot be one thing to this
/// crate and another to the checker. A line the wire cannot read is an error in
/// the wire's own remedy for one, naming the line, rather than a line quietly
/// skipped.
fn read_record(record: &str) -> Result<Record, String> {
    let wire = wire();
    let mut read = Record {
        inputs: BTreeMap::new(),
        named: Vec::new(),
        unknown: Vec::new(),
    };
    for line in record_lines(record) {
        match wire.line(line) {
            Line::Named(locator) => read.named.push(locator),
            Line::Unknown(locator) => read.unknown.push(locator.to_owned()),
            Line::Input(locator, content) => {
                read.inputs.insert(locator.to_owned(), content.to_owned());
            }
            Line::Malformed => {
                return Err(format!("{} The line is {line:?}.", wire.malformed_remedy));
            }
        }
    }
    Ok(read)
}

/// The sha256 of a file's bytes, or `None` when it cannot be read — a file the
/// build consumed and the tree no longer holds is a difference, not an error.
fn content_hash(path: &Path) -> Option<String> {
    std::fs::read(path)
        .ok()
        .map(|content| format!("{:x}", Sha256::digest(content)))
}

fn environment(name: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| panic!("{name} is set when a build script runs"))
}
