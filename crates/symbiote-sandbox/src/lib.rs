//! Linux bubblewrap 0.12 invocation, against a trusted Arch-style `/usr` base.
//! This is an internal trusted launcher, not an authenticated Host endpoint.
//! Path checks do not defeat a malicious same-UID process racing path replacement;
//! the caller must reserve the worktree and its private parent for this execution.
//! Consent binds a command descriptor, not executable bytes or dependency closure.
#![cfg(target_os = "linux")]

use nix::{
    dir::Dir,
    fcntl::{OFlag, open, openat},
    sys::stat::{Mode, SFlag, fstat},
    unistd::geteuid,
};
use serde::Serialize;
use serde_json::Value;
use std::{
    collections::BTreeMap,
    ffi::OsString,
    fmt,
    fs::File,
    os::fd::AsFd,
    path::{Component, Path, PathBuf},
    time::Duration,
};
use symbiote_domain::{AccessSnapshot, HostId, Permission, RootId, Timestamp};
use symbiote_runtime_transport::{
    CancelReport, JsonlTransport, ProcessExit, SpawnSpec, TransportError, TransportLimits,
};
use symbiote_trust::{Fingerprint, ResourceConsent, ResourceSnapshot, authorize_load};

pub const INVOCATION_VERSION: &str = "linux-bwrap-0.12-v1";
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Profile {
    ReadOnly,
    WorktreeWrite,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SandboxError {
    InvalidCommand,
    UnsafePath,
    ProtectedOverlap,
    ConsentDenied,
    PermissionDenied,
    UnsupportedEntry,
    ResourceLimit,
    Unavailable,
    SetupFailed(SetupFailure),
    Transport(TransportError),
}
/// Bounded setup stderr is available only through explicit inspection.
#[derive(Clone, PartialEq, Eq)]
pub struct SetupFailure {
    cause: Option<TransportError>,
    diagnostics: Vec<String>,
    truncated: bool,
}
impl SetupFailure {
    pub fn cause(&self) -> Option<&TransportError> {
        self.cause.as_ref()
    }
    pub fn diagnostics(&self) -> &[String] {
        &self.diagnostics
    }
    pub fn truncated(&self) -> bool {
        self.truncated
    }
}
impl fmt::Debug for SetupFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SetupFailure")
            .field("cause", &self.cause)
            .field("diagnostic_count", &self.diagnostics.len())
            .field("truncated", &self.truncated)
            .finish()
    }
}
impl SandboxError {
    pub fn setup_failure(&self) -> Option<&SetupFailure> {
        match self {
            Self::SetupFailed(failure) => Some(failure),
            _ => None,
        }
    }
}

fn setup_failure(transport: &mut JsonlTransport, cause: Option<TransportError>) -> SandboxError {
    let deadline = std::time::Instant::now() + Duration::from_millis(200);
    while !transport.diagnostics_finished() && std::time::Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(1));
    }
    // Acquire completion before draining: a true observation guarantees the
    // producer's final entry is already published into the diagnostic queue.
    let finished = transport.diagnostics_finished();
    let batch = transport.diagnostics();
    let mut truncated = !finished || batch.dropped > 0 || batch.entries.len() > 8;
    let diagnostics = batch
        .entries
        .into_iter()
        .take(8)
        .map(|entry| {
            let mut text = entry.text;
            truncated |= entry.truncated || text.len() > 1024;
            if text.len() > 1024 {
                let mut boundary = 1024;
                while !text.is_char_boundary(boundary) {
                    boundary -= 1;
                }
                text.truncate(boundary);
            }
            text
        })
        .collect();
    SandboxError::SetupFailed(SetupFailure {
        cause,
        diagnostics,
        truncated,
    })
}
impl fmt::Display for SandboxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "sandbox rejected: {self:?}")
    }
}
impl std::error::Error for SandboxError {}
pub type Result<T> = std::result::Result<T, SandboxError>;

