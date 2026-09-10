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

#[test]
fn desktop_controller_drives_the_first_release_flow_across_a_crash() {
    let scratch = std::env::temp_dir().join(format!("symbiote-desktop-{}", std::process::id()));
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

    // The controller is what the UI calls; this is its whole flow.
    let mut controller = DesktopController::new(
        DesktopPaths::from_directory(&bin_dir()).unwrap(),
        &state_dir,
        &reservation_base,
        &repo,
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
        Some("implemented the bounded change; produced.txt written by the sandboxed tool")
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
    let _ = std::fs::remove_dir_all(&scratch);
}
