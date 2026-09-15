//! What the test harness actually compiles and runs, read from cargo and from
//! the CI workflow's own discovery patterns rather than assumed from the file
//! layout.
//!
//! A `#[test]` inside a file the harness never compiles is not evidence. The
//! binding channel used to accept any file carrying a `#[test]` attribute, so a
//! binding could name a check that never ran — a fixture under `tests/fixtures/`
//! was refused only because it also carried `#[ignore]`. This module makes the
//! requirement structural: a Rust binding must resolve to a file cargo reports
//! as a target of a workspace member, or to a file inside a member's compiled
//! source tree whose module a `mod` declaration actually reaches; a Python
//! binding must resolve to a file one of the maintenance suites discovers.

use crate::catalog::Invariant;
use crate::report::Outcome;
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The maintenance suites the `offline-validation` job runs, as the workflow
/// spells them: a directory and the pattern `unittest discover -p` matches.
const PYTHON_SUITES: &[(&str, &str)] = &[
    ("planning", "test_legacy_importers.py"),
    ("planning/integrity", "test_*.py"),
    ("planning/housekeeping", "test_*.py"),
];

/// The files a workspace test run compiles, and the suites it discovers.
pub struct Harness {
    root: PathBuf,
    /// The `src_path` of every target cargo reports for a workspace member.
    targets: BTreeSet<PathBuf>,
    /// `<member>/src` for every member, whose module tree cargo compiles.
    source_dirs: Vec<PathBuf>,
}

