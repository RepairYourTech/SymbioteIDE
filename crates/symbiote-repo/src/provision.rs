//! Dispatch-time worktree provisioning (#211): the composition point where
//! a Change Stream's reserved location is verified, its source repository's
//! base commit is validated against the stream's recorded base, and the
//! stream's worktree is materialized for a started dispatch.
//!
//! Ordering is the security contract:
//! 1. **Establish the reservation** — the marker must match the derived
//!    identity exactly; a foreign or tampered location fails loudly. A
//!    location this stream already holds is not re-claimed, it is
//!    re-proven: see `provision` for what reuse additionally requires.
//! 2. **Validate the base** — the source repository's actual HEAD must
//!    resolve and equal the stream's recorded base commit. A moved base
//!    means the stream's recorded premise is stale: refusal, never a
//!    materialization against unknown ground.
//! 3. **Materialize, or reuse** — `git worktree add` onto the
//!    verified-empty location at the stream's derived branch, then observe
//!    the result. A location that already holds a checkout is reused only
//!    when git registers that checkout at exactly this path on exactly this
//!    stream's branch.
//!
//! Every step here refuses before it acts, and a refusal in any of them
//! leaves the location exactly as it found it: a refused run never takes
//! work with it, which is what lets the operator fix the cause and run
//! again.
//!
//! The composition takes the canonical identities only (stream record,
//! root host path, reservation base directory, policy seed). It never
//! accepts caller-supplied paths: the location derives from the same
//! identities the store recorded, and the source repository path comes
//! from the Root's host-path placement for this Host.
//!
//! Errors are honest about the stage that failed — reservation, base
//! validation, or materialization — so retry policy can distinguish "the
//! location is compromised" from "the stream's base moved" from "git
//! refused".
use crate::HeadObservation;
use std::path::{Path, PathBuf};
use symbiote_domain::{ChangeStream, CommitSha, RootId};

/// Which stage of provisioning failed. The stage matters: a reservation
/// failure is a security event (the location did not verify), a base
/// failure is a stale-premise event (rebase/revalidation territory), and a
/// materialization failure is an execution event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProvisionError {
    /// The reserved location did not verify against the derived identity.
    Reservation,
    /// The source repository's HEAD does not resolve.
    BaseUnresolved,
    /// The source repository's HEAD moved off the stream's recorded base.
    BaseMoved,
    /// Git refused the materialization or observation.
    Git(crate::GitError),
    /// The Root record carries no placement for this Host.
    NoHostPath,
}

impl std::fmt::Display for ProvisionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "worktree provisioning failed: {self:?}")
    }
}
impl std::error::Error for ProvisionError {}

/// The provisioning outcome: the verified materialized worktree and the
/// observed HEAD after materialization (the branch tip the work starts
/// from — recorded by the caller for the dispatch's evidence trail).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Provisioned {
    pub worktree: PathBuf,
    pub branch: String,
    pub head: HeadObservation,
}

/// The inputs, all canonical: the stream's recorded identities (base,
/// branch, worktree id), the derived reservation location (identity, base
/// dir), the source repository path from the Root's Host placement, and
/// the policy seed binding the derivation.
pub struct ProvisionInputs<'a> {
    pub stream: &'a ChangeStream,
    pub root_id: &'a RootId,
    pub project_id: &'a symbiote_domain::ProjectId,
    pub stream_id: &'a symbiote_domain::ChangeStreamId,
    pub policy_seed: &'a str,
    pub reservation_base: &'a Path,
    /// The source repository checkout on this Host (from the Root's
    /// host_paths placement, resolved by the caller from trusted state).
    pub source_repository: &'a Path,
}

impl<'a> ProvisionInputs<'a> {
    /// The derived worktree location under the reservation base — the same
    /// path `provision` materializes into, exposed for callers that need it
    /// beforehand (e.g. sandbox launch arguments).
    pub fn derived_worktree_path(&self, base: &Path) -> PathBuf {
        symbiote_worktrees::Derived::derive(symbiote_worktrees::DeriveInputs {
            project_id: self.project_id,
            root_id: self.root_id,
            stream_id: self.stream_id,
            seed: self.policy_seed,
        })
        .map(|derived| derived.worktree_path(base))
        .unwrap_or_else(|_| base.to_path_buf())
    }
}

