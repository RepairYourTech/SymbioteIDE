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
//! request at all, and that decision never happens silently. A dangerous
//! command requires `--yes`, or an interactive `yes` at the prompt when
//! stdin is a terminal; otherwise the CLI refuses **before connecting**, so
//! an unattended script cannot stop a Host, start work, or smuggle either
//! through `raw` by omitting a flag.
use std::io::{IsTerminal, Write};
use std::path::PathBuf;
use std::process::exit;
use std::sync::atomic::{AtomicU64, Ordering};

/// Exit codes, stable for scripts.
const EXIT_OK: i32 = 0;
const EXIT_USAGE: i32 = 1;
const EXIT_REFUSED: i32 = 2;
const EXIT_AUTHORIZATION_REQUIRED: i32 = 3;

/// The machine-readable envelope's schema identity. Automation keys on this
/// string, not on the presence of individual fields.
pub const CLI_SCHEMA: &str = "symbiote.cli/v1";

fn main() {
    match run() {
        Ok(code) => exit(code),
        Err(error) => {
            eprintln!("symbiote: {error}");
            exit(EXIT_USAGE);
        }
    }
}

#[derive(Debug)]
struct Usage(String);

impl std::fmt::Display for Usage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for Usage {}

type Args<'a> = &'a [String];

/// What a command can do to the Host. The classification decides whether the
/// CLI sends it without an explicit authorization.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Risk {
    /// Reads only: no daemon state changes.
    ReadOnly,
    /// Changes daemon state but starts and stops nothing.
    Mutation,
    /// Starts or ends execution — or can express any operation (`raw`).
    Dangerous,
}

impl Risk {
    fn requires_authorization(self) -> bool {
        matches!(self, Risk::Dangerous)
    }
}

/// The client-side authorization decision: pure, so every branch is unit
/// tested without a daemon or a terminal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Authorization {
    /// Send the request as-is.
    Allowed,
    /// Ask on the terminal; only an explicit `yes` proceeds.
    Prompt,
    /// Refuse before connecting.
    Refused,
}

fn decide(risk: Risk, authorized: bool, interactive: bool) -> Authorization {
    if !risk.requires_authorization() || authorized {
        return Authorization::Allowed;
    }
    if interactive {
        Authorization::Prompt
    } else {
        Authorization::Refused
    }
}

/// Only the literal `yes` (trimmed, case-insensitive) authorizes. An empty
/// read (closed stdin) is a refusal, never a default.
fn confirmation_accepted(line: &str) -> bool {
    line.trim().eq_ignore_ascii_case("yes")
}

/// Risk per protocol operation kind. This ONE table classifies both a typed
/// command and the same operation supplied through `raw`, so the two paths
/// cannot drift apart — and the decision follows what the operation can do,
/// not which command name was typed. A kind this table does not know cannot
/// be proven safe, so it is Dangerous rather than silently allowed.
fn risk_of_kind(kind: &str) -> Risk {
    match kind {
        // Reads: no daemon state changes.
        "hello"
        | "health"
        | "get_host_pulse"
        | "get_project"
        | "get_task"
        | "read_journal"
        | "get_dispatch_preparation"
        | "get_scheduling_projection"
        | "get_team"
        | "get_task_origin"
        | "get_binding"
        | "get_binding_readiness"
        | "get_task_dependencies"
        | "get_route"
        | "resolve_route"
        | "get_work"
        | "get_provider_connection"
        | "get_billing_entitlement"
        | "get_model_descriptor"
        | "get_resource_consent" => Risk::ReadOnly,
        // Mutations: they change state, but they neither start or end
        // execution nor widen authority.
        "create_task"
        | "prepare_dispatch"
        | "request_task_completion"
        | "request_elevation"
        | "replace_binding"
        | "record_route"
        | "set_task_dependencies"
        | "acquire_task_lease"
        | "release_task_lease"
        | "expire_stale_leases"
        | "replace_provider_connection"
        | "replace_billing_entitlement"
        | "replace_model_descriptor"
        | "replace_team"
        | "create_work"
        | "change_work"
        | "assign_task_origin"
        | "register_project"
        | "observe_root_placement"
        | "revoke_elevation"
        | "revoke_resource_consent" => Risk::Mutation,
        // Starting or ending execution, or granting capability.
        "start_prepared_task"
        | "run_started_dispatch"
        | "shutdown"
        | "decide_elevation"
        | "record_resource_consent" => Risk::Dangerous,
        _ => Risk::Dangerous,
    }
}

