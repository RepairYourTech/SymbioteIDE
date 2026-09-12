use std::collections::{BTreeMap, BTreeSet};
use symbiote_domain::*;

macro_rules! id {
    ($kind:ident, $value:expr) => {
        $kind::new($value).unwrap()
    };
}
/// A dispatch records the limits it was staffed with, and a limit the contract
/// does not allow never becomes one: the recorded limits are what a Host must
/// bind the execution to, so an unallowable declaration refuses compilation as
/// well as replay.
#[test]
fn a_dispatch_records_the_limits_it_was_staffed_with_or_refuses() {
    let f = Fixture::new(RuntimeKind::NativeSymbiote);
    let mut declared = fixture_limits();
    declared.max_memory_bytes = 8 << 20;
    let dispatch = f.compile_with(&declared).unwrap();
    assert_eq!(dispatch.contract().limits(), &declared);
    for broken in [
        ResourceLimits {
            max_memory_bytes: 0,
            ..fixture_limits()
        },
        ResourceLimits {
            max_cpu_millicores: Some(0),
            ..fixture_limits()
        },
        ResourceLimits {
            max_cpu_millicores: Some(MAX_CPU_MILLICORES + 1),
            ..fixture_limits()
        },
    ] {
        assert_eq!(f.compile_with(&broken), Err(DomainError::ResourceLimit));
    }
    // The same invariant holds on replay: a journaled contract carrying a
    // limit the contract does not allow is refused, never rehydrated into a
    // dispatch whose execution would be bound by something impossible.
    let mut wire = serde_json::to_value(dispatch).unwrap();
    wire["contract"]["limits"]["max_memory_bytes"] = serde_json::json!(u64::MAX);
    assert!(serde_json::from_value::<Dispatch>(wire).is_err());
}

/// The limits a fixture dispatch records: what its execution may consume.
fn fixture_limits() -> ResourceLimits {
    ResourceLimits {
        max_total_tokens: 100_000,
        max_wall_time_ms: 60_000,
        max_concurrency: 1,
        max_memory_bytes: 1 << 30,
        max_cpu_millicores: None,
    }
}

fn sha(c: char) -> CommitSha {
    CommitSha::new(c.to_string().repeat(40)).unwrap()
}

