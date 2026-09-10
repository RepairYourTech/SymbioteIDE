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
    Actor, CommandId, Dispatch, ElevationRequest, Permission, RuntimeKind, TaskAction, TaskCommand,
    TaskId, TaskState, Timestamp,
};
use symbiote_external_agent::{ApprovalRefusal, ExternalSession, StopKind};
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

/// The sandbox shell executor plus the worktree it runs in, attached to one
/// native run. The executor is operator composition: it consults the
/// operator's consent authority per invocation (`crate::shell_executor`) and
/// launches inside the reserved worktree. `None` leaves declared tools
/// propose-only — recorded, never executed.
pub struct ToolExecution {
    pub executor: Box<dyn symbiote_native_agent::tools::ShellToolExecutor>,
    pub worktree: std::path::PathBuf,
}

/// The trusted per-run inputs a shell executor factory receives, derived from
/// the dispatch contract and the provisioned worktree — never from loop or
/// model input.
pub struct ShellExecutorInputs<'a> {
    pub root_id: &'a symbiote_domain::RootId,
    pub worktree: &'a std::path::Path,
    pub host: &'a symbiote_domain::HostId,
    pub project_id: &'a symbiote_domain::ProjectId,
    pub role_id: &'a symbiote_domain::RoleId,
    pub profile_id: &'a symbiote_domain::RuntimeProfileId,
    /// The dispatch binding's access snapshot — the current policy the
    /// sandbox checks the consent's recorded access against.
    pub access: &'a symbiote_domain::AccessSnapshot,
    /// The consenting principal (the daemon's local owner).
    pub user_id: &'a symbiote_domain::UserId,
}

/// Builds one shell executor per native run. Production installs the
/// sandboxed launch composition with the operator's consent authority; tests
/// install scripted executors — a model cannot cause a side effect in a test
/// through any other path.
pub trait ShellExecutorFactory {
    fn build(
        &mut self,
        inputs: ShellExecutorInputs<'_>,
    ) -> Result<Box<dyn symbiote_native_agent::tools::ShellToolExecutor>, &'static str>;
}

/// The Host's configured worker transports. The daemon holds one instance;
/// the production constructor is empty — live native and external transports
/// require the sandboxed launch path plus explicit user authorization for
/// credentials and billing, so an unconfigured Host refuses activation with
/// a typed error instead of silently doing nothing or falling back.
#[derive(Default)]
pub struct WorkerTransports {
    native: Option<Box<dyn NativeTransportFactory>>,
    external: Option<Box<dyn ExternalTransportFactory>>,
    /// The operator's sandbox shell-executor composition. Absent means
    /// declared shell tools stay propose-only on native runs.
    shell: Option<Box<dyn ShellExecutorFactory>>,
    /// The operator's credential broker (#217). Absent means runs resolve
    /// context only — no credential leases are issued, and a profile
    /// referencing a credential gets the typed `NoCredentialBroker`
    /// refusal instead of a silent empty lease. The broker itself holds
    /// the ONLY plaintext; nothing here serializes values.
    pub(crate) broker: Option<std::rc::Rc<std::cell::RefCell<symbiote_context::CredentialBroker>>>,
    /// The Host's reserved-location base directory for derived worktrees
    /// (the premise symbiote-worktrees verifies). Empty means worktree
    /// provisioning has no configured base and refuses.
    pub(crate) reservation_base: Option<std::path::PathBuf>,
}

impl WorkerTransports {
    /// The production configuration: no live transports. Activation refuses
    /// with [`RunnerError::NoTransport`] until the operator explicitly
    /// configures one.
    pub fn production() -> Self {
        Self::default()
    }

    pub fn with_native(mut self, factory: Box<dyn NativeTransportFactory>) -> Self {
        self.native = Some(factory);
        self
    }

    pub fn with_external(mut self, factory: Box<dyn ExternalTransportFactory>) -> Self {
        self.external = Some(factory);
        self
    }

    pub fn with_shell_executor(mut self, factory: Box<dyn ShellExecutorFactory>) -> Self {
        self.shell = Some(factory);
        self
    }

    /// Attaches the operator's credential broker. The broker is shared
    /// (registration and revocation are operator surfaces outside the run
    /// path); a run only ever reads leases from it.
    pub fn with_credential_broker(
        mut self,
        broker: std::rc::Rc<std::cell::RefCell<symbiote_context::CredentialBroker>>,
    ) -> Self {
        self.broker = Some(broker);
        self
    }

    /// Configures the derived-worktree base directory. Provisioning is
    /// refused with a typed error until this is set.
    pub fn with_reservation_base(mut self, base: std::path::PathBuf) -> Self {
        self.reservation_base = Some(base);
        self
    }

    pub(crate) fn reservation_base(&self) -> Result<std::path::PathBuf, RunnerError> {
        self.reservation_base
            .clone()
            .ok_or(RunnerError::NoReservationBase)
    }

    pub(crate) fn git(&mut self) -> symbiote_repo::SystemGit {
        symbiote_repo::SystemGit::new()
    }

    pub fn native_build(
        &mut self,
    ) -> Result<Box<dyn symbiote_native_agent::InferenceTransport>, crate::runner::RunnerError>
    {
        match self.native.as_mut() {
            Some(factory) => factory
                .build()
                .map_err(crate::runner::RunnerError::TransportBuild),
            None => Err(crate::runner::RunnerError::NoTransport),
        }
    }

    pub fn external_build(
        &mut self,
    ) -> Result<Box<dyn symbiote_external_agent::CodexTransport>, crate::runner::RunnerError> {
        match self.external.as_mut() {
            Some(factory) => factory
                .build()
                .map_err(crate::runner::RunnerError::TransportBuild),
            None => Err(crate::runner::RunnerError::NoTransport),
        }
    }

    /// Builds the shell executor for one native run. Unconfigured is not an
    /// error — it is the propose-only composition — while a configured
    /// factory's refusal is a typed build failure.
    pub fn shell_build(
        &mut self,
        inputs: ShellExecutorInputs<'_>,
    ) -> Result<Option<Box<dyn symbiote_native_agent::tools::ShellToolExecutor>>, RunnerError> {
        match self.shell.as_mut() {
            Some(factory) => factory
                .build(inputs)
                .map(Some)
                .map_err(RunnerError::ShellExecutorBuild),
            None => Ok(None),
        }
    }

    /// Whether the operator provisioned a credential broker (#217). The
    /// dispatch compiler turns this into the Host's `Credentials`
    /// enforcement claim: a binding granting `UseCredential` cannot even
    /// start a dispatch on a Host that has no broker to hold the lease.
    pub fn has_credential_broker(&self) -> bool {
        self.broker.is_some()
    }

    /// Whether the operator provisioned the external execution path (the
    /// explicitly labeled fixture harness; a live pinned-binary path stays
    /// gated on explicit user authorization). The Host record advertises
    /// `ExternalHarness` support only when this is set: without operator
    /// provisioning the Host cannot execute external dispatches, so a
    /// binding with an external profile is refused at start instead of
    /// falling back to anything else.
    pub fn has_external_transport(&self) -> bool {
        self.external.is_some()
    }

    /// Resolves one credential lease for a dispatch from the operator's
    /// broker. Refusal identities are mapped payload-free. The lease's
    /// plaintext does not leave the broker/lease objects — no materialize
    /// call exists in this crate (environment injection into a transport
    /// is the pending next slice), and external runs never call this
    /// (harness credentials stay harness-owned).
    pub fn resolve_leases(
        &self,
        reference: &symbiote_domain::CredentialReferenceId,
        request: symbiote_context::BrokerRequest<'_>,
    ) -> Result<Option<symbiote_context::CredentialLease>, RunnerError> {
        let Some(broker) = self.broker.as_ref() else {
            return Err(RunnerError::NoCredentialBroker);
        };
        broker
            .borrow()
            .resolve(reference, request)
            .map(Some)
            .map_err(|error| RunnerError::CredentialRefused(broker_error_name(error)))
    }

    /// Resolves a native credential lease. A `use_credential_not_granted`
    /// refusal journals an elevation *ask* first — evidence, never a grant —
    /// then returns the same typed refusal so the run cannot proceed.
    pub(crate) fn resolve_native_credential(
        &self,
        store: &mut Store,
        task_id: &TaskId,
        dispatch: &Dispatch,
        reference: &symbiote_domain::CredentialReferenceId,
        request: symbiote_context::BrokerRequest<'_>,
        at: Timestamp,
    ) -> Result<Option<symbiote_context::CredentialLease>, RunnerError> {
        match self.resolve_leases(reference, request) {
            Ok(lease) => Ok(lease),
            Err(RunnerError::CredentialRefused("use_credential_not_granted")) => {
                file_elevation_ask(
                    store,
                    task_id,
                    dispatch,
                    Permission::UseCredential,
                    "native run referenced a credential without UseCredential on the binding",
                    at,
                )?;
                Err(RunnerError::CredentialRefused("use_credential_not_granted"))
            }
            Err(error) => Err(error),
        }
    }
}

fn broker_error_name(error: symbiote_context::BrokerError) -> &'static str {
    match error {
        symbiote_context::BrokerError::UnknownReference => "unknown_reference",
        symbiote_context::BrokerError::Revoked => "revoked",
        symbiote_context::BrokerError::CrossProjectDenied => "cross_project_denied",
        symbiote_context::BrokerError::NotReferencedByProfile => "not_referenced_by_profile",
        symbiote_context::BrokerError::UseCredentialNotGranted => "use_credential_not_granted",
        symbiote_context::BrokerError::InvalidEnvironmentLabel => "invalid_environment_label",
        symbiote_context::BrokerError::Expired => "expired",
        symbiote_context::BrokerError::NotYetValid => "not_yet_valid",
    }
}

/// Builds one native inference transport per run. Tests install a factory
/// producing the scripted transport; production installs the live client
/// only after explicit user authorization for credentials and billing.
pub trait NativeTransportFactory {
    fn build(&mut self)
    -> Result<Box<dyn symbiote_native_agent::InferenceTransport>, &'static str>;
}

