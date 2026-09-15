//! The crate's real entry point, driven the way a consumer drives it: a fixture
//! workspace whose build script calls [`build_stamp`], built by cargo, then
//! checked by [`changed_sources`].
//!
//! The unit tests in `src/tests.rs` reach the walk's own functions, so they can
//! say which inputs a record holds but not that the wiring that produces one
//! works — the `OUT_DIR` and `PWD` readings, the invoking process's directory and
//! command line, the candidate roots, the resolution gate. Every proof of that
//! wiring has been made by hand, in scratch workspaces, and the regressions
//! audits found since (a candidate root dropped for an optional dependency, the
//! `--target` level mis-counted, and a `--manifest-path` build whose resolution
//! root nothing but the command line named) survived because nothing here would
//! have failed. These tests are that proof, in the suite.
//!
//! They are hermetic: a fresh directory under the temporary directory per test,
//! every cargo invocation `--offline`, and no fixture outside those
//! directories. They need the cargo that is running them (`CARGO`), which is
//! what a test run through `cargo test` has.
//!
//! [`build_stamp`]: symbiote_source_stamp::build_stamp

use std::path::{Path, PathBuf};
use std::process::Command;

use symbiote_source_stamp::{Input, RECORD_FILE, changed_sources};

/// The fixture the cases build from.
///
/// ```text
/// own/            the workspace the package names, with its own fork
/// pkg/            the package: a build script that stamps, a library that
///                 carries the record and compiles the patched dependency
/// sibling/        a workspace whose member takes the package by path, with a
///                 fork of its own
/// optional/       the same, but the member reaches the package only through an
///                 optional dependency it enables by feature
/// solo/           a package no `[workspace]` manifest owns, taking the package
///                 by path, with a fork its own `[patch]` table names
/// ```
struct Fixture {
    root: PathBuf,
}

impl Fixture {
    /// The fixture under a fresh temporary directory named after `case`.
    fn new(case: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "symbiote-source-stamp-{case}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("the fixture directory");
        // Canonical, because a record spells a path outside the package's own
        // workspace absolutely and the walk canonicalizes what it records.
        let root = std::fs::canonicalize(&root).expect("a canonical fixture directory");
        let fixture = Self { root };
        // The package the cases stamp, and the workspace it names. Its fork is
        // the one its own build compiles.
        fixture.write(
            "own/Cargo.toml",
            "[workspace]\nmembers = [\"../pkg\"]\nresolver = \"2\"\n\n\
             [patch.crates-io]\nserde = { path = \"fork\" }\n",
        );
        fixture.fork("own", "own");
        let stamp = format!("{:?}", env!("CARGO_MANIFEST_DIR"));
        fixture.write(
            "pkg/Cargo.toml",
            &format!(
                "[package]\nname = \"pkg\"\nversion = \"0.0.0\"\nedition = \"2021\"\n\
                 workspace = \"../own\"\n\n\
                 [dependencies]\nserde = \"1\"\n\n\
                 [build-dependencies]\nsymbiote-source-stamp = {{ path = {stamp} }}\n"
            ),
        );
        fixture.write(
            "pkg/build.rs",
            "fn main() {\n    symbiote_source_stamp::build_stamp();\n}\n",
        );
        fixture.write(
            "pkg/src/lib.rs",
            "include!(concat!(env!(\"OUT_DIR\"), \"/source_record.rs\"));\n\n\
             pub fn fork() -> &'static str {\n    serde::FORK\n}\n",
        );
        // The sibling workspace: its member takes the package by path, so a
        // build it runs compiles the package with *its* fork.
        fixture.write(
            "sibling/Cargo.toml",
            "[workspace]\nmembers = [\"member\"]\nresolver = \"2\"\n\n\
             [patch.crates-io]\nserde = { path = \"fork\" }\n",
        );
        fixture.write(
            "sibling/member/Cargo.toml",
            "[package]\nname = \"member\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n\
             [dependencies]\npkg = { path = \"../../pkg\" }\n",
        );
        fixture.write(
            "sibling/member/src/lib.rs",
            "pub fn member() -> &'static str {\n    pkg::fork()\n}\n",
        );
        fixture.fork("sibling", "sibling");
        // The workspace whose only route to the package is an optional
        // dependency: a resolution under the default features has no package to
        // resolve, while a build with the feature compiles it and this fork.
        fixture.write(
            "optional/Cargo.toml",
            "[workspace]\nmembers = [\"member\"]\nresolver = \"2\"\n\n\
             [patch.crates-io]\nserde = { path = \"fork\" }\n",
        );
        fixture.write(
            "optional/member/Cargo.toml",
            "[package]\nname = \"member\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n\
             [dependencies]\npkg = { path = \"../../pkg\", optional = true }\n\n\
             [features]\npkg = [\"dep:pkg\"]\n",
        );
        fixture.write(
            "optional/member/src/lib.rs",
            "pub fn member() -> &'static str {\n    \"member\"\n}\n",
        );
        fixture.fork("optional", "optional");
        // A third workspace, which is where the next case runs cargo from: it
        // names a workspace, so its fork is a candidate root, while the
        // resolution is read in the workspace the command line names.
        fixture.write(
            "third/Cargo.toml",
            "[workspace]\nmembers = []\nresolver = \"2\"\n\n\
             [patch.crates-io]\nserde = { path = \"fork\" }\n",
        );
        fixture.fork("third", "third");
        // A package no `[workspace]` manifest owns: cargo makes it a workspace
        // of one, so the `[patch]` table beside its manifest applies to a build
        // of it and no manifest above it is an input.
        fixture.write(
            "solo/Cargo.toml",
            "[package]\nname = \"solo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n\
             [dependencies]\npkg = { path = \"../pkg\" }\n\n\
             [patch.crates-io]\nserde = { path = \"fork\" }\n",
        );
        fixture.write(
            "solo/src/lib.rs",
            "pub fn solo() -> &'static str {\n    pkg::fork()\n}\n",
        );
        fixture.fork("solo", "solo");
        fixture
    }