struct Fixture {
    task: Task,
    role: Role,
    binding: WorkforceBinding,
    profile: RuntimeProfile,
    host: Host,
    stream: ChangeStream,
}
impl Fixture {
    fn new(runtime: RuntimeKind) -> Self {
        let project_id = id!(ProjectId, "project");
        let role_id = id!(RoleId, "engineer");
        let root_id = id!(RootId, "root");
        let host_id = id!(HostId, "host");
        let task_id = id!(TaskId, "task");
        let stream_id = id!(ChangeStreamId, "stream");
        let task = Task::new(
            task_id.clone(),
            project_id.clone(),
            root_id.clone(),
            role_id.clone(),
            stream_id.clone(),
            VersionedTaskContract {
                id: id!(TaskContractId, "task-contract"),
                revision: Revision(1),
            },
        );
        let role = Role {
            id: role_id.clone(),
            project_id: project_id.clone(),
            revision: Revision(1),
            name: "Engineer".into(),
            operating_contract: VersionedRoleContract {
                id: id!(RoleContractId, "role-contract"),
                revision: Revision(1),
            },
        };
        let profile = RuntimeProfile {
            id: id!(RuntimeProfileId, "profile"),
            revision: Revision(1),
            runtime,
            adapter: id!(AgentRuntimeAdapterId, "runtime-adapter"),
            installation: if runtime == RuntimeKind::ExternalHarness {
                Some(id!(InstallationId, "codex-installation"))
            } else {
                None
            },
            provider: id!(ProviderConnectionId, "provider"),
            credential: id!(CredentialReferenceId, "credential"),
            billing_entitlement: id!(BillingEntitlementId, "billing"),
            model: id!(ModelId, "model"),
            eligible_hosts: BTreeSet::from([host_id.clone()]),
        };
        let binding = WorkforceBinding {
            id: id!(BindingId, "binding"),
            revision: Revision(1),
            project_id: project_id.clone(),
            role_id,
            profile_id: profile.id.clone(),
            profile_revision: profile.revision,
            protocol: VersionedProtocol {
                id: id!(ProtocolId, "workforce-protocol"),
                revision: Revision(1),
            },
            access: AccessSnapshot {
                project_id: project_id.clone(),
                roots: BTreeSet::from([root_id.clone()]),
                grants: BTreeSet::from([Permission::ReadRoot, Permission::MutateStream]),
                policy_revision: Revision(1),
            },
            required_controls: BTreeSet::new(),
            context: ContextPolicy {
                bundle: id!(ContextBundleId, "context"),
                revision: Revision(1),
                max_input_tokens: 4096,
                reserved_output_tokens: 1024,
            },
            required_tools: BTreeSet::new(),
            required_skills: BTreeSet::new(),
            escalation: EscalationPolicy::StopAndRequestHuman,
        };
        let host = Host {
            id: host_id,
            revision: Revision(1),
            device: id!(DeviceId, "device"),
            fabric: None,
            supported_runtimes: vec![RuntimeKind::NativeSymbiote, RuntimeKind::ExternalHarness],
            controls: [
                Control::Filesystem,
                Control::Cancellation,
                Control::CompletionAuthority,
            ]
            .into_iter()
            .map(|c| {
                (
                    c,
                    EnforcementClaim {
                        strength: EnforcementStrength::HostEnforced,
                        evidence: id!(EvidenceId, "proof"),
                        verified_at: Timestamp(1),
                        expires_at: Timestamp(1000),
                    },
                )
            })
            .collect::<BTreeMap<_, _>>(),
        };
        let stream = ChangeStream::new(NewChangeStream {
            id: stream_id,
            project_id,
            root_id,
            tasks: BTreeSet::from([task_id]),
            originating_chat: id!(ChatId, "chat"),
            worktree: id!(WorktreeId, "worktree"),
            branch: "issue-36".into(),
            lineage: StreamLineage::Independent,
            base: sha('a'),
            target: sha('b'),
        })
        .unwrap();
        Self {
            task,
            role,
            binding,
            profile,
            host,
            stream,
        }
    }
    fn compile(&self) -> Result<Dispatch, DomainError> {
        self.compile_with(&fixture_limits())
    }
    /// The same compilation with declared limits of the caller's choosing, so a
    /// test can put a limit the contract does not allow in front of it.
    fn compile_with(&self, limits: &ResourceLimits) -> Result<Dispatch, DomainError> {
        Dispatch::compile(
            id!(DispatchId, format!("dispatch-{}", self.task.revision().0)),
            id!(RuntimeContractId, "runtime-contract"),
            DispatchInputs {
                task: &self.task,
                role: &self.role,
                binding: &self.binding,
                profile: &self.profile,
                host: &self.host,
                limits,
                minimum_enforcement: &std::collections::BTreeMap::new(),
                now: Timestamp(10),
            },
        )
    }
    fn host_command(&self, name: &str, action: TaskAction) -> TaskCommand {
        TaskCommand {
            id: id!(CommandId, name),
            expected_revision: self.task.revision(),
            actor: Actor::Host(self.host.id.clone()),
            at: Timestamp(20),
            action,
        }
    }
    fn start(&mut self) -> Dispatch {
        let dispatch = self.compile().unwrap();
        self.task
            .apply(self.host_command(
                "start",
                TaskAction::Start {
                    dispatch: Box::new(dispatch.clone()),
                },
            ))
            .unwrap();
        dispatch
    }
    fn verifying(&mut self) -> Dispatch {
        let dispatch = self.start();
        self.task
            .apply(TaskCommand {
                id: id!(CommandId, "worker-done"),
                expected_revision: self.task.revision(),
                actor: Actor::Worker(dispatch.id().clone()),
                at: Timestamp(20),
                action: TaskAction::RequestCompletion {
                    dispatch_id: dispatch.id().clone(),
                    report: "change ready for verification".into(),
                },
            })
            .unwrap();
        assert_eq!(self.task.state(), &TaskState::CompletionRequested);
        self.task
            .apply(self.host_command("verify", TaskAction::BeginVerification))
            .unwrap();
        self.stream
            .apply(
                self.stream_command(
                    "validate",
                    StreamAction::Validate {
                        head: self.stream.head().clone(),
                        target: self.stream.target().clone(),
                        run: id!(VerificationRunId, "integration-run"),
                    },
                ),
                &self.binding.access,
            )
            .unwrap();
        dispatch
    }
    fn evidence(&self, dispatch: &Dispatch) -> Vec<VerificationEvidence> {
        Gate::required()
            .into_iter()
            .enumerate()
            .map(|(i, gate)| VerificationEvidence {
                id: id!(EvidenceId, format!("evidence-{i}")),
                run_id: id!(VerificationRunId, format!("run-{i}")),
                artifact_id: id!(ArtifactId, format!("artifact-{i}")),
                project_id: self.task.project_id().clone(),
                task_id: self.task.id().clone(),
                dispatch_id: dispatch.id().clone(),
                stream_id: self.stream.id().clone(),
                gate,
                outcome: VerificationOutcome::Passed,
                head: self.stream.head().clone(),
                target: self.stream.target().clone(),
                verified_by: self.host.id.clone(),
                reviewer: if gate == Gate::IndependentReview {
                    Some(id!(RoleId, "independent-reviewer"))
                } else {
                    None
                },
                recorded_at: Timestamp(20),
            })
            .collect()
    }
    fn complete(&mut self, evidence: Vec<VerificationEvidence>) -> Result<Revision, DomainError> {
        self.task.apply(self.host_command(
            "complete",
            TaskAction::Complete {
                stream: Box::new(self.stream.clone()),
                evidence,
            },
        ))
    }
    fn stream_command(&self, id: &str, action: StreamAction) -> StreamCommand {
        StreamCommand {
            id: id!(CommandId, id),
            expected_revision: self.stream.revision(),
            host_id: self.host.id.clone(),
            at: Timestamp(20),
            action,
        }
    }
}

