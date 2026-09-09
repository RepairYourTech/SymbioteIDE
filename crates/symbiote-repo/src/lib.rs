//! Repository observation and worktree materialization for plain Git
//! repositories (#190 foundation): the canonical facts the worker loops and
//! dispatch preparation need before touching a reserved worktree. This crate
//! executes `git` as an external process with bounded frames and deadlines
//! (the same subprocess discipline as the runtime transport), and treats
//! every observation as an advertisement — nothing here authorizes access,
//! grants permissions, or claims remote identity.
//!
//! Boundary: this is the observation/materialization layer only. It never
//! pushes, pulls, fetches, commits on the user's behalf, or contacts a
//! remote (no network in any supported invocation). Repository identity
//! stays with the canonical Root records; this crate never second-guesses
//! them. Credential delegation, remotes, submodules/LFS/sparse-checkout,
//! status/diff/history normalization and safe multi-step transactions
//! remain pending on #190.
//!
//! Everything is offline-testable: `GitExecutor` is injected, and tests use
//! the real `git` binary against temporary repositories plus scripted
//! failure executors.
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
    let bytes = git.run(worktree, &["status", "--porcelain"])?;
    let text = std::str::from_utf8(&bytes).map_err(|_| GitError::MalformedOutput)?;
    let mut status = WorktreeStatus::default();
    for line in text.lines() {
        if line.len() < 4 {
            continue;
        }
        let (codes, path) = line.split_at(3);
        let path = path.trim_start();
        if path.is_empty() || path.len() > MAX_PATH_BYTES {
            return Err(GitError::MalformedOutput);
        }
        let path = rename_destination(path);
        let untracked = codes.starts_with("??");
        let clean_path = path.to_owned();
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

fn rename_destination(path: &str) -> &str {
    match path.find(" -> ") {
        Some(index) => &path[index + 4..],
        None => path,
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
    /// The fully-qualified branch to check out in the new worktree.
    pub branch: &'a str,
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
        return Err(GitError::MalformedOutput);
    }
    // `git worktree add` needs the target path and branch. The executor is
    // rooted at the source repository for this invocation.
    git.run(
        request.repository,
        &[
            "worktree",
            "add",
            "--no-checkout",
            "--",
            path_arg(request.worktree)?,
            request.branch,
        ],
    )?;
    // Checkout the branch contents into the new worktree (git worktree add
    // with --no-checkout leaves it bare of files; `checkout` fills it).
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
        let mut line = b"?? ".to_vec();
        line.extend_from_slice(&long_path);
        line.push(b'\n');
        let mut git = Scripted::new(vec![Ok(line)]);
        assert_eq!(
            observe_status(&mut git, Path::new("/w")),
            Err(GitError::MalformedOutput)
        );
    }

    #[test]
    fn git_refusals_surface_as_git_refused_not_executed() {
        let mut git = Scripted::new(vec![Err(GitError::GitRefused)]);
        assert_eq!(
            observe_head(&mut git, Path::new("/w")),
            Err(GitError::GitRefused)
        );
        let mut git = Scripted::new(vec![]);
        assert_eq!(
            observe_head(&mut git, Path::new("/w")),
            Err(GitError::GitRefused)
        );
    }

    #[test]
    fn materialize_creates_a_worktree_on_the_named_branch() {
        let repo = TempRepo::new("materialize");
        // Create a second branch with distinct content.
        let run = |args: &[&str]| {
            Command::new("git")
                .arg("-C")
                .arg(&repo.dir)
                .args(args)
                .output()
                .unwrap()
        };
        assert!(
            run(&["checkout", "-q", "-b", "task/stream"])
                .status
                .success()
        );
        std::fs::write(repo.dir.join("feature.txt"), "from task stream\n").unwrap();
        assert!(run(&["add", "."]).status.success());
        assert!(
            run(&[
                "-c",
                "user.email=t@symbiote.test",
                "-c",
                "user.name=t",
                "commit",
                "-q",
                "-m",
                "task"
            ])
            .status
            .success()
        );
        assert!(run(&["checkout", "-q", "main"]).status.success());
        // The reserved empty worktree directory.
        let worktree = std::env::temp_dir().join(format!(
            "symbiote-repo-{}-materialize-wt",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&worktree);
        std::fs::create_dir_all(&worktree).unwrap();
        assert!(std::fs::read_dir(&worktree).unwrap().next().is_none());
        let mut git = SystemGit::new();
        let head = materialize(
            &mut git,
            MaterializeRequest {
                repository: &repo.dir,
                worktree: &worktree,
                branch: "task/stream",
            },
        )
        .unwrap();
        assert!(matches!(&head.state, HeadState::Branch { name } if name == "task/stream"));
        // The materialized tree contains the branch's file, not main's.
        assert!(worktree.join("feature.txt").exists());
        // Materializing over a non-empty worktree fails (git itself refuses).
        let again = materialize(
            &mut git,
            MaterializeRequest {
                repository: &repo.dir,
                worktree: &worktree,
                branch: "main",
            },
        );
        assert!(again.is_err());
        // Materializing a branch that does not exist fails.
        let empty = std::env::temp_dir().join(format!(
            "symbiote-repo-{}-materialize-empty",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&empty);
        std::fs::create_dir_all(&empty).unwrap();
        let missing = materialize(
            &mut git,
            MaterializeRequest {
                repository: &repo.dir,
                worktree: &empty,
                branch: "no-such-branch",
            },
        );
        assert!(missing.is_err());
        let _ = std::fs::remove_dir_all(&worktree);
        let _ = std::fs::remove_dir_all(&empty);
        let _ = run(&["worktree", "prune"]);
    }

    #[test]
    fn scripted_materialize_verifies_the_exact_invocations() {
        // The executor sees worktree add rooted at the repository, then
        // checkout rooted at the new worktree, then the head observation.
        let mut git = Scripted::new(vec![
            Ok(b"".to_vec()),                                           // worktree add
            Ok(b"".to_vec()),                                           // checkout
            Ok(b"task/stream\n".to_vec()),                              // symbolic-ref (first)
            Ok(b"abc123abc123abc123abc123abc123abc123abcd\n".to_vec()), // rev-parse (40 hex)
        ]);
        let repository = Path::new("/source/repo");
        let worktree = Path::new("/base/proj/st-x");
        let head = materialize(
            &mut git,
            MaterializeRequest {
                repository,
                worktree,
                branch: "task/stream",
            },
        )
        .unwrap();
        assert!(matches!(&head.state, HeadState::Branch { name } if name == "task/stream"));
        assert_eq!(git.calls[0].0, repository);
        assert_eq!(
            git.calls[0].1[..3],
            [
                "worktree".to_string(),
                "add".to_string(),
                "--no-checkout".to_string()
            ]
        );
        assert_eq!(git.calls[1].0, worktree);
        assert_eq!(git.calls[1].1[0], "checkout");
    }

    #[test]
    fn oversized_output_is_bounded() {
        // A scripted executor that claims an over-bound output is rejected
        // by the parser bound on branch names too, but the output bound
        // belongs to the executor; a real SystemGit against a huge `status`
        // would return OutputTooLarge. Here we pin the constant's contract:
        // the observer rejects facts over their own bounds instead.
        let huge = vec![b'a'; MAX_GIT_OUTPUT_BYTES + 1];
        let mut git = Scripted::new(vec![Ok(huge)]);
        // The branch-name bound fires before any larger processing.
        assert_eq!(
            observe_head(&mut git, Path::new("/w")),
            Err(GitError::MalformedOutput)
        );
    }
}
