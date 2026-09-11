//! Repository observation and worktree materialization for plain Git
//! repositories (#190 foundation): the canonical facts the worker loops and
//! dispatch preparation need before touching a reserved worktree. This crate
//! executes `git` as an external process with bounded frames and deadlines
//! (the same subprocess discipline as the runtime transport), and treats
//! every observation as an advertisement — nothing here authorizes access,
//! grants permissions, or claims remote identity.
//!
//! Boundary: this is the observation/materialization layer only. It never
//! pushes, pulls, fetches, clones, or commits on the user's behalf, and it
//! sends no commands that contact a remote. Caveat that is the sandbox's
//! business, not this crate's: `checkout` executes repo-configured
//! post-checkout hooks and smudge filters, and a repo with git-lfs
//! configured could reach its remote through the filter — the caller must
//! compose this crate inside the Host sandbox (no network). Repository
//! identity stays with the canonical Root records; this crate never
//! second-guesses them. Credential delegation, remotes,
//! submodules/LFS/sparse-checkout, history normalization and safe
//! multi-step transactions remain pending on #190. Diff evidence is
//! bounded preview material only: `observe_run_diff` collects the tracked
//! diff and 16 KiB heads of the untracked files for rendering inert
//! downstream, size-capped at every layer with an explicit flag or note
//! for every degradation; it authorizes nothing, and full diff/history
//! normalization remains pending.
//!
//! Everything is offline-testable: `GitExecutor` is injected, and tests use
//! the real `git` binary against temporary repositories plus scripted
//! failure executors.
pub mod provision;

use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;
use std::time::Duration;

/// The git invocation boundary. Implementations run one `git` invocation
/// with a deadline and return its bounded stdout. Tests script outputs and
/// failures; production runs the trusted `git` binary.
pub trait GitExecutor {
    /// Run `git -C <worktree> <args>` and return stdout. `Err` covers
    /// spawn failure, deadline, nonzero exit, and oversized output — the
    /// caller cannot distinguish them and must not guess.
    fn run(&mut self, worktree: &Path, args: &[&str]) -> Result<Vec<u8>, GitError>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GitError {
    /// Spawn failure, deadline, or I/O error during the invocation.
    Execution,
    /// Nonzero exit: the operation was refused or failed on the git side.
    GitRefused,
    /// Output exceeded the bounded frame.
    OutputTooLarge,
    /// A parsed fact violated its structural bound (bad encoding, out-of-
    /// range value, oversized field).
    MalformedOutput,
    /// The caller's request was invalid before any invocation (empty or
    /// overlong branch name, non-representable path).
    InvalidRequest,
}

impl std::fmt::Display for GitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "git observation failed: {self:?}")
    }
}
impl std::error::Error for GitError {}

/// The largest stdout this crate will buffer from one invocation.
pub const MAX_GIT_OUTPUT_BYTES: usize = 256 * 1024;
/// The default deadline for one invocation.
pub const GIT_DEADLINE: Duration = Duration::from_secs(10);
/// Bounded lengths for parsed facts.
pub const MAX_BRANCH_BYTES: usize = 512;
pub const MAX_PATH_BYTES: usize = 4096;

/// Production executor: the trusted `git` binary from PATH, run inside the
/// caller's deadline with bounded output capture. No shell, no network
/// flags, no environment manipulation — the sandbox owns isolation.
#[derive(Debug)]
pub struct SystemGit {
    pub deadline: Duration,
}

impl SystemGit {
    pub fn new() -> Self {
        Self {
            deadline: GIT_DEADLINE,
        }
    }
}

impl Default for SystemGit {
    fn default() -> Self {
        Self::new()
    }
}

impl GitExecutor for SystemGit {
    /// One bounded invocation: no shell, stdin null, stderr discarded, and
    /// output captured through the bounded reader. The deadline is enforced
    /// by the deadline thread killing the child on overrun (a hung git
    /// cannot hold the caller forever); the output bound is enforced by the
    /// reader. Exit status: nonzero is `GitRefused`.
    fn run(&mut self, worktree: &Path, args: &[&str]) -> Result<Vec<u8>, GitError> {
        use std::io::Read;
        use std::sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        };
        let mut command = Command::new("git");
        command
            .arg("-C")
            .arg(worktree)
            .args(args)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null());
        let mut child = command.spawn().map_err(|_| GitError::Execution)?;
        let timed_out = Arc::new(AtomicBool::new(false));
        let watchdog_flag = Arc::clone(&timed_out);
        let completion_flag = Arc::clone(&timed_out);
        let deadline = self.deadline;
        let pid = child.id() as i32;
        let watchdog = std::thread::spawn(move || {
            let step = 50;
            let mut waited = 0u64;
            while waited < deadline.as_millis() as u64 {
                std::thread::sleep(Duration::from_millis(step));
                waited += step;
                if watchdog_flag.load(Ordering::Acquire) {
                    return;
                }
            }
            watchdog_flag.store(true, Ordering::Release);
            // Terminate the hung child group-free: the direct child only.
            let _ = nix::sys::signal::kill(
                nix::unistd::Pid::from_raw(pid),
                nix::sys::signal::Signal::SIGKILL,
            );
        });
        let mut stdout = child.stdout.take().ok_or(GitError::Execution)?;
        let mut buffer = Vec::new();
        let mut chunk = [0u8; 8192];
        let read_result = loop {
            match stdout.read(&mut chunk) {
                Ok(0) => break Ok(()),
                Ok(count) => {
                    if buffer.len() + count > MAX_GIT_OUTPUT_BYTES {
                        break Err(GitError::OutputTooLarge);
                    }
                    buffer.extend_from_slice(&chunk[..count]);
                }
                Err(_) => break Err(GitError::Execution),
            }
        };
        let status = child.wait().map_err(|_| GitError::Execution);
        completion_flag.store(true, Ordering::Release);
        let _ = watchdog.join();
        match (read_result, status) {
            (Ok(()), Ok(status)) if status.success() => Ok(buffer),
            (Ok(()), Ok(_)) => Err(GitError::GitRefused),
            (Ok(()), Err(_)) => Err(GitError::Execution),
            (Err(error), _) => Err(error),
        }
    }
}

