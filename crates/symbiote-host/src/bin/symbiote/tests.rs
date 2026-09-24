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
    assert_eq!(rows.len(), 3, "the table declares one arm per command kind");

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
        3,
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
    let dangerous = dangerous_kinds();
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