/// All authority-bearing fields must come from trusted Host state, not client JSON.
pub struct LaunchRequest<'a> {
    /// Trusted installed launcher binary. Revalidated as regular, non-writable by others.
    pub helper_path: &'a Path,
    pub consent: &'a ResourceConsent,
    pub snapshot: &'a ResourceSnapshot,
    pub policy: &'a AccessSnapshot,
    pub host: &'a HostId,
    pub at: Timestamp,
    pub root_id: &'a RootId,
    pub worktree: &'a Path,
    /// Complete protected Host directories; nonempty, absolute, existing and symlink-free.
    pub protected_paths: &'a [PathBuf],
    pub profile: Profile,
    /// Only absolute /usr/bin executables are accepted; arguments are never shell-expanded.
    pub program: &'a str,
    pub args: &'a [String],
    pub limits: TransportLimits,
}

/// The descriptor includes the absolute worktree, Root, command and fixed policy version.
/// It deliberately does not claim to fingerprint the installed runtime or imported files.
pub fn fingerprint_command(
    root: &RootId,
    worktree: &Path,
    profile: Profile,
    program: &str,
    args: &[String],
) -> Result<Fingerprint> {
    validate_command(program, args)?;
    validate_absolute(worktree)?;
    let path = worktree.to_str().ok_or(SandboxError::UnsafePath)?;
    let bytes = serde_json::to_vec(&(INVOCATION_VERSION, root, path, profile, program, args))
        .map_err(|_| SandboxError::InvalidCommand)?;
    Ok(Fingerprint::of(&bytes))
}
fn validate_command(program: &str, args: &[String]) -> Result<()> {
    let suffix = program
        .strip_prefix("/usr/bin/")
        .ok_or(SandboxError::InvalidCommand)?;
    if program.len() > 256
        || suffix.is_empty()
        || !suffix
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
        || args.len() > 64
        || args.iter().any(|a| a.len() > 65_536 || a.contains('\0'))
        || args.iter().map(String::len).sum::<usize>() > 131_072
    {
        return Err(SandboxError::InvalidCommand);
    }
    Ok(())
}
fn validate_absolute(path: &Path) -> Result<()> {
    use std::os::unix::ffi::OsStrExt;
    if path.as_os_str().as_bytes().len() > 4096
        || path.components().count() > 128
        || !path.is_absolute()
        || path
            .components()
            .any(|c| !matches!(c, Component::RootDir | Component::Normal(_)))
    {
        return Err(SandboxError::UnsafePath);
    }
    Ok(())
}
fn flags() -> OFlag {
    OFlag::O_RDONLY | OFlag::O_DIRECTORY | OFlag::O_NOFOLLOW | OFlag::O_CLOEXEC
}
fn directory(path: &Path) -> Result<File> {
    validate_absolute(path)?;
    let mut fd =
        open(Path::new("/"), flags(), Mode::empty()).map_err(|_| SandboxError::UnsafePath)?;
    for component in path.components() {
        if let Component::Normal(name) = component {
            fd = openat(&fd, name, flags(), Mode::empty()).map_err(|_| SandboxError::UnsafePath)?;
            let stat = fstat(&fd).map_err(|_| SandboxError::UnsafePath)?;
            if ![0, geteuid().as_raw()].contains(&stat.st_uid)
                || (stat.st_mode & 0o022 != 0 && stat.st_mode & 0o1000 == 0)
            {
                return Err(SandboxError::UnsafePath);
            }
        }
    }
    Ok(fd.into())
}
fn owned_private(fd: &impl AsFd, exact: bool) -> Result<()> {
    let stat = fstat(fd).map_err(|_| SandboxError::UnsafePath)?;
    if stat.st_uid != geteuid().as_raw()
        || stat.st_mode & 0o022 != 0
        || (exact && stat.st_mode & 0o7777 != 0o700)
    {
        return Err(SandboxError::UnsafePath);
    }
    Ok(())
}
fn validate_helper(path: &Path) -> Result<()> {
    validate_absolute(path)?;
    let parent = directory(path.parent().ok_or(SandboxError::UnsafePath)?)?;
    let name = path.file_name().ok_or(SandboxError::UnsafePath)?;
    let fd = openat(
        &parent,
        name,
        OFlag::O_PATH | OFlag::O_NOFOLLOW | OFlag::O_CLOEXEC,
        Mode::empty(),
    )
    .map_err(|_| SandboxError::Unavailable)?;
    let stat = fstat(&fd).map_err(|_| SandboxError::Unavailable)?;
    if SFlag::from_bits_truncate(stat.st_mode) & SFlag::S_IFMT != SFlag::S_IFREG
        || ![0, geteuid().as_raw()].contains(&stat.st_uid)
        || stat.st_mode & 0o022 != 0
        || stat.st_mode & 0o111 == 0
    {
        return Err(SandboxError::UnsafePath);
    }
    Ok(())
}
fn overlap(a: &Path, b: &Path) -> bool {
    a.starts_with(b) || b.starts_with(a)
}