#[test]
fn identities_reject_invalid_wire_values_and_normalize_full_commit_ids() {
    for value in ["", "not an id", "host/path", "💻"] {
        assert!(ProjectId::new(value).is_err());
        assert!(serde_json::from_str::<ProjectId>(&serde_json::to_string(value).unwrap()).is_err());
    }
    assert!(RoleId::new("a".repeat(129)).is_err());
    assert!(CommitSha::new("abc123").is_err());
    assert_eq!(CommitSha::new("A".repeat(40)).unwrap(), sha('a'));
    assert!(CommitSha::new("a".repeat(64)).is_ok());
}

#[test]
fn native_and_external_paths_share_canonical_completion_contract() {
    for runtime in [RuntimeKind::NativeSymbiote, RuntimeKind::ExternalHarness] {
        let mut f = Fixture::new(runtime);
        let dispatch = f.verifying();
        assert_eq!(dispatch.contract().profile().runtime, runtime);
        assert_eq!(f.complete(f.evidence(&dispatch)), Ok(Revision(4)));
        assert_eq!(f.task.state(), &TaskState::Completed);
        assert_eq!(f.task.history().len(), 4);
    }
}

#[test]
fn compilation_is_deterministic_and_role_survives_restaffing() {
    let mut f = Fixture::new(RuntimeKind::NativeSymbiote);
    let old = f.compile().unwrap();
    assert_eq!(old, f.compile().unwrap());
    f.profile.id = id!(RuntimeProfileId, "codex-profile");
    f.profile.runtime = RuntimeKind::ExternalHarness;
    f.profile.model = id!(ModelId, "different-model");
    f.profile.credential = id!(CredentialReferenceId, "different-auth");
    f.binding.profile_id = f.profile.id.clone();
    f.binding.revision = Revision(2);
    let new = f.compile().unwrap();
    assert_eq!(old.contract().role(), new.contract().role());
    assert_ne!(old.contract().profile(), new.contract().profile());
    assert_eq!(old.contract().binding().revision, Revision(1));
    assert_eq!(new.contract().binding().revision, Revision(2));
}

#[test]
fn observed_emulated_unsupported_and_expired_controls_cannot_authorize() {
    for strength in [
        EnforcementStrength::ExternallyObserved,
        EnforcementStrength::Emulated,
        EnforcementStrength::Unsupported,
    ] {
        let mut f = Fixture::new(RuntimeKind::NativeSymbiote);
        f.host
            .controls
            .get_mut(&Control::CompletionAuthority)
            .unwrap()
            .strength = strength;
        assert_eq!(f.compile(), Err(DomainError::UnsupportedControl));
    }
    let mut f = Fixture::new(RuntimeKind::ExternalHarness);
    f.host
        .controls
        .get_mut(&Control::Filesystem)
        .unwrap()
        .expires_at = Timestamp(10);
    assert_eq!(f.compile(), Err(DomainError::UnsupportedControl));
    f.host.controls.remove(&Control::Cancellation);
    assert_eq!(f.compile(), Err(DomainError::UnsupportedControl));
}

