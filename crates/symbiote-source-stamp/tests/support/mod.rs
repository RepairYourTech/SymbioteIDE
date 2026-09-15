//! The fixture scaffolding both real-build harnesses need: a temporary workspace
//! the cargo that is running the test builds, the record that build wrote, and
//! the two readers of it.
//!
//! Shared for the reason `crates/symbiote-host/tests/support` is: two copies of
//! the cargo invocation, of how a record is found under a build directory, or of
//! the checker's command line would let one harness prove something the other
//! does not, and every one of those is a place a measured claim lives. What is
//! *not* shared is either harness's own fixture — what a case writes into the
//! workspace, and what it asserts about the record — which is the whole
//! difference between the entry-point proofs and the guard's.
//!
//! Each test crate compiles its own copy and uses a subset of it, so unused
//! helpers here are not dead code.
#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use symbiote_source_stamp::{Input, RECORD_FILE, changed_sources};

/// A workspace under a fresh temporary directory named after `case`.
///
/// Nothing it holds outlives it: the directory is removed when this is dropped,
/// a passed case and a failed one alike.
pub struct Workspace {
    root: PathBuf,
}

impl Workspace {
    /// The workspace under a fresh temporary directory named after `case`.
    pub fn new(case: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "symbiote-source-stamp-{case}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("the fixture directory");
        // Canonical, because a record spells a path outside the package's own
        // workspace absolutely and the walk canonicalizes what it records.
        Self {
            root: std::fs::canonicalize(&root).expect("a canonical fixture directory"),
        }
    }

    /// The path of a fixture file, whether or not it exists yet.
    pub fn path(&self, relative: &str) -> PathBuf {
        self.root.join(relative)
    }

    /// Writes a fixture file, making the directories above it.
    pub fn write(&self, relative: &str, contents: &str) {
        let path = self.path(relative);
        std::fs::create_dir_all(path.parent().expect("a parent")).expect("the directory");
        std::fs::write(path, contents).expect("the fixture file");
    }

    /// A cargo command line of its own, for a case that sets the environment
    /// itself: the build every harness runs, with the parts a case varies.
    pub fn cargo(&self, run_from: &str, target: &str, args: &[&str]) -> Command {
        let cargo = std::env::var_os("CARGO").expect("these tests run under cargo");
        let mut command = Command::new(cargo);
        command
            .args(["build", "--offline"])
            .args(args)
            .current_dir(self.path(run_from))
            .env("CARGO_TARGET_DIR", self.path(target));
        command
    }

    /// Cargo's build of the fixture, `args` beside `build --offline`, run in
    /// `run_from` with the build directory `target`.
    pub fn build(&self, run_from: &str, target: &str, args: &[&str]) -> Output {
        self.cargo(run_from, target, args)
            .output()
            .expect("cargo runs")
    }

    /// The record the fixture's build wrote, as the lines it holds — the
    /// generated file under the build directory, the way a harness finds the
    /// artifact that carries it beside it.
    ///
    /// A build told `--target` nests that directory under the triple's own, so it
    /// is looked for both where cargo puts it by default and one level down.
    pub fn record(&self, target: &str) -> String {
        let root = self.path(target);
        let mut builds = vec![root.join("debug/build")];
        builds.extend(
            std::fs::read_dir(&root)
                .into_iter()
                .flatten()
                .flatten()
                .map(|entry| entry.path().join("debug/build")),
        );
        let mut records: Vec<PathBuf> = builds
            .iter()
            .flat_map(|build| std::fs::read_dir(build).into_iter().flatten().flatten())
            .map(|entry| entry.path().join("out").join(RECORD_FILE))
            .filter(|path| path.is_file())
            .collect();
        records.sort();
        assert_eq!(records.len(), 1, "one record for the package: {records:?}");
        std::fs::read_to_string(records.pop().expect("the package's record"))
            .expect("the record is readable")
    }

    /// The guard's verdict for `artifact`: the inputs whose recorded content no
    /// longer matches the tree, or the reason the record cannot be checked.
    ///
    /// `package` is the package directory the artifact was built from, which is
    /// what a record's own locators are relative to.
    pub fn verdict(&self, artifact: &Path, package: &str) -> Result<Vec<Input>, String> {
        changed_sources(artifact, &self.path(package))
    }

    /// The verdict for an artifact whose record must stand behind the tree, so a
    /// refusal is a failure of the fixture rather than a verdict.
    pub fn differences(&self, artifact: &Path, package: &str) -> Vec<Input> {
        self.verdict(artifact, package)
            .unwrap_or_else(|problem| panic!("the fixture's record is readable: {problem}"))
    }

    /// The repository's own checker as a command line waiting for its `--binary`
    /// arguments: the interpreter it is written in, the workspace and profile
    /// directory the check is of, and the offline resolution a fixture's build
    /// leaves behind.
    ///
    /// This crate is the first reader of a record and the Python tool the second,
    /// so the two agree about an artifact only if both are put to it: what a
    /// build's record has to name is the checker's question, and a harness's
    /// shapes are refused or accepted by it rather than by this crate. A harness
    /// appends the artifacts of its own cases, since the checker takes any number
    /// of them and the two name them differently.
    pub fn checker(&self, workspace: &str, profile: &Path) -> Command {
        let checker =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../planning/integrity/source_record.py");
        let mut command = Command::new("python3");
        command
            .arg(&checker)
            .arg("--workspace")
            .arg(self.path(workspace))
            .arg("--target-dir")
            .arg(profile)
            // The fixture builds offline, so its metadata resolves offline too.
            .env("CARGO_NET_OFFLINE", "true");
        command
    }
}

/// Nothing the fixture built outlives the test that built it: a build directory
/// per case, passed and failed alike, is otherwise left in the temporary
/// directory.
impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

/// The fields of the wire file's lines with `keyword`, in the order the file
/// spells them: a test takes a value from the one copy of the wire rather than
/// from a copy of it that could drift. A `mark` line is a role, a locator and the
/// remedy both readers print.
pub fn wire_lines(keyword: &str) -> Vec<Vec<String>> {
    std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/wire.txt"))
        .expect("the wire file")
        .lines()
        .filter_map(|line| {
            let mut fields = line.split('\t');
            (fields.next() == Some(keyword)).then(|| fields.map(str::to_owned).collect())
        })
        .collect()
}

/// The wire file's line for a mark: its locator and the remedy both readers print.
pub fn wire_mark(role: &str) -> (String, String) {
    let fields = wire_lines("mark")
        .into_iter()
        .find(|fields| fields[0] == role)
        .expect("the wire file names that mark");
    (fields[1].clone(), fields[2].clone())
}

/// The one value a scalar keyword holds.
pub fn wire_scalar(keyword: &str) -> String {
    wire_lines(keyword)
        .first()
        .expect("the wire file names that keyword")[0]
        .clone()
}
