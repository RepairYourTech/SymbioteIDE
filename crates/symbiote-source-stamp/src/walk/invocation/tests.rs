//! What the invocation readings do with a cargo command line: which argument is
//! the subcommand, and the manifest the line names — in the line itself or in the
//! alias it expands to. Both come from the command line of the cargo that
//! spawned the build, so a line read wrongly places the build in a workspace it
//! did not resolve in, and the record then omits the `[patch]` fork it compiled.

#![cfg(target_os = "linux")]

use super::{expanded_arguments, manifest_path, subcommand_index};

use crate::tests::fixture;
use crate::walk::paths::canonical;

fn line(arguments: &[&str]) -> Vec<String> {
    arguments
        .iter()
        .map(|argument| (*argument).to_owned())
        .collect()
}

#[test]
fn the_subcommand_is_the_first_argument_that_is_neither_an_option_nor_a_toolchain() {
    // The index decides where alias expansion starts. A `+toolchain` argument is
    // one argument: read as an option with a value it consumes the word after it,
    // and the word after it is the subcommand.
    for (name, command_line, expected) in [
        ("plain", ["cargo", "build"].as_slice(), Some(1)),
        (
            "toolchain",
            ["cargo", "+nightly", "build"].as_slice(),
            Some(2),
        ),
        (
            "toolchain before an alias",
            ["cargo", "+nightly", "b", "--offline"].as_slice(),
            Some(2),
        ),
        (
            "color with its value",
            ["cargo", "--color", "always", "build"].as_slice(),
            Some(3),
        ),
        (
            "config with its value",
            ["cargo", "--config", "net.offline=true", "build"].as_slice(),
            Some(3),
        ),
        (
            "color and its value in one argument",
            ["cargo", "--color=always", "build"].as_slice(),
            Some(2),
        ),
        (
            "options and no subcommand",
            ["cargo", "--offline"].as_slice(),
            None,
        ),
        (
            "a toolchain and no subcommand",
            ["cargo", "+nightly"].as_slice(),
            None,
        ),
    ] {
        assert_eq!(
            subcommand_index(&line(command_line)),
            expected,
            "{name}: {command_line:?}"
        );
    }
}

#[test]
fn a_toolchain_argument_does_not_swallow_the_subcommand_an_alias_expands_from() {
    // The arrangement the crate documents behind a configuration file: run in the
    // third workspace, the subcommand is an alias whose expansion names the
    // sibling's manifest, so the command line itself carries no manifest. Read as
    // an option with a value, a `+toolchain` argument ahead of the alias consumes
    // it, no subcommand is found, no expansion is read, and the line names no
    // manifest — the run directory's workspace is recorded while the fork the
    // build compiled goes unnamed.
    let root = fixture("invocation-toolchain");
    let member = root.join("sibling/member");
    std::fs::create_dir_all(root.join("third/.cargo")).expect("the configuration directory");
    std::fs::create_dir_all(&member).expect("the sibling's member");
    std::fs::write(
        root.join("third/.cargo/config.toml"),
        "[alias]\nb = \"build --manifest-path ../sibling/member/Cargo.toml\"\n",
    )
    .expect("the alias");
    std::fs::write(member.join("Cargo.toml"), "[package]\nname = \"member\"\n")
        .expect("the manifest the alias names");
    let directory = root.join("third");
    let manifest = canonical(&member.join("Cargo.toml"));
    for command_line in [
        ["cargo", "b", "--offline"].as_slice(),
        ["cargo", "+nightly", "b", "--offline"].as_slice(),
    ] {
        assert_eq!(
            manifest_path(&line(command_line), &directory),
            Some(manifest.clone()),
            "the alias names the manifest, {command_line:?} included"
        );
    }
    assert_eq!(
        expanded_arguments(&line(&["cargo", "+nightly", "b", "--offline"]), &directory),
        line(&[
            "build",
            "--manifest-path",
            "../sibling/member/Cargo.toml",
            "--offline",
        ]),
        "the expansion replaces the subcommand and keeps the rest of the line"
    );
}