#[test]
fn grants_require_corresponding_enforcement_and_root_scope() {
    let mut f = Fixture::new(RuntimeKind::NativeSymbiote);
    f.binding.access.grants.insert(Permission::Network);
    assert_eq!(f.compile(), Err(DomainError::UnsupportedControl));
    f.binding.access.grants.remove(&Permission::Network);
    f.binding.access.roots.clear();
    assert_eq!(f.compile(), Err(DomainError::PermissionDenied));
}

#[test]
fn dispatch_rejects_cross_project_stale_profile_and_ineligible_host() {
    let mut f = Fixture::new(RuntimeKind::NativeSymbiote);
    f.binding.project_id = id!(ProjectId, "other-project");
    assert_eq!(f.compile(), Err(DomainError::LineageMismatch));
    f.binding.project_id = f.role.project_id.clone();
    f.profile.revision = Revision(2);
    assert_eq!(f.compile(), Err(DomainError::LineageMismatch));
    f.binding.profile_revision = f.profile.revision;
    f.profile.eligible_hosts.clear();
    assert_eq!(f.compile(), Err(DomainError::IneligibleHost));
}

#[test]
fn worker_done_is_advisory_and_worker_cannot_begin_or_complete_verification() {
    let mut f = Fixture::new(RuntimeKind::NativeSymbiote);
    let dispatch = f.start();
    for action in [
        TaskAction::BeginVerification,
        TaskAction::Complete {
            stream: Box::new(f.stream.clone()),
            evidence: vec![],
        },
    ] {
        let mut command = f.host_command("forged-host", action);
        command.actor = Actor::Worker(dispatch.id().clone());
        assert_eq!(f.task.apply(command), Err(DomainError::PermissionDenied));
        assert_eq!(f.task.state(), &TaskState::Running);
    }
}

#[test]
fn wrong_worker_and_wrong_host_are_denied_without_mutation() {
    let mut f = Fixture::new(RuntimeKind::NativeSymbiote);
    let dispatch = f.start();
    let command = TaskCommand {
        id: id!(CommandId, "wrong-worker"),
        expected_revision: f.task.revision(),
        actor: Actor::Worker(id!(DispatchId, "other-dispatch")),
        at: Timestamp(20),
        action: TaskAction::RequestCompletion {
            dispatch_id: dispatch.id().clone(),
            report: "done".into(),
        },
    };
    assert_eq!(f.task.apply(command), Err(DomainError::PermissionDenied));
    let mut command = f.host_command(
        "wrong-host",
        TaskAction::Cancel {
            reason: "cancel".into(),
        },
    );
    command.actor = Actor::Host(id!(HostId, "other-host"));
    assert_eq!(f.task.apply(command), Err(DomainError::PermissionDenied));
    assert_eq!(f.task.revision(), Revision(1));
}

#[test]
fn completion_rejects_missing_failed_duplicate_stale_and_self_review_evidence() {
    let mut f = Fixture::new(RuntimeKind::NativeSymbiote);
    let dispatch = f.verifying();
    let baseline = f.evidence(&dispatch);
    let mut missing = baseline.clone();
    missing.pop();
    assert_eq!(f.complete(missing), Err(DomainError::EvidenceMissing));
    let mut failed = baseline.clone();
    failed[0].outcome = VerificationOutcome::Failed;
    assert_eq!(f.complete(failed), Err(DomainError::EvidenceRejected));
    let mut inconclusive = baseline.clone();
    inconclusive[0].outcome = VerificationOutcome::Inconclusive;
    assert_eq!(f.complete(inconclusive), Err(DomainError::EvidenceRejected));
    let mut duplicate = baseline.clone();
    duplicate.push(baseline[0].clone());
    assert_eq!(f.complete(duplicate), Err(DomainError::EvidenceRejected));
    let mut stale = baseline.clone();
    stale[0].head = sha('c');
    assert_eq!(f.complete(stale), Err(DomainError::StaleEvidence));
    let mut stale_target = baseline.clone();
    stale_target[0].target = sha('c');
    assert_eq!(f.complete(stale_target), Err(DomainError::StaleEvidence));
    let mut review = baseline.clone();
    review
        .iter_mut()
        .find(|e| e.gate == Gate::IndependentReview)
        .unwrap()
        .reviewer = Some(f.role.id.clone());
    assert_eq!(
        f.complete(review),
        Err(DomainError::IndependentReviewRequired)
    );
    assert_eq!(f.task.state(), &TaskState::Verifying);
    assert_eq!(f.task.revision(), Revision(3));
    assert!(f.complete(baseline).is_ok());
}

