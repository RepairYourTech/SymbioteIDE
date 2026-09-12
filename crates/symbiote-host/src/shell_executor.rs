//! The production shell-tool executor (#218/#465): runs one validated shell
//! invocation inside the Linux sandbox (bubblewrap via the trusted launcher),
//! in the dispatch's reserved worktree, with the WorktreeWrite profile — no
//! network, private HOME, literal argv, bounded output.
//!
//! Authorization layering, kept explicit:
//! - the sandbox provides OS containment (`symbiote-sandbox::launch`): every
//!   launch re-derives the command fingerprint and requires a consent whose
//!   snapshot covers EXACTLY that fingerprint, so a widened command fails the
//!   sandbox's own check rather than executing;
//! - the [`ShellConsentAuthority`] is the operator's command authorization.
//!   It is consulted per invocation with the exact descriptor (root, worktree,
//!   wrapped program, arguments) and returns the operator's consent — or
//!   refuses. The executor never mints, caches, or widens authority: a refused
//!   invocation maps to `ToolExecError::Refused`, which the loop records as
//!   ToolFailed feedback to the model and continues.
//!
//! A live MODEL turn (the Responses API or a Codex App Server thread) is a
//! separate authorization for credentials and billing that remains surfaced,
//! never attempted here; this executor runs tool commands only.
//!
//! Output capture: the sandbox child's stdout is the transport's JSONL frame
//! pipe (the ready marker is consumed by `launch`; any raw line would corrupt
//! framing), so tool output cannot flow through it. The executor wraps the
//! invocation in a fixed `/usr/bin/sh` script that redirects the tool's
//! stdout and stderr into a file inside the worktree (`/workspace` in the
//! sandbox) and reads it back from the Host after exit, then deletes it. The
//! model's program and arguments are passed to the wrapper as positional
//! parameters — never embedded in a shell string — so arguments stay literal
//! end to end and the wrapper script is a constant that fingerprints stably.
//!
//! Note the layering consequence: a tool invocation of an interpreter (e.g.
//! `sh -c ...`) is not an escape — OS containment holds regardless — but the
//! consent authority sees the full wrapped argument vector and can refuse
//! such shapes; authoring that policy is the operator's decision, not this
//! executor's.
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use symbiote_domain::{
    AccessSnapshot, HostId, ProcessBound, ProjectId, RoleId, RootId, RuntimeProfileId, Timestamp,
};
use symbiote_native_agent::tools::{ShellInvocation, ShellToolExecutor, ToolExecError};
use symbiote_runtime_transport::TransportError;
use symbiote_sandbox::{LaunchRequest, Profile, SandboxError};
use symbiote_trust::ResourceConsent;

/// The consent authority the operator configures. Implementations own the
/// operator's authorization policy (recorded consents, user prompting, allow
/// lists) and MUST return a consent whose snapshot fingerprint covers the
/// exact wrapped command — the sandbox independently verifies that equality
/// at launch, so a loose or mismatched consent cannot execute.
pub trait ShellConsentAuthority {
    /// Returns the operator's consent for exactly this descriptor, or a
    /// stable refusal reason. `access` is the dispatch binding's access
    /// snapshot (the current policy at run time); a consent whose snapshot
    /// access exceeds it is refused by the trust check, not widened here.
    fn consent(
        &mut self,
        request: ShellConsentRequest<'_>,
    ) -> Result<ResourceConsent, &'static str>;
}

/// The exact descriptor one tool invocation presents to the authority.
pub struct ShellConsentRequest<'a> {
    pub root_id: &'a RootId,
    pub worktree: &'a Path,
    /// The wrapped program the sandbox will launch (always `/usr/bin/sh`).
    pub program: &'a str,
    /// The wrapped arguments: the fixed wrapper script plus the model's
    /// program and literal arguments as positional parameters.
    pub args: &'a [String],
    pub host: &'a HostId,
    pub project_id: &'a ProjectId,
    pub role_id: &'a RoleId,
    pub profile_id: &'a RuntimeProfileId,
    /// The dispatch binding's access snapshot — the current policy.
    pub access: &'a AccessSnapshot,
    /// The run's observed time, for the consent's validity window.
    pub at: Timestamp,
    /// The consenting principal (the daemon's local owner) — the consent
    /// record's user, not model-supplied.
    pub user_id: &'a symbiote_domain::UserId,
}