/// Runs the three-stage provisioning for one dispatch, and is idempotent for
/// the one case that has to be: a stream that already holds its location.
///
/// "Already holds it" is proven, not assumed, and by the same facts the first
/// claim was: the marker must bind this derived identity, every path
/// component must still be private, and the location must be the derived one.
/// What is NOT proven by that is where the content came from, so a location
/// that holds anything further has to prove itself twice more before a run may
/// execute in it — git must have a worktree registered at exactly that path,
/// and that worktree's HEAD must be the stream's derived branch. Content that
/// fails any of the four is refused, which is what it always did.
///
/// The reason this exists at all is that a Task has exactly one Change Stream
/// for its whole life. `recover_task` is the only way out of a failed or
/// interrupted run, and its whole purpose is to make the task dispatchable
/// again — so a contract that refuses to provision a location twice makes the
/// recovery a trap: the task goes from `failed`, with a readable reason and a
/// reclaimable checkout, to `ready`, and every run of it from then on refuses
/// with `Reservation`. The same held location is what a crashed run leaves
/// behind, and what the operator's second attempt lands on. A Task that can
/// only ever be run once is not a Task with a recovery operation.
///
/// The four states this distinguishes, and what each one is:
/// - no marker, no directory: the first run. Claim, then materialize.
/// - marker, empty directory: a run that claimed and then refused to
///   materialize (a moved base, most often). The operator fixed the cause and
///   is running again, so this materializes like a first run.
/// - marker, content, registered on the derived branch: this stream's own
///   checkout from an earlier run, uncommitted output and all. Reused, because
///   the alternative is a task that can never run twice, and the content is
///   the same stream's work in the same stream's directory.
/// - marker, content, anything else: content this Host cannot attribute to the
///   stream. Refused.
pub fn provision(
    git: &mut impl crate::GitExecutor,
    inputs: ProvisionInputs<'_>,
) -> Result<Provisioned, ProvisionError> {
    // The derived identity must reproduce the stream's own worktree id and
    // branch: the store recorded these at Task creation from the same seed.
    let derived = symbiote_worktrees::Derived::derive(symbiote_worktrees::DeriveInputs {
        project_id: inputs.project_id,
        root_id: inputs.root_id,
        stream_id: inputs.stream_id,
        seed: inputs.policy_seed,
    })
    .map_err(|_| ProvisionError::Reservation)?;
    if derived.worktree_id != *inputs.stream.worktree_id() {
        return Err(ProvisionError::Reservation);
    }
    if derived.branch != inputs.stream.branch {
        return Err(ProvisionError::Reservation);
    }
    // The reservation is addressed by identity, not by whether this call is
    // the first one to ask for it. `verify_reservation` answers "does this
    // stream still hold this location" and, unlike `verify`, does not care that
    // the location has content in it: emptiness is a premise for handing the
    // directory to git, not a property of holding it.
    let reservation = symbiote_worktrees::Reservation {
        derived: derived.clone(),
        base: inputs.reservation_base.to_path_buf(),
    };
    let held = symbiote_worktrees::verify_reservation(&reservation).is_ok();
    if !held {
        // Nothing of this stream's is here, so claim it. A base that does not
        // exist yet is created by this call, and every refusal it can make
        // (a foreign-owned base, a symlinked component, a marker binding
        // another identity) is the refusal it made before.
        symbiote_worktrees::reserve(&derived, inputs.reservation_base, wall_clock_now())
            .map_err(|_| ProvisionError::Reservation)?;
        symbiote_worktrees::verify(&reservation).map_err(|_| ProvisionError::Reservation)?;
    }
    // Base validation against the source repository's actual HEAD.
    let head = crate::observe_head(git, inputs.source_repository).map_err(|error| match error {
        crate::GitError::GitRefused => ProvisionError::BaseUnresolved,
        other => ProvisionError::Git(other),
    })?;
    // Unborn classification must precede the base comparison: an unborn
    // HEAD has an empty commit string, which would otherwise read as
    // BaseMoved and send "repo has no commits" into rebase/revalidation
    // territory instead of the honest "does not resolve".
    if head.commit.is_empty() {
        return Err(ProvisionError::BaseUnresolved);
    }
    if !head
        .commit
        .eq_ignore_ascii_case(inputs.stream.base().as_str())
    {
        return Err(ProvisionError::BaseMoved);
    }
    let worktree = inputs.derived_worktree_path(inputs.reservation_base);
    if held {
        // The location is this stream's, so the only question left is whether
        // what is in it is too. `verify` is the emptiness test: a held
        // location with nothing in it is a claim whose materialization never
        // happened, and the run that needs it gets it.
        if symbiote_worktrees::verify(&reservation).is_err() {
            let registered = crate::registered_worktrees(git, inputs.source_repository)
                .map_err(ProvisionError::Git)?
                .contains(&worktree);
            if !registered {
                // Content in a location git does not know about. Whatever put
                // it there, this Host cannot show that this stream did.
                return Err(ProvisionError::Reservation);
            }
            let head = crate::observe_head(git, &worktree).map_err(ProvisionError::Git)?;
            match &head.state {
                crate::HeadState::Branch { name } if *name == derived.branch => {
                    return Ok(Provisioned {
                        worktree,
                        branch: derived.branch,
                        head,
                    });
                }
                // A detached HEAD, an unborn branch, or some other branch: a
                // checkout in the right place on the wrong ref is not this
                // stream's work either.
                _ => return Err(ProvisionError::Reservation),
            }
        }
    }
    let observation = crate::materialize(
        git,
        crate::MaterializeRequest {
            repository: inputs.source_repository,
            worktree: &worktree,
            branch: &derived.branch,
            start_point: inputs.stream.base().as_str(),
        },
    )
    .map_err(ProvisionError::Git)?;
    Ok(Provisioned {
        worktree,
        branch: derived.branch,
        head: observation,
    })
}

