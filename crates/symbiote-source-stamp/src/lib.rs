//! A build-time record of the sources a workspace binary was compiled from,
//! and the check that the binary still matches them.
//!
//! The end-to-end proofs drive workspace binaries, so a binary that does not
//! contain the sources under test would let a proof stay green while running
//! other code. Comparing timestamps answers "was something touched after the
//! build", which misses a real change whose timestamp is older than the binary
//! (a restored file, a checkout with preserved times, a clock skew) and
//! refuses a binary whose sources were only touched. This crate records the
//! **content** instead, next to the binary the build produced:
//!
//! * [`build_stamp`] is called from a `build.rs`. It walks the package's
//!   normal and build dependency edges through `path = "..."` manifests,
//!   hashes every `.rs` file they compile plus the manifests that shape them,
//!   the package's own `build.rs` and the workspace lockfile, writes
//!   `<target>/<profile>/<package>.source-stamp`, and asks cargo to rerun when
//!   any of those files changes. The record sits beside the binary rather
//!   than in `OUT_DIR` because the proof knows which binary it is about to
//!   run and nothing else; `OUT_DIR` names a build-script invocation, not the
//!   artifact that will be driven.
//! * [`changed_sources`] is called by the proof. It re-hashes what the stamp
//!   recorded and names every file whose content (or presence) differs, or
//!   reports that the binary carries no stamp to check.
//!
//! The walk states what it covers rather than claiming the whole tree.
//! Registry dependencies are outside it because `Cargo.lock`, which is inside
//! it, pins them. Dev-dependency edges are outside it because nothing a binary
//! compiles comes from them, and so are test, example and bench targets, which
//! the binary's build does not compile. A file a crate consumes without
//! compiling it — a fixture read with `include_str!`, say — is outside it too,
//! and `src/` is walked for `.rs` files only.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

/// What a build record is called: the package it belongs to plus this suffix,
/// in the profile directory that holds the binary.
pub const STAMP_SUFFIX: &str = ".source-stamp";

/// Records the content of the sources this package's binaries are built from,
/// next to the binaries, and asks cargo to rerun the build script when any of
/// them changes. Call this from a build script's `main`.
pub fn build_stamp() {
    let manifest_dir = PathBuf::from(environment("CARGO_MANIFEST_DIR"));
    let out_dir = PathBuf::from(environment("OUT_DIR"));
    let stamp = profile_directory(&out_dir)
        .join(format!("{}{STAMP_SUFFIX}", environment("CARGO_PKG_NAME")));
    let workspace = workspace_root(&manifest_dir)
        .unwrap_or_else(|problem| panic!("cannot record the sources under test: {problem}"));

    let mut record = String::new();
    for source in closure_sources(&manifest_dir, &workspace) {
        let content = std::fs::read(&source)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", source.display()));
        record.push_str(&format!(
            "{:x}\t{}\n",
            Sha256::digest(&content),
            relative_to(&workspace, &source).display()
        ));
        println!("cargo:rerun-if-changed={}", source.display());
    }
    std::fs::write(&stamp, record).unwrap_or_else(|error| {
        panic!(
            "cannot write the source record at {}: {error}",
            stamp.display()
        )
    });
}

/// Every file whose recorded content no longer matches what is on disk, or an
/// error naming why the binary at `binary` cannot be checked at all. An empty
/// list means the binary was built from exactly the content the tree holds.
pub fn changed_sources(
    binary: &Path,
    package: &str,
    manifest_dir: &Path,
) -> Result<Vec<PathBuf>, String> {
    let stamp = binary.with_file_name(format!("{package}{STAMP_SUFFIX}"));
    let recorded = read_stamp(&stamp)?;
    let workspace = workspace_root(manifest_dir)?;
    Ok(recorded
        .into_iter()
        .filter_map(|(path, hash)| {
            let source = if Path::new(&path).is_absolute() {
                PathBuf::from(&path)
            } else {
                workspace.join(&path)
            };
            (content_hash(&source).as_deref() != Some(hash.as_str())).then_some(source)
        })
        .collect())
}

/// The files recorded in a stamp: `(path, sha256)` per line.
fn read_stamp(stamp: &Path) -> Result<Vec<(String, String)>, String> {
    let record = std::fs::read_to_string(stamp)
        .map_err(|error| format!("there is no source record at {} ({error})", stamp.display()))?;
    let mut recorded = Vec::new();
    for line in record.lines().filter(|line| !line.is_empty()) {
        let Some((hash, path)) = line.split_once('\t') else {
            return Err(format!("{} is not a source record", stamp.display()));
        };
        recorded.push((path.to_owned(), hash.to_owned()));
    }
    Ok(recorded)
}