/// The fixed wrapper: run the first positional parameter (the model's
/// program) with the remaining parameters as its arguments, capturing
/// stdout and stderr through a FIFO into the output file inside the
/// worktree. POSIX sh (`shift` + `"$@"`), no quoting of model input
/// anywhere. The capture is bounded AT THE SOURCE by `head -c` (1 MiB, four
/// times the Host read bound): a tool spamming its output gets SIGPIPE and
/// dies instead of filling the Host's disk for the whole deadline, while
/// the tool's own (unrelated) file writes in the worktree are NOT limited —
/// no `ulimit -f`, which would break every consented build tool. The tool's
/// exit status is taken directly (`st=$?` before the wait), and the FIFO
/// lives in the per-launch private /tmp so nothing residues in the
/// worktree.
const WRAPPER_SCRIPT: &str = "p=\"$1\"; shift; mkfifo /tmp/.symbiote-cappipe; head -c 1048576 > .symbiote-tool-output < /tmp/.symbiote-cappipe & \"$p\" \"$@\" > /tmp/.symbiote-cappipe 2>&1; st=$?; wait $!; rm -f /tmp/.symbiote-cappipe; exit $st";
/// The capture bound enforced by the wrapper's `head -c` (bytes). Four
/// times the Host read bound: the read-back is the tight bound, the
/// wrapper bound is the disk-safety backstop.
pub const WRAPPER_CAPTURE_CAP_BYTES: usize = 1_048_576;
/// The wrapper's `$0` slot (never used by the script; keeps argv shape explicit).
const WRAPPER_NAME: &str = "symbiote-tool";
/// The tool-output file, relative to the worktree (= `/workspace` inside).
pub const TOOL_OUTPUT_FILE: &str = ".symbiote-tool-output";
/// Output bound per tool invocation, read back from the worktree file.
pub const MAX_SHELL_OUTPUT_BYTES: usize = 256 * 1024;
/// Deadline for one tool invocation, enforced by the executor.
pub const SHELL_DEADLINE: Duration = Duration::from_secs(120);

fn now() -> Timestamp {
    Timestamp(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis().try_into().unwrap_or(u64::MAX))
            .unwrap_or(u64::MAX),
    )
}

/// Resolves the model's invocation into the wrapped command the sandbox
/// launches: `/usr/bin/sh -c <WRAPPER> <name> <program> <args...>`. Bare
/// program names resolve against /usr/bin only (the launcher's rule); any
/// other path is rejected — the sandbox cannot execute it. The wrapped
/// vector is checked against the sandbox's own validation bounds (64
/// arguments, 128 KiB total) because the wrapper adds four fixed slots —
/// without this check a legal-shape 61–64-argument proposal would fail
/// later inside `fingerprint_command` and surface as a confusing consent
/// refusal instead of the honest shape rejection. Both rejections are
/// `InvalidCommand` (recorded composition feedback), never `Refused` (the
/// operator is not consulted for a command that cannot be composed).
pub fn wrapped_command(
    invocation: &ShellInvocation,
) -> Result<(String, Vec<String>), ToolExecError> {
    let program = if let Some(suffix) = invocation.program.strip_prefix("/usr/bin/") {
        if suffix.is_empty() {
            return Err(ToolExecError::InvalidCommand);
        }
        invocation.program.clone()
    } else if invocation.program.contains('/') {
        return Err(ToolExecError::InvalidCommand);
    } else {
        format!("/usr/bin/{}", invocation.program)
    };
    let mut args = Vec::with_capacity(invocation.arguments.len() + 4);
    args.push("-c".to_owned());
    args.push(WRAPPER_SCRIPT.to_owned());
    args.push(WRAPPER_NAME.to_owned());
    args.push(program);
    args.extend(invocation.arguments.iter().cloned());
    // The sandbox validates the WRAPPED vector: mirror its exact bounds
    // here so a wrap-overhead overflow is a composition rejection.
    if args.len() > 64 || args.iter().map(String::len).sum::<usize>() > 131_072 {
        return Err(ToolExecError::InvalidCommand);
    }
    Ok(("/usr/bin/sh".to_owned(), args))
}