/// One command: the operation JSON builder plus the kind it emits. `raw`
/// declares none, because the operator's file decides what it sends.
struct Command {
    name: &'static str,
    summary: &'static str,
    usage: &'static str,
    kind: Option<&'static str>,
    build: fn(Args, &mut serde_json::Map<String, serde_json::Value>) -> Result<(), Usage>,
}

fn field(args: Args, index: usize, name: &str) -> Result<String, Usage> {
    args.get(index)
        .cloned()
        .ok_or_else(|| Usage(format!("missing <{name}>")))
}

/// Every domain identity (TaskId, HostId, DispatchId, …) serializes as a
/// plain JSON string on the wire (serde try_from String).
fn id_field(
    args: Args,
    index: usize,
    key: &str,
    name: &str,
    map: &mut serde_json::Map<String, serde_json::Value>,
) -> Result<(), Usage> {
    let value = field(args, index, name)?;
    map.insert(key.to_owned(), serde_json::Value::String(value));
    Ok(())
}

fn plain(
    key: &str,
    value: serde_json::Value,
    map: &mut serde_json::Map<String, serde_json::Value>,
) {
    map.insert(key.to_owned(), value);
}

fn read_json(path: &str) -> Result<serde_json::Value, Usage> {
    let bytes =
        std::fs::read(path).map_err(|error| Usage(format!("cannot read {path}: {error}")))?;
    if bytes.len() > 60_000 {
        return Err(Usage(format!("{path} exceeds the 64 KiB request bound")));
    }
    serde_json::from_slice(&bytes)
        .map_err(|error| Usage(format!("{path} is not valid JSON: {error}")))
}