/// Observed HEAD state of a repository: the exact commit and the branch,
/// or an explicit unborn/detached classification. The commit is empty only
/// for an unborn branch. All fields are advertisements — observed
/// provenance, never authorization.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HeadObservation {
    /// The commit HEAD points at. Always present for a valid repository
    /// (an unborn branch is a separate classification).
    pub commit: String,
    pub state: HeadState,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum HeadState {
    /// HEAD points at a branch tip.
    Branch { name: String },
    /// HEAD is detached at a commit.
    Detached,
    /// The checked-out branch has no commits yet (unborn).
    Unborn,
}

/// The observed difference set of a worktree: changed paths only, never
/// content (content is the diff command's business, not the observer's).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct WorktreeStatus {
    pub uncommitted: Vec<String>,
    pub untracked: Vec<String>,
}

/// Reads HEAD: `symbolic-ref` for the branch, `rev-parse HEAD` for the
/// commit. Unborn branches are detected by `rev-parse --verify -q HEAD`
/// failing while `symbolic-ref` succeeds.
pub fn observe_head(
    git: &mut impl GitExecutor,
    worktree: &Path,
) -> Result<HeadObservation, GitError> {
    // symbolic-ref first: it distinguishes branch, detached, and (with
    // rev-parse) unborn. On an unborn branch symbolic-ref succeeds while
    // rev-parse --verify HEAD fails quietly.
    let branch = git.run(worktree, &["symbolic-ref", "--quiet", "--short", "HEAD"]);
    match branch {
        Err(GitError::GitRefused) => {
            // Detached: symbolic-ref fails quietly; HEAD still resolves.
            let commit_bytes = git.run(worktree, &["rev-parse", "--verify", "-q", "HEAD"])?;
            let commit = trimmed_hex(&commit_bytes)?;
            Ok(HeadObservation {
                commit,
                state: HeadState::Detached,
            })
        }
        Err(error) => Err(error),
        Ok(name_bytes) => {
            let name = trimmed_name(&name_bytes, "branch")?;
            let commit_bytes = git.run(worktree, &["rev-parse", "--verify", "-q", "HEAD"]);
            match commit_bytes {
                Ok(bytes) => {
                    let commit = trimmed_hex(&bytes)?;
                    Ok(HeadObservation {
                        commit,
                        state: HeadState::Branch { name },
                    })
                }
                Err(GitError::GitRefused) => {
                    // No commit yet: unborn branch.
                    Ok(HeadObservation {
                        commit: String::new(),
                        state: HeadState::Unborn,
                    })
                }
                Err(error) => Err(error),
            }
        }
    }
}

/// Reads the worktree's uncommitted and untracked paths via
/// `status --porcelain`. Path parsing follows porcelain v1 exactly: two
/// status columns, a space, then the path (rename entries carry `->` and
/// are recorded as the destination path).
pub fn observe_status(
    git: &mut impl GitExecutor,
    worktree: &Path,
) -> Result<WorktreeStatus, GitError> {
    let bytes = git.run(worktree, &["status", "--porcelain", "-z"])?;
    let text = std::str::from_utf8(&bytes).map_err(|_| GitError::MalformedOutput)?;
    let mut status = WorktreeStatus::default();
    // NUL-separated records; rename/copy records carry the origin as the
    // NEXT NUL-terminated field and the destination in this record's path.
    let mut records = text.split('\0');
    while let Some(record) = records.next() {
        if record.is_empty() {
            continue;
        }
        if record.len() < 4 {
            // Porcelain -z always emits two columns + a space before the
            // path; anything shorter is a truncated or foreign frame.
            return Err(GitError::MalformedOutput);
        }
        let (codes, path) = record.split_at(3);
        if path.is_empty() || path.len() > MAX_PATH_BYTES {
            return Err(GitError::MalformedOutput);
        }
        let is_rename = codes.contains('R') || codes.contains('C');
        if is_rename {
            match records.next() {
                Some(origin) if !origin.is_empty() => {
                    if origin.len() > MAX_PATH_BYTES {
                        return Err(GitError::MalformedOutput);
                    }
                }
                // A rename record without a non-empty origin path is a
                // truncated frame.
                _ => return Err(GitError::MalformedOutput),
            }
        }
        let clean_path = path.to_owned();
        let untracked = codes.starts_with("??");
        if untracked {
            if !status.untracked.contains(&clean_path) {
                status.untracked.push(clean_path);
            }
        } else if !status.uncommitted.contains(&clean_path) {
            status.uncommitted.push(clean_path);
        }
    }
    Ok(status)
}

/// Per-file bounds for the untracked content heads.
pub const MAX_UNTRACKED_FILE_BYTES: usize = 16 * 1024;
/// The maximum number of untracked files that get content heads.
pub const MAX_UNTRACKED_FILES: usize = 32;

/// The bounded diff evidence of a run in a worktree: the tracked unified
/// diff plus bounded content heads of the untracked paths. This is
/// PREVIEW evidence — rendered inert (escaped) downstream — and it is
/// size-capped at every layer: an oversized tracked diff degrades to the
/// `--stat` summary with an explicit flag, an oversized untracked file is
/// truncated with an explicit flag, and symlinked or non-regular paths
/// are never read (a bracketed placeholder names them instead).
///
/// Collection never fails the caller: a tracked diff that cannot be
/// observed degrades to a bracketed placeholder plus a
/// [`RunDiff::tracked_note`], and the untracked heads are collected
/// regardless. Losing every untracked head because one tracked byte was
/// not UTF-8 would make the preview less honest, not safer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct RunDiff {
    /// The tracked unified diff (`git diff HEAD`), the degraded `--stat`
    /// summary when the full diff exceeded the output bound, or a
    /// bracketed placeholder when the diff could not be observed at all
    /// (see [`RunDiff::tracked_note`]).
    pub tracked: String,
    /// True when `tracked` is the degraded `--stat` summary, not the
    /// full diff.
    pub tracked_truncated: bool,
    /// Why `tracked` is degraded — empty when it is the full diff. The
    /// `--stat` degradation is described by [`RunDiff::tracked_truncated`]
    /// instead, so the two never disagree.
    pub tracked_note: String,
    /// Untracked paths (at most [`MAX_UNTRACKED_FILES`], in status
    /// order) with bounded content heads.
    pub untracked: Vec<UntrackedContent>,
    /// How many further untracked paths the [`MAX_UNTRACKED_FILES`]
    /// bound left without a content head. Non-zero means this evidence
    /// is partial in a way its consumers must say out loud.
    pub untracked_omitted: usize,
}

