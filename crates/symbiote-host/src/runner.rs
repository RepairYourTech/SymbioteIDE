//! Host-side worker runner: the composition point where a started dispatch
//! is actually executed by one of the two worker loops (#465), and where the
//! loop's completion report is filed as canonical `RequestCompletion`
//! evidence. The runner owns no loop logic — native turns come from
//! `symbiote-native-agent`, external turns from `symbiote-external-agent`,
//! selected strictly by the dispatch contract's runtime kind. It owns the
//! wiring facts:
//!
//! - the dispatch is re-validated by the loop session at creation time (the
//!   loop refuses expired or foreign contracts), and the runner refuses a
//!   dispatch whose runtime kind does not match the loop it is about to run;
//! - the task must still be `Running` under the started dispatch before the
//!   run begins, and the completion report is applied as the `Worker` actor
//!   through the store's journaled `apply_task`, exactly like the protocol
//!   operation — so it can only move the task to `CompletionRequested`;
//!   Host verification and independent review remain the completion gates;
//! - a loop that halts without a completed turn files nothing: the task
//!   stays `Running` and the failure is returned for Host retry policy.
//!
//! Transports are injected. Tests use deterministic scripted transports; the
//! live native Responses-API client and the live external Codex turn each
//! require explicit user authorization for credentials and billing and are
//! not implemented here. No silent native-billing fallback for external work
//! exists and none may be added: the runtime kind decides the loop, and the
//! runner refuses mismatches.
use symbiote_domain::{
    Actor, CommandId, Dispatch, RuntimeKind, TaskAction, TaskCommand, TaskId, TaskState, Timestamp,
};
use symbiote_external_agent::{ExternalSession, StopKind};
use symbiote_native_agent::{LoopError, NativeSession};
use symbiote_runtime_sdk::events::RuntimeEventKind;
use symbiote_store::Store;

/// One wired worker run: the loop's own durable summary plus the journaled
/// completion evidence, if a completed turn produced a report.
#[derive(
    Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct WorkerOutcome {
    /// The dispatch the loop ran against.
    pub dispatch_id: symbiote_domain::DispatchId,
    pub task_id: TaskId,
    /// Completion evidence filed with the store, if the run completed a turn.
    /// The task can only be `CompletionRequested` after this; verification is
    /// a separate Host gate.
    pub completion_filed: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunnerError {
    /// The dispatch's runtime kind does not match the loop being run.
    RuntimeMismatch,
    /// The task is not in the Running state under this dispatch.
    NotRunning,
    /// The dispatch contract no longer validates at the run time.
    InvalidContract,
    /// The loop halted without a completed turn; nothing was filed.
    LoopFailed,
    /// The loop completed but produced no reportable final message.
    NoReport,
    /// The store refused the completion (illegal transition, CAS, etc.).
    Store,
}

impl std::fmt::Display for RunnerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "worker runner: {self:?}")
    }
}
impl std::error::Error for RunnerError {}

/// Preconditions every runner shares: the task is Running under this exact
/// dispatch and the loop's runtime kind matches the contract.
fn check_runnable(
    store: &Store,
    task_id: &TaskId,
    dispatch: &Dispatch,
    expected: RuntimeKind,
) -> Result<(), RunnerError> {
    if dispatch.contract().profile().runtime != expected {
        return Err(RunnerError::RuntimeMismatch);
    }
    let task = store.task(task_id).map_err(|_| RunnerError::Store)?;
    if task.state() != &TaskState::Running {
        return Err(RunnerError::NotRunning);
    }
    match task.current_dispatch() {
        Some(current) if current.id() == dispatch.id() => {}
        _ => return Err(RunnerError::NotRunning),
    }
    Ok(())
}

/// Files the loop's completion report through the store as Worker evidence.
fn file_completion(
    store: &mut Store,
    task_id: &TaskId,
    dispatch: &Dispatch,
    report: &str,
    at: Timestamp,
) -> Result<(), RunnerError> {
    let task = store.task(task_id).map_err(|_| RunnerError::Store)?;
    let command = TaskCommand {
        id: CommandId::new(format!("worker-completion-{}", task_id.as_str()))
            .map_err(|_| RunnerError::Store)?,
        expected_revision: task.revision(),
        actor: Actor::Worker(dispatch.id().clone()),
        at,
        action: TaskAction::RequestCompletion {
            dispatch_id: dispatch.id().clone(),
            report: report.to_owned(),
        },
    };
    store
        .apply_task(task_id, command)
        .map_err(|error: symbiote_store::StoreError| {
            // An illegal transition here means the state moved under us
            // (lease expiry, concurrent edit, replay). The loop's evidence
            // stays in the run summary; the Host decides the retry.
            let _ = error;
            RunnerError::Store
        })?;
    Ok(())
}