fn commands() -> Vec<Command> {
    vec![
        Command {
            name: "hello",
            summary: "negotiate protocol version and capabilities",
            usage: "hello",
            kind: Some("hello"),
            build: |_, map| {
                plain("kind", serde_json::json!("hello"), map);
                plain(
                    "supported_versions",
                    serde_json::json!([symbiote_protocol::CURRENT_VERSION]),
                    map,
                );
                Ok(())
            },
        },
        Command {
            name: "health",
            summary: "check the daemon is serving",
            usage: "health",
            kind: Some("health"),
            build: |_, map| {
                plain("kind", serde_json::json!("health"), map);
                Ok(())
            },
        },
        Command {
            name: "host-pulse",
            summary: "owner-only passive Host resource pulse",
            usage: "host-pulse",
            kind: Some("get_host_pulse"),
            build: |_, map| {
                plain("kind", serde_json::json!("get_host_pulse"), map);
                Ok(())
            },
        },
        Command {
            name: "shutdown",
            summary: "drain and stop the daemon",
            usage: "shutdown",
            kind: Some("shutdown"),
            build: |_, map| {
                plain("kind", serde_json::json!("shutdown"), map);
                Ok(())
            },
        },
        Command {
            name: "get-project",
            summary: "read a registered Project",
            usage: "get-project <project_id>",
            kind: Some("get_project"),
            build: |args, map| {
                plain("kind", serde_json::json!("get_project"), map);
                plain("project_id", field(args, 0, "project_id")?.into(), map);
                Ok(())
            },
        },
        Command {
            name: "create-task",
            summary: "create a Task from a draft (JSON file)",
            usage: "create-task <task.json>",
            kind: Some("create_task"),
            build: |args, map| {
                plain("kind", serde_json::json!("create_task"), map);
                plain("task", read_json(&field(args, 0, "task.json")?)?, map);
                Ok(())
            },
        },
        Command {
            name: "get-task",
            summary: "read a Task's durable state",
            usage: "get-task <project_id> <task_id>",
            kind: Some("get_task"),
            build: |args, map| {
                plain("kind", serde_json::json!("get_task"), map);
                plain("project_id", field(args, 0, "project_id")?.into(), map);
                id_field(args, 1, "task_id", "task_id", map)?;
                Ok(())
            },
        },
        Command {
            name: "read-journal",
            summary: "read a Project's journaled events after a sequence",
            usage: "read-journal <project_id> <after> <limit>",
            kind: Some("read_journal"),
            build: |args, map| {
                plain("kind", serde_json::json!("read_journal"), map);
                plain("project_id", field(args, 0, "project_id")?.into(), map);
                plain(
                    "after",
                    field(args, 1, "after")?
                        .parse::<u64>()
                        .map_err(|_| Usage("<after> must be a sequence number".into()))?
                        .into(),
                    map,
                );
                plain(
                    "limit",
                    field(args, 2, "limit")?
                        .parse::<u32>()
                        .map_err(|_| Usage("<limit> must be a page size".into()))?
                        .into(),
                    map,
                );
                Ok(())
            },
        },
        Command {
            name: "prepare-dispatch",
            summary: "compose a dispatch preparation for a Task",
            usage: "prepare-dispatch <task_id>",
            kind: Some("prepare_dispatch"),
            build: |args, map| {
                plain("kind", serde_json::json!("prepare_dispatch"), map);
                id_field(args, 0, "task_id", "task_id", map)?;
                Ok(())
            },
        },
        Command {
            name: "get-dispatch-preparation",
            summary: "read the recorded dispatch preparation",
            usage: "get-dispatch-preparation <task_id>",
            kind: Some("get_dispatch_preparation"),
            build: |args, map| {
                plain("kind", serde_json::json!("get_dispatch_preparation"), map);
                id_field(args, 0, "task_id", "task_id", map)?;
                Ok(())
            },
        },
        Command {
            name: "start-prepared-task",
            summary: "start a prepared task on this Host",
            usage: "start-prepared-task <task_id> <host_id>",
            kind: Some("start_prepared_task"),
            build: |args, map| {
                plain("kind", serde_json::json!("start_prepared_task"), map);
                id_field(args, 0, "task_id", "task_id", map)?;
                id_field(args, 1, "host_id", "host_id", map)?;
                Ok(())
            },
        },
        Command {
            name: "run-started-dispatch",
            summary: "activate a started dispatch (refuses without configured transports)",
            usage: "run-started-dispatch <task_id> <dispatch_id>",
            kind: Some("run_started_dispatch"),
            build: |args, map| {
                plain("kind", serde_json::json!("run_started_dispatch"), map);
                id_field(args, 0, "task_id", "task_id", map)?;
                id_field(args, 1, "dispatch_id", "dispatch_id", map)?;
                Ok(())
            },
        },
        Command {
            name: "request-task-completion",
            summary: "file a worker completion report as evidence",
            usage: "request-task-completion <task_id> <dispatch_id> <report>",
            kind: Some("request_task_completion"),
            build: |args, map| {
                plain("kind", serde_json::json!("request_task_completion"), map);
                id_field(args, 0, "task_id", "task_id", map)?;
                id_field(args, 1, "dispatch_id", "dispatch_id", map)?;
                plain("report", field(args, 2, "report")?.into(), map);
                Ok(())
            },
        },
        Command {
            name: "request-elevation",
            summary: "file a worker elevation ask as evidence (never grants a lease)",
            usage: "request-elevation <task_id> <dispatch_id> <permission> <reason>",
            kind: Some("request_elevation"),
            build: |args, map| {
                plain("kind", serde_json::json!("request_elevation"), map);
                id_field(args, 0, "task_id", "task_id", map)?;
                id_field(args, 1, "dispatch_id", "dispatch_id", map)?;
                plain("permission", field(args, 2, "permission")?.into(), map);
                plain("reason", field(args, 3, "reason")?.into(), map);
                Ok(())
            },
        },
        Command {
            name: "scheduling-projection",
            summary: "explain which tasks are schedulable/blocked and why",
            usage: "scheduling-projection",
            kind: Some("get_scheduling_projection"),
            build: |_, map| {
                plain("kind", serde_json::json!("get_scheduling_projection"), map);
                Ok(())
            },
        },
        Command {
            name: "get-team",
            summary: "read a Project's Team configuration",
            usage: "get-team <project_id>",
            kind: Some("get_team"),
            build: |args, map| {
                plain("kind", serde_json::json!("get_team"), map);
                plain("project_id", field(args, 0, "project_id")?.into(), map);
                Ok(())
            },
        },
        Command {
            name: "get-task-origin",
            summary: "read a Task's classified origin",
            usage: "get-task-origin <project_id> <task_id>",
            kind: Some("get_task_origin"),
            build: |args, map| {
                plain("kind", serde_json::json!("get_task_origin"), map);
                plain("project_id", field(args, 0, "project_id")?.into(), map);
                id_field(args, 1, "task_id", "task_id", map)?;
                Ok(())
            },
        },
        Command {
            name: "raw",
            summary: "send a raw operation JSON file (typed, validated by the daemon)",
            usage: "raw <operation.json>",
            // `raw` declares no kind: the authorization decision is made from
            // the operation the file actually names (see `risk_of_kind`), so
            // `raw` cannot be a way around the gate for `shutdown` and cannot
            // be over-gated for a read the typed commands do not cover.
            kind: None,
            build: |args, map| {
                let value = read_json(&field(args, 0, "operation.json")?)?;
                let object = value
                    .as_object()
                    .ok_or_else(|| Usage("operation file must be a JSON object".into()))?;
                for (key, value) in object {
                    map.insert(key.clone(), value.clone());
                }
                Ok(())
            },
        },
    ]
}

