use crate::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Provenance {
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
    pub actor: Actor,
    pub external_references: Vec<ExternalReference>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", content = "id", rename_all = "snake_case")]
pub enum Actor {
    User(UserId),
    Host(HostId),
    Worker(DispatchId),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExternalReference {
    pub system: ExternalSystem,
    pub locator: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExternalSystem {
    Github,
    Harness,
    Ci,
    Deployment,
    SourceSymbol,
    Publication,
    RemoteHost,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RecordDisposition {
    Active,
    Archived { at: Timestamp },
    Tombstoned { at: Timestamp, reason: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Project {
    pub id: ProjectId,
    pub revision: Revision,
    pub name: String,
    pub owner: UserId,
    pub roots: BTreeSet<RootId>,
    pub lead: RoleId,
    pub disposition: RecordDisposition,
    pub provenance: Provenance,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Root {
    pub id: RootId,
    pub project_id: ProjectId,
    pub revision: Revision,
    pub repository: Option<ExternalReference>,
    /// An observed placement, not identity or filesystem authorization.
    pub host_paths: BTreeMap<HostId, String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Role {
    pub id: RoleId,
    pub project_id: ProjectId,
    pub revision: Revision,
    pub name: String,
    pub operating_contract: VersionedRoleContract,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RuntimeKind {
    NativeSymbiote,
    ExternalHarness,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RuntimeProfile {
    pub id: RuntimeProfileId,
    pub revision: Revision,
    pub runtime: RuntimeKind,
    pub adapter: AgentRuntimeAdapterId,
    pub installation: Option<InstallationId>,
    pub provider: ProviderConnectionId,
    pub credential: CredentialReferenceId,
    pub billing_entitlement: BillingEntitlementId,
    pub model: ModelId,
    pub eligible_hosts: BTreeSet<HostId>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProviderConnection {
    pub id: ProviderConnectionId,
    pub adapter: InferenceProviderAdapterId,
    pub endpoint_reference: String,
    pub authentication: AuthenticationKind,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AuthenticationKind {
    ApiCredential,
    HarnessManaged,
    LocalUnauthenticated,
}

/// Contains only a vault reference. Credentials never belong in the contract.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CredentialReference {
    pub id: CredentialReferenceId,
    pub vault_key: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BillingEntitlement {
    pub id: BillingEntitlementId,
    pub provider: ProviderConnectionId,
    pub kind: BillingKind,
    pub verification_evidence: EvidenceId,
    pub expires_at: Timestamp,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum BillingKind {
    MeteredApi,
    HarnessSubscription,
    Local,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkforceBinding {
    pub id: BindingId,
    pub revision: Revision,
    pub project_id: ProjectId,
    pub role_id: RoleId,
    pub profile_id: RuntimeProfileId,
    pub profile_revision: Revision,
    pub protocol: VersionedProtocol,
    pub access: AccessSnapshot,
    pub required_controls: BTreeSet<Control>,
    pub context: ContextPolicy,
    pub required_tools: BTreeSet<String>,
    pub required_skills: BTreeSet<String>,
    pub escalation: EscalationPolicy,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Permission {
    ReadRoot,
    MutateStream,
    ExecuteProcess,
    Network,
    UseCredential,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AccessSnapshot {
    pub project_id: ProjectId,
    pub roots: BTreeSet<RootId>,
    pub grants: BTreeSet<Permission>,
    pub policy_revision: Revision,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Control {
    Filesystem,
    Process,
    Network,
    Credentials,
    Cancellation,
    CompletionAuthority,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EnforcementStrength {
    Native,
    HostEnforced,
    ExternallyObserved,
    Emulated,
    Unsupported,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EnforcementClaim {
    pub strength: EnforcementStrength,
    pub evidence: EvidenceId,
    pub verified_at: Timestamp,
    pub expires_at: Timestamp,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ContextPolicy {
    pub bundle: ContextBundleId,
    pub revision: Revision,
    pub max_input_tokens: u32,
    pub reserved_output_tokens: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EscalationPolicy {
    StopAndRequestHuman,
    ReturnToLead,
}

macro_rules! version_ref {
    ($name:ident, $id:ident) => {
        #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
        #[serde(deny_unknown_fields)]
        pub struct $name {
            pub id: $id,
            pub revision: Revision,
        }
    };
}
version_ref!(VersionedProtocol, ProtocolId);
version_ref!(VersionedRoleContract, RoleContractId);
version_ref!(VersionedTaskContract, TaskContractId);

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Host {
    pub id: HostId,
    pub revision: Revision,
    pub device: DeviceId,
    pub fabric: Option<FabricId>,
    pub supported_runtimes: Vec<RuntimeKind>,
    pub controls: BTreeMap<Control, EnforcementClaim>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Session {
    pub id: SessionId,
    pub dispatch_id: DispatchId,
    pub kind: RuntimeKind,
    pub foreign_reference: Option<ExternalReference>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Artifact {
    pub id: ArtifactId,
    pub project_id: ProjectId,
    pub task_id: TaskId,
    pub dispatch_id: DispatchId,
    pub stream_id: ChangeStreamId,
    pub head: CommitSha,
    pub media_type: String,
    pub content_digest: String,
    pub provenance: Provenance,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EpistemicStatus {
    Confirmed,
    Derived,
    Inferred,
    Unresolved,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Freshness {
    Current,
    Stale,
    Contradicted,
    Debt,
    Unresolved,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct KnowledgeClaim {
    pub id: KnowledgeId,
    pub project_id: ProjectId,
    pub revision: Revision,
    pub statement: String,
    pub audience: BTreeSet<String>,
    pub epistemic_status: EpistemicStatus,
    pub freshness: Freshness,
    pub evidence: BTreeSet<EvidenceId>,
    pub ancestry: BTreeSet<KnowledgeId>,
    pub source_anchors: Vec<ExternalReference>,
    pub projections: BTreeSet<ProjectionId>,
    pub provenance: Provenance,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GoalRun {
    pub id: GoalRunId,
    pub project_id: ProjectId,
    pub objective: ObjectiveId,
    pub max_tokens: u64,
    pub deadline: Timestamp,
    pub executions: BTreeSet<NativeExecutionId>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NativeExecution {
    pub id: NativeExecutionId,
    pub dispatch: DispatchId,
    pub goal: GoalRunId,
    pub parent: Option<NativeExecutionId>,
    pub model: ModelId,
    pub access: AccessSnapshot,
    pub token_limit: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExecutionEpisode {
    pub id: ExecutionEpisodeId,
    pub dispatch: DispatchId,
    pub session: SessionId,
    pub started_at: Timestamp,
    pub ended_at: Option<Timestamp>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LearnedMethod {
    pub id: LearnedMethodId,
    pub project_id: ProjectId,
    pub revision: Revision,
    pub opted_in_by: UserId,
    pub evidence: BTreeSet<EvidenceId>,
    /// Content is a proposal; it has no permission or completion-policy authority.
    pub artifact: ArtifactId,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Experiment {
    pub id: ExperimentId,
    pub project_id: ProjectId,
    pub method: LearnedMethodId,
    pub baseline: BTreeSet<EvidenceId>,
    pub candidate: BTreeSet<EvidenceId>,
    pub sample_size: u64,
    pub missing_samples: u64,
}

/// The schema's roots. Supplementary ontology owners are listed in domain.md;
/// an absent implementation is not disguised as an untyped catch-all entity.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "entity", content = "record", rename_all = "snake_case")]
pub enum DomainRecord {
    Project(Project),
    Root(Root),
    Role(Role),
    WorkforceBinding(WorkforceBinding),
    RuntimeProfile(RuntimeProfile),
    Host(Host),
    Dispatch(Dispatch),
    Task(Task),
    ChangeStream(ChangeStream),
    Session(Session),
    Artifact(Artifact),
    Evidence(VerificationEvidence),
    Knowledge(KnowledgeClaim),
    ProviderConnection(ProviderConnection),
    CredentialReference(CredentialReference),
    BillingEntitlement(BillingEntitlement),
    GoalRun(GoalRun),
    NativeExecution(NativeExecution),
    ExecutionEpisode(ExecutionEpisode),
    LearnedMethod(LearnedMethod),
    Experiment(Experiment),
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DomainEnvelope {
    pub schema_version: u32,
    pub record: DomainRecord,
}

impl DomainEnvelope {
    pub fn validate_version(&self) -> Result<(), DomainError> {
        if self.schema_version == SCHEMA_VERSION {
            Ok(())
        } else {
            Err(DomainError::UnsupportedSchemaVersion)
        }
    }
}
