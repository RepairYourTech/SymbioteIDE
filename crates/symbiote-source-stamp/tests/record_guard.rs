//! The guard's decisive shapes, driven through real cargo builds.
//!
//! Every shape here was found by an audit, reproduced by hand in a scratch
//! workspace, and fixed — and then the scratch workspace was thrown away, so
//! what held the fix in place afterwards was a unit test on a synthetic tree
//! plus a paragraph of prose. These tests are the same proof, in the suite: a
//! real cargo builds a fixture, and both readers of the record it stamps are put
//! to the bytes it produced.
//!
//! Four things are proved here that no unit test can:
//!
//! * a file the *compiler* found in a directory the walk skips is named by the
//!   record, because a declaration reaches it — measured on a build rather than
//!   on a tree the test assembled;
//! * a record a build wrote, with one line removed, is refused — the shape a
//!   build would have written had it not read that file, and the one shape no
//!   reading of the bytes alone can tell from a build's own;
//! * a build driven into a directory of its own inside the workspace is
//!   complete, though the record cannot name the files cargo wrote under it;
//! * two packages whose units carry one crate name are told apart by cargo's own
//!   identity for them, not by the name their dep-info files share.
//!
//! They are hermetic: a fresh directory under the temporary directory per case,
//! every cargo invocation `--offline`, no fixture outside those directories, and
//! no dependency but this crate. They need the cargo that is running them
//! (`CARGO`) and `python3`, which the second reader is written in — the same two
//! things the entry-point tests need — and the scaffolding they share with those
//! tests lives in `support`, so neither harness can drift from the other's
//! mechanics.
//!
//! [`changed_sources`]: symbiote_source_stamp::changed_sources

mod support;

use std::path::{Path, PathBuf};

use support::{Workspace, wire_scalar};
use symbiote_source_stamp::Input;

/// The fixture the cases build from.
///
/// ```text
/// ws/                     the workspace both readers are pointed at
/// ws/probe/               the driven package: a build script that stamps, a
///                         binary that carries the record, a module under a
///                         directory name the walk skips (`src/target/`), and a
///                         module the build script declares under one (`tests/`,
///                         which the wire file skips at a package root alone)
/// ws/probe/tests/mod.rs   a name cargo looks for target directories under
/// ws/probe/src/lib.rs     a second unit of the driven package, whose dep-info
///                         carries the same crate name as the binary's
/// ws/other/               a package whose library carries the driven binary's
///                         crate name, so the two units' dep-info files differ in
///                         nothing but the metadata hash
/// ```
struct Fixture {
    workspace: Workspace,
}

impl Fixture {
    /// The fixture: its cases' workspace under a fresh temporary directory named
    /// after `case`, and the files below.
    fn new(case: &str) -> Self {
        let fixture = Self {
            workspace: Workspace::new(case),
        };
        let stamp = format!("{:?}", env!("CARGO_MANIFEST_DIR"));
        fixture.write(
            "ws/Cargo.toml",
            "[workspace]\nmembers = [\"probe\", \"other\"]\nresolver = \"2\"\n",
        );
        fixture.write(
            "ws/probe/Cargo.toml",
            &format!(
                "[package]\nname = \"probe\"\nversion = \"0.0.0\"\nedition = \"2021\"\n\n\
                 [build-dependencies]\nsymbiote-source-stamp = {{ path = {stamp} }}\n"
            ),
        );
        // The build script declares a module in the directory cargo looks for
        // integration tests in, which the wire file skips at a package root
        // alone. A build script is a crate like any other, so `mod tests;` is
        // how a build reads a file no walk of that name reaches.
        fixture.write(
            "ws/probe/build.rs",
            "mod tests;\n\nfn main() {\n    let _ = tests::DECLARED;\n    \
             symbiote_source_stamp::build_stamp();\n}\n",
        );
        fixture.write(
            "ws/probe/tests/mod.rs",
            "pub const DECLARED: &str = \"a module the build script declares\";\n",
        );
        // And the binary declares one in a directory the walk skips at every
        // depth, then carries the record its build stamped — read in `main`
        // rather than merely declared, the way a shipped binary carries it,
        // because a static nothing reads is one the linker drops.
        fixture.write(
            "ws/probe/src/main.rs",
            "include!(concat!(env!(\"OUT_DIR\"), \"/source_record.rs\"));\n\n\
             mod target;\n\n\
             fn main() {\n    std::hint::black_box(SOURCE_RECORD);\n    \
             println!(\"{}\", target::SKIPPED);\n}\n",
        );
        // A second unit of the same package, in the same crate name as the
        // binary: cargo builds it alongside the binary, and its dep-info is
        // spelled the same way, so it is what a rule reading only the package or
        // the crate name would take for the binary's own evidence.
        fixture.write(
            "ws/probe/src/lib.rs",
            "pub const LIBRARY: &str = \"a second unit of the driven package\";\n",
        );
        fixture.write(
            "ws/probe/src/target/mod.rs",
            "pub const SKIPPED: &str = \"a module under a name the walk skips\";\n",
        );
        // A second package, in the same workspace and in no dependency of the
        // driven one, whose *library* carries the driven binary's crate name: the
        // two units' dep-info files are spelled alike and differ only in the
        // metadata hash, so nothing but cargo's own identity for a unit can tell
        // them apart.
        fixture.write(
            "ws/other/Cargo.toml",
            "[package]\nname = \"other\"\nversion = \"0.0.0\"\nedition = \"2021\"\n\n\
             [lib]\nname = \"probe\"\npath = \"src/lib.rs\"\n",
        );
        fixture.write(
            "ws/other/src/lib.rs",
            "pub const OTHER: &str = \"a unit of another package\";\n",
        );
        fixture
    }

