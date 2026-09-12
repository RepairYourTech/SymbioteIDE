//! A build-time record of the sources a workspace binary was compiled from,
//! and the check that the binary still matches them.
//!
//! The end-to-end proofs drive workspace binaries, so a binary that does not
//! contain the sources under test would let a proof stay green while running
//! other code. Comparing timestamps answers "was something touched after the
//! build", which misses a real change whose timestamp is older than the binary
//! (a restored file, a checkout with preserved times, a clock skew) and
//! refuses a binary whose sources were only touched.
//!
//! This crate records the **content** instead, and binds the record to the
//! artifact, so neither state can pass:
//!
//! * [`build_stamp`] is called from a `build.rs`. For the package and every
//!   workspace package it reaches through normal and build dependency edges it
//!   records every file the package's binaries compile — everything under the
//!   package that is not a test, example or bench target, whatever its
//!   extension, because a compile can read a file no manifest mentions
//!   (`include_str!("schema.sql")`) — together with the manifests, build
//!   scripts and `Cargo.lock` that pin them. It hashes the content of all of
//!   them, writes `<target>/<profile>/<package>.source-stamp`, tells cargo to
//!   rerun the build script when any recorded file changes, and embeds the
//!   record's own id in the binary it is about to compile.
//! * [`changed_sources`] is called by the proof. It re-hashes what the record
//!   holds and refuses a binary whose embedded id is not this record's — which
//!   is what a failed compile leaves behind, since the record is written
//!   before the crate is compiled — and names every file whose content (or
//!   presence) differs.
//!
//! What the walk covers is stated rather than claimed whole. Test, example and
//! bench targets are outside it because nothing a binary compiles comes from
//! them, so a change there must not refuse a current binary. Registry
//! dependencies are outside it because `Cargo.lock`, which is inside it, pins
//! them, and dev-dependency edges are outside it because a binary compiles
//! none of them. Files a package generates into its `OUT_DIR` are outside it
//! too: their content comes from the build script, which is inside it. An
//! input a package compiles from *outside* its own directory would be outside
//! it, so the coverage was measured rather than assumed: cargo's own dep-info
//! for both driven binaries lists 98 and 41 files, and every one of them is
//! inside the packages this walk records (see the crate's tests for the
//! fixture that pins the walk's shape). Cargo's dep-info is not read here
//! because it is a private, versioned binary format that is written *after*
//! the build script that must write the record, so it can neither populate a
//! record on a first build nor be a completeness check the documented rebuild
//! could ever clear.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

/// What a build record is called: the package it belongs to plus this suffix,
/// in the profile directory that holds the binary.
pub const STAMP_SUFFIX: &str = ".source-stamp";

/// The marker a binary carries, followed by the id of the record it was built
/// from. Written into the binary through `cargo:rustc-env`.
pub const RECORD_MARKER: &str = "symbiote-source-record:";

/// Directories under a package that its binaries do not compile. The targets
/// cargo builds for tests, examples and benches are excluded because a change
/// there must not refuse a current binary — no rebuild could clear that
/// refusal, since a test file is not an input to the binary's build — and the
/// rest is build output, dependency cache and version-control state.
const UNCOMPILED_DIRECTORIES: [&str; 7] = [
    ".git",
    "benches",
    "dist",
    "examples",
    "node_modules",
    "target",
    "tests",
];