fn print_help() {
    println!("symbiote — administrative client for a running symbioted Host");
    println!();
    println!(
        "usage: symbiote [--state-dir DIR] [--command-id ID] [--json] [--yes] <command> [args...]"
    );
    println!(
        "       symbiote help   (--command-id overrides the minted id; same id + same\n                        intent replays a lost response instead of re-executing)"
    );
    println!();
    println!("options:");
    println!("  --state-dir DIR   the private directory symbioted runs with");
    println!("  --command-id ID   idempotency key for a retried command");
    println!("  --json            one machine-readable envelope per invocation on stdout");
    println!("  --yes             explicit authorization for a dangerous command");
    println!();
    println!("responses are the daemon's JSON (pretty-printed). Exit codes:");
    println!("0 success, 1 usage/connection failure, 2 daemon-refused command,");
    println!("3 authorization required (no request was sent).");
    println!();
    println!("commands marked * can send a dangerous operation — one that starts or ends");
    println!("execution, or grants capability. Those require --yes, or an interactive \"yes\"");
    println!("prompt when stdin is a terminal; otherwise they refuse with exit 3 and send");
    println!("nothing. `raw` is marked because the operation it names decides: it is gated");
    println!("exactly like the equivalent typed command, reads included.");
    println!();
    println!("commands:");
    for command in commands() {
        let gated = match command.kind {
            Some(kind) => risk_of_kind(kind).requires_authorization(),
            // `raw` has no declared kind, so it MAY send a dangerous one.
            None => true,
        };
        let marker = if gated { "*" } else { " " };
        println!(" {marker} {:<62} {}", command.usage, command.summary);
    }
}

/// The parsed command line, including the flags that decide authorization.
#[derive(Debug)]
struct Options {
    state_dir: Option<PathBuf>,
    command_id_override: Option<String>,
    json: bool,
    yes: bool,
    command: Option<String>,
    args: Vec<String>,
}

