//! Test support: the workspace-built binaries the end-to-end proofs drive.
//!
//! Those proofs assert behavior the binaries carry — for `symbioted`, the
//! sandbox mount policy through `symbiote-host`/`symbiote-sandbox`; for
//! `symbiote-sandbox-launch`, the descriptor boundary every sandboxed process
//! is started through — so a binary that does not contain the sources under
//! test would let a proof stay green while running other code. The resolvers
//! here build a binary that is absent and refuse one whose recorded source
//! content no longer matches the tree, naming the rebuild that fixes it. The
//! record is written by the binary's own build (`symbiote-source-stamp`), so
//! the check is about content rather than timestamps: a file restored with an
//! older timestamp still refuses, and a file only touched does not. Gated
//! behind the `test-support` feature, so a production build does not carry
//! them.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// The workspace-built `symbioted`, or a refusal naming what outdates it.
pub fn daemon_binary() -> PathBuf {
    static RESOLVED: OnceLock<PathBuf> = OnceLock::new();
    RESOLVED
        .get_or_init(|| binary("symbiote-host", "symbioted", "mount policy"))
        .clone()
}

/// The workspace-built `symbiote-sandbox-launch`, or a refusal naming what
/// outdates it. The launcher is the boundary a sandboxed process is started
/// through, so a stale copy would be the boundary under test rather than the
/// one the sources describe.
pub fn launcher_binary() -> PathBuf {
    static RESOLVED: OnceLock<PathBuf> = OnceLock::new();
    RESOLVED
        .get_or_init(|| {
            binary(
                "symbiote-sandbox",
                "symbiote-sandbox-launch",
                "descriptor boundary",
            )
        })
        .clone()
}

/// `name` built from `package`, resolved once per test process: built when it
/// is absent, refused when the content it was built from is no longer the
/// content of the tree.
fn binary(package: &str, name: &str, subject: &str) -> PathBuf {
    let binary = profile_directory().join(name);
    if !binary.is_file() {
        build(package, name);
    }
    let rebuild = format!(
        "`cargo build --locked -p {package} --bin {name}` (or run \
         `cargo test --workspace --locked`)"
    );
    let problem = match symbiote_source_stamp::changed_sources(
        &binary,
        package,
        Path::new(env!("CARGO_MANIFEST_DIR")),
    ) {
        Ok(changed) if changed.is_empty() => None,
        Ok(changed) => Some(format!(
            "the source content it was built from is not the content of the tree: {}",
            listing(&changed)
        )),
        Err(problem) => Some(problem),
    };
    if let Some(problem) = problem {
        panic!(
            "{name} at {} cannot be shown to reflect the sources under test — {problem}. \
             Rebuild it from sources — {rebuild} — so this proof exercises the current \
             {subject} instead of an old binary.",
            binary.display()
        );
    }
    binary
}

/// The directory cargo writes this test's binaries to: the profile directory
/// two levels above the test executable (`<target>/<profile>/deps/<test>`).
fn profile_directory() -> PathBuf {
    std::env::current_exe()
        .expect("the test executable's path")
        .parent()
        .and_then(Path::parent)
        .expect("the profile directory above the test executable")
        .to_path_buf()
}

/// The changed files, named: a refusal that does not say which file outdates
/// the binary leaves the reader to guess at the rebuild.
fn listing(changed: &[PathBuf]) -> String {
    let named = changed
        .iter()
        .take(3)
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>()
        .join(", ");
    match changed.len().saturating_sub(3) {
        0 => named,
        more => format!("{named} (and {more} more)"),
    }
}

/// Builds one binary, so a partial `-p <crate>` build still runs the proofs
/// instead of failing for want of a binary the workspace gauntlet would have
/// produced.
fn build(package: &str, name: &str) {
    let status =
        std::process::Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned()))
            .args(["build", "--locked", "-p", package, "--bin", name])
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .status()
            .unwrap_or_else(|error| panic!("cannot build {name}: {error}"));
    assert!(
        status.success(),
        "`cargo build --locked -p {package} --bin {name}` failed; the proof cannot run against \
         the current sources"
    );
}