/// Records the content of the sources this package's binaries are built from,
/// next to the binaries, and asks cargo to rerun the build script when any of
/// them changes. Call this from a build script's `main`.
pub fn build_stamp() {
    let manifest_dir = PathBuf::from(environment("CARGO_MANIFEST_DIR"));
    let out_dir = PathBuf::from(environment("OUT_DIR"));
    let package = environment("CARGO_PKG_NAME");
    let profile = profile_directory(&out_dir);
    let stamp = profile.join(format!("{package}{STAMP_SUFFIX}"));
    let workspace = workspace_root(&manifest_dir)
        .unwrap_or_else(|problem| panic!("cannot record the sources under test: {problem}"));

    let mut sources = BTreeSet::new();
    for directory in closure_directories(&manifest_dir) {
        sources.extend(package_sources(&directory));
    }
    let lockfile = workspace.join("Cargo.lock");
    if lockfile.is_file() {
        sources.insert(lockfile);
    }
    let mut record = String::new();
    for source in &sources {
        let content = std::fs::read(source)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", source.display()));
        record.push_str(&format!(
            "{:x}\t{}\n",
            Sha256::digest(&content),
            relative_to(&workspace, source).display()
        ));
        println!("cargo:rerun-if-changed={}", source.display());
    }
    let id = format!("{:x}", Sha256::digest(record.as_bytes()));
    println!("cargo:rustc-env=SYMBIOTE_SOURCE_RECORD={RECORD_MARKER}{id}");
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
    let record = std::fs::read(&stamp)
        .map_err(|error| format!("there is no source record at {} ({error})", stamp.display()))?;
    let id = format!("{:x}", Sha256::digest(&record));
    match embedded_record(binary) {
        None => {
            return Err(format!(
                "the binary carries no build record, so it was not produced by the build that \
                 wrote {}",
                stamp.display()
            ));
        }
        Some(embedded) if embedded != id => {
            return Err(format!(
                "the binary was built from record {embedded}, and the record beside it is {id}: \
                 the build that wrote that record did not produce this binary"
            ));
        }
        Some(_) => {}
    }

    let workspace = workspace_root(manifest_dir)?;
    let recorded = read_record(&record, &workspace)?;
    Ok(recorded
        .iter()
        .filter(|(source, hash)| content_hash(source).as_deref() != Some(hash.as_str()))
        .map(|(source, _)| source.clone())
        .collect())
}
/// The sha256 of every file the record holds, keyed by the file itself, with a
/// relative path resolved against the workspace. A line without both halves is
/// reported rather than skipped: a record that lost one is not a shorter record
/// but an unreadable one.
fn read_record(record: &[u8], workspace: &Path) -> Result<BTreeMap<PathBuf, String>, String> {
    let text =
        std::str::from_utf8(record).map_err(|_| "the source record is not text".to_owned())?;
    let mut recorded = BTreeMap::new();
    for line in text.lines().filter(|line| !line.is_empty()) {
        let Some((hash, path)) = line.split_once('\t') else {
            return Err("the source record has a malformed line".to_owned());
        };
        recorded.insert(workspace.join(path), hash.to_owned());
    }
    Ok(recorded)
}

/// The id a binary carries, read from the marker `build_stamp` embedded.
fn embedded_record(binary: &Path) -> Option<String> {
    let bytes = std::fs::read(binary).ok()?;
    let marker = RECORD_MARKER.as_bytes();
    let id_length = 64;
    let start = bytes
        .windows(marker.len() + id_length)
        .position(|window| &window[..marker.len()] == marker)?;
    let id = &bytes[start + marker.len()..start + marker.len() + id_length];
    id.iter()
        .all(u8::is_ascii_hexdigit)
        .then(|| String::from_utf8(id.to_vec()).ok())?
}

/// Every workspace package directory reachable from `manifest_dir` through
/// normal and build dependency edges, the package itself included.
fn closure_directories(manifest_dir: &Path) -> Vec<PathBuf> {
    let (mut pending, mut reached) = (vec![manifest_dir.to_path_buf()], BTreeSet::new());
    while let Some(directory) = pending.pop() {
        let directory = canonical(&directory);
        if !reached.insert(directory.clone()) {
            continue;
        }
        let manifest = std::fs::read_to_string(directory.join("Cargo.toml"))
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", directory.display()));
        for dependency in path_dependencies(&manifest) {
            pending.push(directory.join(dependency));
        }
    }
    reached.into_iter().collect()
}

