//! The desktop controller: the headless core the Tauri commands call.
//! It owns one daemon generation at a time plus the connected driver,
//! persists the driver's journal positions across restarts (the desktop
//! restarts, the daemon crashes — both resume from the same file), and
//! sequences the first-release demonstration flow. This module is plain
//! Rust with no Tauri types, so the whole UI-facing behavior is proven
//! against a real daemon in headless tests.
//!
//! Boundary: the controller grants nothing and completes nothing — worker
//! completion is evidence, and Host verification plus independent review
//! remain the completion gates. The model turn in the demo configuration
//! is the operator's explicitly labeled fixture transport; a live one
//! stays gated on explicit user authorization for credentials and billing.
use std::{
    io::Write,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};
use symbiote_workflow::{DemoOutcome, DemoWorkflow, WorkflowError};

/// The daemon binaries the controller spawns. In development they come
/// from the workspace target directory; a bundled release ships them as
/// sidecars next to the app executable (the same directory the UI's
/// `bin_dir` names).
pub struct DesktopPaths {
    pub daemon: PathBuf,
    pub sandbox_launcher: PathBuf,
}

impl DesktopPaths {
    /// Resolves the daemon and sandbox launcher from one directory.
    pub fn from_directory(bin_dir: &Path) -> Result<Self, DesktopError> {
        let daemon = bin_dir.join("symbioted");
        let sandbox_launcher = bin_dir.join("symbiote-sandbox-launch");
        for path in [&daemon, &sandbox_launcher] {
            if !path.is_file() {
                return Err(DesktopError::MissingBinary(path.display().to_string()));
            }
        }
        Ok(Self {
            daemon,
            sandbox_launcher,
        })
    }
}

#[derive(Debug)]
pub enum DesktopError {
    /// A required daemon binary was not found in the given directory.
    MissingBinary(String),
    /// The named repository path did not observe as a git work tree.
    NotARepository,
    /// Local filesystem/socket setup failed before anything was sent.
    Setup(String),
    /// The workflow step failed; the typed error is preserved.
    Workflow(WorkflowError),
    /// The daemon exited before its socket was ready; the child's exit
    /// status is surfaced (exit code or signal) instead of discarded.
    DaemonExited(String),
    /// The daemon's socket never became ready.
    DaemonUnready,
    /// The persisted journal positions file exists but does not parse.
    /// Surfaced — never silently ignored: silently resetting it would
    /// discard the resume point the whole restart story depends on.
    CorruptPositions(String),
}

impl std::fmt::Display for DesktopError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingBinary(path) => write!(f, "daemon binary missing: {path}"),
            Self::NotARepository => write!(f, "repository is not a git work tree"),
            Self::Setup(message) => write!(f, "desktop setup failed: {message}"),
            Self::Workflow(error) => write!(f, "{error}"),
            Self::DaemonExited(status) => {
                write!(f, "daemon exited before becoming ready: {status}")
            }
            Self::DaemonUnready => write!(f, "daemon socket never became ready"),
            Self::CorruptPositions(error) => {
                write!(
                    f,
                    "persisted journal positions are corrupt (delete or restore the file, never silently reset): {error}"
                )
            }
        }
    }
}
impl std::error::Error for DesktopError {}

impl From<WorkflowError> for DesktopError {
    fn from(error: WorkflowError) -> Self {
        Self::Workflow(error)
    }
}

/// One desktop generation: environment on disk, one daemon child, one
/// connected driver. `begin_generation` is idempotent across restarts —
/// call it after a crash exactly like after a boot; the persisted
/// positions make both resumes identical.
pub struct DesktopController {
    paths: DesktopPaths,
    state_dir: PathBuf,
    reservation_base: PathBuf,
    repository: PathBuf,
    daemon: Option<Child>,
    workflow: Option<DemoWorkflow>,
}

impl DesktopController {
    /// Prepares the private environment: a 0700 state directory, the
    /// reservation base, and the operator config (0600) naming the sandbox
    /// launcher. The repository must observe as a real git work tree.
    pub fn new(
        paths: DesktopPaths,
        state_dir: &Path,
        reservation_base: &Path,
        repository: &Path,
    ) -> Result<Self, DesktopError> {
        std::fs::create_dir_all(state_dir)
            .map_err(|error| DesktopError::Setup(format!("state directory: {error}")))?;
        std::fs::set_permissions(state_dir, std::fs::Permissions::from_mode(0o700))
            .map_err(|error| DesktopError::Setup(format!("state directory mode: {error}")))?;
        std::fs::create_dir_all(reservation_base)
            .map_err(|error| DesktopError::Setup(format!("reservation base: {error}")))?;
        // The repository must be real before anything is registered: the
        // same observation the Host's placement operation will enforce.
        let mut git = symbiote_repo::SystemGit::new();
        if symbiote_repo::observe_head(&mut git, repository).is_err() {
            return Err(DesktopError::NotARepository);
        }
        let config_path = state_dir.join("operator-config.json");
        write_operator_config(&config_path, reservation_base, &paths.sandbox_launcher)?;
        Ok(Self {
            paths,
            state_dir: state_dir.to_path_buf(),
            reservation_base: reservation_base.to_path_buf(),
            repository: repository.to_path_buf(),
            daemon: None,
            workflow: None,
        })
    }

