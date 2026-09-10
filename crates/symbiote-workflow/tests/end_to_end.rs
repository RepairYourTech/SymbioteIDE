//! The first-release demonstration proof (#54): the complete
//! open-repository → task → started dispatch → SIGKILL → daemon restart →
//! run → completion-evidence → diff sequence, driven through
//! `symbiote-workflow` over the real daemon with the operator-provisioned
//! configuration. The model turn is the operator's explicitly labeled
//! fixture transport; worktree provisioning, sandboxed shell execution,
//! and completion-evidence filing are real. Nothing is spent; no live
//! model turn is attempted.
#![cfg(target_os = "linux")]
use std::fs::DirBuilder;
use std::io::Write as _;
use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};
use symbiote_protocol::ResponseBody;

struct Daemon {
    child: Child,
    state_dir: PathBuf,
}

impl Daemon {
    fn spawn(state_dir: &std::path::Path, config_path: &std::path::Path) -> Self {
        let mut child = Command::new(daemon_binary())
            .arg("--state-dir")
            .arg(state_dir)
            .arg("--operator-config")
            .arg(config_path)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn symbioted");
        let socket = state_dir.join("host.sock");
        let deadline = Instant::now() + Duration::from_secs(15);
        while !socket.exists() {
            if let Some(status) = child.try_wait().expect("daemon status") {
                panic!("daemon exited early: {status}");
            }
            if Instant::now() > deadline {
                let _ = child.kill();
                panic!("daemon socket never appeared");
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        loop {
            match std::os::unix::net::UnixStream::connect(&socket) {
                Ok(_) => break,
                Err(_) if Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(50));
                }
                Err(error) => {
                    let _ = child.kill();
                    panic!("daemon socket never connected: {error}");
                }
            }
        }
        Self {
            child,
            state_dir: state_dir.to_path_buf(),
        }
    }

    /// SIGKILL, exactly as a crashed desktop process dies: no graceful
    /// shutdown, the journal is all that survives. Returns the state
    /// directory for the restart generation.
    fn kill9(&mut self) -> PathBuf {
        nix::sys::signal::kill(
            nix::unistd::Pid::from_raw(self.child.id() as i32),
            nix::sys::signal::Signal::SIGKILL,
        )
        .expect("SIGKILL the daemon");
        let _ = self.child.wait();
        self.state_dir.clone()
    }
}

impl Drop for Daemon {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// The daemon binary from the workspace target directory (the workspace
/// gauntlet builds every bin; a partial single-crate run must build the
/// workspace first).
fn daemon_binary() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for ancestor in manifest.ancestors().skip(1) {
        let candidate = ancestor.join("target/debug/symbioted");
        if candidate.is_file() {
            return candidate;
        }
    }
    panic!("symbioted not built; run the workspace gauntlet (cargo test --workspace)");
}

fn launcher_binary() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for ancestor in manifest.ancestors().skip(1) {
        let candidate = ancestor.join("target/debug/symbiote-sandbox-launch");
        if candidate.is_file() {
            return candidate;
        }
    }
    panic!(
        "symbiote-sandbox-launch not built; run the workspace gauntlet (cargo test --workspace)"
    );
}

fn git(repo: &std::path::Path, args: &[&str]) -> String {
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
    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}