#[test]
fn evidence_cannot_cross_task_dispatch_project_or_time() {
    let mut f = Fixture::new(RuntimeKind::ExternalHarness);
    let dispatch = f.verifying();
    let baseline = f.evidence(&dispatch);
    let mut other = baseline.clone();
    other[0].task_id = id!(TaskId, "other");
    assert_eq!(f.complete(other), Err(DomainError::LineageMismatch));
    let mut other = baseline.clone();
    other[0].dispatch_id = id!(DispatchId, "previous");
    assert_eq!(f.complete(other), Err(DomainError::LineageMismatch));
    let mut other = baseline.clone();
    other[0].project_id = id!(ProjectId, "other");
    assert_eq!(f.complete(other), Err(DomainError::LineageMismatch));
    let mut future = baseline.clone();
    future[0].recorded_at = Timestamp(21);
    assert_eq!(f.complete(future), Err(DomainError::InvalidTimestamp));
    let mut old = baseline;
    old[0].recorded_at = Timestamp(19);
    assert_eq!(f.complete(old), Err(DomainError::InvalidTimestamp));
}

#[test]
fn optimistic_updates_preserve_success_history_and_retries_are_idempotent() {
    let mut f = Fixture::new(RuntimeKind::NativeSymbiote);
    let command = f.host_command(
        "start",
        TaskAction::Start {
            dispatch: Box::new(f.compile().unwrap()),
        },
    );
    assert_eq!(f.task.apply(command.clone()), Ok(Revision(1)));
    assert_eq!(f.task.apply(command.clone()), Ok(Revision(1)));
    let mut changed = command.clone();
    changed.at = Timestamp(21);
    assert_eq!(f.task.apply(changed), Err(DomainError::IdempotencyConflict));
    let mut raced = f.host_command(
        "raced",
        TaskAction::Cancel {
            reason: "cancel".into(),
        },
    );
    raced.expected_revision = Revision(0);
    assert_eq!(f.task.apply(raced), Err(DomainError::RevisionConflict));
    f.task
        .apply(f.host_command(
            "cancel",
            TaskAction::Cancel {
                reason: "cancel".into(),
            },
        ))
        .unwrap();
    assert_eq!(f.task.apply(command), Ok(Revision(1)));
    assert_eq!(f.task.history().len(), 2);
}

#[test]
fn interrupted_and_failed_tasks_recover_with_new_dispatch_cancelled_tasks_are_terminal() {
    for action in [
        TaskAction::Interrupt {
            reason: "process died".into(),
        },
        TaskAction::Fail {
            reason: "tool error".into(),
        },
    ] {
        let mut f = Fixture::new(RuntimeKind::NativeSymbiote);
        let original = f.start();
        f.task.apply(f.host_command("stop", action)).unwrap();
        let encoded = serde_json::to_string(&f.task).unwrap();
        f.task = serde_json::from_str(&encoded).unwrap();
        f.task
            .apply(f.host_command("recover", TaskAction::Recover))
            .unwrap();
        let replacement = f.compile().unwrap();
        assert_ne!(replacement.id(), original.id());
        assert_eq!(f.task.history().len(), 3);
        f.task
            .apply(f.host_command(
                "restart",
                TaskAction::Start {
                    dispatch: Box::new(replacement),
                },
            ))
            .unwrap();
        f.task
            .apply(f.host_command(
                "cancel",
                TaskAction::Cancel {
                    reason: "user cancelled".into(),
                },
            ))
            .unwrap();
        assert_eq!(
            f.task
                .apply(f.host_command("invalid-recovery", TaskAction::Recover)),
            Err(DomainError::IllegalTransition)
        );
    }
}