/// The sha256 of a file's bytes, or `None` when it cannot be read — a file the
/// build consumed and the tree no longer holds is a difference, not an error.
fn content_hash(path: &Path) -> Option<String> {
    std::fs::read(path)
        .ok()
        .map(|content| format!("{:x}", Sha256::digest(content)))
}

/// Every source the binaries built from `manifest_dir` consume: the package,
/// every workspace package it reaches through normal and build dependency
/// edges, the lockfile that pins the rest, and nothing else.
fn closure_sources(manifest_dir: &Path, workspace: &Path) -> Vec<PathBuf> {
    let (mut pending, mut reached, mut sources) = (
        vec![manifest_dir.to_path_buf()],
        BTreeSet::new(),
        BTreeSet::new(),
    );
    while let Some(directory) = pending.pop() {
        let directory = canonical(&directory);
        if !reached.insert(directory.clone()) {
            continue;
        }
        sources.extend(package_sources(&directory));
        let manifest = std::fs::read_to_string(directory.join("Cargo.toml"))
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", directory.display()));
        for dependency in path_dependencies(&manifest) {
            pending.push(directory.join(dependency));
        }
    }
    let lockfile = workspace.join("Cargo.lock");
    if lockfile.is_file() {
        sources.insert(lockfile);
    }
    sources.into_iter().collect()
}

/// What one package's binaries compile: the `.rs` files under its `src`, the
/// manifest that declares them, and its build script.
fn package_sources(directory: &Path) -> Vec<PathBuf> {
    let mut sources = Vec::new();
    let mut directories = vec![directory.join("src")];
    while let Some(directory) = directories.pop() {
        for entry in std::fs::read_dir(&directory)
            .into_iter()
            .flatten()
            .flatten()
        {
            let name = entry.file_name().to_string_lossy().to_string();
            match entry.file_type() {
                Ok(kind) if kind.is_dir() => {
                    if !matches!(name.as_str(), "target" | "node_modules" | "dist" | ".git") {
                        directories.push(entry.path());
                    }
                }
                _ if name.ends_with(".rs") => sources.push(entry.path()),
                _ => {}
            }
        }
    }
    for name in ["Cargo.toml", "build.rs"] {
        let file = directory.join(name);
        if file.is_file() {
            sources.push(file);
        }
    }
    sources
}

/// The `path = "..."` values of a manifest's dependency tables. Dev-dependency
/// tables are skipped: a binary never compiles them, so their content says
/// nothing about it and refusing a current binary for one would be a refusal
/// no rebuild could clear.
fn path_dependencies(manifest: &str) -> Vec<String> {
    let mut dependencies = Vec::new();
    let mut table = false;
    for line in manifest.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            table = is_dependency_table(line);
            continue;
        }
        if !table {
            continue;
        }
        if let Some(value) = inline_table_value(line, "path") {
            dependencies.push(value);
        }
    }
    dependencies
}

fn is_dependency_table(header: &str) -> bool {
    let header = header.trim_start_matches('[').trim_end_matches(']');
    if header.contains("dev-dependencies") {
        return false;
    }
    header == "dependencies"
        || header == "build-dependencies"
        || header.ends_with(".dependencies")
        || header.ends_with(".build-dependencies")
}

/// The value of `key` inside a single-line inline table (`{ key = "value" }`).
/// A dependency written across several lines is not read; no manifest here
/// uses that form.
fn inline_table_value(line: &str, key: &str) -> Option<String> {
    let (_, table) = line.split_once('=')?;
    let table = table.trim().strip_prefix('{')?.trim_end_matches('}');
    table.split(',').find_map(|entry| {
        let (name, value) = entry.split_once('=')?;
        (name.trim() == key).then(|| value.trim().trim_matches('"').to_owned())
    })
}

/// The directory holding the workspace's packages: the nearest ancestor
/// manifest that declares `[workspace]`.
fn workspace_root(manifest_dir: &Path) -> Result<PathBuf, String> {
    let mut directory = Some(canonical(manifest_dir));
    while let Some(candidate) = directory {
        let manifest = candidate.join("Cargo.toml");
        if std::fs::read_to_string(&manifest)
            .is_ok_and(|manifest| manifest.lines().any(|line| line.trim() == "[workspace]"))
        {
            return Ok(candidate);
        }
        directory = candidate.parent().map(Path::to_path_buf);
    }
    Err(format!(
        "no [workspace] manifest above {}",
        manifest_dir.display()
    ))
}

