//! Unit tests for the `symbiote` binary, spanning every module it is built
//! from: the operation table and its risk classification ([`crate::commands`],
//! `symbiote_host::cli_schema`), the option/flag model ([`crate::args`]), the
//! authorization engine (`symbiote_host::cli_authorization`), the envelopes
//! ([`crate::output`]) and the idempotency key the request carries
//! ([`crate::session`]).
//!
//! These are unit tests, not integration tests: they call the same functions
//! `run_with` calls, with no daemon, no socket and no terminal. The end-to-end
//! properties — that a refusal puts no byte on the wire, that `raw` is not a
//! way around the gate — live in the `cli_authorization` and
//! `cli_schema_contract` integration suites instead.
use super::*;
use crate::args::*;
use crate::commands::*;
use crate::gate::*;
use crate::output::*;
use crate::session::*;
use std::collections::BTreeSet;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use symbiote_contract_read::region;
use symbiote_host::cli_authorization::*;
use symbiote_host::cli_schema::{CLI_SCHEMA, POLICY_SCHEMA, Risk, dangerous_kinds, risk_of_kind};

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
        vec![
            "abandon-change-stream",
            "cancel-task",
            "complete-task",
            "decide-elevation",
            "interrupt-task",
            "run-started-dispatch",
            "shutdown",
            "start-prepared-task",
        ]
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
    // A published record is a read: serving one starts nothing and widens
    // nothing, so the gate must not ask for authorization to ask for it.
    assert_eq!(risk_of_kind("get_compatibility_dossier"), Risk::ReadOnly);
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

/// One required argument as the usage spells it: the name to refuse it by, and
/// whether the spelling carries a revision after an `@`.
struct Placeholder {
    name: String,
    versioned: bool,
}

/// Every required argument a command's usage names, in order. The usage is the
/// command's own statement of its argument shape: `[root_id ...]` and the
/// bracket that closes it are not required arguments, and
/// `<contract_id>@<revision>` is one argument spelled as two spans.
/// Every argument a usage names, in the order it names them.
///
/// A usage argument is a `<name>`, optionally followed by more of the same
/// argument: `<id>@<revision>` is one versioned argument, and a record like
/// `<runtime>:<adapter>:<host_id>` or a list like `<root>,<root>` is one
/// argument whose remaining fields are the record itself rather than
/// arguments of their own. Reading only the first `<name>` of each token is
/// what lets one argument carry a shape — and what stops a record's inner
/// field from being counted as a position the command line must supply.
fn placeholders(usage: &str) -> Vec<Placeholder> {
    let mut names: Vec<Placeholder> = Vec::new();
    for token in usage.split_whitespace() {
        let Some(open) = token.find('<') else {
            continue;
        };
        let rest = &token[open + 1..];
        let Some(close) = rest.find('>') else {
            continue;
        };
        names.push(Placeholder {
            name: rest[..close].to_owned(),
            versioned: rest[close + 1..].starts_with("@<"),
        });
    }
    names
}

/// One synthetic argument per placeholder a command's usage names, so building
/// from it holds the usage and the builder together: a command whose builder
/// disagrees with the usage it prints cannot be built at all, and a placeholder
/// this reading does not know is a placeholder the command would refuse in an
/// operator's hands.
fn synthetic_arguments(usage: &str) -> Vec<String> {
    placeholders(usage)
        .into_iter()
        .map(|placeholder| {
            let value = match placeholder.name.as_str() {
                "after" | "limit" | "expected_revision" | "team_revision" => "1".to_owned(),
                "context_window_tokens" => "32768".to_owned(),
                "max_output_tokens" => "4096".to_owned(),
                "capability" => "tools".to_owned(),
                "class" => "maintenance".to_owned(),
                // The elevation decision's three closed-set arguments: a
                // permission the domain declares, the decision itself, and a
                // window inside the domain's bound — so a command that
                // refuses one of the three is held refusing a real line.
                "permission" => "execute_process".to_owned(),
                "approve|deny" => "approve".to_owned(),
                "window_ms" => "300000".to_owned(),
                "base_commit" => BASE.to_owned(),
                "expires_at_ms" => "4102444800000".to_owned(),
                // Closed-set arguments, one per command: the value the domain
                // declares, so a command that lists its choices is held to a
                // choice that is in the list.
                "authentication" => "api_credential".to_owned(),
                "function" => "general_execution".to_owned(),
                // `<kind>` is a billing kind here and nothing else; the
                // Objective class this same spelling carries on `create-work`
                // is spelled `<class>`, so the two cannot collide.
                "kind" => "metered_api".to_owned(),
                // A record argument the command splits on `:`: a shape this
                // reading has to know, or the command refuses its own usage.
                "runtime" => "NATIVE_SYMBIOTE:native-agent:host-1".to_owned(),
                "max_input_tokens" => "1:1".to_owned(),
                // A list argument the command splits on `,`. The three that
                // name a closed set of their own carry a member of it.
                "root" | "grant" => "read_root".to_owned(),
                "control" => "completion_authority".to_owned(),
                "tool" => "shell".to_owned(),
                // One team member: every field of `MEMBER_FIELDS`, in order.
                "member" => "id:general_execution:read_root:root:backend:other".to_owned(),
                // One gate is enough to build the request; the contract's
                // full set is the daemon's to demand, and a command that
                // refused a partial list here would be refusing the claim
                // before the caller could make it.
                "gate" => "tests".to_owned(),
                // A dependency edge is one `<kind>:<task_id>` token, and the
                // usage spells the whole argument rather than its parts so the
                // placeholder is the argument a caller actually types.
                "dependency" => "requires:other-task".to_owned(),
                // A policy reference is the versioned spelling, but the usage
                // does not carry the `@<revision>` itself, so the whole
                // argument is spelled here. `credential_ref` is a vault
                // reference and is not versioned.
                name if name.ends_with("_ref") && name != "credential_ref" => "id@1".to_owned(),
                _ => "id".to_owned(),
            };
            if placeholder.versioned {
                format!("{value}@1")
            } else {
                value
            }
        })
        .collect()
}

