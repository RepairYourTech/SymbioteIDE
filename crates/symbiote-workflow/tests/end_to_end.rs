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

/// The demo's fixture tool: the command the fixture transport proposes and
/// the sandboxed shell executor runs inside the reserved worktree.
const FIXTURE_TOOL_COMMAND: &str = "printf worker-output > produced.txt";

/// A tool that asks its own process what it is bound by: the address-space
/// ceiling the dispatch's declared limits reduce to, and whether an allocation
/// under that ceiling and one past it are granted. The answers are written into
/// the worktree, so the host reads what the dispatch's own process observed.
const BOUND_PROBE_COMMAND: &str = r#"printf '%s' "$(ulimit -v)" > observed.txt; /usr/bin/python3 - <<'PY'
import json, resource
soft, hard = resource.getrlimit(resource.RLIMIT_AS)
def probe(n):
    try:
        return len(bytearray(n))
    except MemoryError:
        return None
json.dump({"soft": soft, "under": probe(64 << 20), "over": probe(400 << 20)}, open("probe.json", "w"))
PY
"#;

/// A harness program that speaks the pinned App Server framing, writes its
/// work inside the reserved worktree, and reports its OWN address-space
/// ceiling (the soft `RLIMIT_AS`) in its agent message: the Host reads what
/// the harness process observed of the bound the dispatch declared, and the
/// worktree evidence reads what the harness actually produced. Launched by the
/// Host inside the sandbox, so the bound is the operator's launcher's, not this
/// script's.
const HARNESS_RESPONDER: &str = r#"import json,sys,resource
soft,_=resource.getrlimit(resource.RLIMIT_AS)
def send(o):
    sys.stdout.write(json.dumps(o)+"\n"); sys.stdout.flush()
for line in sys.stdin:
    line=line.strip()
    if not line: continue
    try: msg=json.loads(line)
    except Exception: continue
    if "id" not in msg: continue
    m=msg.get("method")
    if m=="initialize":
        send({"id":msg["id"],"result":{"userAgent":"symbiote/0.118.0 (Linux)"}})
    elif m=="thread/start":
        send({"id":msg["id"],"result":{"thread":{"id":"thr-bounded"}}})
    elif m=="turn/start":
        send({"id":msg["id"],"result":{"turn":{"id":"turn-bounded"}}})
        open("produced.txt","w").write("the external harness wrote its worktree\n")
        send({"method":"item/completed","params":{"threadId":"thr-bounded","turnId":"turn-bounded","item":{"type":"agentMessage","id":"i1","text":"harness address-space ceiling %d"%soft}}})
        send({"method":"turn/completed","params":{"threadId":"thr-bounded","turnId":"turn-bounded","turn":{"id":"turn-bounded","status":"completed","items":[]}}})
"#;

