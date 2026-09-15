//! The walk's own tests: the closure it reaches and names, the roots a record is
//! written against, the readings that place a build, and the questions it asks
//! cargo — `roots`, `invocation` and `asking` are what the walk is made of, and
//! these drive them through it.

use super::*;

use crate::tests::{edges, fixture, patches, record_over};
use crate::{Input, changed_sources};

use super::asking::{Answer, gate_against_cargo, paths_in, resolved_packages};
use super::invocation::{Invoking, READS_INVOKING_PROCESS, invocation_roots};
use super::paths::canonical;
use super::roots::{workspace_root, workspace_roots};

fn candidate_roots(out_dir: &Path, run_from: &[PathBuf]) -> Vec<PathBuf> {
    invocation_roots(out_dir, run_from, None).roots
}

#[test]
fn a_patch_is_read_from_the_manifest_and_applies_where_the_walk_places_it() {
    // Measured: cargo ignores a member's `[patch]` table whole — it warns
    // `patch for the non root package will be ignored`, exits 0, and resolves the
    // dependency from its own path even when the fork the patch names is there —
    // while a root's patch applies, and so does one in a package no `[workspace]`
    // manifest owns, which is its own root. Which directories those are is a fact
    // about a *build* rather than about one manifest, so the reader reports the
    // table and the walk decides (`a_member_patch_is_not_a_recorded_input`,
    // `a_root_patch_is_recorded_for_a_member_that_takes_it`).
    let member = "\
[dependencies]
dep = { path = \"../dep\" }

[patch.crates-io]
serde = { path = \"../fork\" }

[patch.\"https://github.com/a.b\".other]
path = \"../other\"

[patch.crates-io.replaced]
path = \"../forked\"
";
    assert_eq!(
        edges(member),
        [("../dep".to_owned(), false)],
        "a patch table is not a dependency edge of the manifest that declares it"
    );
    assert_eq!(
        patches(member),
        [
            ("../fork".to_owned(), true),
            ("../other".to_owned(), true),
            ("../forked".to_owned(), true),
        ],
        "a patch is read in both spellings, and each is conditional: a fork that is not there is \
         skipped, because a patch a build never read may name a directory that is gone"
    );
    assert_eq!(
        patches(&format!("[workspace]\n{member}")),
        patches(member),
        "whether a `[workspace]` table sits beside the patch changes which builds read it, not \
         what the manifest declares"
    );
}

#[test]
fn an_edge_is_walked_however_the_manifest_declares_it() {
    // One edge, spelled three ways cargo compiles identically — measured
    // with `cargo metadata`, which resolves `dep.path = "../dep"` to the
    // same package the inline table names. The walk read only the first, so
    // the dependency's sources were named by no record and a binary built
    // before a change to them was reported clean.
    for (spelling, declaration) in [
        ("inline", "[dependencies]\ndep = { path = \"../dep\" }\n"),
        ("dotted", "[dependencies]\ndep.path = \"../dep\"\n"),
        ("table", "[dependencies.dep]\npath = \"../dep\"\n"),
    ] {
        let root = fixture(&format!("declared-{spelling}"));
        let package = root.join("crates/app");
        let dependency = root.join("crates/dep");
        std::fs::create_dir_all(package.join("src")).expect("the package");
        std::fs::create_dir_all(dependency.join("src")).expect("the dependency");
        std::fs::write(
            root.join("Cargo.toml"),
            "[workspace]\nmembers = [\"crates/app\", \"crates/dep\"]\n",
        )
        .expect("the workspace manifest");
        std::fs::write(
            package.join("Cargo.toml"),
            format!("[package]\nname = \"app\"\n\n{declaration}"),
        )
        .expect("the package manifest");
        std::fs::write(package.join("src/lib.rs"), "pub fn nothing() {}\n").expect("the source");
        std::fs::write(dependency.join("Cargo.toml"), "[package]\nname = \"dep\"\n")
            .expect("the dependency manifest");
        let compiled = dependency.join("src/lib.rs");
        std::fs::write(&compiled, "pub const ONE: u8 = 1;\n").expect("the dependency source");

        let (sources, unfollowed, _) = recorded_sources(&package, &root, &[]);
        assert!(unfollowed.is_empty(), "{unfollowed:?}");
        assert!(
            sources.contains(&compiled),
            "a package this binary compiles must have its sources recorded, however the \
                 manifest declares the edge; the {spelling} walk found {sources:?}"
        );

        let binary = record_over(&root, &sources);
        std::fs::write(&compiled, "pub const ONE: u8 = 2;\n").expect("the changed dependency");
        assert_eq!(
            changed_sources(&binary, &root).expect("a readable record"),
            [Input::File(compiled)],
            "the {spelling} record must name the changed dependency"
        );
    }
}

#[test]
fn a_root_patch_whose_path_is_gone_does_not_stop_the_build() {
    // A root cargo read must hold the directory its patch names — measured, the
    // build stops with exit 101 before a record is written — while a candidate
    // root the build never read may name one that is gone, so a patch edge is
    // followed only where the directory is there.
    let root = fixture("absent-patch");
    let workspace = root.join("ws");
    let package = workspace.join("app");
    let dependency = workspace.join("dep");
    std::fs::create_dir_all(package.join("src")).expect("the package");
    std::fs::create_dir_all(dependency.join("src")).expect("the dependency");
    std::fs::write(
        workspace.join("Cargo.toml"),
        "[workspace]\nmembers = [\"app\", \"dep\"]\n\n\
             [patch.crates-io]\ndep = { path = \"gone\" }\n",
    )
    .expect("the root whose patch names a directory that is not there");
    std::fs::write(
        package.join("Cargo.toml"),
        "[package]\nname = \"app\"\n\n[dependencies]\ndep = { path = \"../dep\" }\n",
    )
    .expect("the member manifest");
    std::fs::write(package.join("src/lib.rs"), "pub fn nothing() {}\n").expect("the source");
    std::fs::write(dependency.join("Cargo.toml"), "[package]\nname = \"dep\"\n")
        .expect("the dependency manifest");
    std::fs::write(dependency.join("src/lib.rs"), "pub const ONE: u8 = 1;\n")
        .expect("the dependency source");

    let (sources, unfollowed, _) = recorded_sources(&package, &workspace, &[]);
    assert!(unfollowed.is_empty(), "{unfollowed:?}");
    assert!(
        sources.contains(&dependency.join("src/lib.rs"))
            && sources.contains(&package.join("src/lib.rs")),
        "the edges cargo does resolve are still recorded; the walk found {sources:?}"
    );
}

#[test]
fn a_member_patch_is_not_a_recorded_input() {
    // Measured: a workspace whose *member* patches a dependency exits 0 with
    // `patch for the non root package will be ignored`, and the fork is not in
    // the resolved graph even though its directory is there. Recording it would
    // refuse a binary for a change no rebuild reads — the same wrong input the
    // walk refuses to record for a dev-dependency.
    let root = fixture("member-patch");
    let workspace = root.join("ws");
    let package = workspace.join("app");
    let dependency = workspace.join("dep");
    let fork = workspace.join("fork");
    std::fs::create_dir_all(package.join("src")).expect("the package");
    std::fs::create_dir_all(dependency.join("src")).expect("the dependency");
    std::fs::create_dir_all(fork.join("src")).expect("the fork");
    std::fs::write(
        workspace.join("Cargo.toml"),
        "[workspace]\nmembers = [\"app\", \"dep\"]\n",
    )
    .expect("the workspace manifest");
    std::fs::write(
        package.join("Cargo.toml"),
        "[package]\nname = \"app\"\n\n[dependencies]\ndep = { path = \"../dep\" }\n\n\
             [patch.crates-io]\ndep = { path = \"../fork\" }\n",
    )
    .expect("the member manifest whose patch cargo ignores");
    std::fs::write(package.join("src/lib.rs"), "pub fn nothing() {}\n").expect("the source");
    std::fs::write(dependency.join("Cargo.toml"), "[package]\nname = \"dep\"\n")
        .expect("the dependency manifest");
    std::fs::write(dependency.join("src/lib.rs"), "pub const ONE: u8 = 1;\n")
        .expect("the dependency source");
    std::fs::write(fork.join("Cargo.toml"), "[package]\nname = \"dep\"\n")
        .expect("the fork manifest");
    let ignored = fork.join("src/lib.rs");
    std::fs::write(&ignored, "pub const ONE: u8 = 9;\n").expect("the fork source");

    let (sources, unfollowed, _) = recorded_sources(&package, &workspace, &[]);
    assert!(unfollowed.is_empty(), "{unfollowed:?}");
    assert!(
        !sources.contains(&ignored),
        "the fork a member's ignored patch names is not an input to its build; the walk found \
         {sources:?}"
    );
    assert!(
        sources.contains(&dependency.join("src/lib.rs")),
        "while the edge cargo does resolve is recorded"
    );

    let binary = record_over(&workspace, &sources);
    std::fs::write(&ignored, "pub const ONE: u8 = 8;\n").expect("the changed fork source");
    assert!(
        changed_sources(&binary, &package)
            .expect("a readable record")
            .is_empty(),
        "a change to a directory cargo never reads does not refuse the binary"
    );
}

#[test]
fn a_root_patch_is_recorded_for_a_member_that_takes_it() {
    // A root's patch applies to a member's build — measured, the replacement
    // directory is what cargo compiles, `source = None` — while the member's own
    // manifest states no path at all, so the walk has to read the root's table.
    let root = fixture("root-patch");
    let workspace = root.join("ws");
    let package = workspace.join("app");
    let fork = workspace.join("fork");
    std::fs::create_dir_all(package.join("src")).expect("the package");
    std::fs::create_dir_all(fork.join("src")).expect("the replacement");
    std::fs::write(
        workspace.join("Cargo.toml"),
        "[workspace]\nmembers = [\"app\"]\n\n[patch.crates-io]\nserde = { path = \"fork\" }\n",
    )
    .expect("the root that declares the patch");
    std::fs::write(
        package.join("Cargo.toml"),
        "[package]\nname = \"app\"\n\n[dependencies]\nserde = \"1\"\n",
    )
    .expect("the member manifest that names no path");
    std::fs::write(package.join("src/lib.rs"), "pub fn nothing() {}\n").expect("the source");
    std::fs::write(
        fork.join("Cargo.toml"),
        "[package]\nname = \"serde\"\nversion = \"1.0.0\"\n",
    )
    .expect("the replacement manifest");
    let compiled = fork.join("src/lib.rs");
    std::fs::write(&compiled, "pub const ONE: u8 = 1;\n").expect("the replacement source");

    let (sources, unfollowed, _) = recorded_sources(&package, &workspace, &[]);
    assert!(unfollowed.is_empty(), "{unfollowed:?}");
    assert!(
        sources.contains(&compiled),
        "the replacement the root's patch puts in the build must be recorded; the walk found \
         {sources:?}"
    );
}

#[test]
fn a_dependency_inherited_from_the_workspace_root_is_walked_and_named() {
    // A member that says `dep.workspace = true` names no path itself, so the
    // path exists only in the root's `[workspace.dependencies]` table. Measured:
    // cargo resolves the dependency to the directory that table names and
    // compiles it, so a walk that stops at the member records a binary built
    // from sources no record names — and reports it clean after they change.
    let root = fixture("inherited-edge");
    let workspace = root.join("ws");
    let package = workspace.join("app");
    let dependency = workspace.join("dep");
    std::fs::create_dir_all(package.join("src")).expect("the package");
    std::fs::create_dir_all(dependency.join("src")).expect("the dependency");
    std::fs::write(
        workspace.join("Cargo.toml"),
        "[workspace]\nmembers = [\"app\", \"dep\"]\n\n\
             [workspace.dependencies]\ndep = { path = \"dep\" }\n",
    )
    .expect("the workspace manifest that declares the path");
    std::fs::write(
        package.join("Cargo.toml"),
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\n\n[dependencies]\ndep.workspace = true\n",
    )
    .expect("the member manifest that names no path");
    std::fs::write(package.join("src/lib.rs"), "pub fn nothing() {}\n").expect("the source");
    std::fs::write(dependency.join("Cargo.toml"), "[package]\nname = \"dep\"\n")
        .expect("the dependency manifest");
    let compiled = dependency.join("src/lib.rs");
    std::fs::write(&compiled, "pub const ONE: u8 = 1;\n").expect("the dependency source");

    let (sources, unfollowed, _) = recorded_sources(&package, &workspace, &[]);
    assert!(unfollowed.is_empty(), "{unfollowed:?}");
    assert!(
        sources.contains(&compiled) && sources.contains(&dependency.join("Cargo.toml")),
        "the dependency the workspace root resolves must be recorded with its manifest; the walk \
         found {sources:?}"
    );

    // The record is spelled against the workspace the guard resolves for the
    // package, which is the root that declares the inherited path.
    let binary = record_over(&workspace, &sources);
    std::fs::write(&compiled, "pub const ONE: u8 = 2;\n").expect("the changed source");
    assert_eq!(
        changed_sources(&binary, &package).expect("a readable record"),
        [Input::File(compiled)]
    );
}

#[test]
fn an_inherited_path_only_a_root_cargo_never_read_names_is_not_required() {
    // Measured: a member below a nearer `[workspace]` manifest inherits the
    // invoked root's entry, and the nearer manifest is never loaded — so a
    // candidate root the walk cannot tell from the real one may name a directory
    // that is not there. Stopping for it would stop a build cargo accepts, while
    // the root that cargo did read is still recorded.
    let root = fixture("inherited-roots");
    let outer = root.join("outer");
    let nearer = outer.join("mid");
    let package = nearer.join("app");
    let dependency = outer.join("dep");
    std::fs::create_dir_all(package.join("src")).expect("the package");
    std::fs::create_dir_all(dependency.join("src")).expect("the dependency");
    std::fs::write(
        outer.join("Cargo.toml"),
        "[workspace]\nmembers = [\"mid/app\"]\n\n\
             [workspace.dependencies]\ndep = { path = \"dep\" }\n",
    )
    .expect("the root the build is invoked in");
    std::fs::write(
        nearer.join("Cargo.toml"),
        "[workspace]\n\n[workspace.dependencies]\ndep = { path = \"gone\" }\n",
    )
    .expect("the manifest cargo never loads for this member");
    std::fs::write(
        package.join("Cargo.toml"),
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\n\n[dependencies]\ndep.workspace = true\n",
    )
    .expect("the member manifest");
    std::fs::write(package.join("src/lib.rs"), "pub fn nothing() {}\n").expect("the source");
    std::fs::write(dependency.join("Cargo.toml"), "[package]\nname = \"dep\"\n")
        .expect("the dependency manifest");
    std::fs::write(dependency.join("src/lib.rs"), "pub const ONE: u8 = 1;\n")
        .expect("the dependency source");

    let (sources, unfollowed, _) = recorded_sources(&package, &outer, &[]);
    assert!(unfollowed.is_empty(), "{unfollowed:?}");
    assert!(
        sources.contains(&dependency.join("src/lib.rs")),
        "the path the invoked root declares is recorded although a nearer candidate root names one \
         that is not there; the walk found {sources:?}"
    );
}

#[test]
fn a_patched_path_is_walked_and_named() {
    // A `[patch]` key replaces a dependency's source with a directory in
    // this tree, which the binary then compiles instead. Only the workspace
    // root is read for one, so it is the root *package* of a single-crate
    // workspace that stamps a record with it, and no declaration table
    // names the replacement the patch put in the build.
    let root = fixture("patch-table");
    let package = root.join("ws");
    let patched = root.join("fork");
    std::fs::create_dir_all(package.join("src")).expect("the package");
    std::fs::create_dir_all(patched.join("src")).expect("the replacement");
    std::fs::write(
        package.join("Cargo.toml"),
        "[workspace]\n\n[package]\nname = \"app\"\n\n[dependencies]\nserde = \"1\"\n\n\
             [patch.crates-io]\nserde = { path = \"../fork\" }\n",
    )
    .expect("the manifest cargo reads the patch from");
    std::fs::write(package.join("src/lib.rs"), "pub fn nothing() {}\n").expect("the source");
    std::fs::write(
        patched.join("Cargo.toml"),
        "[package]\nname = \"serde\"\nversion = \"1.0.0\"\n",
    )
    .expect("the replacement manifest");
    let compiled = patched.join("src/lib.rs");
    std::fs::write(&compiled, "pub const ONE: u8 = 1;\n").expect("the replacement source");

    let (sources, unfollowed, _) = recorded_sources(&package, &package, &[]);
    assert!(unfollowed.is_empty(), "{unfollowed:?}");
    assert!(
        sources.contains(&compiled),
        "the source a patch put in the build must be recorded; the walk found {sources:?}"
    );

    let binary = record_over(&package, &sources);
    std::fs::write(&compiled, "pub const ONE: u8 = 2;\n").expect("the changed source");
    assert_eq!(
        changed_sources(&binary, &package).expect("a readable record"),
        [Input::File(compiled)]
    );
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
fn the_resolution_is_named_only_where_a_reading_names_it() {
    // The fact the record's mark rests on. The build directory is not enough: it
    // need not be the workspace the resolution was read in, as the arrangements in
    // `walk::invocation_roots`' documentation measure. The directory the invoking
    // process was started in is, and where that process cannot be read — the
    // platform keeps no such reading, or the parent is not the cargo `CARGO` names
    // — the fact is unavailable and the record is marked rather than trusted.
    let root = fixture("invocation-named-fact");
    let workspace = root.join("ws");
    let out = workspace.join("target/debug/build/app-0f0f0f0f/out");
    std::fs::create_dir_all(&out).expect("the output directory");
    std::fs::write(workspace.join("Cargo.toml"), "[workspace]\nmembers = []\n")
        .expect("the root manifest");
    let unread = invocation_roots(&out, &[], None);
    assert_eq!(
        unread.roots,
        [canonical(&workspace)],
        "the build directory names the workspace that owns it"
    );
    assert!(
        !unread.resolution_named,
        "but the process was not read, so the record is marked"
    );

    let outside = root.join("outside");
    std::fs::create_dir_all(&outside).expect("a directory no workspace owns");
    let run_from = [outside.clone(), workspace.clone()];
    assert_eq!(
        invocation_roots(&out, &run_from, None).resolution_named,
        !READS_INVOKING_PROCESS,
        "where the process can be read, a run directory alone does not establish the resolution"
    );
    assert!(
        !invocation_roots(
            &out,
            std::slice::from_ref(&outside),
            Some(&Invoking {
                directory: outside.clone(),
                manifest: None,
            })
        )
        .resolution_named,
        "an invoking directory no workspace owns names no resolution root either"
    );
    assert!(
        invocation_roots(
            &out,
            &run_from,
            Some(&Invoking {
                directory: workspace.clone(),
                manifest: None,
            })
        )
        .resolution_named,
        "and the directory cargo was run in is the workspace the resolution was read in"
    );
    assert_eq!(
        candidate_roots(&out, &run_from),
        [canonical(&workspace)],
        "the workspace above the run directory is the candidate"
    );
}

#[test]
fn the_manifest_the_command_line_names_is_the_workspace_the_resolution_read() {
    // Measured: `cd third && CARGO_TARGET_DIR=<outside every workspace> cargo build
    // --manifest-path ../sibling/member/Cargo.toml` links the sibling's fork while
    // `third` is the only run directory and the build directory names no workspace,
    // so neither of the readings above places the build. The manifest the command
    // line named does: cargo reads the resolution in the workspace that manifest's
    // package belongs to (`cargo metadata --manifest-path …` reports it as
    // `workspace_root`), and that root is a candidate.
    let root = fixture("invoked-manifest");
    let third = root.join("third");
    let sibling = root.join("sibling");
    let out = third.join("target/debug/build/app-0f0f0f0f/out");
    std::fs::create_dir_all(&out).expect("the output directory");
    std::fs::write(third.join("Cargo.toml"), "[workspace]\nmembers = []\n")
        .expect("the third workspace");
    std::fs::create_dir_all(sibling.join("member/src")).expect("the member package");
    std::fs::write(
        sibling.join("Cargo.toml"),
        "[workspace]\nmembers = [\"member\"]\n",
    )
    .expect("the sibling workspace");
    std::fs::write(
        sibling.join("member/Cargo.toml"),
        "[package]\nname = \"member\"\nversion = \"0.1.0\"\n",
    )
    .expect("the member manifest");
    let named = Invoking {
        directory: third.clone(),
        manifest: Some(sibling.join("member/Cargo.toml")),
    };
    let invocation = invocation_roots(&out, std::slice::from_ref(&third), Some(&named));
    assert!(
        invocation.resolution_named,
        "the command line names the manifest, so the resolution's workspace is established"
    );
    assert!(
        invocation.roots.contains(&canonical(&sibling)),
        "and that manifest's workspace is a candidate root: {:?}",
        invocation.roots
    );

    // A manifest no `[workspace]` manifest owns is its own root, because that is
    // cargo's answer for it rather than a reading this walk cannot make: measured,
    // `cargo locate-project --workspace` in such a directory answers its own
    // manifest, `cargo metadata` reports that directory as `workspace_root`, and a
    // build there links the fork the `[patch]` table beside it names. Reading only
    // the nearest `[workspace]` manifest refused that build with no record at all.
    let standalone = root.join("standalone/src");
    std::fs::create_dir_all(&standalone).expect("a package no workspace owns");
    std::fs::write(
        standalone.join("../Cargo.toml"),
        "[package]\nname = \"standalone\"\nversion = \"0.1.0\"\n",
    )
    .expect("its manifest");
    let unowned = Invoking {
        directory: third.clone(),
        manifest: Some(standalone.join("../Cargo.toml")),
    };
    let standalone_root = canonical(&root.join("standalone"));
    let invocation = invocation_roots(&out, std::slice::from_ref(&third), Some(&unowned));
    assert!(
        invocation.resolution_named,
        "a package no workspace owns is its own root, so the resolution is placed"
    );
    assert!(
        invocation.roots.contains(&standalone_root),
        "and that directory is a candidate root: {:?}",
        invocation.roots
    );
}

#[test]
fn the_workspace_above_the_build_directory_is_a_candidate_root() {
    // A build writes its output to `<build directory>/<profile>/build/<package>-<hash>/out`,
    // so the directory above the profile names the workspace the build was
    // invoked in when it is one at all.
    let root = fixture("invocation-root");
    let workspace = root.join("ws");
    let out = workspace.join("target/debug/build/app-0f0f0f0f/out");
    std::fs::create_dir_all(&out).expect("the output directory");
    std::fs::write(workspace.join("Cargo.toml"), "[workspace]\nmembers = []\n")
        .expect("the root manifest");
    assert_eq!(
        candidate_roots(&out, &[]),
        [canonical(&workspace)],
        "the workspace that owns the build directory is a root the build could have been invoked in"
    );
}

#[test]
#[should_panic(expected = "cannot be named")]
fn a_build_that_names_no_workspace_stops_the_record() {
    // Measured: `env -u PWD CARGO_TARGET_DIR=/tmp/target cargo build -p pkg` run
    // from a sibling workspace writes its output outside every workspace, so no
    // reading names the root whose `[patch]` table the build read — the record
    // names the package's own roots alone and a change to the fork that build
    // compiled is reported clean. That record is refused rather than written.
    let root = fixture("invocation-unnameable");
    let elsewhere = root.join("elsewhere/target/debug/build/app-0f0f0f0f/out");
    std::fs::create_dir_all(&elsewhere).expect("a build directory no workspace owns");
    let _ = candidate_roots(&elsewhere, &[]);
}

#[test]
#[should_panic(expected = "cannot be named")]
fn a_path_that_is_not_a_build_output_directory_names_no_workspace() {
    // The output directory is read by its shape, so a path that is not
    // `…/build/<package>-<hash>/out` names no build directory — and with no
    // workspace named at all the build stops rather than recording every root
    // the tree holds as if that were the whole story.
    let root = fixture("invocation-not-output");
    let workspace = root.join("ws");
    std::fs::create_dir_all(workspace.join("target/debug/build/app-0f0f0f0f"))
        .expect("a directory that is not the output one");
    std::fs::write(workspace.join("Cargo.toml"), "[workspace]\nmembers = []\n")
        .expect("the root manifest");
    let _ = candidate_roots(&workspace.join("target/debug/build/app-0f0f0f0f"), &[]);
}

#[test]
fn a_level_named_like_a_triple_is_not_taken_for_cargos_triple_level() {
    // Measured: `CARGO_TARGET_DIR=<root>/x86_64-unknown-linux-gnu` with no
    // `--target` writes the output one level under a build directory merely
    // *named* like the host triple, and a rule that skips a level because of its
    // name loses the root that is there — a record missing the fork a sibling
    // build compiled, where the pre-fix reading found it. So the name decides
    // nothing: both readings are candidates.
    let root = fixture("invocation-triple-name");
    let workspace = root.join("ws");
    let named_like_a_triple =
        workspace.join("x86_64-unknown-linux-gnu/debug/build/app-0f0f0f0f/out");
    std::fs::create_dir_all(&named_like_a_triple).expect("the output directory");
    std::fs::write(workspace.join("Cargo.toml"), "[workspace]\nmembers = []\n")
        .expect("the root manifest");
    assert_eq!(
        candidate_roots(&named_like_a_triple, &[]),
        [canonical(&workspace)],
        "a build directory named like a triple is the directory above the profile"
    );

    // `--target <triple>` inserts the level, and the directory above it is then
    // the build directory. Same shape, same root, told apart by nothing in the
    // path — which is why both are read.
    let under_a_triple =
        workspace.join("target/x86_64-unknown-linux-gnu/debug/build/app-0f0f0f0f/out");
    std::fs::create_dir_all(&under_a_triple).expect("the output directory");
    assert_eq!(
        candidate_roots(&under_a_triple, &[]),
        [canonical(&workspace)],
        "the triple level is cargo's, and the build directory is above it"
    );
}

#[test]
fn both_readings_of_the_level_above_the_profile_are_candidate_roots() {
    // Each reading can name a workspace — a build directory kept inside another
    // workspace, with the level above it also declaring one — and a record that
    // chose between them could omit the resolution's patches.
    let root = fixture("invocation-both-readings");
    let outer = root.join("outer");
    let out = outer.join("target/debug/build/app-0f0f0f0f/out");
    std::fs::create_dir_all(&out).expect("the output directory");
    for workspace in [&root, &outer] {
        std::fs::write(workspace.join("Cargo.toml"), "[workspace]\nmembers = []\n")
            .expect("a root manifest");
    }
    assert_eq!(
        candidate_roots(&out, &[]),
        [canonical(&root), canonical(&outer)],
        "both readings are candidates, not a choice"
    );
}

#[test]
fn the_directory_cargo_was_run_in_names_a_candidate_root() {
    // Cargo finds the workspace to build by scanning up from the directory it was
    // run in, so that directory names a candidate even where the build directory
    // says nothing: measured, `CARGO_TARGET_DIR=<abs>/ws2/target` run in `ws1`
    // leaves no trace of `ws1` anywhere else.
    let root = fixture("invocation-run-from");
    let workspace = root.join("ws");
    let member = workspace.join("member/src");
    std::fs::create_dir_all(&member).expect("a member directory");
    std::fs::write(
        workspace.join("Cargo.toml"),
        "[workspace]\nmembers = [\"member\"]\n",
    )
    .expect("the root manifest");
    let elsewhere = root.join("elsewhere/target/debug/build/app-0f0f0f0f/out");
    std::fs::create_dir_all(&elsewhere).expect("a build directory no workspace owns");
    assert_eq!(
        candidate_roots(&elsewhere, &[member]),
        [canonical(&workspace)],
        "the workspace above the directory cargo was run in is a candidate"
    );
}

#[test]
fn the_root_cargo_answers_for_a_directory_is_the_candidate_root() {
    // The directory cargo was run in names its root through cargo rather than
    // through a second implementation of the same scan: measured, `cargo
    // locate-project --workspace` run in a package that names a root answers
    // that root even where an ancestor manifest declares `[workspace]`, which
    // the ancestor scan below it would have named instead.
    let root = fixture("invocation-asked");
    let ancestor = root.join("outer");
    let named = root.join("named");
    let package = ancestor.join("pkg");
    for (workspace, manifest) in [
        (&ancestor, "[workspace]\nmembers = [\"pkg\"]\n"),
        (&named, "[workspace]\nmembers = [\"../outer/pkg\"]\n"),
    ] {
        std::fs::create_dir_all(workspace).expect("a workspace directory");
        std::fs::write(workspace.join("Cargo.toml"), manifest).expect("a root manifest");
    }
    std::fs::create_dir_all(package.join("src")).expect("the package");
    std::fs::write(
        package.join("Cargo.toml"),
        "[package]\nname = \"pkg\"\nworkspace = \"../../named\"\n",
    )
    .expect("the package manifest");
    std::fs::write(package.join("src/lib.rs"), "pub fn nothing() {}\n").expect("the source");

    let elsewhere = root.join("elsewhere/target/debug/build/app-0f0f0f0f/out");
    std::fs::create_dir_all(&elsewhere).expect("a build directory no workspace owns");
    assert_eq!(
        candidate_roots(&elsewhere, &[package]),
        [canonical(&named)],
        "the candidate is the root cargo itself answers for the run directory"
    );
}

#[test]
fn an_underivable_build_directory_falls_back_to_the_invocation_directory() {
    // The arrangement that makes the invocation directory a candidate rather
    // than a formality: a sibling workspace points `CARGO_TARGET_DIR` at another
    // workspace's target directory, so the build directory names that other
    // workspace — and nothing names the sibling except the directory cargo was
    // run in. The record must still name the fork the build compiled.
    let root = fixture("invocation-fallback");
    let sibling = root.join("ws1");
    let named = root.join("ws2");
    let package = root.join("pkg");
    for (directory, manifest) in [
        (
            &sibling,
            "[workspace]\nmembers = [\"tool\"]\n\n\
             [patch.crates-io]\nserde = { path = \"sibling-fork\" }\n",
        ),
        (
            &named,
            "[workspace]\nmembers = [\"../pkg\"]\n\n[patch.crates-io]\nserde = { path = \"fork\" }\n",
        ),
    ] {
        std::fs::create_dir_all(directory).expect("a workspace directory");
        std::fs::write(directory.join("Cargo.toml"), manifest).expect("a workspace root");
        std::fs::write(directory.join("Cargo.lock"), "version = 4\n")
            .expect("the lockfile its resolution reads");
    }
    for (fork, source) in [
        (sibling.join("sibling-fork"), "pub const ONE: u8 = 1;\n"),
        (named.join("fork"), "pub const ONE: u8 = 2;\n"),
    ] {
        std::fs::create_dir_all(fork.join("src")).expect("a replacement");
        std::fs::write(
            fork.join("Cargo.toml"),
            "[package]\nname = \"serde\"\nversion = \"1.0.0\"\n",
        )
        .expect("a replacement manifest");
        std::fs::write(fork.join("src/lib.rs"), source).expect("a replacement source");
    }
    // The member that takes the package by path, which is what makes the
    // sibling workspace one cargo resolves the package in: measured, a build of
    // a path dependency reports that workspace's root through `cargo tree -p`.
    let member = sibling.join("tool");
    std::fs::create_dir_all(member.join("src")).expect("the member");
    std::fs::write(
        member.join("Cargo.toml"),
        "[package]\nname = \"tool\"\nversion = \"0.1.0\"\n\n\
         [dependencies]\npkg = { path = \"../../pkg\" }\n",
    )
    .expect("the member manifest");
    std::fs::write(member.join("src/lib.rs"), "pub fn tool() {}\n").expect("the member source");

    let applied = sibling.join("sibling-fork/src/lib.rs");
    let own = named.join("fork/src/lib.rs");
    std::fs::create_dir_all(package.join("src")).expect("the package");
    std::fs::write(
        package.join("Cargo.toml"),
        "[package]\nname = \"pkg\"\nversion = \"0.1.0\"\nworkspace = \"../ws2\"\n\n\
             [dependencies]\nserde = \"1\"\n",
    )
    .expect("the package manifest");
    std::fs::write(package.join("src/lib.rs"), "pub fn nothing() {}\n").expect("the source");

    // The build directory is the named root's own — the arrangement where the
    // path says the named root and only the invocation directory says ws1.
    let out = named.join("target/debug/build/app-0f0f0f0f/out");
    std::fs::create_dir_all(&out).expect("the output directory");
    let candidates = candidate_roots(&out, std::slice::from_ref(&sibling));
    assert_eq!(candidates, [canonical(&sibling), canonical(&named)]);

    let (sources, unfollowed, _) = recorded_sources(&package, &named, &candidates);
    assert!(unfollowed.is_empty(), "{unfollowed:?}");
    for input in [
        &applied,
        &sibling.join("Cargo.toml"),
        &sibling.join("Cargo.lock"),
    ] {
        assert!(
            sources.contains(input),
            "the sibling workspace's replacement, manifest and lockfile are inputs to a build it \
             invokes, although the build directory is the named root's; the walk found {sources:?}"
        );
    }
    assert!(
        sources.contains(&own),
        "and the named root's own replacement is recorded too, since it is the fork its own build \
         would compile"
    );

    let binary = record_over(&named, &sources);
    std::fs::write(&applied, "pub const ONE: u8 = 3;\n").expect("the changed replacement");
    assert_eq!(
        changed_sources(&binary, &package).expect("a readable record"),
        [Input::File(applied)],
        "a change to the replacement this build compiles must refuse the binary"
    );
}

#[test]
fn a_workspace_cargo_does_not_resolve_the_package_in_is_still_a_candidate_root() {
    // Cargo's answer is a false negative for a workspace that reaches the
    // package through an optional dependency a build elsewhere selected:
    // measured, `cargo tree -p pkg --offline` under the default features answers
    // `package ID specification `pkg` did not match any packages` while `cargo
    // build -p tool --features pkg` in that workspace compiles the package and
    // uses the workspace's own `[patch]` fork of its dependency. So the root is
    // kept — its patches, manifest and lockfile are inputs a build it invokes
    // reads — rather than traded for a record that omits the fork it compiled.
    let root = fixture("invocation-optional");
    let workspace = root.join("ws");
    let package = root.join("pkg");
    std::fs::create_dir_all(workspace.join("tool/src")).expect("the member");
    std::fs::write(
        workspace.join("Cargo.toml"),
        "[workspace]\nmembers = [\"tool\"]\n",
    )
    .expect("the root manifest");
    std::fs::write(
        workspace.join("tool/Cargo.toml"),
        "[package]\nname = \"tool\"\nversion = \"0.1.0\"\n\n\
         [dependencies]\npkg = { path = \"../../pkg\", optional = true }\n\n\
         [features]\npkg = [\"dep:pkg\"]\n",
    )
    .expect("the member manifest that reaches the package only under a feature");
    std::fs::write(workspace.join("tool/src/lib.rs"), "pub fn tool() {}\n").expect("the member");
    std::fs::write(workspace.join("Cargo.lock"), "version = 4\n")
        .expect("the lockfile a build invoked there reads");
    std::fs::create_dir_all(package.join("src")).expect("the package");
    std::fs::write(
        package.join("Cargo.toml"),
        "[package]\nname = \"pkg\"\nversion = \"0.1.0\"\n",
    )
    .expect("the package manifest");
    std::fs::write(package.join("src/lib.rs"), "pub fn nothing() {}\n").expect("the source");

    assert!(
        matches!(
            resolved_packages(&workspace, &package),
            Answer::Given(packages) if packages.is_empty()
        ),
        "the answer the walk must not read as an exclusion: cargo ran and resolved nothing, which \
         is not the fact that the workspace cannot build the package"
    );

    let (sources, unfollowed, _) = recorded_sources(&package, &package, &[canonical(&workspace)]);
    assert!(unfollowed.is_empty(), "{unfollowed:?}");
    for input in ["Cargo.toml", "Cargo.lock"] {
        assert!(
            sources.contains(&canonical(&workspace.join(input))),
            "a workspace whose member reaches the package only under a feature is still a root a \
             build of it could have been invoked in, so its {input} is an input; the walk found \
             {sources:?}"
        );
    }
}

#[test]
fn the_packages_a_tree_names_are_the_directories_cargo_resolved() {
    // What the gate reads out of cargo's answer: `cargo tree --prefix none`
    // writes a package of this tree as `name v1.2.3 (/directory)` (measured,
    // with the `[patch]` fork a workspace applied in place of the registry
    // one), a registry package with no directory at all, and a source of a git
    // or registry *replacement* with a URL, which names no directory here.
    let root = fixture("tree-paths");
    for directory in [root.join("pkg"), root.join("fork")] {
        std::fs::create_dir_all(directory).expect("a package directory");
    }
    let tree = format!(
        "pkg v0.1.0 ({})\nserde v1.0.0 ({})\nserde v1.0.229\nother v0.2.0 (git+https://example.invalid/other#abc)\n",
        canonical(&root.join("pkg")).display(),
        canonical(&root.join("fork")).display(),
    );
    assert_eq!(
        paths_in(&tree),
        BTreeSet::from([canonical(&root.join("pkg")), canonical(&root.join("fork"))]),
        "a package directory is read, a registry package names none, and a source URL is not one"
    );
}

#[test]
#[should_panic(expected = "did not reach it")]
fn a_package_cargo_resolves_and_the_walk_misses_stops_the_record() {
    // The gate between the walk's guess and the record. A root the package is
    // not part of is reached through its `[patch]` and inherited tables rather
    // than by walking its closure, and cargo's own answer for that root is the
    // check: a package it resolves for this one that the closure does not hold
    // is a record that is short, so the build stops rather than writing it.
    let root = fixture("gate");
    let reached = canonical(&root.join("pkg"));
    gate_against_cargo(
        &[(
            root.join("ws"),
            BTreeSet::from([reached.clone(), canonical(&root.join("ws/sibling-fork"))]),
        )],
        &[reached],
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
fn a_directory_of_a_target_name_below_the_root_holds_compiled_files() {
    // The targets cargo builds for tests, examples and benches are the ones at
    // the package root, because that is where cargo looks for them. A file under
    // `src/tests/` declared as `mod tests;` — no `#[cfg(test)]` anywhere — is
    // compiled into the binary, so excluding the name at every depth left a
    // compiled file out of the record and a change to it was reported clean.
    let root = fixture("walk-nested-targets");
    for path in ["src/lib.rs", "src/tests/mod.rs", "src/benches/mod.rs"] {
        let file = root.join(path);
        std::fs::create_dir_all(file.parent().expect("a parent")).expect("directories");
        std::fs::write(&file, "content").expect("the source");
    }
    for path in [
        "tests/process.rs",
        "benches/speed.rs",
        "target/debug/leftover",
    ] {
        let file = root.join(path);
        std::fs::create_dir_all(file.parent().expect("a parent")).expect("directories");
        std::fs::write(&file, "content").expect("the target");
    }
    let sources = package_sources(&root);
    for path in ["src/lib.rs", "src/tests/mod.rs", "src/benches/mod.rs"] {
        assert!(
            sources.contains(&root.join(path)),
            "{path} is compiled into the binary: the package root is where cargo looks for \
             target directories, not every directory that carries one of their names"
        );
    }
    for path in [
        "tests/process.rs",
        "benches/speed.rs",
        "target/debug/leftover",
    ] {
        assert!(
            !sources.contains(&root.join(path)),
            "{path} sits in a target directory at the package root, whose own unit reads it: \
             recording it would refuse a current binary for a change no rebuild could clear"
        );
    }
}

#[test]
fn a_directory_no_build_generates_holds_compiled_files_wherever_it_sits() {
    // The wire file's `exclusion` lines name what a walk skips and where. `dist`
    // is not one of them: it is not a name cargo writes (`target`), not installed
    // package state (`node_modules`) and not version-control state (`.git`), so
    // skipping it anywhere can only hide an authored file. Measured with a probe
    // crate, a `mod dist;` compiling `src/dist/mod.rs` was dropped from the
    // record while both readers reported the binary current.
    let root = fixture("walk-dist");
    for path in ["src/lib.rs", "src/dist/mod.rs", "dist/tool.rs"] {
        let file = root.join(path);
        std::fs::create_dir_all(file.parent().expect("a parent")).expect("directories");
        std::fs::write(&file, "content").expect("the source");
    }
    let sources = package_sources(&root);
    for path in ["src/lib.rs", "src/dist/mod.rs", "dist/tool.rs"] {
        assert!(
            sources.contains(&root.join(path)),
            "{path} sits under a name no build generates, so it is walked: skipping the name \
             anywhere would hide a file the build read"
        );
    }
}

#[test]
fn an_input_compiled_from_outside_the_package_is_recorded_and_named() {
    let root = fixture("outside");
    std::fs::create_dir_all(root.join("crates/app/src")).expect("the package");
    std::fs::write(
        root.join("Cargo.toml"),
        "[workspace]\nmembers = [\"crates/app\"]\n",
    )
    .expect("the root manifest");
    std::fs::write(
        root.join("crates/app/Cargo.toml"),
        "[package]\nname = \"app\"\n",
    )
    .expect("the package manifest");
    std::fs::write(
        root.join("crates/app/src/lib.rs"),
        "pub const FIXTURE: &str = include_str!(\"../../../fixtures/register.json\");\n",
    )
    .expect("the source that compiles a file outside its package");
    let compiled = root.join("fixtures/register.json");
    std::fs::create_dir_all(compiled.parent().expect("a parent")).expect("the fixtures");
    std::fs::write(&compiled, "{\"one\": true}\n").expect("the fixture");

    let (sources, unfollowed, _) = recorded_sources(&root.join("crates/app"), &root, &[]);
    assert!(unfollowed.is_empty(), "{unfollowed:?}");
    assert!(
        sources.contains(&canonical(&compiled)),
        "a package that compiles {} must record it; the walk found {sources:?}",
        compiled.display()
    );

    let binary = record_over(&root, &sources);
    std::fs::write(&compiled, "{\"two\": true}\n").expect("the changed fixture");
    assert_eq!(
        changed_sources(&binary, &root).expect("a readable record"),
        [Input::File(compiled)]
    );
}

#[test]
fn an_input_the_build_names_through_a_built_path_is_recorded_and_named() {
    let root = fixture("built-path");
    std::fs::create_dir_all(root.join("crates/app/src")).expect("the package");
    std::fs::write(
        root.join("Cargo.toml"),
        "[workspace]\nmembers = [\"crates/app\"]\n",
    )
    .expect("the root manifest");
    std::fs::write(
        root.join("crates/app/Cargo.toml"),
        "[package]\nname = \"app\"\n",
    )
    .expect("the package manifest");
    std::fs::write(
            root.join("crates/app/src/lib.rs"),
            "pub const FIXTURE: &str = include_str!(concat!(\n    env!(\"CARGO_MANIFEST_DIR\"),\n    \"/fixtures/register.json\"\n));\n",
        )
        .expect("the source that builds its path at compile time");
    let compiled = root.join("crates/app/fixtures/register.json");
    std::fs::create_dir_all(compiled.parent().expect("a parent")).expect("the fixtures");
    std::fs::write(&compiled, "{\"one\": true}\n").expect("the fixture");

    let (sources, unfollowed, _) = recorded_sources(&root.join("crates/app"), &root, &[]);
    assert!(unfollowed.is_empty(), "{unfollowed:?}");
    assert!(
        sources.contains(&canonical(&compiled)),
        "a package that compiles {} must record it; the walk found {sources:?}",
        compiled.display()
    );

    let binary = record_over(&root, &sources);
    std::fs::write(&compiled, "{\"two\": true}\n").expect("the changed fixture");
    assert_eq!(
        changed_sources(&binary, &root).expect("a readable record"),
        [Input::File(compiled)]
    );
}

#[test]
fn an_input_the_scan_cannot_follow_stops_the_record() {
    let root = fixture("unfollowed");
    std::fs::create_dir_all(root.join("crates/app/src")).expect("the package");
    std::fs::write(
        root.join("Cargo.toml"),
        "[workspace]\nmembers = [\"crates/app\"]\n",
    )
    .expect("the root manifest");
    std::fs::write(
        root.join("crates/app/Cargo.toml"),
        "[package]\nname = \"app\"\n",
    )
    .expect("the package manifest");
    std::fs::write(
        root.join("crates/app/src/lib.rs"),
        "pub static SCHEMA: &str = include_str!(SCHEMA_PATH);\n",
    )
    .expect("the source that builds its path elsewhere");

    let (sources, unfollowed, _) = recorded_sources(&root.join("crates/app"), &root, &[]);
    assert_eq!(unfollowed.len(), 1, "{unfollowed:?}");
    assert!(
        unfollowed[0].contains("lib.rs") && unfollowed[0].contains("SCHEMA_PATH"),
        "{}",
        unfollowed[0]
    );
    assert!(
        !sources.iter().any(|source| source.ends_with("SCHEMA_PATH")),
        "the unnameable input must be refused, not dropped"
    );
}

#[test]
fn a_generated_path_is_left_to_the_build_script_that_writes_it() {
    let root = fixture("generated");
    std::fs::create_dir_all(root.join("crates/app/src")).expect("the package");
    std::fs::write(
        root.join("Cargo.toml"),
        "[workspace]\nmembers = [\"crates/app\"]\n",
    )
    .expect("the root manifest");
    std::fs::write(
        root.join("crates/app/Cargo.toml"),
        "[package]\nname = \"app\"\n",
    )
    .expect("the package manifest");
    std::fs::write(
        root.join("crates/app/src/lib.rs"),
        "include!(concat!(env!(\"OUT_DIR\"), \"/generated.rs\"));\n",
    )
    .expect("the source that includes a generated file");

    let (_, unfollowed, _) = recorded_sources(&root.join("crates/app"), &root, &[]);
    assert!(unfollowed.is_empty(), "{unfollowed:?}");
}

#[test]
fn the_workspace_manifest_is_recorded_and_named() {
    let root = fixture("workspace-manifest");
    let package = root.join("crates/app");
    std::fs::create_dir_all(package.join("src")).expect("the package");
    std::fs::write(
        root.join("Cargo.toml"),
        "[workspace]\nmembers = [\"crates/app\"]\n\n[workspace.package]\nedition = \"2024\"\n",
    )
    .expect("the workspace manifest");
    std::fs::write(
        package.join("Cargo.toml"),
        "[package]\nname = \"app\"\nedition.workspace = true\n",
    )
    .expect("the package manifest");
    std::fs::write(package.join("src/lib.rs"), "pub fn nothing() {}\n").expect("the source");

    let (sources, unfollowed, _) = recorded_sources(&package, &root, &[]);
    assert!(unfollowed.is_empty(), "{unfollowed:?}");
    assert!(
        sources.contains(&root.join("Cargo.toml")),
        "the manifest that sets the edition this package compiles under must be recorded; the \
             walk found {sources:?}"
    );

    let binary = record_over(&root, &sources);
    std::fs::write(
        root.join("Cargo.toml"),
        "[workspace]\nmembers = [\"crates/app\"]\n\n[workspace.package]\nedition = \"2021\"\n",
    )
    .expect("the changed workspace manifest");
    assert_eq!(
        changed_sources(&binary, &root).expect("a readable record"),
        [Input::File(root.join("Cargo.toml"))]
    );
}

#[test]
fn a_dependency_in_a_second_workspace_contributes_that_workspace_root() {
    let root = fixture("second-workspace");
    let package = root.join("wsA/app");
    let elsewhere = root.join("other");
    std::fs::create_dir_all(package.join("src")).expect("the package");
    std::fs::create_dir_all(elsewhere.join("dep/src")).expect("the dependency");
    std::fs::write(
        root.join("wsA/Cargo.toml"),
        "[workspace]\nmembers = [\"app\"]\n\n[workspace.package]\nedition = \"2024\"\n",
    )
    .expect("the workspace manifest");
    std::fs::write(
        package.join("Cargo.toml"),
        "[package]\nname = \"app\"\nedition.workspace = true\n\n[dependencies]\n\
             dep = { path = \"../../other/dep\" }\n",
    )
    .expect("the package manifest");
    std::fs::write(package.join("src/lib.rs"), "pub fn nothing() {}\n").expect("the source");
    // The dependency is a member of a workspace of its own, whose root is
    // the manifest that sets the edition and version it is compiled with.
    std::fs::write(
        elsewhere.join("Cargo.toml"),
        "[workspace]\nmembers = [\"dep\"]\n\n[workspace.package]\nedition = \"2024\"\n",
    )
    .expect("the second workspace manifest");
    std::fs::write(
        elsewhere.join("dep/Cargo.toml"),
        "[package]\nname = \"dep\"\nedition.workspace = true\n",
    )
    .expect("the dependency manifest");
    std::fs::write(elsewhere.join("dep/src/lib.rs"), "pub fn nothing() {}\n")
        .expect("the dependency source");

    let (sources, unfollowed, _) = recorded_sources(&package, &root.join("wsA"), &[]);
    assert!(unfollowed.is_empty(), "{unfollowed:?}");
    assert!(
        sources.contains(&elsewhere.join("Cargo.toml")),
        "the manifest that sets this dependency's edition must be recorded; the walk found \
             {sources:?}"
    );

    let binary = record_over(&root.join("wsA"), &sources);
    std::fs::write(
        elsewhere.join("Cargo.toml"),
        "[workspace]\nmembers = [\"dep\"]\n\n[workspace.package]\nedition = \"2021\"\n",
    )
    .expect("the changed second workspace manifest");
    assert_eq!(
        changed_sources(&binary, &package).expect("a readable record"),
        [Input::File(elsewhere.join("Cargo.toml"))]
    );
}

#[test]
fn a_package_below_another_workspace_manifest_records_every_root() {
    // Cargo resolves a package to the *invoked* workspace, which can be an
    // ancestor reached past a nearer `[workspace]` manifest: measured with
    // these two manifests, cargo read the outer one's `[workspace.package]`
    // for both packages. The nearest-ancestor rule named the inner manifest
    // and left the outer one, which cargo did read, unrecorded.
    let root = fixture("nested-workspaces");
    let package = root.join("outer/inner/crate");
    let nested = root.join("outer/inner");
    std::fs::create_dir_all(package.join("src")).expect("the package");
    std::fs::create_dir_all(nested.join("dep/src")).expect("the dependency");
    std::fs::write(
        root.join("outer/Cargo.toml"),
        "[workspace]\nmembers = [\"inner/dep\", \"inner/crate\"]\n\n[workspace.package]\n\
             version = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .expect("the outer manifest");
    std::fs::write(
        nested.join("Cargo.toml"),
        "[workspace]\nmembers = [\"dep\", \"crate\"]\n\n[workspace.package]\n\
             version = \"0.2.0\"\nedition = \"2021\"\n",
    )
    .expect("the nearer manifest");
    std::fs::write(
        package.join("Cargo.toml"),
        "[package]\nname = \"app\"\nversion.workspace = true\nedition.workspace = true\n\n\
             [dependencies]\ndep = { path = \"../dep\" }\n",
    )
    .expect("the package manifest");
    std::fs::write(package.join("src/lib.rs"), "pub fn nothing() {}\n").expect("the source");
    std::fs::write(
        nested.join("dep/Cargo.toml"),
        "[package]\nname = \"dep\"\nversion.workspace = true\nedition.workspace = true\n\
             workspace = \"../..\"\n",
    )
    .expect("the dependency manifest");
    std::fs::write(nested.join("dep/src/lib.rs"), "pub fn nothing() {}\n")
        .expect("the dependency source");

    let roots = workspace_roots(&package);
    assert!(roots.contains(&canonical(&root.join("outer"))), "{roots:?}");
    assert!(roots.contains(&canonical(&nested)), "{roots:?}");

    let (sources, unfollowed, _) = recorded_sources(&package, &root.join("outer"), &[]);
    assert!(unfollowed.is_empty(), "{unfollowed:?}");
    assert!(
        sources.contains(&root.join("outer/Cargo.toml")),
        "the manifest cargo read for this build must be recorded; the walk found {sources:?}"
    );
}

#[test]
fn a_patch_in_each_root_a_build_could_be_invoked_in_is_a_candidate_input() {
    // Measured on two roots that each list the member and patch the same name:
    // invoked at the outer root `serde` resolves to `outer/outer-fork`, invoked
    // at the nearer one to `mid/mid-fork`. Both invocations are valid and they
    // disagree, so nothing in the package's own tree says which root a build
    // read and the walk asks both. A record naming only the nearer root — the
    // base a record is spelled against — would report a binary clean after the
    // fork the outer invocation compiles had changed.
    let root = fixture("nested-patches");
    let outer = root.join("outer");
    let nearer = outer.join("mid");
    let package = nearer.join("app");
    std::fs::create_dir_all(package.join("src")).expect("the package");
    std::fs::write(
        outer.join("Cargo.toml"),
        "[workspace]\nmembers = [\"mid/app\"]\n\n\
             [patch.crates-io]\nserde = { path = \"outer-fork\" }\n",
    )
    .expect("the root the outer invocation is in");
    std::fs::write(
        nearer.join("Cargo.toml"),
        "[workspace]\nmembers = [\"app\"]\n\n\
             [patch.crates-io]\nserde = { path = \"mid-fork\" }\n",
    )
    .expect("the root the nearer invocation is in");
    std::fs::write(
        package.join("Cargo.toml"),
        "[package]\nname = \"app\"\n\n[dependencies]\nserde = \"1\"\n",
    )
    .expect("the member manifest that names no path");
    std::fs::write(package.join("src/lib.rs"), "pub fn nothing() {}\n").expect("the source");

    let mut forks = Vec::new();
    for fork in [outer.join("outer-fork"), nearer.join("mid-fork")] {
        std::fs::create_dir_all(fork.join("src")).expect("the replacement");
        std::fs::write(
            fork.join("Cargo.toml"),
            "[package]\nname = \"serde\"\nversion = \"1.0.0\"\n",
        )
        .expect("the replacement manifest");
        let compiled = fork.join("src/lib.rs");
        std::fs::write(&compiled, "pub const ONE: u8 = 1;\n").expect("the replacement source");
        forks.push(compiled);
    }

    let (sources, unfollowed, _) = recorded_sources(&package, &nearer, &[]);
    assert!(unfollowed.is_empty(), "{unfollowed:?}");
    for fork in &forks {
        assert!(
            sources.contains(fork),
            "a fork either invocation compiles must be recorded; the walk found {sources:?}"
        );
    }

    let binary = record_over(&nearer, &sources);
    for fork in &forks {
        std::fs::write(fork, "pub const ONE: u8 = 2;\n").expect("the changed fork");
        assert_eq!(
            changed_sources(&binary, &package).expect("a readable record"),
            [Input::File(fork.clone())],
            "a change to {} must refuse the binary",
            fork.display()
        );
        std::fs::write(fork, "pub const ONE: u8 = 1;\n").expect("the restored fork");
    }
}

#[test]
fn an_excluded_package_is_its_own_workspace_root() {
    // Cargo does not claim a package its `exclude` names — measured: its
    // search reads the excluding manifest, walks past it, and with no other
    // workspace above, the package is a workspace of one, so its own
    // directory holds the manifest and lockfile the build reads. The
    // excluding manifest is read to learn that, and stays recorded.
    let root = fixture("excluded-package");
    let package = root.join("ws/dep");
    std::fs::create_dir_all(package.join("src")).expect("the package");
    std::fs::create_dir_all(root.join("ws/app/src")).expect("the member");
    std::fs::write(
        root.join("ws/Cargo.toml"),
        "[workspace]\nmembers = [\"app\"]\nexclude = [\"dep\"]\n",
    )
    .expect("the excluding manifest");
    std::fs::write(
        root.join("ws/app/Cargo.toml"),
        "[package]\nname = \"app\"\n",
    )
    .expect("the member manifest");
    std::fs::write(root.join("ws/app/src/lib.rs"), "pub fn nothing() {}\n")
        .expect("the member source");
    std::fs::write(package.join("Cargo.toml"), "[package]\nname = \"dep\"\n")
        .expect("the package manifest");
    std::fs::write(package.join("src/lib.rs"), "pub fn nothing() {}\n")
        .expect("the package source");
    std::fs::write(package.join("Cargo.lock"), "version = 4\n").expect("its own lockfile");

    let roots = workspace_roots(&package);
    assert_eq!(
        roots,
        vec![canonical(&root.join("ws")), canonical(&package)],
        "the excluding manifest is read and the package is a workspace of one"
    );

    let (sources, _, _) = recorded_sources(&package, &root.join("ws"), &[]);
    assert!(
        sources.contains(&package.join("Cargo.lock")),
        "an excluded package's own lockfile is its workspace's; the walk found {sources:?}"
    );
}

#[test]
fn a_root_a_package_names_supplies_its_inherited_paths_and_patches() {
    // Measured on this tree: the package names `../../named` while an ancestor
    // manifest (`outer`) declares `[workspace]` and offers entries of its own.
    // Cargo reports `named` as `workspace_root` and compiles `named/fork`,
    // resolving the inherited `dep` to `named/dep` — the named root's entry,
    // not the ancestor's. Both are what the build compiles, so the walk reads
    // them from the root the package names; the names are the package's own
    // manifest's, so nothing else says where they resolve.
    let root = fixture("named-root");
    let outer = root.join("outer");
    let package = outer.join("app");
    let named = root.join("named");
    let dependency = named.join("dep");
    std::fs::create_dir_all(package.join("src")).expect("the package");
    std::fs::create_dir_all(dependency.join("src")).expect("the dependency");
    std::fs::write(
        outer.join("Cargo.toml"),
        "[workspace]\nmembers = [\"app\"]\n\n\
             [workspace.dependencies]\ndep = { path = \"outer-dep\" }\n",
    )
    .expect("the ancestor manifest the package does not name");
    std::fs::write(
        named.join("Cargo.toml"),
        "[workspace]\nmembers = [\"../outer/app\"]\n\n\
             [workspace.dependencies]\ndep = { path = \"dep\" }\n\n\
             [patch.crates-io]\nserde = { path = \"fork\" }\n",
    )
    .expect("the root the package names");
    std::fs::write(
        package.join("Cargo.toml"),
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nworkspace = \"../../named\"\n\n\
             [dependencies]\ndep.workspace = true\nserde = \"1\"\n",
    )
    .expect("the package manifest that names no paths");
    std::fs::write(package.join("src/lib.rs"), "pub fn nothing() {}\n").expect("the source");
    std::fs::write(dependency.join("Cargo.toml"), "[package]\nname = \"dep\"\n")
        .expect("the dependency manifest");
    let inherited = dependency.join("src/lib.rs");
    std::fs::write(&inherited, "pub const ONE: u8 = 1;\n").expect("the dependency source");
    std::fs::create_dir_all(named.join("fork/src")).expect("the replacement");
    std::fs::write(
        named.join("fork/Cargo.toml"),
        "[package]\nname = \"serde\"\nversion = \"1.0.0\"\n",
    )
    .expect("the replacement manifest");
    let patched = named.join("fork/src/lib.rs");
    std::fs::write(&patched, "pub const ONE: u8 = 1;\n").expect("the replacement source");

    assert_eq!(
        workspace_roots(&package),
        vec![canonical(&named)],
        "the root the package names is the only root a build of it can read"
    );
    assert_eq!(
        workspace_root(&package).expect("the named root"),
        canonical(&named),
        "and the base the record is spelled against"
    );

    let (sources, unfollowed, _) = recorded_sources(&package, &named, &[]);
    assert!(unfollowed.is_empty(), "{unfollowed:?}");
    for input in [&inherited, &patched, &dependency.join("Cargo.toml")] {
        assert!(
            sources.contains(input),
            "the path and the patch the named root supplies are inputs to the build; the walk \
             found {sources:?}"
        );
    }

    let binary = record_over(&named, &sources);
    for input in [&inherited, &patched] {
        let content = std::fs::read_to_string(input).expect("the input");
        std::fs::write(input, "pub const ONE: u8 = 2;\n").expect("the changed input");
        assert_eq!(
            changed_sources(&binary, &package).expect("a readable record"),
            [Input::File((*input).clone())],
            "a change to {} must refuse the binary",
            input.display()
        );
        std::fs::write(input, content).expect("the restored input");
    }
}

#[test]
fn an_ancestor_a_package_does_not_name_contributes_nothing() {
    // Measured on the same tree: a build that both nests the package and claims
    // it as a member stops with `member of the wrong workspace`, and the root the
    // package names settles which `[workspace.dependencies]` entry and which
    // `[patch]` apply. So an ancestor's entry and fork are not inputs — refusing
    // a binary for a change to them would be a refusal no rebuild clears.
    let root = fixture("named-root-ancestor");
    let outer = root.join("outer");
    let package = outer.join("app");
    let named = root.join("named");
    std::fs::create_dir_all(package.join("src")).expect("the package");
    std::fs::create_dir_all(outer.join("outer-dep/src")).expect("the ancestor's path");
    std::fs::create_dir_all(outer.join("outer-fork/src")).expect("the ancestor's fork");
    std::fs::create_dir_all(named.join("dep/src")).expect("the dependency");
    std::fs::write(
        outer.join("Cargo.toml"),
        "[workspace]\nmembers = [\"app\"]\n\n\
             [workspace.dependencies]\ndep = { path = \"outer-dep\" }\n\n\
             [patch.crates-io]\nserde = { path = \"outer-fork\" }\n",
    )
    .expect("the ancestor manifest");
    let ignored_path = outer.join("outer-dep/src/lib.rs");
    std::fs::write(
        outer.join("outer-dep/Cargo.toml"),
        "[package]\nname = \"dep\"\n",
    )
    .expect("the ancestor's path manifest");
    std::fs::write(&ignored_path, "pub const ONE: u8 = 9;\n").expect("the ancestor's path source");
    std::fs::write(
        outer.join("outer-fork/Cargo.toml"),
        "[package]\nname = \"serde\"\nversion = \"1.0.0\"\n",
    )
    .expect("the ancestor's fork manifest");
    let ignored_fork = outer.join("outer-fork/src/lib.rs");
    std::fs::write(&ignored_fork, "pub const ONE: u8 = 9;\n").expect("the ancestor's fork source");
    std::fs::write(
        named.join("Cargo.toml"),
        "[workspace]\nmembers = [\"../outer/app\"]\n\n\
             [workspace.dependencies]\ndep = { path = \"dep\" }\n",
    )
    .expect("the root the package names");
    std::fs::write(named.join("dep/Cargo.toml"), "[package]\nname = \"dep\"\n")
        .expect("the dependency manifest");
    std::fs::write(named.join("dep/src/lib.rs"), "pub const ONE: u8 = 1;\n")
        .expect("the dependency source");
    std::fs::write(
        package.join("Cargo.toml"),
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nworkspace = \"../../named\"\n\n\
             [dependencies]\ndep.workspace = true\nserde = \"1\"\n",
    )
    .expect("the package manifest");
    std::fs::write(package.join("src/lib.rs"), "pub fn nothing() {}\n").expect("the source");

    assert_eq!(
        workspace_roots(&package),
        vec![canonical(&named)],
        "an ancestor manifest is not a root for a package that names its own"
    );

    let (sources, unfollowed, _) = recorded_sources(&package, &named, &[]);
    assert!(unfollowed.is_empty(), "{unfollowed:?}");
    for ignored in [&ignored_path, &ignored_fork] {
        assert!(
            !sources.contains(ignored),
            "a directory the ancestor names is not compiled; the walk found {sources:?}"
        );
    }

    let binary = record_over(&named, &sources);
    for ignored in [&ignored_path, &ignored_fork] {
        std::fs::write(ignored, "pub const ONE: u8 = 8;\n").expect("the changed directory");
        assert!(
            changed_sources(&binary, &package)
                .expect("a readable record")
                .is_empty(),
            "a change to {} must not refuse the binary",
            ignored.display()
        );
    }
}

#[test]
fn a_package_a_sibling_workspace_builds_is_recorded_with_that_workspace() {
    // Measured: `cargo build -p <path dependency>` from the sibling workspace
    // `ws1` builds that package's binary under ws1's resolution — the binary
    // prints ws1's fork for the dependency ws1 patches — while its
    // `dep.workspace = true` still comes from `ws2`, the root it names. Its
    // build script's environment differs from its own build's in nothing but
    // `OUT_DIR`, which is ws1's target directory, so that is the root the build
    // was invoked in. A record that named only the roots the tree names would
    // report a binary built from ws1's fork clean, and a patch in the
    // dependency's own workspace would refuse one no rebuild clears.
    let root = fixture("sibling-build");
    let sibling = root.join("ws1");
    let tool = sibling.join("tool");
    let package = root.join("pkg");
    let named = root.join("ws2");
    let dependency = named.join("dep");
    std::fs::create_dir_all(tool.join("src")).expect("the sibling's member");
    std::fs::create_dir_all(package.join("src")).expect("the package");
    std::fs::create_dir_all(dependency.join("src")).expect("the dependency");
    std::fs::create_dir_all(sibling.join("sibling-dep/src")).expect("the sibling's path");
    std::fs::create_dir_all(sibling.join("sibling-fork/src")).expect("the sibling's replacement");
    std::fs::write(
        sibling.join("Cargo.toml"),
        "[workspace]\nmembers = [\"tool\"]\n\n\
             [workspace.dependencies]\ndep = { path = \"sibling-dep\" }\n\n\
             [patch.crates-io]\nserde = { path = \"sibling-fork\" }\n",
    )
    .expect("the sibling workspace the build is invoked in");
    std::fs::write(sibling.join("Cargo.lock"), "version = 4\n")
        .expect("the lockfile the invocation's resolution read");
    std::fs::write(
        sibling.join("sibling-dep/Cargo.toml"),
        "[package]\nname = \"dep\"\n",
    )
    .expect("the sibling's inherited path manifest");
    std::fs::write(
        sibling.join("sibling-dep/src/lib.rs"),
        "pub const ONE: u8 = 9;\n",
    )
    .expect("the sibling's inherited path source");
    let applied = sibling.join("sibling-fork/src/lib.rs");
    std::fs::write(
        sibling.join("sibling-fork/Cargo.toml"),
        "[package]\nname = \"serde\"\nversion = \"1.0.0\"\n",
    )
    .expect("the sibling's replacement manifest");
    std::fs::write(&applied, "pub const ONE: u8 = 1;\n").expect("the sibling's replacement source");
    std::fs::write(
        tool.join("Cargo.toml"),
        "[package]\nname = \"tool\"\nversion = \"0.1.0\"\n\n\
             [dependencies]\npkg = { path = \"../../pkg\" }\n",
    )
    .expect("the sibling's member that takes the package by path");
    std::fs::write(tool.join("src/lib.rs"), "pub fn nothing() {}\n").expect("the member's source");
    std::fs::write(
        named.join("Cargo.toml"),
        "[workspace]\nmembers = [\"../pkg\"]\n\n\
             [workspace.dependencies]\ndep = { path = \"dep\" }\n\n\
             [patch.crates-io]\nserde = { path = \"fork\" }\n",
    )
    .expect("the root the package names");
    std::fs::write(dependency.join("Cargo.toml"), "[package]\nname = \"dep\"\n")
        .expect("the dependency manifest");
    let inherited = dependency.join("src/lib.rs");
    std::fs::write(&inherited, "pub const ONE: u8 = 1;\n").expect("the dependency source");
    std::fs::create_dir_all(named.join("fork/src")).expect("its own replacement");
    std::fs::write(
        named.join("fork/Cargo.toml"),
        "[package]\nname = \"serde\"\nversion = \"1.0.0\"\n",
    )
    .expect("its own replacement manifest");
    let own = named.join("fork/src/lib.rs");
    std::fs::write(&own, "pub const ONE: u8 = 1;\n").expect("its own replacement source");
    std::fs::write(
        package.join("Cargo.toml"),
        "[package]\nname = \"pkg\"\nversion = \"0.1.0\"\nworkspace = \"../ws2\"\n\n\
             [dependencies]\ndep.workspace = true\nserde = \"1\"\n",
    )
    .expect("the package manifest that names no paths");
    std::fs::write(package.join("src/lib.rs"), "pub fn nothing() {}\n").expect("the source");

    let (sources, unfollowed, _) =
        recorded_sources(&package, &named, std::slice::from_ref(&sibling));
    assert!(unfollowed.is_empty(), "{unfollowed:?}");
    for input in [
        &applied,
        &sibling.join("sibling-fork/Cargo.toml"),
        &sibling.join("Cargo.toml"),
        &sibling.join("Cargo.lock"),
    ] {
        assert!(
            sources.contains(input),
            "the sibling workspace's replacement, manifest and lockfile are inputs to a build it \
             invokes; the walk found {sources:?}"
        );
    }
    assert!(
        sources.contains(&inherited) && !sources.contains(&sibling.join("sibling-dep/src/lib.rs")),
        "the path the package inherits comes from the root it names, not from the workspace that \
         builds it; the walk found {sources:?}"
    );
    assert!(
        sources.contains(&own),
        "while its own root's replacement is what its own build compiles"
    );

    let binary = record_over(&named, &sources);
    std::fs::write(&applied, "pub const ONE: u8 = 2;\n").expect("the changed replacement");
    assert_eq!(
        changed_sources(&binary, &package).expect("a readable record"),
        [Input::File(applied)],
        "a change to the replacement this build compiles must refuse the binary"
    );

    // The build the sibling invokes for its own member takes the resolution's
    // patch — measured, the applied one — and the package's own root's patch is
    // not applied by it. The inherited path is still the package's own root's.
    let (consumer, unfollowed, _) = recorded_sources(&tool, &sibling, &[]);
    assert!(unfollowed.is_empty(), "{unfollowed:?}");
    for input in [&sibling.join("sibling-fork/src/lib.rs"), &inherited] {
        assert!(
            consumer.contains(input),
            "the member's build compiles the resolution's replacement and the path its dependency \
             inherits; the walk found {consumer:?}"
        );
    }
    assert!(
        !consumer.contains(&own),
        "a patch in a dependency's own workspace is not applied to a build that takes it; the walk \
         found {consumer:?}"
    );
}
