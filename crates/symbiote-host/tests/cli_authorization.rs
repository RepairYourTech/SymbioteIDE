//! The administrative CLI's authorization gate, proven against a fake daemon
//! socket. The property that matters is not the message but the wire: a
//! dangerous operation without explicit authorization must not put a single
//! byte on the socket, and `raw` must not be a way around it.
mod support;

use std::os::unix::fs::PermissionsExt;
use std::process::{Command as Process, Stdio};

use support::*;

#[test]
fn dangerous_commands_send_nothing_without_authorization() {
    let raw = write_raw_operation(&serde_json::json!({"kind": "shutdown"}));
    let raw = raw.display().to_string();
    for arguments in [
        vec!["shutdown"],
        vec!["run-started-dispatch", "task", "dispatch"],
        vec!["start-prepared-task", "task", "host"],
        vec!["raw", raw.as_str()],
    ] {
        let (output, frame) = run_cli(&arguments);
        assert_eq!(
            output.status.code(),
            Some(3),
            "{arguments:?} must report authorization required: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            frame.is_none(),
            "{arguments:?} must not reach the daemon without authorization"
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("--yes"),
            "the refusal names the remedy: {stderr}"
        );
    }
}

#[test]
fn an_explicit_yes_sends_the_dangerous_command() {
    let (output, frame) = run_cli(&["shutdown", "--yes"]);
    assert_eq!(output.status.code(), Some(0));
    let frame = frame.expect("an authorized shutdown reaches the daemon");
    assert_eq!(operation_kind(&frame), "shutdown");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("shutdown"), "{stdout}");
}

#[test]
fn raw_is_authorized_not_blocked() {
    let raw = write_raw_operation(&serde_json::json!({"kind": "shutdown"}));
    let raw = raw.display().to_string();
    let (output, frame) = run_cli(&["raw", raw.as_str(), "--yes"]);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        operation_kind(&frame.expect("an authorized raw operation is sent")),
        "shutdown"
    );
}

#[test]
fn read_only_commands_never_require_authorization() {
    let (output, frame) = run_cli(&["health"]);
    assert_eq!(output.status.code(), Some(0));
    assert!(frame.is_some(), "health is sent without any flag");
}

#[test]
fn a_read_through_raw_is_not_gated() {
    // `raw` is the escape hatch for the operations the typed commands do not
    // cover; gating every use of it would make those reads unusable. The
    // operation it names decides, so a read through `raw` is not gated.
    let raw = write_raw_operation(&serde_json::json!({"kind": "health"}));
    let raw = raw.display().to_string();
    let (output, frame) = run_cli(&["raw", raw.as_str()]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        operation_kind(&frame.expect("a raw read is sent")),
        "health"
    );
}

#[test]
fn the_json_envelope_reports_authorization_required() {
    let (output, frame) = run_cli(&["shutdown", "--json"]);
    assert_eq!(output.status.code(), Some(3));
    assert!(frame.is_none());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let envelope: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("one machine-readable line");
    assert_eq!(envelope["schema"], "symbiote.cli/v1");
    assert_eq!(envelope["command"], "shutdown");
    assert_eq!(envelope["ok"], false);
    assert_eq!(envelope["error"]["code"], "authorization_required");
    assert!(
        envelope["command_id"]
            .as_str()
            .is_some_and(|id| !id.is_empty())
    );
}

#[test]
fn the_json_envelope_wraps_a_successful_result() {
    let (output, frame) = run_cli(&["health", "--json"]);
    assert_eq!(output.status.code(), Some(0));
    assert!(frame.is_some());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().lines().collect();
    assert_eq!(lines.len(), 1, "the envelope is one line: {stdout}");
    let envelope: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(envelope["schema"], "symbiote.cli/v1");
    assert_eq!(envelope["ok"], true);
    assert_eq!(envelope["command"], "health");
    assert!(envelope["result"].is_object());
}

#[test]
fn an_unreachable_daemon_is_distinct_from_a_missing_authorization() {
    // Authorized, but no daemon is listening at this state directory: the
    // exit is a transport failure (1) — never confused with the refusal.
    let directory = unique_directory();
    std::fs::create_dir_all(&directory).unwrap();
    std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o700)).unwrap();
    let output = Process::new(CLI)
        .arg("--state-dir")
        .arg(&directory)
        .arg("health")
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    let _ = std::fs::remove_dir_all(&directory);
}

/// A policy naming `shutdown` and nothing else.
const SHUTDOWN_POLICY: &str = r#"{"schema":"symbiote.cli-policy/v1","authorize":["shutdown"]}"#;