/// The operator configuration for the demo: reservation base, the
/// explicitly labeled fixture model transport proposing one shell tool,
/// the fixture credential registration, and the shell program allowlist.
/// 0600 — the load refuses anything looser.
fn write_operator_config(
    path: &std::path::Path,
    reservation_base: &std::path::Path,
    launcher: &std::path::Path,
) {
    let config = serde_json::json!({
        "reservation_base": reservation_base.display().to_string(),
        "native_fixture": {
            "echo_text": "implemented the bounded change; produced.txt written by the sandboxed tool",
            "tool": {"call_id": "call-produce",
                "arguments": {"program": "sh",
                    "arguments": ["-c",
                        "printf worker-output > produced.txt"]}}
        },
        "credential_broker": [{"reference": "native-vault-ref",
            "project": "staffing-demo", "environment": "OPENAI_API_KEY",
            "value": "fixture-not-a-real-secret"}],
        "external_fixture": {"thread_id": "thr-demo-external",
            "agent_message": "external harness implemented the bounded change"},
        "shell_executor": {"launcher_path": launcher.display().to_string(),
            "protected_paths": ["/etc", "/var", "/home"],
            "allowed_programs": ["sh"]}
    });
    let mut file = std::fs::File::create(path).expect("config file");
    file.write_all(serde_json::to_string_pretty(&config).unwrap().as_bytes())
        .unwrap();
    file.set_permissions(std::fs::Permissions::from_mode(0o600))
        .unwrap();
}