/// The directory a build's binaries are written to, from the build script's
/// `OUT_DIR` (`<target>/<profile>/build/<package>-<hash>/out`).
fn profile_directory(out_dir: &Path) -> PathBuf {
    out_dir
        .parent()
        .and_then(Path::parent)
        .and_then(Path::parent)
        .unwrap_or_else(|| panic!("OUT_DIR {} is not a build directory", out_dir.display()))
        .to_path_buf()
}

fn relative_to(workspace: &Path, source: &Path) -> PathBuf {
    source
        .strip_prefix(workspace)
        .map(Path::to_path_buf)
        .unwrap_or_else(|_| source.to_path_buf())
}

fn canonical(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

fn environment(name: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| panic!("{name} is set when a build script runs"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> PathBuf {
        let directory =
            std::env::temp_dir().join(format!("symbiote-stamp-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).expect("a temporary directory");
        directory
    }

    #[test]
    fn dependency_tables_are_read_and_dev_and_target_tables_are_not() {
        let manifest = "\
[package]
name = \"example\"

[dependencies]
symbiote-a = { path = \"../a\" }
serde = { version = \"1\", path = \"../serde\", features = [\"derive\"] }
nix = \"0.30\"

[build-dependencies]
stamp = { path = \"../stamp\" }

[dev-dependencies]
symbiote-test = { path = \"../test\" }

[target.'cfg(unix)'.dependencies]
unix = { path = \"../unix\" }

[target.'cfg(unix)'.dev-dependencies]
unix-test = { path = \"../unix-test\" }

[[bin]]
name = \"example\"
path = \"src/bin/example.rs\"
";
        assert_eq!(
            path_dependencies(manifest),
            ["../a", "../serde", "../stamp", "../unix"]
        );
    }

    #[test]
    fn a_path_dependency_split_across_lines_is_not_claimed() {
        assert!(path_dependencies("[dependencies]\nx = {\n  path = \"../x\"\n}\n").is_empty());
    }

    #[test]
    fn the_workspace_root_is_the_nearest_workspace_manifest() {
        let root = fixture("workspace");
        std::fs::create_dir_all(root.join("crates/inner/src")).expect("crates");
        std::fs::write(
            root.join("Cargo.toml"),
            "[workspace]\nmembers = [\"crates/inner\"]\n",
        )
        .expect("the root manifest");
        std::fs::write(
            root.join("crates/inner/Cargo.toml"),
            "[package]\nname = \"inner\"\n",
        )
        .expect("the package manifest");
        assert_eq!(
            workspace_root(&root.join("crates/inner")).expect("the workspace root"),
            canonical(&root)
        );
    }

    #[test]
    fn a_recorded_source_whose_content_differs_is_named() {
        let root = fixture("changed");
        std::fs::write(root.join("Cargo.toml"), "[workspace]\n").expect("the root manifest");
        let source = root.join("crate/src/lib.rs");
        std::fs::create_dir_all(source.parent().expect("src")).expect("src");
        std::fs::write(&source, "one\n").expect("the source");
        let stamp = root.join("symbiote-example.source-stamp");
        std::fs::write(
            &stamp,
            format!(
                "{:x}\tcrate/src/lib.rs\n",
                Sha256::digest(std::fs::read(&source).expect("the source"))
            ),
        )
        .expect("the stamp");

        let unchanged = changed_sources(&root.join("symbiote-example"), "symbiote-example", &root)
            .expect("a readable record");
        assert!(unchanged.is_empty());

        std::fs::write(&source, "two\n").expect("the changed source");
        assert_eq!(
            changed_sources(&root.join("symbiote-example"), "symbiote-example", &root)
                .expect("a readable record"),
            vec![source.clone()]
        );

        std::fs::remove_file(&source).expect("the removed source");
        assert_eq!(
            changed_sources(&root.join("symbiote-example"), "symbiote-example", &root)
                .expect("a readable record"),
            [root.join("crate/src/lib.rs")]
        );
    }

    #[test]
    fn a_binary_without_a_record_cannot_be_checked() {
        let root = fixture("missing");
        std::fs::write(root.join("Cargo.toml"), "[workspace]\n").expect("the root manifest");
        let problem = changed_sources(&root.join("symbioted"), "symbiote-host", &root)
            .expect_err("no record");
        assert!(problem.contains("no source record"), "{problem}");
    }

    #[test]
    fn the_profile_directory_is_three_levels_above_out_dir() {
        assert_eq!(
            profile_directory(Path::new("/target/debug/build/pkg-1234/out")),
            Path::new("/target/debug")
        );
    }
}