    fn write(&self, relative: &str, contents: &str) {
        self.workspace.write(relative, contents);
    }

    fn path(&self, relative: &str) -> PathBuf {
        self.workspace.path(relative)
    }

    /// Cargo's build of the fixture, `args` beside `build --offline`, run in
    /// `run_from` with the build directory `target` — a directory of the
    /// fixture's own, which is what makes a check of it a check of the directory
    /// it was given rather than of a name.
    fn build(&self, run_from: &str, target: &str, args: &[&str]) -> std::process::Output {
        self.workspace.build(run_from, target, args)
    }

    /// The metadata hash of the unit that compiles `package`'s target
    /// `fingerprint` — `bin-probe` for a binary, `lib-probe` for a library.
    ///
    /// Cargo keeps one fingerprint directory per unit, named after the package
    /// that compiles it and the unit's metadata hash, and holding one file per
    /// target that unit compiles. That hash is the one the unit's dep-info is
    /// spelled with, so this is how a test names the artifact and the dep-info of
    /// the unit it means rather than guessing at a file name: measured, a
    /// package's binary and another package's same-named library write dep-info
    /// files that differ in nothing but this hash.
    fn unit_hash(&self, target: &str, package: &str, fingerprint: &str) -> String {
        let directory = self.path(target).join("debug/.fingerprint");
        for entry in std::fs::read_dir(&directory)
            .expect("cargo wrote its fingerprints")
            .flatten()
        {
            let name = entry.file_name().to_string_lossy().to_string();
            let Some(hash) = name.strip_prefix(&format!("{package}-")) else {
                continue;
            };
            if entry.path().join(fingerprint).is_file() {
                return hash.to_string();
            }
        }
        panic!("cargo wrote a fingerprint for {package}'s {fingerprint} in {directory:?}")
    }

    /// The record the fixture's build wrote, as the lines it holds.
    fn record(&self, target: &str) -> String {
        self.workspace.record(target)
    }

    /// The guard's verdict for `artifact`: the inputs whose recorded content no
    /// longer matches the tree, or the reason the record cannot be checked. The
    /// cases here drive artifacts whose record must stand behind the tree, so a
    /// refusal is a failure of the fixture rather than a verdict.
    fn differences(&self, artifact: &Path) -> Vec<Input> {
        self.workspace.differences(artifact, "ws/probe")
    }