/// Builds one external Codex transport per run: the factory owns the
/// sandboxed launch of the pinned binary (or, in tests, a scripted fixture).
pub trait ExternalTransportFactory {
    fn build(&mut self) -> Result<Box<dyn symbiote_external_agent::CodexTransport>, &'static str>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RunnerError {
    /// The dispatch's runtime kind does not match the loop being run.
    RuntimeMismatch,
    /// The task is not in the Running state under this dispatch.
    NotRunning,
    /// The dispatch contract no longer validates at the run time.
    InvalidContract,
    /// The loop halted without a completed turn; nothing was filed. The
    /// payload is the loop's own halt/error identity, so a caller-sequence
    /// bug (`invalid_input`, `already_complete`, `unknown_tool`) is never
    /// misread as a retryable transient failure.
    LoopFailed(&'static str),
    /// The loop completed but produced no reportable final message.
    NoReport,
    /// The store refused the filing. The payload names the store's own
    /// error (display form) so an idempotency collision is distinguishable
    /// from a state move.
    Store(String),
    /// No transport is configured for this dispatch's runtime kind. The
    /// Host operator has not authorized live execution; nothing ran and
    /// nothing was filed.
    NoTransport,
    /// The configured transport factory refused to build a transport.
    TransportBuild(&'static str),
    /// The Root record carries no repository placement for this Host.
    NoHostPath,
    /// The Host has not configured a reservation base directory for
    /// derived worktrees; provisioning refuses before anything runs.
    NoReservationBase,
    /// Worktree provisioning refused at a named stage (reservation
    /// verification, base validation, or git materialization).
    Provisioning(symbiote_repo::provision::ProvisionError),
    /// The Host's policy seed was rejected by the worktrees crate.
    InvalidSeed,
    /// A provisioning-time store read or integrity check failed.
    ProvisioningStore(String),
    /// The configured shell executor factory refused to build. Nothing ran
    /// and nothing was filed.
    ShellExecutorBuild(&'static str),
    /// The dispatch's profile references a credential, but the operator
    /// has not configured a credential broker. Nothing ran and nothing
    /// was filed — the run cannot receive its referenced credentials.
    NoCredentialBroker,
    /// The operator's broker refused this dispatch's lease (cross-project
    /// reference, revoked, ungranted environment). Typed, payload-free.
    CredentialRefused(&'static str),
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
    let task = store
        .task(task_id)
        .map_err(|error| RunnerError::Store(error.to_string()))?;
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
    let task = store
        .task(task_id)
        .map_err(|error| RunnerError::Store(error.to_string()))?;
    // The command id carries the dispatch id and the filing time: a Host
    // retry of the SAME observed run replays honestly (byte-identical
    // request), while a NEW run of the task gets a fresh id and can never
    // be silently swallowed by the old receipt.
    let id = CommandId::new(format!(
        "worker-completion-{}-{}",
        dispatch.id().as_str(),
        at.0
    ))
    .map_err(|error| RunnerError::Store(error.to_string()))?;
    let command = TaskCommand {
        id,
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
            // Surfaced, never swallowed: an illegal transition means the
            // state moved under us (lease expiry, concurrent edit); an
            // idempotency conflict means a different command already used
            // this id. The loop's evidence stays with the caller; the Host
            // decides the retry.
            RunnerError::Store(error.to_string())
        })?;
    Ok(())
}

fn permission_name(permission: &Permission) -> &'static str {
    match permission {
        Permission::ReadRoot => "read_root",
        Permission::MutateStream => "mutate_stream",
        Permission::ExecuteProcess => "execute_process",
        Permission::Network => "network",
        Permission::UseCredential => "use_credential",
    }
}

/// The shell tool path's current policy: the dispatch binding's access,
/// widened with ExecuteProcess ONLY while an owner-decided elevation lease
/// for THIS dispatch is active at the run's clock — the same per-run basis
/// as the broker's UseCredential gate. A filed ask never widens (filing is
/// evidence, never a grant). The policy is resolved once per run, so an
/// expired or sticky-revoked lease stops the widening from the next run's
/// clock onward; mid-run policy-change semantics remain #269 scope. Every
/// other grant stays exactly what the binding carries.
pub(crate) fn resolve_shell_access(
    store: &Store,
    dispatch: &Dispatch,
    at: Timestamp,
) -> Result<symbiote_domain::AccessSnapshot, RunnerError> {
    let mut access = dispatch.contract().effective_access().clone();
    if store
        .active_elevation(dispatch.id(), &Permission::ExecuteProcess, at)
        .map_err(|error| RunnerError::Store(error.to_string()))?
    {
        access.grants.insert(Permission::ExecuteProcess);
    }
    Ok(access)
}

/// FNV-1a: stable across processes and Rust versions, unlike
/// `DefaultHasher`, so a Host retry mints the same ask id after an
/// upgrade and replays honestly instead of duplicating evidence.
fn dispatch_id_digest(dispatch_id: &str) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in dispatch_id.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// The ask id carries a bounded digest of the dispatch id — a DispatchId
/// itself may run to 128 chars, which would overflow CommandId's own bound
/// — plus the permission and clock in the clear.
fn elevation_ask_command_id(
    dispatch_id: &str,
    permission: &Permission,
    at: Timestamp,
) -> Result<CommandId, RunnerError> {
    CommandId::new(format!(
        "worker-elevation-{:016x}-{}-{}",
        dispatch_id_digest(dispatch_id),
        permission_name(permission),
        at.0
    ))
    .map_err(|error| RunnerError::Store(error.to_string()))
}

/// Files a worker elevation ask as journaled evidence. NEVER licenses:
/// `active_elevation` still reads only decided leases. The id is
/// dispatch+permission+clock so a Host retry of the same observed ask
/// replays honestly.
pub(crate) fn file_elevation_ask(
    store: &mut Store,
    task_id: &TaskId,
    dispatch: &Dispatch,
    permission: Permission,
    reason: &str,
    at: Timestamp,
) -> Result<(), RunnerError> {
    let task = store
        .task(task_id)
        .map_err(|error| RunnerError::Store(error.to_string()))?;
    let id = elevation_ask_command_id(dispatch.id().as_str(), &permission, at)?;
    let ask = ElevationRequest {
        id: id.clone(),
        project_id: task.project_id().clone(),
        task_id: task_id.clone(),
        dispatch_id: dispatch.id().clone(),
        permission,
        reason: reason.to_owned(),
        requested_at: at,
    };
    store
        .request_elevation(id, ask)
        .map(|_| ())
        .map_err(|error| RunnerError::Store(error.to_string()))
}

fn permission_for_harness_refusal(refusal: ApprovalRefusal) -> Option<Permission> {
    match refusal {
        ApprovalRefusal::ExecCommand
        | ApprovalRefusal::CommandExecutionRequest
        | ApprovalRefusal::PermissionsRequest => Some(Permission::ExecuteProcess),
        ApprovalRefusal::ApplyPatch | ApprovalRefusal::FileChangeRequest => {
            Some(Permission::MutateStream)
        }
        ApprovalRefusal::McpElicitation => Some(Permission::Network),
        ApprovalRefusal::ToolCall | ApprovalRefusal::ToolUserInput | ApprovalRefusal::Unknown => {
            None
        }
    }
}

fn file_harness_elevation_asks(
    store: &mut Store,
    task_id: &TaskId,
    dispatch: &Dispatch,
    refusals: &[ApprovalRefusal],
    at: Timestamp,
) -> Result<(), RunnerError> {
    let mut filed = std::collections::BTreeSet::new();
    for refusal in refusals {
        let Some(permission) = permission_for_harness_refusal(*refusal) else {
            continue;
        };
        if !filed.insert(permission.clone()) {
            continue;
        }
        file_elevation_ask(
            store,
            task_id,
            dispatch,
            permission,
            "external harness requested a capability beyond the dispatch binding and was refused",
            at,
        )?;
    }
    Ok(())
}

/// Provisioning outcome for a run: the materialized worktree path and the
/// observed HEAD the work starts from (recorded by the caller for the
/// dispatch's evidence trail). The branch is the stream's derived branch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorktreeProvisioned {
    /// The stream's Root identity — the consent descriptors for shell tool
    /// execution carry it, never a caller-derived value.
    pub root_id: symbiote_domain::RootId,
    pub worktree: std::path::PathBuf,
    pub branch: String,
    pub observed_head: String,
}

/// Resolves and provisions the worktree for a started dispatch from trusted
/// store state: the task's Change Stream (worktree id, derived branch,
/// recorded base commit) and the Root's host-path placement for this Host.
/// Ordering is the #211 contract — reservation verify, base validation
/// against the stream's recorded base, then materialization — and failures
/// are stage-honest. `host_id` selects the Root's observed placement; a
/// Root with no placement for this Host is `NoHostPath` (documented as
/// landing here from PR #503's review).
pub fn provision_worktree(
    store: &Store,
    task_id: &TaskId,
    host_id: &symbiote_domain::HostId,
    git: &mut impl symbiote_repo::GitExecutor,
    reservation_base: &std::path::Path,
) -> Result<WorktreeProvisioned, RunnerError> {
    let task = store.task(task_id).map_err(provisioning_store_error)?;
    let stream = store
        .task_stream(task_id)
        .map_err(provisioning_store_error)?;
    assert_task_in_stream(&stream, task_id)?;
    let root_record = store
        .root(stream.root_id())
        .map_err(provisioning_store_error)?;
    let placement = root_record
        .host_paths
        .get(host_id)
        .ok_or(RunnerError::NoHostPath)?;
    // The policy seed is Host-owned configuration, not client JSON; the
    // derivation must reproduce the stream's recorded identities (the
    // tamper check inside provision).
    let policy_seed = symbiote_worktrees_policy_seed(stream.id())?;
    let inputs = symbiote_repo::provision::ProvisionInputs {
        stream: &stream,
        root_id: stream.root_id(),
        project_id: task.project_id(),
        stream_id: task.stream_id(),
        policy_seed: &policy_seed,
        reservation_base,
        source_repository: std::path::Path::new(placement),
    };
    let provisioned = symbiote_repo::provision::provision(git, inputs).map_err(provision_error)?;
    Ok(WorktreeProvisioned {
        root_id: stream.root_id().clone(),
        worktree: provisioned.worktree,
        branch: provisioned.branch,
        observed_head: provisioned.head.commit,
    })
}

/// Provisioning-time store failures: stage-honest instead of reusing the
/// completion-filing error identity.
fn provisioning_store_error(error: symbiote_store::StoreError) -> RunnerError {
    RunnerError::ProvisioningStore(error.to_string())
}

/// Cheap reader-side defense-in-depth: the stream's task set must contain
/// the task whose stream id named it.
fn assert_task_in_stream(
    stream: &symbiote_domain::ChangeStream,
    task: &TaskId,
) -> Result<(), RunnerError> {
    if stream.tasks().contains(task) {
        Ok(())
    } else {
        Err(RunnerError::ProvisioningStore(
            "stream task set integrity".into(),
        ))
    }
}

fn provision_error(error: symbiote_repo::provision::ProvisionError) -> RunnerError {
    RunnerError::Provisioning(error)
}

/// The Host's worktree policy seed: stable Host-owned configuration that
/// binds the derived naming namespace. One seed per store; it is derived
/// from the stream's id namespace rather than client input, so a client
/// cannot steer the derivation.
fn symbiote_worktrees_policy_seed(
    stream: &symbiote_domain::ChangeStreamId,
) -> Result<String, RunnerError> {
    // Host-owned policy: a digest of the stream id, NOT the id itself.
    // Domain ids allow up to 128 bytes but the worktrees crate's seed bound
    // is 64 — embedding the raw id would break every legal id of 61+ bytes.
    // A hex digest is always 64 bytes of legal charset and deterministic
    // per stream, so the derivation binds the identity without embedding
    // it.
    let digest = symbiote_trust::Fingerprint::of(stream.as_str().as_bytes());
    symbiote_worktrees::policy_seed(digest.as_str())
        .map(|seed| seed.to_owned())
        .map_err(|_| RunnerError::InvalidSeed)
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
    run_native_boxed(store, task_id, dispatch, task_prompt, transport, None, at)
}

/// Object-safe entry for the service: same checks, dyn-dispatched transport.
/// `tool_execution` attaches the operator's sandbox shell executor to the
/// reserved worktree; `None` leaves declared shell tools propose-only.
pub fn run_native_boxed(
    store: &mut Store,
    task_id: &TaskId,
    dispatch: &Dispatch,
    task_prompt: &str,
    transport: &mut dyn symbiote_native_agent::InferenceTransport,
    tool_execution: Option<ToolExecution>,
    at: Timestamp,
) -> Result<WorkerOutcome, RunnerError> {
    check_runnable(store, task_id, dispatch, RuntimeKind::NativeSymbiote)?;
    // The model envelope comes from the Host's provider registry (#464),
    // keyed by the contract's profile model — never from loop input.
    let model = store
        .model_descriptor(&dispatch.contract().profile().model)
        .map_err(|error| RunnerError::Store(error.to_string()))?;
    let session = NativeSession::new(dispatch, model, at).map_err(native_error)?;
    let mut session = match tool_execution {
        Some(execution) => session.with_tool_execution(execution.executor, execution.worktree),
        None => session,
    };
    let summary = session.run(task_prompt, transport).map_err(native_error)?;
    // Only a model-finished turn is a clean stop. The native loop maps
    // FinishReason::Cancelled to the same HaltReason::Stop as a normal
    // finish, so a cancelled turn is recognized by its terminal event: it
    // records CancelAcknowledged, not Exit(0). Filing a cancelled turn
    // would make an unfinished turn look worker-complete.
    let cancelled = session
        .events()
        .iter()
        .any(|event| matches!(event.payload(), RuntimeEventKind::CancelAcknowledged {}));
    if cancelled || summary.halted != Some(symbiote_native_agent::HaltReason::Stop) {
        let reason = match summary.halted {
            Some(halt) => halt_name(halt),
            None => "none",
        };
        return Err(RunnerError::LoopFailed(reason));
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
        other => RunnerError::LoopFailed(native_error_name(other)),
    }
}

/// The loop error's own identity, so caller-sequencing bugs
/// (`invalid_input`, `already_complete`) are never misread as retryable
/// transient failures like `provider_failed`.
fn native_error_name(error: LoopError) -> &'static str {
    match error {
        LoopError::InvalidContract => "invalid_contract",
        LoopError::BudgetExhausted => "budget_exhausted",
        LoopError::TurnCapReached => "turn_cap_reached",
        LoopError::ProviderFailed => "provider_failed",
        LoopError::EnvelopeMismatch => "envelope_mismatch",
        LoopError::UnknownTool => "unknown_tool",
        LoopError::AlreadyComplete => "already_complete",
        LoopError::InvalidInput => "invalid_input",
    }
}