/// Flags are recognized anywhere on the line — `symbiote shutdown --yes` must
/// work as naturally as the `--yes`-first form. The first positional token is
/// the command and the rest are its arguments; `--` ends flag parsing, so an
/// argument that itself starts with `--` stays expressible. An unknown flag is
/// a usage error, never silently treated as the command name or an argument.
fn parse_options(arguments: &[String]) -> Result<Options, Usage> {
    let mut options = Options {
        state_dir: None,
        command_id_override: None,
        json: false,
        yes: false,
        command: None,
        args: Vec::new(),
    };
    let mut positional: Vec<String> = Vec::new();
    let mut only_positional = false;
    let mut index = 0;
    while index < arguments.len() {
        let token = arguments[index].clone();
        if only_positional {
            positional.push(token);
            index += 1;
            continue;
        }
        match token.as_str() {
            "--" => only_positional = true,
            "--json" => options.json = true,
            "--yes" => options.yes = true,
            "--state-dir" | "--command-id" => {
                index += 1;
                let value = arguments
                    .get(index)
                    .cloned()
                    .ok_or_else(|| Usage(format!("{token} needs a value")))?;
                if token == "--state-dir" {
                    options.state_dir = Some(PathBuf::from(value));
                } else {
                    options.command_id_override = Some(value);
                }
            }
            flag if flag.starts_with("--") => {
                return Err(Usage(format!("unknown option {flag}")));
            }
            _ => positional.push(token),
        }
        index += 1;
    }
    options.command = positional.first().cloned();
    options.args = positional.into_iter().skip(1).collect();
    Ok(options)
}

/// The versioned success envelope: the daemon's own body, unmodified.
fn success_envelope(
    command: &str,
    command_id: &str,
    result: &serde_json::Value,
) -> serde_json::Value {
    serde_json::json!({
        "schema": CLI_SCHEMA,
        "command": command,
        "command_id": command_id,
        "ok": true,
        "result": result,
    })
}

/// The versioned failure envelope. `code` is the daemon's own error code, or
/// one of the CLI's own: `authorization_required` (nothing was sent) and
/// `unreachable` (the daemon could not be reached).
fn error_envelope(command: &str, command_id: &str, code: &str, message: &str) -> serde_json::Value {
    serde_json::json!({
        "schema": CLI_SCHEMA,
        "command": command,
        "command_id": command_id,
        "ok": false,
        "error": { "code": code, "message": message },
    })
}

/// Names BOTH the typed command and the operation that makes it dangerous:
/// for `raw`, the operation is the whole story and the operator must see it.
fn confirm(command: &Command, kind: &str) -> bool {
    eprint!(
        "symbiote: {} sends the dangerous operation {kind}; type \"yes\" to continue: ",
        command.name
    );
    let _ = std::io::stderr().flush();
    let mut line = String::new();
    match std::io::stdin().read_line(&mut line) {
        Ok(0) | Err(_) => false,
        Ok(_) => confirmation_accepted(&line),
    }
}

fn run() -> Result<i32, Box<dyn std::error::Error>> {
    run_with(std::env::args().skip(1).collect())
}