    /// The repository's own checker, run on `binaries` — `(package, path under
    /// the profile directory)` pairs — the way CI runs it.
    ///
    /// This crate is the first reader of a record and the Python tool the second,
    /// so the two agree about an artifact only if both are put to it: what a
    /// build's record has to name is the checker's question, and the shapes here
    /// are refused or accepted by it rather than by this crate.
    fn checker(&self, profile: &str, binaries: &[(&str, &str)]) -> std::process::Output {
        let mut command = self.workspace.checker("ws", &self.workspace.path(profile));
        for (package, binary) in binaries {
            command.arg("--binary").arg(format!("{package}:{binary}"));
        }
        command
            .output()
            .expect("the integrity checker runs (it needs python3)")
    }

    /// `bytes` with the record's line naming `locator` removed.
    ///
    /// The record a build would have written had it not read that file — the
    /// short record every completeness rule exists to catch, and the one shape a
    /// reader cannot tell from a build's own by reading the record alone: every
    /// line left is well formed, and the framing is untouched.
    fn without_line(&self, bytes: &[u8], locator: &str) -> Vec<u8> {
        let start = wire_scalar("start").into_bytes();
        let end = wire_scalar("end").into_bytes();
        let opened = find(bytes, &start).expect("the record's framing") + start.len();
        let closed = opened + find(&bytes[opened..], &end).expect("the record's end");
        let body = &bytes[opened..closed];
        let lines: Vec<&[u8]> = body.split(|byte| *byte == b'\n').collect();
        let kept: Vec<&[u8]> = lines
            .iter()
            .copied()
            .filter(|line| line.rsplit(|byte| *byte == b'\t').next() != Some(locator.as_bytes()))
            .collect();
        assert_eq!(
            kept.len(),
            lines.len() - 1,
            "the record names {locator} exactly once"
        );
        let mut forged = bytes[..opened].to_vec();
        forged.extend(kept.join(&b'\n'));
        forged.extend_from_slice(&bytes[closed..]);
        forged
    }
}

/// The first index of `needle` in `haystack`.
fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

/// A module the compiler finds in a directory the walk skips is in the record.
///
/// Both declared files here sit under a name the wire file's `exclusion` lines
/// skip: `src/target/mod.rs` under a name skipped at every depth, and
/// `tests/mod.rs` under one skipped at a package root alone, which is where
/// cargo looks for a target directory of that name and where a build script's
/// `mod tests;` reaches it.
///
/// Measured before the scan followed declarations: `mod target;` compiling
/// `src/target/mod.rs` and a build script's `mod tests;` compiling
/// `tests/mod.rs` were both compiled into the binary and named by nothing, while
/// both readers reported the binary current. A build's own record is what this
/// test asserts on, so a scan that stopped following a declaration fails here —
/// the record stops naming the file, and the checker, which requires the record
/// to name every input its units read, refuses it.
#[test]
fn a_module_under_a_name_the_walk_skips_is_named_by_the_record() {
    let fixture = Fixture::new("declared");
    let build = fixture.build("ws", "ws/target", &["-p", "probe"]);
    assert!(
        build.status.success(),
        "the fixture builds: {}",
        String::from_utf8_lossy(&build.stderr)
    );
    let record = fixture.record("ws/target");
    for locator in ["probe/src/target/mod.rs", "probe/tests/mod.rs"] {
        assert!(
            record.contains(locator),
            "the build compiled {locator}, so the declaration that reaches it names it: {record}"
        );
    }

    let hash = fixture.unit_hash("ws/target", "probe", "bin-probe");
    let binary = format!("deps/probe-{hash}");
    let artifact = fixture.path(&format!("ws/target/debug/{binary}"));
    assert_eq!(
        fixture.differences(&artifact),
        Vec::new(),
        "the tree is what it was built from"
    );
    let checker = fixture.checker("ws/target/debug", &[("probe", binary.as_str())]);
    assert!(
        checker.status.success(),
        "the checker requires the record to name both declared files, so it is the \
         reader that refuses a record the scan left short: {}",
        String::from_utf8_lossy(&checker.stderr)
    );

    // And the record covers the file rather than merely mentioning it: a change
    // to the module under the skipped name is a difference.
    let skipped = fixture.path("ws/probe/src/target/mod.rs");
    fixture.write(
        "ws/probe/src/target/mod.rs",
        "pub const SKIPPED: &str = \"changed\";\n",
    );
    assert_eq!(
        fixture.differences(&artifact),
        [Input::File(skipped)],
        "a change to a module the compiler found by name must refuse the artifact"
    );
}