fn halt_name(halt: symbiote_native_agent::HaltReason) -> &'static str {
    match halt {
        symbiote_native_agent::HaltReason::Stop => "stop",
        symbiote_native_agent::HaltReason::Length => "length",
        symbiote_native_agent::HaltReason::BudgetExhausted => "budget_exhausted",
        symbiote_native_agent::HaltReason::TurnCapReached => "turn_cap_reached",
        symbiote_native_agent::HaltReason::ProviderFailed => "provider_failed",
        symbiote_native_agent::HaltReason::EnvelopeMismatch => "envelope_mismatch",
        symbiote_native_agent::HaltReason::UnknownTool => "unknown_tool",
        symbiote_native_agent::HaltReason::OutputBackstop => "output_backstop",
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
    run_external_boxed(
        store,
        task_id,
        dispatch,
        task_prompt,
        worktree_cwd,
        transport,
        at,
    )
}

/// Object-safe entry for the service: same checks, dyn-dispatched transport.
pub fn run_external_boxed(
    store: &mut Store,
    task_id: &TaskId,
    dispatch: &Dispatch,
    task_prompt: &str,
    worktree_cwd: &str,
    transport: &mut dyn symbiote_external_agent::CodexTransport,
    at: Timestamp,
) -> Result<WorkerOutcome, RunnerError> {
    check_runnable(store, task_id, dispatch, RuntimeKind::ExternalHarness)?;
    let mut session = ExternalSession::new(dispatch, at).map_err(external_error)?;
    session
        .begin_thread(worktree_cwd, transport)
        .map_err(external_error)?;
    let turn_result = session.turn(task_prompt, transport).map_err(external_error);
    // File harness escalation asks even when the turn later fails: the
    // driver already refused, and the journaled ask never licenses. The
    // turn error outranks a filing failure so the operator sees why the
    // run actually failed.
    let refusals = session.refused_approvals();
    let filing = file_harness_elevation_asks(store, task_id, dispatch, &refusals, at);
    turn_result?;
    filing?;
    let stopped = session.run_summary().stopped;
    if stopped != Some(StopKind::Completed) {
        let name = match stopped {
            Some(StopKind::Completed) => "completed",
            Some(StopKind::Failed) => "failed",
            Some(StopKind::Interrupted) => "interrupted",
            Some(StopKind::TransportLost) => "transport_lost",
            None => "none",
        };
        return Err(RunnerError::LoopFailed(name));
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
    use symbiote_external_agent::DriverError;
    match error {
        DriverError::InvalidContract => RunnerError::InvalidContract,
        other => RunnerError::LoopFailed(external_error_name(other)),
    }
}

fn external_error_name(error: symbiote_external_agent::DriverError) -> &'static str {
    use symbiote_external_agent::DriverError;
    match error {
        DriverError::InvalidContract => "invalid_contract",
        DriverError::NotExternalHarness => "not_external_harness",
        DriverError::InvalidInput => "invalid_input",
        DriverError::TransportFailed => "transport_failed",
        DriverError::MalformedFrame => "malformed_frame",
        DriverError::UnexpectedResponse => "unexpected_response",
        DriverError::RpcFailure => "rpc_failure",
        DriverError::UnsupportedVersion => "unsupported_version",
        DriverError::ContractMismatch => "contract_mismatch",
        DriverError::AlreadyComplete => "already_complete",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeSet, VecDeque};
    use std::path::Path;
    use std::process::Command;
    use symbiote_domain::HostId;

    fn store_with_running_task(tag: &str, runtime: RuntimeKind) -> (Store, TaskId, Dispatch) {
        store_with_running_task_tools(tag, runtime, &[])
    }

    fn store_with_running_task_tools(
        tag: &str,
        runtime: RuntimeKind,
        required_tools: &[&str],
    ) -> (Store, TaskId, Dispatch) {
        store_with_running_task_composed(
            tag,
            runtime,
            required_tools,
            &[
                Permission::ReadRoot,
                Permission::ExecuteProcess,
                Permission::MutateStream,
                Permission::UseCredential,
            ],
        )
    }

    /// The elevation-consumer tests need a running dispatch whose binding
    /// LACKS ExecuteProcess (so a decided lease is the only path to it)
    /// while the Team ceiling still carries it (so the owner may decide).
    fn store_with_running_task_worker_grants(
        tag: &str,
        runtime: RuntimeKind,
        worker_grants: &[Permission],
    ) -> (Store, TaskId, Dispatch) {
        store_with_running_task_composed(tag, runtime, &[], worker_grants)
    }

    fn store_with_running_task_composed(
        tag: &str,
        runtime: RuntimeKind,
        required_tools: &[&str],
        worker_grants: &[Permission],
    ) -> (Store, TaskId, Dispatch) {
        let mut store = Store::memory().unwrap();
        let (project, task) = fixture::full_fixture_with_worker_grants(
            &mut store,
            tag,
            runtime,
            None,
            None,
            required_tools,
            worker_grants,
        );
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
        let (_, started) = store
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
        // Fidelity pin: the store re-reads the role/binding from its own
        // tables and compiles its own dispatch. If the fixture's hand-built
        // inputs ever diverged from registered state, this identity check
        // fails loudly instead of testing against a phantom dispatch.
        assert_eq!(started.id(), dispatch.id());
        (store, task, dispatch)
    }

    #[test]
    fn transport_factories_build_and_drive_both_runtimes() {
        // The service→factory→boxed-runner chain with Some(factory): the
        // native factory hands the loop a scripted transport, the external
        // factory a scripted fixture, both through WorkerTransports.
        let (mut store, task, dispatch) =
            store_with_running_task("factory-native", RuntimeKind::NativeSymbiote);
        let mut transports =
            WorkerTransports::default().with_native(Box::new(fixture::EchoFactory {
                text: "implemented the change".into(),
            }));
        let mut boxed = transports.native_build().unwrap();
        let outcome = run_native_boxed(
            &mut store,
            &task,
            &dispatch,
            "do the work",
            boxed.as_mut(),
            None,
            Timestamp(60),
        )
        .unwrap();
        assert!(outcome.completion_filed);
        assert_eq!(
            store.task(&task).unwrap().state(),
            &TaskState::CompletionRequested
        );

        let (mut store, task, dispatch) =
            store_with_running_task("factory-external", RuntimeKind::ExternalHarness);
        let mut transports =
            WorkerTransports::default().with_external(Box::new(fixture::ScriptedFactory));
        let mut boxed = transports.external_build().unwrap();
        let outcome = run_external_boxed(
            &mut store,
            &task,
            &dispatch,
            "do the work",
            "/workspace",
            boxed.as_mut(),
            Timestamp(60),
        )
        .unwrap();
        assert!(outcome.completion_filed);
        assert_eq!(
            store.task(&task).unwrap().state(),
            &TaskState::CompletionRequested
        );
        // An empty factory is the production shape: typed refusal.
        let mut empty = WorkerTransports::default();
        assert!(matches!(
            empty.native_build(),
            Err(RunnerError::NoTransport)
        ));
        assert!(matches!(
            empty.external_build(),
            Err(RunnerError::NoTransport)
        ));
    }

    fn store_with_placed_running_task(
        tag: &str,
        runtime: RuntimeKind,
        host: &HostId,
        repo_dir: &Path,
        base_sha: &str,
    ) -> (Store, TaskId, Dispatch) {
        let mut store = Store::memory().unwrap();
        let (project, task) = fixture::full_fixture_with_placement(
            &mut store,
            tag,
            runtime,
            Some((host, repo_dir)),
            Some(base_sha),
            &[],
        );
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
    fn provision_worktree_refuses_roots_without_this_hosts_placement() {
        // The fixture's Root record carries no host_paths entry: this
        // Host has no observed placement, so provisioning refuses with
        // NoHostPath before any git call (the root placement comes from
        // trusted store state, never from the caller).
        let (store, task, _dispatch) =
            store_with_running_task("prov-wire", RuntimeKind::ExternalHarness);
        let host_id = HostId::new("unplaced-host").unwrap();
        let reservation_base = std::env::temp_dir().join(format!(
            "symbiote-provwire-{}-base-nopl",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&reservation_base);
        std::fs::create_dir_all(&reservation_base).unwrap();
        let mut git = symbiote_repo::SystemGit::new();
        assert_eq!(
            provision_worktree(&store, &task, &host_id, &mut git, &reservation_base),
            Err(RunnerError::NoHostPath)
        );
        let _ = std::fs::remove_dir_all(&reservation_base);
    }

    #[test]
    fn provision_worktree_materializes_from_store_records() {
        // Full happy path: the fixture Root's placement points at a real
        // git repository whose HEAD equals the stream's recorded base.
        let host_id = HostId::new("host-prov-wire").unwrap();
        let repo_dir =
            std::env::temp_dir().join(format!("symbiote-provwire-{}-repo", std::process::id()));
        let _ = std::fs::remove_dir_all(&repo_dir);
        std::fs::create_dir_all(&repo_dir).unwrap();
        let run = |args: &[&str]| {
            let output = Command::new("git")
                .arg("-C")
                .arg(&repo_dir)
                .args(args)
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "git {args:?}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            String::from_utf8_lossy(&output.stdout).trim().to_owned()
        };
        run(&["init", "-q", "-b", "main"]);
        std::fs::write(repo_dir.join("base.txt"), "base\n").unwrap();
        run(&["add", "."]);
        run(&[
            "-c",
            "user.email=t@t",
            "-c",
            "user.name=t",
            "commit",
            "-q",
            "-m",
            "base",
        ]);
        let base_sha = run(&["rev-parse", "HEAD"]);

        // Register the project with a Root record whose host_paths carry
        // this Host's placement, then build the stream from the same seed
        // policy the host derives. The fixture functions are reused for
        // team/binding/provider; the project registration is bespoke.
        let (store, task, _dispatch) = store_with_placed_running_task(
            "prov-full",
            RuntimeKind::ExternalHarness,
            &host_id,
            &repo_dir,
            &base_sha,
        );

        let reservation_base =
            std::env::temp_dir().join(format!("symbiote-provwire-{}-base-mat", std::process::id()));
        let _ = std::fs::remove_dir_all(&reservation_base);
        std::fs::create_dir_all(&reservation_base).unwrap();
        let mut git = symbiote_repo::SystemGit::new();
        let provisioned =
            provision_worktree(&store, &task, &host_id, &mut git, &reservation_base).unwrap();
        assert_eq!(provisioned.observed_head, base_sha);
        assert!(provisioned.worktree.join("base.txt").exists());
        // The materialized HEAD is the derived branch AT the base.
        let head = symbiote_repo::observe_head(&mut git, &provisioned.worktree).unwrap();
        assert_eq!(head.commit, base_sha);
        assert!(matches!(
            &head.state,
            symbiote_repo::HeadState::Branch { .. }
        ));
        // Re-provisioning refuses honestly (non-empty worktree).
        let second = provision_worktree(&store, &task, &host_id, &mut git, &reservation_base);
        // The stage must be Reservation (the reservation layer's
        // non-empty rule), not a Store or wrong-stage error.
        assert!(matches!(
            second,
            Err(RunnerError::Provisioning(
                symbiote_repo::provision::ProvisionError::Reservation
            ))
        ));
        let _ = std::fs::remove_dir_all(&repo_dir);
        let _ = std::fs::remove_dir_all(&reservation_base);
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
            Err(RunnerError::LoopFailed("provider_failed"))
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
            Err(RunnerError::LoopFailed("failed"))
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

    #[test]
    fn external_completed_turn_without_message_files_nothing() {
        let (mut store, task, dispatch) =
            store_with_running_task("no-report", RuntimeKind::ExternalHarness);
        // A completed turn whose only item is a commandExecution: the
        // NoReport path fires instead of filing an empty report the domain
        // would reject.
        let mut transport = fixture::ScriptedCodex::new(
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
                        "item": {"type": "commandExecution", "id": "c1",
                            "command": "cargo test", "status": "completed"}}
                }),
                fixture::codex_completed("completed"),
            ],
            Vec::new(),
        );
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
            Err(RunnerError::NoReport)
        );
        assert_eq!(store.task(&task).unwrap().state(), &TaskState::Running);
    }

    #[test]
    fn external_interrupted_turn_files_nothing() {
        let (mut store, task, dispatch) =
            store_with_running_task("interrupted-run", RuntimeKind::ExternalHarness);
        let mut transport = fixture::codex_transport_with_status("interrupted");
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
            Err(RunnerError::LoopFailed("interrupted"))
        );
        assert_eq!(store.task(&task).unwrap().state(), &TaskState::Running);
    }

    #[test]
    fn native_cancelled_turn_files_nothing() {
        // The native loop maps FinishReason::Cancelled to HaltReason::Stop;
        // the runner must look at the terminal event, not just the halt
        // reason, so a cancelled turn is never filed as worker-complete.
        let (mut store, task, dispatch) =
            store_with_running_task("cancelled-run", RuntimeKind::NativeSymbiote);
        let mut transport = fixture::CancelledTransport;
        assert_eq!(
            run_native(
                &mut store,
                &task,
                &dispatch,
                "do the work",
                &mut transport,
                Timestamp(60)
            ),
            Err(RunnerError::LoopFailed("stop"))
        );
        assert_eq!(store.task(&task).unwrap().state(), &TaskState::Running);
    }

    #[test]
    fn shell_tool_execution_runs_through_the_attached_executor_and_files_completion() {
        // The full dispatch→provision-free runner path with tool execution:
        // the scripted transport proposes `shell` on turn one and the
        // executor's recorded output must enter the conversation before the
        // final report; the completion evidence is filed only after the turn
        // finishes.
        let (mut store, task, dispatch) =
            store_with_running_task_tools("shell-exec", RuntimeKind::NativeSymbiote, &["shell"]);
        let mut transport = fixture::ToolProposalTransport::new(
            "ran the tool",
            vec![symbiote_runtime_sdk::provider::ProviderToolCall {
                call_id: symbiote_domain::RequestId::new("call-exec").unwrap(),
                name: "shell".into(),
                arguments: serde_json::json!({
                    "program": "cargo",
                    "arguments": ["test", "--locked"]
                }),
            }],
        );
        let executor = fixture::FixedOutputExecutor {
            output: b"test result: ok. 1 passed".to_vec(),
            code: Some(0),
        };
        let worktree = std::env::temp_dir().join(format!(
            "symbiote-shell-exec-{}-{}",
            std::process::id(),
            "shell-exec"
        ));
        let _ = std::fs::remove_dir_all(&worktree);
        std::fs::create_dir_all(&worktree).unwrap();
        let outcome = run_native_boxed(
            &mut store,
            &task,
            &dispatch,
            "run the tests",
            &mut transport,
            Some(ToolExecution {
                executor: Box::new(executor),
                worktree: worktree.clone(),
            }),
            Timestamp(60),
        )
        .unwrap();
        assert!(outcome.completion_filed);
        assert_eq!(
            store.task(&task).unwrap().state(),
            &TaskState::CompletionRequested
        );
        let _ = std::fs::remove_dir_all(&worktree);
    }

    #[test]
    fn a_refused_shell_tool_feeds_back_and_the_turn_can_still_complete() {
        // Consent refusal is recorded feedback, not a crash: ToolFailed
        // carries the constant refusal phrase, the run continues, and the
        // turn still files its completion evidence.
        let (mut store, task, dispatch) =
            store_with_running_task_tools("shell-refuse", RuntimeKind::NativeSymbiote, &["shell"]);
        let mut transport = fixture::ToolProposalTransport::new(
            "adapted after refusal",
            vec![symbiote_runtime_sdk::provider::ProviderToolCall {
                call_id: symbiote_domain::RequestId::new("call-refused").unwrap(),
                name: "shell".into(),
                arguments: serde_json::json!({"program": "rm", "arguments": []}),
            }],
        );
        let executor = fixture::RefusingExecutor;
        let worktree = std::env::temp_dir().join(format!(
            "symbiote-shell-exec-{}-{}",
            std::process::id(),
            "shell-refuse"
        ));
        let _ = std::fs::remove_dir_all(&worktree);
        std::fs::create_dir_all(&worktree).unwrap();
        let outcome = run_native_boxed(
            &mut store,
            &task,
            &dispatch,
            "do something",
            &mut transport,
            Some(ToolExecution {
                executor: Box::new(executor),
                worktree: worktree.clone(),
            }),
            Timestamp(60),
        )
        .unwrap();
        assert!(outcome.completion_filed);
        assert_eq!(
            store.task(&task).unwrap().state(),
            &TaskState::CompletionRequested
        );
        // The transport already asserted the refusal entered the
        // conversation as the constant phrase (no command echo, no task
        // content); nothing else to check here.
        let _ = std::fs::remove_dir_all(&worktree);
    }

    #[test]
    fn shell_executor_factory_refusal_is_a_typed_error() {
        // A configured factory that refuses to build surfaces the typed
        // build error at the WorkerTransports level. Service-level ordering
        // (the build happens after provisioning, before run_native_boxed)
        // means this refusal files nothing; the pinned daemon expectations
        // (reservation-base refusal first) are separate tests.
        let (store, task, dispatch) =
            store_with_running_task("shell-buildref", RuntimeKind::NativeSymbiote);
        let mut transports = WorkerTransports::default()
            .with_shell_executor(Box::new(fixture::RefusingShellFactory));
        let worktree = std::env::temp_dir().join("symbiote-shell-exec-buildref");
        let _ = std::fs::remove_dir_all(&worktree);
        std::fs::create_dir_all(&worktree).unwrap();
        // Direct factory-level refusal through shell_build with the fixture
        // contract's identities.
        let inputs = ShellExecutorInputs {
            root_id: &symbiote_domain::RootId::new("root-shell-buildref").unwrap(),
            worktree: &worktree,
            host: &symbiote_domain::HostId::new("host-shell-buildref").unwrap(),
            project_id: &symbiote_domain::ProjectId::new("project-shell-buildref").unwrap(),
            role_id: &symbiote_domain::RoleId::new("worker-shell-buildref").unwrap(),
            profile_id: &symbiote_domain::RuntimeProfileId::new("profile-shell-buildref").unwrap(),
            access: dispatch.contract().effective_access(),
            user_id: &symbiote_domain::UserId::new("owner").unwrap(),
        };
        assert!(matches!(
            transports.shell_build(inputs),
            Err(RunnerError::ShellExecutorBuild("operator refused"))
        ));
        assert_eq!(store.task(&task).unwrap().state(), &TaskState::Running);
        let _ = std::fs::remove_dir_all(&worktree);
    }

    #[test]
    fn run_context_resolves_from_store_and_reaches_the_loop_prompt() {
        // End to end through the real store fixture: the run prompt now
        // carries the origin work item's description AND its structured
        // guidance (requirements/constraints/acceptance), resolved by
        // symbiote-context over the store-backed source. The scripted
        // transport asserts the rendered sections arrived in the task
        // prompt it received.
        let (mut store, task, dispatch) =
            store_with_running_task("ctx-resolve", RuntimeKind::NativeSymbiote);
        struct PromptAssertor {
            seen: std::cell::RefCell<Option<String>>,
        }
        impl symbiote_native_agent::InferenceTransport for PromptAssertor {
            fn request(
                &mut self,
                request: &symbiote_runtime_sdk::provider::ProviderRequest,
                _model: &symbiote_runtime_sdk::provider::ModelDescriptor,
            ) -> Result<
                symbiote_runtime_sdk::provider::ProviderResponse,
                symbiote_runtime_sdk::provider::ProviderError,
            > {
                use symbiote_runtime_sdk::provider::{
                    FinishReason, PROVIDER_CONTRACT_VERSION, ProviderResponse,
                };
                *self.seen.borrow_mut() = Some(
                    request
                        .messages
                        .iter()
                        .filter_map(|m| {
                            m.content.iter().find_map(|p| match p {
                                symbiote_runtime_sdk::provider::InputPart::Text { text } => {
                                    Some(text.clone())
                                }
                                _ => None,
                            })
                        })
                        .next()
                        .unwrap_or_default(),
                );
                Ok(ProviderResponse {
                    schema_version: PROVIDER_CONTRACT_VERSION,
                    request_id: request.request_id.clone(),
                    provider_id: request.provider_id.clone(),
                    model_id: request.model_id.clone(),
                    text: "done".into(),
                    tool_calls: Vec::new(),
                    finish_reason: FinishReason::Stop,
                    usage: symbiote_runtime_sdk::provider::TokenUsage::Known {
                        input_tokens: 1,
                        output_tokens: 1,
                        cached_input_tokens: None,
                        reasoning_tokens: None,
                    },
                })
            }
        }
        let assertor = PromptAssertor {
            seen: std::cell::RefCell::new(None),
        };
        let mut transport = assertor;
        let outcome = run_native(
            &mut store,
            &task,
            &dispatch,
            "Owns task ctx-resolve\n\nRequirements:\n- requirement-of-ctx-resolve\n\nConstraints:\n- constraint-of-ctx-resolve\n\nAcceptance criteria:\n- acceptance-of-ctx-resolve",
            &mut transport,
            Timestamp(60),
        )
        .unwrap();
        assert!(outcome.completion_filed);
        // The store-backed resolution path: the task's origin work is
        // resolvable and carries the fixture's structured guidance.
        let (project, origin_work) =
            crate::context_resolution::task_origin_work(&store, &task).unwrap();
        let work = store.work_item(&project, &origin_work).unwrap();
        let spec = work.spec();
        assert_eq!(spec.requirements, vec!["requirement-of-ctx-resolve"]);
        assert_eq!(spec.acceptance, vec!["acceptance-of-ctx-resolve"]);
        // The loop received the prompt the daemon renders from these parts.
        let seen = transport.seen.borrow().clone().unwrap_or_default();
        assert!(seen.starts_with("Owns task ctx-resolve"));
        assert!(seen.contains("Requirements:\n- requirement-of-ctx-resolve"));
        assert!(seen.contains("Acceptance criteria:\n- acceptance-of-ctx-resolve"));
    }

    #[test]
    fn lease_resolution_runs_through_the_real_contract_scope() {
        // The reviewer-required end-to-end seam test (PR #508 round 1): a
        // broker configured, the fixture's own credential reference
        // registered, and the scope fields taken from the REAL dispatch
        // contract — proving the passing lease, the UseCredential gate,
        // and the typed refusals through WorkerTransports.
        let (store, task, dispatch) =
            store_with_running_task("lease-e2e", RuntimeKind::NativeSymbiote);
        let credential = dispatch.contract().profile().credential.clone();
        let scope = symbiote_context::LeaseScope {
            dispatch_id: dispatch.id().clone(),
            project_id: store.task(&task).unwrap().project_id().clone(),
            role_id: dispatch.contract().binding().role_id.clone(),
            profile_id: dispatch.contract().profile().id.clone(),
            host_id: symbiote_domain::HostId::new("host-lease-e2e").unwrap(),
        };
        // No broker: the typed daemon gate.
        let empty = WorkerTransports::default();
        assert!(matches!(
            empty.resolve_leases(
                &credential,
                symbiote_context::BrokerRequest {
                    scope: &scope,
                    profile_credential_refs: std::slice::from_ref(&credential),
                    use_credential_granted: true,
                    at: Timestamp(60),
                },
            ),
            Err(RunnerError::NoCredentialBroker)
        ));
        // Broker without a registration: UnknownReference.
        let bare_broker = WorkerTransports::default().with_credential_broker(std::rc::Rc::new(
            std::cell::RefCell::new(symbiote_context::CredentialBroker::new()),
        ));
        assert!(matches!(
            bare_broker.resolve_leases(
                &credential,
                symbiote_context::BrokerRequest {
                    scope: &scope,
                    profile_credential_refs: std::slice::from_ref(&credential),
                    use_credential_granted: true,
                    at: Timestamp(60),
                },
            ),
            Err(RunnerError::CredentialRefused("unknown_reference"))
        ));
        // Registered under the dispatch's project: the lease issues and
        // its scope matches the contract verbatim.
        let broker = std::rc::Rc::new(std::cell::RefCell::new(
            symbiote_context::CredentialBroker::new(),
        ));
        broker
            .borrow_mut()
            .register(
                credential.clone(),
                scope.project_id.clone(),
                "OPENAI_API_KEY".into(),
                b"sk-fixture-lease-e2e".to_vec(),
            )
            .unwrap();
        let configured = WorkerTransports::default().with_credential_broker(broker.clone());
        let lease = configured
            .resolve_leases(
                &credential,
                symbiote_context::BrokerRequest {
                    scope: &scope,
                    profile_credential_refs: std::slice::from_ref(&credential),
                    use_credential_granted: dispatch
                        .contract()
                        .binding()
                        .access
                        .grants
                        .contains(&symbiote_domain::Permission::UseCredential),
                    at: Timestamp(60),
                },
            )
            .unwrap()
            .expect("the fixture grants UseCredential");
        assert_eq!(lease.scope, scope);
        let (label, value) = lease.materialize(Timestamp(61)).unwrap();
        assert_eq!(label, "OPENAI_API_KEY");
        assert_eq!(value, "sk-fixture-lease-e2e");
        // A cross-project dispatch scope for the SAME reference: refused.
        let foreign = symbiote_context::LeaseScope {
            project_id: symbiote_domain::ProjectId::new("project-foreign").unwrap(),
            ..scope.clone()
        };
        assert!(matches!(
            configured.resolve_leases(
                &credential,
                symbiote_context::BrokerRequest {
                    scope: &foreign,
                    profile_credential_refs: std::slice::from_ref(&credential),
                    use_credential_granted: true,
                    at: Timestamp(60),
                },
            ),
            Err(RunnerError::CredentialRefused("cross_project_denied"))
        ));
        // Revocation flips the next resolution to the sticky refusal.
        broker.borrow_mut().revoke(&credential);
        assert!(matches!(
            configured.resolve_leases(
                &credential,
                symbiote_context::BrokerRequest {
                    scope: &scope,
                    profile_credential_refs: std::slice::from_ref(&credential),
                    use_credential_granted: true,
                    at: Timestamp(60),
                },
            ),
            Err(RunnerError::CredentialRefused("revoked"))
        ));
        assert_eq!(store.task(&task).unwrap().state(), &TaskState::Running);
    }

    #[test]
    fn elevation_ask_id_stays_within_the_command_id_bound() {
        let dispatch_id = symbiote_domain::DispatchId::new("d".repeat(128)).unwrap();
        let id = elevation_ask_command_id(
            dispatch_id.as_str(),
            &Permission::ExecuteProcess,
            Timestamp(u64::MAX),
        )
        .unwrap();
        assert!(id.as_str().len() <= 128);
    }

    #[test]
    fn native_credential_refusal_files_an_ask_and_never_licenses() {
        let (mut store, task, dispatch) =
            store_with_running_task("native-ask", RuntimeKind::NativeSymbiote);
        let credential = dispatch.contract().profile().credential.clone();
        let scope = symbiote_context::LeaseScope {
            dispatch_id: dispatch.id().clone(),
            project_id: store.task(&task).unwrap().project_id().clone(),
            role_id: dispatch.contract().binding().role_id.clone(),
            profile_id: dispatch.contract().profile().id.clone(),
            host_id: HostId::new("host-native-ask").unwrap(),
        };
        let broker = std::rc::Rc::new(std::cell::RefCell::new(
            symbiote_context::CredentialBroker::new(),
        ));
        broker
            .borrow_mut()
            .register(
                credential.clone(),
                scope.project_id.clone(),
                "OPENAI_API_KEY".into(),
                b"sk-fixture-native-ask".to_vec(),
            )
            .unwrap();
        let transports = WorkerTransports::default().with_credential_broker(broker);
        let at = Timestamp(60);
        assert!(matches!(
            transports.resolve_native_credential(
                &mut store,
                &task,
                &dispatch,
                &credential,
                symbiote_context::BrokerRequest {
                    scope: &scope,
                    profile_credential_refs: std::slice::from_ref(&credential),
                    use_credential_granted: false,
                    at,
                },
                at,
            ),
            Err(RunnerError::CredentialRefused("use_credential_not_granted"))
        ));
        assert!(
            !store
                .active_elevation(dispatch.id(), &Permission::UseCredential, Timestamp(61))
                .unwrap()
        );
        let events = store
            .events(store.task(&task).unwrap().project_id(), 0, 256)
            .unwrap();
        assert!(
            events.events.iter().any(|event| matches!(
                event.payload,
                symbiote_store::EventPayload::ElevationRequested { ref ask }
                    if ask.permission == Permission::UseCredential
                        && ask.dispatch_id == *dispatch.id()
            )),
            "native credential refusal must journal an elevation ask"
        );
        assert_eq!(store.task(&task).unwrap().state(), &TaskState::Running);
    }

    #[test]
    fn external_harness_refusal_files_an_ask_and_never_licenses() {
        let (mut store, task, dispatch) =
            store_with_running_task("external-ask", RuntimeKind::ExternalHarness);
        let mut transport = fixture::ScriptedCodex::new(
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
                fixture::codex_completed("completed"),
            ],
            vec![symbiote_external_agent::ServerRequest {
                id: serde_json::json!(7),
                method: "item/commandExecution/requestApproval".into(),
                params: serde_json::json!({}),
            }],
        );
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
        assert!(
            !store
                .active_elevation(dispatch.id(), &Permission::ExecuteProcess, Timestamp(61))
                .unwrap()
        );
        let events = store
            .events(store.task(&task).unwrap().project_id(), 0, 256)
            .unwrap();
        assert!(
            events.events.iter().any(|event| matches!(
                event.payload,
                symbiote_store::EventPayload::ElevationRequested { ref ask }
                    if ask.permission == Permission::ExecuteProcess
                        && ask.dispatch_id == *dispatch.id()
            )),
            "external harness refusal must journal an elevation ask"
        );
        assert_eq!(
            store.task(&task).unwrap().state(),
            &TaskState::CompletionRequested
        );
    }

    #[test]
    fn a_decided_execute_process_lease_licenses_the_shell_path_alone() {
        let (mut store, task, dispatch) = store_with_running_task_worker_grants(
            "shell-elevate",
            RuntimeKind::NativeSymbiote,
            &[
                Permission::ReadRoot,
                Permission::MutateStream,
                Permission::UseCredential,
            ],
        );
        let at = Timestamp(60);
        // The binding base carries no ExecuteProcess: the shell path's
        // policy refuses to run tools before any elevation.
        let base = resolve_shell_access(&store, &dispatch, at).unwrap();
        assert!(!base.grants.contains(&Permission::ExecuteProcess));
        assert!(base.grants.contains(&Permission::ReadRoot));
        // A filed ASK is evidence only: it must not widen the policy.
        file_elevation_ask(
            &mut store,
            &task,
            &dispatch,
            Permission::ExecuteProcess,
            "the run needs the consented build tool",
            at,
        )
        .unwrap();
        assert!(
            !resolve_shell_access(&store, &dispatch, at)
                .unwrap()
                .grants
                .contains(&Permission::ExecuteProcess)
        );
        // The owner decides; the lease widens EXACTLY ExecuteProcess.
        let project = store.task(&task).unwrap().project_id().clone();
        store
            .decide_elevation(
                CommandId::new("elevate-shell-elevate").unwrap(),
                symbiote_domain::ElevationLease {
                    id: CommandId::new("elevate-shell-elevate").unwrap(),
                    project_id: project.clone(),
                    task_id: task.clone(),
                    dispatch_id: dispatch.id().clone(),
                    permission: Permission::ExecuteProcess,
                    reason: "the run needs the consented build tool".into(),
                    approved: true,
                    approved_by: fixture::user(),
                    decided_at: Timestamp(60),
                    expires_at: Timestamp(60 + 300_000),
                    revoked_at: None,
                },
            )
            .unwrap();
        let widened = resolve_shell_access(&store, &dispatch, at).unwrap();
        assert!(widened.grants.contains(&Permission::ExecuteProcess));
        assert_eq!(widened.grants.len(), base.grants.len() + 1);
        assert_eq!(widened.roots, base.roots);
        assert_eq!(widened.policy_revision, base.policy_revision);
        // Expiry is the automatic revocation, checked at the use site.
        assert!(
            !resolve_shell_access(&store, &dispatch, Timestamp(60 + 300_000 + 1))
                .unwrap()
                .grants
                .contains(&Permission::ExecuteProcess)
        );
        // Sticky manual revocation kills the lease inside its window.
        store
            .revoke_elevation(
                CommandId::new("revoke-shell-elevate").unwrap(),
                &project,
                &CommandId::new("elevate-shell-elevate").unwrap(),
                &fixture::user(),
                Timestamp(70),
            )
            .unwrap();
        assert!(
            !resolve_shell_access(&store, &dispatch, Timestamp(71))
                .unwrap()
                .grants
                .contains(&Permission::ExecuteProcess)
        );
    }

    #[test]
    fn the_store_decides_only_permissions_with_enforcement_consumers() {
        // A compilable dispatch's binding always carries MutateStream
        // (Dispatch::compile requires it), so this is the narrowest
        // executable binding without ExecuteProcess.
        let (mut store, task, dispatch) = store_with_running_task_worker_grants(
            "elevate-allowlist",
            RuntimeKind::NativeSymbiote,
            &[
                Permission::ReadRoot,
                Permission::MutateStream,
                Permission::UseCredential,
            ],
        );
        let project = store.task(&task).unwrap().project_id().clone();
        let lease = |id: &str, permission: Permission| symbiote_domain::ElevationLease {
            id: CommandId::new(id).unwrap(),
            project_id: project.clone(),
            task_id: task.clone(),
            dispatch_id: dispatch.id().clone(),
            permission,
            reason: "decide-scope pin".into(),
            approved: true,
            approved_by: fixture::user(),
            decided_at: Timestamp(60),
            expires_at: Timestamp(60 + 300_000),
            revoked_at: None,
        };
        // Network has no enforcement consumer and the binding does not
        // carry it, so the consumer allowlist (the first decide check) is
        // what refuses — with the allowlist widened this would instead be
        // an ElevationCeiling refusal (the fixture ceiling lacks Network).
        assert!(matches!(
            store.decide_elevation(
                CommandId::new("elevate-allowlist-network").unwrap(),
                lease("elevate-allowlist-network", Permission::Network),
            ),
            Err(symbiote_store::StoreError::InvalidElevation)
        ));
        // MutateStream is refused too; for a Running dispatch its binding
        // always carries MutateStream, so the never-re-grant check shadows
        // the allowlist here — either way deciding it refuses.
        assert!(matches!(
            store.decide_elevation(
                CommandId::new("elevate-allowlist-mutate").unwrap(),
                lease("elevate-allowlist-mutate", Permission::MutateStream),
            ),
            Err(symbiote_store::StoreError::InvalidElevation)
        ));
        // ExecuteProcess has the native shell-tool consumer (#269): the
        // decision licenses, the ask alone never did.
        store
            .decide_elevation(
                CommandId::new("elevate-allowlist-exec").unwrap(),
                lease("elevate-allowlist-exec", Permission::ExecuteProcess),
            )
            .unwrap();
        assert!(
            store
                .active_elevation(dispatch.id(), &Permission::ExecuteProcess, Timestamp(61))
                .unwrap()
        );
    }

    /// The runtime-aware decide rule (#269): EVERY consumer gate lives on
    /// the native loop, so a decision on an EXTERNAL_HARNESS dispatch
    /// would journal an approval that licenses nothing — the harness
    /// never consults active_elevation (its escalations are refused by
    /// the driver regardless of any lease). The binding lacks both
    /// permissions here, so the refusals are unambiguously the runtime
    /// rule, not a re-grant accident.
    #[test]
    fn a_decision_on_an_external_dispatch_refuses_its_lease_entirely() {
        let (mut store, task, dispatch) = store_with_running_task_worker_grants(
            "elevate-external",
            RuntimeKind::ExternalHarness,
            &[Permission::ReadRoot, Permission::MutateStream],
        );
        let project = store.task(&task).unwrap().project_id().clone();
        let lease = |id: &str, permission: Permission| symbiote_domain::ElevationLease {
            id: CommandId::new(id).unwrap(),
            project_id: project.clone(),
            task_id: task.clone(),
            dispatch_id: dispatch.id().clone(),
            permission,
            reason: "runtime-aware decide pin".into(),
            approved: true,
            approved_by: fixture::user(),
            decided_at: Timestamp(60),
            expires_at: Timestamp(60 + 300_000),
            revoked_at: None,
        };
        for (id, permission) in [
            ("elevate-ext-exec", Permission::ExecuteProcess),
            ("elevate-ext-cred", Permission::UseCredential),
        ] {
            assert!(
                matches!(
                    store.decide_elevation(CommandId::new(id).unwrap(), lease(id, permission)),
                    Err(symbiote_store::StoreError::InvalidElevation)
                ),
                "a decision on an EXTERNAL_HARNESS dispatch must refuse: {id}"
            );
        }
        // Nothing was licensed: no active elevation on either permission.
        assert!(
            !store
                .active_elevation(dispatch.id(), &Permission::ExecuteProcess, Timestamp(61))
                .unwrap()
        );
        assert!(
            !store
                .active_elevation(dispatch.id(), &Permission::UseCredential, Timestamp(61))
                .unwrap()
        );
    }

    #[test]
    fn a_binding_execute_process_grant_needs_no_lease() {
        let (store, _task, dispatch) =
            store_with_running_task("shell-pregranted", RuntimeKind::NativeSymbiote);
        assert!(
            resolve_shell_access(&store, &dispatch, Timestamp(60))
                .unwrap()
                .grants
                .contains(&Permission::ExecuteProcess)
        );
        assert!(
            !store
                .active_elevation(dispatch.id(), &Permission::ExecuteProcess, Timestamp(60))
                .unwrap()
        );
    }

    /// Test fixtures for the store wiring. Mirrors the store's own
    /// preparation_tests fixture: a full project/team/binding/provider
    /// composition whose profile carries the requested runtime kind.
    mod fixture {
        use super::*;
        use std::collections::BTreeMap;
        use std::path::Path;
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

        pub fn full_fixture_with_placement(
            store: &mut Store,
            tag: &str,
            runtime: RuntimeKind,
            host_placement: Option<(&HostId, &Path)>,
            base_override: Option<&str>,
            required_tools: &[&str],
        ) -> (ProjectId, TaskId) {
            full_fixture_with_worker_grants(
                store,
                tag,
                runtime,
                host_placement,
                base_override,
                required_tools,
                &[
                    Permission::ReadRoot,
                    Permission::ExecuteProcess,
                    Permission::MutateStream,
                    Permission::UseCredential,
                ],
            )
        }

        pub fn full_fixture_with_worker_grants(
            store: &mut Store,
            tag: &str,
            runtime: RuntimeKind,
            host_placement: Option<(&HostId, &Path)>,
            base_override: Option<&str>,
            required_tools: &[&str],
            worker_grants: &[Permission],
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
            let mut host_paths = BTreeMap::new();
            if let Some((host, path)) = host_placement {
                host_paths.insert(host.clone(), path.display().to_string());
            }
            let root = Root {
                id: root_id.clone(),
                project_id: project_id.clone(),
                revision: Revision(0),
                repository: None,
                host_paths,
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
            // UseCredential is in the base grants so the credential-lease
            // path (#217) can pass for profiles that reference a credential
            // (the daemon gates leases on it).
            let access = AccessSnapshot {
                project_id: project.id.clone(),
                roots: project.roots.clone(),
                grants: BTreeSet::from([
                    Permission::ReadRoot,
                    Permission::ExecuteProcess,
                    Permission::UseCredential,
                ]),
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
            // The Role policy keeps the Team's full general-executor
            // access: team validation requires a GeneralExecution member
            // carrying ExecuteProcess. Least privilege narrows at the
            // BINDING, which may grant fewer permissions than its role —
            // exactly the state an elevation lease elevates from.
            let mut worker_access = access.clone();
            worker_access.grants = BTreeSet::from([
                Permission::ReadRoot,
                Permission::ExecuteProcess,
                Permission::MutateStream,
                Permission::UseCredential,
            ]);
            let mut binding_access = access.clone();
            binding_access.grants = worker_grants.iter().cloned().collect();
            let worker_access_for_binding = binding_access;
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
                // The primary scope's access must equal the binding's
                // declared access (BindingConfiguration validation) — the
                // narrowed least-privilege set, not the role's.
                access: worker_access_for_binding.clone(),
                tools: required_tools
                    .iter()
                    .map(|tool| (*tool).to_owned())
                    .collect(),
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
                    required_tools: required_tools
                        .iter()
                        .map(|tool| (*tool).to_owned())
                        .collect(),
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
                        Control::Credentials,
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
                requirements: vec![format!("requirement-of-{tag}")],
                constraints: vec![format!("constraint-of-{tag}")],
                risks: vec![],
                acceptance: vec![format!("acceptance-of-{tag}")],
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
            // The stream's worktree/branch must be what the HOST policy
            // seed derives — the same derivation provision re-checks.
            let derived = symbiote_worktrees::Derived::derive(symbiote_worktrees::DeriveInputs {
                project_id: &project.id,
                root_id: &root_id,
                stream_id: &ChangeStreamId::new(format!("stream-{tag}")).unwrap(),
                seed: &symbiote_worktrees_policy_seed(
                    &ChangeStreamId::new(format!("stream-{tag}")).unwrap(),
                )
                .unwrap(),
            })
            .unwrap();
            let stream = ChangeStream::new(NewChangeStream {
                id: ChangeStreamId::new(format!("stream-{tag}")).unwrap(),
                project_id: project.id.clone(),
                root_id: root_id.clone(),
                tasks: BTreeSet::from([task.id().clone()]),
                originating_chat: ChatId::new(format!("chat-{tag}")).unwrap(),
                worktree: derived.worktree_id,
                branch: derived.branch,
                lineage: StreamLineage::Independent,
                base: base_override
                    .map(|value| CommitSha::new(value.to_owned()).unwrap())
                    .unwrap_or_else(|| sha('a')),
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
                    Control::Credentials,
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
            // The domain Role is reconstructed here; its fidelity is pinned
            // by the start step below: the store's start_prepared_task
            // re-reads the role from the roles table and compiles its own
            // dispatch, and the test asserts the store's started dispatch
            // id equals this fixture's dispatch id — a mismatched role
            // (lineage mismatch) would fail that start loudly.
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
                    minimum_enforcement: &std::collections::BTreeMap::new(),
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

        pub fn codex_transport_with_status(
            status: &str,
        ) -> impl symbiote_external_agent::CodexTransport {
            ScriptedCodex::new(
                vec![
                    Ok(serde_json::json!({"userAgent": format!(
                        "symbiote/{} (Linux)",
                        symbiote_runtime_discovery::codex::CODEX_VERSION
                    )})),
                    Ok(serde_json::json!({"thread": {"id": "thr-fixture"}})),
                    Ok(serde_json::json!({"turn": {"id": "turn-fixture"}})),
                ],
                vec![codex_completed(status)],
                Vec::new(),
            )
        }

        pub struct EchoFactory {
            pub text: String,
        }
        impl super::NativeTransportFactory for EchoFactory {
            fn build(
                &mut self,
            ) -> Result<Box<dyn symbiote_native_agent::InferenceTransport>, &'static str>
            {
                Ok(Box::new(EchoTransport {
                    text: self.text.clone(),
                }))
            }
        }

        pub struct ScriptedFactory;
        impl super::ExternalTransportFactory for ScriptedFactory {
            fn build(
                &mut self,
            ) -> Result<Box<dyn symbiote_external_agent::CodexTransport>, &'static str>
            {
                Ok(Box::new(ScriptedCodex::new(
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
                )))
            }
        }

        /// A native "provider" that proposes one tool call on the first turn
        /// and completes on the second, asserting the tool result (or the
        /// constant refusal phrase) entered the conversation before the
        /// final report.
        pub struct ToolProposalTransport {
            final_text: String,
            tool_calls: Vec<symbiote_runtime_sdk::provider::ProviderToolCall>,
            expected_result_substring: &'static str,
            proposed: bool,
        }
        impl ToolProposalTransport {
            pub fn new(
                final_text: &str,
                tool_calls: Vec<symbiote_runtime_sdk::provider::ProviderToolCall>,
            ) -> Self {
                Self {
                    final_text: final_text.to_owned(),
                    expected_result_substring: "tool shell result: ",
                    tool_calls,
                    proposed: false,
                }
            }
        }
        impl symbiote_native_agent::InferenceTransport for ToolProposalTransport {
            fn request(
                &mut self,
                request: &symbiote_runtime_sdk::provider::ProviderRequest,
                _model: &symbiote_runtime_sdk::provider::ModelDescriptor,
            ) -> Result<
                symbiote_runtime_sdk::provider::ProviderResponse,
                symbiote_runtime_sdk::provider::ProviderError,
            > {
                use symbiote_runtime_sdk::provider::{
                    FinishReason, PROVIDER_CONTRACT_VERSION, ProviderResponse,
                };
                if !self.proposed {
                    self.proposed = true;
                    return Ok(ProviderResponse {
                        schema_version: PROVIDER_CONTRACT_VERSION,
                        request_id: request.request_id.clone(),
                        provider_id: request.provider_id.clone(),
                        model_id: request.model_id.clone(),
                        text: "proposing the tool".into(),
                        tool_calls: self.tool_calls.clone(),
                        finish_reason: FinishReason::ToolCalls,
                        usage: symbiote_runtime_sdk::provider::TokenUsage::Known {
                            input_tokens: 1,
                            output_tokens: 1,
                            cached_input_tokens: None,
                            reasoning_tokens: None,
                        },
                    });
                }
                // The tool result (or refusal) must be in the conversation
                // before the model can finish: the amnesia fix under test.
                assert!(
                    request.messages.iter().any(|m| m.content.iter().any(
                        |p| matches!(p, symbiote_runtime_sdk::provider::InputPart::Text { text }
                            if text.starts_with(self.expected_result_substring))
                    )),
                    "the tool result must enter the conversation"
                );
                Ok(ProviderResponse {
                    schema_version: PROVIDER_CONTRACT_VERSION,
                    request_id: request.request_id.clone(),
                    provider_id: request.provider_id.clone(),
                    model_id: request.model_id.clone(),
                    text: self.final_text.clone(),
                    tool_calls: Vec::new(),
                    finish_reason: FinishReason::Stop,
                    usage: symbiote_runtime_sdk::provider::TokenUsage::Known {
                        input_tokens: 1,
                        output_tokens: 1,
                        cached_input_tokens: None,
                        reasoning_tokens: None,
                    },
                })
            }
        }

        /// A scripted shell executor: returns a fixed output and exit code.
        pub struct FixedOutputExecutor {
            pub output: Vec<u8>,
            pub code: Option<i32>,
        }
        impl symbiote_native_agent::tools::ShellToolExecutor for FixedOutputExecutor {
            fn run_shell(
                &mut self,
                _invocation: &symbiote_native_agent::tools::ShellInvocation,
                _worktree: &std::path::Path,
            ) -> Result<(Vec<u8>, Option<i32>), symbiote_native_agent::tools::ToolExecError>
            {
                Ok((self.output.clone(), self.code))
            }
        }

        /// A shell executor whose consent authority always refuses.
        pub struct RefusingExecutor;
        impl symbiote_native_agent::tools::ShellToolExecutor for RefusingExecutor {
            fn run_shell(
                &mut self,
                _invocation: &symbiote_native_agent::tools::ShellInvocation,
                _worktree: &std::path::Path,
            ) -> Result<(Vec<u8>, Option<i32>), symbiote_native_agent::tools::ToolExecError>
            {
                Err(symbiote_native_agent::tools::ToolExecError::Refused)
            }
        }

        /// A shell executor factory that refuses to build — the operator
        /// config error path, before any loop runs.
        pub struct RefusingShellFactory;
        impl super::ShellExecutorFactory for RefusingShellFactory {
            fn build(
                &mut self,
                _inputs: super::ShellExecutorInputs<'_>,
            ) -> Result<Box<dyn symbiote_native_agent::tools::ShellToolExecutor>, &'static str>
            {
                Err("operator refused")
            }
        }

        /// A native "provider" that reports a mid-turn cancellation.
        pub struct CancelledTransport;
        impl symbiote_native_agent::InferenceTransport for CancelledTransport {
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
                    text: "I'll continue implementing...".into(),
                    tool_calls: Vec::new(),
                    finish_reason: symbiote_runtime_sdk::provider::FinishReason::Cancelled,
                    usage: symbiote_runtime_sdk::provider::TokenUsage::Known {
                        input_tokens: 1,
                        output_tokens: 1,
                        cached_input_tokens: None,
                        reasoning_tokens: None,
                    },
                })
            }
        }

        pub fn codex_completed(status: &str) -> serde_json::Value {
            serde_json::json!({
                "method": "turn/completed",
                "params": {"threadId": "thr-fixture", "turnId": "turn-fixture",
                    "turn": {"id": "turn-fixture", "status": status, "items": []}}
            })
        }

        /// Minimal scripted Codex transport for runner wiring tests.
        pub struct ScriptedCodex {
            responses: Vec<Result<serde_json::Value, symbiote_external_agent::DriverError>>,
            notifications: VecDeque<Option<serde_json::Value>>,
            server_requests: VecDeque<symbiote_external_agent::ServerRequest>,
            cursor: usize,
        }
        impl ScriptedCodex {
            pub fn new(
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

/// The operator-configured fixture model transport (#54): a SCRIPTED model
/// turn for demonstrating the dispatch→provisioning→sandbox→completion
/// mechanics without spending. Turn one proposes the configured tool call
/// (or none); turn two completes with the configured report text. This is
/// not a model and must never be presented as one — a live Responses-API
/// transport remains gated on explicit user authorization for credentials
/// and billing.
pub struct FixtureNativeTransport {
    echo_text: String,
    tool: Option<FixtureToolCall>,
    proposed: bool,
}

#[derive(Clone, Debug)]
pub struct FixtureToolCall {
    pub call_id: String,
    pub arguments: serde_json::Value,
}

impl FixtureNativeTransport {
    pub fn new(echo_text: String, tool: Option<FixtureToolCall>) -> Self {
        Self {
            echo_text,
            tool,
            proposed: false,
        }
    }
}

impl symbiote_native_agent::InferenceTransport for FixtureNativeTransport {
    fn request(
        &mut self,
        request: &symbiote_runtime_sdk::provider::ProviderRequest,
        _model: &symbiote_runtime_sdk::provider::ModelDescriptor,
    ) -> Result<
        symbiote_runtime_sdk::provider::ProviderResponse,
        symbiote_runtime_sdk::provider::ProviderError,
    > {
        use symbiote_runtime_sdk::provider::{
            FinishReason, PROVIDER_CONTRACT_VERSION, ProviderResponse,
        };
        let (text, tool_calls, finish) = if self.proposed {
            (self.echo_text.clone(), Vec::new(), FinishReason::Stop)
        } else {
            self.proposed = true;
            let mut calls = Vec::new();
            if let Some(tool) = &self.tool {
                let call_id = symbiote_domain::RequestId::new(&tool.call_id)
                    .map_err(|_| symbiote_runtime_sdk::provider::ProviderError::Unavailable)?;
                calls.push(symbiote_runtime_sdk::provider::ProviderToolCall {
                    call_id,
                    name: "shell".into(),
                    arguments: tool.arguments.clone(),
                });
            }
            (
                "proposing the configured tool".into(),
                calls,
                FinishReason::ToolCalls,
            )
        };
        Ok(ProviderResponse {
            schema_version: PROVIDER_CONTRACT_VERSION,
            request_id: request.request_id.clone(),
            provider_id: request.provider_id.clone(),
            model_id: request.model_id.clone(),
            text,
            tool_calls,
            finish_reason: finish,
            usage: symbiote_runtime_sdk::provider::TokenUsage::Known {
                input_tokens: 1,
                output_tokens: 1,
                cached_input_tokens: None,
                reasoning_tokens: None,
            },
        })
    }
}

/// Builds one fixture transport per run (the factory contract).
pub struct FixtureNativeFactory {
    pub echo_text: String,
    pub tool: Option<FixtureToolCall>,
}
impl NativeTransportFactory for FixtureNativeFactory {
    fn build(
        &mut self,
    ) -> Result<Box<dyn symbiote_native_agent::InferenceTransport>, &'static str> {
        Ok(Box::new(FixtureNativeTransport::new(
            self.echo_text.clone(),
            self.tool.clone(),
        )))
    }
}

/// The operator-configured fixture harness transport (#54/#269): a SCRIPTED
/// Codex App-Server conversation — version/thread/turn responses, one agent
/// message, one clean turn completion — so external runs can be
/// demonstrated without the pinned binary, a model turn, or any spending.
/// This is not Codex and must never be presented as one: every harness
/// escalation is still refused (there are none scripted), and a live Codex
/// turn remains gated on explicit user authorization for credentials and
/// billing.
pub struct ScriptedCodexTransport {
    responses:
        std::collections::VecDeque<Result<serde_json::Value, symbiote_external_agent::DriverError>>,
    notifications: std::collections::VecDeque<Option<serde_json::Value>>,
}

impl ScriptedCodexTransport {
    pub fn new(thread_id: &str, agent_message: &str) -> Self {
        let turn_id = format!("turn-{thread_id}");
        Self {
            responses: std::collections::VecDeque::from([
                Ok(serde_json::json!({"userAgent": format!(
                    "symbiote/{} (Linux)",
                    symbiote_runtime_discovery::codex::CODEX_VERSION
                )})),
                Ok(serde_json::json!({"thread": {"id": thread_id}})),
                Ok(serde_json::json!({"turn": {"id": turn_id}})),
            ]),
            notifications: std::collections::VecDeque::from([
                Some(serde_json::json!({
                    "method": "item/completed",
                    "params": {"threadId": thread_id, "turnId": turn_id,
                        "item": {"type": "agentMessage", "id": "i1",
                            "text": agent_message}}
                })),
                Some(serde_json::json!({
                    "method": "turn/completed",
                    "params": {"threadId": thread_id, "turnId": turn_id,
                        "turn": {"id": turn_id, "status": "completed", "items": []}}
                })),
                None,
            ]),
        }
    }
}

impl symbiote_external_agent::CodexTransport for ScriptedCodexTransport {
    fn call(
        &mut self,
        _method: &str,
        _params: &serde_json::Value,
    ) -> Result<serde_json::Value, symbiote_external_agent::DriverError> {
        self.responses
            .pop_front()
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
    ) -> Result<Option<serde_json::Value>, symbiote_external_agent::DriverError> {
        Ok(self.notifications.pop_front().flatten())
    }
    fn recv_server_request(
        &mut self,
    ) -> Result<Option<symbiote_external_agent::ServerRequest>, symbiote_external_agent::DriverError>
    {
        // No escalations are scripted: the fixture conversation stays
        // inside the binding, so the run completes with nothing refused.
        Ok(None)
    }
    fn refuse_server_request(
        &mut self,
        _request: &symbiote_external_agent::ServerRequest,
        _decision: symbiote_external_agent::ApprovalDecision,
    ) -> Result<(), symbiote_external_agent::DriverError> {
        Ok(())
    }
}

/// Builds one fixture harness transport per run (the factory contract).
pub struct FixtureExternalFactory {
    pub thread_id: String,
    pub agent_message: String,
}
impl ExternalTransportFactory for FixtureExternalFactory {
    fn build(&mut self) -> Result<Box<dyn symbiote_external_agent::CodexTransport>, &'static str> {
        Ok(Box::new(ScriptedCodexTransport::new(
            &self.thread_id.clone(),
            &self.agent_message.clone(),
        )))
    }
}

/// Builds the production sandboxed shell executor per run from the
/// operator's launch configuration and the program-allowlist consent
/// authority (#466 allowlist flow).
pub struct SandboxShellExecutorFactory {
    pub launcher_path: std::path::PathBuf,
    pub protected_paths: Vec<std::path::PathBuf>,
    pub allowed_programs: Vec<String>,
}
impl ShellExecutorFactory for SandboxShellExecutorFactory {
    fn build(
        &mut self,
        inputs: ShellExecutorInputs<'_>,
    ) -> Result<Box<dyn symbiote_native_agent::tools::ShellToolExecutor>, &'static str> {
        Ok(Box::new(crate::shell_executor::SandboxShellExecutor {
            launcher_path: self.launcher_path.clone(),
            protected_paths: self.protected_paths.clone(),
            authority: Box::new(crate::shell_executor::ProgramAllowlistAuthority {
                allowed_programs: self.allowed_programs.clone(),
            }),
            root_id: inputs.root_id.clone(),
            host: inputs.host.clone(),
            project_id: inputs.project_id.clone(),
            role_id: inputs.role_id.clone(),
            profile_id: inputs.profile_id.clone(),
            access: inputs.access.clone(),
            user_id: inputs.user_id.clone(),
        }))
    }
}

/// Assembles the operator-provisioned transports from the configuration
/// file (#54). Each element maps one operator authority onto the seam it
/// owns: reservation base (#211), the explicitly-labeled fixture model
/// transport and fixture harness transport (demonstration without
/// spending, native and external), the #217 credential broker
/// registrations, and the #466 program-allowlist consent authority behind
/// the #507 sandbox shell executor. Values and paths stay in operator
/// state — nothing here is client-reachable configuration.
pub fn assemble_operator_transports(
    config: crate::operator::OperatorConfig,
) -> Result<WorkerTransports, &'static str> {
    let mut transports =
        WorkerTransports::default().with_reservation_base(config.reservation_base.clone());
    if let Some(fixture) = &config.native_fixture {
        transports = transports.with_native(Box::new(FixtureNativeFactory {
            echo_text: fixture.echo_text.clone(),
            tool: fixture.tool.as_ref().map(|tool| FixtureToolCall {
                call_id: tool.call_id.clone(),
                arguments: tool.arguments.clone(),
            }),
        }));
    }
    if let Some(fixture) = &config.external_fixture {
        transports = transports.with_external(Box::new(FixtureExternalFactory {
            thread_id: fixture.thread_id.clone(),
            agent_message: fixture.agent_message.clone(),
        }));
    }
    if !config.credential_broker.is_empty() {
        let broker = std::rc::Rc::new(std::cell::RefCell::new(
            symbiote_context::CredentialBroker::new(),
        ));
        for registration in &config.credential_broker {
            broker
                .borrow_mut()
                .register(
                    symbiote_domain::CredentialReferenceId::new(&registration.reference)
                        .map_err(|_| "operator credential reference")?,
                    symbiote_domain::ProjectId::new(&registration.project)
                        .map_err(|_| "operator credential project")?,
                    registration.environment.clone(),
                    registration.value.clone().into_bytes(),
                )
                .map_err(|_| "operator credential registration refused")?;
        }
        transports = transports.with_credential_broker(broker);
    }
    if let Some(shell) = &config.shell_executor {
        transports = transports.with_shell_executor(Box::new(SandboxShellExecutorFactory {
            launcher_path: shell.launcher_path.clone(),
            protected_paths: shell.protected_paths.clone(),
            allowed_programs: shell.allowed_programs.clone(),
        }));
    }
    Ok(transports)
}