    /// A `serde` fork of `workspace`'s own, whose constant names it.
    fn fork(&self, workspace: &str, name: &str) {
        self.write(
            &format!("{workspace}/fork/Cargo.toml"),
            "[package]\nname = \"serde\"\nversion = \"1.0.0\"\nedition = \"2021\"\n",
        );
        self.write(
            &format!("{workspace}/fork/src/lib.rs"),
            &format!("pub const FORK: &str = {name:?};\n"),
        );
    }

    fn write(&self, relative: &str, contents: &str) {
        let path = self.path(relative);
        std::fs::create_dir_all(path.parent().expect("a parent")).expect("the directory");
        std::fs::write(path, contents).expect("the fixture file");
    }

    fn path(&self, relative: &str) -> PathBuf {
        self.root.join(relative)
    }

    /// Cargo's build of the fixture, `args` beside `build --offline`, run in
    /// `run_from` with the build directory `target` and `PWD` as a shell that
    /// ran it records it.
    fn build(&self, run_from: &str, target: &str, args: &[&str]) -> std::process::Output {
        self.build_in(run_from, target, args, Some(run_from))
    }

    /// The same build with `PWD` set to `pwd` — a directory other than the one
    /// cargo is run in, which is what a launcher that sets cargo's working
    /// directory and leaves the variable inherited leaves behind — or removed
    /// where it is `None`, which is the `env -u PWD cargo build` case: measured,
    /// cargo passes that variable through rather than setting it.
    fn build_in(
        &self,
        run_from: &str,
        target: &str,
        args: &[&str],
        pwd: Option<&str>,
    ) -> std::process::Output {
        let mut command = self.cargo(run_from, target, args);
        match pwd {
            Some(directory) => {
                command.env("PWD", self.path(directory));
            }
            None => {
                command.env_remove("PWD");
            }
        }
        command.output().expect("cargo runs")
    }

    /// A cargo command line of its own, for the alias case: cargo's subcommand
    /// is then the alias's name, and what it expands to comes from the
    /// configuration cargo reads rather than from the arguments.
    fn alias_build(&self, run_from: &str, target: &str, args: &[&str]) -> std::process::Output {
        let cargo = std::env::var_os("CARGO").expect("these tests run under cargo");
        Command::new(cargo)
            .args(args)
            .current_dir(self.path(run_from))
            .env("CARGO_TARGET_DIR", self.path(target))
            .env("PWD", self.path(run_from))
            .output()
            .expect("cargo runs")
    }

    fn cargo(&self, run_from: &str, target: &str, args: &[&str]) -> Command {
        let cargo = std::env::var_os("CARGO").expect("these tests run under cargo");
        let mut command = Command::new(cargo);
        command
            .args(["build", "--offline"])
            .args(args)
            .current_dir(self.path(run_from))
            .env("CARGO_TARGET_DIR", self.path(target));
        command
    }

    /// The package's library as cargo built it: the artifact that carries the
    /// record, since `pkg/src/lib.rs` includes the generated static. A build
    /// told `--target` nests it under the triple's own directory, so the
    /// dependency directories are looked for both there and one level down.
    fn artifact(&self, target: &str) -> PathBuf {
        let root = self.path(target);
        let mut directories = vec![root.join("debug/deps")];
        directories.extend(
            std::fs::read_dir(&root)
                .into_iter()
                .flatten()
                .flatten()
                .map(|entry| entry.path().join("debug/deps")),
        );
        let mut libraries: Vec<PathBuf> = directories
            .iter()
            .flat_map(|deps| std::fs::read_dir(deps).into_iter().flatten().flatten())
            .map(|entry| entry.path())
            .filter(|path| {
                let name = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                name.starts_with("libpkg-") && name.ends_with(".rlib")
            })
            .collect();
        libraries.sort();
        assert_eq!(
            libraries.len(),
            1,
            "one library for the package: {libraries:?}"
        );
        libraries.pop().expect("the package's library")
    }

    /// The guard's verdict for `artifact`: the inputs whose recorded content no
    /// longer matches the tree, or the reason the record cannot be checked.
    fn verdict(&self, artifact: &Path) -> Result<Vec<Input>, String> {
        changed_sources(artifact, &self.path("pkg"))
    }

    /// The verdict for an artifact whose record must stand behind the tree, so
    /// a refusal is a failure of the fixture rather than a verdict.
    fn differences(&self, artifact: &Path) -> Vec<Input> {
        self.verdict(artifact)
            .unwrap_or_else(|problem| panic!("the fixture's record is readable: {problem}"))
    }