#[test]
fn a_policy_pre_authorizes_a_named_dangerous_operation_without_a_flag() {
    let policy = write_policy(SHUTDOWN_POLICY);
    let (output, frame) = run_cli(&["--policy", policy.to_str().unwrap(), "shutdown"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "a policy pre-authorizes a noninteractive run: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        operation_kind(&frame.expect("a policy-authorized shutdown is sent")),
        "shutdown"
    );
}

#[test]
fn a_policy_does_not_authorize_a_kind_it_does_not_name() {
    let policy = write_policy(SHUTDOWN_POLICY);
    let policy = policy.to_str().unwrap();
    for arguments in [
        vec!["--policy", policy, "start-prepared-task", "task", "host"],
        vec![
            "--policy",
            policy,
            "run-started-dispatch",
            "task",
            "dispatch",
        ],
    ] {
        let (output, frame) = run_cli(&arguments);
        assert_eq!(
            output.status.code(),
            Some(3),
            "{arguments:?} must stay unauthorized: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            frame.is_none(),
            "{arguments:?} must not reach the daemon on an unrelated grant"
        );
    }
}

#[test]
fn a_policy_authorizes_raw_by_the_operation_it_names() {
    let policy = write_policy(SHUTDOWN_POLICY);
    let policy = policy.to_str().unwrap();
    // The policy names `shutdown`, so `raw` naming `shutdown` is authorized...
    let shutdown = write_raw_operation(&serde_json::json!({"kind": "shutdown"}));
    let (output, frame) = run_cli(&["--policy", policy, "raw", shutdown.to_str().unwrap()]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        operation_kind(&frame.expect("an authorized raw operation is sent")),
        "shutdown"
    );
    // ... and the same policy does not authorize a *different* dangerous kind
    // through `raw`: the operation decides, not the command name.
    let start = write_raw_operation(&serde_json::json!({
        "kind": "start_prepared_task", "task_id": "t", "host_id": "h"
    }));
    let (output, frame) = run_cli(&["--policy", policy, "raw", start.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(3));
    assert!(frame.is_none());
    // A read through `raw` needs no policy at all.
    let read = write_raw_operation(&serde_json::json!({"kind": "health"}));
    let (output, frame) = run_cli(&["raw", read.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        operation_kind(&frame.expect("a raw read is sent")),
        "health"
    );
}

#[test]
fn an_expired_policy_grants_nothing_and_says_why() {
    let policy = write_policy(
        r#"{"schema":"symbiote.cli-policy/v1","authorize":["shutdown"],"expires_at":1}"#,
    );
    let policy = policy.to_str().unwrap();
    let (output, frame) = run_cli(&["--policy", policy, "shutdown"]);
    assert_eq!(output.status.code(), Some(3));
    assert!(frame.is_none(), "an expired policy must send nothing");
    // The operator must learn *why* the policy did not apply, or a lapsed
    // expiry looks identical to a policy that was never configured.
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("expired"), "the lapse is named: {stderr}");
    assert!(
        stderr.contains(policy),
        "the lapse names the file: {stderr}"
    );

    // A lapsed policy that never named this kind must NOT be described as a
    // lapsed grant *for* it: saying so would assert something about the file
    // that is not true, and would send the operator looking for a grant they
    // never wrote.
    let other = write_policy(
        r#"{"schema":"symbiote.cli-policy/v1","authorize":["start_prepared_task"],"expires_at":1}"#,
    );
    let (output, frame) = run_cli(&["--policy", other.to_str().unwrap(), "shutdown"]);
    assert_eq!(output.status.code(), Some(3));
    assert!(frame.is_none());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("pre-authorized"),
        "a lapse is not reported for a kind the policy never named: {stderr}"
    );
}

#[test]
fn a_policy_that_cannot_be_honored_is_a_usage_failure_and_sends_nothing() {
    // Shared permissions: a neighbor could rewrite the grant, so it is
    // refused rather than honored.
    let shared = unique_directory().with_extension("shared.json");
    std::fs::write(&shared, SHUTDOWN_POLICY).unwrap();
    std::fs::set_permissions(&shared, std::fs::Permissions::from_mode(0o644)).unwrap();
    let shared = shared.to_str().unwrap().to_owned();
    let (output, frame) = run_cli(&["--policy", &shared, "shutdown"]);
    assert_eq!(
        output.status.code(),
        Some(1),
        "an unusable policy is an operator error, not a missing authorization"
    );
    assert!(frame.is_none());
    assert!(String::from_utf8_lossy(&output.stderr).contains("policy"));

    // An unknown schema is the same class of failure, and the automation
    // envelope names it with its own code.
    let bogus = write_policy(r#"{"schema":"symbiote.cli-policy/v9","authorize":["shutdown"]}"#);
    let bogus = bogus.to_str().unwrap().to_owned();
    let (output, frame) = run_cli(&["--policy", &bogus, "shutdown", "--json"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(frame.is_none());
    let envelope: serde_json::Value =
        serde_json::from_str(String::from_utf8_lossy(&output.stdout).trim()).unwrap();
    assert_eq!(envelope["schema"], "symbiote.cli/v1");
    assert_eq!(envelope["error"]["code"], "policy_invalid");

    // A missing policy file is not silently treated as "no policy".
    let missing = unique_directory().with_extension("missing.json");
    let missing = missing.to_str().unwrap().to_owned();
    let (output, frame) = run_cli(&["--policy", &missing, "shutdown"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(frame.is_none());

    // A broken policy never blocks a read or a mutation: it is consulted only
    // for an operation that is actually dangerous.
    for arguments in [
        vec!["--policy", bogus.as_str(), "health"],
        vec!["--policy", bogus.as_str(), "scheduling-projection"],
    ] {
        let (output, frame) = run_cli(&arguments);
        assert_eq!(
            output.status.code(),
            Some(0),
            "{arguments:?} must not be broken by an unrelated policy: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(frame.is_some());
    }

    // `--yes` is a complete authorization on its own, so a broken policy does
    // not block an explicitly authorized run.
    let (output, frame) = run_cli(&["--policy", &bogus, "shutdown", "--yes"]);
    assert_eq!(output.status.code(), Some(0));
    assert!(frame.is_some());
}

#[test]
fn the_environment_selects_the_policy_when_the_flag_does_not() {
    let policy = write_policy(SHUTDOWN_POLICY);
    let policy = policy.to_str().unwrap().to_owned();
    let (output, frame) = run_cli_with(&["shutdown"], Some(&policy));
    assert_eq!(
        output.status.code(),
        Some(0),
        "the environment policy authorizes an unattended run: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        operation_kind(&frame.expect("the environment policy's grant is sent")),
        "shutdown"
    );

    // The flag wins over the environment: the environment's grant is not read.
    let other =
        write_policy(r#"{"schema":"symbiote.cli-policy/v1","authorize":["start_prepared_task"]}"#);
    let other = other.to_str().unwrap().to_owned();
    let (output, frame) = run_cli_with(&["--policy", &other, "shutdown"], Some(&policy));
    assert_eq!(output.status.code(), Some(3));
    assert!(frame.is_none());

    // An empty variable means "no policy", like an unset one — it must not be
    // read as a path, and it must not authorize anything.
    let (output, frame) = run_cli_with(&["shutdown"], Some(""));
    assert_eq!(output.status.code(), Some(3));
    assert!(frame.is_none());
}

#[test]
fn a_policy_that_names_a_non_dangerous_or_unknown_kind_is_refused() {
    // A read or a mutation is already ungated, so naming one in a policy would
    // be a no-op the operator could mistake for coverage; a wildcard would be
    // a blanket grant; and an unknown kind cannot be classified at all. Each is
    // rejected — and, because the policy is consulted only for a dangerous
    // operation, surfacing it takes a dangerous command.
    for kind in [
        "get_project",
        "create_task",
        "health",
        "*",
        "delete_everything",
    ] {
        let policy = write_policy(&format!(
            r#"{{"schema":"symbiote.cli-policy/v1","authorize":["{kind}"]}}"#
        ));
        let policy = policy.to_str().unwrap();
        let (output, frame) = run_cli(&["--policy", policy, "shutdown"]);
        assert_eq!(output.status.code(), Some(1), "policy naming {kind:?}");
        assert!(frame.is_none(), "policy naming {kind:?} must not send");
        assert!(String::from_utf8_lossy(&output.stderr).contains("policy"));
    }
}

#[test]
fn a_policy_authorizes_sending_but_never_overrides_the_daemon() {
    // The property that must hold: a policy changes only whether the CLI
    // *sends*. The daemon's own authorization is untouched, so a
    // policy-authorized command the daemon refuses is still the daemon's typed
    // refusal (exit 2), exactly as a `--yes`-authorized one would be.
    let policy = write_policy(SHUTDOWN_POLICY);
    let policy = policy.to_str().unwrap();
    let (output, frame) = run_cli_refused(&["--policy", policy, "shutdown"]);
    assert!(
        frame.is_some(),
        "the policy did authorize sending the request"
    );
    assert_eq!(
        output.status.code(),
        Some(2),
        "the daemon's refusal is still exit 2: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let error: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(error["code"], "permission_denied");
}
