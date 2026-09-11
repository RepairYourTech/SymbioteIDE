//! The fake daemon the `symbiote` CLI integration tests run against: a real
//! private socket that a test thread serves, plus the policy-file and raw
//! operation writers the CLI's fixtures need. Shared so the authorization
//! tests and the schema-contract tests drive the *same* harness — two
//! divergent fake daemons would let one suite prove something the other does
//! not. Each test crate compiles its own copy and uses a subset of it, so
//! unused helpers here are not dead code.
#![allow(dead_code)]

use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::process::{Command as Process, Output, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use symbiote_protocol::{
    ErrorCode, ProtocolError, Request, Response, ResponseBody, encode_response,
};

/// The binary under test, built by cargo for this integration test.
pub const CLI: &str = env!("CARGO_BIN_EXE_symbiote");

static NEXT_DIRECTORY: AtomicUsize = AtomicUsize::new(0);

pub fn unique_directory() -> PathBuf {
    std::env::temp_dir().join(format!(
        "symbiote-cli-contract-{}-{}",
        std::process::id(),
        NEXT_DIRECTORY.fetch_add(1, Ordering::SeqCst)
    ))
}

/// How the fake daemon answers. `Refuse` is the daemon's own typed rejection,
/// which a policy must never be able to talk it out of.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Answer {
    Success,
    Refuse,
}

/// Reads one frame and answers it. The fixed `Shutdown` body is deliberate:
/// these tests pin the authorization, exit-code and envelope contracts, not
/// the daemon's per-operation bodies.
pub fn reply(stream: UnixStream, answer: Answer) -> Vec<u8> {
    stream.set_nonblocking(false).unwrap();
    let frame = symbiote_host::transport::read_frame(&stream).expect("one bounded request frame");
    let request: Request =
        serde_json::from_slice(&frame).expect("the CLI sends a typed request envelope");
    let response = match answer {
        Answer::Success => Response::success(&request, ResponseBody::Shutdown {}),
        Answer::Refuse => Response::failure(
            Some(request.correlation_id.clone()),
            ProtocolError::new(ErrorCode::PermissionDenied),
        ),
    };
    let bytes = encode_response(&response).unwrap();
    symbiote_host::transport::write_response(&stream, &bytes).unwrap();
    frame
}

/// Waits for at most one request. Returns `None` when the CLI never
/// connected — including a final check after the CLI has exited, so a
/// connection that was already queued can never be reported as "never sent".
pub fn serve(listener: &UnixListener, cli_done: &AtomicBool, answer: Answer) -> Option<Vec<u8>> {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        match listener.accept() {
            Ok((stream, _)) => return Some(reply(stream, answer)),
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(error) => panic!("fake daemon accept failed: {error}"),
        }
        if cli_done.load(Ordering::SeqCst) {
            return listener
                .accept()
                .ok()
                .map(|(stream, _)| reply(stream, answer));
        }
        assert!(Instant::now() < deadline, "the fake daemon waited too long");
        std::thread::sleep(Duration::from_millis(5));
    }
}

/// Runs the CLI against a fresh fake daemon and returns its output plus the
/// request frame the daemon saw (`None` when none arrived).
pub fn run_cli(arguments: &[&str]) -> (Output, Option<Vec<u8>>) {
    run_cli_full(arguments, None, Answer::Success)
}

/// Runs the CLI against a fake daemon that refuses with its own typed error.
pub fn run_cli_refused(arguments: &[&str]) -> (Output, Option<Vec<u8>>) {
    run_cli_full(arguments, None, Answer::Refuse)
}

/// Runs the CLI with an optional `SYMBIOTE_CLI_POLICY` exported for the child.
/// The variable is always cleared first, so a policy can only reach the CLI
/// when a test names it and an ambient value cannot decide the outcome.
pub fn run_cli_with(arguments: &[&str], policy_env: Option<&str>) -> (Output, Option<Vec<u8>>) {
    run_cli_full(arguments, policy_env, Answer::Success)
}

pub fn run_cli_full(
    arguments: &[&str],
    policy_env: Option<&str>,
    answer: Answer,
) -> (Output, Option<Vec<u8>>) {
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
        let server = scope.spawn(move || serve(listener, &watched, answer));
        let mut process = Process::new(CLI);
        process
            .arg("--state-dir")
            .arg(&directory)
            .args(arguments)
            // No terminal on stdin: exactly the unattended-script case.
            .stdin(Stdio::null())
            .env_remove("SYMBIOTE_CLI_POLICY");
        if let Some(value) = policy_env {
            process.env("SYMBIOTE_CLI_POLICY", value);
        }
        let output = process.output().expect("the CLI binary runs");
        cli_done.store(true, Ordering::SeqCst);
        (
            output,
            server.join().expect("the fake daemon thread finishes"),
        )
    });
    let _ = std::fs::remove_dir_all(&directory);
    (output, frame)
}

pub fn operation_kind(frame: &[u8]) -> String {
    let request: Request = serde_json::from_slice(frame).unwrap();
    serde_json::to_value(&request.operation).unwrap()["kind"]
        .as_str()
        .expect("every operation carries a kind")
        .to_owned()
}

pub fn write_raw_operation(operation: &serde_json::Value) -> PathBuf {
    let path = unique_directory().with_extension("json");
    let mut file = std::fs::File::create(&path).unwrap();
    file.write_all(operation.to_string().as_bytes()).unwrap();
    path
}

/// Writes a policy file with the only mode the CLI honors, so tests exercise
/// the policy's *content* rather than its permissions.
pub fn write_policy(document: &str) -> PathBuf {
    let path = unique_directory().with_extension("policy.json");
    std::fs::write(&path, document).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
    path
}