    /// The repository's own checker, run on `artifacts` the way CI runs it.
    ///
    /// This crate is the first reader of a record and the Python tool the
    /// second, so the two agree about an artifact only if both are put to it:
    /// the proof that a marked artifact is refused runs the checker on the same
    /// bytes rather than only this crate. Several artifacts go to one run, since
    /// the checker takes any number of them.
    fn checker(&self, workspace: &str, artifacts: &[&Path]) -> std::process::Output {
        let checker =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../planning/integrity/source_record.py");
        let first = artifacts.first().expect("at least one artifact");
        let deps = first.parent().expect("the deps directory");
        let mut command = Command::new("python3");
        command.arg(&checker);
        for artifact in artifacts {
            let name = artifact.file_name().expect("an artifact name");
            command
                .arg("--binary")
                .arg(format!("pkg:deps/{}", name.to_string_lossy()));
        }
        command
            .arg("--workspace")
            .arg(self.path(workspace))
            .arg("--target-dir")
            .arg(deps.parent().expect("the profile directory"))
            // The fixture builds offline, so its metadata resolves offline too.
            .env("CARGO_NET_OFFLINE", "true")
            .output()
            .expect("the integrity checker runs (it needs python3)")
    }

    /// The record the fixture's build wrote, as the lines it holds — the
    /// generated file under the build directory the way `artifact` finds the
    /// library beside it.
    fn record(&self, target: &str) -> String {
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
}

/// Nothing the fixture built outlives the test that built it: a build directory
/// per case, passed and failed alike, is otherwise left in the temporary
/// directory.
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

/// The arrangement the crate exists for: a member of a sibling workspace takes
/// the package by path, the sibling's `[patch]` fork is what the build compiles,
/// and the build directory belongs to the *package's* workspace — so only the
/// directory cargo was run in names the sibling at all. A change to the fork it
/// compiled must refuse the artifact, and nothing else must.
#[test]
fn a_sibling_workspace_that_compiles_the_package_records_its_fork() {
    let fixture = Fixture::new("sibling");
    let build = fixture.build("sibling", "own/target", &["-p", "member"]);
    assert!(
        build.status.success(),
        "the fixture builds: {}",
        String::from_utf8_lossy(&build.stderr)
    );
    let artifact = fixture.artifact("own/target");
    assert_eq!(
        fixture.differences(&artifact),
        [],
        "the tree is what it was built from"
    );

    let fork = fixture.path("sibling/fork/src/lib.rs");
    fixture.write(
        "sibling/fork/src/lib.rs",
        "pub const FORK: &str = \"changed\";\n",
    );
    assert_eq!(
        fixture.differences(&artifact),
        [Input::File(fork)],
        "a change to the fork the sibling's build compiled must refuse the artifact"
    );

    fixture.write(
        "sibling/fork/src/lib.rs",
        "pub const FORK: &str = \"sibling\";\n",
    );
    assert_eq!(
        fixture.differences(&artifact),
        [],
        "and the tree it was built from is accepted again"
    );
}

/// The arrangement that regressed last: a workspace reaches the package only
/// through an optional dependency its member enables, so a resolution asked for
/// under the default features answers that the package is not there — while the
/// build with the feature compiles the package and this workspace's fork. The
/// fork must be an input regardless of what that answer says.
#[test]
fn a_workspace_reaching_the_package_through_an_optional_dependency_records_its_fork() {
    let fixture = Fixture::new("optional");
    let build = fixture.build(
        "optional",
        "elsewhere",
        &["-p", "member", "--features", "pkg"],
    );
    assert!(
        build.status.success(),
        "the fixture builds: {}",
        String::from_utf8_lossy(&build.stderr)
    );
    let artifact = fixture.artifact("elsewhere");
    assert_eq!(
        fixture.differences(&artifact),
        [],
        "the tree is what it was built from"
    );

    let fork = fixture.path("optional/fork/src/lib.rs");
    fixture.write(
        "optional/fork/src/lib.rs",
        "pub const FORK: &str = \"changed\";\n",
    );
    assert_eq!(
        fixture.differences(&artifact),
        [Input::File(fork)],
        "the fork the feature-enabled build compiled must refuse the artifact"
    );

    fixture.write(
        "optional/fork/src/lib.rs",
        "pub const FORK: &str = \"optional\";\n",
    );
    assert_eq!(
        fixture.differences(&artifact),
        [],
        "and the tree it was built from is accepted again"
    );
}

/// The arrangement this crate used to report clean: the sibling workspace does
/// the building and points the target directory at the *package's own* root's
/// `target`, and no `PWD` says where cargo was run — so the build directory is
/// the only reading, and it names a workspace that need not be the one the
/// resolution was made in. The directory cargo was started in says which one it
/// is, so the record holds the fork that build compiled.
///
/// The target triple is there because the build directory of a `--target` build
/// sits one level under the target directory: the readings have to take the root
/// two levels above the profile, and a rule that read one level where cargo put
/// two would find no workspace at all and stop the build.
#[test]
fn a_build_without_a_reported_run_directory_records_the_workspace_cargo_resolved_in() {
    let fixture = Fixture::new("unreported");
    let triple = host_triple();
    let build = fixture.build_in(
        "sibling",
        "own/target",
        &["-p", "member", "--target", &triple],
        None,
    );
    assert!(
        build.status.success(),
        "the fixture builds: {}",
        String::from_utf8_lossy(&build.stderr)
    );
    let artifact = fixture.artifact("own/target");

    // The workspace cargo was run in, and the fork its build compiled, are
    // inputs the build directory alone would not have named.
    let record = fixture.record("own/target");
    for named in [
        "sibling/fork/src/lib.rs",
        "sibling/Cargo.toml",
        "sibling/Cargo.lock",
    ] {
        assert!(
            record.contains(named),
            "{named} is an input of a build from that workspace: {record}"
        );
    }

    let fork = fixture.path("sibling/fork/src/lib.rs");
    assert_eq!(
        fixture.differences(&artifact),
        [],
        "the tree is what it was built from"
    );
    fixture.write(
        "sibling/fork/src/lib.rs",
        "pub const FORK: &str = \"changed\";\n",
    );
    assert_eq!(
        fixture.differences(&artifact),
        [Input::File(fork)],
        "so a change to the fork that build compiled must refuse the artifact"
    );
}

/// The arrangement the audit measured: cargo run in the sibling workspace while
/// `PWD` names the *other* one, as a launcher that sets cargo's working directory
/// and leaves the variable inherited leaves it. Trusting `PWD` alone, the record
/// named only the workspace it named and a change to the fork the binary was
/// compiled with was reported clean.
#[test]
fn a_run_directory_that_names_another_workspace_does_not_hide_the_resolution() {
    let fixture = Fixture::new("stale");
    let build = fixture.build_in(
        "sibling",
        "elsewhere",
        &["-p", "member"],
        // `PWD` names the package's own root; cargo runs in the sibling.
        Some("own"),
    );
    assert!(
        build.status.success(),
        "the fixture builds: {}",
        String::from_utf8_lossy(&build.stderr)
    );
    let artifact = fixture.artifact("elsewhere");
    let record = fixture.record("elsewhere");
    assert!(
        record.contains("sibling/fork/src/lib.rs") && record.contains("fork/src/lib.rs"),
        "both workspaces' replacements are inputs, so neither can be missed: {record}"
    );

    let fork = fixture.path("sibling/fork/src/lib.rs");
    fixture.write(
        "sibling/fork/src/lib.rs",
        "pub const FORK: &str = \"changed\";\n",
    );
    assert_eq!(
        fixture.differences(&artifact),
        [Input::File(fork)],
        "a change to the fork the sibling's build compiled must refuse the artifact"
    );
}

/// The arrangement the audit measured, and the one that used to be reported
/// clean: cargo is told to build another workspace's manifest while it is run in
/// a third workspace and writes its build directory outside every workspace — so
/// the run directory and the build directory both name something, and neither is
/// the workspace the resolution was read in. Only cargo's own command line names
/// it. The fork that build compiled must be an input, and a change to it must
/// refuse the artifact.
#[test]
fn a_manifest_path_into_another_workspace_names_the_workspace_the_resolution_read() {
    let fixture = Fixture::new("manifest-path");
    let manifest = fixture.path("sibling/member/Cargo.toml");
    let manifest = manifest.to_str().expect("a utf-8 path");
    let build = fixture.build_in(
        "third",
        "elsewhere",
        &["--manifest-path", manifest],
        Some("third"),
    );
    assert!(
        build.status.success(),
        "the fixture builds: {}",
        String::from_utf8_lossy(&build.stderr)
    );
    let artifact = fixture.artifact("elsewhere");
    let record = fixture.record("elsewhere");
    assert!(
        record.contains("sibling/fork/src/lib.rs"),
        "the fork the resolution read is an input: {record}"
    );
    assert!(
        record.contains("third/fork/src/lib.rs"),
        "and the run directory's fork is recorded too, the union being the safe side: {record}"
    );
    assert_eq!(
        fixture.differences(&artifact),
        [],
        "the tree is what it was built from"
    );

    let fork = fixture.path("sibling/fork/src/lib.rs");
    fixture.write(
        "sibling/fork/src/lib.rs",
        "pub const FORK: &str = \"changed\";\n",
    );
    assert_eq!(
        fixture.differences(&artifact),
        [Input::File(fork)],
        "a change to the fork that build compiled must refuse the artifact"
    );
}

/// The arrangement the audit measured behind a configuration file: run in the
/// third workspace, cargo's subcommand is an *alias* whose expansion names the
/// sibling's manifest, so the command line itself carries no manifest at all —
/// `argv=["…/bin/cargo", "b", "--offline"]` — while the sibling's fork is what
/// the build compiled. A reading that stops at the command line names the third
/// workspace, and a change to the fork the binary compiled is reported clean. The
/// expansion is read where cargo reads it, so the resolution is placed rather
/// than the record marked.
#[test]
fn an_alias_naming_another_workspace_manifest_names_the_workspace_the_build_read() {
    let fixture = Fixture::new("alias");
    fixture.write(
        "third/.cargo/config.toml",
        "[alias]\nb = \"build --manifest-path ../sibling/member/Cargo.toml\"\n",
    );
    let build = fixture.alias_build("third", "elsewhere", &["b", "--offline"]);
    assert!(
        build.status.success(),
        "the fixture builds: {}",
        String::from_utf8_lossy(&build.stderr)
    );
    let artifact = fixture.artifact("elsewhere");
    let record = fixture.record("elsewhere");
    assert!(
        record.contains("sibling/fork/src/lib.rs"),
        "the fork the resolution read is an input: {record}"
    );
    assert!(
        !record.contains("resolution-not-named"),
        "the alias's expansion places the build, so the record is not marked: {record}"
    );
    assert_eq!(
        fixture.differences(&artifact),
        [],
        "the tree is what it was built from"
    );

    let fork = fixture.path("sibling/fork/src/lib.rs");
    fixture.write(
        "sibling/fork/src/lib.rs",
        "pub const FORK: &str = \"changed\";\n",
    );
    assert_eq!(
        fixture.differences(&artifact),
        [Input::File(fork)],
        "a change to the fork the alias's build compiled must refuse the artifact"
    );
}

/// A package no `[workspace]` manifest owns is its own workspace root, and cargo
/// answers that itself: `cargo locate-project --workspace` in it names its own
/// manifest, `cargo metadata` reports `workspace_root` there, and a build of it
/// reads the `[patch]` table beside that manifest. The walk used to refuse this
/// build — no nearest `[workspace]` manifest to name — so no record was written
/// and the fork the build compiled went unnamed.
#[test]
fn a_package_no_workspace_owns_is_its_own_root() {
    let fixture = Fixture::new("solo");
    let build = fixture.build("solo", "elsewhere", &[]);
    assert!(
        build.status.success(),
        "the fixture builds: {}",
        String::from_utf8_lossy(&build.stderr)
    );
    let artifact = fixture.artifact("elsewhere");
    let record = fixture.record("elsewhere");
    assert!(
        record.contains("solo/fork/src/lib.rs"),
        "the fork that workspace's `[patch]` table names is an input: {record}"
    );
    assert!(
        !record.contains("resolution-not-named"),
        "cargo names that root itself, so the record is not marked: {record}"
    );
    assert_eq!(
        fixture.differences(&artifact),
        [],
        "the tree is what it was built from"
    );

    let fork = fixture.path("solo/fork/src/lib.rs");
    fixture.write(
        "solo/fork/src/lib.rs",
        "pub const FORK: &str = \"changed\";\n",
    );
    assert_eq!(
        fixture.differences(&artifact),
        [Input::File(fork)],
        "so a change to the fork that build compiled must refuse the artifact"
    );
}

/// The triple cargo builds for when it is not told one, which is the host: the
/// only target every toolchain has installed, and the one `--target` can name
/// without a fixture needing to download anything.
fn host_triple() -> String {
    let cargo = std::env::var_os("CARGO").expect("these tests run under cargo");
    let version = Command::new(cargo)
        .arg("-vV")
        .output()
        .expect("cargo answers with its version");
    String::from_utf8_lossy(&version.stdout)
        .lines()
        .find_map(|line| line.strip_prefix("host: ").map(str::to_owned))
        .expect("cargo reports the host triple")
}

/// `--manifest-path` into a package that *is* the workspace the build directory
/// names used to be refused at check time: from inside the build its readings
/// matched a build told to build a *member* of another workspace, whose record
/// would have omitted that workspace's fork, and neither reading could tell them
/// apart. The command line now names the manifest, so both are placed and the
/// record is correct rather than refused; the stop is what remains for a build no
/// reading places at all.
#[test]
fn a_manifest_path_from_a_directory_that_owns_no_workspace_records_the_resolution() {
    let fixture = Fixture::new("named-manifest");
    let manifest = fixture.path("pkg/Cargo.toml");
    let manifest = manifest.to_str().expect("a utf-8 path");
    let build = fixture.build(".", "own/target", &["--manifest-path", manifest]);
    assert!(
        build.status.success(),
        "the fixture builds: {}",
        String::from_utf8_lossy(&build.stderr)
    );
    let artifact = fixture.artifact("own/target");
    let record = fixture.record("own/target");
    assert!(
        !record.contains("resolution-not-named"),
        "the command line placed the build, so the record is not marked: {record}"
    );
    assert_eq!(
        fixture.differences(&artifact),
        [],
        "the tree is what it was built from"
    );

    let fork = fixture.path("own/fork/src/lib.rs");
    fixture.write(
        "own/fork/src/lib.rs",
        "pub const FORK: &str = \"changed\";\n",
    );
    assert_eq!(
        fixture.differences(&artifact),
        [Input::File(fork)],
        "so a change to the fork that workspace patches with must refuse the artifact"
    );
}

/// The cargo a build runs under is asked the two things only cargo answers: the
/// workspace a run directory names (`cargo locate-project`) and the closure it
/// resolves for the package there (`cargo tree`). A `CARGO` naming no program
/// that can be started leaves both unknown, and reading either failure as
/// "nothing here" — the ancestor scan in place of the first, an empty gate in
/// place of the second — writes a record whose root list, and whose closure,
/// nothing checked. The build marks the record instead, and the guard refuses it
/// with the mark that says what to do about it.
#[test]
fn a_build_whose_cargo_cannot_be_run_marks_the_record_rather_than_guessing() {
    let fixture = Fixture::new("unaskable-cargo");
    // The package's own build script is the one place `CARGO` can be replaced by a
    // program that cannot be started: cargo sets that variable for the build script
    // itself, and this is what the stamp does with it.
    fixture.write(
        "pkg/build.rs",
        "fn main() {\n    std::env::set_var(\"CARGO\", \"/nonexistent/cargo\");\n    \
         symbiote_source_stamp::build_stamp();\n}\n",
    );
    let build = fixture.build("own", "own/target", &["-p", "pkg"]);
    assert!(
        build.status.success(),
        "the fixture builds: {}",
        String::from_utf8_lossy(&build.stderr)
    );
    let record = fixture.record("own/target");
    assert!(
        record.contains("cargo-not-asked"),
        "the cargo could not be asked, so the record says so: {record}"
    );
    assert!(
        !record.contains("resolution-not-named"),
        "and the reading that does not need cargo placed the build, so no second mark is \
         written: {record}"
    );
    let artifact = fixture.artifact("own/target");
    let problem = fixture
        .verdict(&artifact)
        .expect_err("a marked record is refused rather than compared with a tree");
    assert!(
        problem.contains("cargo-not-asked") && problem.contains("a cargo that can be started"),
        "the refusal names the mark and the rebuild that clears it: {problem}"
    );
}

/// The fields of the wire file's lines with `keyword`, in the order the file
/// spells them: a test takes a value from the one copy of the wire rather than
/// from a copy of it that could drift. A `mark` line is a role, a locator and the
/// remedy both readers print.
fn wire_lines(keyword: &str) -> Vec<Vec<String>> {
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
fn wire_mark(role: &str) -> (String, String) {
    let fields = wire_lines("mark")
        .into_iter()
        .find(|fields| fields[0] == role)
        .expect("the wire file names that mark");
    (fields[1].clone(), fields[2].clone())
}

/// The one value a scalar keyword holds.
fn wire_scalar(keyword: &str) -> String {
    wire_lines(keyword)
        .first()
        .expect("the wire file names that keyword")[0]
        .clone()
}

/// A record can carry both marks, and the check reports the one a rebuild has to
/// clear first.
///
/// One replacement loses both facts. The cargo that ran the build could not be
/// asked, which takes with it the run directory's `cargo locate-project` answer —
/// one of the readings that can name the resolved workspace; and where `CARGO` is
/// also not spelled like the cargo that ran the build, no invoking process can be
/// read either, so nothing names the resolution. `build_stamp` writes the
/// resolution's line first and the cargo's second, and the guard reports
/// `cargo-not-asked`: the resolution's remedy alone — a rebuild from the
/// workspace, where the shell reports `PWD` — leaves a cargo that cannot be asked
/// still unaskable and would be refused again. `planning/integrity/
/// source_record.py` — the second reader of a record — is then run on the very
/// artifact this build produced, and has to reach the same mark in the same
/// remedy text, which both take from the one wire file.
#[test]
fn a_build_that_can_name_neither_marks_both_and_the_cargo_is_reported() {
    let fixture = Fixture::new("both-marks");
    // Not spelled `cargo`, so the invoking process is not recognised as one, and
    // not a program, so no question can be put to it: this one replacement loses
    // both readings the two marks stand for.
    fixture.write(
        "pkg/build.rs",
        "fn main() {\n    std::env::set_var(\"CARGO\", \"/nonexistent/notcargo\");\n    \
         symbiote_source_stamp::build_stamp();\n}\n",
    );
    let build = fixture.build("own", "own/target", &["-p", "pkg"]);
    assert!(
        build.status.success(),
        "the fixture builds: {}",
        String::from_utf8_lossy(&build.stderr)
    );
    let record = fixture.record("own/target");
    assert!(
        record.contains("cargo-not-asked"),
        "the cargo could not be asked, so the record says so: {record}"
    );
    assert!(
        record.contains("resolution-not-named"),
        "and no invoking process could be read, so nothing names the resolution either: {record}"
    );
    assert!(
        record.find("resolution-not-named") < record.find("cargo-not-asked"),
        "the record writes the resolution's line before the cargo's, so the mark the refusal \
         names is not simply the first line: {record}"
    );
    let artifact = fixture.artifact("own/target");
    let problem = fixture
        .verdict(&artifact)
        .expect_err("a marked record is refused rather than compared with a tree");
    assert!(
        problem.contains("cargo-not-asked") && problem.contains("a cargo that can be started"),
        "the refusal reports the mark a rebuild has to clear first: {problem}"
    );
    assert!(
        !problem.contains("resolution-not-named"),
        "and not the mark whose remedy alone would leave the cargo unaskable: {problem}"
    );

    // The repository's own checker is the second reader, and the same artifact is
    // put to it: it must refuse it for the same mark, in the crate's own remedy
    // text, since both take that text from the one wire file.
    let checker = fixture.checker("own", &[&artifact]);
    let refusal = String::from_utf8_lossy(&checker.stderr).to_string();
    assert!(
        !checker.status.success(),
        "the Python checker refuses a marked artifact: {refusal}"
    );
    let (_, remedy) = wire_mark("unasked-cargo");
    assert!(
        problem.contains(&remedy) && refusal.contains(&remedy),
        "both readers print the remedy the wire file holds: {problem}\n{refusal}"
    );
    assert!(
        refusal.contains("cargo-not-asked") && !refusal.contains("resolution-not-named"),
        "and the checker reports the mark that blocks the rebuild, not the other: {refusal}"
    );
}

/// Every step of the rule the wire file states, read by both readers on the same
/// bytes.
///
/// Each shape is written into a binary of its own — its own line, so the shape is
/// the only thing the two readers can differ about — and put to both readers in
/// one checker run, so their verdicts have to agree on the bytes rather than only
/// in their own tests. What a shape is read as is asserted by the text only that
/// reading produces: the remedy beside the mark the line's locator names, the
/// file's remedy for a mark it does not name or for a line it cannot read at all.
/// An input is required to be read as one — the crate compares it instead of
/// refusing it, and the checker refuses it for none of the file's remedies.
#[test]
fn both_readers_read_a_record_line_by_the_wires_own_rule() {
    let fixture = Fixture::new("one-rule");
    let build = fixture.build("own", "own/target", &["-p", "pkg"]);
    assert!(
        build.status.success(),
        "the fixture builds: {}",
        String::from_utf8_lossy(&build.stderr)
    );

    let hash = "0".repeat(64);
    let (mark, remedy) = wire_mark("unasked-cargo");
    let unset = wire_scalar("unset");
    let unknown = wire_scalar("unknown");
    let malformed = wire_scalar("malformed");
    let undecodable = wire_scalar("undecodable");
    let encoding = wire_scalar("encoding");
    // A record's body, the step of the rule it exercises, and — where the line is
    // not an input — the wire text only the reading it must get produces. `None` is
    // an input: the crate's verdict is a list of differences rather than a refusal,
    // and the checker's refusal for that record holds none of the file's remedies.
    let shapes = vec![
        // 2. a locator a `mark` line names is that mark, whatever its content.
        (
            "mark-under-a-hash",
            format!("{hash}\t{mark}\n"),
            Some(vec![mark.clone(), remedy.clone()]),
        ),
        (
            "mark-under-unset",
            format!("{unset}\t{mark}\n"),
            Some(vec![mark.clone(), remedy.clone()]),
        ),
        // 1. a line with no TAB, or no locator, is malformed — a blank line too,
        // which both readers used to pass over.
        (
            "no-tab",
            "no tab at all\n".to_owned(),
            Some(vec![malformed.clone()]),
        ),
        (
            "blank-line",
            format!("{hash}\tsrc/lib.rs\n\n"),
            Some(vec![malformed.clone()]),
        ),
        (
            "blank-record",
            "\n".to_owned(),
            Some(vec![malformed.clone()]),
        ),
        (
            "no-locator",
            format!("{unset}\t\n"),
            Some(vec![malformed.clone()]),
        ),
        // A CR is not part of a locator: measured, a CRLF record was read as inputs
        // whose locators each ended in a CR, so a line carried the CR instead of
        // the record being refused for one.
        (
            "carriage-return",
            format!("{hash}\tsrc/lib.rs\r\n"),
            Some(vec![malformed.clone()]),
        ),
        // 6. a content that is neither `unset` nor a hash says neither what the
        // record read nor that it cannot be stood behind.
        (
            "not-a-content",
            "abc\tsrc/lib.rs\n".to_owned(),
            Some(vec![malformed.clone()]),
        ),
        // 3. a locator spelled with `prefix` is an input, hash or `unset` alike.
        ("environment-hash", format!("{hash}\tenv:CARGO\n"), None),
        ("environment-unset", format!("{unset}\tenv:CARGO\n"), None),
        // 5. any other locator whose content is a hash is an input.
        ("file-hash", format!("{hash}\tsrc/lib.rs\n"), None),
        // 4. any other locator whose content is `unset` is a mark the file does
        // not name.
        (
            "unknown-mark",
            format!("{unset}\tzzz-old\n"),
            Some(vec![String::from("zzz-old"), unknown.clone()]),
        ),
    ];

    let start = wire_scalar("start");
    let end = wire_scalar("end");
    let mut artifacts = Vec::new();
    for (name, body, _) in &shapes {
        // Only the framing and the record between it are read by either reader, so
        // a shape needs no build of its own; the file lives in the deps directory
        // the checker is pointed at, which is where it looks for a `pkg:` artifact.
        let artifact = fixture.path(&format!("own/target/debug/deps/{name}"));
        std::fs::write(&artifact, format!("ELF\x00{start}{body}{end}"))
            .expect("the artifact carrying the record");
        artifacts.push(artifact);
    }
    // The wire states the encoding of the bytes it frames, so a record that is not
    // it is a shape of its own: bytes that decode as nothing, inside a line of the
    // record the wire frames. Both readers have to refuse it in the file's own
    // remedy, naming the byte — measured, one raised over such an artifact while
    // the other reported it as carrying no record.
    let prefix = format!("{hash}\tsrc/lib.rs\n{hash}\tsrc/");
    let mut bytes = format!("ELF\x00{start}{prefix}").into_bytes();
    let offset = prefix.len();
    bytes.extend_from_slice(&[0xff, 0xfe]);
    bytes.extend_from_slice(format!("rs\n{end}").as_bytes());
    let unreadable = fixture.path("own/target/debug/deps/not-utf8");
    std::fs::write(&unreadable, bytes).expect("the artifact carrying the record");
    // And the framing itself: a record whose own bytes spell the end marker is one
    // no build writes, since a build refuses to stamp a locator that spells one,
    // and one the wire cannot frame — so both readers have to refuse it rather
    // than read it up to that marker and drop every line after it, which is what
    // both used to do with such an artifact.
    let unframed = fixture.path("own/target/debug/deps/spells-a-marker");
    let body = format!("{hash}\tsrc/a.rs{end}\n{hash}\tsrc/b.rs\n");
    std::fs::write(&unframed, format!("ELF\x00{start}{body}{end}"))
        .expect("the artifact carrying the record");

    let mut checkable = artifacts.clone();
    checkable.push(unreadable.clone());
    checkable.push(unframed.clone());
    let paths: Vec<&Path> = checkable.iter().map(PathBuf::as_path).collect();
    let checker = fixture.checker("own", &paths);
    let refusal = String::from_utf8_lossy(&checker.stderr).to_string();
    assert!(
        !checker.status.success(),
        "the checker refuses every shape a record cannot be read for: {refusal}"
    );
    for ((name, _, expected), artifact) in shapes.into_iter().zip(&artifacts) {
        let reported = refusal
            .lines()
            .find(|line| line.contains(&format!("deps/{name}:")))
            .unwrap_or_else(|| panic!("the checker reports {name}: {refusal}"));
        let by_crate = changed_sources(artifact, &fixture.path("pkg"));
        match expected {
            Some(tokens) => {
                let problem = by_crate.expect_err("both readers refuse this line");
                for token in &tokens {
                    assert!(
                        problem.contains(token.as_str()) && reported.contains(token.as_str()),
                        "both readers read {name} as the wire says: {problem}\n{reported}"
                    );
                }
            }
            None => {
                assert!(
                    by_crate.is_ok(),
                    "the crate reads {name} as an input and compares it: {by_crate:?}"
                );
                for token in [remedy.as_str(), unknown.as_str(), malformed.as_str()] {
                    assert!(
                        !reported.contains(token),
                        "and the checker refuses {name} for no mark and no unread line: {reported}"
                    );
                }
            }
        }
    }

    let reported = refusal
        .lines()
        .find(|line| line.contains("deps/not-utf8:"))
        .unwrap_or_else(|| panic!("the checker reports the record it cannot decode: {refusal}"))
        .to_owned();
    let by_crate = changed_sources(&unreadable, &fixture.path("pkg"))
        .expect_err("a record whose bytes are not the encoding cannot be decoded");
    // The whole refusal is the file's, down to where its offset is spelled: the
    // proof fills the file's own `{byte}` and `{encoding}` and requires both
    // readers to print exactly that, so a reader that reworded any of it — or
    // appended the offset in its own words — fails here.
    let expected = undecodable
        .replace("{encoding}", &encoding)
        .replace("{byte}", &offset.to_string());
    assert!(
        undecodable.contains("{byte}") && undecodable.contains("{encoding}"),
        "the file states where the offset and the encoding are spelled, so no reader holds that \
         text: {undecodable}"
    );
    // The whole line each reader prints, compared rather than contained, so a reader
    // that reworded any of it — or appended the offset in words of its own, which is
    // what the two used to do — fails here rather than passing on a prefix.
    assert_eq!(
        by_crate,
        format!(
            "the binary at {} carries a record that cannot be read: {expected}",
            unreadable.display()
        ),
        "the crate prints the file's own text for a record that is not {encoding}"
    );
    assert_eq!(
        reported,
        format!("PROBLEM: deps/not-utf8: {expected}"),
        "and so does the checker, at the same offset, with nothing added by either"
    );

    let reported = refusal
        .lines()
        .find(|line| line.contains("deps/spells-a-marker:"))
        .unwrap_or_else(|| panic!("the checker reports the record it cannot frame: {refusal}"))
        .to_owned();
    let unframed_remedy = wire_scalar("unframed");
    let by_crate = changed_sources(&unframed, &fixture.path("pkg"))
        .expect_err("a record whose own bytes spell a marker cannot be framed");
    for verdict in [&by_crate, &reported] {
        assert!(
            verdict.contains(&unframed_remedy),
            "both readers refuse a record that spells one of its own framing markers in the file's \
             own remedy, rather than reading it up to that marker and short: {by_crate}\n{reported}"
        );
    }
}

/// A build refuses to stamp a locator its own record could not survive — and the
/// workspace that holds none of them still round-trips.
///
/// Three things in a path make the record it would be named in unreadable: the LF
/// its lines are divided at, the CR its rule reads as malformed, and a marker its
/// framing rests on. Measured, a package holding a path with a line feed in it
/// built successfully and stamped a record both readers then refused, in a remedy
/// asking for a rebuild that reproduces the same bytes — so a workspace could
/// never pass its own guard. The build stops instead, naming the path it found and
/// what that path holds; and with no such path, the same workspace builds and both
/// readers accept what it stamps, so the refusal is the only thing standing
/// between a build and bytes its readers cannot read.
#[test]
fn a_build_refuses_a_locator_its_own_record_could_not_survive() {
    let fixture = Fixture::new("unstampable");
    let marker = wire_scalar("end");
    // Each path, the parts of it the refusal has to name back — split around the
    // byte it holds, since cargo indents the continuation of a build script's own
    // message at each LF — and what the rule says that path holds.
    let cases = [
        (
            format!("pkg/src/a{}b.rs", '\n'),
            ("pkg/src/a", "b.rs", "a line feed"),
        ),
        (
            format!("pkg/src/c{}d.rs", '\r'),
            ("pkg/src/c", "d.rs", "a carriage return"),
        ),
        (
            format!("pkg/src/m{marker}"),
            ("pkg/src/m", marker.as_str(), marker.as_str()),
        ),
    ];
    for (path, (before, after, holds)) in &cases {
        fixture.write(path, "// a path a record cannot hold\n");
        let build = fixture.build("own", "own/target", &["-p", "pkg"]);
        assert!(
            !build.status.success(),
            "the build stops rather than write a record it cannot read back: {path:?}"
        );
        let stderr = String::from_utf8_lossy(&build.stderr);
        assert!(
            stderr.contains("cannot record the sources under test")
                && stderr.contains(before)
                && stderr.contains(after)
                && stderr.contains(holds),
            "the failure names the path it found ({path:?}) and what that path holds ({holds}): \
             {stderr}"
        );
        std::fs::remove_file(fixture.path(path)).expect("the offending path");
    }

    // The round trip: with nothing a record cannot hold, the build succeeds and
    // both readers accept the bytes it stamped.
    let build = fixture.build("own", "own/target", &["-p", "pkg"]);
    assert!(
        build.status.success(),
        "the fixture builds once the offending paths are gone: {}",
        String::from_utf8_lossy(&build.stderr)
    );
    let artifact = fixture.artifact("own/target");
    assert_eq!(
        fixture.differences(&artifact),
        Vec::new(),
        "the crate reads back the record of the build that had nothing to refuse"
    );
    let checker = fixture.checker("own", &[&artifact]);
    assert!(
        checker.status.success(),
        "and the checker accepts the same bytes: {}",
        String::from_utf8_lossy(&checker.stderr)
    );
}
