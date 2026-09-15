//! The canonical entities #36 requires that were previously identity-only.
//!
//! The domain contract's coverage table recorded these nouns as "ID-only entries
//! are not delivered entity schemas". Each record here gives one noun exactly one
//! owner: identity, the owning scope, cardinality, lifecycle state where the noun
//! has one, `RecordDisposition` for archival and tombstoning, and `Provenance` for
//! creation/update lineage and external anchors. Records that carry meaning beyond
//! their identity are entities with a state; the ones that are pure observations
//! are value objects and have none.

use crate::ids::*;
use crate::lifecycle::VerificationOutcome;
use crate::model::{Actor, ExternalReference, Provenance, RecordDisposition};
use crate::work::{ObjectiveClass, RequestUtterance};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// A bounded confidence for a knowledge claim, in hundredths of a percent so a
/// claim can be "high confidence" without a floating-point wire type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "u16", into = "u16")]
pub struct Confidence(u16);

impl Confidence {
    /// Confidence in basis points of certainty: `0` is none, `10_000` is certain.
    pub const CERTAIN: Confidence = Confidence(10_000);

    pub fn new(basis_points: u16) -> Result<Self, &'static str> {
        if basis_points > 10_000 {
            return Err("confidence is 0..=10000 basis points of certainty");
        }
        Ok(Self(basis_points))
    }

    pub fn basis_points(self) -> u16 {
        self.0
    }
}

impl TryFrom<u16> for Confidence {
    type Error = &'static str;
    fn try_from(value: u16) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}
impl From<Confidence> for u16 {
    fn from(value: Confidence) -> Self {
        value.0
    }
}