// A pathname Unix socket inside a bind mount can reach the Host even with an
// isolated network namespace. Reject sockets/devices/FIFOs and aliased hardlinks.
fn inspect_tree(fd: &impl AsFd, depth: usize, left: &mut usize) -> Result<()> {
    if depth > 64 {
        return Err(SandboxError::ResourceLimit);
    }
    let mut entries =
        Dir::openat(fd, ".", flags(), Mode::empty()).map_err(|_| SandboxError::UnsafePath)?;
    for entry in entries.iter() {
        let entry = entry.map_err(|_| SandboxError::UnsafePath)?;
        let name = entry.file_name();
        if name.to_bytes() == b"." || name.to_bytes() == b".." {
            continue;
        }
        *left = left.checked_sub(1).ok_or(SandboxError::ResourceLimit)?;
        // O_PATH avoids opening device nodes and never follows symlinks.
        let child = openat(
            fd,
            name,
            OFlag::O_PATH | OFlag::O_NOFOLLOW | OFlag::O_CLOEXEC,
            Mode::empty(),
        )
        .map_err(|_| SandboxError::UnsafePath)?;
        let stat = fstat(&child).map_err(|_| SandboxError::UnsafePath)?;
        let kind = SFlag::from_bits_truncate(stat.st_mode) & SFlag::S_IFMT;
        if stat.st_uid != geteuid().as_raw() || stat.st_mode & 0o022 != 0 {
            return Err(SandboxError::UnsafePath);
        }
        match kind {
            SFlag::S_IFDIR => inspect_tree(&child, depth + 1, left)?,
            SFlag::S_IFREG if stat.st_nlink == 1 => {}
            // Symlinks can bypass scans of sockets under another visible mount.
            _ => return Err(SandboxError::UnsupportedEntry),
        }
    }
    Ok(())
}

pub struct SandboxProcess {
    transport: JsonlTransport,
}
impl SandboxProcess {
    /// Explicit opt-in to bounded raw child stderr; Debug on this batch is redacted.
    pub fn diagnostics(&mut self) -> symbiote_runtime_transport::DiagnosticBatch {
        self.transport.diagnostics()
    }
    pub fn send(&mut self, value: &Value, timeout: Duration) -> Result<()> {
        self.transport
            .send(value, timeout)
            .map_err(SandboxError::Transport)
    }
    pub fn recv(&mut self, timeout: Duration) -> Result<Value> {
        self.transport
            .recv(timeout)
            .map_err(SandboxError::Transport)
    }
    pub fn try_wait(&mut self) -> Result<Option<ProcessExit>> {
        self.transport.try_wait().map_err(SandboxError::Transport)
    }
    /// The underlying report remains conservative: descendant cleanup is Unknown.
    pub fn cancel(&mut self, timeout: Duration) -> Result<CancelReport> {
        self.transport
            .cancel(timeout)
            .map_err(SandboxError::Transport)
    }
}

