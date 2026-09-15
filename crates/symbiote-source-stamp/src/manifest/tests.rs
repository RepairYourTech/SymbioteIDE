//! What a manifest's text says: the dependency and `[patch]` edges in whichever
//! spelling a manifest gives one, the tables cargo ignores, and the workspace a
//! package names for itself.

use super::*;

use crate::tests::{fixture, paths};
use crate::walk::paths::canonical;
use crate::walk::recorded_sources;
use crate::walk::roots::{workspace_root, workspace_roots};

#[test]
fn dependency_tables_are_read_and_dev_and_target_tables_are_not() {
    let manifest = "\
[package]
name = \"example\"

[dependencies]
symbiote-a = { path = \"../a\" }
serde = { version = \"1\", path = \"../serde\", features = [\"derive\"] }
comma = { path = \"../a,b\", features = [\"x\", \"y\"] }
nix = \"0.30\"
symbiote-c.path = \"../c\"
symbiote-d.version = \"1\"

[dependencies.symbiote-b]
path = \"../b\"
version = \"1.0\"

[build-dependencies]
stamp = { path = \"../stamp\" }

[build-dependencies.helper]
path = \"../helper\"

[dev-dependencies]
symbiote-test = { path = \"../test\" }

[dev-dependencies.dev-only]
path = \"../dev-only\"

[target.'cfg(unix)'.dependencies]
unix = { path = \"../unix\" }

[target.'cfg(unix)'.dependencies.unix-two]
path = \"../unix-two\"

[target.'cfg(unix)'.dev-dependencies]
unix-test = { path = \"../unix-test\" }

[target.'cfg(unix)'.dev-dependencies.unix-three]
path = \"../unix-three\"

[[bin]]
name = \"example\"
path = \"src/bin/example.rs\"
";
    assert_eq!(
        paths(manifest),
        [
            "../a,b",
            "../serde",
            "../a",
            "../c",
            "../b",
            "../stamp",
            "../helper",
            "../unix",
            "../unix-two",
        ],
        "every edge cargo compiles is read, in whichever spelling it allows, and no \
             dev-dependency edge is: `symbiote-d.version` names no path, every dev-dependency \
             edge is skipped, and a path holding a comma is one path"
    );
}

#[test]
fn a_declaration_that_inherits_is_read_from_the_workspace_root() {
    // Measured: `workspace = true` makes the *root's* path the one cargo
    // resolves, and a `path` written beside it is ignored in every spelling — so
    // a member that inherits names no edge of its own, and the walk has to read
    // the path from the root that declares it.
    let member = "\
[package]
name = \"app\"

[dependencies]
inline.workspace = true
dotted.workspace = true
ignored = { workspace = true, path = \"../never-compiled\" }

[dependencies.tabled]
workspace = true
path = \"../never-compiled-either\"

[build-dependencies]
stamp.workspace = true

[dev-dependencies]
test-only.workspace = true

[workspace.dependencies]
root-only = { path = \"../root-only\" }
";
    assert!(
        read(member).edges.is_empty(),
        "a declaration that inherits names no path of its own, and the workspace table is not an \
         edge of the manifest that declares it; the reader found {:?}",
        paths(member)
    );
    assert_eq!(
        read(member)
            .inherited
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["dotted", "ignored", "inline", "stamp", "tabled"],
        "every inherited dependency is named, in whichever spelling, and no dev-dependency edge is"
    );
    assert_eq!(
        read(member).shared,
        [("root-only", "../root-only")]
            .into_iter()
            .map(|(name, path)| (name.to_owned(), path.to_owned()))
            .collect::<BTreeMap<_, _>>(),
        "a path declared beside a member's own edges is a path for members to inherit"
    );

    let root = "\
[workspace]
members = [\"app\"]

[workspace.dependencies]
inline = { path = \"../inline\" }
dotted.path = \"../dotted\"
features = { path = \"../features\", features = [\"a\", \"b\"] }
registry = { version = \"1\" }

[workspace.dependencies.tabled]
path = \"../tabled\"
";
    assert_eq!(
        read(root).shared,
        [
            ("dotted", "../dotted"),
            ("features", "../features"),
            ("inline", "../inline"),
            ("tabled", "../tabled"),
        ]
        .into_iter()
        .map(|(name, path)| (name.to_owned(), path.to_owned()))
        .collect::<BTreeMap<_, _>>(),
        "the root's paths are read in every spelling, and an entry that names no path is not one"
    );
}

#[test]
fn a_comment_declares_no_edge_and_is_not_part_of_a_path() {
    // A commented-out dependency is ordinary in a manifest, and the walk
    // read one as an edge — so a commented path that is gone stopped a
    // build cargo accepts — while a comment left beside a declaration ran
    // into the path it was read as. A `#` inside a path is a character of
    // that path, where neither is true.
    for text in ["# dep = { path = \"../gone\" }", "# dep.path = \"../gone\""] {
        assert!(
            paths(&format!("[dependencies]\n{text}\n")).is_empty(),
            "{text} declares no edge, so recording one would refuse a current binary for a \
                 change to a directory no rebuild reads"
        );
    }
    assert_eq!(
        paths(
            "[dependencies]\ndep = { path = \"../dep\" } # pinned to the fork\n\
                 dep2 = { path = \"../a#b\" }\n"
        ),
        ["../dep", "../a#b"],
        "a comment beside a declaration is not part of its path, while the `#` of a quoted \
             path is"
    );
}

#[test]
fn a_split_at_an_unquoted_delimiter_keeps_quoted_text_whole() {
    assert_eq!(
        split_unquoted("target.'cfg(unix)'.dependencies", '.'),
        ["target", "'cfg(unix)'", "dependencies"]
    );
    assert_eq!(
        split_unquoted("patch.\"https://github.com/a.b\".fork", '.'),
        ["patch", "\"https://github.com/a.b\"", "fork"]
    );
    assert_eq!(split_unquoted("workspace", '.'), ["workspace"]);
    assert_eq!(split_unquoted("[bin]", '.'), ["[bin]"]);
    // The comma of a path is inside its quotes and stays one value; the commas of
    // a `features` list are outside them, and the parts they cut name no key, so
    // the reader of a declaration ignores them.
    assert_eq!(
        split_unquoted("path = \"../a,b\", features = [\"x\", \"y\"]", ','),
        ["path = \"../a,b\"", " features = [\"x\"", " \"y\"]"]
    );
}

#[test]
fn a_path_dependency_split_across_lines_is_not_claimed() {
    assert!(paths("[dependencies]\nx = {\n  path = \"../x\"\n}\n").is_empty());
}

#[test]
fn an_explicit_workspace_key_is_read_as_the_scalar_cargo_spells() {
    // `workspace = "../other"` in `[package]` names the root a package
    // inherits from, and cargo spells it as a plain string rather than an
    // inline table. `../other` is not an ancestor of the package, so only
    // the explicit key reaches it, and the header carries the spacing and
    // the comment TOML allows. Measured: the named root is the root cargo
    // resolves such a package to, so it is the only root the walk asks, and
    // the base a record of it is spelled against.
    let root = fixture("explicit-workspace");
    let package = root.join("app/crate");
    let target = root.join("app/other");
    std::fs::create_dir_all(package.join("src")).expect("the package");
    std::fs::create_dir_all(target.join("src")).expect("the target");
    std::fs::write(target.join("Cargo.toml"), "[ workspace ] # the root\n")
        .expect("the target manifest");
    std::fs::write(target.join("Cargo.lock"), "version = 4\n").expect("the target lockfile");
    std::fs::write(target.join("src/lib.rs"), "pub fn nothing() {}\n").expect("the target source");
    std::fs::write(
        package.join("Cargo.toml"),
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nworkspace = \"../other\" # the root\n",
    )
    .expect("the package manifest");
    std::fs::write(package.join("src/lib.rs"), "pub fn nothing() {}\n")
        .expect("the package source");

    assert_eq!(
        workspace_roots(&package),
        vec![canonical(&target)],
        "the named root is the only root a build of this package can read"
    );
    assert_eq!(
        workspace_root(&package).expect("the named root"),
        canonical(&target),
        "the named root is the base the record's locators are spelled against"
    );

    let (sources, unfollowed, _) = recorded_sources(&package, &root, &[]);
    assert!(unfollowed.is_empty(), "{unfollowed:?}");
    for shared in ["Cargo.toml", "Cargo.lock"] {
        assert!(
            sources.contains(&target.join(shared)),
            "the root a package names must be recorded; the walk found {sources:?}"
        );
    }
}
