//! The administrative CLI's authorization gate, proven against a fake daemon
//! socket. The property that matters is not the message but the wire: a
//! dangerous operation without explicit authorization must not put a single
//! byte on the socket, and `raw` must not be a way around it.
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::process::{Command as Process, Output, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use symbiote_protocol::{Request, Response, ResponseBody, encode_response};

const CLI: &str = env!("CARGO_BIN_EXE_symbiote");

static NEXT_DIRECTORY: AtomicUsize = AtomicUsize::new(0);

fn unique_directory() -> PathBuf {
    std::env::temp_dir().join(format!(
        "symbiote-cli-authorization-{}-{}",
        std::process::id(),
        NEXT_DIRECTORY.fetch_add(1, Ordering::SeqCst)
    ))
}

/// Reads one frame and answers it. The fixed `Shutdown` body is deliberate:
/// these tests pin the authorization and exit-code contract, not the
/// daemon's per-operation bodies.
fn reply(stream: UnixStream) -> Vec<u8> {
    stream.set_nonblocking(false).unwrap();
    let frame = symbiote_host::transport::read_frame(&stream).expect("one bounded request frame");
    let request: Request =
        serde_json::from_slice(&frame).expect("the CLI sends a typed request envelope");
    let bytes = encode_response(&Response::success(&request, ResponseBody::Shutdown {})).unwrap();
    symbiote_host::transport::write_response(&stream, &bytes).unwrap();
    frame
}

/// Waits for at most one request. Returns `None` when the CLI never
/// connected — including a final check after the CLI has exited, so a
/// connection that was already queued can never be reported as "never sent".
fn serve(listener: &UnixListener, cli_done: &AtomicBool) -> Option<Vec<u8>> {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        match listener.accept() {
            Ok((stream, _)) => return Some(reply(stream)),
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(error) => panic!("fake daemon accept failed: {error}"),
        }
        if cli_done.load(Ordering::SeqCst) {
            return listener.accept().ok().map(|(stream, _)| reply(stream));
        }
        assert!(Instant::now() < deadline, "the fake daemon waited too long");
        std::thread::sleep(Duration::from_millis(5));
    }
}

/// Runs the CLI against a fresh fake daemon and returns its output plus the
/// request frame the daemon saw (`None` when none arrived).
fn run_cli(arguments: &[&str]) -> (Output, Option<Vec<u8>>) {
    let directory = unique_directory();
    std::fs::create_dir_all(&directory).unwrap();
    std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o700)).unwrap();
    // The listener keeps its exclusive host lock until it drops, so it must
    // outlive the serving thread: a scoped thread borrows it instead of
    // moving the `Drop` type out.
    let daemon = symbiote_host::transport::LocalListener::bind(&directory)
        .expect("the private state directory accepts a daemon socket");
    daemon.listener.set_nonblocking(true).unwrap();
    let cli_done = Arc::new(AtomicBool::new(false));
    let (output, frame) = std::thread::scope(|scope| {
        let listener = &daemon.listener;
        let watched = cli_done.clone();
        let server = scope.spawn(move || serve(listener, &watched));
        let output = Process::new(CLI)
            .arg("--state-dir")
            .arg(&directory)
            .args(arguments)
            // No terminal on stdin: exactly the unattended-script case.
            .stdin(Stdio::null())
            .output()
            .expect("the CLI binary runs");
        cli_done.store(true, Ordering::SeqCst);
        (
            output,
            server.join().expect("the fake daemon thread finishes"),
        )
    });
    let _ = std::fs::remove_dir_all(&directory);
    (output, frame)
}

fn operation_kind(frame: &[u8]) -> String {
    let request: Request = serde_json::from_slice(frame).unwrap();
    serde_json::to_value(&request.operation).unwrap()["kind"]
        .as_str()
        .expect("every operation carries a kind")
        .to_owned()
}

fn write_raw_operation(operation: &serde_json::Value) -> PathBuf {
    let path = unique_directory().with_extension("json");
    let mut file = std::fs::File::create(&path).unwrap();
    file.write_all(operation.to_string().as_bytes()).unwrap();
    path
}

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
fn a_daemon_refusal_is_distinct_from_a_missing_authorization() {
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