/// The operator configuration for the demo: reservation base, the
/// explicitly labeled fixture model transport proposing one shell tool,
/// the fixture credential registration, and the shell program allowlist.
/// 0600 — the load refuses anything looser.
fn write_operator_config(
    path: &std::path::Path,
    reservation_base: &std::path::Path,
    launcher: &std::path::Path,
    tool_command: &str,
    with_external_fixture: bool,
    external_harness: Option<(&str, &[String])>,
) {
    let mut config = serde_json::json!({
        "reservation_base": reservation_base.display().to_string(),
        "native_fixture": {
            "echo_text": symbiote_workflow::demo::FIXTURE_REPORT,
            "tool": {"call_id": "call-produce",
                "arguments": {"program": "sh",
                    "arguments": ["-c", tool_command]}}
        },
        "credential_broker": [
            {"reference": "native-vault-ref",
             "project": "staffing-demo", "environment": "OPENAI_API_KEY",
             "value": "fixture-not-a-real-secret"},
            {"reference": "b-vault-ref",
             "project": "project-isolation-b", "environment": "OPENAI_API_KEY",
             "value": "fixture-b-not-a-real-secret"}
        ],
        "shell_executor": {"launcher_path": launcher.display().to_string(),
            "protected_paths": ["/etc", "/var", "/home"],
            "allowed_programs": ["sh"]}
    });
    if with_external_fixture {
        config["external_fixture"] = serde_json::json!({"thread_id": "thr-demo-external",
            "agent_message": "external harness implemented the bounded change"});
    }
    if let Some((program, args)) = external_harness {
        config["external_harness"] = serde_json::json!({
            "launcher_path": launcher.display().to_string(),
            "protected_paths": ["/etc", "/var", "/home"],
            "program": program,
            "args": args,
        });
    }
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
    /// A SECOND real repository for the Project-isolation demo: two
    /// projects, each rooted at its own repository, on ONE daemon.
    repo_b: PathBuf,
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
    let repo = make_repo(&scratch.join("repo"), "demo repository\n");
    let repo_b = make_repo(&scratch.join("repo-b"), "second project repository\n");
    let config_path = scratch.join("operator-config.json");
    write_operator_config(
        &config_path,
        &reservation_base,
        &launcher_binary(),
        FIXTURE_TOOL_COMMAND,
        true,
        None,
    );
    DemoEnv {
        scratch,
        state_dir,
        reservation_base,
        repo,
        repo_b,
        config_path,
    }
}