#[test]
fn rebasing_invalidates_checks_and_collisions_require_explicit_reconciliation() {
    let mut f = Fixture::new(RuntimeKind::NativeSymbiote);
    let dispatch = f.verifying();
    let evidence = f.evidence(&dispatch);
    let old_head = f.stream.head().clone();
    f.stream
        .apply(
            f.stream_command(
                "advance",
                StreamAction::Advance {
                    head: sha('c'),
                    target: sha('d'),
                },
            ),
            &f.binding.access,
        )
        .unwrap();
    assert_eq!(f.complete(evidence), Err(DomainError::StreamNotReady));
    assert_eq!(
        f.stream.apply(
            f.stream_command(
                "old-proof",
                StreamAction::Validate {
                    head: old_head,
                    target: sha('b'),
                    run: id!(VerificationRunId, "old-run"),
                }
            ),
            &f.binding.access
        ),
        Err(DomainError::StaleEvidence)
    );
    f.stream
        .apply(
            f.stream_command(
                "collision",
                StreamAction::Collision {
                    other: id!(ChangeStreamId, "other"),
                },
            ),
            &f.binding.access,
        )
        .unwrap();
    assert_eq!(
        f.stream.apply(
            f.stream_command(
                "unresolved",
                StreamAction::Validate {
                    head: sha('c'),
                    target: sha('d'),
                    run: id!(VerificationRunId, "run"),
                }
            ),
            &f.binding.access
        ),
        Err(DomainError::StreamNotReady)
    );
    f.stream
        .apply(
            f.stream_command(
                "reconcile",
                StreamAction::Reconcile {
                    head: sha('e'),
                    target: sha('d'),
                },
            ),
            &f.binding.access,
        )
        .unwrap();
    assert_eq!(f.stream.state(), &StreamState::NeedsRevalidation);
    f.stream
        .apply(
            f.stream_command(
                "revalidate",
                StreamAction::Validate {
                    head: sha('e'),
                    target: sha('d'),
                    run: id!(VerificationRunId, "new-run"),
                },
            ),
            &f.binding.access,
        )
        .unwrap();
    assert!(f.complete(f.evidence(&dispatch)).is_ok());
}

#[test]
fn stream_permissions_concurrency_and_exact_target_integration() {
    let mut f = Fixture::new(RuntimeKind::NativeSymbiote);
    let command = f.stream_command(
        "validate",
        StreamAction::Validate {
            head: f.stream.head().clone(),
            target: f.stream.target().clone(),
            run: id!(VerificationRunId, "run"),
        },
    );
    let mut denied = f.binding.access.clone();
    denied.grants.clear();
    assert_eq!(
        f.stream.apply(command.clone(), &denied),
        Err(DomainError::PermissionDenied)
    );
    assert_eq!(
        f.stream.apply(command.clone(), &f.binding.access),
        Ok(Revision(1))
    );
    assert_eq!(
        f.stream.apply(command.clone(), &f.binding.access),
        Ok(Revision(1))
    );
    let mut raced = command;
    raced.id = id!(CommandId, "raced");
    assert_eq!(
        f.stream.apply(raced, &f.binding.access),
        Err(DomainError::RevisionConflict)
    );
    assert_eq!(
        f.stream.apply(
            f.stream_command(
                "stale-target",
                StreamAction::Integrate {
                    head: sha('a'),
                    target: sha('c')
                }
            ),
            &f.binding.access
        ),
        Err(DomainError::StaleEvidence)
    );
    f.stream
        .apply(
            f.stream_command(
                "integrate",
                StreamAction::Integrate {
                    head: sha('a'),
                    target: sha('b'),
                },
            ),
            &f.binding.access,
        )
        .unwrap();
    assert_eq!(f.stream.state(), &StreamState::Integrated);
    assert_eq!(
        f.stream.apply(
            f.stream_command("after-integrate", StreamAction::Cancel),
            &f.binding.access
        ),
        Err(DomainError::IllegalTransition)
    );
}

#[test]
fn independent_and_stacked_streams_have_separate_worktree_and_chat_identity() {
    let f = Fixture::new(RuntimeKind::NativeSymbiote);
    let stacked = ChangeStream::new(NewChangeStream {
        id: id!(ChangeStreamId, "child-stream"),
        project_id: f.task.project_id().clone(),
        root_id: f.task.root_id().clone(),
        tasks: BTreeSet::from([id!(TaskId, "child-task")]),
        originating_chat: id!(ChatId, "child-chat"),
        worktree: id!(WorktreeId, "child-worktree"),
        branch: "child-branch".into(),
        lineage: StreamLineage::Stacked {
            parent: f.stream.id().clone(),
            parent_head: f.stream.head().clone(),
        },
        base: f.stream.head().clone(),
        target: f.stream.target().clone(),
    })
    .unwrap();
    assert_ne!(stacked.worktree, f.stream.worktree);
    assert_ne!(stacked.originating_chat, f.stream.originating_chat);
    assert!(matches!(f.stream.lineage(), StreamLineage::Independent));
    assert!(matches!(stacked.lineage(), StreamLineage::Stacked { .. }));
    let json = serde_json::to_string(&stacked).unwrap();
    assert_eq!(
        serde_json::from_str::<ChangeStream>(&json).unwrap(),
        stacked
    );
}