impl JsonSchema for Confidence {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "Confidence".into()
    }
    fn json_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
        schemars::json_schema!({
            "type": "integer",
            "minimum": 0,
            "maximum": 10_000,
            "description": "basis points of certainty; 10000 is certain"
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum UserKind {
    Human,
    Service,
}

/// A principal that can hold authority. Owns Projects, Devices and Fabric planes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct User {
    pub id: UserId,
    pub revision: Revision,
    pub kind: UserKind,
    pub display_name: String,
    pub disposition: RecordDisposition,
    pub provenance: Provenance,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FabricState {
    Enrolled,
    Degraded,
    Retired,
}

/// The set of Hosts one User operates. A Fabric owns Hosts; a Host belongs to at
/// most one Fabric, which is what makes single-machine and multi-Host use the
/// same semantics rather than two models.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Fabric {
    pub id: FabricId,
    pub revision: Revision,
    pub user_id: UserId,
    pub hosts: BTreeSet<HostId>,
    pub state: FabricState,
    pub disposition: RecordDisposition,
    pub provenance: Provenance,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DeviceKind {
    Desktop,
    Mobile,
    Headless,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DeviceState {
    Unpaired,
    Paired,
    Revoked,
}

/// A physical or virtual machine a client runs on. It is not a Host: a Device
/// may be paired to a Host, and revocation is a state rather than a deletion.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Device {
    pub id: DeviceId,
    pub revision: Revision,
    pub user_id: UserId,
    pub kind: DeviceKind,
    pub host_id: Option<HostId>,
    pub state: DeviceState,
    pub disposition: RecordDisposition,
    pub provenance: Provenance,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ClientKind {
    Desktop,
    Cli,
    Mobile,
    Plugin,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ClientState {
    Connected,
    Disconnected,
    Superseded,
}

/// A client installation. Capability advertisement is recorded here so a client
/// can be refused rather than assumed capable; the advertisement is a claim the
/// Host qualifies, never an authorization.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Client {
    pub id: ClientId,
    pub revision: Revision,
    pub user_id: UserId,
    pub device_id: DeviceId,
    pub kind: ClientKind,
    pub advertised_capabilities: BTreeSet<String>,
    pub state: ClientState,
    pub disposition: RecordDisposition,
    pub provenance: Provenance,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ControllerSessionState {
    Open,
    Closed,
}

/// A client's control session over a Project. Closing one releases control
/// without ending the Project, its Team or any authorized headless work.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ControllerSession {
    pub id: ControllerSessionId,
    pub revision: Revision,
    pub client_id: ClientId,
    pub project_id: ProjectId,
    pub state: ControllerSessionState,
    pub disposition: RecordDisposition,
    pub provenance: Provenance,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PairingTransport {
    Local,
    LoopbackRelay,
    RemoteRelay,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PairingState {
    Proposed,
    Confirmed,
    Revoked,
    Expired,
}

/// A device-to-Host pairing. Expiry and revocation are terminal for the pairing
/// identity: re-pairing is a new Pairing, never a resurrected one.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Pairing {
    pub id: PairingId,
    pub revision: Revision,
    pub device_id: DeviceId,
    pub host_id: HostId,
    pub transport: PairingTransport,
    pub state: PairingState,
    pub expires_at: Timestamp,
    pub disposition: RecordDisposition,
    pub provenance: Provenance,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EnvironmentState {
    Discovered,
    Verified,
    Unavailable,
    Retired,
}

/// An execution environment: the machine plus the installed harness set a
/// dispatch may resolve against. Ownership sits with the Root it executes in.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Environment {
    pub id: EnvironmentId,
    pub revision: Revision,
    pub project_id: ProjectId,
    pub root_id: RootId,
    pub host_id: HostId,
    pub harness_drivers: BTreeSet<HarnessDriverId>,
    pub state: EnvironmentState,
    pub disposition: RecordDisposition,
    pub provenance: Provenance,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DriverState {
    Registered,
    Deprecated,
    Retired,
}

/// A harness driver: the software family a Harness Pack integrates with. This is
/// a value object for the domain — integration depth and truthful degradation
/// belong to the adapter SDK, which owns that claim.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct HarnessDriver {
    pub id: HarnessDriverId,
    pub revision: Revision,
    pub name: String,
    pub vendor: String,
    pub state: DriverState,
    pub disposition: RecordDisposition,
    pub provenance: Provenance,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DiscoverySource {
    Configured,
    PathScan,
    UserDeclared,
    PackageManager,
}

/// One installed driver on one Environment. Version and discovery source are
/// observations with a date, not standing facts about the machine.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Installation {
    pub id: InstallationId,
    pub revision: Revision,
    pub environment_id: EnvironmentId,
    pub harness_driver_id: HarnessDriverId,
    pub version: String,
    pub path: String,
    pub discovered_by: DiscoverySource,
    pub disposition: RecordDisposition,
    pub provenance: Provenance,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ModelState {
    Available,
    Degraded,
    Unavailable,
    Retired,
}

/// A model offered through a provider connection. Models are records because
/// availability moves independently of the connection that offers them.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Model {
    pub id: ModelId,
    pub revision: Revision,
    pub provider_connection_id: ProviderConnectionId,
    pub name: String,
    pub state: ModelState,
    pub disposition: RecordDisposition,
    pub provenance: Provenance,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RequestState {
    Received,
    Accepted,
    Rejected,
    Fulfilled,
    Abandoned,
}

/// A client request as received, before any interpretation. Preserving the
/// request separately from the objective it produces is what makes "the
/// transcript is not the source of truth" checkable: the accepted intent is a
/// record with its own identity.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub id: RequestId,
    pub revision: Revision,
    pub project_id: ProjectId,
    pub requested_by: UserId,
    pub utterance: RequestUtterance,
    pub state: RequestState,
    pub disposition: RecordDisposition,
    pub provenance: Provenance,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityState {
    Proposed,
    Qualified,
    Unavailable,
    Retired,
}

/// A capability the product claims, and whether it is qualified. Capability
/// Closure is assessed against these records rather than against prose.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Capability {
    pub id: CapabilityId,
    pub revision: Revision,
    pub project_id: ProjectId,
    pub name: String,
    pub required_controls: BTreeSet<String>,
    pub state: CapabilityState,
    pub disposition: RecordDisposition,
    pub provenance: Provenance,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ObjectiveState {
    Proposed,
    Active,
    Achieved,
    Abandoned,
}

/// An accepted objective. It is deliberate that this is not a Task: an objective
/// may be reached by no task at all, and the objective outlives the dispatch of
/// any single one.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Objective {
    pub id: ObjectiveId,
    pub revision: Revision,
    pub project_id: ProjectId,
    pub class: ObjectiveClass,
    pub statement: String,
    pub source_request: Option<RequestId>,
    pub state: ObjectiveState,
    pub disposition: RecordDisposition,
    pub provenance: Provenance,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ChatOrigin {
    Human,
    Orchestrator,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ChatState {
    Open,
    Closed,
}

/// A conversation. Distinct from Session (a runtime session), from ChangeStream
/// (the durable unit of mutation) and from Worktree: a chat may create a stream,
/// and a stream may outlive any chat that wrote to it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Chat {
    pub id: ChatId,
    pub revision: Revision,
    pub project_id: ProjectId,
    pub stream_id: Option<ChangeStreamId>,
    pub origin: ChatOrigin,
    pub mutating: bool,
    pub state: ChatState,
    pub disposition: RecordDisposition,
    pub provenance: Provenance,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum WorktreeState {
    Allocated,
    Released,
    Reclaimed,
}

/// A Git worktree allocated to a Root. Worktrees isolate Git mutation, not
/// network, secrets, production services or the filesystem; this record is the
/// allocation, and the enforcing boundary is owned by the sandbox.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Worktree {
    pub id: WorktreeId,
    pub revision: Revision,
    pub project_id: ProjectId,
    pub root_id: RootId,
    pub stream_id: Option<ChangeStreamId>,
    pub branch: String,
    pub path: String,
    pub state: WorktreeState,
    pub disposition: RecordDisposition,
    pub provenance: Provenance,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RequirementState {
    Draft,
    Accepted,
    Superseded,
    Retired,
}

/// An approved requirement. Supersession is lineage, not deletion: an accepted
/// requirement is immutable and a change arrives as a new record naming it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Requirement {
    pub id: RequirementId,
    pub revision: Revision,
    pub project_id: ProjectId,
    pub statement: String,
    pub supersedes: Option<RequirementId>,
    pub state: RequirementState,
    pub disposition: RecordDisposition,
    pub provenance: Provenance,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DecisionState {
    Proposed,
    Investigating,
    Accepted,
    Superseded,
    Rejected,
    Retired,
}

/// An architecture decision. The state set is the one #173 owns, expressed here
/// as the domain shape so downstream issues do not each invent their own.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Decision {
    pub id: DecisionId,
    pub revision: Revision,
    pub project_id: ProjectId,
    pub statement: String,
    pub alternatives: Vec<String>,
    pub evidence: BTreeSet<EvidenceId>,
    pub supersedes: Option<DecisionId>,
    pub state: DecisionState,
    pub disposition: RecordDisposition,
    pub provenance: Provenance,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AmbiguityState {
    Open,
    Escalated,
    Resolved,
}

/// An unresolved question. An implementation worker may not quietly convert one
/// of these into code, which is why it is a record with an owner and a state.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Ambiguity {
    pub id: AmbiguityId,
    pub revision: Revision,
    pub project_id: ProjectId,
    pub question: String,
    pub resolved_by: Option<DecisionId>,
    pub state: AmbiguityState,
    pub disposition: RecordDisposition,
    pub provenance: Provenance,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Info,
    Warning,
    Error,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticState {
    Emitted,
    Acknowledged,
    Resolved,
}

/// An observable finding. A degraded or unknown state is emitted here rather
/// than swallowed, so "an unknown is not a zero" has somewhere to live.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Diagnostic {
    pub id: DiagnosticId,
    pub revision: Revision,
    pub project_id: ProjectId,
    pub task_id: Option<TaskId>,
    pub severity: Severity,
    pub message: String,
    pub state: DiagnosticState,
    pub disposition: RecordDisposition,
    pub provenance: Provenance,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", content = "id", rename_all = "snake_case")]
pub enum AnnotationTarget {
    Artifact(ArtifactId),
    Evidence(EvidenceId),
    Knowledge(KnowledgeId),
    Requirement(RequirementId),
    Decision(DecisionId),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AnnotationState {
    Open,
    Resolved,
}

/// A human or worker note attached to another record. The target is typed, so an
/// annotation cannot float free of what it annotates.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Annotation {
    pub id: AnnotationId,
    pub revision: Revision,
    pub project_id: ProjectId,
    pub target: AnnotationTarget,
    pub body: String,
    pub author: Actor,
    pub state: AnnotationState,
    pub disposition: RecordDisposition,
    pub provenance: Provenance,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PluginState {
    Available,
    Enabled,
    Disabled,
    Retired,
}

/// A plugin identity. Enabling grants no authority by itself: trust and the data
/// ownership policy are owned elsewhere, and this record only says which plugin
/// is enabled where.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Plugin {
    pub id: PluginId,
    pub revision: Revision,
    pub project_id: Option<ProjectId>,
    pub name: String,
    pub state: PluginState,
    pub disposition: RecordDisposition,
    pub provenance: Provenance,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReleaseTargetState {
    Planned,
    Frozen,
    Released,
    Retired,
}

/// A release target. Frozen is a state rather than a lock, so an unplanned
/// mutation is a state conflict rather than a silent edit.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReleaseTarget {
    pub id: ReleaseTargetId,
    pub revision: Revision,
    pub project_id: ProjectId,
    pub label: String,
    pub state: ReleaseTargetState,
    pub disposition: RecordDisposition,
    pub provenance: Provenance,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum VerificationRunState {
    Started,
    Finished,
}

/// A recorded verification run. This is the record that makes the completion
/// gates auditable: `VerificationEvidence` cites a run, and the run carries the
/// exact source and target pair it was produced against. Recording a run is not
/// asserting that it passed — `outcome` carries what the run reported.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct VerificationRun {
    pub id: VerificationRunId,
    pub revision: Revision,
    pub project_id: ProjectId,
    pub task_id: TaskId,
    pub dispatch_id: DispatchId,
    pub stream_id: ChangeStreamId,
    pub source: CommitSha,
    pub target: CommitSha,
    pub outcome: VerificationOutcome,
    pub state: VerificationRunState,
    pub started_at: Timestamp,
    pub finished_at: Option<Timestamp>,
    pub disposition: RecordDisposition,
    pub provenance: Provenance,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SecretLeaseState {
    Active,
    Expired,
    Revoked,
}

/// A lease over a credential reference. The record holds a reference and an
/// expiry, never a secret value; expiry and revocation are states so a lease is
/// never silently reusable.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SecretLease {
    pub id: SecretLeaseId,
    pub revision: Revision,
    pub project_id: ProjectId,
    pub credential_reference_id: CredentialReferenceId,
    pub expires_at: Timestamp,
    pub state: SecretLeaseState,
    pub disposition: RecordDisposition,
    pub provenance: Provenance,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProjectionKind {
    HumanDocumentation,
    AgentContext,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProjectionState {
    Draft,
    Published,
    Retracted,
}

/// A published projection of knowledge. Human documentation and compact agent
/// context are projections of the same claims, so neither is an independent
/// authority: both point back at the `KnowledgeClaim` they render.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Projection {
    pub id: ProjectionId,
    pub revision: Revision,
    pub knowledge_id: KnowledgeId,
    pub kind: ProjectionKind,
    pub state: ProjectionState,
    pub published_at: Option<Timestamp>,
    pub disposition: RecordDisposition,
    pub provenance: Provenance,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PlanState {
    Draft,
    Active,
    Achieved,
    Abandoned,
}

/// A plan for reaching an Objective. It is not a Task and owns no dispatch: a
/// plan is the intention, and tasks are what carries it out.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Plan {
    pub id: PlanId,
    pub revision: Revision,
    pub project_id: ProjectId,
    pub objective_id: ObjectiveId,
    pub statement: String,
    pub state: PlanState,
    pub disposition: RecordDisposition,
    pub provenance: Provenance,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MilestoneState {
    Planned,
    Reached,
    Missed,
    Abandoned,
}

/// A milestone within a plan. Reaching one is recorded, not inferred from task
/// completion, so a plan's progress is auditable rather than narrated.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Milestone {
    pub id: MilestoneId,
    pub revision: Revision,
    pub project_id: ProjectId,
    pub plan_id: PlanId,
    pub statement: String,
    pub state: MilestoneState,
    pub disposition: RecordDisposition,
    pub provenance: Provenance,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CheckpointState {
    Pending,
    Reached,
    Skipped,
}

/// A checkpoint within a plan: a point a plan may be paused or resumed from.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Checkpoint {
    pub id: CheckpointId,
    pub revision: Revision,
    pub project_id: ProjectId,
    pub plan_id: PlanId,
    pub sequence: u32,
    pub state: CheckpointState,
    pub disposition: RecordDisposition,
    pub provenance: Provenance,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FollowUpState {
    Open,
    Accepted,
    Declined,
    Done,
}

/// Work discovered while doing other work. It is a record rather than a note so
/// newly discovered scope is linked canonical work and not silently dropped.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FollowUp {
    pub id: FollowUpId,
    pub revision: Revision,
    pub project_id: ProjectId,
    pub origin: ExternalReference,
    pub statement: String,
    pub state: FollowUpState,
    pub disposition: RecordDisposition,
    pub provenance: Provenance,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ConfigState {
    Projected,
    Stale,
    Withdrawn,
}

/// The projection of canonical configuration into a runtime's own config home.
/// Ownership is recorded so a runtime's own config file is never mistaken for
/// the canonical record it was derived from.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ConfigProjection {
    pub id: ConfigProjectionId,
    pub revision: Revision,
    pub project_id: ProjectId,
    pub runtime_profile_id: RuntimeProfileId,
    pub locator: String,
    pub state: ConfigState,
    pub disposition: RecordDisposition,
    pub provenance: Provenance,
}