/// The operator's program-allowlist consent authority (#466 allowlist
/// flow): consents to a tool invocation when the model's program (the
/// wrapped argument vector's fourth element — the resolved
/// /usr/bin/<program>) is in the operator-authored allowlist. The consent
/// is authored over the EXACT wrapped command via the sandbox's own
/// fingerprint derivation, issued from the dispatch's identities, and
/// expires with the 5-minute lease window. Programs outside the allowlist
/// are refused — the executor never mints authority beyond this list.
pub struct ProgramAllowlistAuthority {
    pub allowed_programs: Vec<String>,
}
impl ShellConsentAuthority for ProgramAllowlistAuthority {
    fn consent(
        &mut self,
        request: ShellConsentRequest<'_>,
    ) -> Result<ResourceConsent, &'static str> {
        let Some(model_program) = request.args.get(3) else {
            return Err("malformed wrapped command");
        };
        let Some(bare) = model_program.strip_prefix("/usr/bin/") else {
            return Err("program outside /usr/bin");
        };
        if !self.allowed_programs.iter().any(|allowed| allowed == bare) {
            return Err("program not in the operator allowlist");
        }
        let fingerprint = symbiote_sandbox::fingerprint_command(
            request.root_id,
            request.worktree,
            Profile::WorktreeWrite,
            request.program,
            request.args,
        )
        .map_err(|_| "command rejected")?;
        Ok(ResourceConsent {
            id: symbiote_domain::CommandId::new(format!("allowlist-{}", request.project_id))
                .map_err(|_| "consent id")?,
            snapshot: symbiote_trust::ResourceSnapshot {
                project_id: request.project_id.clone(),
                role_id: request.role_id.clone(),
                profile_id: request.profile_id.clone(),
                host_id: request.host.clone(),
                resource_ref: "shell-tool".into(),
                fingerprint,
                access: request.access.clone(),
            },
            user_id: request.user_id.clone(),
            issued_at: request.at,
            expires_at: Timestamp(request.at.0.saturating_add(300_000)),
            revoked_at: None,
        })
    }
}

/// The production executor. Constructed per run from the operator's launch
/// configuration (trusted launcher path, protected Host directories) and the
/// operator's consent authority; each `run_shell` consults the authority and
/// launches a fresh sandboxed process — no state carries between tools.
pub struct SandboxShellExecutor {
    pub launcher_path: PathBuf,
    pub protected_paths: Vec<PathBuf>,
    pub authority: Box<dyn ShellConsentAuthority>,
    /// The run's trusted identities from the dispatch contract: the sandbox
    /// launch and every consent descriptor carry these, never values derived
    /// from caller input.
    pub root_id: RootId,
    pub host: HostId,
    pub project_id: ProjectId,
    pub role_id: RoleId,
    pub profile_id: RuntimeProfileId,
    /// The dispatch binding's access snapshot — the current policy at run
    /// time, checked against the consent's recorded access by the trust
    /// layer and against the operator's ceiling by the Host.
    pub access: AccessSnapshot,
    /// The consenting principal (the daemon's local owner).
    pub user_id: symbiote_domain::UserId,
    /// The bound this dispatch's declared limits reduce to: every tool process
    /// is launched with it, so the execution the dispatch produces is bounded
    /// by what it declared rather than by nothing.
    pub bound: ProcessBound,
}