    /// Spawns the daemon (first boot or after a crash), waits for its
    /// socket, connects the driver, and restores the persisted journal
    /// positions. A fresh environment bootstraps at cursor 0; a restart
    /// resumes exactly where the previous generation stopped.
    pub fn begin_generation(&mut self) -> Result<(), DesktopError> {
        if self.daemon.is_some() {
            return Ok(());
        }
        let mut child = Command::new(&self.paths.daemon)
            .arg("--state-dir")
            .arg(&self.state_dir)
            .arg("--operator-config")
            .arg(self.state_dir.join("operator-config.json"))
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| DesktopError::Setup(format!("daemon spawn: {error}")))?;
        let socket = self.state_dir.join("host.sock");
        let deadline = Instant::now() + Duration::from_secs(15);
        loop {
            if socket.exists() && std::os::unix::net::UnixStream::connect(&socket).is_ok() {
                break;
            }
            // Every observation of the child reaps it before returning:
            // no early return between spawn and ownership transfer can
            // leave a daemon behind.
            match child.try_wait() {
                Ok(Some(status)) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(DesktopError::DaemonExited(status.to_string()));
                }
                Ok(None) => {}
                Err(error) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(DesktopError::Setup(format!("daemon status: {error}")));
                }
            }
            if Instant::now() > deadline {
                let _ = child.kill();
                let _ = child.wait();
                return Err(DesktopError::DaemonUnready);
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        // Connect before the controller owns the child: a connect failure
        // takes the just-spawned daemon down with it instead of leaving a
        // detached process behind.
        let workflow = match DemoWorkflow::connect(
            &self.state_dir,
            &self.reservation_base,
            &self.repository,
        ) {
            Ok(workflow) => workflow,
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(error.into());
            }
        };
        // Restore the persisted journal positions. A corrupt file is an
        // ERROR, never a silent reset — and every failure path here reaps
        // the just-spawned daemon: the controller must not own a child it
        // is not returning with.
        let restored = match self.read_positions_file() {
            Ok(restored) => restored,
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(error);
            }
        };
        self.workflow = Some(match restored {
            Some(positions) => workflow.with_positions(positions),
            None => workflow,
        });
        self.daemon = Some(child);
        Ok(())
    }

    /// The first half of the demonstration: open the repository, describe
    /// the task, and START the dispatch. Everything is journaled — this
    /// survives a daemon SIGKILL, which is the restart/resume proof.
    pub fn start_demo(&mut self) -> Result<String, DesktopError> {
        let workflow = self.workflow()?;
        let dispatch_id = workflow.start_demo()?;
        self.persist_positions()?;
        Ok(dispatch_id)
    }

    /// Follows the durable evidence trail to the head, advancing and
    /// persisting the driver's tracked positions.
    pub fn read_journal(&mut self) -> Result<u64, DesktopError> {
        let cursor = self.workflow()?.read_journal()?;
        self.persist_positions()?;
        Ok(cursor)
    }

    /// The second half: execute the started dispatch (real provisioning,
    /// real sandboxed tool execution, the labeled fixture model turn) and
    /// read the completion evidence and worktree diff. Works across a
    /// daemon restart — possibly across a crash.
    pub fn finish_demo(&mut self, dispatch_id: &str) -> Result<DemoOutcome, DesktopError> {
        let outcome = self.workflow()?.finish_demo(dispatch_id)?;
        self.persist_positions()?;
        Ok(outcome)
    }

    /// The driver's persisted journal positions (what a desktop restart
    /// restores).
    pub fn journal_position(&self) -> u64 {
        self.workflow
            .as_ref()
            .map(|workflow| {
                let project =
                    symbiote_domain::ProjectId::new(symbiote_workflow::demo::PROJECT.to_string())
                        .expect("fixture project");
                workflow.journal_position(&project).0
            })
            .unwrap_or(0)
    }

    /// Graceful shutdown: the daemon exits cleanly and the driver drops.
    pub fn stop(&mut self) {
        if let Some(workflow) = self.workflow.as_mut() {
            let _ = workflow.shutdown();
        }
        self.workflow = None;
        let mut child = match self.daemon.take() {
            Some(child) => child,
            None => return,
        };
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            match child.try_wait() {
                Ok(Some(_)) => break,
                Ok(None) if Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(50));
                }
                _ => {
                    let _ = child.kill();
                    let _ = child.wait();
                    break;
                }
            }
        }
    }

    /// The crash path — the daemon dies exactly like a crashed process
    /// (SIGKILL, no graceful anything); the next [`Self::begin_generation`]
    /// is the restart. Production code never calls this; the crash test
    /// and the crash-evidence story do.
    pub fn crash_daemon(&mut self) {
        if let Some(child) = self.daemon.as_mut() {
            // std's Child::kill is SIGKILL on Unix: exactly a crashed process.
            let _ = child.kill();
            let _ = child.wait();
        }
        self.daemon = None;
        self.workflow = None;
    }

    /// Bounded diff evidence for a finished run's worktree. The
    /// requested path must resolve inside THIS controller's reservation
    /// base — the UI is the owner surface, but the boundary stays
    /// structural rather than trusting the front end's path strings.
    pub fn run_diff(&self, worktree: &str) -> Result<symbiote_repo::RunDiff, DesktopError> {
        let requested = std::fs::canonicalize(worktree)
            .map_err(|error| DesktopError::Setup(format!("worktree resolve: {error}")))?;
        let base = std::fs::canonicalize(&self.reservation_base)
            .map_err(|error| DesktopError::Setup(format!("reservation base: {error}")))?;
        if !requested.starts_with(&base) {
            return Err(DesktopError::Setup(
                "worktree outside this session's reservation base".into(),
            ));
        }
        let mut git = symbiote_repo::SystemGit::new();
        let status = symbiote_repo::observe_status(&mut git, &requested)
            .map_err(|error| DesktopError::Setup(error.to_string()))?;
        // Collecting the diff itself is total: every degradation is inside
        // the returned RunDiff (flags and notes), so a partly unreadable
        // worktree still previews what it can.
        Ok(symbiote_repo::observe_run_diff(
            &mut git, &requested, &status,
        ))
    }

    fn workflow(&mut self) -> Result<&mut DemoWorkflow, DesktopError> {
        self.workflow.as_mut().ok_or(DesktopError::Setup(
            "no connected generation; begin one".into(),
        ))
    }

    fn positions_path(&self) -> PathBuf {
        self.state_dir.join("desktop-positions.json")
    }

    /// The persisted journal positions, if a file exists. A file that
    /// exists but does not parse is an ERROR — silently treating it as
    /// absent would reset the resume point the whole restart story
    /// depends on.
    fn read_positions_file(
        &self,
    ) -> Result<Option<Vec<symbiote_client_sdk::JournalPosition>>, DesktopError> {
        let bytes = match std::fs::read(self.positions_path()) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => {
                return Err(DesktopError::Setup(format!("positions read: {error}")));
            }
        };
        match serde_json::from_slice(&bytes) {
            Ok(positions) => Ok(Some(positions)),
            Err(error) => Err(DesktopError::CorruptPositions(error.to_string())),
        }
    }

    fn persist_positions(&mut self) -> Result<(), DesktopError> {
        let positions = self
            .workflow
            .as_ref()
            .map(|workflow| workflow.positions())
            .unwrap_or_default();
        let bytes = serde_json::to_vec(&positions)
            .map_err(|error| DesktopError::Setup(format!("positions: {error}")))?;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(self.positions_path())
            .map_err(|error| DesktopError::Setup(format!("positions file: {error}")))?;
        file.write_all(&bytes)
            .map_err(|error| DesktopError::Setup(format!("positions file: {error}")))?;
        Ok(())
    }
}

