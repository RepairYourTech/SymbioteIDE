//! Dispatch-time worktree provisioning (#211): the composition point where
//! a Change Stream's reserved location is verified, its source repository's
//! base commit is validated against the stream's recorded base, and the
//! stream's worktree is materialized for a started dispatch.
//!
//! Ordering is the security contract:
//! 1. **Verify the reservation** — the marker must match the derived
//!    identity exactly; a foreign or tampered location fails loudly.
//! 2. **Validate the base** — the source repository's actual HEAD must
//!    resolve and equal the stream's recorded base commit. A moved base
//!    means the stream's recorded premise is stale: refusal, never a
//!    materialization against unknown ground.
//! 3. **Materialize** — `git worktree add` onto the verified-empty
//!    location at the stream's derived branch, then observe the result.
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

/// Runs the three-stage provisioning for one dispatch. Idempotent by
/// construction: `worktrees::reserve` claims exactly the derived location,
/// `verify` accepts an existing correct marker, and re-materializing over a
/// materialized worktree fails in git (non-empty), so a double provisioning
/// attempt surfaces instead of silently reusing unknown state.
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
    let reservation =
        symbiote_worktrees::reserve(&derived, inputs.reservation_base, wall_clock_now())
            .map_err(|_| ProvisionError::Reservation)?;
    symbiote_worktrees::verify(&reservation).map_err(|_| ProvisionError::Reservation)?;
    // Base validation against the source repository's actual HEAD.
    let head = crate::observe_head(git, inputs.source_repository).map_err(|error| match error {
        crate::GitError::GitRefused => ProvisionError::BaseUnresolved,
        other => ProvisionError::Git(other),
    })?;
    if !head
        .commit
        .eq_ignore_ascii_case(inputs.stream.base().as_str())
    {
        return Err(ProvisionError::BaseMoved);
    }
    if head.commit.is_empty() {
        return Err(ProvisionError::BaseUnresolved);
    }
    let observation = crate::materialize(
        git,
        crate::MaterializeRequest {
            repository: inputs.source_repository,
            worktree: &inputs.derived_worktree_path(inputs.reservation_base),
            branch: &derived.branch,
            start_point: inputs.stream.base().as_str(),
        },
    )
    .map_err(ProvisionError::Git)?;
    Ok(Provisioned {
        worktree: inputs.derived_worktree_path(inputs.reservation_base),
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
        // Re-provisioning: the second attempt fails honestly. The first
        // attempt's worktree is non-empty, so the reservation layer refuses
        // to reclaim it — double provisioning never reuses unknown state.
        let second = provision(
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
        assert_eq!(second, Err(ProvisionError::Reservation));
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
}
