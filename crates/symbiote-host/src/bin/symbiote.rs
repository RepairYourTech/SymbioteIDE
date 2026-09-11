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
use std::collections::BTreeSet;
use std::io::{IsTerminal, Read, Write};
use std::path::{Path, PathBuf};
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

/// The authorization policy file's schema identity, versioned independently
/// of the envelope so either can evolve without silently reinterpreting the
/// other.
pub const POLICY_SCHEMA: &str = "symbiote.cli-policy/v1";

/// The environment variable consulted when `--policy` is absent. An empty
/// value means "no policy", like an unset one.
pub const POLICY_ENV: &str = "SYMBIOTE_CLI_POLICY";

/// A policy grants authority, so it is held to the same bound the Host holds
/// its own operator config to rather than being read whole.
const POLICY_LIMIT: u64 = 64 * 1024;

/// The kind the help text's policy example names. A test pins that it is still
/// a dangerous kind, so the example can never drift into one a policy would
/// have to reject.
const POLICY_EXAMPLE_KIND: &str = "shutdown";

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

/// The ONE table of operation kinds this CLI knows and what sending each can
/// do. Lookups, the dangerous list the help prints, and policy validation all
/// read this array, so they cannot drift apart. A kind absent from the table
/// cannot be proven safe and is treated as dangerous.
const OPERATION_RISKS: &[(&str, Risk)] = &[
    // Reads: no daemon state changes.
    ("hello", Risk::ReadOnly),
    ("health", Risk::ReadOnly),
    ("get_host_pulse", Risk::ReadOnly),
    ("get_project", Risk::ReadOnly),
    ("get_task", Risk::ReadOnly),
    ("read_journal", Risk::ReadOnly),
    ("get_dispatch_preparation", Risk::ReadOnly),
    ("get_scheduling_projection", Risk::ReadOnly),
    ("get_team", Risk::ReadOnly),
    ("get_task_origin", Risk::ReadOnly),
    ("get_binding", Risk::ReadOnly),
    ("get_binding_readiness", Risk::ReadOnly),
    ("get_task_dependencies", Risk::ReadOnly),
    ("get_route", Risk::ReadOnly),
    ("resolve_route", Risk::ReadOnly),
    ("get_work", Risk::ReadOnly),
    ("get_provider_connection", Risk::ReadOnly),
    ("get_billing_entitlement", Risk::ReadOnly),
    ("get_model_descriptor", Risk::ReadOnly),
    ("get_resource_consent", Risk::ReadOnly),
    // Mutations: they change state, but they neither start or end execution
    // nor widen authority.
    ("create_task", Risk::Mutation),
    ("prepare_dispatch", Risk::Mutation),
    ("request_task_completion", Risk::Mutation),
    ("request_elevation", Risk::Mutation),
    ("replace_binding", Risk::Mutation),
    ("record_route", Risk::Mutation),
    ("set_task_dependencies", Risk::Mutation),
    ("acquire_task_lease", Risk::Mutation),
    ("release_task_lease", Risk::Mutation),
    ("expire_stale_leases", Risk::Mutation),
    ("replace_provider_connection", Risk::Mutation),
    ("replace_billing_entitlement", Risk::Mutation),
    ("replace_model_descriptor", Risk::Mutation),
    ("replace_team", Risk::Mutation),
    ("create_work", Risk::Mutation),
    ("change_work", Risk::Mutation),
    ("assign_task_origin", Risk::Mutation),
    ("register_project", Risk::Mutation),
    ("observe_root_placement", Risk::Mutation),
    ("revoke_elevation", Risk::Mutation),
    ("revoke_resource_consent", Risk::Mutation),
    // Starting or ending execution, or granting capability.
    ("start_prepared_task", Risk::Dangerous),
    ("run_started_dispatch", Risk::Dangerous),
    ("shutdown", Risk::Dangerous),
    ("decide_elevation", Risk::Dangerous),
    ("record_resource_consent", Risk::Dangerous),
];

/// `None` for a kind this build does not know: the caller must fail closed
/// rather than assume safety.
fn known_risk_of_kind(kind: &str) -> Option<Risk> {
    OPERATION_RISKS
        .iter()
        .find(|(known, _)| *known == kind)
        .map(|(_, risk)| *risk)
}

/// An unknown kind cannot be proven safe, so it is Dangerous rather than
/// silently allowed — and, for the same reason, it is not *nameable* by a
/// policy (`known_risk_of_kind` returns `None` for it).
fn risk_of_kind(kind: &str) -> Risk {
    known_risk_of_kind(kind).unwrap_or(Risk::Dangerous)
}