#[test]
fn typed_schemas_roundtrip_dispatch_and_reject_unknown_fields_and_versions() {
    let f = Fixture::new(RuntimeKind::NativeSymbiote);
    let dispatch = f.compile().unwrap();
    let value = serde_json::to_value(&dispatch).unwrap();
    assert_eq!(
        serde_json::from_value::<Dispatch>(value.clone()).unwrap(),
        dispatch
    );
    let mut invalid = value;
    invalid["unknown"] = serde_json::json!(true);
    assert!(serde_json::from_value::<Dispatch>(invalid).is_err());
    let mut envelope = DomainEnvelope {
        schema_version: SCHEMA_VERSION,
        record: DomainRecord::Task(f.task),
    };
    assert_eq!(envelope.validate_version(), Ok(()));
    envelope.schema_version += 1;
    assert_eq!(
        envelope.validate_version(),
        Err(DomainError::UnsupportedSchemaVersion)
    );
    let schema = serde_json::to_value(schemars::schema_for!(DomainEnvelope)).unwrap();
    for name in [
        "Task",
        "Role",
        "WorkforceBinding",
        "WorkforceRuntimeContract",
        "Dispatch",
        "Session",
        "RuntimeProfile",
        "ChangeStream",
    ] {
        assert!(
            schema["$defs"].get(name).is_some(),
            "missing schema: {name}"
        );
    }
}

#[test]
fn rehydration_rejects_forged_task_stream_and_dispatch_authority() {
    let mut f = Fixture::new(RuntimeKind::NativeSymbiote);
    let mut task = serde_json::to_value(&f.task).unwrap();
    task["state"] = serde_json::json!("completed");
    assert!(serde_json::from_value::<Task>(task).is_err());
    let mut stream = serde_json::to_value(&f.stream).unwrap();
    stream["state"] = serde_json::json!("validated");
    stream["last_validated_head"] = serde_json::json!(f.stream.head());
    stream["last_validated_target"] = serde_json::json!(f.stream.target());
    assert!(serde_json::from_value::<ChangeStream>(stream).is_err());
    let mut dispatch = serde_json::to_value(f.compile().unwrap()).unwrap();
    dispatch["contract"]["gates"] = serde_json::json!([]);
    assert!(serde_json::from_value::<Dispatch>(dispatch).is_err());
    let mut dispatch = serde_json::to_value(f.compile().unwrap()).unwrap();
    dispatch["contract"]["enforcement"] = serde_json::json!({});
    assert!(serde_json::from_value::<Dispatch>(dispatch).is_err());
    let dispatch = f.verifying();
    f.complete(f.evidence(&dispatch)).unwrap();
    assert_eq!(
        serde_json::from_value::<Task>(serde_json::to_value(&f.task).unwrap()).unwrap(),
        f.task
    );
    let mut altered_history = serde_json::to_value(&f.task).unwrap();
    altered_history["history"][0]["resulting_revision"] = serde_json::json!(99);
    assert!(serde_json::from_value::<Task>(altered_history).is_err());
}

#[test]
fn dispatch_pins_root_stream_task_contract_and_start_time() {
    let mut f = Fixture::new(RuntimeKind::NativeSymbiote);
    let dispatch = f.compile().unwrap();
    for (root, stream, contract) in [
        (
            id!(RootId, "other-root"),
            f.task.stream_id().clone(),
            f.task.task_contract().clone(),
        ),
        (
            f.task.root_id().clone(),
            id!(ChangeStreamId, "other-stream"),
            f.task.task_contract().clone(),
        ),
        (
            f.task.root_id().clone(),
            f.task.stream_id().clone(),
            VersionedTaskContract {
                id: id!(TaskContractId, "other-contract"),
                revision: Revision(2),
            },
        ),
    ] {
        let mut other = Task::new(
            f.task.id().clone(),
            f.task.project_id().clone(),
            root,
            f.task.role_id().clone(),
            stream,
            contract,
        );
        let command = f.host_command(
            "wrong-target",
            TaskAction::Start {
                dispatch: Box::new(dispatch.clone()),
            },
        );
        assert_eq!(other.apply(command), Err(DomainError::LineageMismatch));
    }
    let mut expired = f.host_command(
        "expired",
        TaskAction::Start {
            dispatch: Box::new(dispatch.clone()),
        },
    );
    expired.at = Timestamp(1000);
    assert_eq!(f.task.apply(expired), Err(DomainError::UnsupportedControl));
    let mut too_early = f.host_command(
        "before-compile",
        TaskAction::Start {
            dispatch: Box::new(dispatch),
        },
    );
    too_early.at = Timestamp(9);
    assert_eq!(f.task.apply(too_early), Err(DomainError::InvalidTimestamp));
    assert_eq!(f.task.revision(), Revision(0));
}