impl SandboxShellExecutor {
    /// Reads the tool-output file back, bounded; `truncated` marks an
    /// over-bound file (the caller keeps the prefix — the same discipline as
    /// every other bounded read).
    ///
    /// The worktree is model-writable DURING the run (WorktreeWrite), so the
    /// capture file can be replaced by a consented command while it runs:
    /// the open follows the sandbox crate's own path discipline —
    /// O_NOFOLLOW (a symlinked capture file is refused, never dereferenced:
    /// no host file can leak into model context) and O_NONBLOCK (a planted
    /// FIFO cannot wedge the read; it is refused by the regular-file
    /// check). run_shell executes inside the daemon's synchronous
    /// connection loop, so a blocking open would brick the whole Host.
    fn read_output(worktree: &Path) -> std::io::Result<(Vec<u8>, bool)> {
        use std::os::fd::AsFd;
        use std::os::unix::fs::OpenOptionsExt;
        let file = std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(
                nix::fcntl::OFlag::O_NOFOLLOW.bits() | nix::fcntl::OFlag::O_NONBLOCK.bits(),
            )
            .open(worktree.join(TOOL_OUTPUT_FILE))?;
        let stat = nix::sys::stat::fstat(file.as_fd())
            .map_err(|_| std::io::Error::other("capture fstat"))?;
        if nix::sys::stat::SFlag::from_bits_truncate(stat.st_mode) & nix::sys::stat::SFlag::S_IFMT
            != nix::sys::stat::SFlag::S_IFREG
        {
            // Not a regular file (FIFO, device, socket): refuse, never read.
            return Err(std::io::Error::other("capture file is not a regular file"));
        }
        let mut file = file;
        let mut buffer = Vec::new();
        let mut chunk = [0u8; 8192];
        loop {
            match file.read(&mut chunk)? {
                0 => return Ok((buffer, false)),
                count => {
                    if buffer.len() + count > MAX_SHELL_OUTPUT_BYTES {
                        let keep = MAX_SHELL_OUTPUT_BYTES - buffer.len();
                        buffer.extend_from_slice(&chunk[..keep]);
                        return Ok((buffer, true));
                    }
                    buffer.extend_from_slice(&chunk[..count]);
                }
            }
        }
    }
}

impl ShellToolExecutor for SandboxShellExecutor {
    fn run_shell(
        &mut self,
        invocation: &ShellInvocation,
        worktree: &Path,
    ) -> Result<(Vec<u8>, Option<i32>), ToolExecError> {
        let (program, args) = wrapped_command(invocation)?;
        let at = now();
        // Per-invocation consent from the operator's authority. The consent
        // is consumed exactly as authored: its own snapshot is what launch
        // re-derives the fingerprint against, so nothing here can widen it.
        let consent = self
            .authority
            .consent(ShellConsentRequest {
                root_id: &self.root_id,
                worktree,
                program: &program,
                args: &args,
                host: &self.host,
                project_id: &self.project_id,
                role_id: &self.role_id,
                profile_id: &self.profile_id,
                access: &self.access,
                at,
                user_id: &self.user_id,
            })
            .map_err(|_| ToolExecError::Refused)?;
        // Clear any stale output file so the read-back is unambiguous.
        let _ = std::fs::remove_file(worktree.join(TOOL_OUTPUT_FILE));
        let mut process = symbiote_sandbox::launch(LaunchRequest {
            helper_path: &self.launcher_path,
            consent: &consent,
            snapshot: &consent.snapshot,
            policy: &self.access,
            host: &self.host,
            at,
            root_id: &self.root_id,
            worktree,
            protected_paths: &self.protected_paths,
            profile: Profile::WorktreeWrite,
            program: &program,
            args: &args,
            limits: symbiote_runtime_transport::TransportLimits::default(),
            address_space_bytes: self.bound.address_space_bytes,
        })
        .map_err(execution)?;
        let deadline = Instant::now() + SHELL_DEADLINE;
        // The tool's stdout never reaches the frame pipe (redirected); EOF
        // arrives at process exit. Timeout errors are the poll cadence.
        loop {
            if Instant::now() >= deadline {
                let _ = process.cancel(Duration::from_secs(2));
                // The capture file (if the wrapper created it) is residue in
                // the worker's diff — remove it on the deadline path too.
                let _ = std::fs::remove_file(worktree.join(TOOL_OUTPUT_FILE));
                return Err(ToolExecError::Execution);
            }
            match process.recv(Duration::from_millis(250)) {
                Ok(_) => continue,
                Err(SandboxError::Transport(TransportError::DeadlineExceeded)) => continue,
                Err(_) => break,
            }
        }
        // Poll briefly for the reaped exit even after EOF.
        let mut exit = None;
        let reaped_deadline = Instant::now() + Duration::from_secs(5);
        while exit.is_none() {
            match process.try_wait().map_err(execution)? {
                Some(observed) => exit = Some(observed),
                None if Instant::now() >= reaped_deadline => break,
                None => std::thread::sleep(Duration::from_millis(10)),
            }
        }
        let _ = process.cancel(Duration::from_secs(2));
        let (mut output, truncated) = match Self::read_output(worktree) {
            Ok(result) => result,
            Err(_) => {
                // A refused read-back (planted symlink/FIFO) must not leave
                // the planted entry in the worker's diff: remove whatever
                // sits at the capture path, then fail.
                let _ = std::fs::remove_file(worktree.join(TOOL_OUTPUT_FILE));
                return Err(ToolExecError::Execution);
            }
        };
        let _ = std::fs::remove_file(worktree.join(TOOL_OUTPUT_FILE));
        if truncated {
            output.extend_from_slice("\n…[truncated]".as_bytes());
        }
        Ok((output, exit.and_then(|observed| observed.code)))
    }
}