fn wall_clock_now() -> symbiote_domain::Timestamp {
    symbiote_domain::Timestamp(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or_default() as u64,
    )
}

/// CommitSha equality helper re-exported for the caller's evidence trail.
pub fn commit_matches(observed: &str, recorded: &CommitSha) -> bool {
    observed.eq_ignore_ascii_case(recorded.as_str())
}

/// Path helper: the derived worktree location under a base, for callers
/// that need the path before provisioning (e.g. sandbox launch arguments).
pub fn derived_worktree_path(
    project_id: &symbiote_domain::ProjectId,
    root_id: &RootId,
    stream_id: &symbiote_domain::ChangeStreamId,
    policy_seed: &str,
    base: &Path,
) -> Result<PathBuf, ProvisionError> {
    let derived = symbiote_worktrees::Derived::derive(symbiote_worktrees::DeriveInputs {
        project_id,
        root_id,
        stream_id,
        seed: policy_seed,
    })
    .map_err(|_| ProvisionError::Reservation)?;
    Ok(derived.worktree_path(base))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SystemGit;
    use std::process::Command;
    use symbiote_domain::{ChangeStreamId, ProjectId};

    /// A real source repository with two commits on `main` and a
    /// `task/stream` branch off the first commit.
    struct SourceRepo {
        dir: PathBuf,
        base_sha: String,
    }
    impl SourceRepo {
        fn new(tag: &str) -> Self {
            let dir =
                std::env::temp_dir().join(format!("symbiote-prov-{}-{tag}", std::process::id()));
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
                String::from_utf8_lossy(&output.stdout).trim().to_owned()
            };
            run(&["init", "-q", "-b", "main"]);
            std::fs::write(dir.join("base.txt"), "base content\n").unwrap();
            run(&["add", "."]);
            let _ = run(&[
                "-c",
                "user.email=t@symbiote.test",
                "-c",
                "user.name=t",
                "commit",
                "-q",
                "-m",
                "base",
            ]);
            let base_sha = run(&["rev-parse", "HEAD"]);
            run(&["checkout", "-q", "-b", "task/stream"]);
            std::fs::write(dir.join("work.txt"), "work content\n").unwrap();
            run(&["add", "."]);
            run(&[
                "-c",
                "user.email=t@symbiote.test",
                "-c",
                "user.name=t",
                "commit",
                "-q",
                "-m",
                "work",
            ]);
            run(&["checkout", "-q", "main"]);
            Self { dir, base_sha }
        }
    }
    impl Drop for SourceRepo {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.dir);
        }
    }

    const SEED: &str = "test-seed";

    /// Recomputes the derived branch the way the stream fixture records it:
    /// the test must use the store's own derivation, not a parallel one.
    fn derived_branch(project: &ProjectId, root: &RootId, stream: &ChangeStreamId) -> String {
        symbiote_worktrees::Derived::derive(symbiote_worktrees::DeriveInputs {
            project_id: project,
            root_id: root,
            stream_id: stream,
            seed: SEED,
        })
        .unwrap()
        .branch
    }

    #[test]
    fn provision_materializes_at_the_recorded_base_and_branch() {
        let repo = SourceRepo::new("happy");
        let project = ProjectId::new("prov-project").unwrap();
        let root = RootId::new("prov-root").unwrap();
        let stream_id = ChangeStreamId::new("prov-stream").unwrap();
        let branch = derived_branch(&project, &root, &stream_id);
        // The recorded base is the repo's HEAD (main = the base commit).
        let base = CommitSha::new(repo.base_sha.clone()).unwrap();
        // Build a stream-shaped record via NewChangeStream? That needs the
        // full change stream constructor; instead observe the real API by
        // provisioning against a stream whose base/branch/worktree match.
        // ChangeStream::new requires a chat and target; supply a target of
        // HEAD (integration happens later in the stream lifecycle).
        let target = CommitSha::new(repo.base_sha.clone()).unwrap();
        let stream = ChangeStream::new(symbiote_domain::NewChangeStream {
            id: stream_id.clone(),
            project_id: project.clone(),
            root_id: root.clone(),
            tasks: std::collections::BTreeSet::from([
                symbiote_domain::TaskId::new("prov-task").unwrap()
            ]),
            originating_chat: symbiote_domain::ChatId::new("chat-prov").unwrap(),
            worktree: symbiote_worktrees::Derived::derive(symbiote_worktrees::DeriveInputs {
                project_id: &project,
                root_id: &root,
                stream_id: &stream_id,
                seed: SEED,
            })
            .unwrap()
            .worktree_id,
            branch: branch.clone(),
            lineage: symbiote_domain::StreamLineage::Independent,
            base: base.clone(),
            target,
        })
        .unwrap();
        let reservation_base =
            std::env::temp_dir().join(format!("symbiote-prov-base-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&reservation_base);
        std::fs::create_dir_all(&reservation_base).unwrap();
        let mut git = SystemGit::new();
        let provisioned = provision(
            &mut git,
            ProvisionInputs {
                stream: &stream,
                root_id: &root,
                project_id: &project,
                stream_id: &stream_id,
                policy_seed: SEED,
                reservation_base: &reservation_base,
                source_repository: &repo.dir,
            },
        )
        .unwrap();
        assert_eq!(provisioned.branch, branch);
        // The worktree HEAD is the new branch AT the validated base: work
        // builds from there.
        assert_eq!(provisioned.head.commit, repo.base_sha);
        assert!(
            matches!(&provisioned.head.state, crate::HeadState::Branch { name } if name == &provisioned.branch)
        );
        // The materialized tree contains the base content and none of the
        // source repo's later work.
        assert!(provisioned.worktree.join("base.txt").exists());
        assert!(!provisioned.worktree.join("work.txt").exists());
        // Re-provisioning the same stream: this is the retry `recover_task`
        // exists to enable, and it is the case the reuse branch answers. The
        // assertion lives in the reuse test below, which is about it in full.
        let _ = std::fs::remove_dir_all(&reservation_base);
    }

    #[test]
    fn provision_refuses_when_the_recorded_base_has_moved() {
        let repo = SourceRepo::new("moved");
        let project = ProjectId::new("prov-moved").unwrap();
        let root = RootId::new("prov-moved-root").unwrap();
        let stream_id = ChangeStreamId::new("prov-moved-stream").unwrap();
        let branch = derived_branch(&project, &root, &stream_id);
        // Record a base that is NOT the repo's HEAD: advance main by a
        // commit, then record the ORIGINAL head as the stream's base.
        let run = |args: &[&str]| {
            let output = Command::new("git")
                .arg("-C")
                .arg(&repo.dir)
                .args(args)
                .output()
                .unwrap();
            assert!(output.status.success());
            String::from_utf8_lossy(&output.stdout).trim().to_owned()
        };
        let stale_base = CommitSha::new(repo.base_sha.clone()).unwrap();
        std::fs::write(repo.dir.join("advance.txt"), "moved ahead\n").unwrap();
        run(&["add", "."]);
        run(&[
            "-c",
            "user.email=t@symbiote.test",
            "-c",
            "user.name=t",
            "commit",
            "-q",
            "-m",
            "advance",
        ]);
        let stream = ChangeStream::new(symbiote_domain::NewChangeStream {
            id: stream_id.clone(),
            project_id: project.clone(),
            root_id: root.clone(),
            tasks: std::collections::BTreeSet::from([symbiote_domain::TaskId::new(
                "prov-moved-task",
            )
            .unwrap()]),
            originating_chat: symbiote_domain::ChatId::new("chat-moved").unwrap(),
            worktree: symbiote_worktrees::Derived::derive(symbiote_worktrees::DeriveInputs {
                project_id: &project,
                root_id: &root,
                stream_id: &stream_id,
                seed: SEED,
            })
            .unwrap()
            .worktree_id,
            branch: branch.clone(),
            lineage: symbiote_domain::StreamLineage::Independent,
            base: stale_base,
            target: CommitSha::new(repo.base_sha.clone()).unwrap(),
        })
        .unwrap();
        let reservation_base =
            std::env::temp_dir().join(format!("symbiote-prov-moved-base-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&reservation_base);
        std::fs::create_dir_all(&reservation_base).unwrap();
        let mut git = SystemGit::new();
        let result = provision(
            &mut git,
            ProvisionInputs {
                stream: &stream,
                root_id: &root,
                project_id: &project,
                stream_id: &stream_id,
                policy_seed: SEED,
                reservation_base: &reservation_base,
                source_repository: &repo.dir,
            },
        );
        assert_eq!(result, Err(ProvisionError::BaseMoved));
        // The refusal happened before materialization: the reserved
        // location exists (the reservation claimed it) but holds no git
        // content — the work premise was never acted on.
        let derived_path = ProvisionInputs {
            stream: &stream,
            root_id: &root,
            project_id: &project,
            stream_id: &stream_id,
            policy_seed: SEED,
            reservation_base: &reservation_base,
            source_repository: &repo.dir,
        }
        .derived_worktree_path(&reservation_base);
        assert!(
            std::fs::read_dir(&derived_path)
                .map(|mut entries| entries.next().is_none())
                .unwrap_or(true),
            "refused provisioning must not leave materialized content"
        );
        // The operator's answer to a moved base is to put it back, and the
        // second attempt has to be able to take the location the first one
        // claimed. Before the reuse branch this refused `AlreadyReserved` —
        // a location this stream holds, holding nothing, refusing the stream
        // that holds it, because the first attempt had already been here.
        let restore = Command::new("git")
            .arg("-C")
            .arg(&repo.dir)
            .args(["reset", "--hard", "HEAD~1"])
            .output()
            .unwrap();
        assert!(restore.status.success());
        let retried = provision(
            &mut git,
            ProvisionInputs {
                stream: &stream,
                root_id: &root,
                project_id: &project,
                stream_id: &stream_id,
                policy_seed: SEED,
                reservation_base: &reservation_base,
                source_repository: &repo.dir,
            },
        );
        assert_eq!(
            retried
                .as_ref()
                .map(|provisioned| provisioned.branch.clone()),
            Ok(branch),
            "a claim whose materialization never happened is a claim to materialize: {retried:?}"
        );
        assert!(
            retried.unwrap().worktree.join("base.txt").exists(),
            "and the retry hands the run a real checkout"
        );
        let _ = std::fs::remove_dir_all(&reservation_base);
    }

    #[test]
    fn tampered_identity_refuses_before_any_git_call() {
        // A seed mismatch must refuse at the identity check — BEFORE any
        // git invocation. Pinned with a scripted executor that records
        // every call: a reordering that ran git first would fail here.
        let repo = SourceRepo::new("tamper-scripted");
        let project = ProjectId::new("prov-tamper-s").unwrap();
        let root = RootId::new("prov-tamper-s-root").unwrap();
        let stream_id = ChangeStreamId::new("prov-tamper-s-stream").unwrap();
        let branch = derived_branch(&project, &root, &stream_id);
        let base = CommitSha::new(repo.base_sha.clone()).unwrap();
        let stream = ChangeStream::new(symbiote_domain::NewChangeStream {
            id: stream_id.clone(),
            project_id: project.clone(),
            root_id: root.clone(),
            tasks: std::collections::BTreeSet::from([symbiote_domain::TaskId::new(
                "prov-tamper-s-task",
            )
            .unwrap()]),
            originating_chat: symbiote_domain::ChatId::new("chat-tamper-s").unwrap(),
            worktree: symbiote_worktrees::Derived::derive(symbiote_worktrees::DeriveInputs {
                project_id: &project,
                root_id: &root,
                stream_id: &stream_id,
                seed: "different-seed",
            })
            .unwrap()
            .worktree_id,
            branch: branch.clone(),
            lineage: symbiote_domain::StreamLineage::Independent,
            base,
            target: CommitSha::new(repo.base_sha.clone()).unwrap(),
        })
        .unwrap();
        let reservation_base =
            std::env::temp_dir().join(format!("symbiote-prov-ts-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&reservation_base);
        std::fs::create_dir_all(&reservation_base).unwrap();
        let mut git = crate::tests::RecordingExecutor::default();
        assert_eq!(
            provision(
                &mut git,
                ProvisionInputs {
                    stream: &stream,
                    root_id: &root,
                    project_id: &project,
                    stream_id: &stream_id,
                    policy_seed: SEED,
                    reservation_base: &reservation_base,
                    source_repository: &repo.dir,
                },
            ),
            Err(ProvisionError::Reservation)
        );
        assert!(
            git.calls.is_empty(),
            "tamper refusal must precede any git call, got {:?}",
            git.calls
        );
        let _ = std::fs::remove_dir_all(&reservation_base);
    }

    #[test]
    fn provision_refuses_a_tampered_reservation_identity() {
        let repo = SourceRepo::new("tamper");
        let project = ProjectId::new("prov-tamper").unwrap();
        let root = RootId::new("prov-tamper-root").unwrap();
        let stream_id = ChangeStreamId::new("prov-tamper-stream").unwrap();
        let branch = derived_branch(&project, &root, &stream_id);
        let base = CommitSha::new(repo.base_sha.clone()).unwrap();
        let stream = ChangeStream::new(symbiote_domain::NewChangeStream {
            id: stream_id.clone(),
            project_id: project.clone(),
            root_id: root.clone(),
            tasks: std::collections::BTreeSet::from([symbiote_domain::TaskId::new(
                "prov-tamper-task",
            )
            .unwrap()]),
            originating_chat: symbiote_domain::ChatId::new("chat-tamper").unwrap(),
            worktree: symbiote_worktrees::Derived::derive(symbiote_worktrees::DeriveInputs {
                project_id: &project,
                root_id: &root,
                stream_id: &stream_id,
                seed: "different-seed",
            })
            .unwrap()
            .worktree_id,
            branch: branch.clone(),
            lineage: symbiote_domain::StreamLineage::Independent,
            base,
            target: CommitSha::new(repo.base_sha.clone()).unwrap(),
        })
        .unwrap();
        let reservation_base =
            std::env::temp_dir().join(format!("symbiote-prov-tamper-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&reservation_base);
        std::fs::create_dir_all(&reservation_base).unwrap();
        let mut git = SystemGit::new();
        // The stream's worktree_id was derived with a DIFFERENT seed than
        // the policy seed: the derivation will not reproduce it — the
        // identity itself is the tamper detection.
        assert_eq!(
            provision(
                &mut git,
                ProvisionInputs {
                    stream: &stream,
                    root_id: &root,
                    project_id: &project,
                    stream_id: &stream_id,
                    policy_seed: SEED,
                    reservation_base: &reservation_base,
                    source_repository: &repo.dir,
                },
            ),
            Err(ProvisionError::Reservation)
        );
        let _ = std::fs::remove_dir_all(&reservation_base);
    }

    /// Everything the reuse cases share: a real repository, a stream whose
    /// recorded identities are the derived ones, and a private base.
    fn reuse_fixture(
        tag: &str,
    ) -> (
        SourceRepo,
        ChangeStream,
        ProjectId,
        RootId,
        ChangeStreamId,
        PathBuf,
    ) {
        let repo = SourceRepo::new(tag);
        let project = ProjectId::new(format!("prov-{tag}-project")).unwrap();
        let root = RootId::new(format!("prov-{tag}-root")).unwrap();
        let stream_id = ChangeStreamId::new(format!("prov-{tag}-stream")).unwrap();
        let branch = derived_branch(&project, &root, &stream_id);
        let worktree = symbiote_worktrees::Derived::derive(symbiote_worktrees::DeriveInputs {
            project_id: &project,
            root_id: &root,
            stream_id: &stream_id,
            seed: SEED,
        })
        .unwrap()
        .worktree_id;
        let base_sha = CommitSha::new(repo.base_sha.clone()).unwrap();
        let stream = ChangeStream::new(symbiote_domain::NewChangeStream {
            id: stream_id.clone(),
            project_id: project.clone(),
            root_id: root.clone(),
            tasks: std::collections::BTreeSet::from([symbiote_domain::TaskId::new(format!(
                "prov-{tag}-task"
            ))
            .unwrap()]),
            originating_chat: symbiote_domain::ChatId::new(format!("chat-{tag}")).unwrap(),
            worktree,
            branch,
            lineage: symbiote_domain::StreamLineage::Independent,
            base: base_sha.clone(),
            target: base_sha,
        })
        .unwrap();
        let reservation_base =
            std::env::temp_dir().join(format!("symbiote-prov-{tag}-base-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&reservation_base);
        std::fs::create_dir_all(&reservation_base).unwrap();
        (repo, stream, project, root, stream_id, reservation_base)
    }

    /// The inputs, as a function: a closure cannot hand back a struct that
    /// borrows from what it closed over, and every case here calls this twice
    /// or more.
    fn inputs<'a>(
        stream: &'a ChangeStream,
        root: &'a RootId,
        project: &'a ProjectId,
        stream_id: &'a ChangeStreamId,
        reservation_base: &'a Path,
        source_repository: &'a Path,
    ) -> ProvisionInputs<'a> {
        ProvisionInputs {
            stream,
            root_id: root,
            project_id: project,
            stream_id,
            policy_seed: SEED,
            reservation_base,
            source_repository,
        }
    }

    /// A Task has one Change Stream for its whole life, and `recover_task` is
    /// the only way out of a failed or interrupted run — so provisioning has to
    /// be able to answer a second time for the same stream. This is that
    /// answer, checked against the state a crashed run leaves: a checkout with
    /// uncommitted output in it.
    #[test]
    fn a_stream_that_already_holds_its_location_is_provisioned_again() {
        let (repo, stream, project, root, stream_id, base) = reuse_fixture("reuse");
        let mut git = SystemGit::new();
        let first = provision(
            &mut git,
            inputs(&stream, &root, &project, &stream_id, &base, &repo.dir),
        )
        .unwrap();
        let location = first.worktree.clone();
        std::fs::write(
            location.join("half-written.txt"),
            "the first run's last output\n",
        )
        .unwrap();

        let second = provision(
            &mut git,
            inputs(&stream, &root, &project, &stream_id, &base, &repo.dir),
        )
        .unwrap();
        assert_eq!(
            second.worktree, location,
            "the same derived location: a task that can only be run once has no \
             recovery, and `recover_task` is a recovery"
        );
        assert_eq!(second.branch, first.branch);
        assert_eq!(second.head.commit, first.head.commit);
        assert!(
            location.join("half-written.txt").exists(),
            "the earlier run's uncommitted output is still there, because it is \
             the same stream's work in the same stream's directory — a run that \
             inherited it is a run continuing it, and `abandon_change_stream` is \
             the operation that discards it on purpose"
        );

        // And after a commit on the stream's branch, the retry starts from the
        // branch tip rather than the recorded base: that is where the work is,
        // and reporting the base instead would be a lie the evidence trail
        // would carry.
        let commit = Command::new("git")
            .arg("-C")
            .arg(&location)
            .args(["add", "-A"])
            .output()
            .unwrap();
        assert!(commit.status.success());
        let commit = Command::new("git")
            .arg("-C")
            .arg(&location)
            .args([
                "-c",
                "user.email=t@symbiote.test",
                "-c",
                "user.name=t",
                "commit",
                "-qm",
                "the first run's work",
            ])
            .output()
            .unwrap();
        assert!(commit.status.success());
        let third = provision(
            &mut git,
            inputs(&stream, &root, &project, &stream_id, &base, &repo.dir),
        )
        .unwrap();
        assert_ne!(
            third.head.commit, first.head.commit,
            "the observed head is the branch tip, which moved"
        );
        assert!(matches!(
            third.head.state,
            crate::HeadState::Branch { ref name } if *name == third.branch
        ));
        let _ = std::fs::remove_dir_all(&base);
    }

    /// The other side of the same proof, and the reason it is a proof: a
    /// location this stream holds, holding content, that git does not know
    /// about. Nothing here shows the content is the stream's, so nothing here
    /// runs in it — and the refusal removes nothing either.
    #[test]
    fn content_in_a_held_location_that_git_never_saw_is_refused() {
        let (repo, stream, project, root, stream_id, base) = reuse_fixture("stranger");
        let derived = symbiote_worktrees::Derived::derive(symbiote_worktrees::DeriveInputs {
            project_id: &project,
            root_id: &root,
            stream_id: &stream_id,
            seed: SEED,
        })
        .unwrap();
        symbiote_worktrees::reserve(
            &derived,
            &base,
            symbiote_domain::Timestamp(1_700_000_000_000),
        )
        .unwrap();
        let location = derived.worktree_path(&base);
        std::fs::write(location.join("stranger.txt"), "not this stream's work\n").unwrap();
        let mut git = SystemGit::new();
        assert_eq!(
            provision(
                &mut git,
                ProvisionInputs {
                    stream: &stream,
                    root_id: &root,
                    project_id: &project,
                    stream_id: &stream_id,
                    policy_seed: SEED,
                    reservation_base: &base,
                    source_repository: &repo.dir,
                },
            ),
            Err(ProvisionError::Reservation)
        );
        assert!(
            location.join("stranger.txt").exists(),
            "and the refusal is not a removal: nothing here is this Host's to \
             delete"
        );
        let _ = std::fs::remove_dir_all(&base);
    }

    /// A checkout in the right place on the wrong ref. The marker and the path
    /// both match, so the first two proofs pass; the third is what a
    /// mis-provisioned or hand-edited directory looks like, and it is refused.
    #[test]
    fn a_held_location_on_another_branch_is_refused() {
        let (repo, stream, project, root, stream_id, base) = reuse_fixture("wrong-ref");
        let mut git = SystemGit::new();
        let first = provision(
            &mut git,
            inputs(&stream, &root, &project, &stream_id, &base, &repo.dir),
        )
        .unwrap();
        let elsewhere = Command::new("git")
            .arg("-C")
            .arg(&first.worktree)
            .args(["checkout", "-q", "-b", "not-this-streams-branch"])
            .output()
            .unwrap();
        assert!(elsewhere.status.success());
        assert_eq!(
            provision(
                &mut git,
                inputs(&stream, &root, &project, &stream_id, &base, &repo.dir)
            ),
            Err(ProvisionError::Reservation),
            "the right path on the wrong ref is not this stream's work"
        );
        let _ = std::fs::remove_dir_all(&base);
    }
}
