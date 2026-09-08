//! Transport-neutral, bounded v1 request boundary. Principals never come from JSON.
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use symbiote_domain::*;
pub use symbiote_trust::{ResourceConsent, ResourceSnapshot};

pub const MAX_REQUEST_BYTES: usize = 64 * 1024;
pub const MAX_RESPONSE_BYTES: usize = 1024 * 1024;
pub const MAX_PAGE_SIZE: u32 = 100;
pub const CURRENT_VERSION: ProtocolVersion = ProtocolVersion {
    major: 1,
    minor: 10,
};

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct ProtocolVersion {
    pub major: u16,
    pub minor: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    InvalidRequest,
    FailedPrecondition,
    RequestTooLarge,
    UnsupportedVersion,
    PermissionDenied,
    NotFound,
    Conflict,
    StaleRevision,
    IdempotencyConflict,
    InvalidCursor,
    ResourceExhausted,
    Unavailable,
    Internal,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProtocolError {
    pub code: ErrorCode,
    pub message: String,
}
impl ProtocolError {
    /// Messages are stable and contain no input values, secrets, or internal paths.
    pub fn new(code: ErrorCode) -> Self {
        let message = match code {
            ErrorCode::InvalidRequest => "invalid request shape or field value",
            ErrorCode::FailedPrecondition => "preparation composition refused by recorded state",
            ErrorCode::RequestTooLarge => "request exceeds 65536 bytes",
            ErrorCode::UnsupportedVersion => "no supported protocol version",
            ErrorCode::PermissionDenied => {
                "operation is not permitted for this principal and project"
            }
            ErrorCode::NotFound => "resource not found",
            ErrorCode::Conflict => "resource conflict",
            ErrorCode::StaleRevision => "expected revision is stale",
            ErrorCode::IdempotencyConflict => "command identity was reused for different intent",
            ErrorCode::InvalidCursor => "cursor is outside the available journal",
            ErrorCode::ResourceExhausted => "operation exceeds a configured resource bound",
            ErrorCode::Unavailable => "service is temporarily unavailable",
            ErrorCode::Internal => "internal operation failed",
        };
        Self {
            code,
            message: message.into(),
        }
    }
}
impl std::fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for ProtocolError {}
fn invalid() -> ProtocolError {
    ProtocolError::new(ErrorCode::InvalidRequest)
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub version: ProtocolVersion,
    /// A retry may use a new correlation ID; it must retain its command ID.
    pub correlation_id: RequestId,
    /// The durable idempotency key, scoped by the Host to authenticated intent.
    pub command_id: CommandId,
    pub operation: Operation,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Operation {
    ReplaceBinding {
        expected_revision: Option<Revision>,
        configuration: Box<symbiote_workforce::BindingConfiguration>,
    },
    GetBinding {
        project_id: ProjectId,
        binding_id: BindingId,
    },
    GetBindingReadiness {
        project_id: ProjectId,
        binding_id: BindingId,
    },
    ResolveRoute {
        request: symbiote_workforce::RouteRequest,
    },
    RecordRoute {
        request: symbiote_workforce::RouteRequest,
    },
    SetTaskDependencies {
        project_id: ProjectId,
        task_id: TaskId,
        dependencies: Vec<symbiote_domain::TaskDependencyEdge>,
    },
    GetTaskDependencies {
        project_id: ProjectId,
        task_id: TaskId,
    },
    AcquireTaskLease {
        task_id: TaskId,
        dispatch_id: DispatchId,
        host_id: HostId,
        duration_ms: u64,
    },
    ReleaseTaskLease {
        task_id: TaskId,
        dispatch_id: DispatchId,
        fencing_token: u64,
    },
    ExpireStaleLeases {},
    GetSchedulingProjection {},
    ReplaceProviderConnection {
        attribution: ProjectId,
        connection: symbiote_domain::ProviderConnection,
    },
    ReplaceBillingEntitlement {
        attribution: ProjectId,
        entitlement: Box<symbiote_domain::BillingEntitlement>,
    },
    ReplaceModelDescriptor {
        attribution: ProjectId,
        descriptor: Box<symbiote_runtime_sdk::provider::ModelDescriptor>,
    },
    GetProviderConnection {
        provider_id: ProviderConnectionId,
    },
    GetBillingEntitlement {
        entitlement_id: BillingEntitlementId,
    },
    GetModelDescriptor {
        model_id: ModelId,
    },
    PrepareDispatch {
        task_id: TaskId,
    },
    GetDispatchPreparation {
        task_id: TaskId,
    },
    GetRoute {
        project_id: ProjectId,
        work_id: WorkId,
    },
    GetHostPulse {},
    ReplaceTeam {
        expected_revision: Option<Revision>,
        team: Box<TeamConfiguration>,
    },
    GetTeam {
        project_id: ProjectId,
    },
    CreateWork {
        work: WorkSpec,
    },
    ChangeWork {
        project_id: ProjectId,
        id: WorkId,
        expected_revision: Revision,
        edit: WorkEdit,
    },
    GetWork {
        project_id: ProjectId,
        id: WorkId,
    },
    AssignTaskOrigin {
        project_id: ProjectId,
        task_id: TaskId,
        origin: TaskOrigin,
    },
    GetTaskOrigin {
        project_id: ProjectId,
        task_id: TaskId,
    },
    Hello {
        supported_versions: Vec<ProtocolVersion>,
    },
    Health {},
    Shutdown {},
    RegisterProject {
        project: ProjectDraft,
    },
    GetProject {
        project_id: ProjectId,
    },
    CreateTask {
        task: TaskDraft,
    },
    GetTask {
        project_id: ProjectId,
        task_id: TaskId,
    },
    RecordResourceConsent {
        snapshot: ResourceSnapshot,
        expires_at: Timestamp,
    },
    RevokeResourceConsent {
        project_id: ProjectId,
        consent_id: CommandId,
    },
    GetResourceConsent {
        project_id: ProjectId,
        consent_id: CommandId,
    },
    ReadJournal {
        project_id: ProjectId,
        after: JournalCursor,
        limit: u32,
    },
}
impl Operation {
    pub fn project_id(&self) -> Option<&ProjectId> {
        match self {
            Self::ReplaceBinding { configuration, .. } => Some(&configuration.binding.project_id),
            Self::GetBinding { project_id, .. } | Self::GetBindingReadiness { project_id, .. } => {
                Some(project_id)
            }
            Self::ResolveRoute { request } | Self::RecordRoute { request } => {
                Some(&request.project_id)
            }
            Self::SetTaskDependencies { project_id, .. }
            | Self::GetTaskDependencies { project_id, .. } => Some(project_id),
            Self::AcquireTaskLease { .. }
            | Self::ReleaseTaskLease { .. }
            | Self::ExpireStaleLeases {}
            | Self::GetSchedulingProjection {} => None,
            Self::ReplaceProviderConnection { attribution, .. }
            | Self::ReplaceBillingEntitlement { attribution, .. }
            | Self::ReplaceModelDescriptor { attribution, .. } => Some(attribution),
            Self::GetProviderConnection { .. }
            | Self::GetBillingEntitlement { .. }
            | Self::GetModelDescriptor { .. } => None,
            Self::PrepareDispatch { .. } | Self::GetDispatchPreparation { .. } => None,
            Self::GetRoute { project_id, .. } => Some(project_id),
            Self::ReplaceTeam { team, .. } => Some(&team.project_id),
            Self::GetTeam { project_id } => Some(project_id),
            Self::CreateWork { work } => Some(&work.project_id),
            Self::ChangeWork { project_id, .. }
            | Self::GetWork { project_id, .. }
            | Self::AssignTaskOrigin { project_id, .. }
            | Self::GetTaskOrigin { project_id, .. } => Some(project_id),
            Self::RegisterProject { project } => Some(&project.id),
            Self::CreateTask { task } => Some(&task.project_id),
            Self::RecordResourceConsent { snapshot, .. } => Some(&snapshot.project_id),
            Self::GetProject { project_id }
            | Self::RevokeResourceConsent { project_id, .. }
            | Self::GetResourceConsent { project_id, .. }
            | Self::GetTask { project_id, .. }
            | Self::ReadJournal { project_id, .. } => Some(project_id),
            Self::Hello { .. } | Self::Health {} | Self::Shutdown {} | Self::GetHostPulse {} => {
                None
            }
        }
    }
    pub fn is_mutation(&self) -> bool {
        matches!(
            self,
            Self::CreateWork { .. }
                | Self::ReplaceBinding { .. }
                | Self::ReplaceTeam { .. }
                | Self::ChangeWork { .. }
                | Self::AssignTaskOrigin { .. }
                | Self::RecordRoute { .. }
                | Self::SetTaskDependencies { .. }
                | Self::AcquireTaskLease { .. }
                | Self::ReleaseTaskLease { .. }
                | Self::ExpireStaleLeases {}
                | Self::ReplaceProviderConnection { .. }
                | Self::ReplaceBillingEntitlement { .. }
                | Self::ReplaceModelDescriptor { .. }
                | Self::PrepareDispatch { .. }
                | Self::RegisterProject { .. }
                | Self::CreateTask { .. }
                | Self::Shutdown {}
                | Self::RecordResourceConsent { .. }
                | Self::RevokeResourceConsent { .. }
        )
    }
}

impl Request {
    pub fn validate(&self) -> Result<(), ProtocolError> {
        // Hello uses a known envelope while explicitly offering future protocol versions.
        if self.version != CURRENT_VERSION {
            return Err(ProtocolError::new(ErrorCode::UnsupportedVersion));
        }
        match &self.operation {
            Operation::ReplaceBinding {
                configuration,
                expected_revision,
            } => {
                configuration.validate().map_err(|_| invalid())?;
                let revision = match expected_revision {
                    None => 0,
                    Some(revision) => revision.0.checked_add(1).ok_or_else(invalid)?,
                };
                if configuration.binding.revision.0 != revision {
                    return Err(invalid());
                }
                Ok(())
            }
            Operation::ReplaceTeam {
                team,
                expected_revision,
            } => {
                team.validate().map_err(|_| invalid())?;
                let revision = match expected_revision {
                    None => 0,
                    Some(revision) => revision.0.checked_add(1).ok_or_else(invalid)?,
                };
                if team.revision.0 != revision {
                    return Err(invalid());
                }
                Ok(())
            }
            Operation::ResolveRoute { request } | Operation::RecordRoute { request } => {
                request.validate().map_err(|_| invalid())?;
                Ok(())
            }
            Operation::SetTaskDependencies {
                project_id,
                task_id,
                dependencies,
            } => {
                if dependencies.len() > symbiote_domain::MAX_TASK_EDGES {
                    return Err(invalid());
                }
                for edge in dependencies {
                    if edge.target.task_id == *task_id && edge.target.project_id == *project_id {
                        return Err(invalid());
                    }
                    if &edge.target.project_id != project_id
                        && !matches!(
                            edge.kind,
                            symbiote_domain::TaskDependencyKind::Requires
                                | symbiote_domain::TaskDependencyKind::ConsumesContractFrom
                                | symbiote_domain::TaskDependencyKind::Blocks
                                | symbiote_domain::TaskDependencyKind::Reviews
                                | symbiote_domain::TaskDependencyKind::Verifies
                                | symbiote_domain::TaskDependencyKind::Supersedes
                                | symbiote_domain::TaskDependencyKind::ConflictsWith
                                | symbiote_domain::TaskDependencyKind::FollowUpTo
                        )
                    {
                        return Err(invalid());
                    }
                }
                Ok(())
            }
            Operation::AssignTaskOrigin {
                project_id, origin, ..
            } if &origin.reference().project_id != project_id => Err(invalid()),
            Operation::CreateWork { work } => work.validate().map_err(|_| invalid()),
            Operation::ChangeWork {
                project_id,
                id,
                edit: WorkEdit::Revise { spec },
                ..
            } => {
                if &spec.project_id != project_id || &spec.id != id {
                    return Err(invalid());
                }
                spec.validate().map_err(|_| invalid())
            }
            Operation::Hello { supported_versions }
                if supported_versions.is_empty() || supported_versions.len() > 16 =>
            {
                Err(invalid())
            }
            Operation::RegisterProject { project } => project.validate(),
            Operation::CreateTask { task } => task.validate(),
            Operation::RecordResourceConsent { snapshot, .. } => {
                snapshot.validate().map_err(|_| invalid())
            }
            Operation::ReadJournal { limit, .. } if *limit == 0 || *limit > MAX_PAGE_SIZE => {
                Err(invalid())
            }
            _ => Ok(()),
        }
    }
}

/// Framing and authentication are transport-owned. This parser accepts exactly one
/// JSON value, rejects unknown/duplicate fields, and checks size before decoding.
pub fn parse_request(bytes: &[u8]) -> Result<Request, ProtocolError> {
    if bytes.len() > MAX_REQUEST_BYTES {
        return Err(ProtocolError::new(ErrorCode::RequestTooLarge));
    }
    let request: Request = serde_json::from_slice(bytes).map_err(|_| invalid())?;
    request.validate()?;
    Ok(request)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ProjectPermission {
    ManageBindings,
    ManageTeam,
    ManageWork,
    Register,
    Read,
    CreateTask,
    ReadJournal,
}

/// Server-created identity/grants only. Deliberately not Serialize/Deserialize.
#[derive(Clone, Debug)]
pub struct Principal {
    user: UserId,
    grants: BTreeMap<ProjectId, BTreeSet<ProjectPermission>>,
    local_owner: bool,
}
impl Principal {
    pub fn restricted(
        user: UserId,
        grants: BTreeMap<ProjectId, BTreeSet<ProjectPermission>>,
    ) -> Self {
        Self {
            user,
            grants,
            local_owner: false,
        }
    }
    /// Explicit bootstrap policy for an authenticated same-UID local owner only.
    /// Never infer this from a claimed wire user, IP address, or successful connect.
    pub fn local_owner(user: UserId) -> Self {
        Self {
            user,
            grants: BTreeMap::new(),
            local_owner: true,
        }
    }
    pub fn user_id(&self) -> &UserId {
        &self.user
    }
    /// True only for the explicit bootstrap local-owner policy.
    pub fn is_local_owner(&self) -> bool {
        self.local_owner
    }
    pub fn permits(&self, project: &ProjectId, permission: ProjectPermission) -> bool {
        self.local_owner
            || self
                .grants
                .get(project)
                .is_some_and(|grants| grants.contains(&permission))
    }
}

pub fn authorize(principal: &Principal, request: &Request) -> Result<(), ProtocolError> {
    request.validate()?;
    let permitted = match &request.operation {
        Operation::ReplaceBinding { configuration, .. } => principal.permits(
            &configuration.binding.project_id,
            ProjectPermission::ManageBindings,
        ),
        Operation::GetBinding { project_id, .. } => {
            principal.permits(project_id, ProjectPermission::Read)
        }
        Operation::GetBindingReadiness { .. } => principal.local_owner,
        Operation::ResolveRoute { request } => {
            principal.permits(&request.project_id, ProjectPermission::Read)
        }
        Operation::RecordRoute { request } => {
            principal.permits(&request.project_id, ProjectPermission::ManageWork)
        }
        Operation::SetTaskDependencies {
            project_id,
            dependencies,
            ..
        } => {
            principal.permits(project_id, ProjectPermission::ManageWork)
                && dependencies
                    .iter()
                    .all(|edge| principal.permits(&edge.target.project_id, ProjectPermission::Read))
        }
        Operation::GetTaskDependencies { project_id, .. } => {
            // Target-project Read is re-checked by the Host against the stored
            // edges before the response is built; the owning-Project check
            // here bounds the lookup itself.
            principal.permits(project_id, ProjectPermission::Read)
        }
        Operation::AcquireTaskLease { .. } | Operation::ReleaseTaskLease { .. } => {
            // Lease authority belongs to the authenticated Host; the local
            // owner bootstrap policy is not a worker identity, but only the
            // Host process holds dispatch identity today.
            principal.local_owner
        }
        Operation::ExpireStaleLeases {} | Operation::GetSchedulingProjection {} => {
            principal.local_owner
        }
        Operation::ReplaceProviderConnection { .. }
        | Operation::ReplaceBillingEntitlement { .. }
        | Operation::ReplaceModelDescriptor { .. } => {
            // Provider identity is Host-owned infrastructure; the local owner
            // bootstrap policy registers it. The attribution project is
            // provenance only and scopes nothing (the store separately
            // verifies the project exists).
            principal.local_owner
        }
        Operation::GetProviderConnection { .. }
        | Operation::GetBillingEntitlement { .. }
        | Operation::GetModelDescriptor { .. } => principal.local_owner,
        Operation::PrepareDispatch { .. } | Operation::GetDispatchPreparation { .. } => {
            principal.local_owner
        }
        Operation::GetRoute { project_id, .. } => {
            principal.permits(project_id, ProjectPermission::Read)
        }
        Operation::ReplaceTeam { team, .. } => {
            principal.permits(&team.project_id, ProjectPermission::ManageTeam)
        }
        Operation::GetTeam { project_id } => principal.permits(project_id, ProjectPermission::Read),
        Operation::CreateWork { work } => {
            authorize_work_spec(principal, work, ProjectPermission::ManageWork)?;
            true
        }
        Operation::ChangeWork {
            project_id, edit, ..
        } => {
            if let WorkEdit::Revise { spec } = edit {
                authorize_work_spec(principal, spec, ProjectPermission::ManageWork)?;
            }
            principal.permits(project_id, ProjectPermission::ManageWork)
        }
        Operation::AssignTaskOrigin { project_id, .. } => {
            principal.permits(project_id, ProjectPermission::ManageWork)
        }
        Operation::GetWork { project_id, .. } | Operation::GetTaskOrigin { project_id, .. } => {
            principal.permits(project_id, ProjectPermission::Read)
        }
        Operation::Hello { .. } | Operation::Health {} => true,
        Operation::GetHostPulse {} => principal.local_owner,
        Operation::Shutdown {} => principal.local_owner,
        Operation::RecordResourceConsent { .. } | Operation::RevokeResourceConsent { .. } => {
            principal.local_owner
        }
        Operation::RegisterProject { project } => {
            principal.permits(&project.id, ProjectPermission::Register)
        }
        Operation::GetProject { project_id }
        | Operation::GetTask { project_id, .. }
        | Operation::GetResourceConsent { project_id, .. } => {
            principal.permits(project_id, ProjectPermission::Read)
        }
        Operation::CreateTask { task } => {
            principal.permits(&task.project_id, ProjectPermission::CreateTask)
        }
        Operation::ReadJournal { project_id, .. } => {
            principal.permits(project_id, ProjectPermission::ReadJournal)
        }
    };
    if permitted {
        Ok(())
    } else {
        Err(ProtocolError::new(ErrorCode::PermissionDenied))
    }
}

/// Canonical completion is deliberately absent from client operations.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum WorkEdit {
    Revise { spec: Box<WorkSpec> },
    Clarify { question: String },
    RequestApproval {},
    Approve {},
    Start {},
    RequestCompletion {},
    Cancel { reason: String },
    Reopen { reason: String },
}
impl WorkEdit {
    pub fn into_action(self) -> WorkAction {
        match self {
            Self::Revise { spec } => WorkAction::Revise { spec },
            Self::Clarify { question } => WorkAction::Clarify { question },
            Self::RequestApproval {} => WorkAction::RequestApproval {},
            Self::Approve {} => WorkAction::Approve {},
            Self::Start {} => WorkAction::Start {},
            Self::RequestCompletion {} => WorkAction::RequestCompletion {},
            Self::Cancel { reason } => WorkAction::Cancel { reason },
            Self::Reopen { reason } => WorkAction::Reopen { reason },
        }
    }
}

pub fn authorize_work_spec(
    principal: &Principal,
    spec: &WorkSpec,
    permission: ProjectPermission,
) -> Result<(), ProtocolError> {
    if !principal.permits(&spec.project_id, permission)
        || spec
            .references()
            .iter()
            .any(|reference| !principal.permits(&reference.project_id, ProjectPermission::Read))
    {
        return Err(ProtocolError::new(ErrorCode::PermissionDenied));
    }
    Ok(())
}
pub fn authorize_work_resource(
    principal: &Principal,
    project_id: &ProjectId,
    item: &WorkItem,
) -> Result<(), ProtocolError> {
    if item.project_id() != project_id
        || !principal.permits(project_id, ProjectPermission::Read)
        || item
            .references()
            .iter()
            .any(|reference| !principal.permits(&reference.project_id, ProjectPermission::Read))
    {
        return Err(ProtocolError::new(ErrorCode::PermissionDenied));
    }
    Ok(())
}

/// After fetching by task identity, bind the stored resource to the authorized
/// Project before returning any data. A client-supplied project ID is not evidence.
pub fn authorize_task_resource(
    principal: &Principal,
    project: &ProjectId,
    task: &Task,
) -> Result<(), ProtocolError> {
    if task.project_id() != project || !principal.permits(project, ProjectPermission::Read) {
        return Err(ProtocolError::new(ErrorCode::PermissionDenied));
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProjectDraft {
    pub id: ProjectId,
    pub name: String,
    pub lead: RoleId,
    pub roots: Vec<Root>,
    pub roles: Vec<Role>,
}
impl ProjectDraft {
    pub fn validate(&self) -> Result<(), ProtocolError> {
        if self.name.trim().is_empty()
            || self.name.len() > 256
            || self.roots.is_empty()
            || self.roots.len() > 32
            || self.roles.is_empty()
            || self.roles.len() > 128
        {
            return Err(invalid());
        }
        let mut roots = BTreeSet::new();
        for root in &self.roots {
            if root.project_id != self.id
                || root.revision != Revision(0)
                || !root.host_paths.is_empty()
                || !roots.insert(root.id.clone())
            {
                return Err(invalid());
            }
        }
        let mut roles = BTreeSet::new();
        for role in &self.roles {
            if role.project_id != self.id
                || role.revision != Revision(0)
                || role.name.trim().is_empty()
                || role.name.len() > 256
                || !roles.insert(role.id.clone())
            {
                return Err(invalid());
            }
        }
        if !roles.contains(&self.lead) {
            return Err(invalid());
        }
        Ok(())
    }
    pub fn into_records(
        self,
        principal: &Principal,
        at: Timestamp,
    ) -> Result<(Project, Vec<Root>, Vec<Role>), ProtocolError> {
        self.validate()?;
        if !principal.permits(&self.id, ProjectPermission::Register) {
            return Err(ProtocolError::new(ErrorCode::PermissionDenied));
        }
        let project = Project {
            id: self.id,
            revision: Revision(0),
            name: self.name,
            owner: principal.user.clone(),
            roots: self.roots.iter().map(|root| root.id.clone()).collect(),
            lead: self.lead,
            disposition: RecordDisposition::Active,
            provenance: Provenance {
                created_at: at,
                updated_at: at,
                actor: Actor::User(principal.user.clone()),
                external_references: Vec::new(),
            },
        };
        Ok((project, self.roots, self.roles))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct InitialStream {
    pub id: ChangeStreamId,
    pub originating_chat: ChatId,
    pub worktree: WorktreeId,
    pub branch: String,
    pub base: CommitSha,
    pub target: CommitSha,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TaskDraft {
    pub origin: TaskOrigin,
    pub id: TaskId,
    pub project_id: ProjectId,
    pub root_id: RootId,
    pub role_id: RoleId,
    pub task_contract: VersionedTaskContract,
    pub stream: InitialStream,
}
impl TaskDraft {
    pub fn validate(&self) -> Result<(), ProtocolError> {
        if self.origin.reference().project_id != self.project_id
            || self.stream.branch.trim().is_empty()
            || self.stream.branch.len() > 512
            || self.stream.branch.chars().any(char::is_control)
        {
            return Err(invalid());
        }
        Ok(())
    }
    /// Storage must verify root/Role existence and Project lineage transactionally.
    /// Declaring a worktree/branch neither creates it nor authorizes filesystem I/O.
    pub fn into_records(self) -> Result<(Task, ChangeStream), ProtocolError> {
        self.validate()?;
        let task = Task::new(
            self.id.clone(),
            self.project_id.clone(),
            self.root_id.clone(),
            self.role_id,
            self.stream.id.clone(),
            self.task_contract,
        );
        let stream = ChangeStream::new(NewChangeStream {
            id: self.stream.id,
            project_id: self.project_id,
            root_id: self.root_id,
            tasks: [self.id].into(),
            originating_chat: self.stream.originating_chat,
            worktree: self.stream.worktree,
            branch: self.stream.branch,
            lineage: StreamLineage::Independent,
            base: self.stream.base,
            target: self.stream.target,
        })
        .map_err(|_| invalid())?;
        Ok((task, stream))
    }
}

#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(transparent)]
pub struct JournalCursor(pub u64);

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    DispatchPreparation,
    ProviderRegistryWrite,
    ProviderRegistryRead,
    TaskLeaseManagement,
    SchedulingProjection,
    TaskDependencyWrite,
    TaskDependencyRead,
    RouteResolution,
    RouteRecording,
    RouteRead,
    BindingReadiness,
    BindingConfiguration,
    BindingRead,
    HostPulseRead,
    TeamConfiguration,
    TeamRead,
    WorkCreation,
    WorkChange,
    WorkRead,
    TaskOriginAssignment,
    TaskOriginRead,
    ProjectRegistration,
    ProjectRead,
    TaskCreation,
    TaskRead,
    JournalPagination,
    LocalShutdown,
    ResourceConsentRecord,
    ResourceConsentRead,
    ResourceConsentRevoke,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ServerHello {
    pub version: ProtocolVersion,
    pub capabilities: BTreeSet<Capability>,
    pub max_request_bytes: u32,
    pub max_page_size: u32,
}
pub fn negotiate(offered: &[ProtocolVersion]) -> Result<ServerHello, ProtocolError> {
    if offered.is_empty() || offered.len() > 16 {
        return Err(invalid());
    }
    if !offered.contains(&CURRENT_VERSION) {
        return Err(ProtocolError::new(ErrorCode::UnsupportedVersion));
    }
    Ok(ServerHello {
        version: CURRENT_VERSION,
        capabilities: [
            Capability::DispatchPreparation,
            Capability::ProviderRegistryWrite,
            Capability::ProviderRegistryRead,
            Capability::TaskLeaseManagement,
            Capability::SchedulingProjection,
            Capability::TaskDependencyWrite,
            Capability::TaskDependencyRead,
            Capability::RouteResolution,
            Capability::RouteRecording,
            Capability::RouteRead,
            Capability::BindingReadiness,
            Capability::BindingConfiguration,
            Capability::BindingRead,
            Capability::HostPulseRead,
            Capability::TeamConfiguration,
            Capability::TeamRead,
            Capability::WorkCreation,
            Capability::WorkChange,
            Capability::WorkRead,
            Capability::TaskOriginAssignment,
            Capability::TaskOriginRead,
            Capability::ProjectRegistration,
            Capability::ProjectRead,
            Capability::TaskCreation,
            Capability::TaskRead,
            Capability::JournalPagination,
            Capability::LocalShutdown,
            Capability::ResourceConsentRecord,
            Capability::ResourceConsentRead,
            Capability::ResourceConsentRevoke,
        ]
        .into(),
        max_request_bytes: MAX_REQUEST_BYTES as u32,
        max_page_size: MAX_PAGE_SIZE,
    })
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Receipt {
    pub sequence: u64,
    pub revision: Revision,
    pub replayed: bool,
}

/// Durable events are server output. There is no wire command to append one.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum EventPayload {
    BindingReplaced {
        configuration: Box<symbiote_workforce::BindingConfiguration>,
        expected_revision: Option<Revision>,
        actor: UserId,
        at: Timestamp,
    },
    TeamReplaced {
        team: Box<TeamConfiguration>,
        expected_revision: Option<Revision>,
        actor: UserId,
        at: Timestamp,
    },
    WorkItemCreated {
        item: Box<WorkItem>,
    },
    WorkItemChanged {
        project_id: ProjectId,
        work_id: WorkId,
        command: Box<WorkCommand>,
        item: Box<WorkItem>,
    },
    TaskOriginAssigned {
        task_id: TaskId,
        project_id: ProjectId,
        origin: TaskOrigin,
        actor: UserId,
        at: Timestamp,
    },
    WorkRouted {
        decision: Box<symbiote_workforce::RouteDecision>,
        actor: UserId,
        at: Timestamp,
    },
    TaskDependenciesSet {
        task_id: TaskId,
        project_id: ProjectId,
        edges: Vec<symbiote_domain::TaskDependencyEdge>,
        actor: UserId,
        at: Timestamp,
    },
    TaskLeased {
        lease: Box<symbiote_domain::TaskLease>,
        actor: UserId,
        at: Timestamp,
    },
    ProviderRegistered {
        attribution: ProjectId,
        connection: symbiote_domain::ProviderConnection,
        actor: UserId,
        at: Timestamp,
    },
    EntitlementRegistered {
        attribution: ProjectId,
        entitlement: Box<symbiote_domain::BillingEntitlement>,
        actor: UserId,
        at: Timestamp,
    },
    ModelRegistered {
        attribution: ProjectId,
        descriptor: Box<symbiote_runtime_sdk::provider::ModelDescriptor>,
        actor: UserId,
        at: Timestamp,
    },
    DispatchPrepared {
        preparation: Box<symbiote_domain::DispatchPreparation>,
        actor: UserId,
        at: Timestamp,
    },
    ResourceConsentRecorded {
        consent: Box<ResourceConsent>,
    },
    ResourceConsentRevoked {
        consent: Box<ResourceConsent>,
        revoked_by: UserId,
    },
    ProjectRegistered {
        project: Project,
        roots: Vec<Root>,
        roles: Vec<Role>,
    },
    TaskCreated {
        task: Box<Task>,
        stream: Box<ChangeStream>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        origin: Option<TaskOrigin>,
    },
    TaskChanged {
        task_id: TaskId,
        command: TaskCommand,
        task: Box<Task>,
    },
}
impl EventPayload {
    fn project_id(&self) -> &ProjectId {
        match self {
            Self::BindingReplaced { configuration, .. } => &configuration.binding.project_id,
            Self::TeamReplaced { team, .. } => &team.project_id,
            Self::WorkItemCreated { item } | Self::WorkItemChanged { item, .. } => {
                item.project_id()
            }
            Self::TaskOriginAssigned { project_id, .. } => project_id,
            Self::ResourceConsentRecorded { consent }
            | Self::ResourceConsentRevoked { consent, .. } => &consent.snapshot.project_id,
            Self::ProjectRegistered { project, .. } => &project.id,
            Self::TaskCreated { task, .. } | Self::TaskChanged { task, .. } => task.project_id(),
            Self::WorkRouted { decision, .. } => &decision.project_id,
            Self::TaskDependenciesSet { project_id, .. } => project_id,
            Self::TaskLeased { lease, .. } => &lease.project_id,
            Self::ProviderRegistered { attribution, .. }
            | Self::EntitlementRegistered { attribution, .. }
            | Self::ModelRegistered { attribution, .. } => attribution,
            Self::DispatchPrepared { preparation, .. } => &preparation.project_id,
        }
    }
    fn lineage_matches(&self, project_id: &ProjectId) -> bool {
        match self {
            Self::BindingReplaced {
                configuration,
                expected_revision,
                ..
            } => {
                &configuration.binding.project_id == project_id
                    && configuration.validate().is_ok()
                    && match expected_revision {
                        None => configuration.binding.revision == Revision(0),
                        Some(revision) => {
                            revision.0.checked_add(1) == Some(configuration.binding.revision.0)
                        }
                    }
            }
            Self::TeamReplaced {
                team,
                expected_revision,
                ..
            } => {
                &team.project_id == project_id
                    && team.validate().is_ok()
                    && match expected_revision {
                        None => team.revision == Revision(0),
                        Some(revision) => revision.0.checked_add(1) == Some(team.revision.0),
                    }
            }
            Self::WorkItemCreated { item } => item.project_id() == project_id,
            Self::WorkItemChanged {
                project_id: declared,
                work_id,
                item,
                ..
            } => declared == project_id && item.project_id() == project_id && item.id() == work_id,
            Self::TaskOriginAssigned {
                project_id: declared,
                origin,
                ..
            } => declared == project_id && &origin.reference().project_id == project_id,
            Self::ResourceConsentRecorded { consent } => {
                consent.validate().is_ok()
                    && &consent.snapshot.project_id == project_id
                    && consent.revoked_at.is_none()
            }
            Self::ResourceConsentRevoked { consent, .. } => {
                consent.validate().is_ok()
                    && &consent.snapshot.project_id == project_id
                    && consent.revoked_at.is_some()
            }
            Self::ProjectRegistered {
                project,
                roots,
                roles,
            } => {
                &project.id == project_id
                    && roots.iter().all(|root| &root.project_id == project_id)
                    && roles.iter().all(|role| &role.project_id == project_id)
            }
            Self::TaskCreated {
                task,
                stream,
                origin,
            } => {
                task.project_id() == project_id
                    && origin
                        .as_ref()
                        .is_none_or(|o| &o.reference().project_id == project_id)
                    && stream.project_id() == project_id
                    && task.stream_id() == stream.id()
                    && task.root_id() == stream.root_id()
                    && stream.tasks().contains(task.id())
            }
            Self::WorkRouted { decision, .. } => {
                &decision.project_id == project_id && decision.validate().is_ok()
            }
            Self::TaskDependenciesSet {
                project_id: declared,
                task_id,
                edges,
                ..
            } => {
                declared == project_id
                    && edges.len() <= symbiote_domain::MAX_TASK_EDGES
                    && edges.iter().all(|edge| {
                        (edge.target.project_id.clone(), edge.target.task_id.clone())
                            != (declared.clone(), task_id.clone())
                    })
            }
            Self::TaskLeased { lease, .. } => {
                &lease.project_id == project_id
                    && lease.validate_shape().is_ok()
                    && lease.fencing_token > 0
            }
            // Provider registry events are attributed to a real Project but
            // describe global identity records. Standalone descriptor and
            // timestamp guarantees mirror the store's write-path checks.
            Self::ProviderRegistered { connection, at, .. } => {
                at.0 <= i64::MAX as u64 && !connection.endpoint_reference.trim().is_empty()
            }
            Self::EntitlementRegistered {
                entitlement, at, ..
            } => {
                at.0 <= i64::MAX as u64
                    && !entitlement.provider.as_str().is_empty()
                    && entitlement.expires_at.0 > 0
            }
            Self::ModelRegistered { descriptor, at, .. } => {
                at.0 <= i64::MAX as u64 && descriptor.validate().is_ok()
            }
            Self::TaskChanged { task_id, task, .. } => {
                task.project_id() == project_id && task_id == task.id()
            }
            Self::DispatchPrepared {
                preparation, at, ..
            } => {
                at.0 <= i64::MAX as u64
                    && preparation.validate().is_ok()
                    && &preparation.project_id == project_id
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct JournalEvent {
    pub sequence: u64,
    pub project_id: ProjectId,
    pub command_id: CommandId,
    pub revision: Revision,
    pub payload: EventPayload,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct JournalPage {
    pub events: Vec<JournalEvent>,
    pub next_cursor: JournalCursor,
    pub has_more: bool,
}
impl JournalPage {
    /// Sequence gaps are permitted because other Projects can share a global
    /// journal. Cursor only advances over events actually returned to this client.
    pub fn validate(
        &self,
        project: &ProjectId,
        after: JournalCursor,
        limit: u32,
    ) -> Result<(), ProtocolError> {
        if limit == 0 || limit > MAX_PAGE_SIZE || self.events.len() > limit as usize {
            return Err(invalid());
        }
        let mut previous = after.0;
        for event in &self.events {
            if &event.project_id != project
                || event.payload.project_id() != project
                || !event.payload.lineage_matches(project)
                || event.sequence <= previous
            {
                return Err(ProtocolError::new(ErrorCode::InvalidCursor));
            }
            previous = event.sequence;
        }
        if self.next_cursor.0 != previous || (self.has_more && self.events.is_empty()) {
            return Err(ProtocolError::new(ErrorCode::InvalidCursor));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExpiredLease {
    pub task_id: TaskId,
    pub fencing_token: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum ResponseBody {
    BindingReadiness(Box<symbiote_workforce::ReadinessReport>),
    RouteDecision(Box<symbiote_workforce::RouteDecision>),
    TaskDependencies(Vec<symbiote_domain::TaskDependencyEdge>),
    SchedulerSweep {
        expired: Vec<ExpiredLease>,
        schedulable: Vec<symbiote_domain::SchedulableTask>,
        blocked: Vec<symbiote_domain::BlockedTask>,
    },
    Binding(Box<symbiote_workforce::BindingConfiguration>),
    HostPulse(Box<symbiote_host_inventory::HostPulse>),
    Team(Box<TeamConfiguration>),
    Work(Box<WorkItem>),
    TaskOrigin(Option<TaskOrigin>),
    Hello(ServerHello),
    Project(Project),
    Task(Box<Task>),
    ResourceConsent(Box<ResourceConsent>),
    Receipt(Receipt),
    TaskLease(Box<symbiote_domain::TaskLease>),
    ProviderConnection(Box<symbiote_domain::ProviderConnection>),
    BillingEntitlement(Box<symbiote_domain::BillingEntitlement>),
    ModelDescriptor(Box<symbiote_runtime_sdk::provider::ModelDescriptor>),
    DispatchPreparation(Box<symbiote_domain::DispatchPreparation>),
    Journal(JournalPage),
    Shutdown {},
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Response {
    pub version: ProtocolVersion,
    /// None only when parsing failed before a trusted correlation ID was decoded.
    pub correlation_id: Option<RequestId>,
    pub result: Result<ResponseBody, ProtocolError>,
}
impl Response {
    pub fn success(request: &Request, body: ResponseBody) -> Self {
        Self {
            version: CURRENT_VERSION,
            correlation_id: Some(request.correlation_id.clone()),
            result: Ok(body),
        }
    }
    pub fn failure(correlation_id: Option<RequestId>, error: ProtocolError) -> Self {
        Self {
            version: CURRENT_VERSION,
            correlation_id,
            result: Err(error),
        }
    }
}

/// Bound serialization while writing, rather than allocating an unbounded JSON
/// buffer and rejecting it afterwards. The transport adds its framing separately.
pub fn encode_response(response: &Response) -> Result<Vec<u8>, ProtocolError> {
    struct Bounded(Vec<u8>);
    impl std::io::Write for Bounded {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            if bytes.len() > MAX_RESPONSE_BYTES - self.0.len() {
                return Err(std::io::Error::other("response bound exceeded"));
            }
            self.0.extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut writer = Bounded(Vec::new());
    serde_json::to_writer(&mut writer, response)
        .map_err(|_| ProtocolError::new(ErrorCode::ResourceExhausted))?;
    Ok(writer.0)
}

/// Lossy observations have no durable sequence and cannot be replayed as domain
/// events or treated as completion evidence. No arbitrary raw harness payloads.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Telemetry {
    ClientHeartbeat { client_id: ClientId },
    QueueDepth { project_id: ProjectId, pending: u32 },
}