pub fn launch(request: LaunchRequest<'_>) -> Result<SandboxProcess> {
    validate_helper(request.helper_path)?;
    authorize_load(
        request.consent,
        request.snapshot,
        request.policy,
        request.host,
        request.at,
    )
    .map_err(|_| SandboxError::ConsentDenied)?;
    if request.snapshot.fingerprint
        != fingerprint_command(
            request.root_id,
            request.worktree,
            request.profile,
            request.program,
            request.args,
        )?
    {
        return Err(SandboxError::ConsentDenied);
    }
    let grants = &request.snapshot.access.grants;
    if !request.snapshot.access.roots.contains(request.root_id)
        || !grants.contains(&Permission::ReadRoot)
        || !grants.contains(&Permission::ExecuteProcess)
        || (request.profile == Profile::WorktreeWrite
            && !grants.contains(&Permission::MutateStream))
    {
        return Err(SandboxError::PermissionDenied);
    }
    if request.protected_paths.is_empty() || request.protected_paths.len() > 32 {
        return Err(SandboxError::UnsafePath);
    }
    let worktree = directory(request.worktree)?;
    owned_private(&worktree, false)?;
    owned_private(
        &directory(request.worktree.parent().ok_or(SandboxError::UnsafePath)?)?,
        true,
    )?;
    if overlap(request.worktree, Path::new("/usr")) {
        return Err(SandboxError::ProtectedOverlap);
    }
    for protected in request.protected_paths {
        directory(protected)?;
        if overlap(request.worktree, protected) || overlap(Path::new("/usr"), protected) {
            return Err(SandboxError::ProtectedOverlap);
        }
    }
    inspect_tree(&worktree, 0, &mut 100_000)?;
    let mut args: Vec<OsString> = [
        "--unshare-all",
        "--unshare-user",
        "--disable-userns",
        "--assert-userns-disabled",
        "--die-with-parent",
        "--new-session",
        "--clearenv",
        "--cap-drop",
        "ALL",
        "--ro-bind",
        "/usr",
        "/usr",
        "--symlink",
        "usr/bin",
        "/bin",
        "--symlink",
        "usr/lib",
        "/lib",
        "--symlink",
        "usr/lib64",
        "/lib64",
        "--proc",
        "/proc",
        "--dev",
        "/dev",
        "--tmpfs",
        "/tmp",
        "--tmpfs",
        "/home",
        "--dir",
        "/home/agent",
    ]
    .into_iter()
    .map(Into::into)
    .collect();
    args.push(
        if request.profile == Profile::ReadOnly {
            "--ro-bind"
        } else {
            "--bind"
        }
        .into(),
    );
    args.push(request.worktree.as_os_str().into());
    args.extend(
        [
            "/workspace",
            "--chdir",
            "/workspace",
            "--setenv",
            "HOME",
            "/home/agent",
            "--setenv",
            "PATH",
            "/usr/bin:/bin",
            "--setenv",
            "TMPDIR",
            "/tmp",
            "--remount-ro",
            "/",
            "--",
            "/usr/bin/sh",
            "-c",
            "printf '%s\\n' '{\"symbiote_sandbox_ready\":1}'; exec \"$@\"",
            "symbiote",
        ]
        .into_iter()
        .map(OsString::from),
    );
    args.push(request.program.into());
    args.extend(request.args.iter().map(OsString::from));
    let mut transport = JsonlTransport::spawn(
        SpawnSpec {
            executable: request.helper_path.into(),
            args,
            cwd: "/".into(),
            env: BTreeMap::new(),
        },
        request.limits,
    )
    .map_err(SandboxError::Transport)?;
    let ready = match transport.recv(Duration::from_secs(5)) {
        Ok(ready) => ready,
        Err(error) => return Err(setup_failure(&mut transport, Some(error))),
    };
    if ready != serde_json::json!({"symbiote_sandbox_ready":1}) {
        return Err(setup_failure(&mut transport, None));
    }
    Ok(SandboxProcess { transport })
}