/// A record with a line removed is refused, and only the checker can refuse it.
///
/// The file removed here is one the build genuinely read — `tests/mod.rs`,
/// declared by the build script — and one the checker used to excuse by name:
/// measured, removing that line left the checker reporting the record clean and
/// this crate's reader reporting the binary current, so a short record passed
/// both readers. The check now excuses nothing by name, and this is the shape
/// that fails if that returns: the refusal has to name the file, and it has to
/// be a refusal for a missing name rather than for a line the wire cannot read.
///
/// The crate's own reader still accepts the bytes, and that is its documented
/// division of labour rather than a defect: it compares the content of the files
/// a record names, so a record naming fewer files is a record whose named files
/// are unchanged. Only the checker, which holds cargo's dep-info of what the
/// build read, can see that a name is missing.
#[test]
fn a_record_that_leaves_out_a_file_the_build_read_is_refused() {
    let fixture = Fixture::new("short-record");
    let build = fixture.build("ws", "ws/target", &["-p", "probe"]);
    assert!(
        build.status.success(),
        "the fixture builds: {}",
        String::from_utf8_lossy(&build.stderr)
    );
    let hash = fixture.unit_hash("ws/target", "probe", "bin-probe");
    let binary = format!("deps/probe-{hash}");
    let artifact = fixture.path(&format!("ws/target/debug/{binary}"));
    let original = std::fs::read(&artifact).expect("the artifact reads");
    let checker = fixture.checker("ws/target/debug", &[("probe", binary.as_str())]);
    assert!(
        checker.status.success(),
        "the fixture's own record is complete: {}",
        String::from_utf8_lossy(&checker.stderr)
    );

    let locator = "probe/tests/mod.rs";
    std::fs::write(&artifact, fixture.without_line(&original, locator))
        .expect("the forged artifact");
    assert_eq!(
        fixture.differences(&artifact),
        Vec::new(),
        "the crate compares the files a record names, so a name removed is not its question"
    );
    let forged = fixture.checker("ws/target/debug", &[("probe", binary.as_str())]);
    let refusal = String::from_utf8_lossy(&forged.stderr).to_string();
    assert!(
        !forged.status.success(),
        "the checker refuses a record that leaves out a file the build read: {refusal}"
    );
    assert!(
        refusal.contains(locator),
        "and names the file the build read: {refusal}"
    );
    assert!(
        !refusal.contains(&wire_scalar("malformed")),
        "refused for the name it leaves out, not for a line it cannot read: {refusal}"
    );

    // The control: with the line back, the same bytes are complete again, so what
    // the refusal read is the line and not something else about the forgery.
    std::fs::write(&artifact, original).expect("the artifact restored");
    let restored = fixture.checker("ws/target/debug", &[("probe", binary.as_str())]);
    assert!(
        restored.status.success(),
        "the build's own record is accepted again: {}",
        String::from_utf8_lossy(&restored.stderr)
    );
}

/// A build driven into a directory of its own inside the workspace is complete.
///
/// `CARGO_TARGET_DIR=<workspace>/build-output` is a legitimate build of the same
/// sources, and the record it stamps cannot name the files cargo wrote under that
/// directory — they are the build's own output, and the record names the build
/// script that wrote them. So a check has to excuse them by the directory it was
/// *given* rather than by a name, and this is the shape that fails if a name
/// returns: measured on this workspace, a build driven into
/// `<workspace>/build-output` was refused with one problem for every file cargo
/// wrote there, 353 of them, because the rule read the name `target`.
#[test]
fn a_build_driven_into_its_own_directory_inside_the_workspace_is_checked_ok() {
    let fixture = Fixture::new("own-output-directory");
    let build = fixture.build("ws", "ws/build-output", &["-p", "probe"]);
    assert!(
        build.status.success(),
        "the fixture builds into a directory of its own: {}",
        String::from_utf8_lossy(&build.stderr)
    );
    let record = fixture.record("ws/build-output");
    let hash = fixture.unit_hash("ws/build-output", "probe", "bin-probe");
    let binary = format!("deps/probe-{hash}");
    let artifact = fixture.path(&format!("ws/build-output/debug/{binary}"));
    assert_eq!(
        fixture.differences(&artifact),
        Vec::new(),
        "the tree is what it was built from"
    );

    assert!(
        !record.contains("build-output"),
        "the record cannot name the files cargo wrote under that directory — they are the \
         build's own output, and the record names the build script that wrote them — so an \
         acceptance cannot be the record naming them: {record}"
    );
    let checker = fixture.checker("ws/build-output/debug", &[("probe", binary.as_str())]);
    let verdict = String::from_utf8_lossy(&checker.stderr).to_string();
    assert!(
        checker.status.success(),
        "the build is the same build wherever its output went, so nothing here is \
         evidence the record is short: {verdict}"
    );
}