/// The demo's private environment: scratch state directory, reservation
/// base, a real one-commit git repository, and the operator config. Each
/// test gets its own namespace so parallel runs never collide.
struct DemoEnv {
    scratch: PathBuf,
    state_dir: PathBuf,
    reservation_base: PathBuf,
    repo: PathBuf,
    config_path: PathBuf,
}
impl Drop for DemoEnv {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.scratch);
    }
}
fn demo_environment(tag: &str) -> DemoEnv {
    let scratch = std::env::temp_dir().join(format!(
        "symbiote-workflow-demo-{tag}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&scratch);
    std::fs::create_dir_all(&scratch).unwrap();
    let state_dir = scratch.join("state");
    let reservation_base = scratch.join("worktrees");
    DirBuilder::new().mode(0o700).create(&state_dir).unwrap();
    std::fs::create_dir_all(&reservation_base).unwrap();
    let repo = scratch.join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    git(&repo, &["init", "-q", "-b", "main"]);
    std::fs::write(repo.join("README.md"), "demo repository\n").unwrap();
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
    let config_path = scratch.join("operator-config.json");
    write_operator_config(&config_path, &reservation_base, &launcher_binary());
    DemoEnv {
        scratch,
        state_dir,
        reservation_base,
        repo,
        config_path,
    }
}

#[test]
fn first_release_demo_workflow_drives_daemon_end_to_end_with_restart_resume() {
    let env = demo_environment("main");
    let state_dir = &env.state_dir;
    let reservation_base = &env.reservation_base;
    let repo = &env.repo;
    let config_path = &env.config_path;

    // Generation 1: drive the workflow up to the STARTED dispatch — the
    // point where canonical state is fully journaled — then SIGKILL the
    // daemon exactly like a crashed desktop process.
    let mut daemon = Daemon::spawn(state_dir, config_path);
    let mut workflow = symbiote_workflow::DemoWorkflow::connect(state_dir, reservation_base, repo)
        .expect("connect");
    let dispatch_id = workflow.start_demo().expect("demo start half");
    // The desktop shell follows the evidence trail before the crash: the
    // driver's tracked journal position is what resumes across restart.
    let cursor_at_kill = workflow.read_journal().expect("journal read before kill");
    assert!(
        cursor_at_kill > 0,
        "the journal page advanced before the kill"
    );
    let state_dir_after_kill = daemon.kill9();

    // Generation 2: restart on the SAME state directory. The project,
    // task, and started dispatch all survived; the driver reconnects and
    // finishes the workflow against the restarted daemon.
    let _daemon = Daemon::spawn(&state_dir_after_kill, config_path);
    let outcome = workflow
        .finish_demo(&dispatch_id)
        .expect("demo finish half after restart");
    assert_eq!(outcome.task_state, "completion_requested");
    // The worker's report is the fixture transport's text — evidence, not
    // verified completion.
    assert_eq!(
        outcome.report.as_deref(),
        Some("implemented the bounded change; produced.txt written by the sandboxed tool")
    );
    // The diff evidence: the run really produced the file inside the
    // reserved worktree through the sandboxed shell executor.
    assert!(
        outcome.worktree.contains("produced.txt"),
        "worktree evidence must list produced.txt: {:?}",
        outcome.worktree
    );
    let content = std::fs::read_to_string(outcome.worktree.worktree.join("produced.txt"))
        .expect("produced file readable");
    assert_eq!(content.trim(), "worker-output");
    // Resume: the driver's journal position advanced across the restart,
    // reading the durable evidence trail from the restarted daemon.
    assert!(
        outcome.journal_cursor >= cursor_at_kill,
        "journal cursor must not go backwards across a restart"
    );
}

#[test]
fn driver_restart_restores_serialized_journal_positions_and_resumes() {
    let env = demo_environment("driver-restart");
    let state_dir = &env.state_dir;
    let reservation_base = &env.reservation_base;
    let repo = &env.repo;
    let config_path = &env.config_path;
    let project = symbiote_domain::ProjectId::new(symbiote_workflow::demo::PROJECT).unwrap();

    // Generation 1: the start half plus a read of the evidence trail —
    // then the desktop process itself dies, taking the driver's in-memory
    // state with it.
    let mut daemon = Daemon::spawn(state_dir, config_path);
    let mut workflow = symbiote_workflow::DemoWorkflow::connect(state_dir, reservation_base, repo)
        .expect("connect");
    let dispatch_id = workflow.start_demo().expect("demo start half");
    let cursor_at_kill = workflow.read_journal().expect("journal read before kill");
    assert!(
        cursor_at_kill > 0,
        "the journal page advanced before the kill"
    );
    // Persist the driver's positions exactly as a desktop shell would:
    // serialize to disk, so the restored driver shares NO memory with the
    // one that observed these events.
    let positions_path = env.scratch.join("driver-positions.json");
    std::fs::write(
        &positions_path,
        serde_json::to_vec(&workflow.positions()).expect("serialize positions"),
    )
    .unwrap();
    let _state_dir = daemon.kill9();

    // Generation 2: the daemon restarts on the same state directory and a
    // FRESH driver is reconstructed from the serialized positions.
    let _daemon = Daemon::spawn(state_dir, config_path);
    let restored: Vec<symbiote_client_sdk::JournalPosition> =
        serde_json::from_slice(&std::fs::read(&positions_path).unwrap())
            .expect("restore serialized positions");
    let mut workflow = symbiote_workflow::DemoWorkflow::connect(state_dir, reservation_base, repo)
        .expect("connect")
        .with_positions(restored);
    assert_eq!(
        workflow.journal_position(&project).0,
        cursor_at_kill,
        "the restored driver must resume exactly where the dead one stopped"
    );
    // Resume: the run's own events (filed after the restored position)
    // carry the completion evidence, and the journal never rewinds.
    let outcome = workflow
        .finish_demo(&dispatch_id)
        .expect("demo finish half");
    assert!(outcome.journal_cursor >= cursor_at_kill);
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
}

/// The two-harness demo (#269): native AND external workers staffing ONE
/// project, driven end to end over the wire across a daemon SIGKILL. Both
/// lanes run the real daemon path — provisioning, runtime-kind strictness,
/// harness approval refusal, completion-evidence filing — with the
/// operator's explicitly labeled fixture transports (no binary, no model
/// turn, nothing spent; live turns for BOTH runtimes remain gated on
/// explicit user authorization for credentials and billing). The
/// assertions pin what "without leakage" means observably: the lanes'
/// reports and worktrees never cross, and the external binding carries no
/// UseCredential grant and its own credential reference, so the operator's
/// broker has nothing to give it.
#[test]
fn two_harness_demo_native_and_external_workers_on_one_project_without_leakage() {
    let env = demo_environment("two-harness");
    let state_dir = &env.state_dir;
    let reservation_base = &env.reservation_base;
    let repo = &env.repo;
    let config_path = &env.config_path;

    // Both lanes start on the same project against the same daemon.
    let mut daemon = Daemon::spawn(state_dir, config_path);
    let mut workflow = symbiote_workflow::DemoWorkflow::connect(state_dir, reservation_base, repo)
        .expect("connect");
    let native_dispatch = workflow.start_demo().expect("native start half");
    let external_dispatch = workflow.start_demo_external().expect("external start half");
    assert_ne!(native_dispatch, external_dispatch);
    // Mixed staffing survives a crash: both started dispatches are
    // journaled canonical state; the restarted daemon resumes both lanes.
    let state_dir_after_kill = daemon.kill9();
    let _daemon = Daemon::spawn(&state_dir_after_kill, config_path);

    // Run each lane through its own dispatch: the runtime comes from each
    // dispatch's contract (the driver refuses a crossed lane).
    let native = workflow.finish_demo(&native_dispatch).expect("native run");
    let external = workflow
        .finish_demo_external(&external_dispatch)
        .expect("external run");
    assert_eq!(native.task_state, "completion_requested");
    assert_eq!(external.task_state, "completion_requested");
    // Work isolation: each lane's report is its own worker's text.
    assert_eq!(
        native.report.as_deref(),
        Some("implemented the bounded change; produced.txt written by the sandboxed tool")
    );
    assert_eq!(
        external.report.as_deref(),
        Some("external harness implemented the bounded change")
    );
    assert_ne!(native.report, external.report);
    // The native lane really produced its file inside its own worktree;
    // the external lane's worktree is a DIFFERENT directory with no
    // native output in it.
    assert!(
        native.worktree.contains("produced.txt"),
        "native worktree evidence must list produced.txt: {:?}",
        native.worktree
    );
    assert_ne!(
        native.worktree.worktree, external.worktree.worktree,
        "the lanes must not share a worktree"
    );
    assert!(
        !external.worktree.contains("produced.txt"),
        "the external lane must not observe native work: {:?}",
        external.worktree
    );

    // Configuration/credential isolation, read back over the wire: the
    // native binding grants UseCredential and its profile references the
    // broker-registered fixture credential; the external binding grants
    // NO UseCredential and its profile references the harness's own
    // credential — the broker has no registration for it.
    let mut driver = symbiote_workflow::Driver::connect(state_dir).expect("driver");
    let project = symbiote_domain::ProjectId::new(symbiote_workflow::demo::PROJECT).unwrap();
    let native_binding = driver
        .call(
            "two-harness-binding-native",
            serde_json::json!({"kind":"get_binding","project_id":"staffing-demo",
            "binding_id":"engineer-binding"}),
            Some(&project),
        )
        .expect("native binding read");
    let external_binding = driver
        .call(
            "two-harness-binding-external",
            serde_json::json!({"kind":"get_binding","project_id":"staffing-demo",
            "binding_id":"engineer-external-binding"}),
            Some(&project),
        )
        .expect("external binding read");
    let ResponseBody::Binding(native) = native_binding else {
        panic!("native binding readback");
    };
    let ResponseBody::Binding(external) = external_binding else {
        panic!("external binding readback");
    };
    assert!(
        native
            .binding
            .access
            .grants
            .contains(&symbiote_domain::Permission::UseCredential),
        "the native lane's binding grants UseCredential"
    );
    assert!(
        !external
            .binding
            .access
            .grants
            .contains(&symbiote_domain::Permission::UseCredential),
        "the external lane's binding must NOT grant UseCredential"
    );
    assert_eq!(
        external.primary.profile.credential.as_str(),
        "external-harness-ref"
    );
    assert_ne!(
        native.primary.profile.credential, external.primary.profile.credential,
        "the lanes must not share a credential reference"
    );
    assert_ne!(
        native.primary.profile.runtime,
        external.primary.profile.runtime
    );
}