/// A real one-commit git repository with one file — the Root placement
/// each project observes.
fn make_repo(path: &std::path::Path, readme: &str) -> PathBuf {
    std::fs::create_dir_all(path).unwrap();
    git(path, &["init", "-q", "-b", "main"]);
    std::fs::write(path.join("README.md"), readme).unwrap();
    git(path, &["add", "."]);
    git(
        path,
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
    path.to_path_buf()
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
        Some(symbiote_workflow::demo::FIXTURE_REPORT)
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
        Some(symbiote_workflow::demo::FIXTURE_REPORT)
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
        Some(symbiote_workflow::demo::FIXTURE_REPORT)
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

/// The no-fallback pin (review P2): with an operator config that does NOT
/// provision the external execution path, the Host record must not
/// advertise `ExternalHarness`, so the external lane's preparation records
/// a durable refusal — while the native lane still works. A future
/// refactor that advertises both runtimes unconditionally fails here
/// instead of silently reintroducing a native fallback for external
/// execution (the thing the runtime-kind contract forbids).
#[test]
fn an_external_binding_refuses_to_start_without_operator_provisioned_harness_support() {
    let env = demo_environment("no-external");
    let state_dir = &env.state_dir;
    let reservation_base = &env.reservation_base;
    let repo = &env.repo;
    let config_path = &env.config_path;

    // Overwrite the shared config with one that does NOT provision the
    // external execution path.
    write_operator_config(
        config_path,
        reservation_base,
        &launcher_binary(),
        FIXTURE_TOOL_COMMAND,
        false,
        None,
    );
    // The daemon stays alive for the whole test; its Drop cleans up.
    let _daemon = Daemon::spawn(state_dir, config_path);
    let mut workflow = symbiote_workflow::DemoWorkflow::connect(state_dir, reservation_base, repo)
        .expect("connect");
    // The native lane is unaffected: no harness support is needed for it.
    let native_dispatch = workflow.start_demo().expect("native lane works");
    assert!(!native_dispatch.is_empty());
    // The external lane refuses AT THE DAEMON with a typed refusal: the
    // start recompiles the dispatch against the live Host record, whose
    // supported runtimes exclude the harness. It is never rerouted to the
    // native runtime. (Preparation is host-agnostic by design, so it is
    // here that the refusal must surface.)
    let error = workflow
        .start_demo_external()
        .expect_err("external start must refuse without provisioning");
    match error {
        symbiote_workflow::WorkflowError::Refused(protocol_error) => {
            // The domain's IneligibleHost refusal currently maps to the
            // coarse invalid_request wire code; the meaningful pin is that
            // the START REFUSES — the lane is never rerouted to native.
            assert!(
                matches!(
                    protocol_error.code,
                    symbiote_protocol::ErrorCode::InvalidRequest
                        | symbiote_protocol::ErrorCode::FailedPrecondition
                ),
                "the start must refuse, got {:?}: {}",
                protocol_error.code,
                protocol_error.message
            );
        }
        other => panic!("the start must refuse at the daemon, got {other:?}"),
    }
}

/// The Project-isolation demo (first-release definition of done): TWO
/// projects on ONE daemon — each with its own repository, team, bindings,
/// task, and stream — with the isolation between them made observable and
/// pinned: project-scoped reads refuse foreign ids, each project's
/// journal and worktree namespace never cross, and a credential
/// registered for one project is REFUSED when another project's dispatch
/// references it (the broker's owning-project check) while the correct
/// per-project credential works. The model turns are the operator's
/// explicitly labeled fixture transports; nothing is spent and no live
/// turn is attempted.
#[test]
fn two_projects_on_one_daemon_without_work_configuration_or_credential_leakage() {
    let env = demo_environment("two-projects");
    let state_dir = &env.state_dir;
    let reservation_base = &env.reservation_base;
    let config_path = &env.config_path;

    // The daemon stays alive for the whole test; its Drop cleans up.
    let _daemon = Daemon::spawn(state_dir, config_path);
    let mut workflow =
        symbiote_workflow::DemoWorkflow::connect(state_dir, reservation_base, &env.repo)
            .expect("connect")
            .with_repository("project-isolation-b", &env.repo_b);

    // Lane A — the staffing project, full run (its worktree really
    // produces the file through the sandboxed shell executor).
    let a_dispatch = workflow.start_demo().expect("lane A start");
    let a = workflow.finish_demo(&a_dispatch).expect("lane A run");
    assert_eq!(a.task_state, "completion_requested");
    assert!(a.worktree.contains("produced.txt"), "{:?}", a.worktree);

    // Lane B, leak attempt: the binding references the credential
    // PROJECT A registered. The gate (UseCredential) is satisfied — the
    // broker's owning-project check is what must refuse.
    let b_leak = b_leak_lane();
    let leak_dispatch = workflow
        .start_demo_lane(&b_leak)
        .expect("lane B (leak) start");
    let leak_error = workflow
        .finish_demo_lane(&b_leak, &leak_dispatch)
        .expect_err("a foreign project's credential must be refused");
    match leak_error {
        symbiote_workflow::WorkflowError::Refused(protocol_error) => {
            assert_eq!(
                protocol_error.code,
                symbiote_protocol::ErrorCode::FailedPrecondition,
                "{}",
                protocol_error.message
            );
            assert_eq!(
                protocol_error.message, "credential lease refused: cross_project_denied",
                "the refusal must be the broker's owning-project check"
            );
        }
        other => panic!("the run must refuse at the daemon, got {other:?}"),
    }
    // The refusal left the dispatch intact for Host retry policy: the
    // leak task stays Running, and the binding replacement does not
    // rewire its already-compiled contract.
    let leak_task = driver_reads_task(state_dir, "b-leak-task", "project-isolation-b");
    assert_eq!(leak_task, "running");

    // Lane B, correct configuration: the project's OWN credential. This
    // lane's worktree cannot exist yet — project A's run never created
    // anything under project B's derived namespace.
    let b_ok = b_ok_lane();
    let b_worktree_before = symbiote_workflow::observe_worktree_evidence(
        reservation_base,
        &symbiote_domain::ProjectId::new(b_ok.project).unwrap(),
        &symbiote_domain::RootId::new(b_ok.root).unwrap(),
        &symbiote_domain::ChangeStreamId::new(b_ok.stream).unwrap(),
    );
    assert!(
        b_worktree_before.is_err(),
        "project B's worktree must not exist before its own run"
    );
    // The operator fixes the configuration: replace the binding with the
    // project's own credential reference (revision-CAS replacement).
    workflow
        .replace_lane_binding(&b_ok, "b-vault-ref")
        .expect("binding replacement");
    let b_dispatch = workflow
        .start_followon_lane(&b_ok, false)
        .expect("lane B (ok) start");
    let b = workflow
        .finish_demo_lane(&b_ok, &b_dispatch)
        .expect("lane B run with its own credential");
    assert_eq!(b.task_state, "completion_requested");
    assert!(b.worktree.contains("produced.txt"), "{:?}", b.worktree);
    assert_ne!(
        a.worktree.worktree, b.worktree.worktree,
        "the projects must not share a worktree"
    );

    // Project-scoped reads: foreign ids are unreachable, not hidden.
    let project_a = symbiote_domain::ProjectId::new("staffing-demo").unwrap();
    let project_b = symbiote_domain::ProjectId::new("project-isolation-b").unwrap();
    let mut driver = symbiote_workflow::Driver::connect(state_dir).expect("driver");
    let foreign_task_a = driver
        .call(
            "iso-task-a-read",
            serde_json::json!({"kind":"get_task","project_id":"staffing-demo",
            "task_id":"b-task"}),
            Some(&project_a),
        )
        .expect_err("project A must not see project B's task");
    // Task ids live in one global namespace: a foreign read is DENIED
    // (PermissionDenied) rather than reported absent — cross-project
    // existence is never revealed.
    assert_eq!(
        refused_code(foreign_task_a),
        symbiote_protocol::ErrorCode::PermissionDenied
    );
    let foreign_task_b = driver
        .call(
            "iso-task-b-read",
            serde_json::json!({"kind":"get_task","project_id":"project-isolation-b",
            "task_id":"staffing-task"}),
            Some(&project_b),
        )
        .expect_err("project B must not see project A's task");
    assert_eq!(
        refused_code(foreign_task_b),
        symbiote_protocol::ErrorCode::PermissionDenied
    );
    let foreign_binding = driver
        .call(
            "iso-binding-read",
            serde_json::json!({"kind":"get_binding","project_id":"staffing-demo",
            "binding_id":"b-binding"}),
            Some(&project_a),
        )
        .expect_err("project A must not see project B's binding");
    // Binding reads are (project, id)-scoped: a foreign id is simply
    // absent from the requesting project's namespace.
    assert_eq!(
        refused_code(foreign_binding),
        symbiote_protocol::ErrorCode::NotFound
    );

    // Journal scoping: each project's evidence trail carries only its own
    // work. Page both journals fully and check the absence of the other
    // project's identities.
    let journals = [
        ("staffing-demo", &project_a, "project-isolation-b"),
        ("project-isolation-b", &project_b, "staffing-demo"),
    ];
    for (name, project, foreign) in journals {
        let mut cursor = 0u64;
        let mut blob = String::new();
        loop {
            let page = driver
                .call(
                    &format!("iso-journal-{name}-{cursor}"),
                    serde_json::json!({"kind":"read_journal","project_id":name,
                    "after":cursor,"limit":100}),
                    Some(project),
                )
                .expect("journal page");
            let ResponseBody::Journal(page) = page else {
                panic!("journal page body");
            };
            cursor = page.next_cursor.0;
            for event in &page.events {
                blob.push_str(&serde_json::to_string(event).expect("event serializes"));
            }
            if !page.has_more {
                break;
            }
        }
        assert!(
            !blob.contains(foreign),
            "project {name}'s journal must not carry project {foreign}'s identities"
        );
    }
}

/// Reads one task's state over the wire through a fresh driver.
fn driver_reads_task(state_dir: &std::path::Path, task: &str, project: &str) -> String {
    let mut driver = symbiote_workflow::Driver::connect(state_dir).expect("driver");
    let project = symbiote_domain::ProjectId::new(project).unwrap();
    let read = driver
        .call(
            &format!("iso-task-read-{task}"),
            serde_json::json!({"kind":"get_task","project_id":project,
            "task_id":task}),
            Some(&project),
        )
        .expect("task read");
    let ResponseBody::Task(task) = read else {
        panic!("task read body");
    };
    serde_json::to_value(task.state())
        .ok()
        .and_then(|value| value.as_str().map(str::to_owned))
        .expect("task state")
}

/// Project B's leak-attempt lane: the binding references PROJECT A's
/// registered credential (the broker must refuse it).
fn b_leak_lane() -> symbiote_workflow::demo::DemoLane {
    symbiote_workflow::demo::DemoLane {
        project: "project-isolation-b",
        root: "isolation-b-root",
        objective: "b-leak-objective",
        task: "b-leak-task",
        stream: "b-leak-stream",
        chat: "chat-b-leak",
        lead_role: "lead-b",
        role: "engineer-b",
        binding_id: "b-leak-binding",
        provider: "b-openai",
        model: "b-coding-model",
        entitlement: "b-entitlement",
        credential: "native-vault-ref",
        with_external_lane: false,
    }
}

/// Project B's corrected lane: the project's OWN registered credential,
/// on the SAME (replaced) binding.
fn b_ok_lane() -> symbiote_workflow::demo::DemoLane {
    symbiote_workflow::demo::DemoLane {
        project: "project-isolation-b",
        root: "isolation-b-root",
        objective: "b-objective",
        task: "b-task",
        stream: "b-stream",
        chat: "chat-b",
        lead_role: "lead-b",
        role: "engineer-b",
        binding_id: "b-leak-binding",
        provider: "b-openai",
        model: "b-coding-model",
        entitlement: "b-entitlement",
        credential: "b-vault-ref",
        with_external_lane: false,
    }
}

/// The typed wire code a driver refusal carries.
fn refused_code(error: symbiote_workflow::WorkflowError) -> symbiote_protocol::ErrorCode {
    match error {
        symbiote_workflow::WorkflowError::Refused(protocol_error) => protocol_error.code,
        other => panic!("expected a daemon refusal, got {other:?}"),
    }
}

/// The positions-restart headroom both review records named (#518/#519),
/// in its full form: a driver tracking TWO projects — with two lanes on
/// the second — across a daemon SIGKILL. Positions are serialized to disk
/// exactly as a desktop shell would; a FRESH driver is restored from that
/// file; BOTH projects' cursors are restored exactly; and both lanes
/// finish on the restarted daemon with their reports still lane-scoped.
#[test]
fn positions_track_two_projects_and_both_lanes_across_a_daemon_crash() {
    let env = demo_environment("positions-two-projects");
    let state_dir = &env.state_dir;
    let reservation_base = &env.reservation_base;
    let config_path = &env.config_path;
    let positions_path = env.scratch.join("driver-positions.json");

    // Generation 1: compose both projects, start both lanes, read both
    // journals, persist the positions, and SIGKILL the daemon.
    let mut daemon = Daemon::spawn(state_dir, config_path);
    let mut workflow =
        symbiote_workflow::DemoWorkflow::connect(state_dir, reservation_base, &env.repo)
            .expect("connect")
            .with_repository("project-isolation-b", &env.repo_b);
    let a_dispatch = workflow.start_demo().expect("lane A start");
    let b_leak_dispatch = workflow
        .start_demo_lane(&b_leak_lane())
        .expect("lane B (leak) start");
    let _refused = workflow
        .finish_demo_lane(&b_leak_lane(), &b_leak_dispatch)
        .expect_err("the leak must refuse in generation 1");
    workflow
        .replace_lane_binding(&b_ok_lane(), "b-vault-ref")
        .expect("binding replacement");
    let b_dispatch = workflow
        .start_followon_lane(&b_ok_lane(), false)
        .expect("lane B (ok) start");
    let a_cursor = workflow.read_journal().expect("journal A read");
    let b_cursor = workflow
        // Both lanes share project B; the lane choice only names the project.
        .read_journal_lane(&b_ok_lane())
        .expect("journal B read");
    assert!(a_cursor > 0, "project A's evidence trail advanced");
    assert!(b_cursor > 0, "project B's evidence trail advanced");
    std::fs::write(
        &positions_path,
        serde_json::to_vec(&workflow.positions()).expect("serialize positions"),
    )
    .unwrap();
    let state_dir_after_kill = daemon.kill9();

    // Generation 2: a fresh daemon and a FRESH driver reconstructed from
    // the serialized positions — no memory shared with the dead one.
    let _daemon = Daemon::spawn(&state_dir_after_kill, config_path);
    let restored: Vec<symbiote_client_sdk::JournalPosition> =
        serde_json::from_slice(&std::fs::read(&positions_path).unwrap())
            .expect("restore serialized positions");
    // Both projects are tracked: a restore that lost either key would
    // default that project's cursor to 0 and fail the exact-equality
    // asserts below — this pins the file's SHAPE too.
    assert_eq!(restored.len(), 2, "one position per tracked project");
    let mut workflow =
        symbiote_workflow::DemoWorkflow::connect(state_dir, reservation_base, &env.repo)
            .expect("connect")
            .with_repository("project-isolation-b", &env.repo_b)
            .with_positions(restored);
    let project_a = symbiote_domain::ProjectId::new("staffing-demo").unwrap();
    let project_b = symbiote_domain::ProjectId::new("project-isolation-b").unwrap();
    assert_eq!(
        workflow.journal_position(&project_a).0,
        a_cursor,
        "project A's cursor must be restored exactly"
    );
    assert_eq!(
        workflow.journal_position(&project_b).0,
        b_cursor,
        "project B's cursor must be restored exactly"
    );
    // Both lanes finish on the restarted daemon, reports still scoped.
    let a = workflow
        .finish_demo(&a_dispatch)
        .expect("lane A after restart");
    assert_eq!(a.task_state, "completion_requested");
    assert_eq!(
        a.report.as_deref(),
        Some(symbiote_workflow::demo::FIXTURE_REPORT)
    );
    let b = workflow
        .finish_demo_lane(&b_ok_lane(), &b_dispatch)
        .expect("lane B after restart");
    assert_eq!(b.task_state, "completion_requested");
    // Both native lanes read the ONE global fixture factory, so the
    // report TEXTS are equal by fixture design — that is not leakage.
    // What proves the projects' work does not cross is the per-task
    // journal filter above and the distinct worktrees the runs produced
    // in (asserted below, and in the Project-isolation test).
    assert_eq!(
        b.report.as_deref(),
        Some(symbiote_workflow::demo::FIXTURE_REPORT)
    );
    assert_ne!(a.worktree.worktree, b.worktree.worktree);
}

/// A started dispatch's execution is bound by the limits it declared: the
/// process the dispatch's own tool runs in carries the candidate's declared
/// memory ceiling as a real kernel limit, an allocation under it is granted,
/// and one past it fails even though this machine has far more memory than the
/// declaration. The number asserted is read from the same committed fixture the
/// demo composes the staffing candidate from, so a bound that drifts from the
/// declaration fails here.
#[test]
fn a_started_dispatch_runs_its_tools_under_the_declared_memory_bound() {
    let env = demo_environment("declared-bound");
    let state_dir = &env.state_dir;
    let reservation_base = &env.reservation_base;
    let config_path = &env.config_path;
    // The declared ceiling, straight from the fixture the demo staffs from.
    let declared: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/workforce-bindings/configure.json"
    ))
    .expect("binding fixture");
    let declared = declared["operation"]["configuration"]["primary"]["limits"]["max_memory_bytes"]
        .as_u64()
        .expect("the fixture candidate declares a memory limit");
    // The operator's fixture transport proposes the probe tool instead of the
    // demo's producer: the rest of the composition is unchanged.
    write_operator_config(
        config_path,
        reservation_base,
        &launcher_binary(),
        BOUND_PROBE_COMMAND,
        true,
        None,
    );

    let _daemon = Daemon::spawn(state_dir, config_path);
    let mut workflow =
        symbiote_workflow::DemoWorkflow::connect(state_dir, reservation_base, &env.repo)
            .expect("connect");
    let dispatch_id = workflow.start_demo().expect("demo start half");
    let outcome = workflow
        .finish_demo(&dispatch_id)
        .expect("demo finish half");

    let observed: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(outcome.worktree.worktree.join("probe.json"))
            .expect("the tool's own observation of its bound"),
    )
    .expect("probe json");
    assert_eq!(
        observed["soft"].as_u64(),
        Some(declared),
        "the tool process carries the dispatch's declared memory ceiling: {observed}"
    );
    assert_eq!(
        observed["under"].as_u64(),
        Some(64 << 20),
        "an allocation under the declared ceiling is granted: {observed}"
    );
    assert_eq!(
        observed["over"],
        serde_json::Value::Null,
        "an allocation past the declared ceiling fails, though the machine has far more memory: {observed}"
    );
}

