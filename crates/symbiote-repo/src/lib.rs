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
//! submodules/LFS/sparse-checkout, status/diff/history normalization and
//! safe multi-step transactions remain pending on #190.
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
}
