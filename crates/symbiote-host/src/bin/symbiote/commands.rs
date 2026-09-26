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

/// The help table's usage column. A usage that does not fit it is printed on
/// its own line with the summary beneath, so no command's arguments are ever
/// truncated or run into the next column.
const USAGE_COLUMN: usize = 62;

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

/// One `<contract_id>@<revision>` argument: the identity a contract is
/// recorded under, and the revision of it this command records. The domain
/// owns what an identity may contain; this owns the spelling, so a token that
/// is not one is refused here — naming the argument, the shape it wanted and
/// the value it got — rather than sent to be refused as an invalid request.
fn versioned(token: &str, name: &str) -> Result<(String, u64), Usage> {
    let shape = || {
        Usage(format!(
            "<{name}> must be <contract_id>@<revision>; got {token:?}"
        ))
    };
    let (id, revision) = token
        .split_once('@')
        .filter(|(id, revision)| !id.is_empty() && !revision.is_empty() && !revision.contains('@'))
        .ok_or_else(shape)?;
    let revision = revision.parse::<u64>().map_err(|_| {
        Usage(format!(
            "the revision in <{name}> is not a whole number: {token:?}"
        ))
    })?;
    Ok((id.to_owned(), revision))
}

/// The arguments a command cannot take. A command with an optional trailing
/// argument has a way to drop one by mistyping it, and a silently dropped
/// `<root_id>` or `<description>` is a request the operator did not make: the
/// surplus is named, never ignored.
fn no_more(args: Args, taken: usize, command: &str) -> Result<(), Usage> {
    match args.get(taken..).and_then(|extra| extra.first()) {
        Some(unexpected) => Err(Usage(format!(
            "{command} does not take the argument {unexpected:?}"
        ))),
        None => Ok(()),
    }
}

/// The classifications an Objective may carry, in the order a refusal lists
/// them. `maintenance` and `operational` are the two a Task may originate from
/// and `outcome` is the third the domain declares; which one this Objective is
/// decides whether `create-task` can name it, so the caller states it rather
/// than have the CLI invent a classification.
const OBJECTIVE_CLASSES: [&str; 3] = ["maintenance", "operational", "outcome"];