/// Runs the native loop (#465 native side) against an already-started
/// dispatch and files the completion evidence on a clean stop. The final
/// agent message is the report: it is what the model offered as its
/// summary, and it routes through the same Worker-evidence gate as the
/// protocol operation.
pub fn run_native(
    store: &mut Store,
    task_id: &TaskId,
    dispatch: &Dispatch,
    task_prompt: &str,
    transport: &mut impl symbiote_native_agent::InferenceTransport,
    at: Timestamp,
) -> Result<WorkerOutcome, RunnerError> {
    check_runnable(store, task_id, dispatch, RuntimeKind::NativeSymbiote)?;
    // The model envelope comes from the Host's provider registry (#464),
    // keyed by the contract's profile model — never from loop input.
    let model = store
        .model_descriptor(&dispatch.contract().profile().model)
        .map_err(|_| RunnerError::Store)?;
    let mut session = NativeSession::new(dispatch, model, at).map_err(native_error)?;
    let summary = session.run(task_prompt, transport).map_err(native_error)?;
    if summary.halted != Some(symbiote_native_agent::HaltReason::Stop) {
        return Err(RunnerError::LoopFailed);
    }
    // The final assistant message is the worker's report; it is recorded in
    // the event stream as the last Message before Exit.
    let report = session
        .events()
        .iter()
        .rev()
        .find_map(|event| match event.payload() {
            RuntimeEventKind::Message { text } => Some(text.as_str().to_owned()),
            _ => None,
        })
        .ok_or(RunnerError::NoReport)?;
    file_completion(store, task_id, dispatch, &report, at)?;
    Ok(WorkerOutcome {
        dispatch_id: dispatch.id().clone(),
        task_id: task_id.clone(),
        completion_filed: true,
    })
}

fn native_error(error: LoopError) -> RunnerError {
    match error {
        LoopError::InvalidContract => RunnerError::InvalidContract,
        _ => RunnerError::LoopFailed,
    }
}

/// Runs the external loop (#465 external side) against an already-started
/// dispatch and files the completion evidence on a completed turn. The
/// final harness agent message is the report. Any other stop kind — failed,
/// interrupted, transport lost — files nothing.
pub fn run_external(
    store: &mut Store,
    task_id: &TaskId,
    dispatch: &Dispatch,
    task_prompt: &str,
    worktree_cwd: &str,
    transport: &mut impl symbiote_external_agent::CodexTransport,
    at: Timestamp,
) -> Result<WorkerOutcome, RunnerError> {
    check_runnable(store, task_id, dispatch, RuntimeKind::ExternalHarness)?;
    let mut session = ExternalSession::new(dispatch, at).map_err(|error| match error {
        symbiote_external_agent::DriverError::InvalidContract => RunnerError::InvalidContract,
        _ => RunnerError::LoopFailed,
    })?;
    session
        .begin_thread(worktree_cwd, transport)
        .map_err(external_error)?;
    session
        .turn(task_prompt, transport)
        .map_err(external_error)?;
    if session.run_summary().stopped != Some(StopKind::Completed) {
        return Err(RunnerError::LoopFailed);
    }
    let report = session
        .events()
        .iter()
        .rev()
        .find_map(|event| match event.payload() {
            RuntimeEventKind::Message { text } => Some(text.as_str().to_owned()),
            _ => None,
        })
        .ok_or(RunnerError::NoReport)?;
    file_completion(store, task_id, dispatch, &report, at)?;
    Ok(WorkerOutcome {
        dispatch_id: dispatch.id().clone(),
        task_id: task_id.clone(),
        completion_filed: true,
    })
}

fn external_error(error: symbiote_external_agent::DriverError) -> RunnerError {
    match error {
        symbiote_external_agent::DriverError::InvalidContract => RunnerError::InvalidContract,
        _ => RunnerError::LoopFailed,
    }
}