#[test]
fn every_typed_command_emits_the_kind_it_declares() {
    for command in commands() {
        // `raw` is the one command that reads an operator-supplied file, so it
        // is the one command with no argument shape to build from.
        if command.name == "raw" {
            continue;
        }
        let arguments = synthetic_arguments(command.usage);
        let mut operation = serde_json::Map::new();
        if let Err(error) = (command.build)(&arguments, &mut operation) {
            panic!(
                "{} must build with the arguments its usage names ({arguments:?}): {error}",
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

/// The operation a command builds for the arguments its usage names, by
/// invoking the table's own builder.
fn built(name: &str, arguments: &[&str]) -> serde_json::Value {
    let command = commands()
        .into_iter()
        .find(|command| command.name == name)
        .expect("the table defines the command");
    let arguments: Vec<String> = arguments.iter().map(|one| (*one).to_owned()).collect();
    let mut operation = serde_json::Map::new();
    (command.build)(&arguments, &mut operation).expect("the command builds these arguments");
    serde_json::Value::Object(operation)
}

/// The refusal a command gives an argument list, or the panic that the command
/// accepted an argument list it should have refused.
fn refused(name: &str, arguments: &[&str]) -> String {
    let command = commands()
        .into_iter()
        .find(|command| command.name == name)
        .expect("the table defines the command");
    let arguments: Vec<String> = arguments.iter().map(|one| (*one).to_owned()).collect();
    let mut operation = serde_json::Map::new();
    match (command.build)(&arguments, &mut operation) {
        Ok(()) => panic!("{name} accepted {arguments:?}"),
        Err(error) => error.to_string(),
    }
}

/// The three write commands build the operation their usage spells, and every
/// part of that operation the usage does not mention is derived rather than
/// asked for: a root names no path, a role is named for its own identity, and
/// a created task's stream names no commit because no work has run. Nothing
/// here is a value the operator did not type.
#[test]
fn the_write_commands_build_the_operations_their_usage_spells() {
    let project = built(
        "register-project",
        &[
            "symbiote",
            "Symbiote",
            "lead",
            "lead-contract@1",
            "root-main,root-tools",
            "engineer:engineer-contract:1",
        ],
    );
    assert_eq!(project["kind"], "register_project");
    assert_eq!(project["project"]["id"], "symbiote");
    assert_eq!(project["project"]["name"], "Symbiote");
    assert_eq!(project["project"]["lead"], "lead");
    // Every root named on the command line, in order, and no placement: an
    // observed path is a Host's to record, not this command's to invent.
    assert_eq!(
        project["project"]["roots"].as_array().map(|r| r.len()),
        Some(2)
    );
    assert_eq!(project["project"]["roots"][0]["id"], "root-main");
    assert_eq!(project["project"]["roots"][1]["id"], "root-tools");
    assert_eq!(project["project"]["roots"][0]["project_id"], "symbiote");
    assert_eq!(project["project"]["roots"][0]["revision"], 0);
    assert_eq!(
        project["project"]["roots"][0]["repository"],
        serde_json::Value::Null
    );
    assert_eq!(
        project["project"]["roots"][0]["host_paths"],
        serde_json::json!({})
    );
    // A Role that is the lead again, or one that repeats, is refused by name:
    // a Project holds one Role per identity, and the lead's is already
    // registered from <lead_role_id>.
    for (roles, names) in [
        (vec!["lead:lead-contract:1"], vec!["lead"]),
        (vec!["lead:other:1"], vec!["lead"]),
        (
            vec!["engineer:contract:1", "engineer:contract:2"],
            vec!["engineer", "one Role per identity"],
        ),
        (vec!["engineer:contract"], vec!["<role_id>", "contract_id"]),
        (vec!["engineer:contract:next"], vec!["revision", "next"]),
    ] {
        let mut arguments = vec![
            "symbiote",
            "Symbiote",
            "lead",
            "lead-contract@1",
            "root-main",
        ];
        arguments.extend(roles.iter().copied());
        let message = refused("register-project", &arguments);
        for name in names {
            assert!(
                message.contains(name),
                "roles {roles:?} were refused with {message:?}, which does not name {name:?}"
            );
        }
    }
    // Every Role is registered, the lead's first and then the rest, because a
    // Team cannot name a Role the Project does not hold — so a command that
    // registered the lead alone could never produce a Team the domain accepts.
    assert_eq!(
        project["project"]["roles"].as_array().map(|r| r.len()),
        Some(2)
    );
    assert_eq!(
        project["project"]["roles"][1],
        serde_json::json!({
            "id": "engineer",
            "project_id": "symbiote",
            "revision": 0,
            "name": "engineer",
            "operating_contract": {"id": "engineer-contract", "revision": 1},
        })
    );
    assert_eq!(
        project["project"]["roles"][0]["operating_contract"]["id"],
        "lead-contract"
    );
    assert_eq!(
        project["project"]["roles"][0]["operating_contract"]["revision"],
        1
    );

    let work = built(
        "create-work",
        &[
            "symbiote",
            "lead",
            "ship-it",
            "operational",
            "Ship it",
            "With a description",
        ],
    );
    assert_eq!(work["kind"], "create_work");
    assert_eq!(
        work["work"]["id"],
        serde_json::json!({"kind": "objective", "id": "ship-it"})
    );
    assert_eq!(work["work"]["objective_class"], "operational");
    assert_eq!(work["work"]["title"], "Ship it");
    assert_eq!(work["work"]["description"], "With a description");
    // The description is optional and absent means empty, never a placeholder.
    let bare = built(
        "create-work",
        &["symbiote", "lead", "ship-two", "maintenance", "Title"],
    );
    assert_eq!(bare["work"]["description"], "");

    let task = built(
        "create-task",
        &[
            "symbiote",
            "task-one",
            "root-main",
            "lead",
            "ship-it",
            "coding@2",
            BASE,
        ],
    );
    assert_eq!(task["kind"], "create_task");
    assert_eq!(task["task"]["id"], "task-one");
    assert_eq!(
        task["task"]["task_contract"],
        serde_json::json!({"id": "coding", "revision": 2})
    );
    // The origin names the objective the caller typed, in the work item it must
    // already be: creating a task does not create the work it originates from.
    assert_eq!(
        task["task"]["origin"],
        serde_json::json!({
            "kind": "objective",
            "work": {"project_id": "symbiote", "id": {"kind": "objective", "id": "ship-it"}},
        })
    );
    // The stream and its chat are named for the task. The worktree and the
    // branch are not the caller's to state: the Host re-derives both and
    // refuses a stream whose recorded names differ, so a branch argument here
    // could only ever have been recorded and then refused.
    assert_eq!(task["task"]["stream"]["id"], "stream-task-one");
    assert_eq!(task["task"]["stream"]["originating_chat"], "chat-task-one");
    // The base is the commit the caller typed — a Task starts from a real
    // commit or not at all — and the target is the same commit, because that
    // is where the Root is until work lands. It used to be the null Git object
    // id, on the reasoning that a task that has done no work has reached
    // nothing: which made a task this command creates dispatchable, runnable,
    // completable and then permanently un-integratable, because a Root can
    // never be "at" a commit that does not exist. A Task that cannot finish
    // should not be creatable.
    assert_eq!(task["task"]["stream"]["base"], BASE);
    assert_eq!(task["task"]["stream"]["target"], BASE);
}

/// A commit id the tests can type: forty hex characters that are not the null
/// object id, which no commit can carry.
const BASE: &str = "5117b5736e2e5cc2ff537a3852f8e1e37c6b0a12";

/// `create-task` records the worktree identity and branch the Host will insist
/// on, derived by the Host's own rule, and refuses a base commit that is not a
/// commit.
///
/// Before this, `create-task` named a worktree after the Task and accepted any
/// branch the operator liked. Both were accepted, written to the Task's stream,
/// and then refused at `run_started_dispatch` — `worktree provisioning refused:
/// Reservation` — because the Host re-derives the pair from its own policy seed
/// and a stream that disagrees is a stream it will not provision. The command
/// could name a branch and never get it, and the refusal arrived with nothing
/// to act on. So the derivation has one owner, `derived_worktree`, and this
/// holds the command to it: whatever the command writes, the Host's own
/// derivation is what wrote it.
#[test]
fn create_task_records_the_worktree_the_host_will_insist_on() {
    let project = symbiote_domain::ProjectId::new("symbiote").unwrap();
    let root = symbiote_domain::RootId::new("root-main").unwrap();
    let stream = symbiote_domain::ChangeStreamId::new("stream-task-one").unwrap();
    let derived = symbiote_host::runner::derived_worktree(&project, &root, &stream).unwrap();

    let task = built(
        "create-task",
        &[
            "symbiote",
            "task-one",
            "root-main",
            "lead",
            "ship-it",
            "coding@2",
            BASE,
        ],
    );
    assert_eq!(
        task["task"]["stream"]["worktree"],
        derived.worktree_id.to_string(),
        "the worktree the command records must be the worktree the Host derives"
    );
    assert_eq!(
        task["task"]["stream"]["branch"], derived.branch,
        "the branch the command records must be the branch the Host derives"
    );
    // The derivation is bound to the three identities, so a different stream
    // is a different worktree: a command that recorded one pair for every Task
    // would collide, and the Host would refuse the second one it saw.
    let other = symbiote_domain::ChangeStreamId::new("stream-task-two").unwrap();
    assert_ne!(
        derived.branch,
        symbiote_host::runner::derived_worktree(&project, &root, &other)
            .unwrap()
            .branch
    );

    // A base that is not a commit is refused by name, here, before the Task is
    // on record — the Host checks the recorded base against the repository's
    // real HEAD before it provisions, so a base the repository does not have is
    // a Task that would be dispatchable and then unprovisionable.
    for (base, why) in [
        ("task/one", "not a commit id at all"),
        ("5117b573", "abbreviated"),
        (
            "0000000000000000000000000000000000000000",
            "the null object id",
        ),
        (
            "g117b5736e2e5cc2ff537a3852f8e1e37c6b0a12",
            "not hexadecimal",
        ),
    ] {
        let message = refused(
            "create-task",
            &[
                "symbiote",
                "task-one",
                "root-main",
                "lead",
                "ship-it",
                "coding@2",
                base,
            ],
        );
        assert!(
            message.contains("<base_commit>") && message.contains(base),
            "a base that is {why} was refused with {message:?}, which names neither the argument \
             nor the value"
        );
    }
}

/// The six commands that register a Project's runtime, and the rule each one
/// is held to: every value the operation carries is a fact the operator typed
/// or a derivation the domain itself requires, and nothing is a name this
/// binary invented.
///
/// That rule is the whole point. `create-task` used to name a worktree after
/// the Task and accept any branch, and the Host refused both at
/// `run_started_dispatch` — a Task that could never be dispatched, refused
/// with nothing to act on. A first-class command that fills a field nobody
/// asked for reproduces that defect in a new place, so each derivation below
/// is one the domain would have written itself, and each refusal names an
/// argument whose value the caller can see and change.
#[test]
fn the_runtime_registration_commands_carry_only_facts_and_required_derivations() {
    // A Root's placement: the Host verifies the path is a real git work tree
    // and refuses a foreign Host, so the path and the Host are the operator's
    // facts and nothing is derived.
    let placement = built(
        "observe-root-placement",
        &["symbiote", "root-main", "host-1", "/srv/checkout", "3"],
    );
    assert_eq!(placement["kind"], "observe_root_placement");
    assert_eq!(placement["project_id"], "symbiote");
    assert_eq!(placement["root_id"], "root-main");
    assert_eq!(placement["host_id"], "host-1");
    assert_eq!(placement["path"], "/srv/checkout");
    assert_eq!(placement["expected_revision"], 3);

    // A provider connection carries no Project of its own — which Project may
    // name it is the attribution on the operation — and no secret: the
    // endpoint is a reference, and the credential is in the operator's broker.
    let connection = built(
        "register-provider-connection",
        &[
            "symbiote",
            "openai",
            "openai-responses",
            "https://api.openai.example/v1",
            "api_credential",
        ],
    );
    assert_eq!(connection["kind"], "replace_provider_connection");
    assert_eq!(connection["attribution"], "symbiote");
    assert_eq!(
        connection["connection"],
        serde_json::json!({
            "id": "openai",
            "adapter": "openai-responses",
            "endpoint_reference": "https://api.openai.example/v1",
            "authentication": "api_credential",
        })
    );
    // A closed set is the domain's, so an authentication it does not declare
    // is refused by name with the whole set beside it.
    let message = refused(
        "register-provider-connection",
        &[
            "symbiote",
            "openai",
            "openai-responses",
            "https://api.openai.example/v1",
            "oauth",
        ],
    );
    assert!(
        message.contains("api_credential") && message.contains("oauth"),
        "{message}"
    );

    // A model descriptor's schema version is the one this binary speaks: a
    // caller-supplied number could name a document this daemon does not
    // implement, and the refusal for that arrives from the provider turn.
    let model = built(
        "register-model-descriptor",
        &[
            "symbiote",
            "coding",
            "openai",
            "32768",
            "4096",
            "tools",
            "streaming",
            "reasoning:high",
        ],
    );
    assert_eq!(
        model["descriptor"]["schema_version"],
        symbiote_runtime_sdk::provider::PROVIDER_CONTRACT_VERSION
    );
    assert_eq!(model["descriptor"]["context_window_tokens"], 32768);
    assert_eq!(model["descriptor"]["max_output_tokens"], 4096);
    assert_eq!(model["descriptor"]["capabilities"]["tools"], true);
    assert_eq!(model["descriptor"]["capabilities"]["streaming"], true);
    assert_eq!(model["descriptor"]["capabilities"]["images"], false);
    // The reasoning efforts are a set, so the order they were typed in cannot
    // survive into the record — two orderings of one set are one descriptor.
    assert_eq!(
        model["descriptor"]["capabilities"]["reasoning_efforts"],
        serde_json::json!(["high"])
    );
    let reordered = built(
        "register-model-descriptor",
        &[
            "symbiote",
            "coding",
            "openai",
            "32768",
            "4096",
            "streaming",
            "reasoning:high",
            "tools",
        ],
    );
    assert_eq!(
        reordered["descriptor"], model["descriptor"],
        "the same capabilities typed in another order are one descriptor"
    );
    // A repeated effort is a set, so typing one twice is typing it once.
    let repeated = built(
        "register-model-descriptor",
        &[
            "symbiote",
            "coding",
            "openai",
            "32768",
            "4096",
            "tools",
            "streaming",
            "reasoning:high",
            "reasoning:high",
        ],
    );
    assert_eq!(
        repeated["descriptor"], model["descriptor"],
        "a repeated capability is one capability"
    );
    // A model that cannot hold its own output is a descriptor no request could
    // ever satisfy, and the check is the descriptor's own: both figures are on
    // this command line, so the incoherence is refused here rather than as a
    // `context_limit` from a provider turn long after the descriptor was on
    // record.
    //
    // The incoherence this does NOT catch is the one between two documents: a
    // binding whose `max_input_tokens`, plus this model's `max_output_tokens`,
    // exceeds this model's window. Neither command holds both figures, so that
    // pair is the provider's to refuse and the tree's own fixtures to get
    // right — the tree's example descriptor is one that does not.
    for (window, output, why) in [
        ("4096", "8192", "an output wider than the window"),
        ("32768", "0", "a model that emits nothing"),
        ("32768", "32769", "an output one token past the window"),
    ] {
        let message = refused(
            "register-model-descriptor",
            &["symbiote", "coding", "openai", window, output],
        );
        assert!(
            message.contains("<max_output_tokens>")
                && message.contains("<context_window_tokens>")
                && message.contains(window)
                && message.contains(output),
            "{why} was refused with {message:?}, which names neither argument nor value"
        );
    }
    // And a capability the model descriptor does not declare, named as such.
    let message = refused(
        "register-model-descriptor",
        &["symbiote", "coding", "openai", "32768", "4096", "telepathy"],
    );
    assert!(
        message.contains("<capability>") && message.contains("telepathy"),
        "{message}"
    );

    // A route: the work id is a tagged union and this is the one kind a typed
    // command files, so the kind is derived from `create-work`'s own shape
    // rather than asked for as a second spelling of the same fact.
    let route = built(
        "record-route",
        &["symbiote", "ship-it", "engineer", "backend"],
    );
    assert_eq!(route["kind"], "record_route");
    assert_eq!(
        route["request"],
        serde_json::json!({
            "project_id": "symbiote",
            "work_id": {"kind": "objective", "id": "ship-it"},
            "requested": "engineer",
            "domains": ["backend"],
        })
    );

    // A Team. Every member's access is the member's own — a Team exists so
    // two roles can reach different places — and the ceiling is the union of
    // what the members were granted, so no member can reach past it. The two
    // are read apart here because a command that copied the ceiling into
    // every member would be spelling one access and calling it a team.
    let team = built(
        "replace-team",
        &[
            "symbiote",
            "lead",
            "ctx",
            "tools",
            "skills",
            "bounded",
            "lead:lead_orchestrator:read_root:root:coordination:engineer",
            "engineer:general_execution:read_root,execute_process:root,second:coding:lead",
        ],
    );
    assert_eq!(team["kind"], "replace_team");
    let members = team["team"]["members"].as_array().unwrap();
    assert_eq!(members.len(), 2);
    assert_eq!(members[0]["role_id"], "lead");
    assert_eq!(members[0]["function"], "lead_orchestrator");
    assert_eq!(
        members[1]["access"]["grants"],
        serde_json::json!(["read_root", "execute_process"])
    );
    assert_eq!(members[1]["task_domains"], serde_json::json!(["coding"]));
    assert_eq!(
        members[1]["independent_reviewers"],
        serde_json::json!(["lead"])
    );
    assert_eq!(members[1]["context_policy_ref"], "ctx");
    // The ceiling holds every root and grant any member holds, and no others.
    let ceiling = &team["team"]["access_ceiling"];
    assert_eq!(ceiling["project_id"], "symbiote");
    assert_eq!(ceiling["roots"], serde_json::json!(["root", "second"]));
    assert_eq!(
        ceiling["grants"],
        serde_json::json!(["execute_process", "read_root"])
    );
    // A member that cannot be reviewed is refused by the domain, and a member
    // that reviews itself is refused twice over. Both are named here, where the
    // member argument is in the operator's hands.
    // A closed-set field names itself, because the operator is looking at a
    // member argument and needs to know WHICH of its six fields to change. A
    // shape or emptiness refusal names the field of the record instead.
    for (member, names) in [
        // A member with no reviewer field at all is refused as a record of
        // the wrong shape, and the shape it wanted names the reviewer.
        (
            "engineer:general_execution:read_root:root:coding",
            vec!["<member>", "reviewer"],
        ),
        // A member whose reviewer field is present and empty is refused as an
        // empty one, because an unreviewed role is what the domain refuses.
        (
            "engineer:general_execution:read_root:root:coding:",
            vec!["<member>", "reviewer"],
        ),
        (
            "engineer:general_execution:read_root:root:coding:engineer",
            vec!["<member>", "engineer"],
        ),
        (
            "engineer:general_execution:read_root::coding:lead",
            vec!["<member>", "root,[root...]"],
        ),
        (
            "engineer:general_execution:read_root:root::lead",
            vec!["<member>", "domain,[domain...]"],
        ),
        (
            "engineer:captain:read_root:root:coding:lead",
            vec!["<function>", "captain"],
        ),
        (
            "engineer:general_execution:teleport:root:coding:lead",
            vec!["<grant>", "teleport"],
        ),
    ] {
        let message = refused(
            "replace-team",
            &[
                "symbiote", "lead", "ctx", "tools", "skills", "bounded", member,
            ],
        );
        for name in names {
            assert!(
                message.contains(name),
                "a member {member:?} was refused with {message:?}, which does not name {name:?}"
            );
        }
    }
    // A member is a record, so a record of the wrong shape is refused naming
    // the shape rather than arriving at the domain with no access at all.
    let message = refused(
        "replace-team",
        &[
            "symbiote",
            "lead",
            "ctx",
            "tools",
            "skills",
            "bounded",
            "engineer:general_execution",
        ],
    );
    assert!(
        message.contains("<member>") && message.contains("reviewer"),
        "{message}"
    );

    // A binding: the candidate and the binding it staffs must agree on their
    // access, context, tools and skills, which the domain refuses. They are
    // separate fields of one document, so the only way they can be written
    // from one set of facts is to write them from one set of facts.
    let binding = built(
        "replace-binding",
        &[
            "symbiote",
            "engineer-binding",
            "engineer",
            "0",
            "native-worker@1",
            "workforce-protocol@1",
            "openai",
            "vault-ref",
            "entitlement",
            "coding",
            "NATIVE_SYMBIOTE:native-agent:host-1",
            "root",
            "read_root,execute_process",
            "completion_authority",
            "coding-context@1",
            "8000:2000",
            "shell",
            "environment@1",
            "worktree@1",
            "verification@1",
            "documentation@1",
            "artifacts@1",
            "mcp@1",
            "escalation@1",
        ],
    );
    assert_eq!(binding["kind"], "replace_binding");
    let configuration = &binding["configuration"];
    assert_eq!(
        configuration["schema_version"],
        symbiote_workforce::BINDING_VERSION
    );
    assert_eq!(
        configuration["binding"]["access"], configuration["primary"]["access"],
        "the binding and the candidate it staffs cannot disagree about access"
    );
    assert_eq!(
        configuration["binding"]["context"], configuration["primary"]["context"],
        "the binding and the candidate it staffs cannot disagree about context"
    );
    assert_eq!(
        configuration["binding"]["required_tools"], configuration["primary"]["tools"],
        "the binding and the candidate it staffs cannot disagree about tools"
    );
    // The seven policies are the operator's documents, named one argument
    // each: a reference this command invented would name a document that does
    // not exist.
    for (policy, reference) in [
        ("environment", "environment@1"),
        ("worktree", "worktree@1"),
        ("verification", "verification@1"),
        ("documentation", "documentation@1"),
        ("artifacts", "artifacts@1"),
        ("mcp", "mcp@1"),
        ("escalation", "escalation@1"),
    ] {
        assert_eq!(
            configuration["policies"][policy],
            serde_json::json!({"id": reference.split('@').next().unwrap(), "revision": 1}),
            "policies.{policy}"
        );
    }
    // Every control the binding requires names the enforcement it needs, and
    // the domain refuses a required control this map does not mention.
    assert_eq!(
        configuration["policies"]["minimum_enforcement"],
        serde_json::json!({"completion_authority": "host_enforced"})
    );
    // A lane of the wrong shape is a runtime, an adapter and a Host, and a
    // lane of the wrong spelling is refused naming all three.
    let message = refused(
        "replace-binding",
        &[
            "symbiote",
            "engineer-binding",
            "engineer",
            "0",
            "native-worker@1",
            "workforce-protocol@1",
            "openai",
            "vault-ref",
            "entitlement",
            "coding",
            "NATIVE_SYMBIOTE:native-agent",
            "root",
            "read_root",
            "completion_authority",
            "coding-context@1",
            "8000:2000",
            "shell",
            "environment@1",
            "worktree@1",
            "verification@1",
            "documentation@1",
            "artifacts@1",
            "mcp@1",
            "escalation@1",
        ],
    );
    assert!(
        message.contains("<runtime>")
            && message.contains("<adapter>")
            && message.contains("<host_id>"),
        "{message}"
    );
    // A context that cannot fit the binding's own token limit is refused here,
    // where both figures are on the command line, rather than at the provider
    // turn that discovers it.
    let message = refused(
        "replace-binding",
        &[
            "symbiote",
            "engineer-binding",
            "engineer",
            "0",
            "native-worker@1",
            "workforce-protocol@1",
            "openai",
            "vault-ref",
            "entitlement",
            "coding",
            "NATIVE_SYMBIOTE:native-agent:host-1",
            "root",
            "read_root",
            "completion_authority",
            "coding-context@1",
            "99000:2000",
            "shell",
            "environment@1",
            "worktree@1",
            "verification@1",
            "documentation@1",
            "artifacts@1",
            "mcp@1",
            "escalation@1",
        ],
    );
    assert!(
        message.contains("<max_input_tokens>")
            && message.contains("99000")
            && message.contains("100000"),
        "a context past the binding's own token limit was refused with {message:?}, which does \
         not name the argument, the figure it was given, or the limit it was measured against"
    );
}

/// Every argument position of every command, driven one position at a time.
///
/// A command line that stops before position `i` must be refused naming
/// position `i` — not by indexing what is not there, which is the panic a
/// builder writes when it uses an argument it never asked for, and not by
/// accepting a shorter line than its own usage names. And an argument that is
/// there and empty is refused by name for the same reason: there is no reading
/// under which an empty identity, name, title or branch is a value.
#[test]
fn every_argument_position_is_asked_for_before_it_is_used() {
    for command in commands() {
        // `raw` is the one command with no argument shape: it reads a file
        // path, which is the one argument it does take.
        if command.name == "raw" {
            continue;
        }
        let required = placeholders(command.usage);
        let arguments = synthetic_arguments(command.usage);
        assert_eq!(
            required.len(),
            arguments.len(),
            "{}: the usage names {} arguments and this reading builds {}",
            command.name,
            required.len(),
            arguments.len()
        );
        for (position, placeholder) in required.iter().enumerate() {
            let name = placeholder.name.as_str();
            let mut operation = serde_json::Map::new();
            let prefix = &arguments[..position];
            match (command.build)(prefix, &mut operation) {
                Err(error) => {
                    let message = error.to_string();
                    assert!(
                        message.contains(&format!("<{name}>")),
                        "{} accepted a line stopping before position {position} <{name}> but said \
                         {message:?}, which names no argument",
                        command.name
                    );
                }
                Ok(()) => panic!(
                    "{} accepted {prefix:?}, which is missing <{name}>",
                    command.name
                ),
            }
            // The same position, present and empty.
            let mut empty = arguments.clone();
            empty[position] = String::new();
            let mut operation = serde_json::Map::new();
            let built = (command.build)(&empty, &mut operation);
            assert!(
                built.is_err(),
                "{} accepted an empty <{name}> at position {position}: {built:?}",
                command.name
            );
            let message = built.unwrap_err().to_string();
            assert!(
                message.contains(&format!("<{name}>")),
                "{} refused an empty <{name}> with {message:?}, which names no argument",
                command.name
            );
        }
    }
}

/// Every way of mistyping an argument to the three write commands is refused
/// before anything is sent, and the refusal names the argument, the shape it
/// wanted and the value it got — the way the lock refusal names the lock it
/// could not take.
#[test]
fn the_write_commands_refuse_a_mistyped_argument_by_name() {
    for (name, arguments, named) in [
        // A missing argument names the argument that is missing. Every
        // position is driven, including the last one before the roots, because
        // a command that indexes its own arguments without asking first panics
        // on exactly the line an operator mistypes.
        (
            "register-project",
            vec!["symbiote", "Symbiote", "lead"],
            "missing <lead_contract_id>",
        ),
        (
            "register-project",
            vec!["symbiote", "Symbiote", "lead", "lead-contract@1"],
            "missing <root_id>",
        ),
        (
            "create-work",
            vec!["symbiote", "lead", "ship-it"],
            "missing <class>",
        ),
        (
            "create-task",
            vec!["symbiote", "task-one", "root-main"],
            "missing <role_id>",
        ),
        (
            "create-task",
            vec![
                "symbiote",
                "task-one",
                "root-main",
                "lead",
                "ship-it",
                "coding@1",
            ],
            "missing <base_commit>",
        ),
        // A contract without its revision, and a revision that is not a
        // number, each name the argument and the value they got.
        (
            "register-project",
            vec!["symbiote", "Symbiote", "lead", "lead-contract", "root"],
            r#"<lead_contract_id> must be <contract_id>@<revision>; got "lead-contract""#,
        ),
        (
            "register-project",
            vec!["symbiote", "Symbiote", "lead", "lead-contract@next", "root"],
            r#"the revision in <lead_contract_id> is not a whole number: "lead-contract@next""#,
        ),
        // A class the domain does not declare names the ones it does.
        (
            "create-work",
            vec!["symbiote", "lead", "ship-it", "urgent", "Title"],
            r#"<class> must be one of maintenance, operational, outcome; got "urgent""#,
        ),
        // The worker's completion report takes three arguments, and a line
        // missing one names it: the report is the evidence a verification
        // gate reads, so the shape it arrives in is checked before the wire.
        (
            "request-task-completion",
            vec!["symbiote", "task-one"],
            "missing <report>",
        ),
        // The two evidence reads name their ids positionally, and a surplus
        // argument is named rather than dropped — a reader scripting the
        // worktree observation or the run's context must not discover on the
        // wire that its line was truncated.
        ("observe-worktree", vec!["symbiote"], "missing <task_id>"),
        (
            "observe-worktree",
            vec!["symbiote", "project-one", "surplus"],
            r#"observe-worktree does not take the argument "surplus""#,
        ),
        (
            "get-task-context",
            vec!["symbiote", "project-one"],
            "missing <dispatch_id>",
        ),
        // Both elevation commands name a permission, so both check it: a
        // typo in the one field a worker's ask and an authority's answer share
        // is a question nobody can answer, and the daemon's answer to one is
        // a shape error that names neither the argument nor the spellings.
        (
            "request-elevation",
            vec!["task", "dispatch", "execute-proces", "the build needs it"],
            r#"<permission> must be one of read_root, mutate_stream, execute_process, network, use_credential; got "execute-proces""#,
        ),
        (
            "decide-elevation",
            vec![
                "task",
                "dispatch",
                "execute-proces",
                "approve",
                "yes",
                "300000",
            ],
            r#"<permission> must be one of read_root, mutate_stream, execute_process, network, use_credential; got "execute-proces""#,
        ),
        // A decision is one of two words, and a third is not a decision.
        (
            "decide-elevation",
            vec![
                "task",
                "dispatch",
                "execute_process",
                "maybe",
                "yes",
                "300000",
            ],
            r#"<decision> must be one of approve, deny; got "maybe""#,
        ),
        // A window is a number of milliseconds, and a denial's is zero: the
        // two refusals are separate because a mistyped window and a
        // contradictory one are different mistakes.
        (
            "decide-elevation",
            vec![
                "task",
                "dispatch",
                "execute_process",
                "approve",
                "yes",
                "five-minutes",
            ],
            r#"<window_ms> is a count of milliseconds; got "five-minutes""#,
        ),
        (
            "decide-elevation",
            vec!["task", "dispatch", "execute_process", "deny", "no", "60000"],
            r#"<window_ms> is 0 for a denial, which licenses nothing; a denial that carries a 60000ms window is not a decision"#,
        ),
        // A surplus argument is named rather than dropped: `description` is the
        // last argument this command takes, and a mistyped root would be one
        // too many just the same.
        (
            "create-work",
            vec![
                "symbiote",
                "lead",
                "ship-it",
                "maintenance",
                "Title",
                "Body",
                "surplus",
            ],
            r#"create-work does not take the argument "surplus""#,
        ),
        (
            "create-task",
            vec![
                "symbiote",
                "task-one",
                "root-main",
                "lead",
                "ship-it",
                "coding",
                BASE,
            ],
            r#"<contract_id> must be <contract_id>@<revision>; got "coding""#,
        ),
        (
            "create-task",
            vec![
                "symbiote",
                "task-one",
                "root-main",
                "lead",
                "ship-it",
                "coding@1",
                BASE,
                "surplus",
            ],
            r#"create-task does not take the argument "surplus""#,
        ),
    ] {
        let message = refused(name, &arguments);
        assert!(
            message.contains(named),
            "{name} refused {arguments:?} with {message:?}, which does not name {named:?}"
        );
    }
    // And each refusal names the offending value too, so a script reading the
    // message knows which argument was wrong rather than only which command.
    let message = refused(
        "create-work",
        &["symbiote", "lead", "ship-it", "urgent", "Title"],
    );
    assert!(message.contains("urgent"), "{message}");
}

/// The first word of every code span in `text`, skipping spans inside parentheses: a
/// parenthetical that names a value the list does not carry is not part of the list.
fn listed_names(text: &str) -> Vec<&str> {
    let mut names = Vec::new();
    let mut depth = 0i32;
    let mut rest = text;
    while let Some(start) = rest.find('`') {
        for character in rest[..start].chars() {
            match character {
                '(' => depth += 1,
                ')' => depth -= 1,
                _ => {}
            }
        }
        let after = &rest[start + 1..];
        let Some(end) = after.find('`') else { break };
        let token = &after[..end];
        if depth == 0 {
            if let Some(first) = token.split_whitespace().next() {
                names.push(first);
            }
        }
        rest = &after[end + 1..];
    }
    names
}

/// Every `--flag` spelling `text` names, as a set: a flag named twice is one flag.
fn flags_in(text: &str) -> BTreeSet<&str> {
    let mut names = BTreeSet::new();
    let mut rest = text;
    while let Some(at) = rest.find("--") {
        let after = &rest[at..];
        let length = after[2..]
            .find(|character: char| !(character.is_ascii_lowercase() || character == '-'))
            .map_or(after.len(), |offset| offset + 2);
        names.insert(&after[..length]);
        rest = &after[length..];
    }
    names
}

/// Every unsigned integer `text` states.
fn numbers(text: &str) -> Vec<i64> {
    let mut found = Vec::new();
    let mut digits = String::new();
    for character in text.chars() {
        if character.is_ascii_digit() {
            digits.push(character);
        } else if !digits.is_empty() {
            found.push(digits.parse().expect("a bounded run of digits"));
            digits.clear();
        }
    }
    if !digits.is_empty() {
        found.push(digits.parse().expect("a bounded run of digits"));
    }
    found
}

/// The command surface `docs/contracts/host.md` publishes is the table this binary runs:
/// every command the table defines is named in the document's own list, and the document
/// names no command the table does not define. The dangerous-kind list in the same document
/// is held the same way against the operation table's classification, with the parenthetical
/// that names the narrowing revocation read as prose (it is not part of the list).
///
/// What it does not read: the summaries and usages beside the names, which the table's own
/// builders are driven with, and the meanings the document writes for each kind.
#[test]
fn the_contract_document_names_the_commands_and_kinds_the_table_defines() {
    let document = include_str!("../../../../../docs/contracts/host.md");

    let stated = listed_names(region(
        document,
        "maps onto typed protocol",
        "for anything the typed",
    ));
    let declared: Vec<&str> = commands().iter().map(|command| command.name).collect();
    for name in &declared {
        assert!(
            stated.contains(name),
            "the contract document's command list never names {name}, which the table defines"
        );
    }
    for name in &stated {
        assert!(
            declared.contains(name),
            "the contract document's command list names {name}, and the table defines no such command"
        );
    }

    let stated = listed_names(region(
        document,
        "grant capability:",
        "A kind the table does not know",
    ));
    let dangerous = dangerous_kinds();
    for kind in &dangerous {
        assert!(
            stated.contains(kind),
            "the contract document's dangerous-kind list never names {kind}, which the table classifies dangerous"
        );
    }
    for kind in &stated {
        assert!(
            dangerous.contains(kind),
            "the contract document's dangerous-kind list names {kind}, and the table does not classify it dangerous"
        );
    }
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
fn flag_applicability_is_declared_per_command() {
    // One table decides which flags each command can honor, so adding a
    // local command means naming its flags there rather than sprinkling
    // `if` checks through the handler. `schema` honors `--write` and
    // `--check`, `help` honors nothing, and daemon commands honor the
    // daemon flags. `--help`/`-h` is honored everywhere.
    for arguments in [
        vec!["--write", "/tmp/s", "schema"],
        vec!["--check", "/tmp/s", "schema"],
        vec!["--write", "/tmp/s", "--check", "/tmp/s", "schema"],
    ] {
        let supplied = options(&arguments).supplied();
        assert!(
            supplied.unsuited_for(honored_flags("schema")).is_empty(),
            "{arguments:?}"
        );
    }
    for (arguments, named) in [
        (vec!["--json", "schema"], "--json"),
        (vec!["--yes", "help"], "--yes"),
        (vec!["--write", "/tmp/s", "health"], "--write"),
        (vec!["--check", "/tmp/s", "health"], "--check"),
    ] {
        let supplied = options(&arguments).supplied();
        let command = arguments.last().unwrap();
        assert_eq!(
            supplied.unsuited_for(honored_flags(command)),
            vec![named],
            "{arguments:?}"
        );
    }
    let daemon = options(&["--state-dir", "/s", "--json", "--yes", "shutdown"]);
    assert!(
        daemon
            .supplied()
            .unsuited_for(honored_flags("shutdown"))
            .is_empty()
    );
    // `--help` is universal: no command reports it unsuited.
    for command in ["schema", "help", "shutdown", "bogus"] {
        assert!(
            options(&["--help", command])
                .supplied()
                .unsuited_for(honored_flags(command))
                .is_empty(),
            "{command}"
        );
    }
}

/// The flag surface is the struct: every field of [`Flags`] is a flag `unsuited_for` names,
/// and every flag it names is a field. Both lists are read from the model's own source, so a
/// ninth flag fails this case by name rather than being supplied and never reported as
/// unsuited — which is what the check cannot notice on its own, since it decides applicability
/// from the same struct it would have to enumerate.
///
/// What it does not read: how a flag is parsed, or what each command honors — those are the
/// options model's and the arms' own claims, which
/// `flag_applicability_is_declared_per_command` drives with arguments.
#[test]
fn every_flag_the_model_carries_is_one_the_applicability_check_names() {
    let source = include_str!("args.rs");
    let declared = source
        .find("pub(crate) struct Flags {")
        .expect("the flag struct");
    let body = &source[declared..];
    let body = &body[body.find('{').expect("the struct's opening brace") + 1..];
    let body = &body[..body.find("\n}").expect("the struct's end")];
    let fields: Vec<&str> = body
        .lines()
        .filter_map(|line| line.trim_start().strip_prefix("pub(crate) "))
        .filter_map(|line| line.split(':').next())
        .map(str::trim)
        .collect();
    assert!(
        fields.len() > 4,
        "the flag struct carries the whole flag surface: {fields:?}"
    );

    let check = source
        .find("fn unsuited_for")
        .expect("the applicability check");
    let check = &source[check..];
    let check = &check[..check.find("\n    }").expect("the check's end")];
    let mut named: Vec<String> = Vec::new();
    for piece in check.split("\"--").skip(1) {
        if let Some(name) = piece.split('"').next() {
            named.push(format!("--{name}"));
        }
    }
    assert!(
        !named.is_empty(),
        "the applicability check names the flags it decides: {named:?}"
    );

    for field in &fields {
        let flag = format!("--{}", field.replace('_', "-"));
        assert!(
            named.iter().any(|one| one == &flag),
            "the flag struct carries {field}, and the applicability check never names {flag}"
        );
    }
    for flag in &named {
        let field = flag.trim_start_matches("--").replace('-', "_");
        assert!(
            fields.contains(&field.as_str()),
            "the applicability check names {flag}, and the flag struct carries no {field}"
        );
    }
}

/// The flag surface the help and the contract state is the applicability table's own rows:
/// each arm is read for the flags it grants, the flags every row shares are required to be
/// exactly the one both copies call universal, and every flag any row grants must be named
/// wherever the surface is laid out — the contract's sentence, the help paragraph and the
/// help's usage lines — so a flag added to the table cannot leave a copy behind.
///
/// What it does not read: the applicability check's behaviour, which
/// `flag_applicability_is_declared_per_command` drives with arguments.
#[test]
fn the_flag_surface_the_help_and_the_contract_state_is_the_tables_rows() {
    let table = region(
        include_str!("args.rs"),
        "pub(crate) fn honored_flags",
        "\n}\n",
    );
    let mut rows: Vec<BTreeSet<String>> = Vec::new();
    let mut rest = table;
    while let Some(at) = rest.find("Flags {") {
        let after = &rest[at + "Flags {".len()..];
        let Some(end) = after.find('}') else { break };
        rows.push(
            after[..end]
                .lines()
                .filter_map(|line| line.trim().strip_suffix(": true,"))
                .map(|field| format!("--{}", field.trim().replace('_', "-")))
                .collect(),
        );
        rest = &after[end..];
    }
    assert_eq!(rows.len(), 4, "the table declares one arm per command kind");

    let mut universal = rows[0].clone();
    for row in &rows[1..] {
        universal = universal.intersection(row).cloned().collect();
    }
    assert_eq!(
        universal,
        BTreeSet::from(["--help".to_string()]),
        "the arms share exactly `--help`, the one flag the copies call universal"
    );
    let mut surface = BTreeSet::new();
    for row in &rows {
        surface.extend(row.iter().cloned());
    }
    for flag in &universal {
        surface.remove(flag);
    }

    let contract = include_str!("../../../../../docs/contracts/cli.md");
    let help = include_str!("commands.rs");
    for (label, stated) in [
        (
            "docs/contracts/cli.md",
            flags_in(region(
                contract,
                "The daemon commands honor",
                "`--help`/`-h` is the one universal flag",
            )),
        ),
        (
            "the help paragraph",
            flags_in(region(
                help,
                "flags are per command:",
                "`--help`/`-h` is universal",
            )),
        ),
        (
            "the help usage lines",
            flags_in(region(
                help,
                "usage: symbiote [",
                "replays a lost response instead of re-executing",
            )),
        ),
    ] {
        for flag in &surface {
            assert!(
                stated.contains(flag.as_str()),
                "{label} never names {flag}, which an applicability arm grants"
            );
        }
        for flag in &stated {
            assert!(
                surface.contains(*flag),
                "{label} names {flag}, which no applicability arm grants"
            );
        }
    }
}

#[test]
fn help_is_a_universal_flag_that_connects_to_nothing() {
    // `--help`/`-h` are flags, not commands, and are answered from the
    // table alone: alone, or with any command, they print the table and
    // exit 0 without a state directory or a socket. This fails against
    // the earlier parser, which rejected `--help` as an unknown option and
    // treated `-h` as an unknown command.
    for arguments in [
        vec!["--help"],
        vec!["-h"],
        vec!["--help", "shutdown"],
        vec!["shutdown", "--help"],
        vec!["schema", "--help"],
        vec!["health", "-h"],
    ] {
        let code = run_with(arguments.iter().map(|s| s.to_string()).collect()).unwrap();
        assert_eq!(code, EXIT_OK, "{arguments:?}");
    }
    // With no command and no `--help`, the table is still a usage error.
    assert_eq!(run_with(Vec::new()).unwrap(), EXIT_USAGE);
    // A help request is validated against the `help` row, so a flag it
    // cannot honor is refused identically whether the command named honors
    // that flag or not: `--json --help health` behaves like `--json help`
    // rather than exiting 0 and dropping `--json`.
    for arguments in [
        vec!["--json", "help"],
        vec!["--json", "--help"],
        vec!["--json", "--help", "help"],
        vec!["--json", "--help", "health"],
        vec!["--help", "health", "--state-dir", "/d"],
        vec!["--write", "/d", "schema", "--help"],
    ] {
        assert_eq!(
            run_with(arguments.iter().map(|s| s.to_string()).collect()).unwrap(),
            EXIT_USAGE,
            "{arguments:?}"
        );
    }
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

/// The published fixtures under `docs/contracts/schemas/` are the external
/// contract. These assertions bind them to this binary, so a change here
/// cannot leave the published documents describing something else.
fn load_fixture(name: &str) -> serde_json::Value {
    let path = symbiote_host::cli_schema::fixture_directory().join(name);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    serde_json::from_str(&text)
        .unwrap_or_else(|error| panic!("{} is not JSON: {error}", path.display()))
}

#[test]
fn the_published_schemas_track_the_cli_contract() {
    let envelope = load_fixture("symbiote.cli.v1.schema.json");
    assert_eq!(envelope["properties"]["schema"]["const"], CLI_SCHEMA);
    // The declared keys are the whole shape: with `additionalProperties:
    // false` in the fixture, an envelope may carry these and no others.
    let declared: BTreeSet<&str> = envelope["properties"]
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(
        declared,
        BTreeSet::from(["command", "command_id", "error", "ok", "result", "schema"])
    );
    assert_eq!(
        envelope["required"],
        serde_json::json!(["schema", "command", "command_id", "ok"])
    );
    // The non-empty `command_id` the fixture requires is enforced by the
    // parser, not merely documented here: see the empty-override test below.
    assert_eq!(envelope["properties"]["command_id"]["minLength"], 1);
    assert_eq!(envelope["oneOf"].as_array().unwrap().len(), 2);
    assert_eq!(
        envelope["$defs"]["error"]["required"],
        serde_json::json!(["code", "message"])
    );
    // Both constructors write a subset of the declared keys, and exactly
    // one of result/error — the exclusivity the fixture encodes as oneOf
    // and a plain struct cannot.
    let success = success_envelope("health", "cli-1", &serde_json::json!({"kind": "hello"}));
    let failure = error_envelope("health", "cli-1", "unreachable", "down");
    for written in [&success, &failure] {
        let keys: BTreeSet<&str> = written
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        assert!(
            keys.is_subset(&declared),
            "{keys:?} must stay within {declared:?}"
        );
    }
    assert!(success.get("result").is_some() && success.get("error").is_none());
    assert!(failure.get("error").is_some() && failure.get("result").is_none());

    let policy = load_fixture("symbiote.cli-policy.v1.schema.json");
    assert_eq!(policy["properties"]["schema"]["const"], POLICY_SCHEMA);
    assert_eq!(
        policy["required"],
        serde_json::json!(["schema", "authorize"])
    );
    assert_eq!(policy["properties"]["authorize"]["minItems"], 1);
    // The policy schema may name exactly the kinds this CLI classifies as
    // dangerous: no more (it would publish a grant the CLI refuses) and no
    // fewer (a new dangerous kind would be un-nameable by any policy).
    let mut published: Vec<&str> = policy["properties"]["authorize"]["items"]["enum"]
        .as_array()
        .unwrap()
        .iter()
        .map(|kind| kind.as_str().unwrap())
        .collect();
    published.sort_unstable();
    let mut dangerous = dangerous_kinds();
    dangerous.sort_unstable();
    assert_eq!(published, dangerous);
    assert!(published.contains(&POLICY_EXAMPLE_KIND));
}

/// The third argument of an `error_envelope` call: the code the envelope carries. Read as an
/// argument rather than as the last string, because one call's message is a literal too.
fn envelope_code(call: &str) -> Option<String> {
    let mut depth = 0i32;
    let mut quoted = false;
    let mut arguments: Vec<String> = vec![String::new()];
    for character in call["error_envelope(".len()..].chars() {
        match character {
            '"' => {
                quoted = !quoted;
                arguments.last_mut().expect("an argument").push(character);
            }
            '(' | '[' if !quoted => {
                depth += 1;
                arguments.last_mut().expect("an argument").push(character);
            }
            ')' | ']' if !quoted => {
                depth -= 1;
                if depth < 0 {
                    break;
                }
                arguments.last_mut().expect("an argument").push(character);
            }
            ',' if !quoted && depth == 0 => arguments.push(String::new()),
            _ => arguments.last_mut().expect("an argument").push(character),
        }
    }
    let code = arguments.get(2)?.trim();
    Some(code.strip_prefix('"')?.strip_suffix('"')?.to_string())
}

/// The codes this binary uses are the ones the documents state: `args.rs`'s exit constants
/// are the exit surface, and the codes the gate and the session hand the envelope are read
/// off their own `error_envelope` calls. Each copy — the contract's exit table and error
/// bullets, `host.md`'s sentence, the entry point's own doc comment, the help text, the
/// published fixture's description and `output.rs` — must name exactly them, so a fifth
/// exit, a renumbered code or a fourth CLI-origin error code fails here by name rather than
/// in prose nobody reads.
///
/// What it does not read: the meanings written beside the codes and the conditions that
/// choose them, which the suites drive with real invocations and policies.
#[test]
fn the_codes_the_documents_state_are_the_ones_this_binary_uses() {
    let mut exits: Vec<i64> = include_str!("args.rs")
        .lines()
        .filter_map(|line| line.strip_prefix("pub(crate) const EXIT_"))
        .filter_map(|rest| rest.split(": i32 = ").nth(1))
        .filter_map(|value| value.trim_end_matches(';').trim().parse().ok())
        .collect();
    exits.sort_unstable();
    assert_eq!(
        exits.len(),
        4,
        "the exit surface is the constants: {exits:?}"
    );
    let exits: BTreeSet<i64> = exits.into_iter().collect();

    let contract = include_str!("../../../../../docs/contracts/cli.md");
    let host = include_str!("../../../../../docs/contracts/host.md");
    let entry = include_str!("main.rs");
    let help = include_str!("commands.rs");
    for (label, stated) in [
        (
            "docs/contracts/cli.md's exit table",
            numbers(region(
                contract,
                "## Exit codes",
                "## The `--json` envelope",
            )),
        ),
        (
            "docs/contracts/host.md",
            numbers(region(host, "Exit codes", "scripts branch")),
        ),
        (
            "the entry point's doc comment",
            numbers(region(entry, "Exit codes:", "distinct exits")),
        ),
        (
            "the help text",
            numbers(region(
                help,
                "Exit codes:",
                "authorization required (no request was sent)",
            )),
        ),
    ] {
        let stated: BTreeSet<i64> = stated.into_iter().collect();
        assert_eq!(
            stated, exits,
            "{label} states the codes this binary exits with"
        );
    }

    let mut errors: Vec<String> = Vec::new();
    for source in [include_str!("gate.rs"), include_str!("session.rs")] {
        let mut rest = source;
        while let Some(at) = rest.find("error_envelope(") {
            let call = &rest[at..];
            let mut depth = 0i32;
            let mut end = call.len();
            for (index, character) in call.char_indices() {
                match character {
                    '(' => depth += 1,
                    ')' => {
                        depth -= 1;
                        if depth == 0 {
                            end = index;
                            break;
                        }
                    }
                    _ => {}
                }
            }
            if let Some(code) = envelope_code(&call[..end]) {
                errors.push(code);
            }
            rest = &call[end..];
        }
    }
    errors.sort();
    errors.dedup();
    assert_eq!(
        errors.len(),
        4,
        "the envelope's CLI-origin codes: {errors:?}"
    );

    let stated = listed_names(region(contract, "one of the CLI's", "```json"));
    let stated: Vec<&str> = stated
        .into_iter()
        .filter(|name| !name.starts_with("--"))
        .collect();
    for code in &errors {
        assert!(
            stated.iter().any(|name| name == code),
            "docs/contracts/cli.md's error bullets never name {code}, which this binary hands the envelope"
        );
    }
    for name in &stated {
        assert!(
            errors.iter().any(|code| code == name),
            "docs/contracts/cli.md's error bullets name {name}, and this binary never hands the envelope that code"
        );
    }

    let stated = listed_names(region(
        include_str!("output.rs"),
        "one of the CLI's own:",
        "pub(crate) fn error_envelope",
    ));
    for code in &errors {
        assert!(
            stated.contains(&code.as_str()),
            "output.rs never names {code}, which a caller hands the envelope"
        );
    }
    for name in &stated {
        assert!(
            errors.iter().any(|code| code == name),
            "output.rs names {name}, and no caller hands the envelope that code"
        );
    }

    let envelope = load_fixture("symbiote.cli.v1.schema.json");
    let description = envelope["$defs"]["error"]["properties"]["code"]["description"]
        .as_str()
        .expect("the fixture describes the error code");
    let stated = listed_names(description);
    for code in &errors {
        assert!(
            stated.contains(&code.as_str()),
            "the published envelope fixture never names {code}, which this binary hands the envelope"
        );
    }
    for name in &stated {
        assert!(
            errors.iter().any(|code| code == name),
            "the published envelope fixture names {name}, and this binary never hands the envelope that code"
        );
    }
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

#[test]
fn an_empty_command_id_override_is_refused() {
    // The published envelope schema requires a non-empty `command_id`, so
    // the parser must not carry an empty override through to an envelope:
    // that would make the CLI violate its own contract, and an empty
    // idempotency key makes every such invocation a replay of the last.
    let empty = parse_options(&[
        "--command-id".to_string(),
        String::new(),
        "health".to_string(),
    ]);
    assert!(matches!(empty, Err(Usage(ref message)) if message.contains("command-id")));
    // A non-empty override is still honored, and the flag still works after
    // the command, so the refusal is only about the empty value.
    let after = options(&["health", "--command-id", "retry-1"]);
    assert_eq!(after.command_id_override.as_deref(), Some("retry-1"));
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

/// A private Host state directory and a discovery document naming whatever
/// Host identity that directory resolves to. The test removes both.
fn state_and_document(host: &str) -> (PathBuf, PathBuf) {
    static NEXT_STATE: AtomicU64 = AtomicU64::new(0);
    let sequence = NEXT_STATE.fetch_add(1, Ordering::Relaxed);
    let directory = std::env::temp_dir().join(format!(
        "symbiote-cli-publish-unit-{}-{sequence}",
        std::process::id(),
    ));
    symbiote_host::transport::private_directory(&directory).unwrap();
    let document = std::env::temp_dir().join(format!(
        "symbiote-cli-publish-document-{}-{sequence}",
        std::process::id(),
    ));
    std::fs::write(&document, discovery_document(host)).unwrap();
    (directory, document)
}

/// One valid inventory record naming `host`, in the wire shape the discovery
/// contract parses. A discovery run produces this; here it stands in for one,
/// because what this command is measured on is what it does with the document,
/// not how the document was obtained.
fn discovery_document(host: &str) -> String {
    format!(
        r#"{{"schema_version":1,"records":[{{"schema_version":1,"intent":{{"profile_id":"profile-1","installation_id":"installation-1","host_id":"{host}","runtime_kind":"EXTERNAL_HARNESS","adapter_id":"adapter-1","instance_name":"Codex","config_identity":"codex-default"}},"observation":{{"config_identity":"codex-default","profile_id":"profile-1","installation_id":"installation-1","host_id":"{host}","runtime_kind":"EXTERNAL_HARNESS","adapter_id":"adapter-1","version":{{"status":"known","value":"0.118.0"}},"interface":{{"status":"known","value":{{"name":"app_server","version":"0.118.0"}}}},"facts":{{"health":{{"status":"known","value":"reachable"}},"authentication":{{"status":"known","value":{{"mode":"none","state":"required","account_ref":null}}}},"models":{{"status":"unknown"}},"methods":{{"initialize":"available"}},"isolation":{{"status":"unknown"}}}},"observed_at":1000,"expires_at":2000,"provenance":{{"probe_id":"probe-1","source":"sandboxed_probe","adapter_revision":"0.118.0"}}}}}}]}}"#
    )
}

/// The local publish command is the only way a discovery document reaches a
/// Host, so it is driven end to end here: it installs a document the daemon
/// will serve, it names a refusal instead of installing a bad one, it never
/// opens a socket, and it accepts only the flags the applicability table
/// grants it.
#[test]
fn the_publish_command_installs_what_the_host_serves_and_names_every_refusal() {
    let (directory, _) = state_and_document("unused");
    // The identity is the one the daemon would load, so the records the
    // command accepts are the records the read will serve — and the Host mints
    // that identity on first start, so the document has to be written after it
    // resolves rather than guessed before.
    let identity = symbiote_host::host_identity(&directory).unwrap();
    let document = directory.join("discovery.json");
    std::fs::write(&document, discovery_document(identity.as_str())).unwrap();
    let published = symbiote_host::runtime_inventory::Published::new(&directory, identity.clone());
    assert!(published.read().is_err());

    // A document naming another Host is refused, and nothing is installed.
    let run = |arguments: &[&str]| {
        run_with(arguments.iter().map(|value| value.to_string()).collect()).unwrap()
    };
    let foreign = state_and_document("host_somewhere_else");
    assert_eq!(
        run(&[
            "publish-runtime-inventory",
            foreign.1.to_str().unwrap(),
            "--state-dir",
            directory.to_str().unwrap()
        ]),
        EXIT_USAGE
    );
    assert!(published.read().is_err());

    // The document this Host's own identity names is installed, and the file
    // the read then answers from is the private file the read requires.
    assert_eq!(
        run(&[
            "publish-runtime-inventory",
            document.to_str().unwrap(),
            "--state-dir",
            directory.to_str().unwrap()
        ]),
        EXIT_OK
    );
    assert_eq!(published.read().unwrap().records().len(), 1);

    // Usage refusals: no document, no state directory, a document that is not
    // there, and a flag this command does not honor.
    assert_eq!(run(&["publish-runtime-inventory"]), EXIT_USAGE);
    assert_eq!(
        run(&["publish-runtime-inventory", document.to_str().unwrap()]),
        EXIT_USAGE
    );
    assert_eq!(
        run(&[
            "publish-runtime-inventory",
            directory.join("absent.json").to_str().unwrap(),
            "--state-dir",
            directory.to_str().unwrap()
        ]),
        EXIT_USAGE
    );
    assert_eq!(
        run(&[
            "publish-runtime-inventory",
            document.to_str().unwrap(),
            "--state-dir",
            directory.to_str().unwrap(),
            "--json"
        ]),
        EXIT_USAGE
    );
    // A directory that is not a Host state directory at all.
    let shared = std::env::temp_dir();
    assert_eq!(
        run(&[
            "publish-runtime-inventory",
            document.to_str().unwrap(),
            "--state-dir",
            shared.to_str().unwrap()
        ]),
        EXIT_USAGE
    );
    for path in [&directory, &foreign.0] {
        std::fs::remove_dir_all(path).unwrap();
    }
    remove(&foreign.1);
}

/// The document `docs/contracts/cli.md` publishes for the publish command
/// states the bound and the file mode the module enforces, so a moved bound
/// cannot leave a stale figure behind.
#[test]
fn the_publish_contract_states_the_bound_the_module_enforces() {
    let document = include_str!("../../../../../docs/contracts/cli.md");
    let section = region(
        document,
        "## Publishing a runtime inventory",
        "\n## Exit codes",
    );
    let stated = numbers(region(section, "a bound of ", " KiB"));
    assert_eq!(
        stated,
        vec![(symbiote_host::runtime_inventory::MAX_PUBLISHED_BYTES / 1024) as i64],
        "the publish section's bound is the one the module enforces"
    );
    assert!(
        section.contains("0600"),
        "the section states the installed mode"
    );
    assert!(
        section.contains("atomically"),
        "the section states the install"
    );
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

/// The invariant behind [`super::commands::consequence`], stated as one: an
/// operator must not learn what a terminal step costs by attempting the next
/// one. Every step that ends a task, a run or a stream says so where the
/// operator is standing, and every note is keyed to a command that exists.
#[test]
fn every_terminal_step_says_what_it_costs_and_where_it_will_appear() {
    for command in [
        // The step that creates the cost...
        "create-task",
        // ...and the ones that charge it.
        "abandon-change-stream",
        "release-change-stream",
        "complete-task",
        "interrupt-task",
        "recover-task",
        "cancel-task",
    ] {
        let note = super::commands::consequence(command).unwrap_or_else(|| {
            panic!("{command} is a step an operator cannot undo, and it says nothing")
        });
        assert!(
            note.ends_with('.'),
            "{command}: a sentence, not a fragment: {note}"
        );
        assert!(
            commands().iter().any(|entry| entry.name == command),
            "{command} has a consequence note and no command: nobody will ever read it"
        );
    }
    // A read has no consequence to report, and saying so for every command
    // would be noise. This is the other half: the table stays small.
    assert!(super::commands::consequence("get-task").is_none());
    assert!(super::commands::consequence("health").is_none());
}

/// A dependency edge is a gate, and only two of the eight relations are
/// enforced at a boundary. The typed command narrows to those two and refuses
/// the rest by name, because the alternative is a silent no-op: an edge
/// recorded where nothing reads it, which reads back to an operator as a gate
/// that was accepted. The first draft of this check mapped over a lookup's
/// error, which asks "is this *none* of them?" — and the five provenance kinds
/// are in the list, so `reviews` was stored. Only the positive form works.
#[test]
fn a_typed_dependency_command_refuses_a_relation_no_boundary_enforces() {
    let mut operation = serde_json::Map::new();
    let table = commands();
    let set = table
        .iter()
        .find(|command| command.name == "set-task-dependencies")
        .expect("the command exists");
    for kind in ["requires", "blocks"] {
        let mut operation = serde_json::Map::new();
        (set.build)(
            &[
                "demo".to_owned(),
                "dependent".to_owned(),
                format!("{kind}:prereq"),
            ],
            &mut operation,
        )
        .unwrap_or_else(|error| panic!("{kind} is enforced and must be accepted: {error}"));
        assert_eq!(operation["kind"], "set_task_dependencies");
    }
    for kind in [
        "reviews",
        "verifies",
        "supersedes",
        "conflicts_with",
        "follow_up_to",
        "consumes_contract_from",
    ] {
        let error = (set.build)(
            &[
                "demo".to_owned(),
                "dependent".to_owned(),
                format!("{kind}:prereq"),
            ],
            &mut operation,
        )
        .expect_err("a relation no boundary reads must not be storable by a typed command");
        let said = error.to_string();
        assert!(
            said.contains("provenance no boundary enforces") && said.contains("requires"),
            "the refusal names the relation and the ones that are real: {said}"
        );
    }
    // And the shape of a token is checked before it is looked up, so a
    // malformed line says which argument is wrong rather than which kind.
    let error = (set.build)(
        &[
            "demo".to_owned(),
            "dependent".to_owned(),
            "prereq".to_owned(),
        ],
        &mut operation,
    )
    .expect_err("a bare task id is not an edge");
    assert!(
        error
            .to_string()
            .contains("<dependency> is <kind>:<task_id>"),
        "{error}"
    );
}

#[test]
fn every_dangerous_kind_in_the_table_can_be_named_by_a_policy() {
    let dangerous = dangerous_kinds();
    // Pins the set the help text and the policy schema describe.
    assert_eq!(
        dangerous,
        // The table's own order, which is what the help text and the policy
        // schema describe. `abandon_change_stream` is the one operation that
        // destroys work: a policy that has to name it is a policy that has to
        // be told about it, which is the point of the list.
        vec![
            "abandon_change_stream",
            "interrupt_task",
            "cancel_task",
            "start_prepared_task",
            "run_started_dispatch",
            "shutdown",
            "decide_elevation",
            "record_resource_consent",
            "complete_task",
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

/// `init` writes a file the daemon will actually load, at the mode the daemon
/// will actually accept, and it never replaces one that is already there.
///
/// The proof that matters is the first assertion: a generated operator config
/// that the daemon refuses is worse than no command at all, because the
/// operator has a file that looks right and a daemon that will not start. So
/// the file is loaded here by the daemon's own loader — the same function
/// `symbioted` calls, with the same `deny_unknown_fields` and the same mode
/// check — and the loaded configuration is asserted to be the one written.
#[test]
fn the_init_command_writes_a_configuration_the_daemon_loads() {
    static NEXT_INIT: AtomicU64 = AtomicU64::new(0);
    let sequence = NEXT_INIT.fetch_add(1, Ordering::Relaxed);
    let directory = std::env::temp_dir().join(format!(
        "symbiote-cli-init-unit-{}-{sequence}",
        std::process::id()
    ));
    symbiote_host::transport::private_directory(&directory).unwrap();
    let base = directory.join("worktrees");
    let run = |arguments: &[&str]| {
        run_with(arguments.iter().map(|value| value.to_string()).collect()).unwrap()
    };

    // Written, and loadable by the loader the daemon itself uses.
    assert_eq!(
        run(&["init", directory.to_str().unwrap(), base.to_str().unwrap()]),
        EXIT_OK
    );
    let path = directory.join("operator.json");
    let config = symbiote_host::operator::load(&path).unwrap();
    assert_eq!(config.reservation_base, base);
    assert_eq!(
        config
            .native_fixture
            .as_ref()
            .map(|fixture| fixture.echo_text.as_str()),
        Some("The agent ran end to end inside its own worktree.")
    );
    // The fixture carries no tool, because none was chosen; and no credential,
    // executor, harness or dossier, because those are the operator's to make.
    assert!(config.native_fixture.as_ref().unwrap().tool.is_none());
    assert!(config.credential_broker.is_empty());
    assert!(config.external_fixture.is_none());
    assert!(config.external_harness.is_none());
    assert!(config.shell_executor.is_none());
    assert!(config.compatibility_dossiers.is_empty());

    // The mode the daemon demands, checked on the file rather than on the
    // call: 0600 from the moment it exists, so a concurrent reader — a daemon
    // starting at the same moment — never sees a wider one.
    let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600, "{} is not private", path.display());
    // And no temporary file is left behind for a later reader to trip over.
    let leftovers: Vec<PathBuf> = std::fs::read_dir(&directory)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|entry| entry.extension().is_some_and(|ext| ext == "tmp"))
        .collect();
    assert_eq!(leftovers, Vec::<PathBuf>::new());

    // An existing configuration is the operator's, and `init` never replaces
    // one: a second install leaves the first byte-for-byte.
    let before = std::fs::read(&path).unwrap();
    assert_eq!(
        run(&["init", directory.to_str().unwrap(), base.to_str().unwrap()]),
        EXIT_USAGE
    );
    assert_eq!(std::fs::read(&path).unwrap(), before);

    // A reservation base that is not absolute is refused, and nothing is
    // written: a worktree under the state directory would be inside the store,
    // and a relative one would be wherever a script happened to be run from.
    let relative = directory.join("relative");
    symbiote_host::transport::private_directory(&relative).unwrap();
    assert_eq!(
        run(&["init", relative.to_str().unwrap(), "worktrees"]),
        EXIT_USAGE
    );
    assert!(!relative.join("operator.json").exists());

    // The usage refusals, and the flag this command does not honor.
    for arguments in [
        vec!["init"],
        vec!["init", directory.to_str().unwrap()],
        vec![
            "init",
            directory.to_str().unwrap(),
            base.to_str().unwrap(),
            "surplus",
        ],
        vec!["init", directory.to_str().unwrap(), ""],
        vec![
            "--json",
            "init",
            directory.to_str().unwrap(),
            base.to_str().unwrap(),
        ],
        vec![
            "init",
            directory.to_str().unwrap(),
            base.to_str().unwrap(),
            "--yes",
        ],
        vec![
            "init",
            directory.to_str().unwrap(),
            base.to_str().unwrap(),
            "--state-dir",
            directory.to_str().unwrap(),
        ],
    ] {
        assert_eq!(run(&arguments), EXIT_USAGE, "init accepted {arguments:?}");
    }
    std::fs::remove_dir_all(&directory).unwrap();
}

/// The step between a Host that records dispatches and one that executes
/// them. Every runtime profile names a credential, and a native run refuses
/// without one by name, so this is not a convenience: it is why `init`'s
/// configuration can be written and then not run.
///
/// The file is read back with the daemon's own loader, and the mode is read
/// off the file, because a second parser and a second chmod would each be
/// able to agree with a broken writer.
#[test]
fn a_credential_value_is_registered_only_where_the_daemon_can_hold_it() {
    static NEXT_CREDENTIAL: AtomicU64 = AtomicU64::new(0);
    let sequence = NEXT_CREDENTIAL.fetch_add(1, Ordering::Relaxed);
    let directory = std::env::temp_dir().join(format!(
        "symbiote-cli-credential-unit-{}-{sequence}",
        std::process::id()
    ));
    symbiote_host::transport::private_directory(&directory).unwrap();
    let base = directory.join("worktrees");
    let path = directory.join("operator.json");
    let run = |arguments: &[&str], value: &str| {
        let mut input = value.as_bytes();
        run_with_input(
            arguments.iter().map(|value| value.to_string()).collect(),
            &mut input,
        )
        .unwrap()
    };
    assert_eq!(
        run(
            &["init", directory.to_str().unwrap(), base.to_str().unwrap()],
            ""
        ),
        EXIT_OK
    );

    // The value arrives on standard input and nowhere else: not in the
    // arguments, not in the output, and not in a file the command left.
    let out = run(
        &[
            "add-credential",
            path.to_str().unwrap(),
            "native-vault-ref",
            "demo",
            "OPENAI_API_KEY",
        ],
        "sk-the-value\n",
    );
    assert_eq!(out, EXIT_OK);
    let config = symbiote_host::operator::load(&path).unwrap();
    assert_eq!(config.credential_broker.len(), 1);
    let registration = &config.credential_broker[0];
    assert_eq!(registration.reference, "native-vault-ref");
    assert_eq!(registration.project, "demo");
    assert_eq!(registration.environment, "OPENAI_API_KEY");
    // One trailing newline is a line, not part of the secret: a pipe and a
    // here document both deliver the value the operator meant.
    assert_eq!(
        registration.value, "sk-the-value",
        "the value is kept byte for byte apart from the trailing newline"
    );
    let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600, "a rewritten configuration stays private");
    let leftovers: Vec<PathBuf> = std::fs::read_dir(&directory)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|entry| entry.extension().is_some_and(|ext| ext == "tmp"))
        .collect();
    assert_eq!(leftovers, Vec::<PathBuf>::new());

    // A registration is never replaced: the broker refuses re-registration by
    // design, so a command that overwrote one would write a file the daemon
    // then refuses to load — after reporting success. Rotation is a NEW
    // reference, and that one is allowed.
    let before = std::fs::read(&path).unwrap();
    assert_eq!(
        run(
            &[
                "add-credential",
                path.to_str().unwrap(),
                "native-vault-ref",
                "demo",
                "OPENAI_API_KEY",
            ],
            "sk-a-second-value\n",
        ),
        EXIT_USAGE
    );
    assert_eq!(std::fs::read(&path).unwrap(), before);
    assert_eq!(
        run(
            &[
                "add-credential",
                path.to_str().unwrap(),
                "native-vault-ref-2",
                "demo",
                "OPENAI_API_KEY",
            ],
            "sk-rotated\n",
        ),
        EXIT_OK
    );
    assert_eq!(
        symbiote_host::operator::load(&path)
            .unwrap()
            .credential_broker
            .len(),
        2,
        "the rotation is a second registration, not a replaced one"
    );

    // What the broker itself would refuse, refused here instead — a daemon
    // that will not start is a worse place to learn that an identity is
    // unusable.
    for (arguments, value) in [
        (
            vec![
                "add-credential",
                path.to_str().unwrap(),
                "",
                "demo",
                "OPENAI_API_KEY",
            ],
            "sk-x\n",
        ),
        (
            vec![
                "add-credential",
                path.to_str().unwrap(),
                "not a reference",
                "demo",
                "OPENAI_API_KEY",
            ],
            "sk-x\n",
        ),
        (
            vec![
                "add-credential",
                path.to_str().unwrap(),
                "another-ref",
                "",
                "OPENAI_API_KEY",
            ],
            "sk-x\n",
        ),
        (
            vec![
                "add-credential",
                path.to_str().unwrap(),
                "another-ref",
                "demo",
                "",
            ],
            "sk-x\n",
        ),
        // An empty value registers nothing, and says so rather than writing
        // a secret-shaped hole.
        (
            vec![
                "add-credential",
                path.to_str().unwrap(),
                "another-ref",
                "demo",
                "OPENAI_API_KEY",
            ],
            "",
        ),
        // A configuration that is not there, and one that is not an object.
        (
            vec![
                "add-credential",
                directory.join("absent.json").to_str().unwrap(),
                "another-ref",
                "demo",
                "OPENAI_API_KEY",
            ],
            "sk-x\n",
        ),
    ] {
        assert_eq!(run(&arguments, value), EXIT_USAGE, "accepted {arguments:?}");
    }
    let unchanged = symbiote_host::operator::load(&path).unwrap();
    assert_eq!(
        unchanged.credential_broker.len(),
        2,
        "no refused registration reached the file"
    );

    // The usage refusals, and the flag this command does not honor.
    for (arguments, value) in [
        (vec!["add-credential"], "sk-x\n"),
        (vec!["add-credential", path.to_str().unwrap()], "sk-x\n"),
        (
            vec![
                "add-credential",
                path.to_str().unwrap(),
                "another-ref",
                "demo",
            ],
            "sk-x\n",
        ),
        (
            vec![
                "add-credential",
                path.to_str().unwrap(),
                "another-ref",
                "demo",
                "OPENAI_API_KEY",
                "surplus",
            ],
            "sk-x\n",
        ),
        (
            vec!["--json", "add-credential", path.to_str().unwrap()],
            "sk-x\n",
        ),
        (
            vec![
                "add-credential",
                path.to_str().unwrap(),
                "another-ref",
                "demo",
                "OPENAI_API_KEY",
                "--yes",
            ],
            "sk-x\n",
        ),
    ] {
        assert_eq!(
            run(&arguments, value),
            EXIT_USAGE,
            "add-credential accepted {arguments:?}"
        );
    }
    std::fs::remove_dir_all(&directory).unwrap();
}
