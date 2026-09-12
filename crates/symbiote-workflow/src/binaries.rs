//! Test support: the daemon binary the end-to-end proofs drive.
//!
//! Those proofs assert behavior the binary carries — for `symbioted`, the
//! sandbox mount policy through `symbiote-host`/`symbiote-sandbox` — so a
//! binary that does not contain the sources under test would let a proof stay
//! green while running other code. [`daemon_binary`] refuses such a binary
//! with the rebuild that fixes it, rather than running it. Gated behind the
//! `test-support` feature, so a production build does not carry it.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::SystemTime;

use serde_json::Value;

/// The workspace-built `symbioted`, or a refusal naming what outdates it.
pub fn daemon_binary() -> PathBuf {
    static RESOLVED: OnceLock<PathBuf> = OnceLock::new();
    RESOLVED.get_or_init(resolve).clone()
}

fn resolve() -> PathBuf {
    let metadata = metadata();
    let binary = path(&metadata["target_directory"]).join("debug/symbioted");
    let stale = if binary.is_file() {
        stale_source(
            &daemon_sources(&metadata, &path(&metadata["workspace_root"])),
            &binary,
        )
        .map(|source| format!("{} changed after it was built", source.display()))
    } else {
        Some("it has not been built in this tree".to_owned())
    };
    if let Some(reason) = stale {
        panic!(
            "symbioted at {} is not current for the sources under test: {reason}. Rebuild it from \
             sources — `cargo build --locked -p symbiote-host --bin symbioted` (or run \
             `cargo test --workspace --locked`) — so this proof exercises the current mount \
             policy instead of an old binary.",
            binary.display()
        );
    }
    binary
}

/// Cargo's own resolved view of the workspace. The daemon's sources are then
/// cargo's definition of them rather than this guard's guess: `cargo test`
/// builds `target/debug/symbioted` **without** writing its dep-info
/// (`target/debug/symbioted.d`), so a guard that reads that file refuses the
/// healthy tree CI has.
fn metadata() -> Value {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned());
    let output = std::process::Command::new(cargo)
        .args(["metadata", "--locked", "--format-version", "1"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap_or_else(|error| panic!("cannot run cargo metadata: {error}"));
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "cannot resolve the sources `symbioted` is built from, so no proof can show the \
             binary reflects them — `cargo metadata --locked` exited with {} ({error}):\n{}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

/// The workspace-local sources of every package `symbiote-host` reaches
/// through its normal and build dependency edges — exactly what rebuilding the
/// daemon would consume from this tree. Dev/test edges are excluded, and so are
/// registry sources: neither is compiled into the binary, so a change there
/// must not refuse a daemon that is current — a refusal nothing but touching the
/// binary could clear.
fn daemon_sources(metadata: &Value, workspace: &Path) -> Vec<PathBuf> {
    let packages = array(&metadata["packages"]);
    let mut dependencies = std::collections::BTreeMap::new();
    for node in array(&metadata["resolve"]["nodes"]) {
        dependencies.insert(
            text(&node["id"]).to_owned(),
            array(&node["deps"])
                .iter()
                .filter(|dependency| {
                    let kinds = array(&dependency["dep_kinds"]);
                    kinds.is_empty() || kinds.iter().any(|kind| text(&kind["kind"]) != "dev")
                })
                .map(|dependency| text(&dependency["pkg"]).to_owned())
                .collect::<Vec<_>>(),
        );
    }
    let host = packages
        .iter()
        .find(|package| text(&package["name"]) == "symbiote-host")
        .expect("the workspace's symbiote-host package");
    let (mut pending, mut reached, mut sources) = (
        vec![text(&host["id"]).to_owned()],
        BTreeSet::new(),
        Vec::new(),
    );
    while let Some(package) = pending.pop() {
        if !reached.insert(package.clone()) {
            continue;
        }
        pending.extend(dependencies.get(&package).into_iter().flatten().cloned());
        let Some(manifest) = packages
            .iter()
            .find(|candidate| text(&candidate["id"]) == package)
            .map(|candidate| path(&candidate["manifest_path"]))
            .and_then(|manifest| manifest.parent().map(Path::to_path_buf))
        else {
            continue;
        };
        if manifest.starts_with(workspace) {
            sources.extend(source_files(&manifest));
        }
    }
    sources
}

/// Every source file under one workspace crate that its library and binaries
/// are built from: the `.rs` files cargo would compile and the manifests that
/// shape them. Test, example and bench targets are skipped because
/// `cargo build --locked -p symbiote-host --bin symbioted` does not compile
/// them, so their timestamps say nothing about the daemon.
fn source_files(crate_directory: &Path) -> Vec<PathBuf> {
    let mut directories = vec![crate_directory.to_path_buf()];
    let mut sources = Vec::new();
    while let Some(directory) = directories.pop() {
        for entry in std::fs::read_dir(&directory)
            .into_iter()
            .flatten()
            .flatten()
        {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            match entry.file_type() {
                Ok(kind) if kind.is_dir() => {
                    if !matches!(
                        name.as_ref(),
                        "target"
                            | "node_modules"
                            | "dist"
                            | ".git"
                            | "tests"
                            | "examples"
                            | "benches"
                    ) {
                        directories.push(entry.path());
                    }
                }
                _ if name.ends_with(".rs") || name == "Cargo.toml" => sources.push(entry.path()),
                _ => {}
            }
        }
    }
    sources
}

/// The newest source newer than `binary`, if any: one the binary was not
/// built from, and so a daemon that no longer reflects the tree under test.
fn stale_source(sources: &[PathBuf], binary: &Path) -> Option<PathBuf> {
    let built = binary.metadata().ok()?.modified().ok()?;
    sources
        .iter()
        .filter_map(|source| Some((source.metadata().ok()?.modified().ok()?, source)))
        .filter(|(modified, _)| *modified > built)
        .max_by_key(|(modified, _): &(SystemTime, &PathBuf)| *modified)
        .map(|(_, source)| source.clone())
}

fn array(value: &Value) -> Vec<&Value> {
    value
        .as_array()
        .map(|array| array.iter().collect())
        .unwrap_or_default()
}

fn text(value: &Value) -> &str {
    value.as_str().unwrap_or_default()
}

fn path(value: &Value) -> PathBuf {
    PathBuf::from(text(value))
}