/// Neither another package's unit nor another unit of this package is evidence.
///
/// Three units in this fixture carry the crate name `probe`: the driven binary,
/// a library of the same package, and a library of a package no build of the
/// driven binary reaches. Their dep-info files are spelled alike and differ only
/// in the metadata hash, and cargo's own fingerprint is the only thing that
/// places each in the package that compiles it. Two faces are asserted, because
/// each was a different defect:
///
/// * with every unit present, the driven binary is complete: another package's
///   unit is not one of this binary's closure, and its source is a file this
///   record does not name. Measured before the identity came from cargo's
///   fingerprint, that unit was credited to the driven package and the check
///   refused the binary over another package's source.
/// * with the driven unit's own dep-info gone — what `cargo clean -p probe`
///   leaves — the check refuses, because no other unit's reads are evidence of
///   what *this* binary's build read. Measured before a green had to rest on the
///   producing unit, the same package's library satisfied the rule and the
///   record was called complete having never been compared with this build; on
///   this workspace 45 units satisfied it while exactly one is the binary.
#[test]
fn a_unit_of_another_package_is_not_evidence_for_this_binary() {
    let fixture = Fixture::new("collision");
    let build = fixture.build("ws", "ws/target", &["-p", "probe", "-p", "other"]);
    assert!(
        build.status.success(),
        "the fixture builds both packages: {}",
        String::from_utf8_lossy(&build.stderr)
    );
    let hash = fixture.unit_hash("ws/target", "probe", "bin-probe");
    let sibling = fixture.unit_hash("ws/target", "probe", "lib-probe");
    let foreign = fixture.unit_hash("ws/target", "other", "lib-probe");
    let binary = format!("deps/probe-{hash}");
    let own_dep_info = fixture.path(&format!("ws/target/debug/deps/probe-{hash}.d"));
    let sibling_dep_info = fixture.path(&format!("ws/target/debug/deps/probe-{sibling}.d"));
    let foreign_dep_info = fixture.path(&format!("ws/target/debug/deps/probe-{foreign}.d"));
    assert!(
        own_dep_info.is_file() && sibling_dep_info.is_file() && foreign_dep_info.is_file(),
        "this package's two units and the other package's one are spelled alike: \
         {own_dep_info:?}, {sibling_dep_info:?}, {foreign_dep_info:?}"
    );

    let checker = fixture.checker("ws/target/debug", &[("probe", binary.as_str())]);
    assert!(
        checker.status.success(),
        "another package's unit is not one of this binary's closure: {}",
        String::from_utf8_lossy(&checker.stderr)
    );

    std::fs::remove_file(&own_dep_info).expect("the driven unit's own dep-info");
    let refused = fixture.checker("ws/target/debug", &[("probe", binary.as_str())]);
    let verdict = String::from_utf8_lossy(&refused.stderr).to_string();
    assert!(
        sibling_dep_info.is_file() && foreign_dep_info.is_file(),
        "a unit of this package and one of another are both there to be taken for the \
         evidence: {sibling_dep_info:?}, {foreign_dep_info:?}"
    );
    assert!(
        !refused.status.success(),
        "a same-named unit of another package is not evidence of what this binary's build \
         read, so a record nothing was compared with is refused: {verdict}"
    );
    assert!(
        verdict.contains("holds no dep-info for the unit that builds it"),
        "and it is refused for the evidence being absent rather than (as before a green had to \
         rest on the producing unit) for a name some other unit's reads were credited with: \
         {verdict}"
    );
}