/// One untracked path's bounded content head.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UntrackedContent {
    pub path: String,
    /// The file's content head (at most
    /// [`MAX_UNTRACKED_FILE_BYTES`]), or a bracketed placeholder when
    /// the content was not read (symlink, directory, non-UTF-8,
    /// unreadable).
    pub content: String,
    /// True when `content` is a truncated head.
    pub truncated: bool,
}

/// Observes the bounded diff evidence for a worktree whose changed-path
/// set is already known (from [`observe_status`]). Total by design: every
/// degradation is recorded in the returned [`RunDiff`] rather than
/// returned as an error, so one unobservable tracked diff cannot cost the
/// caller its untracked evidence.
pub fn observe_run_diff(
    git: &mut impl GitExecutor,
    worktree: &Path,
    status: &WorktreeStatus,
) -> RunDiff {
    let mut diff = RunDiff::default();
    // Untracked heads are filesystem reads only, so they are collected
    // first and survive any tracked-side failure.
    for path in status.untracked.iter().take(MAX_UNTRACKED_FILES) {
        diff.untracked.push(untracked_content(worktree, path));
    }
    diff.untracked_omitted = status.untracked.len().saturating_sub(MAX_UNTRACKED_FILES);
    observe_tracked_diff(git, worktree, &mut diff);
    diff
}

/// Fills `diff.tracked`, or records why it could not be filled. Git's
/// `diff` output is only non-UTF-8 when the changed content is (a binary
/// blob, a latin-1 source file): that is a fact about the run, so it is
/// named rather than dropped.
fn observe_tracked_diff(git: &mut impl GitExecutor, worktree: &Path, diff: &mut RunDiff) {
    match git.run(worktree, &["diff", "HEAD", "--"]) {
        Ok(bytes) => match std::str::from_utf8(&bytes) {
            Ok(text) => diff.tracked = text.to_owned(),
            Err(_) => degrade_tracked(
                diff,
                "<tracked diff is not UTF-8>",
                "the tracked diff is not UTF-8",
            ),
        },
        Err(GitError::OutputTooLarge) => {
            // Degrade honestly: the summary names what changed, the flag
            // says the full diff did not fit the bound.
            match git.run(worktree, &["diff", "HEAD", "--stat"]) {
                Ok(summary) => match std::str::from_utf8(&summary) {
                    Ok(text) => {
                        diff.tracked = text.to_owned();
                        diff.tracked_truncated = true;
                    }
                    Err(_) => degrade_tracked(
                        diff,
                        "<the --stat summary is not UTF-8>",
                        "the --stat summary is not UTF-8",
                    ),
                },
                Err(error) => degrade_tracked(
                    diff,
                    "<the --stat summary also failed>",
                    &format!(
                        "the --stat summary also failed after the full diff exceeded the bound: {error}"
                    ),
                ),
            }
        }
        // An unborn HEAD has nothing to diff against: that is not a
        // refusal for preview evidence — the tracked diff is empty. Every
        // other refusal is named.
        Err(GitError::GitRefused) => match head_is_unborn(git, worktree) {
            Ok(true) => {}
            Ok(false) => degrade_tracked(
                diff,
                "<git refused the tracked diff>",
                "git refused the tracked diff",
            ),
            Err(error) => degrade_tracked(
                diff,
                "<git refused the tracked diff>",
                &format!("git refused the tracked diff: {error}"),
            ),
        },
        Err(error) => degrade_tracked(
            diff,
            "<the tracked diff could not be observed>",
            &format!("the tracked diff could not be observed: {error}"),
        ),
    }
}

/// Records a degraded tracked diff: the placeholder is what renders, the
/// note says why. Never sets `tracked_truncated` — that flag means the
/// `--stat` summary specifically.
fn degrade_tracked(diff: &mut RunDiff, placeholder: &str, note: &str) {
    diff.tracked = placeholder.to_owned();
    diff.tracked_note = note.to_owned();
}

/// Distinguishes an unborn HEAD from a refusal, mirroring [`observe_head`]:
/// an unborn branch is one whose `symbolic-ref` resolves while
/// `rev-parse HEAD` refuses. A HEAD that resolves to nothing while other
/// refs exist is a broken HEAD, not an unborn branch, and `Ok(false)`
/// says so.
fn head_is_unborn(git: &mut impl GitExecutor, worktree: &Path) -> Result<bool, GitError> {
    match git.run(worktree, &["symbolic-ref", "--quiet", "--short", "HEAD"]) {
        // Detached HEAD, or not a repository at all: rev-parse decides.
        Err(GitError::GitRefused) => {
            match git.run(worktree, &["rev-parse", "--verify", "-q", "HEAD"]) {
                Ok(_) => Ok(false),
                Err(error) => Err(error),
            }
        }
        Err(error) => Err(error),
        Ok(_) => match git.run(worktree, &["rev-parse", "--verify", "-q", "HEAD"]) {
            Ok(_) => Ok(false),
            Err(GitError::GitRefused) => {
                // Nothing is reachable from HEAD. That is an unborn branch
                // only when the repository holds no refs at all.
                match git.run(worktree, &["show-ref", "--quiet"]) {
                    Err(GitError::GitRefused) => Ok(true),
                    Ok(_) => Ok(false),
                    Err(_) => Ok(false),
                }
            }
            Err(error) => Err(error),
        },
    }
}

