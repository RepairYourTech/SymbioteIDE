//! What the test harness actually compiles and runs, read from cargo and from
//! the workflows' own discovery commands rather than assumed from the file
//! layout or restated here.
//!
//! A `#[test]` inside a file the harness never compiles is not evidence, and
//! neither is a Python test in a file no workflow's discovery command names.
//! The binding channel used to accept any file carrying a `#[test]` attribute,
//! so a binding could name a check that never ran — a fixture under
//! `tests/fixtures/` was refused only because it also carried `#[ignore]`, and
//! the Python half was read from a list kept in this module, which fell four
//! suites behind the job it described while every check stayed green. This
//! module makes the requirement structural: a Rust binding must resolve to a
//! file cargo reports as a target of a workspace member, or to a file inside a
//! member's compiled
//! source tree whose module a `mod` declaration actually reaches; a Python
//! binding must resolve to a file one of the workflows' own discovery commands
//! names.

use crate::catalog::Invariant;
use crate::report::Outcome;
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The Python suites the workflows run, read from their own `unittest discover`
/// commands rather than restated here.
///
/// One owner for this fact: a suite this file listed but no workflow ran would
/// be evidence nothing executes, and a suite a workflow runs but this file
/// omitted would make every binding into it a false refusal — which is how a
/// list kept here fell four suites behind the job it described. Adding, moving
/// or removing a suite is a change to the workflow, and this reading follows it.
///
/// A workflows directory that names no suite is an error rather than an empty
/// list: a harness with nothing to compare a binding against has not passed it.
pub fn suites_in(workflows: &Path) -> Result<Vec<(String, String)>, String> {
    let entries = std::fs::read_dir(workflows)
        .map_err(|error| format!("the workflow directory could not be read: {error}"))?;
    let mut files: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| ext == "yml" || ext == "yaml")
        })
        .collect();
    files.sort();
    let mut found: Vec<(String, String)> = Vec::new();
    for file in files {
        let text = std::fs::read_to_string(&file)
            .map_err(|error| format!("{} could not be read: {error}", file.display()))?;
        for suite in suites_in_text(&text) {
            if !found.contains(&suite) {
                found.push(suite);
            }
        }
    }
    if found.is_empty() {
        return Err(
            "no workflow names a Python suite with `unittest discover`, so a Python \
                    binding has nothing to be checked against"
                .into(),
        );
    }
    Ok(found)
}

/// The suites one workflow's own commands name: every `unittest discover` line,
/// as the directory it searches and the pattern it matches.
pub fn suites_in_text(workflow: &str) -> Vec<(String, String)> {
    workflow.lines().filter_map(suite_of).collect()
}

/// The suite one `unittest discover` command names, or `None` where the line is
/// not one: the directory from `-s`/`--start-directory` or from the `cd` the
/// command was reached through, and the pattern from `-p`/`--pattern`.
fn suite_of(line: &str) -> Option<(String, String)> {
    if !line.contains("unittest") || !line.contains("discover") {
        return None;
    }
    let tokens = shell_tokens(line);
    let at = tokens.iter().position(|token| token == "discover")?;
    let mut directory: Option<String> = None;
    let mut pattern: Option<String> = None;
    let mut index = at + 1;
    while index < tokens.len() {
        let token = tokens[index].as_str();
        if token == "-s" || token == "--start-directory" {
            directory = tokens.get(index + 1).cloned();
            index += 2;
            continue;
        }
        if token == "-p" || token == "--pattern" {
            pattern = tokens.get(index + 1).cloned();
            index += 2;
            continue;
        }
        if let Some(rest) = token.strip_prefix("--start-directory=") {
            directory = Some(rest.to_string());
        } else if let Some(rest) = token.strip_prefix("--pattern=") {
            pattern = Some(rest.to_string());
        }
        index += 1;
    }
    if directory.is_none() {
        let mut before = tokens[..at].iter();
        while let Some(token) = before.next() {
            if token == "cd" {
                directory = before.next().cloned();
            }
        }
    }
    let directory = directory.unwrap_or_else(|| ".".to_string());
    let directory = directory
        .strip_prefix("./")
        .unwrap_or(directory.as_str())
        .to_string();
    Some((directory, pattern.unwrap_or_else(|| "test*.py".to_string())))
}

/// A command's words, with quotes removed, so a `-p 'test_*.py'` names the same
/// pattern a shell would hand `unittest`.
fn shell_tokens(line: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    for character in line.chars() {
        match quote {
            Some(open) if character == open => quote = None,
            Some(_) => current.push(character),
            None if character == '\'' || character == '"' => quote = Some(character),
            None if character.is_whitespace() => {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
            }
            None => current.push(character),
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

/// The files a workspace test run compiles, and the suites it discovers.
pub struct Harness {
    root: PathBuf,
    /// The `src_path` of every target cargo reports for a workspace member.
    targets: BTreeSet<PathBuf>,
    /// `<member>/src` for every member, whose module tree cargo compiles.
    source_dirs: Vec<PathBuf>,
    /// The Python suites the workflows' own commands name.
    python_suites: Vec<(String, String)>,
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
        let python_suites = suites_in(&root.join(".github").join("workflows"))?;
        Ok(Self {
            root,
            targets,
            source_dirs,
            python_suites,
        })
    }

    /// The Python suites every workflow's own `unittest discover` commands name.
    pub fn python_suites(&self) -> &[(String, String)] {
        &self.python_suites
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
            return python_outcome(subject, path, name, &source, &self.python_suites);
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

/// Python bindings are checked against the discovery the workflows perform,
/// since `cargo test` never runs them.
fn python_outcome(
    subject: String,
    path: &str,
    name: &str,
    source: &str,
    suites: &[(String, String)],
) -> Outcome {
    let discovered = suites.iter().any(|(dir, pattern)| {
        path.starts_with(&format!("{dir}/")) && pattern_matches(pattern, path)
    });
    if !discovered {
        return Outcome::fail(
            "test",
            subject,
            "no workflow's `unittest discover` command names that file, so the test never runs",
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

/// Whether a file matches one `unittest discover -p` pattern, the way the
/// interpreter's own glob reads it for the patterns this repository uses.
pub fn pattern_matches(pattern: &str, path: &str) -> bool {
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
