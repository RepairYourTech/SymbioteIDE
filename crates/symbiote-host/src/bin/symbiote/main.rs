//! `symbiote` administrative CLI: a scriptable, human-readable client for a
//! running Host daemon, using the same authenticated Unix-socket protocol as
//! every other controller. Administrative scope only — the interactive and
//! headless coding-agent experience (#467) is a separate surface that shares
//! this client plumbing; no model loop exists here.
//!
//! Commands map one-to-one onto typed protocol operations. Every response is
//! the daemon's own JSON (machine-readable, stable field names); `--json`
//! wraps it in a versioned envelope for automation. Exit codes:
//! 0 success, 1 connection/usage failure, 2 the daemon refused the command
//! (`Err` response), 3 authorization required for a dangerous operation —
//! distinct exits so scripts can branch on refusal vs transport failure vs
//! a missing authorization.
//!
//! The CLI holds no daemon authority of its own: it cannot widen what the
//! protocol permits. What it does own is the decision to *send* a dangerous
//! request at all, and that decision never happens silently. An invocation
//! authorizes a dangerous command one of three ways — `--yes`, an `yes` typed
//! at the prompt when stdin is a terminal, or a configured **authorization
//! policy** (`--policy FILE`, else `SYMBIOTE_CLI_POLICY`) that pre-authorizes
//! the named operation kinds. Otherwise the CLI refuses **before connecting**,
//! so an unattended script cannot stop a Host, start work, or smuggle either
//! through `raw` by omitting a flag.
//!
//! A policy is a pre-authorization, never a widening: it is only consulted
//! for an operation that is already dangerous, it is read with the same
//! private-file discipline the Host applies to its own operator config
//! (regular file, owned by this user, no group/other bits, bounded), and a
//! policy that is missing, malformed, insecure or naming an unknown or
//! non-dangerous kind is a usage failure (exit 1) that sends nothing. It
//! cannot grant a permission the daemon would refuse, and it does not raise
//! the CLI's own authority by one bit.
//!
//! This file is only the entry point and the dispatch. The pieces it composes
//! live beside it: `args` owns the option/flag model, `commands` the operation
//! table and help text, `output` the envelopes and the prompt, `gate` whether
//! a dangerous operation may be sent, and `session` the socket round trip. The
//! authorization engine the gate consults is the library's
//! `symbiote_host::cli_authorization`, kept next to the policy schema it
//! enforces.
mod args;
mod commands;
mod gate;
mod output;
mod session;

use std::process::exit;

use symbiote_host::cli_schema::{
    document_text, published_schemas, schema_drift, selected_schemas, selectors, write_schemas,
};

use crate::args::{EXIT_OK, EXIT_USAGE, Options, honored_flags, parse_options};
use crate::commands::{commands, print_help};
use crate::gate::Gate;

fn main() {
    match run() {
        Ok(code) => exit(code),
        Err(error) => {
            eprintln!("symbiote: {error}");
            exit(EXIT_USAGE);
        }
    }
}

fn run() -> Result<i32, Box<dyn std::error::Error>> {
    run_with(std::env::args().skip(1).collect())
}

