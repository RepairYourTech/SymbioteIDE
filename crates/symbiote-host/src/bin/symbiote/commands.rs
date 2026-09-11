//! The command table: every command the CLI knows, the JSON operation it
//! builds, and the versioned-usage help text.
//!
//! This is the single owner of what `symbiote` can send. Authorization follows
//! the operation a command *emits*, not its name (`raw` declares no kind
//! because the operator's file decides), so the `kind` each builder writes is
//! the same table entry [`crate::gate`] classifies.
use crate::args::{Args, Usage};
use symbiote_host::cli_authorization::POLICY_ENV;
use symbiote_host::cli_schema::{POLICY_SCHEMA, dangerous_kinds, risk_of_kind};

/// The kind the help text's policy example names. A test pins that it is still
/// a dangerous kind, so the example can never drift into one a policy would
/// have to reject.
pub(crate) const POLICY_EXAMPLE_KIND: &str = "shutdown";

/// One command: the operation JSON builder plus the kind it emits. `raw`
/// declares none, because the operator's file decides what it sends.
pub(crate) struct Command {
    pub(crate) name: &'static str,
    pub(crate) summary: &'static str,
    pub(crate) usage: &'static str,
    pub(crate) kind: Option<&'static str>,
    pub(crate) build:
        fn(Args, &mut serde_json::Map<String, serde_json::Value>) -> Result<(), Usage>,
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

pub(crate) fn commands() -> Vec<Command> {
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

pub(crate) fn print_help() {
    println!("symbiote — administrative client for a running symbioted Host");
    println!();
    println!(
        "usage: symbiote [--state-dir DIR] [--command-id ID] [--policy FILE] [--json] [--yes] <command> [args...]"
    );
    println!(
        "       symbiote schema [envelope|policy] [--write DIR | --check DIR]\n       symbiote help   (--command-id overrides the minted id; same id + same\n                        intent replays a lost response instead of re-executing)"
    );
    println!();
    println!("options:");
    println!("  --state-dir DIR   the private directory symbioted runs with");
    println!("  --command-id ID   idempotency key for a retried command");
    println!("  --policy FILE     pre-authorize dangerous operation kinds for noninteractive runs");
    println!("  --write DIR       with `schema`, regenerate the selection into DIR");
    println!("  --check DIR       with `schema`, compare DIR against the published documents");
    println!("  --help, -h        print this table; connects to nothing, honors no other flag");
    println!("  --json            one machine-readable envelope per invocation on stdout");
    println!("  --yes             explicit authorization for this one dangerous command");
    println!();
    println!("flags are per command: daemon commands honor --state-dir, --command-id,");
    println!("--policy, --json and --yes; `schema` honors --write and --check; `help` honors");
    println!("nothing. `--help`/`-h` is universal, and a help request honors no other flag:");
    println!("`--json --help health` is refused exactly as `--json help` is. A flag a command");
    println!("cannot honor is a usage error that names it, never silently dropped.");
    println!();
    println!("responses are the daemon's JSON (pretty-printed). Exit codes:");
    println!("0 success, 1 usage/connection failure, 2 daemon-refused command,");
    println!("3 authorization required (no request was sent).");
    println!();
    println!("commands marked * can send a dangerous operation — one that starts or ends");
    println!("execution, or grants capability:");
    let dangerous = dangerous_kinds();
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
    let schema_name = "schema";
    let schema_summary = "print the published symbiote.cli and cli-policy JSON Schemas";
    println!("   {schema_name:<62} {schema_summary}");
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