/// What a created Task's stream records before any work runs: the null Git
/// object id, which no commit can carry. Creating a task does no work, so its
/// stream names no commit, and a null id cannot be mistaken for one that
/// exists. A stream that carries real lineage is created through `raw`.
const NO_COMMIT: &str = "0000000000000000000000000000000000000000";

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
            name: "runtime-inventory",
            summary: "owner-only read of this Host's published runtime inventory",
            usage: "runtime-inventory",
            kind: Some("get_runtime_inventory"),
            build: |_, map| {
                plain("kind", serde_json::json!("get_runtime_inventory"), map);
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
            name: "list-projects",
            summary: "list the Projects this caller may read",
            usage: "list-projects",
            kind: Some("list_projects"),
            build: |_, map| {
                plain("kind", serde_json::json!("list_projects"), map);
                Ok(())
            },
        },
        Command {
            name: "snapshot",
            summary: "read a Project's records with the journal cursor they were read at",
            usage: "snapshot <project_id>",
            kind: Some("snapshot"),
            build: |args, map| {
                plain("kind", serde_json::json!("snapshot"), map);
                plain("project_id", field(args, 0, "project_id")?.into(), map);
                Ok(())
            },
        },
        Command {
            name: "register-project",
            summary: "register a Project with its lead Role and its Roots",
            usage: "register-project <project_id> <name> <lead_role_id> <lead_contract_id>@<revision> <root_id> [root_id ...]",
            kind: Some("register_project"),
            build: |args, map| {
                plain("kind", serde_json::json!("register_project"), map);
                let project = field(args, 0, "project_id")?;
                let name = field(args, 1, "name")?;
                let lead = field(args, 2, "lead_role_id")?;
                let (contract, revision) =
                    versioned(&field(args, 3, "lead_contract_id")?, "lead_contract_id")?;
                // A Root is an identity the Project has, not a host placement:
                // the daemon records those itself, per Host, once one has
                // observed the checkout, so a root registered here names no
                // repository and no path. A Project has at least one root, so
                // the first is required and the rest are named with it.
                field(args, 4, "root_id")?;
                let roots: Vec<serde_json::Value> = args[4..]
                    .iter()
                    .map(|root| {
                        serde_json::json!({
                            "id": root,
                            "project_id": project,
                            "revision": 0,
                            "repository": null,
                            "host_paths": {},
                        })
                    })
                    .collect();
                plain(
                    "project",
                    serde_json::json!({
                        "id": project,
                        "name": name,
                        "lead": lead,
                        "roots": roots,
                        "roles": [{
                            "id": lead,
                            "project_id": project,
                            "revision": 0,
                            // The Role is named for the identity it is
                            // registered under. A Project with more than one
                            // Role, or a Role under another contract, is
                            // registered through `raw`.
                            "name": lead,
                            "operating_contract": {"id": contract, "revision": revision},
                        }],
                    }),
                    map,
                );
                Ok(())
            },
        },
        Command {
            name: "create-work",
            summary: "create a classified Objective a Task can originate from",
            usage: "create-work <project_id> <role_id> <objective_id> <class> <title> [description]",
            kind: Some("create_work"),
            build: |args, map| {
                plain("kind", serde_json::json!("create_work"), map);
                let project = field(args, 0, "project_id")?;
                let role = field(args, 1, "role_id")?;
                let objective = field(args, 2, "objective_id")?;
                let class = field(args, 3, "class")?;
                if !OBJECTIVE_CLASSES.contains(&class.as_str()) {
                    return Err(Usage(format!(
                        "<class> must be one of {}; got {class:?}",
                        OBJECTIVE_CLASSES.join(", ")
                    )));
                }
                let title = field(args, 4, "title")?;
                let description = args.get(5).cloned().unwrap_or_default();
                no_more(args, 6, "create-work")?;
                plain(
                    "work",
                    serde_json::json!({
                        "id": {"kind": "objective", "id": objective},
                        "project_id": project,
                        "role_id": role,
                        "title": title,
                        "description": description,
                        "utterance": null,
                        "objective_class": class,
                        "parent": null,
                        "dependencies": [],
                        "requirements": [],
                        "constraints": [],
                        "risks": [],
                        "acceptance": [],
                        "priority": 0,
                        "budget": null,
                        "external_references": [],
                    }),
                    map,
                );
                Ok(())
            },
        },
        Command {
            name: "create-task",
            summary: "create a Task and its stream under an Objective",
            usage: "create-task <project_id> <task_id> <root_id> <role_id> <objective_id> <contract_id>@<revision> <branch>",
            kind: Some("create_task"),
            build: |args, map| {
                plain("kind", serde_json::json!("create_task"), map);
                no_more(args, 7, "create-task")?;
                let project = field(args, 0, "project_id")?;
                let task = field(args, 1, "task_id")?;
                let root = field(args, 2, "root_id")?;
                let role = field(args, 3, "role_id")?;
                let objective = field(args, 4, "objective_id")?;
                let (contract, revision) =
                    versioned(&field(args, 5, "contract_id")?, "contract_id")?;
                let branch = field(args, 6, "branch")?;
                plain(
                    "task",
                    serde_json::json!({
                        "id": task,
                        "project_id": project,
                        "root_id": root,
                        "role_id": role,
                        "origin": {
                            "kind": "objective",
                            "work": {
                                "project_id": project,
                                "id": {"kind": "objective", "id": objective},
                            },
                        },
                        "task_contract": {"id": contract, "revision": revision},
                        "stream": {
                            // The stream, its worktree and its chat are named
                            // for the task they belong to: one task, one
                            // stream, and nothing invented about where the work
                            // will land beyond the branch the caller named.
                            "id": format!("stream-{task}"),
                            "originating_chat": format!("chat-{task}"),
                            "worktree": format!("worktree-{task}"),
                            "branch": branch,
                            "base": NO_COMMIT,
                            "target": NO_COMMIT,
                        },
                    }),
                    map,
                );
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
            summary: "readiness, blockers, progress and the critical path for a project",
            usage: "scheduling-projection <project_id>",
            kind: Some("get_scheduling_projection"),
            build: |args, map| {
                plain("kind", serde_json::json!("get_scheduling_projection"), map);
                id_field(args, 0, "project_id", "project_id", map)?;
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
        "       symbiote schema [envelope|policy] [--write DIR | --check DIR]\n       symbiote publish-runtime-inventory DOCUMENT --state-dir DIR\n       symbiote help   (--command-id overrides the minted id; same id + same\n                        intent replays a lost response instead of re-executing)"
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
    println!("--policy, --json and --yes; `schema` honors --write and --check;");
    println!("`publish-runtime-inventory` honors --state-dir; `help` honors nothing.");
    println!("`--help`/`-h` is universal, and a help request honors no other flag:");
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
    println!(
        "   {schema_name:<width$} {schema_summary}",
        width = USAGE_COLUMN
    );
    let publish_name = "publish-runtime-inventory";
    let publish_summary = "install a discovery document as this Host's runtime inventory";
    println!(
        "   {publish_name:<width$} {publish_summary}",
        width = USAGE_COLUMN
    );
    for command in commands() {
        let gated = match command.kind {
            Some(kind) => risk_of_kind(kind).requires_authorization(),
            // `raw` has no declared kind, so it MAY send a dangerous one.
            None => true,
        };
        let marker = if gated { "*" } else { " " };
        // A usage that will not fit beside the summary gets the line to
        // itself and the summary indented under it, so the table stays
        // readable whatever a command's arguments spell rather than running
        // the summary into the usage.
        if command.usage.chars().count() > USAGE_COLUMN {
            println!(" {marker} {}", command.usage);
            println!("   {}", command.summary);
        } else {
            println!(
                " {marker} {:<width$} {}",
                command.usage,
                command.summary,
                width = USAGE_COLUMN
            );
        }
    }
}
