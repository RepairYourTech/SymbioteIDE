//! The desktop controller's architecture proof, headless: the SAME
//! controller the Tauri commands call drives a REAL daemon through the
//! first-release flow — open repository, start, crash the daemon exactly
//! like a dead desktop process, restart, resume from persisted positions,
//! run with real sandboxed tool execution, and read the completion
//! evidence. The model turn is the operator's explicitly labeled fixture
//! transport; nothing is spent and no live model turn is attempted.
#![cfg(target_os = "linux")]
use std::path::PathBuf;
use std::process::Command;
use symbiote_desktop_lib::controller::{DesktopController, DesktopPaths};

fn git(repo: &std::path::Path, args: &[&str]) {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .expect("git");
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn bin_dir() -> PathBuf {
    // Workspace target directory: the gauntlet builds every binary.
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for ancestor in manifest.ancestors().skip(1) {
        let candidate = ancestor.join("target/debug");
        if candidate.join("symbioted").is_file()
            && candidate.join("symbiote-sandbox-launch").is_file()
        {
            return candidate;
        }
    }
    panic!("workspace binaries not built; run the workspace gauntlet");
}

fn test_env(tag: &str) -> TestEnv {
    let scratch =
        std::env::temp_dir().join(format!("symbiote-desktop-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&scratch);
    std::fs::create_dir_all(&scratch).unwrap();
    let state_dir = scratch.join("state");
    let reservation_base = scratch.join("worktrees");
    let repo = scratch.join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    git(&repo, &["init", "-q", "-b", "main"]);
    std::fs::write(repo.join("README.md"), "desktop demo repository\n").unwrap();
    git(&repo, &["add", "."]);
    git(
        &repo,
        &[
            "-c",
            "user.email=t@t",
            "-c",
            "user.name=t",
            "commit",
            "-q",
            "-m",
            "base",
        ],
    );
    TestEnv {
        scratch,
        state_dir,
        reservation_base,
        repo,
    }
}

#[test]
fn desktop_controller_drives_the_first_release_flow_across_a_crash() {
    let env = test_env("main");
    let state_dir = &env.state_dir;
    let reservation_base = &env.reservation_base;
    let repo = &env.repo;

    // The controller is what the UI calls; this is its whole flow.
    let mut controller = DesktopController::new(
        DesktopPaths::from_directory(&bin_dir()).unwrap(),
        state_dir,
        reservation_base,
        repo,
    )
    .expect("controller environment");
    controller.begin_generation().expect("first generation");

    let dispatch_id = controller.start_demo().expect("start half");
    let cursor = controller.read_journal().expect("evidence trail");
    assert!(cursor > 0, "the evidence trail advanced");
    // The desktop persists its positions; verify the file is real.
    let positions_file = state_dir.join("desktop-positions.json");
    let persisted: Vec<symbiote_client_sdk::JournalPosition> =
        serde_json::from_slice(&std::fs::read(&positions_file).expect("persisted positions"))
            .expect("persisted positions parse");
    assert!(!persisted.is_empty());

    // The daemon dies exactly like a crashed process; the controller's
    // crash path is the desktop's. std's kill is SIGKILL on Unix.
    controller.crash_daemon();

    // Restart on the same state directory: a NEW generation resumes from
    // the persisted positions and finishes the workflow.
    controller.begin_generation().expect("restart generation");
    assert!(
        controller.journal_position() > 0,
        "the restarted controller restored the persisted position"
    );
    let outcome = controller.finish_demo(&dispatch_id).expect("finish half");
    assert_eq!(outcome.task_state, "completion_requested");
    assert_eq!(
        outcome.report.as_deref(),
        Some(symbiote_workflow::demo::FIXTURE_REPORT)
    );
    assert!(
        outcome.worktree.contains("produced.txt"),
        "worktree evidence must list produced.txt: {:?}",
        outcome.worktree
    );
    let produced = std::fs::read_to_string(outcome.worktree.worktree.join("produced.txt"))
        .expect("produced file");
    assert_eq!(produced.trim(), "worker-output");

    // Graceful shutdown leaves no daemon behind (the Drop also enforces).
    controller.stop();
    let _ = std::fs::remove_dir_all(&env.scratch);
}

/// The desktop test's private environment; removed on Drop.
struct TestEnv {
    scratch: PathBuf,
    state_dir: PathBuf,
    reservation_base: PathBuf,
    repo: PathBuf,
}
impl Drop for TestEnv {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.scratch);
    }
}

/// The daemon is gone when nothing accepts on its socket any more (the
/// socket FILE may linger after an unclean exit; the connection is the
/// truth). Fails the test if the daemon is still serving at the deadline.
fn daemon_is_gone(state_dir: &std::path::Path) -> bool {
    let socket = state_dir.join("host.sock");
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        if std::os::unix::net::UnixStream::connect(&socket).is_err() {
            return true;
        }
        if std::time::Instant::now() > deadline {
            return false;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
}

/// The Drop guard (#513 review): a controller dropped WITHOUT stop() —
/// a panicking or closed UI, not a graceful shutdown — must still leave
/// no daemon behind.
#[test]
fn a_controller_dropped_without_stop_leaves_no_daemon_behind() {
    let env = test_env("drop-guard");
    {
        let mut controller = symbiote_desktop_lib::controller::DesktopController::new(
            bin_dir_paths(),
            &env.state_dir,
            &env.reservation_base,
            &env.repo,
        )
        .expect("controller environment");
        controller.begin_generation().expect("first generation");
        // No stop(): the scope ends, Drop must do the bounded-stop work.
    }
    assert!(
        daemon_is_gone(&env.state_dir),
        "the daemon must not survive the controller's Drop"
    );
}

/// A corrupt positions file is SURFACED, never silently ignored (#513
/// review): silently resetting it would discard the resume point the
/// whole restart story depends on. The failed generation also reaps its
/// just-spawned daemon.
#[test]
fn a_corrupt_positions_file_is_surfaced_not_silently_ignored() {
    let env = test_env("corrupt-positions");
    let mut controller = symbiote_desktop_lib::controller::DesktopController::new(
        bin_dir_paths(),
        &env.state_dir,
        &env.reservation_base,
        &env.repo,
    )
    .expect("controller environment");
    // The file exists but does not parse: the exact case a silent reset
    // would hide.
    std::fs::write(env.state_dir.join("desktop-positions.json"), b"{not json").unwrap();
    let error = controller
        .begin_generation()
        .expect_err("a corrupt positions file must be an error");
    assert!(
        matches!(
            error,
            symbiote_desktop_lib::controller::DesktopError::CorruptPositions(_)
        ),
        "expected CorruptPositions, got {error:?}"
    );
    assert!(
        daemon_is_gone(&env.state_dir),
        "the failed generation must not leave its daemon behind"
    );
}

/// The daemon's exit status is surfaced in the DaemonExited error (#513
/// review) instead of being discarded: an operator can see the daemon
/// exited with a failure, not just that it "exited".
#[test]
fn begin_generation_reports_the_daemon_status_when_it_exits() {
    let env = test_env("daemon-exited");
    let mut controller = symbiote_desktop_lib::controller::DesktopController::new(
        bin_dir_paths(),
        &env.state_dir,
        &env.reservation_base,
        &env.repo,
    )
    .expect("controller environment");
    // An unparseable operator config makes the daemon exit at startup,
    // before its socket exists.
    std::fs::write(env.state_dir.join("operator-config.json"), b"not a config").unwrap();
    let error = controller
        .begin_generation()
        .expect_err("the daemon must have exited");
    match error {
        symbiote_desktop_lib::controller::DesktopError::DaemonExited(status) => {
            assert!(
                !status.is_empty(),
                "the exit status must be surfaced, got empty"
            );
        }
        other => panic!("expected DaemonExited with a status, got {other:?}"),
    }
}

/// The DesktopPaths from the workspace target directory (the gauntlet
/// builds every binary).
fn bin_dir_paths() -> symbiote_desktop_lib::controller::DesktopPaths {
    symbiote_desktop_lib::controller::DesktopPaths::from_directory(&bin_dir()).expect("bin dir")
}
