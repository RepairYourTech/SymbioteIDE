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

/// One required argument, by position. Two refusals live here rather than in
/// each builder, because a command line has two ways to be incomplete and both
/// are the operator's to fix by name: the argument is absent, or it is there
/// and empty. An empty identity, name, title or branch can never be valid —
/// the domain refuses every one of them — so answering with the daemon's
/// `invalid_request` would name a cause and a request the operator did not
/// recognize, when the position is known here. What an argument may *contain*
/// (its length, its characters, whether it is only whitespace) stays the
/// domain's: this checks that an argument is there, not what it says.
fn field(args: Args, index: usize, name: &str) -> Result<String, Usage> {
    let value = args
        .get(index)
        .cloned()
        .ok_or_else(|| Usage(format!("missing <{name}>")))?;
    if value.is_empty() {
        return Err(Usage(format!("empty <{name}>")));
    }
    Ok(value)
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

/// One `<base_commit>` argument: the commit the work starts from. A stream
/// records the base it was opened at, and the Host checks it against the
/// repository's real HEAD before it provisions anything, so the spelling is
/// owned here rather than left to a typed refusal that arrives only after the
/// Task is on record. A null object id is refused for the same reason it is
/// never written: no commit can carry it, so naming it would ask for a base
/// that cannot exist.
fn commit(token: &str) -> Result<String, Usage> {
    let valid = token.len() == 40
        && token.bytes().all(|b| b.is_ascii_hexdigit())
        && !token.bytes().all(|b| b == b'0');
    if valid {
        return Ok(token.to_ascii_lowercase());
    }
    Err(Usage(format!(
        "<base_commit> must be a full 40-character commit id that is not the null id; got {token:?}"
    )))
}

/// The worktree identity and branch the Host will require for this stream,
/// derived by the Host's own rule rather than invented here. The three
/// identities are the ones the derivation is bound to, so a caller that got
/// them wrong is refused here instead of at `run-started-dispatch`.
fn derived_for(
    project: &str,
    root: &str,
    stream: &str,
) -> Result<symbiote_worktrees::Derived, Usage> {
    let project = symbiote_domain::ProjectId::new(project).map_err(|_| {
        Usage(format!(
            "<project_id> is not a usable identity: {project:?}"
        ))
    })?;
    let root = symbiote_domain::RootId::new(root)
        .map_err(|_| Usage(format!("<root_id> is not a usable identity: {root:?}")))?;
    let stream = symbiote_domain::ChangeStreamId::new(stream).map_err(|_| {
        Usage(format!(
            "<task_id> does not yield a usable stream id: {stream:?}"
        ))
    })?;
    symbiote_host::runner::derived_worktree(&project, &root, &stream).map_err(|_| {
        Usage(
            "this Project, Root and stream identity do not derive a worktree; a worktree \
             branch is reserved under `symbiote/<project>/<stream>` and every component must \
             be a valid Git ref component"
                .into(),
        )
    })
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

/// One value from a closed set the domain declares, named by the caller and
/// refused here with the whole set in the message. The set is written out per
/// argument rather than read from the domain, because a refusal that arrives
/// from the daemon names a code and a request the operator did not recognize,
/// while the value they typed and the choices they had are the CLI's to answer.
/// The verification gates a dispatch contract can require, by the spelling
/// the wire uses. One table, because a gate spelled two ways in two commands
/// is a gate a caller can pass under one name and have refused under another.
const GATE_SPELLINGS: &[(&str, symbiote_domain::Gate)] = &[
    ("tests", symbiote_domain::Gate::Tests),
    ("security", symbiote_domain::Gate::Security),
    ("impact", symbiote_domain::Gate::Impact),
    ("documentation", symbiote_domain::Gate::Documentation),
    (
        "independent_review",
        symbiote_domain::Gate::IndependentReview,
    ),
    ("delivery", symbiote_domain::Gate::Delivery),
    (
        "capability_closure",
        symbiote_domain::Gate::CapabilityClosure,
    ),
];

fn one_of<T: Copy>(token: &str, name: &str, choices: &[(&str, T)]) -> Result<T, Usage> {
    choices
        .iter()
        .find(|(spelling, _)| *spelling == token)
        .map(|(_, value)| *value)
        .ok_or_else(|| {
            Usage(format!(
                "<{name}> must be one of {}; got {token:?}",
                choices
                    .iter()
                    .map(|(spelling, _)| *spelling)
                    .collect::<Vec<_>>()
                    .join(", ")
            ))
        })
}

/// The functions a Role may hold in a Team, in the order a refusal lists them.
/// `lead_orchestrator` is the one the Project's lead must have and the only
/// one exactly one member may; the other two are how the remaining members
/// differ. Which function a Role has decides what it is asked to do, so the
/// caller states it rather than have the CLI infer it from the lead's identity.
/// The grants a member may hold, in the order a refusal lists them. `read_root`
/// is not optional in the domain — a member that cannot read the Root cannot be
/// reviewed against it — so it is added to every member rather than left for the
/// caller to remember on each one.
const ROLE_FUNCTIONS: [(&str, &str); 3] = [
    ("lead_orchestrator", "lead_orchestrator"),
    ("general_execution", "general_execution"),
    ("specialist", "specialist"),
];
const MEMBER_GRANTS: [(&str, &str); 5] = [
    ("read_root", "read_root"),
    ("mutate_stream", "mutate_stream"),
    ("execute_process", "execute_process"),
    ("network", "network"),
    ("use_credential", "use_credential"),
];

/// The fields one `<member>` argument carries, in the order they are read. A
/// member is the one record in a Team with more than one fact in it, so the CLI
/// owns the spelling rather than making an operator hand-write a policy object:
/// the field order is this list, and a record that is not this shape is refused
/// naming the shape it wanted.
const MEMBER_FIELDS: [&str; 6] = [
    "role_id",
    "function",
    "grant,[grant...]",
    "root,[root...]",
    "domain,[domain...]",
    "reviewer,[reviewer...]",
];

/// One `<member>` argument: a Role, what it is, what it may reach, what it
/// works on, and who reviews it.
///
/// Every field is a fact the operator holds. The access is the member's own,
/// not a copy of the team's, because a Team exists precisely so two roles can
/// reach different places — a command that gave every member the same grants
/// would be spelling a ceiling and calling it a team.
fn member(project: &str, token: &str, policies: &[String; 4]) -> Result<serde_json::Value, Usage> {
    let shape = || {
        Usage(format!(
            "<member> must be {} separated by `:`; got {token:?}",
            MEMBER_FIELDS.join(":")
        ))
    };
    let values: Vec<&str> = token.split(':').collect();
    if values.len() != MEMBER_FIELDS.len() {
        return Err(shape());
    }
    if let Some(at) = values.iter().position(|value| value.is_empty()) {
        return Err(Usage(format!(
            "<member>'s <{}> is empty in {token:?}",
            MEMBER_FIELDS[at]
        )));
    }
    let role = values[0];
    let function = one_of(values[1], "function", &ROLE_FUNCTIONS)?;

    let mut grants = list(values[2], "grant")?;
    for grant in &grants {
        one_of(grant, "grant", &MEMBER_GRANTS)?;
    }
    if !grants.iter().any(|grant| grant == "read_root") {
        // A member that cannot read the Root cannot be reviewed against it,
        // so the domain refuses the Team whatever the caller wrote. Adding the
        // grant the domain requires is stating a fact about what a member is,
        // not widening one the operator withheld.
        grants.push("read_root".to_owned());
    }

    let roots = list(values[3], "root")?;
    if roots.is_empty() {
        return Err(Usage(format!(
            "a <member> must name at least one <root>: {token:?} reaches nothing"
        )));
    }
    let domains = list(values[4], "domain")?;
    let reviewers = list(values[5], "reviewer")?;
    if reviewers.is_empty() {
        return Err(Usage(format!(
            "a <member> must name at least one <reviewer>: the domain refuses a Team with an \
             unreviewed role, and {token:?} named none"
        )));
    }
    if reviewers.iter().any(|reviewer| reviewer == role) {
        return Err(Usage(format!(
            "a <member> cannot review itself: {token:?} names {role:?} as its own reviewer"
        )));
    }
    Ok(serde_json::json!({
        "role_id": role,
        "function": function,
        // A member's responsibilities ARE the domains it works in. The domain
        // requires the list to be non-empty and the routing decision reads
        // the domains, so spelling both separately would let a Team record
        // say a role is responsible for work it never routes to.
        "responsibilities": domains,
        "task_domains": domains,
        "access": {
            "project_id": project,
            "roots": roots,
            "grants": grants,
            "policy_revision": 1,
        },
        "context_policy_ref": policies[0],
        "tool_policy_ref": policies[1],
        "skill_policy_ref": policies[2],
        "execution_policy_ref": policies[3],
        "independent_reviewers": reviewers,
        // A fallback is the role that takes over when this one cannot, and
        // who that is is a decision about escalation rather than about a
        // role's shape. A member that names one is `raw`.
        "fallbacks": [],
    }))
}

/// The members of a Team, one `<member>` per argument.
fn team_members(
    tokens: &[String],
    project: &str,
    policies: &[String; 4],
) -> Result<Vec<serde_json::Value>, Usage> {
    let mut members: Vec<serde_json::Value> = Vec::new();
    for token in tokens {
        if token.is_empty() {
            return Err(Usage("empty <member>".into()));
        }
        let role = member(project, token, policies)?;
        // A Role in a Team twice is a Team carrying two policies for one
        // Role, and the domain refuses it as a duplicate. Naming it here says
        // which argument repeated, which the store's typed refusal cannot.
        if members
            .iter()
            .any(|held| held["role_id"] == role["role_id"])
        {
            return Err(Usage(format!(
                "<member> names role {:?} twice; a Team holds one policy per Role",
                role["role_id"].as_str().unwrap_or_default()
            )));
        }
        members.push(role);
    }
    Ok(members)
}

/// One `<a,b,c>` list argument: the comma-separated values of a set the caller
/// holds. An empty element is a value the caller did not type, and a set with
/// an empty member is a set nobody declared, so it is refused by position
/// rather than quietly dropped.
fn list(token: &str, name: &str) -> Result<Vec<String>, Usage> {
    let values: Vec<String> = token.split(',').map(|value| value.to_owned()).collect();
    if values.iter().any(|value| value.is_empty()) {
        return Err(Usage(format!(
            "<{name}> is a comma-separated list; an entry of it is empty in {token:?}"
        )));
    }
    Ok(values)
}

/// One `<whole number>` argument the domain bounds rather than the CLI: a
/// sequence, a page size, a token count, a revision, a millisecond deadline.
/// These are the arguments a script computes, so the refusal has to say what
/// the value was — a bare "not a number" leaves a script with nothing to fix.
fn number<T: std::str::FromStr>(token: &str, name: &str) -> Result<T, Usage> {
    token
        .parse::<T>()
        .map_err(|_| Usage(format!("<{name}> must be a whole number; got {token:?}")))
}

/// One `<policy_ref>@<revision>` argument of a binding's seven policies. They
/// are named one argument each rather than derived: a policy document is
/// something the operator publishes, and a reference the CLI invented would
/// name a document that does not exist.
fn policy(name: &str, args: Args, index: usize) -> Result<serde_json::Value, Usage> {
    let (id, revision) = versioned(&field(args, index, name)?, name)?;
    Ok(serde_json::json!({"id": id, "revision": revision}))
}

/// The enforcement each required control needs. The domain refuses a required
/// control this map does not mention, and refuses any strength of
/// `unsupported`, so every control a binding requires is mapped to the one
/// strength that is both named and supported: the Host enforces the controls
/// it composes. A binding that claims a weaker or stronger enforcement for a
/// control is `raw`, because the claim is the operator's to make.
fn minimum_enforcement(controls: &[String]) -> serde_json::Value {
    let mut map = serde_json::Map::new();
    for control in controls {
        map.insert(control.clone(), serde_json::json!("host_enforced"));
    }
    serde_json::Value::Object(map)
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

/// What a command's success does that its receipt cannot say, for the operator
/// reading the terminal.
///
/// A receipt is a fact about the store, and the fact that decides how an
/// operator reads one of these commands is not in it. A Task owns exactly one
/// Change Stream for its whole life: the worktree and the branch derive from
/// that stream, and the stream is what `abandon_change_stream` cancels. Every
/// other failure in this product is retryable — a refused run, a lost lease, a
/// daemon that was killed mid-flight — and the abandon is the one that is
/// terminal. An operator who learns that from the refusal of the retry they
/// already paid for has learned it too late, so the commands that create the
/// cost and the ones that charge it say so here.
///
/// The note goes to stderr and never into the envelope. `--json` output is a
/// published contract of one object per invocation, and this is a sentence for
/// a person rather than a field for a parser — which is also why it is a table
/// beside the commands rather than something the daemon returns: the daemon
/// cannot know what an operator is about to do next, and a note the operator
/// cannot miss at the moment of the decision is worth more than one they can
/// query afterwards.
pub(crate) fn consequence(name: &str) -> Option<&'static str> {
    match name {
        "create-task" => Some(
            "this Task owns one Change Stream for its whole life: its worktree and branch derive \
             from that stream, and abandoning the stream is terminal, so the retry after an \
             abandon is a new Task.",
        ),
        "abandon-change-stream" => Some(
            "the stream is cancelled and its checkout is gone, with whatever was uncommitted in \
             it; every commit on the branch survives, and the retry is a new Task.",
        ),
        "release-change-stream" => Some(
            "the checkout is reclaimed and the work has landed; the branch and its commits stay \
             on the Root as the stream's record.",
        ),
        "interrupt-task" => Some(
            "the run's lease is released and the Task is Interrupted; recover-task returns it to \
             Ready, and the same stream carries the next run.",
        ),
        "advance-change-stream" => Some(
            "the stream's head is now its checkout's commit and its target is the Root's own \
             HEAD, so any evidence recorded before this is stale and validate-change-stream has \
             to run again",
        ),
        "complete-task" => Some(
            "the gates are filed as evidence and the Task is Completed; the work is not on the \
             Root yet, and integrate-change-stream is what lands it.",
        ),
        "recover-task" => Some(
            "the Task is Ready again and its dispatch binding is dropped; the next run provisions \
             the same stream's own checkout again, so a run that failed there is not lost.",
        ),
        "cancel-task" => Some(
            "the Task is Cancelled and nothing will run it again; the stream it was cut from stays \
             on the Root, and only the Task is closed.",
        ),
        _ => None,
    }
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
            summary: "register a Project with its Roots and its Roles",
            usage: "register-project <project_id> <name> <lead_role_id> <lead_contract_id>@<revision> \
                    <root_id>[,<root_id>...] [role_id:contract_id:revision ...]",
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
                // repository and no path. A Project has at least one root, and
                // they are one comma-separated argument so that the Roles
                // after them have a position of their own.
                let root_ids = list(&field(args, 4, "root_id")?, "root_id")?;
                let roots: Vec<serde_json::Value> = root_ids
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
                // A Role is named for the identity it is registered under, and
                // every Role — the lead's included — is named here rather than
                // only the lead's. A Project whose Team has an execution role
                // cannot be registered otherwise: the store refuses a Team that
                // names a Role the Project does not hold, so a command that
                // registered the lead alone could never produce a Team the
                // domain accepts.
                let mut roles = vec![serde_json::json!({
                    "id": lead,
                    "project_id": project,
                    "revision": 0,
                    "name": lead,
                    "operating_contract": {"id": contract, "revision": revision},
                })];
                for token in args.get(5..).unwrap_or_default() {
                    if token.is_empty() {
                        return Err(Usage("empty <role_id>".into()));
                    }
                    let parts: Vec<&str> = token.split(':').collect();
                    let shape = || {
                        Usage(format!(
                            "<role_id> must be <role_id>:<contract_id>:<revision>; got {token:?}"
                        ))
                    };
                    if parts.len() != 3 || parts.iter().any(|part| part.is_empty()) {
                        return Err(shape());
                    }
                    if parts[0] == lead {
                        return Err(Usage(format!(
                            "<role_id> names the lead {lead:?} a second time: a Project's lead \\
                             Role is registered from <lead_role_id> and \\
                             <lead_contract_id>@<revision>"
                        )));
                    }
                    if roles
                        .iter()
                        .any(|role| role["id"] == serde_json::json!(parts[0]))
                    {
                        return Err(Usage(format!(
                            "<role_id> names {:?} twice: a Project holds one Role per identity",
                            parts[0]
                        )));
                    }
                    roles.push(serde_json::json!({
                        "id": parts[0],
                        "project_id": project,
                        "revision": 0,
                        "name": parts[0],
                        "operating_contract": {
                            "id": parts[1],
                            "revision": number::<u64>(parts[2], "revision")?,
                        },
                    }));
                }
                plain(
                    "project",
                    serde_json::json!({
                        "id": project,
                        "name": name,
                        "lead": lead,
                        "roots": roots,
                        "roles": roles,
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
            usage: "create-task <project_id> <task_id> <root_id> <role_id> <objective_id> <contract_id>@<revision> <base_commit>",
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
                let base = commit(&field(args, 6, "base_commit")?)?;
                // The worktree identity and the branch are the Host's, not the
                // caller's: the Host re-derives both from its own policy seed
                // at provisioning and refuses a stream whose recorded names
                // differ. A branch argument here was accepted, recorded, and
                // then always refused at `run-started-dispatch` — so the
                // caller could name a branch and never get it. There is one
                // derivation, and `symbiote_host::runner::derived_worktree` is
                // it, so this command records exactly what the Host requires.
                let derived = derived_for(&project, &root, &format!("stream-{task}"))?;
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
                            // One task, one stream, one chat. The worktree and
                            // branch come from the Host's derivation above;
                            // the base is the commit the work starts from, and
                            // the Host checks it against the repository's real
                            // HEAD before it provisions anything, so a caller
                            // that names a commit the repository does not have
                            // is refused here rather than after a Task is on
                            // record as dispatchable.
                            "id": format!("stream-{task}"),
                            "originating_chat": format!("chat-{task}"),
                            "worktree": derived.worktree_id.to_string(),
                            "branch": derived.branch,
                            "base": base,
                            // Where the Root is until work lands, which is
                            // the base: the Host has just checked the
                            // repository is at it.
                            "target": base,
                        },
                    }),
                    map,
                );
                Ok(())
            },
        },
        Command {
            name: "observe-root-placement",
            summary: "record a Root's checkout path on this Host",
            usage: "observe-root-placement <project_id> <root_id> <host_id> <path> <expected_revision>",
            kind: Some("observe_root_placement"),
            build: |args, map| {
                plain("kind", serde_json::json!("observe_root_placement"), map);
                plain("project_id", field(args, 0, "project_id")?.into(), map);
                plain("root_id", field(args, 1, "root_id")?.into(), map);
                // The Host the caller claims. The daemon refuses a foreign
                // Host with `permission_denied` rather than recording a
                // placement it cannot observe, so this is a fact the operator
                // holds (their own `host-pulse`) and not a derived value.
                plain("host_id", field(args, 2, "host_id")?.into(), map);
                // The checkout the Host verifies itself: it canonicalizes this
                // and refuses unless a real git work tree is there. A root
                // registered by `register-project` names no path, so this is
                // where the path a registration left out is observed.
                plain("path", field(args, 3, "path")?.into(), map);
                plain(
                    "expected_revision",
                    number::<u64>(&field(args, 4, "expected_revision")?, "expected_revision")?
                        .into(),
                    map,
                );
                Ok(())
            },
        },
        Command {
            name: "register-provider-connection",
            summary: "register an inference provider connection",
            usage: "register-provider-connection <project_id> <connection_id> <adapter> <endpoint_reference> <authentication>",
            kind: Some("replace_provider_connection"),
            build: |args, map| {
                plain(
                    "kind",
                    serde_json::json!("replace_provider_connection"),
                    map,
                );
                let project = field(args, 0, "project_id")?;
                // The connection carries no Project: which Project may name
                // this connection is the attribution on the operation, so the
                // same record is shared by every Project entitled to it.
                plain("attribution", project.clone().into(), map);
                let connection_id = field(args, 1, "connection_id")?;
                let adapter = field(args, 2, "adapter")?;
                // A reference, never a secret: the connection names where to
                // reach, and the credential lives in the operator's broker,
                // not in this record.
                let endpoint = field(args, 3, "endpoint_reference")?;
                let authentication = one_of(
                    &field(args, 4, "authentication")?,
                    "authentication",
                    &[
                        ("api_credential", "api_credential"),
                        ("harness_managed", "harness_managed"),
                        ("local_unauthenticated", "local_unauthenticated"),
                    ],
                )?;
                plain(
                    "connection",
                    serde_json::json!({
                        "id": connection_id,
                        "adapter": adapter,
                        "endpoint_reference": endpoint,
                        "authentication": authentication,
                    }),
                    map,
                );
                Ok(())
            },
        },
        Command {
            name: "register-billing-entitlement",
            summary: "register the billing entitlement for a provider",
            usage: "register-billing-entitlement <project_id> <entitlement_id> <connection_id> <kind> <evidence_id> <expires_at_ms>",
            kind: Some("replace_billing_entitlement"),
            build: |args, map| {
                plain(
                    "kind",
                    serde_json::json!("replace_billing_entitlement"),
                    map,
                );
                let project = field(args, 0, "project_id")?;
                plain("attribution", project.clone().into(), map);
                // Every argument is read in the order it is typed, so a line
                // stopping before position `i` is refused naming position `i`.
                // That only holds if nothing further down the command line has
                // been read yet, which is why the expiry — the last argument —
                // is not read first here.
                let entitlement_id = field(args, 1, "entitlement_id")?;
                let connection = field(args, 2, "connection_id")?;
                let billing = one_of(
                    &field(args, 3, "kind")?,
                    "kind",
                    &[
                        ("metered_api", "metered_api"),
                        ("harness_subscription", "harness_subscription"),
                        ("local", "local"),
                    ],
                )?;
                let evidence = field(args, 4, "evidence_id")?;
                let expires = number::<i64>(&field(args, 5, "expires_at_ms")?, "expires_at_ms")?;
                if expires < 0 {
                    return Err(Usage(format!(
                        "<expires_at_ms> is a Unix millisecond instant and cannot be negative; got \
                         {expires}"
                    )));
                }
                plain(
                    "entitlement",
                    serde_json::json!({
                        "id": entitlement_id,
                        // The entitlement belongs to the connection it pays
                        // for, so a caller cannot register an entitlement
                        // against a provider that is not the one it names.
                        "provider": connection,
                        "kind": billing,
                        // Evidence, not a proof this record carries: what the
                        // operator observed to earn the entitlement.
                        "verification_evidence": evidence,
                        "expires_at": expires,
                    }),
                    map,
                );
                Ok(())
            },
        },
        Command {
            name: "register-model-descriptor",
            summary: "register a model's context window and capabilities",
            usage: "register-model-descriptor <project_id> <model_id> <connection_id> <context_window_tokens> <max_output_tokens> [capability ...]",
            kind: Some("replace_model_descriptor"),
            build: |args, map| {
                plain("kind", serde_json::json!("replace_model_descriptor"), map);
                let project = field(args, 0, "project_id")?;
                plain("attribution", project.clone().into(), map);
                // Read in the order they are typed, like every other command.
                let model_id = field(args, 1, "model_id")?;
                let connection_id = field(args, 2, "connection_id")?;
                let window = number::<u64>(
                    &field(args, 3, "context_window_tokens")?,
                    "context_window_tokens",
                )?;
                let output =
                    number::<u64>(&field(args, 4, "max_output_tokens")?, "max_output_tokens")?;
                // A model that cannot hold its own output is a descriptor no
                // request could ever satisfy, and the request that discovers
                // it is a `context_limit` refusal from a provider turn long
                // after the descriptor was on record. The two figures are
                // compared here, where both are on the command line.
                if output == 0 || output > window {
                    return Err(Usage(format!(
                        "<max_output_tokens> must be at least 1 and no more than \
                         <context_window_tokens> ({window}); got {output}"
                    )));
                }
                // The capabilities are a set the model declares, each named by
                // the caller; `reasoning:<effort>` adds one effort to the set
                // and the three bare names set their flag. Nothing is inferred
                // from the connection: a model that cannot stream says so.
                let mut reasoning: Vec<&str> = Vec::new();
                let (mut tools, mut images, mut streaming) = (false, false, false);
                for capability in args.get(5..).unwrap_or_default() {
                    if capability.is_empty() {
                        return Err(Usage("empty <capability>".into()));
                    }
                    match capability.split_once(':') {
                        Some(("reasoning", effort)) => {
                            one_of(
                                effort,
                                "capability",
                                &[
                                    ("none", "none"),
                                    ("minimal", "minimal"),
                                    ("low", "low"),
                                    ("medium", "medium"),
                                    ("high", "high"),
                                    ("x_high", "x_high"),
                                    ("max", "max"),
                                    ("ultra", "ultra"),
                                ],
                            )?;
                            reasoning.push(match effort {
                                "none" => "none",
                                "minimal" => "minimal",
                                "low" => "low",
                                "medium" => "medium",
                                "high" => "high",
                                "x_high" => "x_high",
                                "max" => "max",
                                _ => "ultra",
                            });
                        }
                        _ => match capability.as_str() {
                            "tools" => tools = true,
                            "images" => images = true,
                            "streaming" => streaming = true,
                            other => {
                                return Err(Usage(format!(
                                    "<capability> must be tools, images, streaming or \
                                     reasoning:<effort>; got {other:?}"
                                )));
                            }
                        },
                    }
                }
                reasoning.sort_unstable();
                reasoning.dedup();
                plain(
                    "descriptor",
                    serde_json::json!({
                        // The contract version this binary speaks, not a
                        // caller-supplied number that could name a document
                        // this daemon does not implement.
                        "schema_version":
                            symbiote_runtime_sdk::provider::PROVIDER_CONTRACT_VERSION,
                        "id": model_id,
                        "provider_id": connection_id,
                        "context_window_tokens": window,
                        "max_output_tokens": output,
                        "capabilities": {
                            "reasoning_efforts": reasoning,
                            "tools": tools,
                            "images": images,
                            "streaming": streaming,
                        },
                    }),
                    map,
                );
                Ok(())
            },
        },
        Command {
            name: "record-route",
            summary: "record which Role a work item routes to",
            usage: "record-route <project_id> <objective_id> <role_id> [domain ...]",
            kind: Some("record_route"),
            build: |args, map| {
                plain("kind", serde_json::json!("record_route"), map);
                let project = field(args, 0, "project_id")?;
                let objective = field(args, 1, "objective_id")?;
                // An Objective, because `create-work` files Objectives: the
                // work id is a tagged union, and the kind is derived from the
                // fact that this is the one kind a typed command files. A
                // route to another kind of work is recorded through `raw`.
                let work = serde_json::json!({"kind": "objective", "id": objective});
                let role = field(args, 2, "role_id")?;
                let domains = args.get(3..).unwrap_or_default();
                // A route with neither a named Role nor a domain routes
                // nothing, which the domain refuses as an invalid request.
                // Saying so here names the two arguments that could each have
                // supplied one, rather than sending a request whose only
                // answer would be a code.
                if domains.is_empty() && role.is_empty() {
                    return Err(Usage(
                        "record-route needs a <role_id> or at least one <domain> to route by"
                            .into(),
                    ));
                }
                plain(
                    "request",
                    serde_json::json!({
                        "project_id": project,
                        "work_id": work,
                        "requested": role,
                        "domains": domains,
                    }),
                    map,
                );
                Ok(())
            },
        },
        Command {
            name: "replace-team",
            summary: "register a Project's Team and its members' access",
            usage: "replace-team <project_id> <lead_role_id> <context_ref> <tool_ref> <skill_ref> \
                    <execution_ref> <member> [member ...]",
            kind: Some("replace_team"),
            build: |args, map| {
                plain("kind", serde_json::json!("replace_team"), map);
                let project = field(args, 0, "project_id")?;
                let lead = field(args, 1, "lead_role_id")?;
                // The four policy references a member carries are the same for
                // every member in every document this tree ships, so they are
                // named once for the team rather than repeated per member,
                // where one of them could disagree with its own team.
                let policies = [
                    field(args, 2, "context_ref")?,
                    field(args, 3, "tool_ref")?,
                    field(args, 4, "skill_ref")?,
                    field(args, 5, "execution_ref")?,
                ];
                let members = team_members(args.get(6..).unwrap_or_default(), &project, &policies)?;
                if members.is_empty() {
                    return Err(Usage(
                        "replace-team needs at least one <member>: a Team with no member has no \
                         lead role"
                            .into(),
                    ));
                }
                // The ceiling is the union of what the members were granted, so
                // `access_within` holds by construction: a member can never
                // reach further than the team does, and the ceiling can never
                // be narrower than the member it bounds.
                let mut roots: Vec<String> = Vec::new();
                let mut grants: Vec<String> = Vec::new();
                for member in &members {
                    for (key, into) in [("roots", &mut roots), ("grants", &mut grants)] {
                        for value in member["access"][key].as_array().into_iter().flatten() {
                            let value = value.as_str().unwrap_or_default().to_owned();
                            if !into.contains(&value) {
                                into.push(value);
                            }
                        }
                    }
                }
                roots.sort();
                grants.sort();
                // A first replacement, so the expected revision is the absent
                // one and the store takes it as "create at revision 0". A team
                // replaced over an existing one is `raw`, which is where the
                // expected revision is stated.
                plain("expected_revision", serde_json::Value::Null, map);
                plain(
                    "team",
                    serde_json::json!({
                        "schema_version": 1,
                        "project_id": project,
                        "revision": 0,
                        "lead_role_id": lead,
                        "members": members,
                        "access_ceiling": {
                            "project_id": project,
                            "roots": roots,
                            "grants": grants,
                            "policy_revision": 1,
                        },
                    }),
                    map,
                );
                Ok(())
            },
        },
        Command {
            name: "replace-binding",
            summary: "register the runtime a Role is staffed with",
            usage: "replace-binding <project_id> <binding_id> <role_id> <team_revision> <profile_id>@\
                    <profile_revision> <protocol_id>@<protocol_revision> <connection_id> \
                    <credential_ref> <entitlement_id> <model_id> <runtime>:<adapter>:<host_id> \
                    <root>,<root> <grant>,<grant> <control>,<control> <context_bundle>@<revision> \
                    <max_input_tokens>:<reserved_output_tokens> <tool>,<tool> <environment_ref> \
                    <worktree_ref> <verification_ref> <documentation_ref> <artifacts_ref> \
                    <mcp_ref> <escalation_ref>",
            kind: Some("replace_binding"),
            build: |args, map| {
                plain("kind", serde_json::json!("replace_binding"), map);
                let project = field(args, 0, "project_id")?;
                let binding_id = field(args, 1, "binding_id")?;
                let role = field(args, 2, "role_id")?;
                let team_revision =
                    number::<u64>(&field(args, 3, "team_revision")?, "team_revision")?;
                let (profile, profile_revision) =
                    versioned(&field(args, 4, "profile_id")?, "profile_id")?;
                let (protocol, protocol_revision) =
                    versioned(&field(args, 5, "protocol_id")?, "protocol_id")?;
                let connection = field(args, 6, "connection_id")?;
                let credential = field(args, 7, "credential_ref")?;
                let entitlement = field(args, 8, "entitlement_id")?;
                let model = field(args, 9, "model_id")?;
                // The runtime lane, the agent adapter that drives it, and the
                // Host it may run on. Which of the three an external-harness
                // binding needs is the domain's rule — it refuses a harness
                // with no installation and an installation with no harness —
                // so all three are named here rather than one being chosen by
                // the CLI on the operator's behalf.
                let lane = field(args, 10, "runtime")?;
                let parts: Vec<&str> = lane.split(':').collect();
                let shape = || {
                    Usage(format!(
                        "<runtime> must be <runtime>:<adapter>:<host_id>; got {lane:?}"
                    ))
                };
                if parts.len() != 3 || parts.iter().any(|part| part.is_empty()) {
                    return Err(shape());
                }
                let (runtime, adapter, host) = (parts[0], parts[1], parts[2]);
                one_of(
                    runtime,
                    "runtime",
                    &[
                        ("NATIVE_SYMBIOTE", "NATIVE_SYMBIOTE"),
                        ("EXTERNAL_HARNESS", "EXTERNAL_HARNESS"),
                    ],
                )?;
                let roots = list(&field(args, 11, "root")?, "root")?;
                let grants = list(&field(args, 12, "grant")?, "grant")?;
                for grant in &grants {
                    one_of(grant, "grant", &MEMBER_GRANTS)?;
                }
                let controls = list(&field(args, 13, "control")?, "control")?;
                if controls.is_empty() {
                    return Err(Usage(
                        "a binding must name at least one <control> it requires enforced".into(),
                    ));
                }
                for control in &controls {
                    one_of(
                        control,
                        "control",
                        &[
                            ("filesystem", "filesystem"),
                            ("process", "process"),
                            ("network", "network"),
                            ("credentials", "credentials"),
                            ("cancellation", "cancellation"),
                            ("completion_authority", "completion_authority"),
                        ],
                    )?;
                }
                // The context bundle, its revision, the input budget and the
                // output reservation, in one record. The two token counts have
                // to sum to no more than the candidate's total limit, and that
                // limit is the one number the operator is stating, so the sum
                // is checked here where both sides are on the command line.
                let context = field(args, 14, "context_bundle")?;
                let (bundle, bundle_revision) = versioned(&context, "context_bundle")?;
                let budgets: Vec<String> = field(args, 15, "max_input_tokens")?
                    .split(':')
                    .map(|value| value.to_owned())
                    .collect();
                if budgets.len() != 2 {
                    return Err(Usage(format!(
                        "<max_input_tokens> must be <max_input_tokens>:<reserved_output_tokens>; \
                         got {:?}",
                        budgets.join(":")
                    )));
                }
                let input = number::<u64>(&budgets[0], "max_input_tokens")?;
                let reserved = number::<u64>(&budgets[1], "reserved_output_tokens")?;
                if input == 0 || reserved == 0 {
                    return Err(Usage(format!(
                        "<max_input_tokens> and <reserved_output_tokens> must both be at least 1; \
                         got {input} and {reserved}"
                    )));
                }
                let tools = list(&field(args, 16, "tool")?, "tool")?;
                // The seven policy documents a binding runs under. Each is a
                // thing the operator publishes, so each is named rather than
                // derived: a reference this command invented would name a
                // document that does not exist.
                let environment = policy("environment_ref", args, 17)?;
                let worktree = policy("worktree_ref", args, 18)?;
                let verification = policy("verification_ref", args, 19)?;
                let documentation = policy("documentation_ref", args, 20)?;
                let artifacts = policy("artifacts_ref", args, 21)?;
                let mcp = policy("mcp_ref", args, 22)?;
                let escalation_policy = policy("escalation_ref", args, 23)?;
                // The limits a candidate runs under. They are the dispatch's
                // own bound, so they are stated: a ceiling the CLI invented
                // would be an execution budget nobody chose. The token total
                // has to hold the context the binding declares, which is why
                // it is the one figure this command checks.
                const MAX_TOTAL_TOKENS: u64 = 100_000;
                const MAX_WALL_TIME_MS: u64 = 600_000;
                const MAX_MEMORY_BYTES: u64 = 512 * 1024 * 1024;
                if input + reserved > MAX_TOTAL_TOKENS {
                    return Err(Usage(format!(
                        "<max_input_tokens> ({} input) plus <reserved_output_tokens> ({} \
                         reserved) is {}, which is past the dispatch token limit of \
                         {MAX_TOTAL_TOKENS} this command states",
                        input,
                        reserved,
                        input + reserved
                    )));
                }
                let access = serde_json::json!({
                    "project_id": project,
                    "roots": roots,
                    "grants": grants,
                    "policy_revision": 1,
                });
                let context_policy = serde_json::json!({
                    "bundle": bundle,
                    "revision": bundle_revision,
                    "max_input_tokens": input,
                    "reserved_output_tokens": reserved,
                });
                // The profile the candidate runs. `installation` is null
                // because an installation is a fact about a foreign runtime
                // this command does not take, and the domain refuses a harness
                // that has none — so a binding for a foreign harness is `raw`.
                // The candidate's profile. Its identity and revision are the
                // two the command read, held under their own names: the
                // `profile` and `profile_revision` the binding names and the
                // `id`/`revision` the candidate does are the same two facts,
                // and the domain refuses a configuration where they differ.
                let profile_id = profile;
                let profile_revision_number = profile_revision;
                let profile = serde_json::json!({
                    "id": profile_id,
                    "revision": profile_revision_number,
                    "runtime": runtime,
                    "adapter": adapter,
                    "installation": null,
                    "provider": connection,
                    "credential": credential,
                    "billing_entitlement": entitlement,
                    "model": model,
                    "eligible_hosts": [host],
                });
                // The candidate and the binding it staffs must agree on their
                // access, context, tools and skills; the domain refuses a
                // configuration where they differ, and the reason it can
                // differ at all is that both are separate fields of one
                // document. They are written from the same values here, so
                // they cannot.
                let candidate = serde_json::json!({
                    "profile": profile,
                    "config_identity": "cli-default",
                    "access": access,
                    "tools": tools,
                    "skills": [],
                    "secret_scopes": [],
                    "context": context_policy,
                    "limits": {
                        "max_total_tokens": MAX_TOTAL_TOKENS,
                        "max_wall_time_ms": MAX_WALL_TIME_MS,
                        "max_concurrency": 1,
                        "max_memory_bytes": MAX_MEMORY_BYTES,
                    },
                });
                plain("expected_revision", serde_json::Value::Null, map);
                plain(
                    "configuration",
                    serde_json::json!({
                        "schema_version": symbiote_workforce::BINDING_VERSION,
                        "binding": {
                            "id": binding_id,
                            "revision": 0,
                            "project_id": project,
                            "role_id": role,
                            "profile_id": profile_id,
                            "profile_revision": profile_revision_number,
                            "protocol": {"id": protocol, "revision": protocol_revision},
                            "access": access,
                            "required_controls": controls,
                            "context": context_policy,
                            "required_tools": tools,
                            "required_skills": [],
                            "escalation": "return_to_lead",
                        },
                        "team_revision": team_revision,
                        "primary": candidate,
                        "fallbacks": [],
                        "policies": {
                            // The seven policy references are named after the
                            // binding they belong to rather than invented: a
                            // policy document is a thing the operator
                            // publishes, and a CLI that named one the
                            // operator never wrote would be recording a
                            // reference to nothing.
                            "environment": environment,
                            "worktree": worktree,
                            "verification": verification,
                            "documentation": documentation,
                            "artifacts": artifacts,
                            "mcp": mcp,
                            "escalation": escalation_policy,
                            "root_effort": "medium",
                            "local_children": {"kind": "disabled"},
                            "fallback_consent": "explicit_required",
                            // Every control the binding requires has to name
                            // the enforcement it needs, and the domain refuses
                            // a required control this map does not mention.
                            "minimum_enforcement": minimum_enforcement(&controls),
                            "required_capabilities": [],
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
            name: "advance-change-stream",
            summary: "record where the stream's work and the Root's head are",
            usage: "advance-change-stream <task_id>",
            kind: Some("advance_change_stream"),
            build: |args, map| {
                plain("kind", serde_json::json!("advance_change_stream"), map);
                id_field(args, 0, "task_id", "task_id", map)?;
                // No commit arguments, which is the whole point: the Host reads
                // both commits out of the stream's own checkout and the Root's,
                // so nothing a caller types can become the stream's record.
                Ok(())
            },
        },
        Command {
            name: "validate-change-stream",
            summary: "record that a dispatched stream's work is where it says it is",
            usage: "validate-change-stream <task_id> <verification_run>",
            kind: Some("validate_change_stream"),
            build: |args, map| {
                plain("kind", serde_json::json!("validate_change_stream"), map);
                id_field(args, 0, "task_id", "task_id", map)?;
                // The run is a fact the operator holds and names. The commit
                // it validated is not theirs to state: the Host observes the
                // worktree, because a validation of a commit nobody looked at
                // is a validation of nothing.
                id_field(args, 1, "verification_run", "verification_run", map)?;
                Ok(())
            },
        },
        Command {
            name: "begin-task-verification",
            summary: "move a completion-requested task into verification",
            usage: "begin-task-verification <task_id>",
            kind: Some("begin_task_verification"),
            build: |args, map| {
                plain("kind", serde_json::json!("begin_task_verification"), map);
                id_field(args, 0, "task_id", "task_id", map)?;
                Ok(())
            },
        },
        Command {
            name: "integrate-change-stream",
            summary: "fast-forward a validated stream into its root's checkout",
            usage: "integrate-change-stream <task_id>",
            kind: Some("integrate_change_stream"),
            build: |args, map| {
                plain("kind", serde_json::json!("integrate_change_stream"), map);
                id_field(args, 0, "task_id", "task_id", map)?;
                Ok(())
            },
        },
        Command {
            name: "release-change-stream",
            summary: "reclaim an integrated stream's worktree",
            usage: "release-change-stream <task_id>",
            kind: Some("release_change_stream"),
            build: |args, map| {
                plain("kind", serde_json::json!("release_change_stream"), map);
                id_field(args, 0, "task_id", "task_id", map)?;
                Ok(())
            },
        },
        Command {
            name: "abandon-change-stream",
            summary: "cancel a stream that never landed and reclaim its worktree",
            usage: "abandon-change-stream <task_id>",
            kind: Some("abandon_change_stream"),
            build: |args, map| {
                plain("kind", serde_json::json!("abandon_change_stream"), map);
                id_field(args, 0, "task_id", "task_id", map)?;
                Ok(())
            },
        },
        Command {
            name: "recover-task",
            summary: "put a failed or interrupted task back in the queue",
            usage: "recover-task <task_id>",
            kind: Some("recover_task"),
            build: |args, map| {
                plain("kind", serde_json::json!("recover_task"), map);
                id_field(args, 0, "task_id", "task_id", map)?;
                Ok(())
            },
        },
        Command {
            name: "interrupt-task",
            summary: "declare that a run is not coming back",
            usage: "interrupt-task <task_id> <reason>",
            kind: Some("interrupt_task"),
            build: |args, map| {
                plain("kind", serde_json::json!("interrupt_task"), map);
                id_field(args, 0, "task_id", "task_id", map)?;
                // The reason is a required argument, not a default: only the
                // caller knows whether a run was lost to a crash or abandoned,
                // and a record that said "interrupted" alone would answer
                // neither question.
                let reason = args.get(1).ok_or_else(|| {
                    Usage("missing <reason>: say why the run is not coming back".into())
                })?;
                if reason.trim().is_empty() {
                    return Err(Usage("<reason> must not be empty".into()));
                }
                plain("reason", serde_json::json!(reason), map);
                Ok(())
            },
        },
        Command {
            name: "cancel-task",
            summary: "close a task that is not coming back",
            usage: "cancel-task <task_id> <reason>",
            kind: Some("cancel_task"),
            build: |args, map| {
                plain("kind", serde_json::json!("cancel_task"), map);
                id_field(args, 0, "task_id", "task_id", map)?;
                // The reason is required for the same reason it is on
                // `interrupt-task`: a Task closed as "cancelled" records no
                // more than the fact, and the fact is the only account anyone
                // will have of why the work was withdrawn.
                let reason = args
                    .get(1)
                    .ok_or_else(|| Usage("missing <reason>: say why this task is closed".into()))?;
                if reason.trim().is_empty() {
                    return Err(Usage("<reason> must not be empty".into()));
                }
                plain("reason", serde_json::json!(reason), map);
                Ok(())
            },
        },
        Command {
            name: "complete-task",
            summary: "file the gates that passed and complete the task",
            usage: "complete-task <task_id> <reviewer> <gate>[,<gate>...]",
            kind: Some("complete_task"),
            build: |args, map| {
                plain("kind", serde_json::json!("complete_task"), map);
                id_field(args, 0, "task_id", "task_id", map)?;
                // The reviewer's own Role, never the Task's: the domain
                // refuses an independent review by the Role whose work it
                // reviews, and a client that could not name one would be
                // asking the Host to review itself.
                let reviewer = field(args, 1, "reviewer")?;
                map.insert("reviewer".to_owned(), serde_json::json!(reviewer));
                // Which gates passed is the caller's claim and the Host's to
                // bind. Naming them here rather than letting the Host fill the
                // contract's set is the difference between a claim and a
                // runtime-local success flag dressed as evidence.
                let gates: Vec<serde_json::Value> = list(&field(args, 2, "gate")?, "gate")?
                    .iter()
                    .map(|token| {
                        one_of(token, "gate", GATE_SPELLINGS).map(|gate| {
                            serde_json::to_value(gate).unwrap_or(serde_json::Value::Null)
                        })
                    })
                    .collect::<Result<Vec<serde_json::Value>, Usage>>()?;
                map.insert("gates".to_owned(), serde_json::Value::Array(gates));
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
        "       symbiote schema [envelope|policy] [--write DIR | --check DIR]\n       symbiote publish-runtime-inventory DOCUMENT --state-dir DIR\n       symbiote init DIR RESERVATION_BASE\n       symbiote add-credential CONFIG REFERENCE PROJECT ENVIRONMENT  (value on stdin)\n       symbiote help   (--command-id overrides the minted id; same id + same\n                        intent replays a lost response instead of re-executing)"
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
    println!(
        "  --                end flag parsing, so an argument that begins with -- is an argument"
    );
    println!();
    println!("flags are per command: daemon commands honor --state-dir, --command-id,");
    println!("--policy, --json and --yes; `schema` honors --write and --check;");
    println!("`publish-runtime-inventory` honors --state-dir; `init`, `add-credential` and");
    println!("`help` honor nothing.");
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
    let init_name = "init";
    let init_summary = "write a 0600 operator configuration this Host will accept";
    println!(
        "   {init_name:<width$} {init_summary}",
        width = USAGE_COLUMN
    );
    let credential_name = "add-credential";
    let credential_summary = "register one credential value in that configuration";
    println!(
        "   {credential_name:<width$} {credential_summary}",
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