/// The external lane's own process carries the ceiling its dispatch declared,
/// and its work lands in its own reserved worktree: the operator-provisioned
/// harness is launched by the Host inside the sandbox under the recorded
/// `max_memory_bytes`, the harness reports its own soft `RLIMIT_AS` in the
/// completion report the Host reads back, and the file it wrote is visible in
/// the worktree evidence. The lane's readiness answer promises this bound, so
/// it must hold for the process the lane actually starts — not only for the
/// native lane's tools.
#[test]
fn a_started_external_harness_runs_under_the_declared_memory_bound() {
    let env = demo_environment("external-bound");
    let state_dir = &env.state_dir;
    let reservation_base = &env.reservation_base;
    let config_path = &env.config_path;
    // The declared ceiling, straight from the fixture the demo staffs from.
    let declared: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/workforce-bindings/configure.json"
    ))
    .expect("binding fixture");
    let declared = declared["operation"]["configuration"]["primary"]["limits"]["max_memory_bytes"]
        .as_u64()
        .expect("the fixture candidate declares a memory limit");
    // The external lane runs the operator's harness program (a scripted
    // responder that reports its own bound) inside the sandbox; the native
    // lane's composition is unchanged.
    write_operator_config(
        config_path,
        reservation_base,
        &launcher_binary(),
        FIXTURE_TOOL_COMMAND,
        false,
        Some((
            "/usr/bin/python3",
            &["-c".to_owned(), HARNESS_RESPONDER.to_owned()],
        )),
    );

    let _daemon = Daemon::spawn(state_dir, config_path);
    let mut workflow =
        symbiote_workflow::DemoWorkflow::connect(state_dir, reservation_base, &env.repo)
            .expect("connect");
    // The shared project/team/root placement comes from the open half; the
    // external lane is its own binding, task and worktree.
    let _native = workflow.start_demo().expect("contract start half");
    let dispatch_id = workflow.start_demo_external().expect("external start half");
    let outcome = workflow
        .finish_demo_external(&dispatch_id)
        .expect("external run");

    assert_eq!(outcome.task_state, "completion_requested");
    assert_eq!(
        outcome.report.as_deref(),
        Some(format!("harness address-space ceiling {declared}").as_str()),
        "the harness process reports the dispatch's declared memory ceiling"
    );
    // The harness's work landed in the dispatch's own reserved worktree: the
    // binding grants stream mutation, so the Host mounts that worktree
    // writable and the harness's own sandbox policy is writable too. A lane
    // launched read-only could only report, never produce.
    assert!(
        outcome.worktree.contains("produced.txt"),
        "the external harness's produced file must be visible in its worktree: {:?}",
        outcome.worktree
    );
}