/// How a request came to be authorized. Kept as a value so the decision is
/// pure and every branch is unit tested without a daemon or a terminal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Authorization {
    /// The operation is not dangerous: nothing needed authorizing.
    NotRequired,
    /// `--yes` authorized this invocation.
    Flag,
    /// A configured policy pre-authorizes this operation kind.
    Policy,
    /// Ask on the terminal; only an explicit `yes` proceeds.
    Prompt,
    /// Refuse before connecting.
    Refused,
}

/// `--yes` outranks a policy (it is the narrower, per-invocation grant) and
/// both outrank the prompt. A policy is only ever consulted for an operation
/// that is already dangerous, so it can never make a read or a mutation
/// "allowed" — those never needed authorization to begin with.
fn authorize(risk: Risk, flag: bool, policy_grant: bool, interactive: bool) -> Authorization {
    if !risk.requires_authorization() {
        return Authorization::NotRequired;
    }
    if flag {
        return Authorization::Flag;
    }
    if policy_grant {
        return Authorization::Policy;
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

/// The parsed authorization policy: the operation kinds an operator has
/// pre-authorized for noninteractive runs, and the point after which that
/// pre-authorization lapses.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Policy {
    authorize: BTreeSet<String>,
    expires_at: Option<u64>,
}

impl Policy {
    /// A policy grants exactly the dangerous kinds it lists, and only until
    /// it expires. An expired policy is not an error: it simply grants
    /// nothing, so the ordinary refusal path reports it.
    fn authorizes(&self, kind: &str, now_ms: u64) -> bool {
        if self.expires_at.is_some_and(|expiry| now_ms > expiry) {
            return false;
        }
        self.authorize.contains(kind)
    }

    fn expired_at(&self, now_ms: u64) -> Option<u64> {
        self.expires_at.filter(|expiry| now_ms > *expiry)
    }
}

/// The policy file as written by an operator. `deny_unknown_fields` matters:
/// a misspelled key (`authorise`, `expires`) would otherwise be ignored and
/// the operator would believe they had configured something they had not.
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct PolicyFile {
    schema: String,
    authorize: Vec<String>,
    #[serde(default)]
    expires_at: Option<u64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PolicyError {
    /// Missing, unreadable, a directory, or larger than the bound.
    Unreadable,
    /// Not a regular file, or readable/writable by group or other.
    Insecure,
    /// Not the policy schema, or an unknown/empty/non-dangerous `authorize`.
    Invalid,
}

impl std::fmt::Display for PolicyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            PolicyError::Unreadable => "cannot read the policy as a regular file",
            PolicyError::Insecure => {
                "the policy must be a regular file, owned by this user, with no group/other \
                 permission bits (chmod 600)"
            }
            PolicyError::Invalid => "the policy is not a valid symbiote.cli-policy/v1 document",
        })
    }
}

impl std::error::Error for PolicyError {}

/// Loads and validates a policy with the discipline the Host applies to its
/// own private files: a symlink, a shared file, a foreign owner, an oversized
/// file or a schema that does not name only *known dangerous* kinds is
/// refused rather than interpreted. A policy that cannot be honored is never
/// silently downgraded to "no policy".
fn load_policy(path: &Path) -> Result<Policy, PolicyError> {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    // `symlink_metadata` so a symlink is refused rather than followed: the
    // file that grants authority must be the file the operator inspected.
    let metadata = std::fs::symlink_metadata(path).map_err(|_| PolicyError::Unreadable)?;
    if !metadata.file_type().is_file()
        || metadata.uid() != nix::unistd::geteuid().as_raw()
        || metadata.permissions().mode() & 0o077 != 0
    {
        return Err(PolicyError::Insecure);
    }
    let file = std::fs::File::open(path).map_err(|_| PolicyError::Unreadable)?;
    let mut bytes = Vec::new();
    (&file)
        .take(POLICY_LIMIT + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| PolicyError::Unreadable)?;
    if bytes.len() as u64 > POLICY_LIMIT {
        return Err(PolicyError::Unreadable);
    }
    let parsed: PolicyFile = serde_json::from_slice(&bytes).map_err(|_| PolicyError::Invalid)?;
    if parsed.schema != POLICY_SCHEMA {
        return Err(PolicyError::Invalid);
    }
    if parsed.authorize.is_empty() {
        return Err(PolicyError::Invalid);
    }
    let mut authorize = BTreeSet::new();
    for kind in parsed.authorize {
        // Only a kind that is *known and dangerous* can be named. A read or a
        // mutation is already ungated, so listing it would be a no-op the
        // operator could mistake for coverage; an unknown kind is one the
        // daemon may not even define, and this CLI refuses to pre-authorize
        // what it cannot classify. A wildcard is deliberately not a syntax.
        if known_risk_of_kind(&kind) != Some(Risk::Dangerous) {
            return Err(PolicyError::Invalid);
        }
        authorize.insert(kind);
    }
    Ok(Policy {
        authorize,
        expires_at: parsed.expires_at,
    })
}