impl Drop for DesktopController {
    fn drop(&mut self) {
        // Never leave a daemon behind: stop() bounds its wait and falls
        // back to the crash path, so the guarantee is real.
        self.stop();
    }
}

fn write_operator_config(
    path: &Path,
    reservation_base: &Path,
    launcher: &Path,
) -> Result<(), DesktopError> {
    let config = serde_json::json!({
        "reservation_base": reservation_base.display().to_string(),
        "native_fixture": {
            "echo_text": symbiote_workflow::demo::FIXTURE_REPORT,
            "tool": {"call_id": "call-produce",
                "arguments": {"program": "sh",
                    "arguments": ["-c",
                        "printf worker-output > produced.txt"]}}
        },
        "credential_broker": [{"reference": "native-vault-ref",
            "project": "staffing-demo", "environment": "OPENAI_API_KEY",
            "value": "fixture-not-a-real-secret"}],
        "shell_executor": {"launcher_path": launcher.display().to_string(),
            "protected_paths": ["/etc", "/var", "/home"],
            "allowed_programs": ["sh"]}
    });
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)
        .map_err(|error| DesktopError::Setup(format!("operator config: {error}")))?;
    file.write_all(serde_json::to_string_pretty(&config).unwrap().as_bytes())
        .map_err(|error| DesktopError::Setup(format!("operator config: {error}")))?;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
        .map_err(|error| DesktopError::Setup(format!("operator config mode: {error}")))?;
    Ok(())
}