/// Reads one untracked path's bounded content head. Never fails the
/// collection: an unreadable path yields an explicit placeholder (the
/// status list still names it).
///
/// The path list is an advertisement, not authority, so every component
/// is checked here instead of trusted: non-relative paths (joining an
/// absolute path REPLACES the base), `..` components and the C-quoted
/// porcelain form this crate's `-z` reads never produce are refused, and
/// the parent directory is resolved and must land inside the worktree, so
/// an intermediate symlinked directory cannot walk the read out of it.
///
/// The final component is then OPENED with `O_NOFOLLOW | O_NONBLOCK` and
/// classified by the opened handle's own metadata — never by a check on
/// the path that a rename could invalidate between the two calls. A
/// symlinked final component is refused by the kernel (ELOOP), and a FIFO
/// or device is neither waited on nor read.
fn untracked_content(worktree: &Path, path: &str) -> UntrackedContent {
    use std::io::Read;
    use std::os::unix::fs::OpenOptionsExt;
    let placeholder = |content: &str| UntrackedContent {
        path: path.to_owned(),
        content: content.to_owned(),
        truncated: false,
    };
    let relative = Path::new(path);
    if relative.is_absolute()
        || path.starts_with('"')
        || relative
            .components()
            .any(|component| matches!(component, std::path::Component::ParentDir))
    {
        return placeholder("<path not readable>");
    }
    let full = worktree.join(relative);
    let root = match std::fs::canonicalize(worktree) {
        Ok(root) => root,
        Err(_) => return placeholder("<unreadable>"),
    };
    let parent = match full
        .parent()
        .and_then(|parent| std::fs::canonicalize(parent).ok())
    {
        Some(parent) => parent,
        None => return placeholder("<unreadable>"),
    };
    if !parent.starts_with(&root) {
        return placeholder("<path outside the worktree>");
    }
    let file = match std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(
            nix::fcntl::OFlag::O_NOFOLLOW.bits()
                | nix::fcntl::OFlag::O_NONBLOCK.bits()
                | nix::fcntl::OFlag::O_CLOEXEC.bits(),
        )
        .open(&full)
    {
        Ok(file) => file,
        Err(_) => return placeholder("<unreadable or non-regular file>"),
    };
    // Classify what was actually opened: a directory open succeeds on
    // Linux, and a FIFO opened without O_NONBLOCK would never return.
    match file.metadata() {
        Ok(metadata) if metadata.is_file() => {}
        Ok(_) => return placeholder("<directory or non-regular file>"),
        Err(_) => return placeholder("<unreadable>"),
    }
    let mut head = Vec::new();
    let mut limited = file.take((MAX_UNTRACKED_FILE_BYTES + 1) as u64);
    if limited.read_to_end(&mut head).is_err() {
        return placeholder("<unreadable>");
    }
    let truncated = head.len() > MAX_UNTRACKED_FILE_BYTES;
    head.truncate(MAX_UNTRACKED_FILE_BYTES);
    match std::str::from_utf8(&head) {
        Ok(text) => UntrackedContent {
            path: path.to_owned(),
            content: text.to_owned(),
            truncated,
        },
        // The byte bound cut through a multi-byte character: keep the
        // complete character prefix. `error_len() == None` is exactly
        // "the input ends mid-character", so a genuinely invalid byte
        // still falls through to the placeholder below.
        Err(error) if truncated && error.error_len().is_none() && error.valid_up_to() > 0 => {
            UntrackedContent {
                path: path.to_owned(),
                content: String::from_utf8(head[..error.valid_up_to()].to_vec())
                    .expect("a valid-UTF-8 prefix is valid UTF-8"),
                truncated,
            }
        }
        Err(_) => placeholder("<non-utf8 content>"),
    }
}

fn trimmed_hex(bytes: &[u8]) -> Result<String, GitError> {
    let text = std::str::from_utf8(bytes).map_err(|_| GitError::MalformedOutput)?;
    let trimmed = text.trim();
    if trimmed.len() != 40 && trimmed.len() != 64 {
        return Err(GitError::MalformedOutput);
    }
    if !trimmed.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(GitError::MalformedOutput);
    }
    Ok(trimmed.to_ascii_lowercase())
}

fn trimmed_name(bytes: &[u8], what: &str) -> Result<String, GitError> {
    let _ = what;
    let text = std::str::from_utf8(bytes).map_err(|_| GitError::MalformedOutput)?;
    let trimmed = text.trim_end_matches('\n').trim();
    if trimmed.len() > MAX_BRANCH_BYTES {
        return Err(GitError::MalformedOutput);
    }
    if trimmed.bytes().any(|b| b == 0 || b < 0x20) {
        return Err(GitError::MalformedOutput);
    }
    Ok(trimmed.to_owned())
}

/// A materialization request: the reserved worktree (verified empty by the
/// reservation layer) receives a git worktree attached to the source
/// repository at the named branch, without contacting any remote.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MaterializeRequest<'a> {
    /// The source repository's Git directory (the canonical checkout).
    pub repository: &'a Path,
    /// The reserved, verified-empty worktree directory (the location layer's
    /// premise; the caller must have `verify`ed the reservation).
    pub worktree: &'a Path,
    /// The new branch name to create (Symbiote's reserved derived name).
    pub branch: &'a str,
    /// The start point the branch is created at: the caller-validated base
    /// commit. Branch creation is explicit and documented, never DWIM.
    pub start_point: &'a str,
}

/// Materializes `git worktree add <worktree> <branch>` against the source
/// repository. The worktree directory must exist and be empty (the
/// reservation layer's premise); git fills it. No remote contact: the
/// branch must already exist in the source repository.
pub fn materialize(
    git: &mut impl GitExecutor,
    request: MaterializeRequest<'_>,
) -> Result<HeadObservation, GitError> {
    if request.branch.is_empty() || request.branch.len() > MAX_BRANCH_BYTES {
        return Err(GitError::InvalidRequest);
    }
    // The work branch is CREATED here, explicitly, at the validated base
    // commit: `worktree add -b <branch> -- <path> <start-point>`. This is
    // not git's DWIM — the branch name is Symbiote's reserved derived name
    // and the start point is the caller-validated base SHA, so there is no
    // remote-tracking ambiguity and no silent ref mutation beyond the
    // documented branch creation.
    git.run(
        request.repository,
        &[
            "worktree",
            "add",
            "--no-checkout",
            "-b",
            request.branch,
            "--",
            path_arg(request.worktree)?,
            request.start_point,
        ],
    )?;
    // Populate the worktree from the branch HEAD (--no-checkout leaves it
    // empty of files; the explicit checkout fills it).
    git.run(request.worktree, &["checkout", request.branch])?;
    observe_head(git, request.worktree)
}