#[test]
fn effective_access_narrows_the_binding_scope_to_the_tasks_root() {
    let mut f = Fixture::new(RuntimeKind::NativeSymbiote);
    let second_root = id!(RootId, "other-root");
    f.binding.access.roots.insert(second_root);
    let dispatch = f.compile().unwrap();
    // The binding snapshot retains the Role's whole authorized scope.
    assert_eq!(dispatch.contract().binding().access.roots.len(), 2);
    // The dispatch operates on exactly this Task's Root: least privilege is
    // recomputed per Dispatch, not inherited wholesale from the binding.
    let effective = dispatch.contract().effective_access();
    assert_eq!(
        effective.roots,
        BTreeSet::from([id!(RootId, "root")]),
        "effective access must be narrowed to the task's root"
    );
    assert_eq!(
        effective.grants,
        dispatch.contract().binding().access.grants
    );
    assert_eq!(effective.project_id, f.binding.access.project_id);
    assert_eq!(effective.policy_revision, f.binding.access.policy_revision);
    // Serialize -> strip the field -> deserialize: contracts journaled
    // before narrowing must still replay, falling back to the binding
    // snapshot they were compiled with.
    let mut wire = serde_json::to_value(&dispatch).unwrap();
    assert!(wire["contract"]["effective_access"].is_object());
    wire["contract"]
        .as_object_mut()
        .unwrap()
        .remove("effective_access");
    let restored: Dispatch = serde_json::from_value(wire).unwrap();
    assert_eq!(restored.contract().effective_access().roots.len(), 2);
    assert_eq!(
        restored.contract().effective_access(),
        &restored.contract().binding().access
    );
}

#[test]
fn binding_enforcement_floor_refuses_weaker_host_claims_at_assignment() {
    let f = Fixture::new(RuntimeKind::NativeSymbiote);
    let compile_with = |minimum: EnforcementStrength| {
        Dispatch::compile(
            id!(DispatchId, "dispatch-floor"),
            id!(RuntimeContractId, "runtime-contract"),
            DispatchInputs {
                task: &f.task,
                role: &f.role,
                binding: &f.binding,
                profile: &f.profile,
                host: &f.host,
                limits: &fixture_limits(),
                minimum_enforcement: &BTreeMap::from([(Control::Filesystem, minimum)]),
                now: Timestamp(10),
            },
        )
        .map(|_| ())
    };
    // The host claims HostEnforced for Filesystem: a matching floor binds.
    assert_eq!(compile_with(EnforcementStrength::HostEnforced), Ok(()));
    // Demanding native enforcement from a host-enforced claim refuses the
    // assignment: the dispatch never launches under weaker isolation.
    assert_eq!(
        compile_with(EnforcementStrength::Native),
        Err(DomainError::UnsupportedControl)
    );
    // A floor on a control this dispatch does not require is vacuous:
    // Process is only required when ExecuteProcess is granted, which this
    // binding does not grant.
    let dispatch = Dispatch::compile(
        id!(DispatchId, "dispatch-vacuous"),
        id!(RuntimeContractId, "runtime-contract"),
        DispatchInputs {
            task: &f.task,
            role: &f.role,
            binding: &f.binding,
            profile: &f.profile,
            host: &f.host,
            limits: &fixture_limits(),
            minimum_enforcement: &BTreeMap::from([(Control::Network, EnforcementStrength::Native)]),
            now: Timestamp(10),
        },
    )
    .unwrap();
    assert!(
        !dispatch
            .contract()
            .enforcement()
            .contains_key(&Control::Network)
    );
}
