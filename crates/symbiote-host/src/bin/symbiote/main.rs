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
//! authorizes a dangerous command with `--yes`, an `yes` typed at the prompt
//! when stdin is a terminal, or a configured **authorization
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

fn run_with(arguments: Vec<String>) -> Result<i32, Box<dyn std::error::Error>> {
    run_with_input(arguments, &mut std::io::stdin())
}

/// The one place the pieces meet: parse, decide applicability, answer a help
/// request or the local `schema` command, then — for a daemon command — build
/// the operation and hand it to the gate, which either refuses it outright or
/// lets the session send it. Everything before the gate is local, so nothing
/// here can open a socket on behalf of an unauthorized invocation.
///
/// `input` is the standard input exactly one command reads — `add-credential`,
/// which takes a credential value and has nowhere else to take it from. It is
/// a parameter so a test can hand that command its value without a value ever
/// being typed at the test runner's own terminal.
fn run_with_input(
    arguments: Vec<String>,
    input: &mut dyn std::io::Read,
) -> Result<i32, Box<dyn std::error::Error>> {
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
    // Publishing a runtime inventory is local too, and deliberately so: the
    // wire surface for the runtime inventory is one owner-only *read*, so
    // installing a document into the state directory is an operator action on
    // this machine, not something a client can ask a daemon to do. It needs the
    // private state directory and nothing else — no daemon, and no
    // authorization, because it is not an operation this binary sends.
    if name == "publish-runtime-inventory" {
        return run_publish_runtime_inventory(&options);
    }
    // `init` is local too, and more so: the file it writes is operator
    // authority, read once at daemon start, so nothing here reaches a
    // daemon even to ask one to write it. It needs the directory to write
    // into and nothing else.
    if name == "init" {
        return run_init(&options);
    }
    // `add-credential` is local for the same reason, and more sharply: its
    // input is a secret, and a request would carry it over a socket, into a
    // process table and toward a journal that records everything else.
    if name == "add-credential" {
        return run_add_credential(&options, input);
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

/// The local `schema` command. An optional selector names one published/// document (`envelope`, `policy`) or, absent, both. `--write DIR` regenerates
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

/// The local `init` command: write an operator configuration the daemon will
/// accept.
///
/// The daemon's operator config is 0600 by construction — it refuses a file
/// with any group or other permission bit, because it holds credential values
/// and a sandbox program allowlist — and it is `deny_unknown_fields`, so a
/// typo in it silently removes a control rather than failing to apply. That
/// makes it a file worth generating rather than writing: an operator who has
/// never run this product has no reason to know the field names, and a
/// hand-written one is refused in prose by a daemon that is not otherwise
/// running yet.
///
/// What it writes is the smallest configuration that turns
/// `run_started_dispatch` from "always refuses" into a working execution path:
/// a reservation base for derived worktrees, and the explicitly labeled
/// fixture transport that demonstrates a native dispatch with no model turn and
/// no spending. It writes nothing else. A credential registration, a sandbox
/// shell executor, an external harness and a compatibility dossier are all
/// operator choices about this machine, and naming them in a generated file
/// would put a value in the record that nobody chose.
///
/// The file is written 0600 through a temporary file in the same directory and
/// renamed into place, so a reader — a daemon starting at the same moment —
/// sees the old file or the new one and never a partial write. An existing
/// file is never overwritten: an operator's configuration is theirs, and this
/// command is not a way to replace one silently.
///
/// It exits 0 on an install and 1 on a usage or refusal, the same distinction
/// `schema --check` makes, because no daemon is asked to do anything else.
fn run_init(options: &Options) -> Result<i32, Box<dyn std::error::Error>> {
    let [directory, base] = options.args.as_slice() else {
        eprintln!(
            "symbiote: `init` takes an output directory and a worktree reservation base; try \
             `symbiote help`"
        );
        return Ok(EXIT_USAGE);
    };
    // Naming the directory is worth the one line: a create that fails inside a
    // directory that was never there reports the temporary file, which is a
    // name the operator never typed and cannot act on.
    if !std::path::Path::new(directory).is_dir() {
        eprintln!("symbiote: {directory} is not a directory; `init` writes into one");
        return Ok(EXIT_USAGE);
    }
    let path = std::path::Path::new(directory).join("operator.json");
    if path.exists() {
        eprintln!(
            "symbiote: {} already exists; an operator configuration is the operator's, and \
             `init` never replaces one",
            path.display()
        );
        return Ok(EXIT_USAGE);
    }
    // The reservation base is where derived worktrees are materialised, and it
    // is the one field with no default: a worktree under the state directory
    // would be inside the store, and one under the current directory would be
    // wherever a script happened to be run from.
    if base.is_empty() || !std::path::Path::new(base).is_absolute() {
        eprintln!("symbiote: the worktree reservation base must be an absolute path; got {base:?}");
        return Ok(EXIT_USAGE);
    }
    let document = serde_json::json!({
        "reservation_base": base,
        "native_fixture": {
            "echo_text": "The agent ran end to end inside its own worktree.",
        },
    });
    let mut bytes = serde_json::to_vec_pretty(&document)?;
    bytes.push(b'\n');
    if let Err(error) = install_private_file(&path, &bytes) {
        eprintln!("symbiote: cannot write {}: {error}", path.display());
        return Ok(EXIT_USAGE);
    }
    println!("wrote {}", path.display());
    println!(
        "start the Host with: symbioted --state-dir DIR --operator-config {}",
        path.display()
    );
    Ok(EXIT_OK)
}

/// Installs an operator-owned file at 0600, atomically.
///
/// The temporary file carries 0600 from the moment it exists: a file that was
/// briefly world-readable would have leaked the configuration before the
/// rename narrowed it, and a configuration holds credential values. The
/// rename means a daemon starting at the same moment sees the old file or the
/// new one, never a partial write.
fn install_private_file(path: &std::path::Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    use std::os::unix::fs::PermissionsExt;

    let temporary = path.with_extension("tmp");
    let written = (|| -> std::io::Result<()> {
        let mut file = std::fs::File::create(&temporary)?;
        file.set_permissions(std::fs::Permissions::from_mode(0o600))?;
        file.write_all(bytes)?;
        file.sync_all()
    })();
    if let Err(error) = written {
        let _ = std::fs::remove_file(&temporary);
        return Err(error);
    }
    if let Err(error) = std::fs::rename(&temporary, path) {
        let _ = std::fs::remove_file(&temporary);
        return Err(error);
    }
    Ok(())
}

/// The local `add-credential` command: record one credential registration in
/// an operator configuration.
///
/// Every runtime profile names a credential, and a native run refuses
/// without one by name —
///
///     no credential broker configured for this dispatch's credential
///     reference
///
/// — so a Host that has never run cannot execute a dispatch at all. The
/// registration is operator authority whose value lives in this one 0600
/// file and nowhere else, which is why this is a local command: a value must
/// never be an argument to a request, appear in a process table, or reach the
/// journal. It is read from standard input for that reason, and `init` writes
/// the file it goes into.
///
/// A registration is never replaced. The broker refuses re-registration by
/// design — a revoked reference can never come back, and rotation uses a NEW
/// reference id — so a command that overwrote one would write a file the
/// daemon then refuses, after telling the operator it succeeded.
fn run_add_credential(
    options: &Options,
    input: &mut dyn std::io::Read,
) -> Result<i32, Box<dyn std::error::Error>> {
    let [path, reference, project, environment] = options.args.as_slice() else {
        eprintln!(
            "symbiote: `add-credential` takes a configuration file, a credential reference, an \
             owning project and an environment label, then reads the value from stdin; try \
             `symbiote help`"
        );
        return Ok(EXIT_USAGE);
    };
    let path = std::path::Path::new(path);
    // The two identities are the daemon's own: a reference the broker cannot
    // name and a project it cannot match are refused at registration, and the
    // operator should learn that here rather than from a daemon that refuses
    // to start.
    if symbiote_domain::CredentialReferenceId::new(reference).is_err() {
        eprintln!("symbiote: {reference:?} is not a credential reference the broker can hold");
        return Ok(EXIT_USAGE);
    }
    if symbiote_domain::ProjectId::new(project).is_err() {
        eprintln!("symbiote: {project:?} is not a project id");
        return Ok(EXIT_USAGE);
    }
    if environment.is_empty() || environment.len() > 128 {
        eprintln!("symbiote: the environment label must be 1..=128 bytes; got {environment:?}");
        return Ok(EXIT_USAGE);
    }
    let mut value = String::new();
    if let Err(error) = input.read_to_string(&mut value) {
        eprintln!("symbiote: cannot read the credential value from stdin: {error}");
        return Ok(EXIT_USAGE);
    }
    // One trailing newline is the line a shell or an editor adds; anything
    // else is the operator's value and is kept byte for byte.
    let value = value.strip_suffix('\n').unwrap_or(&value).to_owned();
    if value.is_empty() {
        eprintln!("symbiote: an empty credential value registers nothing; pipe the value in");
        return Ok(EXIT_USAGE);
    }
    let Ok(bytes) = std::fs::read(path) else {
        eprintln!(
            "symbiote: cannot read {}; `symbiote init` writes one",
            path.display()
        );
        return Ok(EXIT_USAGE);
    };
    let mut document = match serde_json::from_slice::<serde_json::Value>(&bytes) {
        Ok(serde_json::Value::Object(document)) => document,
        _ => {
            eprintln!(
                "symbiote: {} is not a JSON object; the daemon loads the configuration itself and \
                 this command will not guess at it",
                path.display()
            );
            return Ok(EXIT_USAGE);
        }
    };
    let mut registrations = match document.remove("credential_broker") {
        None => Vec::new(),
        Some(serde_json::Value::Array(registrations)) => registrations,
        Some(_) => {
            eprintln!(
                "symbiote: {} has a `credential_broker` that is not a list",
                path.display()
            );
            return Ok(EXIT_USAGE);
        }
    };
    if registrations
        .iter()
        .any(|registration| registration["reference"] == *reference)
    {
        eprintln!(
            "symbiote: {reference} is already registered in {}; a registration is never \
             replaced, so rotate by registering a NEW reference",
            path.display()
        );
        return Ok(EXIT_USAGE);
    }
    registrations.push(serde_json::json!({
        "reference": reference,
        "project": project,
        "environment": environment,
        "value": value,
    }));
    document.insert("credential_broker".into(), registrations.into());
    let mut written = serde_json::to_vec_pretty(&document)?;
    written.push(b'\n');
    if let Err(error) = install_private_file(path, &written) {
        eprintln!("symbiote: cannot write {}: {error}", path.display());
        return Ok(EXIT_USAGE);
    }
    // The daemon reads the file once at start, so a registration made after
    // it is running is not live until it is restarted. Saying so is the
    // difference between an operator who restarts and one who debugs.
    println!("registered {reference} for {project} in {}", path.display());
    println!("restart the Host to load it; the broker holds values in process memory only");
    Ok(EXIT_OK)
}

/// The local `publish-runtime-inventory` command: install a discovery document
/// as the runtime inventory the daemon's `get_runtime_inventory` read serves.
///
/// It is a publish, not an operation: the document is re-validated in full by
/// the discovery contract, every record must name this state directory's own
/// Host identity, and the file is installed atomically at mode 0600. A refusal
/// is named and leaves whatever the Host already held in place, so a bad
/// document cannot unpublish a good one. The exit code is 1 for a usage or
/// refusal and 0 for an install — the same distinction `schema --check` makes,
/// because neither this command nor a daemon is asked to do anything else.
fn run_publish_runtime_inventory(options: &Options) -> Result<i32, Box<dyn std::error::Error>> {
    let [document] = options.args.as_slice() else {
        eprintln!(
            "symbiote: `publish-runtime-inventory` takes exactly one discovery document; try \
             `symbiote help`"
        );
        return Ok(EXIT_USAGE);
    };
    let Some(directory) = options.state_dir.clone() else {
        eprintln!("symbiote: missing --state-dir PRIVATE_DIRECTORY");
        return Ok(EXIT_USAGE);
    };
    let host_id = match symbiote_host::host_identity(&directory) {
        Ok(host_id) => host_id,
        Err(error) => {
            eprintln!(
                "symbiote: {} is not a usable Host state directory: {error}",
                directory.display()
            );
            return Ok(EXIT_USAGE);
        }
    };
    let published = symbiote_host::runtime_inventory::Published::new(&directory, host_id);
    let bytes = match std::fs::read(document) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("symbiote: cannot read {document}: {error}");
            return Ok(EXIT_USAGE);
        }
    };
    let text = match String::from_utf8(bytes) {
        Ok(text) => text,
        Err(_) => {
            eprintln!("symbiote: {document} is not UTF-8 text");
            return Ok(EXIT_USAGE);
        }
    };
    match published.publish(&text) {
        Ok(()) => {
            println!(
                "published {} record(s) to {}",
                // A publish that succeeded has been re-validated whole, so this
                // read cannot fail: the file is exactly what was accepted.
                published
                    .read()
                    .map(|inventory| inventory.records().len())
                    .unwrap_or(0),
                published.path().display()
            );
            Ok(EXIT_OK)
        }
        Err(error) => {
            eprintln!(
                "symbiote: {document} was not published ({}); the Host keeps the inventory it \
                 already held",
                error.name()
            );
            Ok(EXIT_USAGE)
        }
    }
}

#[cfg(test)]
mod tests;