fn run_with(arguments: Vec<String>) -> Result<i32, Box<dyn std::error::Error>> {
    let options = parse_options(&arguments)?;
    let Some(name) = options.command.clone() else {
        print_help();
        return Ok(EXIT_USAGE);
    };
    if name == "help" || name == "--help" {
        print_help();
        return Ok(EXIT_OK);
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
    let decision = decide(
        risk_of_kind(&kind),
        options.yes,
        std::io::stdin().is_terminal(),
    );
    // Decided BEFORE anything touches the daemon: an unauthorized invocation
    // must not even connect (pinned by the integration test's never-accepted
    // socket).
    match decision {
        Authorization::Allowed => {}
        Authorization::Prompt => {
            if !confirm(&command, &kind) {
                if options.json {
                    println!(
                        "{}",
                        error_envelope(
                            &name,
                            &minted_command_id(&options, &name),
                            "authorization_required",
                            "interactive confirmation was not given"
                        )
                    );
                } else {
                    eprintln!(
                        "symbiote: {name} ({kind}) was not confirmed; no request was sent (exit 3)"
                    );
                }
                return Ok(EXIT_AUTHORIZATION_REQUIRED);
            }
        }
        Authorization::Refused => {
            if options.json {
                println!(
                    "{}",
                    error_envelope(
                        &name,
                        &minted_command_id(&options, &name),
                        "authorization_required",
                        "dangerous operation requires --yes when stdin is not a terminal"
                    )
                );
            } else {
                eprintln!(
                    "symbiote: {name} sends the dangerous operation {kind}; noninteractive runs require --yes"
                );
                eprintln!("         (an interactive run prompts instead; no request was sent)");
            }
            return Ok(EXIT_AUTHORIZATION_REQUIRED);
        }
    }
    let Some(directory) = options.state_dir.clone() else {
        eprintln!("symbiote: missing --state-dir PRIVATE_DIRECTORY");
        return Ok(EXIT_USAGE);
    };
    let command_id = minted_command_id(&options, &name);
    let request = serde_json::json!({
        "version": symbiote_protocol::CURRENT_VERSION,
        "correlation_id": command_id,
        "command_id": command_id,
        "operation": operation,
    });
    let bytes = serde_json::to_vec(&request)?;
    let response_bytes = match symbiote_host::transport::exchange(&directory, &bytes) {
        Ok(bytes) => bytes,
        Err(error) => {
            let message = format!(
                "cannot reach the daemon at {}: {error} (is symbioted running with --state-dir {}?)",
                directory.join("host.sock").display(),
                directory.display()
            );
            if options.json {
                println!(
                    "{}",
                    error_envelope(&name, &command_id, "unreachable", &message)
                );
                return Ok(EXIT_USAGE);
            }
            eprintln!("symbiote: {message}");
            return Ok(EXIT_USAGE);
        }
    };
    let response: symbiote_protocol::Response = match serde_json::from_slice(&response_bytes) {
        Ok(response) => response,
        Err(error) => {
            let message = format!("daemon sent an unparseable response: {error}");
            if options.json {
                println!(
                    "{}",
                    error_envelope(&name, &command_id, "unreachable", &message)
                );
                return Ok(EXIT_USAGE);
            }
            eprintln!("symbiote: {message}");
            return Ok(EXIT_USAGE);
        }
    };
    match &response.result {
        Ok(body) => {
            let value = serde_json::to_value(body).unwrap_or(serde_json::Value::Null);
            if options.json {
                println!("{}", success_envelope(&name, &command_id, &value));
            } else {
                println!(
                    "{}",
                    serde_json::to_string_pretty(body).unwrap_or_else(|_| format!("{body:?}"))
                );
            }
            Ok(EXIT_OK)
        }
        Err(error) => {
            if options.json {
                let code = serde_json::to_value(error.code)
                    .ok()
                    .and_then(|value| value.as_str().map(str::to_owned))
                    .unwrap_or_else(|| "internal".into());
                println!(
                    "{}",
                    error_envelope(&name, &command_id, &code, &error.message)
                );
            } else {
                eprintln!(
                    "{}",
                    serde_json::to_string_pretty(error).unwrap_or_else(|_| format!("{error:?}"))
                );
            }
            Ok(EXIT_REFUSED)
        }
    }
}

/// Distinguishes repeated minting inside one process. Its absence is a real
/// hazard rather than a theoretical one: two invocations in the same
/// millisecond would otherwise share an idempotency key, and the daemon would
/// treat the second, different intent as a replay of the first.
static NEXT_INVOCATION: AtomicU64 = AtomicU64::new(0);

/// A caller-supplied `--command-id` restores the documented lost-response
/// retry contract: the same id with identical intent replays the durable
/// receipt instead of re-executing. The default mints an id that is unique
/// per invocation — millisecond, pid, and an in-process sequence — so
/// parallel scripts collide neither across processes nor within one.
fn minted_command_id(options: &Options, name: &str) -> String {
    match &options.command_id_override {
        Some(id) => id.clone(),
        None => format!(
            "cli-{name}-{}-{}-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or_default(),
            std::process::id(),
            NEXT_INVOCATION.fetch_add(1, Ordering::Relaxed)
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Commands whose *declared* operation is dangerous. `raw` declares no
    /// kind, because the operator's file decides what it sends.
    fn typed_dangerous_commands() -> Vec<&'static str> {
        commands()
            .iter()
            .filter(|command| {
                command
                    .kind
                    .is_some_and(|kind| risk_of_kind(kind).requires_authorization())
            })
            .map(|command| command.name)
            .collect()
    }

    #[test]
    fn authorization_follows_the_operation_not_the_command_name() {
        let mut dangerous = typed_dangerous_commands();
        dangerous.sort_unstable();
        assert_eq!(
            dangerous,
            vec!["run-started-dispatch", "shutdown", "start-prepared-task"]
        );

        // One table classifies `raw` too: a dangerous operation is gated
        // whichever route it takes, and a read through `raw` is not gated at
        // all — otherwise the escape hatch would become unusable for exactly
        // the operations the typed commands do not cover.
        for kind in ["shutdown", "run_started_dispatch", "start_prepared_task"] {
            assert!(risk_of_kind(kind).requires_authorization(), "{kind}");
        }
        for kind in ["health", "get_task", "read_journal", "register_project"] {
            assert!(!risk_of_kind(kind).requires_authorization(), "{kind}");
        }
        assert_eq!(risk_of_kind("register_project"), Risk::Mutation);
        assert_eq!(risk_of_kind("create_task"), Risk::Mutation);
        // Grants are dangerous; the matching revocations narrow authority
        // and are not.
        assert!(risk_of_kind("decide_elevation").requires_authorization());
        assert!(risk_of_kind("record_resource_consent").requires_authorization());
        assert!(!risk_of_kind("revoke_elevation").requires_authorization());
        assert!(!risk_of_kind("revoke_resource_consent").requires_authorization());
        // A kind this table does not know cannot be proven safe: fail closed.
        assert!(risk_of_kind("delete_everything").requires_authorization());
        assert!(risk_of_kind("").requires_authorization());
    }

    #[test]
    fn every_typed_command_emits_the_kind_it_declares() {
        let arguments: Vec<String> = (0..6).map(|_| "1".to_string()).collect();
        for command in commands() {
            // The two file-taking commands read an operator-supplied file.
            if matches!(command.name, "create-task" | "raw") {
                continue;
            }
            let mut operation = serde_json::Map::new();
            if let Err(error) = (command.build)(&arguments, &mut operation) {
                panic!(
                    "{} must build with synthetic arguments: {error}",
                    command.name
                );
            }
            let emitted = operation
                .get("kind")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default();
            assert_eq!(
                command.kind,
                Some(emitted),
                "{} must declare the kind it emits",
                command.name
            );
        }
        let undeclared: Vec<&str> = commands()
            .iter()
            .filter(|command| command.kind.is_none())
            .map(|command| command.name)
            .collect();
        assert_eq!(undeclared, vec!["raw"]);
    }

    #[test]
    fn read_only_and_mutating_commands_never_prompt() {
        for risk in [Risk::ReadOnly, Risk::Mutation] {
            assert_eq!(decide(risk, false, false), Authorization::Allowed);
            assert_eq!(decide(risk, false, true), Authorization::Allowed);
            assert_eq!(decide(risk, true, false), Authorization::Allowed);
        }
    }

    #[test]
    fn dangerous_commands_refuse_without_authorization_or_a_terminal() {
        // No flag and no terminal: refuse — never a silent default.
        assert_eq!(
            decide(Risk::Dangerous, false, false),
            Authorization::Refused
        );
        // An interactive terminal prompts instead of refusing.
        assert_eq!(decide(Risk::Dangerous, false, true), Authorization::Prompt);
        // An explicit authorization is honored in both modes.
        assert_eq!(decide(Risk::Dangerous, true, false), Authorization::Allowed);
        assert_eq!(decide(Risk::Dangerous, true, true), Authorization::Allowed);
    }

    #[test]
    fn only_an_explicit_yes_confirms() {
        assert!(confirmation_accepted("yes"));
        assert!(confirmation_accepted("YES"));
        assert!(confirmation_accepted("  Yes\n"));
        // Everything else — including a closed stdin's empty read — refuses.
        for line in ["", "\n", "y", "no", "yes please", "shutdown"] {
            assert!(!confirmation_accepted(line), "{line:?} must not confirm");
        }
    }

    fn options(arguments: &[&str]) -> Options {
        parse_options(&arguments.iter().map(|s| s.to_string()).collect::<Vec<_>>()).unwrap()
    }

    #[test]
    fn flags_parse_before_and_after_the_command() {
        let before = options(&["--json", "--yes", "--state-dir", "/s", "shutdown"]);
        assert!(before.json && before.yes);
        assert_eq!(before.state_dir, Some(PathBuf::from("/s")));
        assert_eq!(before.command.as_deref(), Some("shutdown"));
        assert!(before.args.is_empty());

        // The natural form — flags after the command — must behave the same,
        // or `symbiote shutdown --yes` would silently refuse.
        let after = options(&["shutdown", "--yes", "--json"]);
        assert!(after.json && after.yes);
        assert_eq!(after.command.as_deref(), Some("shutdown"));
        assert!(after.args.is_empty());

        let mixed = options(&[
            "--state-dir",
            "/s",
            "--command-id",
            "c1",
            "get-task",
            "p",
            "t",
        ]);
        assert_eq!(mixed.command_id_override.as_deref(), Some("c1"));
        assert_eq!(mixed.args, vec!["p".to_string(), "t".to_string()]);
    }

    #[test]
    fn an_unknown_or_incomplete_flag_is_a_usage_error_not_a_command() {
        // An unknown flag is refused wherever it appears, and a flag that
        // needs a value is never satisfied by the command name.
        for (arguments, named) in [
            (vec!["--bare".to_string()], "--bare"),
            (vec!["--state-dir".to_string()], "--state-dir"),
            (
                vec!["raw".to_string(), "--yes.json".to_string()],
                "--yes.json",
            ),
        ] {
            let error = parse_options(&arguments).unwrap_err();
            assert!(error.to_string().contains(named), "{error}");
        }
        // `--` keeps an argument that starts with dashes expressible.
        let escaped = options(&["raw", "--", "--yes.json"]);
        assert_eq!(escaped.command.as_deref(), Some("raw"));
        assert_eq!(escaped.args, vec!["--yes.json".to_string()]);
        // ... and the value of a value-taking flag is not re-read as one.
        let literal = options(&["--command-id", "--yes", "health"]);
        assert_eq!(literal.command_id_override.as_deref(), Some("--yes"));
        assert_eq!(literal.command.as_deref(), Some("health"));
        assert!(!literal.yes);
    }

    #[test]
    fn envelopes_carry_the_schema_command_and_id() {
        let result = serde_json::json!({"kind": "shutdown"});
        let success = success_envelope("shutdown", "cli-1", &result);
        assert_eq!(success["schema"], CLI_SCHEMA);
        assert_eq!(success["command"], "shutdown");
        assert_eq!(success["command_id"], "cli-1");
        assert_eq!(success["ok"], true);
        assert_eq!(success["result"], result);

        let failure = error_envelope("shutdown", "cli-1", "authorization_required", "no");
        assert_eq!(failure["schema"], CLI_SCHEMA);
        assert_eq!(failure["ok"], false);
        assert_eq!(failure["error"]["code"], "authorization_required");
        assert!(failure.get("result").is_none());
    }

    #[test]
    fn a_minted_command_id_is_per_invocation_and_override_is_honored() {
        let mut options = parse_options(&["health".to_string()]).unwrap();
        let first = minted_command_id(&options, "health");
        let second = minted_command_id(&options, "health");
        assert!(first.starts_with("cli-health-"));
        assert_ne!(first, second);
        options.command_id_override = Some("retry-1".into());
        assert_eq!(minted_command_id(&options, "health"), "retry-1");
    }
}