impl Harness {
    /// Ask cargo which files the workspace compiles.
    ///
    /// A harness that cannot be discovered is an error rather than an empty
    /// set: a check with nothing to compare against is not a passing check.
    pub fn discover(root: &Path) -> Result<Self, String> {
        let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
        let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
        let output = Command::new(&cargo)
            .args([
                "metadata",
                "--no-deps",
                "--format-version",
                "1",
                "--manifest-path",
            ])
            .arg(root.join("Cargo.toml"))
            .output()
            .map_err(|error| format!("cargo could not be run to list the targets: {error}"))?;
        if !output.status.success() {
            return Err(format!(
                "cargo metadata failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }
        let metadata: Value = serde_json::from_slice(&output.stdout)
            .map_err(|error| format!("cargo metadata was not readable: {error}"))?;
        let mut targets = BTreeSet::new();
        let mut source_dirs = Vec::new();
        for package in metadata["packages"].as_array().into_iter().flatten() {
            if let Some(dir) = package["manifest_path"]
                .as_str()
                .map(Path::new)
                .and_then(Path::parent)
            {
                let src = dir.join("src");
                if src.is_dir() {
                    source_dirs.push(canonical(&src));
                }
            }
            for target in package["targets"].as_array().into_iter().flatten() {
                if let Some(path) = target["src_path"].as_str() {
                    targets.insert(canonical(Path::new(path)));
                }
            }
        }
        if targets.is_empty() {
            return Err("cargo reported no workspace target to check a binding against".into());
        }
        Ok(Self {
            root,
            targets,
            source_dirs,
        })
    }

    /// Every binding an invariant declares, checked against what the harness
    /// actually compiles and runs rather than against the file's shape alone.
    pub fn outcomes(&self, invariant: &Invariant) -> Vec<Outcome> {
        invariant
            .tests
            .iter()
            .map(|binding| self.outcome(binding))
            .collect()
    }

    /// Check one `relative/path::name` binding against what the harness runs.
    pub fn outcome(&self, binding: &str) -> Outcome {
        let subject = binding.to_string();
        let Some((path, name)) = binding.rsplit_once("::") else {
            return Outcome::fail("test", subject, "binding is not `path::test_name`");
        };
        let Ok(source) = std::fs::read_to_string(self.root.join(path)) else {
            return Outcome::fail("test", subject, "the named test file does not exist");
        };
        if path.ends_with(".py") {
            return python_outcome(subject, path, name, &source);
        }
        if !self.compiled(path) {
            return Outcome::fail(
                "test",
                subject,
                "the harness never compiles that file, so the check it names never runs",
            );
        }
        let Some(attributes) = attributes_above(&source, &format!("fn {name}(")) else {
            return Outcome::fail(
                "test",
                subject,
                "no test by that name in the named file, so the evidence moved",
            );
        };
        if !attributes.contains("#[test]") {
            return Outcome::fail(
                "test",
                subject,
                "the named item is not a test: no #[test] attribute",
            );
        }
        if attributes.contains("#[ignore") {
            return Outcome::fail(
                "test",
                subject,
                "the named test is skipped, and a skipped check is not evidence",
            );
        }
        Outcome::pass(
            "test",
            subject,
            "a compiled test the workspace run executes",
        )
    }

    /// Whether cargo compiles the file: it is a target, or it is inside a
    /// member's source tree and a `mod` declaration reaches it.
    fn compiled(&self, path: &str) -> bool {
        let file = canonical(&self.root.join(path));
        if self.targets.contains(&file) {
            return true;
        }
        self.source_dirs
            .iter()
            .any(|dir| file.starts_with(dir) && declares_module(dir, &file))
    }
}

/// Python bindings are checked against the discovery the workflow performs,
/// since `cargo test` never runs them.
fn python_outcome(subject: String, path: &str, name: &str, source: &str) -> Outcome {
    let discovered = PYTHON_SUITES.iter().any(|(dir, pattern)| {
        path.starts_with(&format!("{dir}/")) && pattern_matches(pattern, path)
    });
    if !discovered {
        return Outcome::fail(
            "test",
            subject,
            "no maintenance suite discovers that file, so the test never runs",
        );
    }
    let Some(attributes) = attributes_above(source, &format!("def {name}(")) else {
        return Outcome::fail(
            "test",
            subject,
            "no test by that name in the named file, so the evidence moved",
        );
    };
    if attributes.contains("@unittest.skip") {
        return Outcome::fail(
            "test",
            subject,
            "the named test is skipped, and a skipped check is not evidence",
        );
    }
    Outcome::pass(
        "test",
        subject,
        "a test the offline-validation suite discovers",
    )
}

/// The two `unittest discover -p` patterns the workflow uses.
fn pattern_matches(pattern: &str, path: &str) -> bool {
    let name = path.rsplit('/').next().unwrap_or("");
    match pattern.strip_suffix(".py") {
        Some(rest) => match rest.strip_suffix('*') {
            Some(prefix) => name.starts_with(prefix) && name.ends_with(".py"),
            None => name == pattern,
        },
        None => false,
    }
}

/// Whether a crate's source tree declares `file` as a module. Cargo compiles
/// only what a `mod` declaration or a `#[path]` attribute reaches, so a file
/// nothing declares is not a compiled check however test-shaped it looks.
fn declares_module(src_dir: &Path, file: &Path) -> bool {
    let Some(stem) = file.file_stem().and_then(|stem| stem.to_str()) else {
        return false;
    };
    let Some(name) = file.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    let mut stack = vec![src_dir.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue;
            };
            for line in text.lines() {
                let line = line.trim_start();
                let declaration = line.starts_with("mod ")
                    || line.starts_with("pub mod ")
                    || line.starts_with("pub(crate) mod ")
                    || line.starts_with("pub(super) mod ")
                    || line.starts_with("#[path");
                if !declaration {
                    continue;
                }
                if line.contains(&format!("mod {stem}")) || line.contains(&format!("\"{name}\"")) {
                    return true;
                }
            }
        }
    }
    false
}

/// The contiguous attribute/doc-comment block directly above an item, read so
/// a skipped or non-test item cannot be passed off as evidence.
fn attributes_above(source: &str, needle: &str) -> Option<String> {
    let at = source.find(needle)?;
    let line_start = source[..at].rfind('\n').map_or(0, |end| end + 1);
    let mut block = Vec::new();
    for line in source[..line_start].lines().rev() {
        let trimmed = line.trim();
        if trimmed.starts_with("#[")
            || trimmed.starts_with("///")
            || trimmed.starts_with("//!")
            || trimmed.starts_with('@')
        {
            block.push(trimmed);
        } else {
            break;
        }
    }
    Some(block.join("\n"))
}

fn canonical(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}
