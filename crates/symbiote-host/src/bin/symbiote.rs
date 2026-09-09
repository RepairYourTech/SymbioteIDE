//! `symbiote` administrative CLI: a scriptable, human-readable client for a
//! running Host daemon, using the same authenticated Unix-socket protocol as
//! every other controller. Administrative scope only — the interactive and
//! headless coding-agent experience (#467) is a separate surface that shares
//! this client plumbing; no model loop exists here.
//!
//! Commands map one-to-one onto typed protocol operations. Every response is
//! the daemon's own JSON (machine-readable, stable field names). Exit codes:
//! 0 success, 1 connection/usage failure, 2 the daemon refused the command
//! (`Err` response) — distinct exits so scripts can branch on refusal vs
//! transport failure.
use std::path::PathBuf;
use std::process::exit;

fn main() {
    if let Err(error) = run() {
        eprintln!("symbiote: {error}");
        exit(1);
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

/// One command: the operation JSON builder plus its usage line.
struct Command {
    name: &'static str,
    summary: &'static str,
    usage: &'static str,
    build: fn(Args, &mut serde_json::Map<String, serde_json::Value>) -> Result<(), Usage>,
}

fn field(args: Args, index: usize, name: &str) -> Result<String, Usage> {
    args.get(index)
        .cloned()
        .ok_or_else(|| Usage(format!("missing <{name}>")))
}

/// A canonical identity argument arrives as `{"id": value}` on the wire for
/// the typed-id fields; plain strings for untyped project ids.
fn id_field(
    args: Args,
    index: usize,
    key: &str,
    name: &str,
    map: &mut serde_json::Map<String, serde_json::Value>,
) -> Result<(), Usage> {
    let value = field(args, index, name)?;
    map.insert(key.to_owned(), serde_json::json!({ "id": value }));
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
            build: |_, map| {
                plain("kind", serde_json::json!("hello"), map);
                Ok(())
            },
        },
        Command {
            name: "health",
            summary: "check the daemon is serving",
            usage: "health",
            build: |_, map| {
                plain("kind", serde_json::json!("health"), map);
                Ok(())
            },
        },
        Command {
            name: "host-pulse",
            summary: "owner-only passive Host resource pulse",
            usage: "host-pulse",
            build: |_, map| {
                plain("kind", serde_json::json!("get_host_pulse"), map);
                Ok(())
            },
        },
        Command {
            name: "shutdown",
            summary: "drain and stop the daemon",
            usage: "shutdown",
            build: |_, map| {
                plain("kind", serde_json::json!("shutdown"), map);
                Ok(())
            },
        },
        Command {
            name: "get-project",
            summary: "read a registered Project",
            usage: "get-project <project_id>",
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
            build: |args, map| {
                plain("task", read_json(&field(args, 0, "task.json")?)?, map);
                Ok(())
            },
        },
        Command {
            name: "get-task",
            summary: "read a Task's durable state",
            usage: "get-task <project_id> <task_id>",
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
            build: |args, map| {
                plain("kind", serde_json::json!("request_task_completion"), map);
                id_field(args, 0, "task_id", "task_id", map)?;
                id_field(args, 1, "dispatch_id", "dispatch_id", map)?;
                plain("report", field(args, 2, "report")?.into(), map);
                Ok(())
            },
        },
        Command {
            name: "scheduling-projection",
            summary: "explain which tasks are schedulable/blocked and why",
            usage: "scheduling-projection",
            build: |_, map| {
                plain("kind", serde_json::json!("get_scheduling_projection"), map);
                Ok(())
            },
        },
        Command {
            name: "get-team",
            summary: "read a Project's Team configuration",
            usage: "get-team <project_id>",
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
    println!("usage: symbiote --state-dir DIR <command> [args...]");
    println!("       symbiote help");
    println!();
    println!("responses are the daemon's JSON (pretty-printed). Exit codes:");
    println!("0 success, 1 usage/connection failure, 2 daemon-refused command.");
    println!();
    println!("commands:");
    for command in commands() {
        println!("  {:<62} {}", command.usage, command.summary);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let mut state_dir: Option<PathBuf> = None;
    let mut rest: &[String] = &arguments;
    if rest.first().map(String::as_str) == Some("--state-dir") {
        if rest.len() < 2 {
            return Err(Box::new(Usage("--state-dir requires a directory".into())));
        }
        state_dir = Some(PathBuf::from(&rest[1]));
        rest = &rest[2..];
    }
    let Some(name) = rest.first().cloned() else {
        print_help();
        exit(1);
    };
    if name == "help" || name == "--help" {
        print_help();
        return Ok(());
    }
    let args = &rest[1..];
    let Some(command) = commands().into_iter().find(|c| c.name == name) else {
        eprintln!("symbiote: unknown command {name}; try `symbiote help`");
        exit(1);
    };
    let mut operation = serde_json::Map::new();
    if let Err(usage) = (command.build)(args, &mut operation) {
        eprintln!("symbiote: {usage}");
        eprintln!("usage: symbiote --state-dir DIR {}", command.usage);
        exit(1);
    }
    let Some(directory) = state_dir else {
        eprintln!("symbiote: missing --state-dir PRIVATE_DIRECTORY");
        exit(1);
    };
    let correlation = format!(
        "{}-{}",
        name.replace('_', "-"),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or_default()
    );
    let request = serde_json::json!({
        "version": symbiote_protocol::CURRENT_VERSION,
        "correlation_id": correlation,
        "command_id": format!("cli-{correlation}"),
        "operation": operation,
    });
    let bytes = serde_json::to_vec(&request)?;
    let response_bytes = symbiote_host::transport::exchange(&directory, &bytes).map_err(|error| {
        format!(
            "cannot reach the daemon at {}: {error} (is symbioted running with --state-dir {}?)",
            directory.join("host.sock").display(),
            directory.display()
        )
    })?;
    let response: symbiote_protocol::Response = serde_json::from_slice(&response_bytes)
        .map_err(|error| format!("daemon sent an unparseable response: {error}"))?;
    match &response.result {
        Ok(body) => {
            println!(
                "{}",
                serde_json::to_string_pretty(body).unwrap_or_else(|_| format!("{body:?}"))
            );
        }
        Err(error) => {
            eprintln!(
                "{}",
                serde_json::to_string_pretty(error).unwrap_or_else(|_| format!("{error:?}"))
            );
            exit(2);
        }
    }
    Ok(())
}