/// The one place the pieces meet: parse, decide applicability, answer a help
/// request or the local `schema` command, then — for a daemon command — build
/// the operation and hand it to the gate, which either refuses it outright or
/// lets the session send it. Everything before the gate is local, so nothing
/// here can open a socket on behalf of an unauthorized invocation.
fn run_with(arguments: Vec<String>) -> Result<i32, Box<dyn std::error::Error>> {
    let options = parse_options(&arguments)?;
    // A help request — the `help` command, or `--help`/`-h` with or without a
    // command alongside — prints the table and connects to nothing. It cannot
    // honor any other flag either: help renders the table, not a request, so
    // `--json`, `--state-dir` and the rest mean nothing to it.
    let requested_help = options.help || options.command.as_deref() == Some("help");
    // A flag a command cannot honor is a usage error that names it, never a
    // silent no-op. This is the one place flag applicability is decided — for
    // the local commands, the daemon commands and a help request alike —
    // before any of them is answered or connects. A help request is validated
    // against the `help` row, so `--json --help health` is refused by the same
    // rule as `--json --help help`.
    let honored = if requested_help {
        honored_flags("help")
    } else {
        match options.command.as_deref() {
            Some(name) => honored_flags(name),
            None => {
                // A bare invocation is a usage error, and the table is shown
                // either way.
                print_help();
                return Ok(EXIT_USAGE);
            }
        }
    };
    let unsuited = options.supplied().unsuited_for(honored);
    if !unsuited.is_empty() {
        let named = if requested_help {
            "help"
        } else {
            options.command.as_deref().unwrap_or("help")
        };
        eprintln!(
            "symbiote: `{named}` does not accept {}; try `symbiote help`",
            unsuited.join(", ")
        );
        return Ok(EXIT_USAGE);
    }
    if requested_help {
        print_help();
        return Ok(EXIT_OK);
    }
    let name = options
        .command
        .clone()
        .expect("a non-help invocation names a command");
    // `schema` is local: it emits and compares this binary's own contract and
    // needs no daemon, no state directory and no authorization, so it is
    // answered before any of that is consulted.
    if name == "schema" {
        return run_schema(&options);
    }
    let Some(command) = commands().into_iter().find(|c| c.name == name) else {
        eprintln!("symbiote: unknown command {name}; try `symbiote help`");
        return Ok(EXIT_USAGE);
    };
    let args = &options.args;
    let mut operation = serde_json::Map::new();
    if let Err(usage) = (command.build)(args, &mut operation) {
        eprintln!("symbiote: {usage}");
        eprintln!("usage: symbiote --state-dir DIR {}", command.usage);
        return Ok(EXIT_USAGE);
    }
    // Authorization follows the OPERATION, not the command name: `raw` and the
    // typed commands are classified by the same table, so `raw` can neither
    // bypass the gate on `shutdown` nor be over-gated on a read.
    let kind = operation
        .get("kind")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_owned();
    // The gate reports its own refusal; only an admitted operation is sent.
    match gate::admit(&name, &kind, &options)? {
        Gate::Refused(code) => Ok(code),
        Gate::Proceed => session::send(&name, &options, operation),
    }
}

/// The local `schema` command. An optional selector names one published
/// document (`envelope`, `policy`) or, absent, both. `--write DIR` regenerates
/// the selected files; `--check DIR` reports how the selected files diverge
/// from what this binary emits and writes nothing, so it is the local half of
/// the CI drift step. Neither touches the daemon.
fn run_schema(options: &Options) -> Result<i32, Box<dyn std::error::Error>> {
    let selector = match options.args.as_slice() {
        [] => None,
        [only] => Some(only.as_str()),
        _ => {
            eprintln!(
                "symbiote: `schema` takes at most one of {}; try `symbiote help`",
                selectors().join(" or ")
            );
            return Ok(EXIT_USAGE);
        }
    };
    if options.write.is_some() && options.check.is_some() {
        eprintln!("symbiote: `schema` accepts either --write DIR or --check DIR, not both");
        return Ok(EXIT_USAGE);
    }
    let selected = match selected_schemas(selector) {
        Ok(selected) => selected,
        Err(error) => {
            eprintln!("symbiote: {error}; try `symbiote help`");
            return Ok(EXIT_USAGE);
        }
    };
    if let Some(directory) = &options.check {
        let drift = match schema_drift(directory, &selected) {
            Ok(drift) => drift,
            Err(error) => {
                eprintln!("symbiote: {error}");
                return Ok(EXIT_USAGE);
            }
        };
        if drift.is_empty() {
            return Ok(EXIT_OK);
        }
        for entry in &drift {
            eprintln!("symbiote: {} {}", entry.file_name, entry.detail);
        }
        eprintln!(
            "symbiote: {} is not what this binary publishes ({} difference(s)); `symbiote schema --write DIR` regenerates the documents",
            directory.display(),
            drift.len()
        );
        return Ok(EXIT_USAGE);
    }
    if let Some(directory) = &options.write {
        write_schemas(directory, &selected)?;
        return Ok(EXIT_OK);
    }
    match selector {
        // The bare command prints one object keyed by schema identity.
        None => println!("{}", serde_json::to_string_pretty(&published_schemas())?),
        // A selector prints that document's canonical bytes: exactly what
        // `schema --write` would put in its file.
        Some(_) => print!("{}", document_text(selected[0])?),
    }
    Ok(EXIT_OK)
}

#[cfg(test)]
mod tests;