/// The policy path for this invocation: an explicit `--policy` wins, else
/// `SYMBIOTE_CLI_POLICY`. An empty variable means "no policy", like unset.
fn resolved_policy_path(options: &Options) -> Option<PathBuf> {
    options.policy.clone().or_else(|| {
        std::env::var_os(POLICY_ENV)
            .map(PathBuf::from)
            .filter(|path| !path.as_os_str().is_empty())
    })
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
        "usage: symbiote [--state-dir DIR] [--command-id ID] [--policy FILE] [--json] [--yes] <command> [args...]"
    );
    println!(
        "       symbiote help   (--command-id overrides the minted id; same id + same\n                        intent replays a lost response instead of re-executing)"
    );
    println!();
    println!("options:");
    println!("  --state-dir DIR   the private directory symbioted runs with");
    println!("  --command-id ID   idempotency key for a retried command");
    println!("  --policy FILE     pre-authorize dangerous operation kinds for noninteractive runs");
    println!("  --json            one machine-readable envelope per invocation on stdout");
    println!("  --yes             explicit authorization for this one dangerous command");
    println!();
    println!("responses are the daemon's JSON (pretty-printed). Exit codes:");
    println!("0 success, 1 usage/connection failure, 2 daemon-refused command,");
    println!("3 authorization required (no request was sent).");
    println!();
    println!("commands marked * can send a dangerous operation — one that starts or ends");
    println!("execution, or grants capability:");
    let dangerous: Vec<&str> = OPERATION_RISKS
        .iter()
        .filter(|(_, risk)| risk.requires_authorization())
        .map(|(kind, _)| *kind)
        .collect();
    println!("  {}", dangerous.join(", "));
    println!("Those are authorized by --yes, or by an interactive \"yes\" prompt when stdin is");
    println!("a terminal, or by a policy file that names the operation kind. Otherwise they");
    println!("refuse with exit 3 and send nothing. `raw` is marked because the operation it");
    println!("names decides: it is authorized exactly like the equivalent typed command, reads");
    println!("included.");
    println!();
    println!("a policy ({POLICY_SCHEMA}) names the kinds it pre-authorizes:");
    println!(
        r#"  {{"schema":"{POLICY_SCHEMA}","authorize":["{POLICY_EXAMPLE_KIND}"],"expires_at":4102444800000}}"#
    );
    println!("`--policy FILE`, else ${POLICY_ENV}, selects it. It must be a regular file,");
    println!("owned by you, with no group/other permission bits (chmod 600). It grants only");
    println!("the kinds it names and only until expires_at (optional, unix milliseconds); an");
    println!("unreadable, insecure or invalid policy is a usage failure that sends nothing.");
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
    policy: Option<PathBuf>,
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
        policy: None,
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
            "--state-dir" | "--command-id" | "--policy" => {
                index += 1;
                let value = arguments
                    .get(index)
                    .cloned()
                    .ok_or_else(|| Usage(format!("{token} needs a value")))?;
                match token.as_str() {
                    "--state-dir" => options.state_dir = Some(PathBuf::from(value)),
                    "--command-id" => options.command_id_override = Some(value),
                    _ => options.policy = Some(PathBuf::from(value)),
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
    let risk = risk_of_kind(&kind);
    let now_ms = current_time_ms();
    // A policy is consulted ONLY for an operation that is already dangerous
    // and that `--yes` did not already authorize. Two consequences are
    // deliberate: a read never touches the policy file, so a broken policy
    // cannot brick `health`; and `--yes` is a complete authorization on its
    // own, so a policy cannot block an explicitly authorized run.
    let mut policy_error: Option<(PathBuf, PolicyError)> = None;
    let mut policy_expired: Option<(PathBuf, u64)> = None;
    let mut policy_grant = false;
    if risk.requires_authorization() && !options.yes {
        if let Some(path) = resolved_policy_path(&options) {
            match load_policy(&path) {
                Ok(policy) => {
                    policy_grant = policy.authorizes(&kind, now_ms);
                    // Report a lapse only when the policy actually named this
                    // kind. An expired policy that never mentioned `kind` must
                    // not be described as a lapsed grant *for* it, or the
                    // refusal asserts something about the file that is not
                    // true.
                    if !policy_grant && policy.authorize.contains(&kind) {
                        policy_expired = policy.expired_at(now_ms).map(|expiry| (path, expiry));
                    }
                }
                Err(error) => policy_error = Some((path, error)),
            }
        }
    }
    // A policy that cannot be honored is an operator error, not a missing
    // authorization: it is reported as such and nothing is sent. Silently
    // treating it as "no policy" would let a typo quietly downgrade a
    // pre-authorization the operator believes is in force.
    if let Some((path, error)) = policy_error {
        let message = format!("cannot use the policy {}: {error}", path.display());
        if options.json {
            println!(
                "{}",
                error_envelope(
                    &name,
                    &minted_command_id(&options, &name),
                    "policy_invalid",
                    &message
                )
            );
            return Ok(EXIT_USAGE);
        }
        eprintln!("symbiote: {message}");
        eprintln!("         no request was sent (exit 1)");
        return Ok(EXIT_USAGE);
    }
    let decision = authorize(
        risk,
        options.yes,
        policy_grant,
        std::io::stdin().is_terminal(),
    );
    // Decided BEFORE anything touches the daemon: an unauthorized invocation
    // must not even connect (pinned by the integration test's never-accepted
    // socket).
    match decision {
        Authorization::NotRequired | Authorization::Flag | Authorization::Policy => {}
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
            // The refusal names every remedy, and names the policy explicitly
            // when one was configured but had lapsed — otherwise an operator
            // whose policy expired sees only "--yes" and never learns why.
            let message = match &policy_expired {
                Some((path, expiry)) => format!(
                    "the policy {} pre-authorized {kind} but expired at {expiry}",
                    path.display()
                ),
                None => format!("noninteractive runs require --yes or a policy naming \"{kind}\""),
            };
            if options.json {
                println!(
                    "{}",
                    error_envelope(
                        &name,
                        &minted_command_id(&options, &name),
                        "authorization_required",
                        &message
                    )
                );
            } else {
                eprintln!("symbiote: {name} sends the dangerous operation {kind}; {message}");
                if policy_expired.is_none() {
                    eprintln!("         (an interactive run prompts instead; no request was sent)");
                }
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

/// Unix milliseconds, the unit a policy's `expires_at` is written in. A clock
/// before the epoch reads as 0, which only makes an expiry more likely to have
/// lapsed — the fail-closed direction.
fn current_time_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis() as u64)
        .unwrap_or_default()
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
        assert_eq!(risk_of_kind("get_host_pulse"), Risk::ReadOnly);
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
            assert_eq!(
                authorize(risk, false, false, false),
                Authorization::NotRequired
            );
            assert_eq!(
                authorize(risk, false, false, true),
                Authorization::NotRequired
            );
            assert_eq!(
                authorize(risk, true, false, false),
                Authorization::NotRequired
            );
        }
    }

    #[test]
    fn dangerous_commands_refuse_without_authorization_or_a_terminal() {
        // No flag, no policy, no terminal: refuse — never a silent default.
        assert_eq!(
            authorize(Risk::Dangerous, false, false, false),
            Authorization::Refused
        );
        // An interactive terminal prompts instead of refusing.
        assert_eq!(
            authorize(Risk::Dangerous, false, false, true),
            Authorization::Prompt
        );
        // `--yes` is honored in both modes and is reported as the flag's grant.
        assert_eq!(
            authorize(Risk::Dangerous, true, false, false),
            Authorization::Flag
        );
        assert_eq!(
            authorize(Risk::Dangerous, true, false, true),
            Authorization::Flag
        );
    }

    #[test]
    fn a_policy_grant_is_authorization_without_a_flag_or_a_terminal() {
        assert_eq!(
            authorize(Risk::Dangerous, false, true, false),
            Authorization::Policy
        );
        assert_eq!(
            authorize(Risk::Dangerous, false, true, true),
            Authorization::Policy
        );
        // The per-invocation flag is the narrower grant, so it is the one
        // reported when both apply.
        assert_eq!(
            authorize(Risk::Dangerous, true, true, false),
            Authorization::Flag
        );
    }

    /// The table is the single source of truth, so these pin the properties
    /// the rest of the CLI (and the help text) read out of it.
    #[test]
    fn the_operation_table_is_consistent_and_fails_closed() {
        let mut kinds: Vec<&str> = OPERATION_RISKS.iter().map(|(kind, _)| *kind).collect();
        let mut sorted = kinds.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(kinds.len(), sorted.len(), "the table has duplicate kinds");
        kinds.sort_unstable();
        // Every kind is found by lookup, and the declared entry is the risk.
        for (kind, risk) in OPERATION_RISKS {
            assert_eq!(known_risk_of_kind(kind), Some(*risk), "{kind}");
            assert_eq!(risk_of_kind(kind), *risk, "{kind}");
        }
        // A kind this build does not know is Dangerous (fail closed) but is
        // NOT nameable by a policy, because only `Some(Dangerous)` is.
        assert_eq!(known_risk_of_kind("delete_everything"), None);
        assert_eq!(risk_of_kind("delete_everything"), Risk::Dangerous);
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

    const SHUTDOWN_ONLY: &str = r#"{"schema":"symbiote.cli-policy/v1","authorize":["shutdown"]}"#;

    static NEXT_POLICY_FILE: AtomicU64 = AtomicU64::new(0);

    /// Writes a policy file with an explicit mode and returns its path; the
    /// test removes it. Uniqueness is per (process, call) so parallel tests
    /// cannot see each other's file.
    fn write_policy(contents: &str, mode: u32) -> PathBuf {
        use std::os::unix::fs::PermissionsExt;
        let path = std::env::temp_dir().join(format!(
            "symbiote-cli-policy-unit-{}-{}",
            std::process::id(),
            NEXT_POLICY_FILE.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::write(&path, contents).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(mode)).unwrap();
        path
    }

    fn remove(path: &Path) {
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn a_private_policy_authorizes_exactly_the_dangerous_kinds_it_names() {
        let path = write_policy(SHUTDOWN_ONLY, 0o600);
        let policy = load_policy(&path).expect("a 0600 policy is honored");
        assert_eq!(policy.expires_at, None);
        assert!(policy.authorizes("shutdown", 0));
        // Not a wildcard: a kind the policy does not name stays unauthorized,
        // and a kind that was never gated stays ungated rather than "granted".
        for kind in ["start_prepared_task", "decide_elevation", "health"] {
            assert!(!policy.authorizes(kind, 0), "{kind}");
        }
        remove(&path);
    }

    #[test]
    fn a_policy_lapses_after_its_expiry_rather_than_erroring() {
        let path = write_policy(
            r#"{"schema":"symbiote.cli-policy/v1","authorize":["shutdown"],"expires_at":1000}"#,
            0o600,
        );
        let policy = load_policy(&path).unwrap();
        // The expiry instant itself is still authorized; the next millisecond
        // is not. An expired policy grants nothing but is not a load failure —
        // the ordinary refusal path reports it.
        assert!(policy.authorizes("shutdown", 1000));
        assert!(!policy.authorizes("shutdown", 1001));
        assert_eq!(policy.expired_at(1000), None);
        assert_eq!(policy.expired_at(1001), Some(1000));
        remove(&path);
    }

    #[test]
    fn every_dangerous_kind_in_the_table_can_be_named_by_a_policy() {
        let dangerous: Vec<&str> = OPERATION_RISKS
            .iter()
            .filter(|(_, risk)| risk.requires_authorization())
            .map(|(kind, _)| *kind)
            .collect();
        // Pins the set the help text and the policy schema describe.
        assert_eq!(
            dangerous,
            vec![
                "start_prepared_task",
                "run_started_dispatch",
                "shutdown",
                "decide_elevation",
                "record_resource_consent",
            ]
        );
        let contents = format!(
            r#"{{"schema":"symbiote.cli-policy/v1","authorize":[{}]}}"#,
            dangerous
                .iter()
                .map(|kind| format!("\"{kind}\""))
                .collect::<Vec<_>>()
                .join(",")
        );
        let path = write_policy(&contents, 0o600);
        // The help text's example must stay a kind a policy accepts.
        assert!(dangerous.contains(&POLICY_EXAMPLE_KIND));
        let policy = load_policy(&path).expect("every dangerous kind is nameable");
        for kind in dangerous {
            assert!(policy.authorizes(kind, 0), "{kind}");
        }
        remove(&path);
    }

    #[test]
    fn a_policy_that_cannot_be_honored_is_refused_not_ignored() {
        // A policy grants authority, so it is read strictly: an unknown schema
        // version, a missing or empty list, a wildcard, a typo'd or unknown
        // key, and — deliberately — a kind that is known but NOT dangerous
        // (listing it would be a no-op the operator could mistake for
        // coverage), plus a kind this build cannot classify at all.
        for contents in [
            r#"{"schema":"symbiote.cli-policy/v2","authorize":["shutdown"]}"#,
            r#"{"authorize":["shutdown"]}"#,
            r#"{"schema":"symbiote.cli-policy/v1"}"#,
            r#"{"schema":"symbiote.cli-policy/v1","authorize":[]}"#,
            r#"{"schema":"symbiote.cli-policy/v1","authorize":["*"]}"#,
            r#"{"schema":"symbiote.cli-policy/v1","authorize":["shutdown","health"]}"#,
            r#"{"schema":"symbiote.cli-policy/v1","authorize":["create_task"]}"#,
            r#"{"schema":"symbiote.cli-policy/v1","authorize":["delete_everything"]}"#,
            r#"{"schema":"symbiote.cli-policy/v1","authorise":["shutdown"]}"#,
            r#"{"schema":"symbiote.cli-policy/v1","authorize":["shutdown"],"expires":"soon"}"#,
            "not json",
        ] {
            let path = write_policy(contents, 0o600);
            assert_eq!(load_policy(&path), Err(PolicyError::Invalid), "{contents}");
            remove(&path);
        }
        // A policy that cannot be read at all is Unreadable, not Invalid.
        assert_eq!(
            load_policy(Path::new("/nonexistent/symbiote-policy.json")),
            Err(PolicyError::Unreadable)
        );
    }

    #[test]
    fn a_shared_symlinked_oversized_or_non_file_policy_is_refused() {
        use std::os::unix::fs::PermissionsExt;
        // Any group/other bit is authority a neighbor could rewrite.
        for mode in [0o640, 0o604, 0o606, 0o660, 0o666, 0o777] {
            let path = write_policy(SHUTDOWN_ONLY, mode);
            assert_eq!(load_policy(&path), Err(PolicyError::Insecure), "{mode:o}");
            remove(&path);
        }
        // Owner-only 0400 and 0600 are both honored: the bound is *sharing*,
        // not writability.
        for mode in [0o400, 0o600] {
            let path = write_policy(SHUTDOWN_ONLY, mode);
            assert!(load_policy(&path).is_ok(), "{mode:o}");
            remove(&path);
        }
        // A symlink is refused rather than followed: the file that grants
        // authority must be the file the operator inspected.
        let target = write_policy(SHUTDOWN_ONLY, 0o600);
        let link = target.with_extension("link");
        std::os::unix::fs::symlink(&target, &link).unwrap();
        assert_eq!(load_policy(&link), Err(PolicyError::Insecure));
        std::fs::remove_file(&link).unwrap();
        remove(&target);
        // A directory is not a policy.
        let directory = std::env::temp_dir().join(format!(
            "symbiote-cli-policy-dir-{}-{}",
            std::process::id(),
            NEXT_POLICY_FILE.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o700)).unwrap();
        assert_eq!(load_policy(&directory), Err(PolicyError::Insecure));
        std::fs::remove_dir_all(&directory).unwrap();
        // Bounded: a policy past 64 KiB is refused, and the bound is checked
        // before the document is interpreted (the padding key would otherwise
        // be an unknown field).
        let oversized = format!(
            r#"{{"schema":"symbiote.cli-policy/v1","authorize":["shutdown"],"padding":"{}"}}"#,
            "x".repeat(64 * 1024)
        );
        let path = write_policy(&oversized, 0o600);
        assert_eq!(load_policy(&path), Err(PolicyError::Unreadable));
        remove(&path);
    }

    #[test]
    fn the_policy_flag_selects_the_file_before_or_after_the_command() {
        for arguments in [
            vec!["--policy", "/p.json", "shutdown"],
            vec!["shutdown", "--policy", "/p.json"],
        ] {
            let parsed = options(&arguments);
            assert_eq!(parsed.policy, Some(PathBuf::from("/p.json")));
            assert_eq!(parsed.command.as_deref(), Some("shutdown"));
            assert_eq!(
                resolved_policy_path(&parsed),
                Some(PathBuf::from("/p.json"))
            );
        }
        assert_eq!(options(&["health"]).policy, None);
    }
}