fn path_arg(path: &Path) -> Result<&str, GitError> {
    path.to_str().ok_or(GitError::MalformedOutput)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::path::PathBuf;

    /// A real temporary git repository with one commit on `main`.
    struct TempRepo {
        dir: PathBuf,
    }
    impl TempRepo {
        fn new(name: &str) -> Self {
            let dir =
                std::env::temp_dir().join(format!("symbiote-repo-{}-{name}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            let run = |args: &[&str]| {
                let output = Command::new("git")
                    .arg("-C")
                    .arg(&dir)
                    .args(args)
                    .output()
                    .unwrap();
                assert!(
                    output.status.success(),
                    "git {args:?} failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
            };
            run(&["init", "-q", "-b", "main"]);
            std::fs::write(dir.join("README.md"), format!("# {name}\n")).unwrap();
            run(&["add", "."]);
            run(&[
                "-c",
                "user.email=t@symbiote.test",
                "-c",
                "user.name=t",
                "commit",
                "-q",
                "-m",
                "init",
            ]);
            Self { dir }
        }
    }
    impl Drop for TempRepo {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.dir);
        }
    }

    /// Scripted executor: outputs consumed in order; empty = refused.
    struct Scripted {
        frames: VecDeque<Result<Vec<u8>, GitError>>,
        calls: Vec<(PathBuf, Vec<String>)>,
    }
    impl Scripted {
        fn new(frames: Vec<Result<Vec<u8>, GitError>>) -> Self {
            Self {
                frames: frames.into(),
                calls: Vec::new(),
            }
        }
    }
    impl GitExecutor for Scripted {
        fn run(&mut self, worktree: &Path, args: &[&str]) -> Result<Vec<u8>, GitError> {
            self.calls.push((
                worktree.to_path_buf(),
                args.iter().map(|s| s.to_string()).collect(),
            ));
            self.frames.pop_front().unwrap_or(Err(GitError::GitRefused))
        }
    }

    /// A recording executor for test call-count pins.
    #[derive(Default)]
    pub struct RecordingExecutor {
        pub calls: Vec<(PathBuf, Vec<String>)>,
    }
    impl GitExecutor for RecordingExecutor {
        fn run(&mut self, worktree: &Path, args: &[&str]) -> Result<Vec<u8>, GitError> {
            self.calls.push((
                worktree.to_path_buf(),
                args.iter().map(|s| s.to_string()).collect(),
            ));
            Ok(Vec::new())
        }
    }

    #[test]
    fn observe_head_reads_branch_detached_and_commit() {
        let repo = TempRepo::new("head");
        let mut git = SystemGit::new();
        let head = observe_head(&mut git, &repo.dir).unwrap();
        assert!(matches!(&head.state, HeadState::Branch { name } if name == "main"));
        assert_eq!(head.commit.len(), 40);
        // Detach and observe again.
        let run = |args: &[&str]| {
            Command::new("git")
                .arg("-C")
                .arg(&repo.dir)
                .args(args)
                .output()
                .unwrap()
        };
        assert!(run(&["checkout", "--detach", "HEAD"]).status.success());
        let head = observe_head(&mut git, &repo.dir).unwrap();
        assert_eq!(head.state, HeadState::Detached);
        // A fresh unborn repository classifies as unborn.
        let unborn_dir =
            std::env::temp_dir().join(format!("symbiote-repo-{}-unborn", std::process::id()));
        let _ = std::fs::remove_dir_all(&unborn_dir);
        std::fs::create_dir_all(&unborn_dir).unwrap();
        let _ = Command::new("git")
            .arg("-C")
            .arg(&unborn_dir)
            .arg("init")
            .arg("-q")
            .output()
            .unwrap();
        let head = observe_head(&mut git, &unborn_dir).unwrap();
        assert_eq!(head.state, HeadState::Unborn);
        let _ = std::fs::remove_dir_all(&unborn_dir);
    }

    #[test]
    fn observe_status_separates_uncommitted_from_untracked() {
        let repo = TempRepo::new("status");
        std::fs::write(repo.dir.join("README.md"), "changed\n").unwrap();
        std::fs::write(repo.dir.join("new.txt"), "new\n").unwrap();
        let mut git = SystemGit::new();
        let status = observe_status(&mut git, &repo.dir).unwrap();
        assert_eq!(status.uncommitted, vec!["README.md".to_string()]);
        assert_eq!(status.untracked, vec!["new.txt".to_string()]);
        // A clean tree observes nothing.
        let run = |args: &[&str]| {
            Command::new("git")
                .arg("-C")
                .arg(&repo.dir)
                .args(args)
                .output()
                .unwrap()
        };
        assert!(run(&["checkout", "--", "README.md"]).status.success());
        std::fs::remove_file(repo.dir.join("new.txt")).unwrap();
        let status = observe_status(&mut git, &repo.dir).unwrap();
        assert!(status.uncommitted.is_empty() && status.untracked.is_empty());
    }

    #[test]
    fn malformed_git_output_is_rejected_not_guessed() {
        // Non-hex commit (branch present first).
        let mut git = Scripted::new(vec![Ok(b"main\n".to_vec()), Ok(b"not-a-sha\n".to_vec())]);
        assert_eq!(
            observe_head(&mut git, Path::new("/w")),
            Err(GitError::MalformedOutput)
        );
        // Wrong commit length.
        let mut git = Scripted::new(vec![Ok(b"main\n".to_vec()), Ok(b"abc123\n".to_vec())]);
        assert_eq!(
            observe_head(&mut git, Path::new("/w")),
            Err(GitError::MalformedOutput)
        );
        // Control characters in a branch name.
        let mut git = Scripted::new(vec![Ok(b"bad\nname\n".to_vec())]);
        assert_eq!(
            observe_head(&mut git, Path::new("/w")),
            Err(GitError::MalformedOutput)
        );
        // Overlong branch name.
        let long = vec![b'a'; MAX_BRANCH_BYTES + 1];
        let mut git = Scripted::new(vec![Ok(long)]);
        assert_eq!(
            observe_head(&mut git, Path::new("/w")),
            Err(GitError::MalformedOutput)
        );
        // A status path over the path bound is rejected.
        let long_path = vec![b'x'; MAX_PATH_BYTES + 1];
        let mut record = b"?? ".to_vec();
        record.extend_from_slice(&long_path);
        let mut git = Scripted::new(vec![Ok(record)]);
        assert_eq!(
            observe_status(&mut git, Path::new("/w")),
            Err(GitError::MalformedOutput)
        );
        // A rename record without its origin path is a truncated frame.
        let mut git = Scripted::new(vec![Ok(b"R  new-name\0".to_vec())]);
        assert_eq!(
            observe_status(&mut git, Path::new("/w")),
            Err(GitError::MalformedOutput)
        );
    }

    #[test]
    fn porcelain_z_parses_renames_quotes_and_arrow_named_files() {
        let mut git = Scripted::new(vec![Ok({
            let mut records: Vec<u8> = Vec::new();
            // Untracked with an arrow in the name: NUL framing means the
            // arrow inside a filename can never be mistaken for a rename.
            records.extend_from_slice(b"?? untracked -> name.txt\0");
            // Rename record: origin path is the NEXT NUL field; destination
            // is this record's path.
            records.extend_from_slice(b"R  new-name\0old-name\0");
            // Quoted special-char path ( porcelain -z emits raw bytes, but
            // an explicitly quoted name from a foreign frame is recorded
            // verbatim — this crate does not C-unescape or guess).
            records.extend_from_slice(b" M \"r\xC3\xA9l\xC3\xA9sum.txt\"\0");
            records
        })]);
        let status = observe_status(&mut git, Path::new("/w")).unwrap();
        assert_eq!(status.untracked, vec!["untracked -> name.txt".to_string()]);
        assert_eq!(
            status.uncommitted,
            vec![
                "new-name".to_string(),
                "\"r\u{e9}l\u{e9}sum.txt\"".to_string(),
            ]
        );
    }

    #[test]
    fn scripted_materialize_pins_the_explicit_branch_creation() {
        // Branch creation is EXPLICIT (-b at the caller-validated start
        // point), never git's DWIM: the invocation must carry -b, the
        // reserved branch name, --, the worktree path, and the start point.
        let mut git = Scripted::new(vec![
            Ok(b"".to_vec()),                                           // worktree add -b
            Ok(b"".to_vec()),                                           // checkout
            Ok(b"task/stream\n".to_vec()),                              // symbolic-ref
            Ok(b"abc123abc123abc123abc123abc123abc123abcd\n".to_vec()), // rev-parse
        ]);
        let repository = Path::new("/source/repo");
        let worktree = Path::new("/base/proj/st-x");
        let head = materialize(
            &mut git,
            MaterializeRequest {
                repository,
                worktree,
                branch: "task/stream",
                start_point: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            },
        )
        .unwrap();
        assert!(matches!(&head.state, crate::HeadState::Branch { name } if name == "task/stream"));
        assert_eq!(git.calls[0].0, repository);
        assert_eq!(
            git.calls[0].1,
            vec![
                "worktree".to_string(),
                "add".to_string(),
                "--no-checkout".to_string(),
                "-b".to_string(),
                "task/stream".to_string(),
                "--".to_string(),
                worktree.to_string_lossy().to_string(),
                "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string(),
            ]
        );
        assert_eq!(git.calls[1].0, worktree);
        assert_eq!(git.calls[1].1[0], "checkout");
    }

    #[test]
    fn oversized_output_is_bounded_by_the_executor() {
        // A SystemGit run against a real git invocation producing over 256
        // KiB of stdout exercises the OutputTooLarge path end to end: two
        // 256 KiB tracked files staged as NEW (whole paths are short, but
        // each porcelain record for a NEW file carries the path only —
        // too short). Use `status --porcelain -z` after staging renames?
        // Simplest honest driver: git log -p on a large commit would dump
        // content, but observe_status is the parser under test. Instead:
        // create MANY new files so the NUL-joined records exceed 256 KiB.
        let repo = TempRepo::new("oversize");
        for index in 0..20_000 {
            std::fs::write(repo.dir.join(format!("new-file-{index}.txt")), "new\n").unwrap();
        }
        let run = |args: &[&str]| {
            Command::new("git")
                .arg("-C")
                .arg(&repo.dir)
                .args(args)
                .output()
                .unwrap()
        };
        assert!(run(&["add", "."]).status.success());
        let mut git = SystemGit::new();
        // 20,000 records x ~22 bytes = ~440 KiB of stdout > 256 KiB.
        assert_eq!(
            observe_status(&mut git, &repo.dir),
            Err(GitError::OutputTooLarge)
        );
    }

    /// Observes the bounded diff evidence: tracked hunks plus untracked
    /// content heads, from a REAL repository.
    #[test]
    fn run_diff_covers_tracked_and_untracked_changes() {
        let repo = TempRepo::new("run-diff");
        let mut git = SystemGit::new();
        std::fs::write(repo.dir.join("README.md"), "modified line\n").unwrap();
        std::fs::write(
            repo.dir.join("produced.txt"),
            "worker output\nsecond line\n",
        )
        .unwrap();
        let status = observe_status(&mut git, &repo.dir).unwrap();
        let diff = observe_run_diff(&mut git, &repo.dir, &status);
        assert!(
            diff.tracked.contains("+modified line"),
            "{:?}",
            diff.tracked
        );
        assert!(!diff.tracked_truncated);
        let produced = diff
            .untracked
            .iter()
            .find(|head| head.path == "produced.txt")
            .expect("untracked content head");
        assert_eq!(produced.content, "worker output\nsecond line\n");
        assert!(!produced.truncated);
        assert_eq!(diff.untracked_omitted, 0);
    }

    /// The byte bound cutting through a multi-byte character keeps the
    /// complete character prefix — a truncated head, never a bogus
    /// `<non-utf8 content>` placeholder for a UTF-8 file.
    #[test]
    fn a_boundary_cut_keeps_the_complete_character_prefix() {
        let repo = TempRepo::new("run-diff-utf8-bound");
        let mut git = SystemGit::new();
        // The leading ASCII byte makes the 16 KiB bound land inside one
        // of the two-byte characters.
        let text = format!("a{}", "é".repeat(MAX_UNTRACKED_FILE_BYTES));
        std::fs::write(repo.dir.join("accents.txt"), text.as_bytes()).unwrap();
        let status = observe_status(&mut git, &repo.dir).unwrap();
        let diff = observe_run_diff(&mut git, &repo.dir, &status);
        let head = &diff.untracked[0];
        assert!(head.truncated);
        assert!(!head.content.contains("non-utf8"), "{:?}", head.content);
        assert_eq!(head.content.len(), MAX_UNTRACKED_FILE_BYTES - 1);
        assert!(head.content.starts_with('a'));
        assert!(
            head.content
                .chars()
                .all(|character| character == 'a' || character == 'é'),
            "{:?}",
            head.content
        );
    }

    /// A genuinely invalid byte is named as non-UTF-8 content, never
    /// rendered as mojibake or as a silent truncation.
    #[test]
    fn non_utf8_untracked_content_is_a_placeholder() {
        let repo = TempRepo::new("run-diff-non-utf8");
        let mut git = SystemGit::new();
        std::fs::write(repo.dir.join("blob.bin"), [0xff, 0xfe, 0x00, 0x01]).unwrap();
        let status = observe_status(&mut git, &repo.dir).unwrap();
        let diff = observe_run_diff(&mut git, &repo.dir, &status);
        assert_eq!(diff.untracked[0].content, "<non-utf8 content>");
        assert!(!diff.untracked[0].truncated);
    }

    /// The content-head cap is explicit: the paths it left out are
    /// counted, never silently dropped.
    #[test]
    fn untracked_heads_are_capped_with_an_omitted_count() {
        let repo = TempRepo::new("run-diff-cap");
        let mut git = SystemGit::new();
        for index in 0..(MAX_UNTRACKED_FILES + 3) {
            std::fs::write(repo.dir.join(format!("file-{index:02}.txt")), "x\n").unwrap();
        }
        let status = observe_status(&mut git, &repo.dir).unwrap();
        let diff = observe_run_diff(&mut git, &repo.dir, &status);
        assert_eq!(diff.untracked.len(), MAX_UNTRACKED_FILES);
        assert_eq!(diff.untracked_omitted, 3);
    }

    /// An oversized untracked file is truncated with an explicit flag,
    /// never silently and never unbounded.
    #[test]
    fn oversized_untracked_files_are_truncated_with_a_flag() {
        let repo = TempRepo::new("run-diff-truncate");
        let mut git = SystemGit::new();
        let big = "x".repeat(MAX_UNTRACKED_FILE_BYTES * 3);
        std::fs::write(repo.dir.join("big.txt"), &big).unwrap();
        let status = observe_status(&mut git, &repo.dir).unwrap();
        let diff = observe_run_diff(&mut git, &repo.dir, &status);
        let head = &diff.untracked[0];
        assert_eq!(head.path, "big.txt");
        assert!(head.truncated);
        assert_eq!(head.content.len(), MAX_UNTRACKED_FILE_BYTES);
    }

    /// An oversized tracked diff degrades to the `--stat` summary with
    /// an explicit flag.
    #[test]
    fn oversized_tracked_diffs_degrade_to_the_stat_summary() {
        let repo = TempRepo::new("run-diff-stat");
        let mut git = SystemGit::new();
        // Three hundred kilobytes of changed lines exceed the 256 KiB
        // invocation bound for `git diff HEAD`.
        let big = format!("{}\n", "y".repeat(64)).repeat(300 * 1024 / 65 + 8);
        std::fs::write(repo.dir.join("README.md"), &big).unwrap();
        let status = observe_status(&mut git, &repo.dir).unwrap();
        let diff = observe_run_diff(&mut git, &repo.dir, &status);
        assert!(diff.tracked_truncated, "{:?}", diff.tracked);
        assert!(diff.tracked.contains("README.md"), "{:?}", diff.tracked);
        // The degradation is the summary, not a partial patch, and the
        // flag is the whole story: no note competes with it.
        assert!(!diff.tracked.contains("@@"), "{:?}", diff.tracked);
        assert!(diff.tracked_note.is_empty(), "{}", diff.tracked_note);
    }

    /// A symlinked untracked path is never read: `O_NOFOLLOW` refuses it
    /// in the kernel and the placeholder names it instead.
    #[test]
    fn symlinked_untracked_paths_are_never_read() {
        let repo = TempRepo::new("run-diff-symlink");
        let mut git = SystemGit::new();
        let host_secret = std::fs::read_to_string("/etc/hostname").unwrap();
        std::os::unix::fs::symlink("/etc/hostname", repo.dir.join("sneaky.txt")).unwrap();
        let status = observe_status(&mut git, &repo.dir).unwrap();
        let diff = observe_run_diff(&mut git, &repo.dir, &status);
        let head = &diff.untracked[0];
        assert_eq!(head.path, "sneaky.txt");
        assert_eq!(head.content, "<unreadable or non-regular file>");
        assert_ne!(head.content, host_secret);
    }

    /// A FIFO named by the status list cannot hang the preview: the open
    /// is non-blocking and the classification is of the opened handle.
    #[test]
    fn a_fifo_untracked_path_is_named_and_never_waited_on() {
        use std::sync::mpsc;
        let repo = TempRepo::new("run-diff-fifo");
        nix::unistd::mkfifo(
            &repo.dir.join("pipe"),
            nix::sys::stat::Mode::S_IRUSR | nix::sys::stat::Mode::S_IWUSR,
        )
        .unwrap();
        let dir = repo.dir.clone();
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            let mut git = SystemGit::new();
            let status = WorktreeStatus {
                uncommitted: Vec::new(),
                untracked: vec!["pipe".to_string()],
            };
            let _ = sender.send(observe_run_diff(&mut git, &dir, &status));
        });
        // Without O_NONBLOCK this open waits for a writer forever; the
        // bounded receive turns that regression into a failure.
        let diff = receiver
            .recv_timeout(Duration::from_secs(10))
            .expect("reading a FIFO must not wait for a writer");
        assert_eq!(diff.untracked[0].content, "<directory or non-regular file>");
    }

    /// An unborn HEAD (no commits) is not a refusal: the tracked diff
    /// is empty and untracked heads still collect.
    #[test]
    fn an_unborn_head_is_empty_tracked_diff_not_a_refusal() {
        let dir = std::env::temp_dir().join(format!("symbiote-repo-unborn-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let run = |args: &[&str]| {
            let output = Command::new("git")
                .arg("-C")
                .arg(&dir)
                .args(args)
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "git {args:?} failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        };
        run(&["init", "-q", "-b", "main"]);
        std::fs::write(dir.join("new.txt"), "content\n").unwrap();
        let mut git = SystemGit::new();
        let status = observe_status(&mut git, &dir).unwrap();
        let diff = observe_run_diff(&mut git, &dir, &status);
        assert!(diff.tracked.is_empty());
        assert!(!diff.tracked_truncated);
        assert!(diff.tracked_note.is_empty(), "{}", diff.tracked_note);
        assert_eq!(diff.untracked[0].path, "new.txt");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A tracked diff that is not UTF-8 (a binary blob, a latin-1 source
    /// file) is NAMED, and the untracked heads are still collected: one
    /// bad tracked byte must not cost the whole preview.
    #[test]
    fn a_non_utf8_tracked_diff_is_named_and_untracked_heads_survive() {
        let repo = TempRepo::new("run-diff-tracked-non-utf8");
        let mut git = SystemGit::new();
        // 0xff with no NUL: git treats the file as text and emits the raw
        // byte in the patch, so the diff output is not UTF-8.
        std::fs::write(repo.dir.join("README.md"), b"line\n\xff\n").unwrap();
        std::fs::write(repo.dir.join("produced.txt"), "worker output\n").unwrap();
        let status = observe_status(&mut git, &repo.dir).unwrap();
        let diff = observe_run_diff(&mut git, &repo.dir, &status);
        assert_eq!(diff.tracked, "<tracked diff is not UTF-8>");
        assert!(
            diff.tracked_note.contains("not UTF-8"),
            "{}",
            diff.tracked_note
        );
        assert!(!diff.tracked_truncated);
        assert_eq!(diff.untracked.len(), 1);
        assert_eq!(diff.untracked[0].content, "worker output\n");
    }

    /// A HEAD that resolves to nothing while the repository holds other
    /// refs is BROKEN, not unborn: it is named instead of previewing as
    /// "nothing changed".
    #[test]
    fn a_broken_head_is_named_rather_than_reported_as_empty() {
        let repo = TempRepo::new("run-diff-broken-head");
        let mut git = SystemGit::new();
        std::fs::write(repo.dir.join("README.md"), "changed\n").unwrap();
        let output = Command::new("git")
            .arg("-C")
            .arg(&repo.dir)
            .args(["symbolic-ref", "HEAD", "refs/heads/no-such-branch"])
            .output()
            .unwrap();
        assert!(output.status.success());
        let status = observe_status(&mut git, &repo.dir).unwrap();
        let diff = observe_run_diff(&mut git, &repo.dir, &status);
        assert_eq!(diff.tracked, "<git refused the tracked diff>");
        assert!(!diff.tracked_note.is_empty(), "the refusal must be named");
        assert!(!diff.tracked_truncated);
    }

    /// The path list is an advertisement, not authority: an absolute path
    /// (which REPLACES the base when joined) is refused and never read.
    #[test]
    fn an_absolute_untracked_path_is_never_read() {
        let repo = TempRepo::new("run-diff-absolute");
        let mut git = SystemGit::new();
        let host_secret = std::fs::read_to_string("/etc/hostname").unwrap();
        let status = WorktreeStatus {
            uncommitted: Vec::new(),
            untracked: vec!["/etc/hostname".to_string()],
        };
        let diff = observe_run_diff(&mut git, &repo.dir, &status);
        assert_eq!(diff.untracked[0].content, "<path not readable>");
        assert_ne!(diff.untracked[0].content, host_secret);
    }

    /// An intermediate symlinked directory cannot walk the read out of the
    /// worktree, even when the status list names a path beneath it.
    #[test]
    fn a_symlinked_parent_directory_is_never_followed() {
        let repo = TempRepo::new("run-diff-parent-link");
        let mut git = SystemGit::new();
        let outside =
            std::env::temp_dir().join(format!("symbiote-repo-outside-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&outside);
        std::fs::create_dir_all(&outside).unwrap();
        std::fs::write(outside.join("secret.txt"), "OUTSIDE-SECRET\n").unwrap();
        std::os::unix::fs::symlink(&outside, repo.dir.join("link")).unwrap();
        let status = WorktreeStatus {
            uncommitted: Vec::new(),
            untracked: vec!["link/secret.txt".to_string()],
        };
        let diff = observe_run_diff(&mut git, &repo.dir, &status);
        assert_eq!(diff.untracked[0].content, "<path outside the worktree>");
        let _ = std::fs::remove_dir_all(&outside);
    }

    /// A three-byte character cut by the bound keeps its complete prefix
    /// too (the two-byte case has its own test).
    #[test]
    fn a_three_byte_character_cut_keeps_the_complete_prefix() {
        let repo = TempRepo::new("run-diff-utf8-bound-3");
        let mut git = SystemGit::new();
        // Two leading ASCII bytes: 16384 - 2 = 16382 = 3 x 5460 + 2, so the
        // bound lands two bytes into a three-byte character.
        let text = format!("ab{}", "€".repeat(MAX_UNTRACKED_FILE_BYTES));
        std::fs::write(repo.dir.join("euros.txt"), text.as_bytes()).unwrap();
        let status = observe_status(&mut git, &repo.dir).unwrap();
        let diff = observe_run_diff(&mut git, &repo.dir, &status);
        let head = &diff.untracked[0];
        assert!(head.truncated);
        assert_eq!(head.content.len(), MAX_UNTRACKED_FILE_BYTES - 2);
        assert!(!head.content.contains("non-utf8"), "{:?}", head.content);
        assert!(head.content.starts_with("ab"));
    }

    /// A filename that merely CONTAINS a quote is read — porcelain `-z`
    /// never quotes — while the C-quoted form (always a leading quote) is
    /// refused.
    #[test]
    fn an_interior_quote_is_read_but_a_quoted_porcelain_path_is_refused() {
        let repo = TempRepo::new("run-diff-quote");
        let mut git = SystemGit::new();
        std::fs::write(repo.dir.join("od\"d.txt"), "quoted name\n").unwrap();
        let status = observe_status(&mut git, &repo.dir).unwrap();
        let diff = observe_run_diff(&mut git, &repo.dir, &status);
        assert_eq!(diff.untracked[0].path, "od\"d.txt");
        assert_eq!(diff.untracked[0].content, "quoted name\n");
        let quoted = WorktreeStatus {
            uncommitted: Vec::new(),
            untracked: vec!["\"od\\303\\251.txt\"".to_string()],
        };
        let diff = observe_run_diff(&mut git, &repo.dir, &quoted);
        assert_eq!(diff.untracked[0].content, "<path not readable>");
    }
}