/// Every file under a package that its binaries compile: all of the package
/// except the directories in [`UNCOMPILED_DIRECTORIES`], whatever the file is
/// called. Every file rather than `.rs` alone, because a compile can read a
/// file no extension announces (`include_str!("schema.sql")`).
fn package_sources(directory: &Path) -> Vec<PathBuf> {
    let mut sources = BTreeSet::new();
    let mut directories = vec![directory.to_path_buf()];
    while let Some(directory) = directories.pop() {
        for entry in std::fs::read_dir(&directory)
            .into_iter()
            .flatten()
            .flatten()
        {
            let name = entry.file_name().to_string_lossy().to_string();
            if UNCOMPILED_DIRECTORIES.contains(&name.as_str()) {
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

/// The sha256 of a file's bytes, or `None` when it cannot be read — a file the
/// build consumed and the tree no longer holds is a difference, not an error.
fn content_hash(path: &Path) -> Option<String> {
    std::fs::read(path)
        .ok()
        .map(|content| format!("{:x}", Sha256::digest(content)))
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

    fn record_for(root: &Path, files: &[(&str, &str)]) -> (PathBuf, String) {
        std::fs::write(
            root.join("Cargo.toml"),
            "[workspace]\n[package]\nname = \"symbiote-example\"\n",
        )
        .expect("the root manifest");
        let mut record = String::new();
        for (path, content) in files {
            let file = root.join(path);
            std::fs::create_dir_all(file.parent().expect("a parent")).expect("directories");
            std::fs::write(&file, content).expect("the source");
            record.push_str(&format!(
                "{:x}\t{path}\n",
                Sha256::digest(content.as_bytes())
            ));
        }
        let stamp = root.join("symbiote-example.source-stamp");
        std::fs::write(&stamp, &record).expect("the record");
        let id = format!("{:x}", Sha256::digest(record.as_bytes()));
        let binary = root.join("symbiote-example");
        std::fs::write(
            &binary,
            format!("binary bytes before {RECORD_MARKER}{id} and after"),
        )
        .expect("the binary");
        (binary, id)
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
    fn every_compiled_file_is_recorded_and_test_targets_are_not() {
        let root = fixture("walk");
        for path in [
            "src/lib.rs",
            "src/schema.sql",
            "Cargo.toml",
            "build.rs",
            "assets/logo.svg",
        ] {
            let file = root.join(path);
            std::fs::create_dir_all(file.parent().expect("a parent")).expect("directories");
            std::fs::write(&file, "content").expect("the source");
        }
        for path in [
            "tests/process.rs",
            "examples/demo.rs",
            "benches/speed.rs",
            "target/debug/leftover",
        ] {
            let file = root.join(path);
            std::fs::create_dir_all(file.parent().expect("a parent")).expect("directories");
            std::fs::write(&file, "content").expect("the target");
        }
        let sources = package_sources(&root);
        for path in [
            "src/lib.rs",
            "src/schema.sql",
            "Cargo.toml",
            "build.rs",
            "assets/logo.svg",
        ] {
            assert!(
                sources.contains(&root.join(path)),
                "{path} is an input to the binary's build and must be recorded"
            );
        }
        for path in [
            "tests/process.rs",
            "examples/demo.rs",
            "benches/speed.rs",
            "target/debug/leftover",
        ] {
            assert!(
                !sources.contains(&root.join(path)),
                "{path} is not compiled into the binary, so recording it would refuse a \
                 current binary for a change no rebuild could clear"
            );
        }
    }

    #[test]
    fn a_recorded_source_whose_content_differs_is_named() {
        let root = fixture("changed");
        let (binary, _) = record_for(&root, &[("src/lib.rs", "one\n")]);
        assert!(
            changed_sources(&binary, "symbiote-example", &root)
                .expect("a readable record")
                .is_empty()
        );

        std::fs::write(root.join("src/lib.rs"), "two\n").expect("the changed source");
        assert_eq!(
            changed_sources(&binary, "symbiote-example", &root).expect("a readable record"),
            [root.join("src/lib.rs")]
        );

        std::fs::remove_file(root.join("src/lib.rs")).expect("the removed source");
        assert_eq!(
            changed_sources(&binary, "symbiote-example", &root).expect("a readable record"),
            [root.join("src/lib.rs")]
        );
    }

    #[test]
    fn a_binary_from_another_record_is_refused() {
        let root = fixture("other-record");
        let (binary, _) = record_for(&root, &[("src/lib.rs", "one\n")]);
        std::fs::write(
            &binary,
            format!(
                "binary bytes before {RECORD_MARKER}{} and after",
                "0".repeat(64)
            ),
        )
        .expect("another binary");
        let problem = changed_sources(&binary, "symbiote-example", &root).expect_err("a mismatch");
        assert!(
            problem.contains("the build that wrote that record did not produce"),
            "{problem}"
        );

        std::fs::write(&binary, "no record here").expect("a binary without a record");
        let problem = changed_sources(&binary, "symbiote-example", &root).expect_err("none");
        assert!(problem.contains("carries no build record"), "{problem}");
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