fn execution(error: SandboxError) -> ToolExecError {
    // Launch and transport failures are execution failures, not refusals:
    // the distinction the loop relies on (refusals feed back, failures halt
    // for Host retry policy).
    let _ = error;
    ToolExecError::Execution
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::DirBuilderExt;
    use symbiote_native_agent::tools::ShellInvocation;

    fn invocation(program: &str, arguments: &[&str]) -> ShellInvocation {
        ShellInvocation {
            program: program.to_owned(),
            arguments: arguments.iter().map(|a| (*a).to_owned()).collect(),
        }
    }

    #[test]
    fn wrapped_command_composes_literal_positional_parameters() {
        let (program, args) = wrapped_command(&invocation("cargo", &["test", "--locked"])).unwrap();
        assert_eq!(program, "/usr/bin/sh");
        assert_eq!(
            args,
            vec![
                "-c".to_owned(),
                WRAPPER_SCRIPT.to_owned(),
                WRAPPER_NAME.to_owned(),
                "/usr/bin/cargo".to_owned(),
                "test".to_owned(),
                "--locked".to_owned(),
            ]
        );
    }

    #[test]
    fn absolute_usr_bin_programs_pass_and_other_paths_are_refused() {
        let (program, args) = wrapped_command(&invocation("/usr/bin/python3", &["-V"])).unwrap();
        assert_eq!(program, "/usr/bin/sh");
        assert_eq!(args[3], "/usr/bin/python3");
        // A path outside /usr/bin can never launch: reject at composition
        // (InvalidCommand — no operator is consulted for an incomposable
        // command).
        assert_eq!(
            wrapped_command(&invocation("/tmp/evil", &[])),
            Err(ToolExecError::InvalidCommand)
        );
        assert_eq!(
            wrapped_command(&invocation("relative/path", &[])),
            Err(ToolExecError::InvalidCommand)
        );
        assert_eq!(
            wrapped_command(&invocation("/usr/bin/", &[])),
            Err(ToolExecError::InvalidCommand)
        );
    }

    #[test]
    fn wrapped_argv_overhead_is_rejected_at_composition_not_as_consent() {
        // The sandbox validates the WRAPPED vector (64 args / 128 KiB); the
        // wrapper adds four fixed slots, so 61+ model arguments are
        // incomposable (60 + 4 is the last legal shape). It must be the
        // shape rejection, never a consent refusal (the operator is not
        // consulted for an incomposable command).
        let arguments = (0..60).map(|i| i.to_string()).collect::<Vec<_>>();
        assert!(
            wrapped_command(&ShellInvocation {
                program: "ls".into(),
                arguments: arguments.clone(),
            })
            .is_ok()
        );
        let mut arguments = arguments;
        arguments.push("61st".into()); // 65 wrapped > 64
        assert_eq!(
            wrapped_command(&ShellInvocation {
                program: "ls".into(),
                arguments,
            }),
            Err(ToolExecError::InvalidCommand)
        );
        // Total-bytes bound: 64 args of 2 KiB ≈ 131 KiB wrapped > 128 KiB.
        let big = vec!["x".repeat(2048); 64];
        assert_eq!(
            wrapped_command(&ShellInvocation {
                program: "ls".into(),
                arguments: big,
            }),
            Err(ToolExecError::InvalidCommand)
        );
    }

    #[test]
    fn the_wrapper_is_constant_so_fingerprints_are_authored_against_a_stable_shape() {
        // The script is a compile-time constant; only the positional
        // parameters vary. This is what makes operator consent authorable:
        // the descriptor's variable part is exactly the model's program and
        // literal arguments.
        let (_, first) = wrapped_command(&invocation("ls", &[])).unwrap();
        let (_, second) = wrapped_command(&invocation("git", &["status"])).unwrap();
        assert_eq!(first[1], second[1]);
        assert_eq!(first[1], WRAPPER_SCRIPT);
        assert_eq!(first[0], "-c");
    }

    // Real-sandbox composition tests. They require the trusted launcher
    // binary (built by the workspace gauntlet: `cargo test --workspace`
    // compiles every package's bins) and /usr/bin/bwrap (installed by CI).
    // When the launcher binary is absent from a partial build, the test
    // reports the skip loudly instead of pretending to run.

    /// A fixture authority that consents ONLY to one model program (wrapped
    /// args index 3) and computes the exact fingerprint the sandbox will
    /// re-derive — the same authoring flow an operator tool would follow.
    struct AllowProgramAuthority {
        allowed_program: &'static str,
        access: AccessSnapshot,
    }
    impl ShellConsentAuthority for AllowProgramAuthority {
        fn consent(
            &mut self,
            request: ShellConsentRequest<'_>,
        ) -> Result<ResourceConsent, &'static str> {
            if request.args.get(3).map(String::as_str) != Some(self.allowed_program) {
                return Err("command not consented");
            }
            let fingerprint = symbiote_sandbox::fingerprint_command(
                request.root_id,
                request.worktree,
                Profile::WorktreeWrite,
                request.program,
                request.args,
            )
            .map_err(|_| "command rejected")?;
            Ok(ResourceConsent {
                id: symbiote_domain::CommandId::new("consent-echo").unwrap(),
                snapshot: symbiote_trust::ResourceSnapshot {
                    project_id: request.project_id.clone(),
                    role_id: request.role_id.clone(),
                    profile_id: request.profile_id.clone(),
                    host_id: request.host.clone(),
                    resource_ref: "shell-tool".into(),
                    fingerprint,
                    access: self.access.clone(),
                },
                user_id: symbiote_domain::UserId::new("operator").unwrap(),
                issued_at: symbiote_domain::Timestamp(0),
                expires_at: symbiote_domain::Timestamp(4_102_444_800_000),
                revoked_at: None,
            })
        }
    }

    /// The bound a fixture dispatch's declared limits reduce to: roomy enough
    /// that the tool output tests are about capture, not about the ceiling.
    fn fixture_bound() -> ProcessBound {
        ProcessBound {
            address_space_bytes: 1 << 30,
        }
    }

    fn echo_access(project: &ProjectId, root: &RootId) -> AccessSnapshot {
        AccessSnapshot {
            project_id: project.clone(),
            roots: std::collections::BTreeSet::from([root.clone()]),
            grants: std::collections::BTreeSet::from([
                symbiote_domain::Permission::ReadRoot,
                symbiote_domain::Permission::ExecuteProcess,
                symbiote_domain::Permission::MutateStream,
            ]),
            policy_revision: symbiote_domain::Revision(1),
        }
    }

    /// The trusted launcher: `SYMBIOTE_SANDBOX_LAUNCHER` names one explicitly,
    /// otherwise the workspace build is resolved through the shared freshness
    /// check (PR #507 review P2 — a missing or stale launcher must not let the
    /// e2e degrade to a counted-as-passed skip).
    fn obtain_launcher() -> PathBuf {
        if let Ok(path) = std::env::var("SYMBIOTE_SANDBOX_LAUNCHER") {
            let path = PathBuf::from(path);
            if path.is_file() {
                return path;
            }
        }
        symbiote_workflow::binaries::launcher_binary()
    }

    /// A 0o700 private fixture root with a worktree the launch validation
    /// accepts (owned, non-group-writable, symlink-free).
    struct WorktreeFixture {
        base: PathBuf,
        worktree: PathBuf,
    }
    impl WorktreeFixture {
        fn new(tag: &str) -> Self {
            let base = std::env::temp_dir()
                .join(format!("symbiote-shell-e2e-{}-{tag}", std::process::id()));
            let _ = std::fs::remove_dir_all(&base);
            let mut builder = std::fs::DirBuilder::new();
            builder.mode(0o700);
            builder.create(&base).unwrap();
            let worktree = base.join("worktree");
            let mut builder = std::fs::DirBuilder::new();
            builder.mode(0o700);
            builder.create(&worktree).unwrap();
            // launch() requires every protected path to be an existing
            // absolute directory, so the fixture materializes its host dir.
            let host = base.join("protected-host");
            let mut builder = std::fs::DirBuilder::new();
            builder.mode(0o700);
            builder.create(&host).unwrap();
            Self { base, worktree }
        }
    }
    impl Drop for WorktreeFixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.base);
        }
    }

    #[test]
    fn a_real_sandboxed_shell_turn_captures_tool_output_in_the_reserved_worktree() {
        let launcher = obtain_launcher();
        let fixture = WorktreeFixture::new("echo");
        let root = RootId::new("root-shell-e2e").unwrap();
        let project = ProjectId::new("project-shell-e2e").unwrap();
        let access = echo_access(&project, &root);
        let mut executor = SandboxShellExecutor {
            launcher_path: launcher,
            protected_paths: vec![fixture.base.join("protected-host")],
            authority: Box::new(AllowProgramAuthority {
                allowed_program: "/usr/bin/echo",
                access: access.clone(),
            }),
            root_id: root,
            host: HostId::new("host-shell-e2e").unwrap(),
            project_id: project,
            role_id: RoleId::new("worker-shell-e2e").unwrap(),
            profile_id: RuntimeProfileId::new("profile-shell-e2e").unwrap(),
            user_id: symbiote_domain::UserId::new("operator").unwrap(),
            access,
            bound: fixture_bound(),
        };
        let (output, exit) = executor
            .run_shell(&invocation("echo", &["hello-symbiote"]), &fixture.worktree)
            .unwrap();
        assert_eq!(exit, Some(0));
        assert_eq!(output, b"hello-symbiote\n");
        // The output file is removed from the worktree after the read-back:
        // tool capture leaves no residue in the worker's diff.
        assert!(!fixture.worktree.join(TOOL_OUTPUT_FILE).exists());
    }

    /// The elevation-consumer boundary on the real sandbox: with the
    /// resolved access LACKING ExecuteProcess (binding grant absent, no
    /// decided lease), the sandbox refuses to launch even a command the
    /// consent authority itself fingerprinted. A decided lease is the only
    /// thing that can add the grant (see runner's `resolve_shell_access`
    /// tests: an ask never does).
    #[test]
    fn the_sandbox_refuses_a_tool_when_the_resolved_access_lacks_execute_process() {
        let launcher = obtain_launcher();
        let fixture = WorktreeFixture::new("elevate-refuse");
        let root = RootId::new("root-shell-e2e").unwrap();
        let project = ProjectId::new("project-shell-e2e").unwrap();
        let mut access = echo_access(&project, &root);
        access
            .grants
            .remove(&symbiote_domain::Permission::ExecuteProcess);
        let mut executor = SandboxShellExecutor {
            launcher_path: launcher,
            protected_paths: vec![fixture.base.join("protected-host")],
            authority: Box::new(AllowProgramAuthority {
                allowed_program: "/usr/bin/echo",
                access: access.clone(),
            }),
            root_id: root,
            host: HostId::new("host-shell-e2e").unwrap(),
            project_id: project,
            role_id: RoleId::new("worker-shell-e2e").unwrap(),
            profile_id: RuntimeProfileId::new("profile-shell-e2e").unwrap(),
            user_id: symbiote_domain::UserId::new("operator").unwrap(),
            access,
            bound: fixture_bound(),
        };
        // PermissionDenied at the sandbox surfaces as Execution (launch
        // failures halt for Host retry policy; only authority refusals
        // feed back to the model).
        assert_eq!(
            executor.run_shell(&invocation("echo", &["hello-symbiote"]), &fixture.worktree),
            Err(ToolExecError::Execution)
        );
    }

    #[test]
    fn a_command_outside_the_consented_shape_is_refused_without_launch() {
        let launcher = obtain_launcher();
        let fixture = WorktreeFixture::new("refuse");
        let root = RootId::new("root-shell-e2e").unwrap();
        let project = ProjectId::new("project-shell-e2e").unwrap();
        let access = echo_access(&project, &root);
        let mut executor = SandboxShellExecutor {
            launcher_path: launcher,
            protected_paths: vec![fixture.base.join("protected-host")],
            authority: Box::new(AllowProgramAuthority {
                allowed_program: "/usr/bin/echo",
                access: access.clone(),
            }),
            root_id: root,
            host: HostId::new("host-shell-e2e").unwrap(),
            project_id: project,
            role_id: RoleId::new("worker-shell-e2e").unwrap(),
            profile_id: RuntimeProfileId::new("profile-shell-e2e").unwrap(),
            user_id: symbiote_domain::UserId::new("operator").unwrap(),
            access,
            bound: fixture_bound(),
        };
        // `sh` is not in the consented shape: the authority refuses and the
        // executor surfaces Refused without any sandbox launch.
        assert_eq!(
            executor.run_shell(&invocation("sh", &["-c", "echo pwn"]), &fixture.worktree),
            Err(ToolExecError::Refused)
        );
        assert!(!fixture.worktree.join(TOOL_OUTPUT_FILE).exists());
    }

    /// The real shell tool keeps completing under the hardened mount table: a
    /// tool that uses `/dev/shm` — the shared-memory surface the hardening
    /// deliberately keeps — still runs to exit 0 and reports normally through
    /// the capture path, so the lane the executor drives did not lose a
    /// capability it uses. The boundary itself is pinned once, in the
    /// sandbox's own errno table (`symbiote-sandbox/tests/process.rs`): the
    /// errno rows, the cross-process shared-memory observation and the
    /// `/dev/pts` exception are asserted there, not restated here.
    #[test]
    fn a_real_sandboxed_shell_turn_completes_under_a_read_only_dev() {
        let launcher = obtain_launcher();
        let fixture = WorktreeFixture::new("dev-shm");
        let root = RootId::new("root-shell-dev").unwrap();
        let project = ProjectId::new("project-shell-dev").unwrap();
        let access = echo_access(&project, &root);
        let mut executor = SandboxShellExecutor {
            launcher_path: launcher,
            protected_paths: vec![fixture.base.join("protected-host")],
            authority: Box::new(AllowProgramAuthority {
                allowed_program: "/usr/bin/python3",
                access: access.clone(),
            }),
            root_id: root,
            host: HostId::new("host-shell-dev").unwrap(),
            project_id: project,
            role_id: RoleId::new("worker-shell-dev").unwrap(),
            profile_id: RuntimeProfileId::new("profile-shell-dev").unwrap(),
            user_id: symbiote_domain::UserId::new("operator").unwrap(),
            access,
            bound: fixture_bound(),
        };
        // The tool's work uses `/dev/shm` and reports its own result: a broken
        // shared-memory mount raises here and the turn fails instead of
        // reporting normally.
        let probe = r#"import json,sys
path='/dev/shm/'+sys.argv[1]
open(path,'w').write('shared-memory')
print(json.dumps({'shm':open(path).read()}),flush=True)
"#;
        let name = format!("symbiote-shell-dev-{}", std::process::id());
        let (output, exit) = executor
            .run_shell(
                &invocation("python3", &["-c", probe, name.as_str()]),
                &fixture.worktree,
            )
            .unwrap();
        assert_eq!(
            exit,
            Some(0),
            "the tool must complete under a read-only /dev: {output:?}"
        );
        let report: serde_json::Value = serde_json::from_slice(&output)
            .unwrap_or_else(|_| panic!("tool report was not JSON: {output:?}"));
        assert_eq!(
            report["shm"], "shared-memory",
            "the tool's own /dev/shm work must succeed: {report}"
        );
    }
}