/// Domain-law re-asserted for tests and callers: the runner's filing is
/// bounded by the same domain transition the protocol operation uses. Exposed
/// here so the contract doc's claim ("cannot pass CompletionRequested") is
/// checkable against the actual transition table, not folklore.
pub fn completion_requires_running(state: &TaskState) -> bool {
    matches!(state, TaskState::Running)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeSet, VecDeque};

    fn store_with_running_task(tag: &str, runtime: RuntimeKind) -> (Store, TaskId, Dispatch) {
        let mut store = Store::memory().unwrap();
        let (project, task) = fixture::full_fixture(&mut store, tag, runtime);
        let task_role = store.task(&task).unwrap().role_id().clone();
        let request = symbiote_workforce::RouteRequest {
            project_id: project.clone(),
            work_id: fixture::work_id(tag),
            requested: Some(task_role.clone()),
            domains: BTreeSet::new(),
        };
        let decision =
            symbiote_workforce::resolve_route(&store.get_team(&project).unwrap(), &request)
                .unwrap();
        store
            .record_route(
                fixture::command(tag, "route"),
                decision,
                fixture::user(),
                Timestamp(20),
            )
            .unwrap();
        store
            .prepare_dispatch(
                fixture::command(tag, "prepare"),
                task.clone(),
                fixture::user(),
                Timestamp(30),
            )
            .unwrap();
        let preparation = store.dispatch_preparation(&task).unwrap();
        assert_eq!(preparation.outcome, fixture::ready_outcome());
        let host = fixture::host(tag);
        let binding = fixture::binding(&store, &project, tag);
        let task_record = store.task(&task).unwrap();
        let role = fixture::role(&store, tag);
        let dispatch = fixture::dispatch(tag, &task_record, &role, &binding, &host);
        store
            .start_prepared_task(
                fixture::command(tag, "start"),
                task.clone(),
                dispatch.id().clone(),
                fixture::contract(tag, "start"),
                &host,
                fixture::user(),
                Timestamp(50),
            )
            .unwrap();
        (store, task, dispatch)
    }

    #[test]
    fn native_runner_files_completion_evidence() {
        let (mut store, task, dispatch) =
            store_with_running_task("native-run", RuntimeKind::NativeSymbiote);
        let mut transport = fixture::EchoTransport {
            text: "implemented the change".into(),
        };
        let outcome = run_native(
            &mut store,
            &task,
            &dispatch,
            "do the work",
            &mut transport,
            Timestamp(60),
        )
        .unwrap();
        assert!(outcome.completion_filed);
        assert_eq!(
            store.task(&task).unwrap().state(),
            &TaskState::CompletionRequested
        );
        // Evidence is not completion: no Verified transition happened.
        assert_ne!(store.task(&task).unwrap().state(), &TaskState::Completed);
    }

    #[test]
    fn external_runner_files_completion_evidence() {
        let (mut store, task, dispatch) =
            store_with_running_task("external-run", RuntimeKind::ExternalHarness);
        let mut transport = fixture::codex_transport();
        let outcome = run_external(
            &mut store,
            &task,
            &dispatch,
            "do the work",
            "/workspace",
            &mut transport,
            Timestamp(60),
        )
        .unwrap();
        assert!(outcome.completion_filed);
        assert_eq!(
            store.task(&task).unwrap().state(),
            &TaskState::CompletionRequested
        );
        assert_ne!(store.task(&task).unwrap().state(), &TaskState::Completed);
    }

    #[test]
    fn runners_refuse_foreign_runtime_kinds() {
        let (store, task, dispatch) =
            store_with_running_task("mismatch-native", RuntimeKind::NativeSymbiote);
        let mut store = store;
        let mut external_transport = fixture::codex_transport();
        assert_eq!(
            run_external(
                &mut store,
                &task,
                &dispatch,
                "do the work",
                "/workspace",
                &mut external_transport,
                Timestamp(60)
            ),
            Err(RunnerError::RuntimeMismatch)
        );
        // The task is still Running; nothing was filed.
        assert_eq!(store.task(&task).unwrap().state(), &TaskState::Running);
        let (store, task, dispatch) =
            store_with_running_task("mismatch-external", RuntimeKind::ExternalHarness);
        let mut store = store;
        let mut native_transport = fixture::native_transport(vec![]);
        assert_eq!(
            run_native(
                &mut store,
                &task,
                &dispatch,
                "do the work",
                &mut native_transport,
                Timestamp(60)
            ),
            Err(RunnerError::RuntimeMismatch)
        );
        assert_eq!(store.task(&task).unwrap().state(), &TaskState::Running);
    }

    #[test]
    fn failed_loop_files_nothing_and_task_stays_running() {
        let (mut store, task, dispatch) =
            store_with_running_task("native-fail", RuntimeKind::NativeSymbiote);
        let mut transport = fixture::native_transport(vec![Err(
            symbiote_runtime_sdk::provider::ProviderError::Unavailable,
        )]);
        assert_eq!(
            run_native(
                &mut store,
                &task,
                &dispatch,
                "do the work",
                &mut transport,
                Timestamp(60)
            ),
            Err(RunnerError::LoopFailed)
        );
        assert_eq!(store.task(&task).unwrap().state(), &TaskState::Running);
        // External: a failed turn files nothing either.
        let (mut store, task, dispatch) =
            store_with_running_task("external-fail", RuntimeKind::ExternalHarness);
        let mut transport = fixture::codex_transport_failed();
        assert_eq!(
            run_external(
                &mut store,
                &task,
                &dispatch,
                "do the work",
                "/workspace",
                &mut transport,
                Timestamp(60)
            ),
            Err(RunnerError::LoopFailed)
        );
        assert_eq!(store.task(&task).unwrap().state(), &TaskState::Running);
    }

    #[test]
    fn a_task_not_running_is_refused_before_any_loop() {
        let (mut store, task, dispatch) =
            store_with_running_task("not-running", RuntimeKind::NativeSymbiote);
        // File a completion directly (as the worker would) to leave the task
        // in CompletionRequested; a second run must be refused.
        let task_record = store.task(&task).unwrap();
        let command = TaskCommand {
            id: fixture::command("not-running", "manual-completion"),
            expected_revision: task_record.revision(),
            actor: Actor::Worker(dispatch.id().clone()),
            at: Timestamp(55),
            action: TaskAction::RequestCompletion {
                dispatch_id: dispatch.id().clone(),
                report: "first run".into(),
            },
        };
        store.apply_task(&task, command).unwrap();
        let mut transport = fixture::EchoTransport {
            text: "second".into(),
        };
        assert_eq!(
            run_native(
                &mut store,
                &task,
                &dispatch,
                "do the work",
                &mut transport,
                Timestamp(60)
            ),
            Err(RunnerError::NotRunning)
        );
        assert_eq!(
            store.task(&task).unwrap().state(),
            &TaskState::CompletionRequested
        );
    }

    /// Test fixtures for the store wiring. Mirrors the store's own
    /// preparation_tests fixture: a full project/team/binding/provider
    /// composition whose profile carries the requested runtime kind.
    mod fixture {
        use super::*;
        use std::collections::BTreeMap;
        use symbiote_domain::*;

        pub fn user() -> symbiote_domain::UserId {
            symbiote_domain::UserId::new("owner").unwrap()
        }
        pub fn command(tag: &str, name: &str) -> CommandId {
            CommandId::new(format!("{tag}-{name}")).unwrap()
        }
        pub fn contract(tag: &str, name: &str) -> RuntimeContractId {
            RuntimeContractId::new(format!("{tag}-{name}")).unwrap()
        }
        pub fn work_id(tag: &str) -> WorkId {
            WorkId::Objective(
                symbiote_domain::ObjectiveId::new(format!("objective-{tag}")).unwrap(),
            )
        }
        pub fn sha(c: char) -> symbiote_domain::CommitSha {
            let hex: String = std::iter::repeat_n(c, 40).collect();
            symbiote_domain::CommitSha::new(hex).unwrap()
        }
        pub fn ready_outcome() -> PreparationOutcome {
            PreparationOutcome::Ready
        }

        /// Builds the full store fixture with the requested runtime kind on
        /// the binding's profile. Mirrors the store's own preparation_tests
        /// fixture composition (project/team/binding/provider/task/origin).
        pub fn full_fixture(
            store: &mut Store,
            tag: &str,
            runtime: RuntimeKind,
        ) -> (ProjectId, TaskId) {
            let project_id = ProjectId::new(format!("project-{tag}")).unwrap();
            let root_id = RootId::new(format!("root-{tag}")).unwrap();
            let lead_id = RoleId::new(format!("lead-{tag}")).unwrap();
            let worker_id = RoleId::new(format!("worker-{tag}")).unwrap();
            let project = Project {
                id: project_id.clone(),
                revision: Revision(0),
                name: format!("Project {tag}"),
                owner: user(),
                roots: BTreeSet::from([root_id.clone()]),
                lead: lead_id.clone(),
                disposition: symbiote_domain::RecordDisposition::Active,
                provenance: Provenance {
                    created_at: Timestamp(10),
                    updated_at: Timestamp(10),
                    actor: Actor::User(user()),
                    external_references: vec![],
                },
            };
            let root = Root {
                id: root_id.clone(),
                project_id: project_id.clone(),
                revision: Revision(0),
                repository: None,
                host_paths: BTreeMap::new(),
            };
            let roles = vec![
                Role {
                    id: lead_id.clone(),
                    project_id: project_id.clone(),
                    revision: Revision(0),
                    name: "Lead".into(),
                    operating_contract: VersionedRoleContract {
                        id: RoleContractId::new(format!("lead-contract-{tag}")).unwrap(),
                        revision: Revision(1),
                    },
                },
                Role {
                    id: worker_id.clone(),
                    project_id: project_id.clone(),
                    revision: Revision(0),
                    name: "Engineer".into(),
                    operating_contract: VersionedRoleContract {
                        id: RoleContractId::new(format!("worker-contract-{tag}")).unwrap(),
                        revision: Revision(1),
                    },
                },
            ];
            store
                .register_project(
                    command(tag, "register"),
                    project.clone(),
                    vec![root],
                    roles.clone(),
                )
                .unwrap();
            let access = AccessSnapshot {
                project_id: project.id.clone(),
                roots: project.roots.clone(),
                grants: BTreeSet::from([Permission::ReadRoot, Permission::ExecuteProcess]),
                policy_revision: Revision(1),
            };
            let lead_policy = symbiote_domain::RolePolicy {
                role_id: lead_id.clone(),
                function: symbiote_domain::RoleFunction::LeadOrchestrator,
                responsibilities: vec!["Plan".into()],
                task_domains: BTreeSet::from(["coordination".into()]),
                access: access.clone(),
                context_policy_ref: "context".into(),
                tool_policy_ref: "tools".into(),
                skill_policy_ref: "skills".into(),
                execution_policy_ref: "execution".into(),
                independent_reviewers: BTreeSet::from([worker_id.clone()]),
                fallbacks: vec![],
            };
            let mut worker_access = access.clone();
            worker_access.grants = BTreeSet::from([
                Permission::ReadRoot,
                Permission::ExecuteProcess,
                Permission::MutateStream,
            ]);
            let worker_access_for_binding = worker_access.clone();
            let worker_policy = symbiote_domain::RolePolicy {
                role_id: worker_id.clone(),
                function: symbiote_domain::RoleFunction::GeneralExecution,
                responsibilities: vec!["Implement".into()],
                task_domains: BTreeSet::from(["coding".into()]),
                access: worker_access.clone(),
                context_policy_ref: "context".into(),
                tool_policy_ref: "tools".into(),
                skill_policy_ref: "skills".into(),
                execution_policy_ref: "execution".into(),
                independent_reviewers: BTreeSet::from([lead_id.clone()]),
                fallbacks: vec![],
            };
            let mut ceiling = access.clone();
            ceiling.grants.insert(Permission::MutateStream);
            let team = symbiote_domain::TeamConfiguration {
                schema_version: 1,
                project_id: project.id.clone(),
                revision: Revision(0),
                lead_role_id: lead_id.clone(),
                access_ceiling: ceiling,
                members: vec![lead_policy, worker_policy],
            };
            store
                .replace_team(
                    command(tag, "team"),
                    None,
                    team.clone(),
                    user(),
                    Timestamp(10),
                )
                .unwrap();
            let connection = ProviderConnection {
                id: ProviderConnectionId::new(format!("provider-{tag}")).unwrap(),
                adapter: InferenceProviderAdapterId::new("adapter").unwrap(),
                endpoint_reference: "https://api.openai.example/v1".into(),
                authentication: AuthenticationKind::ApiCredential,
            };
            store
                .replace_provider_connection(
                    command(tag, "provider"),
                    project.id.clone(),
                    connection,
                    user(),
                    Timestamp(11),
                )
                .unwrap();
            // The model descriptor the native loop reads from the registry
            // (only meaningful for native runs; harmless for external).
            store
                .replace_model_descriptor(
                    command(tag, "model"),
                    project.id.clone(),
                    symbiote_runtime_sdk::provider::ModelDescriptor {
                        schema_version: symbiote_runtime_sdk::provider::PROVIDER_CONTRACT_VERSION,
                        id: ModelId::new(format!("model-{tag}")).unwrap(),
                        provider_id: ProviderConnectionId::new(format!("provider-{tag}")).unwrap(),
                        context_window_tokens: 8192,
                        max_output_tokens: 4096,
                        capabilities: symbiote_runtime_sdk::provider::ModelCapabilities {
                            reasoning_efforts: BTreeSet::new(),
                            tools: true,
                            images: false,
                            streaming: false,
                        },
                    },
                    user(),
                    Timestamp(11),
                )
                .unwrap();
            let profile = RuntimeProfile {
                id: RuntimeProfileId::new(format!("profile-{tag}")).unwrap(),
                revision: Revision(0),
                runtime,
                adapter: AgentRuntimeAdapterId::new("adapter").unwrap(),
                // An external-harness profile must pin its installation.
                installation: if runtime == RuntimeKind::ExternalHarness {
                    Some(InstallationId::new("codex-0-118-0").unwrap())
                } else {
                    None
                },
                provider: ProviderConnectionId::new(format!("provider-{tag}")).unwrap(),
                credential: CredentialReferenceId::new("credential").unwrap(),
                billing_entitlement: BillingEntitlementId::new(format!("ent-{tag}")).unwrap(),
                model: ModelId::new(format!("model-{tag}")).unwrap(),
                eligible_hosts: BTreeSet::from([HostId::new(format!("host-{tag}")).unwrap()]),
            };
            let primary = symbiote_workforce::StaffingCandidate {
                profile: profile.clone(),
                config_identity: "fixture".into(),
                access: worker_access.clone(),
                tools: BTreeSet::new(),
                skills: BTreeSet::new(),
                secret_scopes: BTreeSet::new(),
                context: ContextPolicy {
                    bundle: ContextBundleId::new("context").unwrap(),
                    revision: Revision(1),
                    max_input_tokens: 4096,
                    reserved_output_tokens: 2048,
                },
                limits: symbiote_workforce::ResourceLimits {
                    max_total_tokens: 100_000,
                    max_wall_time_ms: 60_000,
                    max_concurrency: 1,
                    max_memory_bytes: 1 << 20,
                },
            };
            let policy = || symbiote_workforce::PolicyReference {
                id: "policy".into(),
                revision: Revision(0),
            };
            let configuration = symbiote_workforce::BindingConfiguration {
                schema_version: symbiote_workforce::BINDING_VERSION,
                binding: WorkforceBinding {
                    id: BindingId::new(format!("binding-{tag}")).unwrap(),
                    revision: Revision(0),
                    project_id: project.id.clone(),
                    role_id: worker_id.clone(),
                    profile_id: profile.id.clone(),
                    profile_revision: profile.revision,
                    protocol: VersionedProtocol {
                        id: ProtocolId::new("protocol").unwrap(),
                        revision: Revision(1),
                    },
                    access: worker_access_for_binding,
                    required_controls: BTreeSet::from([Control::Filesystem]),
                    context: primary.context.clone(),
                    required_tools: BTreeSet::new(),
                    required_skills: BTreeSet::new(),
                    escalation: EscalationPolicy::StopAndRequestHuman,
                },
                team_revision: team.revision,
                primary,
                fallbacks: vec![],
                policies: symbiote_workforce::WorkforcePolicies {
                    environment: policy(),
                    worktree: policy(),
                    verification: policy(),
                    documentation: policy(),
                    artifacts: policy(),
                    mcp: policy(),
                    escalation: policy(),
                    root_effort: symbiote_workforce::RootEffort::Medium,
                    local_children: symbiote_workforce::LocalChildPolicy::default(),
                    fallback_consent: FallbackConsent::ExplicitRequired,
                    minimum_enforcement: [
                        Control::Filesystem,
                        Control::Cancellation,
                        Control::CompletionAuthority,
                        Control::Process,
                    ]
                    .into_iter()
                    .map(|c| (c, EnforcementStrength::HostEnforced))
                    .collect(),
                    required_capabilities: BTreeSet::new(),
                },
            };
            store
                .replace_binding(
                    command(tag, "binding"),
                    None,
                    configuration,
                    user(),
                    Timestamp(12),
                )
                .unwrap();
            // Classified objective origin: legacy unclassified tasks cannot
            // start, so the fixture owns its task through an Objective.
            let work = WorkId::Objective(
                symbiote_domain::ObjectiveId::new(format!("objective-{tag}")).unwrap(),
            );
            let spec = WorkSpec {
                id: work.clone(),
                project_id: project.id.clone(),
                role_id: worker_id.clone(),
                title: format!("Objective {tag}"),
                description: format!("Owns task {tag}"),
                utterance: None,
                objective_class: Some(ObjectiveClass::Maintenance),
                parent: None,
                dependencies: BTreeSet::new(),
                requirements: vec![],
                constraints: vec![],
                risks: vec![],
                acceptance: vec![],
                priority: 1,
                budget: None,
                external_references: vec![],
            };
            let origin = TaskOrigin::Objective(spec.reference());
            let item = WorkItem::new(spec, user(), Timestamp(10)).unwrap();
            store.create_work_item(command(tag, "work"), item).unwrap();
            let task = Task::new(
                TaskId::new(format!("task-{tag}")).unwrap(),
                project.id.clone(),
                root_id.clone(),
                worker_id.clone(),
                ChangeStreamId::new(format!("stream-{tag}")).unwrap(),
                VersionedTaskContract {
                    id: TaskContractId::new(format!("contract-{tag}")).unwrap(),
                    revision: Revision(1),
                },
            );
            let stream = ChangeStream::new(NewChangeStream {
                id: ChangeStreamId::new(format!("stream-{tag}")).unwrap(),
                project_id: project.id.clone(),
                root_id: root_id.clone(),
                tasks: BTreeSet::from([task.id().clone()]),
                originating_chat: ChatId::new(format!("chat-{tag}")).unwrap(),
                worktree: WorktreeId::new(format!("worktree-{tag}")).unwrap(),
                branch: "fixture-branch".into(),
                lineage: StreamLineage::Independent,
                base: sha('a'),
                target: sha('b'),
            })
            .unwrap();
            store
                .create_task(command(tag, "task"), task.clone(), stream, origin)
                .unwrap();
            (project.id, task.id().clone())
        }

        pub fn host(tag: &str) -> Host {
            Host {
                id: HostId::new(format!("host-{tag}")).unwrap(),
                revision: Revision(0),
                device: DeviceId::new(format!("device-{tag}")).unwrap(),
                fabric: None,
                supported_runtimes: vec![RuntimeKind::NativeSymbiote, RuntimeKind::ExternalHarness],
                controls: [
                    Control::Filesystem,
                    Control::Cancellation,
                    Control::CompletionAuthority,
                    Control::Process,
                ]
                .into_iter()
                .map(|c| {
                    (
                        c,
                        EnforcementClaim {
                            strength: EnforcementStrength::HostEnforced,
                            evidence: EvidenceId::new("proof").unwrap(),
                            verified_at: Timestamp(1),
                            expires_at: Timestamp(1_000_000),
                        },
                    )
                })
                .collect(),
            }
        }

        pub fn binding(
            store: &Store,
            project: &ProjectId,
            tag: &str,
        ) -> symbiote_workforce::BindingConfiguration {
            // The fixture's binding id is deterministic; the store's public
            // accessor is the only bridge (its connection is private).
            let binding_id = BindingId::new(format!("binding-{tag}")).unwrap();
            store.get_binding(project, &binding_id).unwrap()
        }

        pub fn role(store: &Store, tag: &str) -> Role {
            let _ = store;
            Role {
                id: RoleId::new(format!("worker-{tag}")).unwrap(),
                project_id: ProjectId::new(format!("project-{tag}")).unwrap(),
                revision: Revision(0),
                name: "Engineer".into(),
                operating_contract: VersionedRoleContract {
                    id: RoleContractId::new(format!("worker-contract-{tag}")).unwrap(),
                    revision: Revision(1),
                },
            }
        }

        pub fn dispatch(
            tag: &str,
            task: &Task,
            role: &Role,
            binding: &symbiote_workforce::BindingConfiguration,
            host: &Host,
        ) -> Dispatch {
            Dispatch::compile(
                DispatchId::new(format!("dispatch-{tag}")).unwrap(),
                contract(tag, "dispatch"),
                DispatchInputs {
                    task,
                    role,
                    binding: &binding.binding,
                    profile: &binding.primary.profile,
                    host,
                    now: Timestamp(40),
                },
            )
            .unwrap()
        }

        /// An echo transport: whatever request the loop builds, the
        /// "provider" answers with a Stop response carrying `text` and the
        /// request's own identities (what a real provider must preserve).
        pub struct EchoTransport {
            pub text: String,
        }
        impl symbiote_native_agent::InferenceTransport for EchoTransport {
            fn request(
                &mut self,
                request: &symbiote_runtime_sdk::provider::ProviderRequest,
                _model: &symbiote_runtime_sdk::provider::ModelDescriptor,
            ) -> Result<
                symbiote_runtime_sdk::provider::ProviderResponse,
                symbiote_runtime_sdk::provider::ProviderError,
            > {
                Ok(symbiote_runtime_sdk::provider::ProviderResponse {
                    schema_version: symbiote_runtime_sdk::provider::PROVIDER_CONTRACT_VERSION,
                    request_id: request.request_id.clone(),
                    provider_id: request.provider_id.clone(),
                    model_id: request.model_id.clone(),
                    text: self.text.clone(),
                    tool_calls: Vec::new(),
                    finish_reason: symbiote_runtime_sdk::provider::FinishReason::Stop,
                    usage: symbiote_runtime_sdk::provider::TokenUsage::Known {
                        input_tokens: 1,
                        output_tokens: 1,
                        cached_input_tokens: None,
                        reasoning_tokens: None,
                    },
                })
            }
        }

        pub fn native_transport(
            responses: Vec<
                Result<
                    symbiote_runtime_sdk::provider::ProviderResponse,
                    symbiote_runtime_sdk::provider::ProviderError,
                >,
            >,
        ) -> symbiote_native_agent::ScriptedTransport {
            symbiote_native_agent::ScriptedTransport::new(responses)
        }

        /// A scripted Codex transport producing one complete turn.
        pub fn codex_transport() -> impl symbiote_external_agent::CodexTransport {
            ScriptedCodex::new(
                vec![
                    Ok(serde_json::json!({"userAgent": format!(
                        "symbiote/{} (Linux)",
                        symbiote_runtime_discovery::codex::CODEX_VERSION
                    )})),
                    Ok(serde_json::json!({"thread": {"id": "thr-fixture"}})),
                    Ok(serde_json::json!({"turn": {"id": "turn-fixture"}})),
                ],
                vec![
                    serde_json::json!({
                        "method": "item/completed",
                        "params": {"threadId": "thr-fixture", "turnId": "turn-fixture",
                            "item": {"type": "agentMessage", "id": "i1",
                                "text": "implemented the change"}}
                    }),
                    codex_completed("completed"),
                ],
                Vec::new(),
            )
        }

        pub fn codex_transport_failed() -> impl symbiote_external_agent::CodexTransport {
            ScriptedCodex::new(
                vec![
                    Ok(serde_json::json!({"userAgent": format!(
                        "symbiote/{} (Linux)",
                        symbiote_runtime_discovery::codex::CODEX_VERSION
                    )})),
                    Ok(serde_json::json!({"thread": {"id": "thr-fixture"}})),
                    Ok(serde_json::json!({"turn": {"id": "turn-fixture"}})),
                ],
                vec![codex_completed("failed")],
                Vec::new(),
            )
        }

        fn codex_completed(status: &str) -> serde_json::Value {
            serde_json::json!({
                "method": "turn/completed",
                "params": {"threadId": "thr-fixture", "turnId": "turn-fixture",
                    "turn": {"id": "turn-fixture", "status": status, "items": []}}
            })
        }

        /// Minimal scripted Codex transport for runner wiring tests.
        struct ScriptedCodex {
            responses: Vec<Result<serde_json::Value, symbiote_external_agent::DriverError>>,
            notifications: VecDeque<Option<serde_json::Value>>,
            server_requests: VecDeque<symbiote_external_agent::ServerRequest>,
            cursor: usize,
        }
        impl ScriptedCodex {
            fn new(
                responses: Vec<Result<serde_json::Value, symbiote_external_agent::DriverError>>,
                notifications: Vec<serde_json::Value>,
                server_requests: Vec<symbiote_external_agent::ServerRequest>,
            ) -> Self {
                Self {
                    responses,
                    notifications: notifications
                        .into_iter()
                        .map(Some)
                        .chain(std::iter::once(None))
                        .collect(),
                    server_requests: server_requests.into(),
                    cursor: 0,
                }
            }
        }
        impl symbiote_external_agent::CodexTransport for ScriptedCodex {
            fn call(
                &mut self,
                _method: &str,
                _params: &serde_json::Value,
            ) -> Result<serde_json::Value, symbiote_external_agent::DriverError> {
                let index = self.cursor;
                self.cursor += 1;
                self.responses
                    .get(index)
                    .cloned()
                    .unwrap_or(Err(symbiote_external_agent::DriverError::TransportFailed))
            }
            fn notify(
                &mut self,
                _method: &str,
                _params: &serde_json::Value,
            ) -> Result<(), symbiote_external_agent::DriverError> {
                Ok(())
            }
            fn recv_notification(
                &mut self,
            ) -> Result<Option<serde_json::Value>, symbiote_external_agent::DriverError>
            {
                Ok(self.notifications.pop_front().flatten())
            }
            fn recv_server_request(
                &mut self,
            ) -> Result<
                Option<symbiote_external_agent::ServerRequest>,
                symbiote_external_agent::DriverError,
            > {
                Ok(self.server_requests.pop_front())
            }
            fn refuse_server_request(
                &mut self,
                _request: &symbiote_external_agent::ServerRequest,
                _decision: symbiote_external_agent::ApprovalDecision,
            ) -> Result<(), symbiote_external_agent::DriverError> {
                Ok(())
            }
        }
    }
}
