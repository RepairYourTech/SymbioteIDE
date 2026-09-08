//! Canonical work hierarchy. Worker/external statuses have no transition authority.
//! Host actors and evidence require authentication outside this pure contract.
use crate::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    content = "id",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum WorkId {
    Request(RequestId),
    Objective(ObjectiveId),
    Capability(CapabilityId),
    Plan(PlanId),
    Milestone(MilestoneId),
    Checkpoint(CheckpointId),
    FollowUp(FollowUpId),
}
impl WorkId {
    pub fn key(&self) -> String {
        match self {
            Self::Request(id) => format!("request:{id}"),
            Self::Objective(id) => format!("objective:{id}"),
            Self::Capability(id) => format!("capability:{id}"),
            Self::Plan(id) => format!("plan:{id}"),
            Self::Milestone(id) => format!("milestone:{id}"),
            Self::Checkpoint(id) => format!("checkpoint:{id}"),
            Self::FollowUp(id) => format!("follow_up:{id}"),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkRef {
    pub project_id: ProjectId,
    pub id: WorkId,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RequestUtterance {
    Idea,
    Question,
    Constraint,
    Decision,
    Approval,
    Feedback,
    RequestedWork,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ObjectiveClass {
    Outcome,
    Maintenance,
    Operational,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkBudget {
    pub tokens: Option<u64>,
    pub milliseconds: Option<u64>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkSpec {
    pub id: WorkId,
    pub project_id: ProjectId,
    pub role_id: RoleId,
    pub title: String,
    pub description: String,
    pub utterance: Option<RequestUtterance>,
    pub objective_class: Option<ObjectiveClass>,
    pub parent: Option<WorkRef>,
    pub dependencies: BTreeSet<WorkRef>,
    pub requirements: Vec<String>,
    pub constraints: Vec<String>,
    pub risks: Vec<String>,
    pub acceptance: Vec<String>,
    pub priority: u8,
    pub budget: Option<WorkBudget>,
    pub external_references: Vec<ExternalReference>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum WorkError {
    InvalidSpec,
    ResourceLimit,
    RevisionConflict,
    IdempotencyConflict,
    IllegalTransition,
    PermissionDenied,
    InvalidTimestamp,
    IdentityChanged,
    InvalidEvidence,
    MissingReference,
    ParentKindMismatch,
    Cycle,
    InvalidTaskOrigin,
    InvalidSnapshot,
}
impl fmt::Display for WorkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for WorkError {}
fn bounded_text(text: &str, max: usize, nonempty: bool) -> bool {
    text.len() <= max && (!nonempty || !text.trim().is_empty()) && !text.contains('\0')
}
const MAX_WORK_BYTES: usize = 262_144;
const MAX_SPEC_BYTES: usize = 32_768;
struct WorkSize(usize, usize);
impl std::io::Write for WorkSize {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if bytes.len() > self.1 - self.0 {
            return Err(std::io::Error::other("work aggregate limit"));
        }
        self.0 += bytes.len();
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
impl WorkSpec {
    pub fn reference(&self) -> WorkRef {
        WorkRef {
            project_id: self.project_id.clone(),
            id: self.id.clone(),
        }
    }
    pub fn references(&self) -> BTreeSet<WorkRef> {
        self.dependencies
            .iter()
            .chain(self.parent.iter())
            .cloned()
            .collect()
    }
    pub fn validate(&self) -> Result<(), WorkError> {
        if !bounded_text(&self.title, 256, true)
            || !bounded_text(&self.description, 16_384, false)
            || self.priority > 5
        {
            return Err(WorkError::InvalidSpec);
        }
        if (self.utterance.is_some() && !matches!(self.id, WorkId::Request(_)))
            || (self.objective_class.is_some() && !matches!(self.id, WorkId::Objective(_)))
        {
            return Err(WorkError::InvalidSpec);
        }
        if self.dependencies.len() + usize::from(self.parent.is_some()) > 64 {
            return Err(WorkError::ResourceLimit);
        }
        for list in [
            &self.requirements,
            &self.constraints,
            &self.risks,
            &self.acceptance,
        ] {
            if list.len() > 32 {
                return Err(WorkError::ResourceLimit);
            }
            if list.iter().any(|v| !bounded_text(v, 2048, true)) {
                return Err(WorkError::InvalidSpec);
            }
        }
        if let Some(budget) = &self.budget {
            if (budget.tokens.is_none() && budget.milliseconds.is_none())
                || budget.tokens.is_some_and(|v| v == 0 || v > 1_000_000_000)
                || budget
                    .milliseconds
                    .is_some_and(|v| v == 0 || v > 604_800_000)
            {
                return Err(WorkError::InvalidSpec);
            }
        }
        if self.external_references.len() > 32 {
            return Err(WorkError::ResourceLimit);
        }
        if self
            .external_references
            .iter()
            .any(|r| !bounded_text(&r.locator, 2048, true))
        {
            return Err(WorkError::InvalidSpec);
        }
        if self.references().contains(&self.reference()) {
            return Err(WorkError::Cycle);
        }
        serde_json::to_writer(WorkSize(0, MAX_SPEC_BYTES), self)
            .map_err(|_| WorkError::ResourceLimit)?;
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum WorkState {
    Draft,
    Clarifying,
    AwaitingApproval,
    Approved,
    Running,
    CompletionRequested,
    Completed,
    Cancelled,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AcceptanceEvidence {
    pub criterion: u16,
    pub id: EvidenceId,
    pub artifact_id: ArtifactId,
    pub outcome: VerificationOutcome,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkGateEvidence {
    pub gate: Gate,
    pub id: EvidenceId,
    pub run_id: VerificationRunId,
    pub artifact_id: ArtifactId,
    pub outcome: VerificationOutcome,
    pub reviewer: Option<RoleId>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkCompletionEvidence {
    pub work: WorkRef,
    pub revision: Revision,
    pub verified_by: HostId,
    pub verified_at: Timestamp,
    pub acceptance: Vec<AcceptanceEvidence>,
    pub gates: Vec<WorkGateEvidence>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum WorkAction {
    Revise {
        spec: Box<WorkSpec>,
    },
    Clarify {
        question: String,
    },
    RequestApproval {},
    Approve {},
    Start {},
    RequestCompletion {},
    Complete {
        evidence: Box<WorkCompletionEvidence>,
    },
    Cancel {
        reason: String,
    },
    Reopen {
        reason: String,
    },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkCommand {
    pub id: CommandId,
    pub expected_revision: Revision,
    pub actor: Actor,
    pub at: Timestamp,
    pub action: WorkAction,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkReceipt {
    pub revision: Revision,
    pub replayed: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(try_from = "WorkWire")]
pub struct WorkItem {
    schema_version: u32,
    initial_spec: WorkSpec,
    spec: WorkSpec,
    created_by: UserId,
    created_at: Timestamp,
    revision: Revision,
    state: WorkState,
    history: Vec<WorkCommand>,
}
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct WorkWire {
    schema_version: u32,
    initial_spec: WorkSpec,
    spec: WorkSpec,
    created_by: UserId,
    created_at: Timestamp,
    revision: Revision,
    state: WorkState,
    #[serde(deserialize_with = "history_decode")]
    history: Vec<WorkCommand>,
}
fn history_decode<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<WorkCommand>, D::Error> {
    struct History;
    impl<'de> serde::de::Visitor<'de> for History {
        type Value = Vec<WorkCommand>;
        fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("at most 128 work commands")
        }
        fn visit_seq<A: serde::de::SeqAccess<'de>>(
            self,
            mut seq: A,
        ) -> Result<Self::Value, A::Error> {
            let mut history = Vec::new();
            while let Some(command) = seq.next_element()? {
                if history.len() == 128 {
                    return Err(serde::de::Error::custom("work history limit"));
                }
                history.push(command);
            }
            Ok(history)
        }
    }
    deserializer.deserialize_seq(History)
}
impl TryFrom<WorkWire> for WorkItem {
    type Error = WorkError;
    fn try_from(wire: WorkWire) -> Result<Self, WorkError> {
        if wire.schema_version != 1 {
            return Err(WorkError::InvalidSnapshot);
        }
        let mut item = Self::new(wire.initial_spec, wire.created_by, wire.created_at)?;
        for command in wire.history {
            if item.apply(command)?.replayed {
                return Err(WorkError::InvalidSnapshot);
            }
        }
        if item.spec != wire.spec || item.state != wire.state || item.revision != wire.revision {
            return Err(WorkError::InvalidSnapshot);
        }
        Ok(item)
    }
}
impl WorkItem {
    pub fn new(spec: WorkSpec, created_by: UserId, at: Timestamp) -> Result<Self, WorkError> {
        spec.validate()?;
        let item = Self {
            schema_version: 1,
            initial_spec: spec.clone(),
            spec,
            created_by,
            created_at: at,
            revision: Revision(0),
            state: WorkState::Draft,
            history: Vec::new(),
        };
        serde_json::to_writer(WorkSize(0, MAX_WORK_BYTES), &item)
            .map_err(|_| WorkError::ResourceLimit)?;
        Ok(item)
    }
    /// The wire ingress also caps aggregate bytes; deserialization alone never grants authority.
    pub fn parse(input: &str) -> Result<Self, WorkError> {
        if input.len() > MAX_WORK_BYTES {
            return Err(WorkError::ResourceLimit);
        }
        serde_json::from_str(input).map_err(|_| WorkError::InvalidSnapshot)
    }
    pub fn spec(&self) -> &WorkSpec {
        &self.spec
    }
    pub fn id(&self) -> &WorkId {
        &self.spec.id
    }
    pub fn project_id(&self) -> &ProjectId {
        &self.spec.project_id
    }
    pub fn role_id(&self) -> &RoleId {
        &self.spec.role_id
    }
    pub fn revision(&self) -> Revision {
        self.revision
    }
    pub fn state(&self) -> WorkState {
        self.state
    }
    pub fn history(&self) -> &[WorkCommand] {
        &self.history
    }
    pub fn created_at(&self) -> Timestamp {
        self.created_at
    }
    pub fn created_by(&self) -> &UserId {
        &self.created_by
    }
    pub fn references(&self) -> BTreeSet<WorkRef> {
        let mut refs = self.initial_spec.references();
        refs.extend(self.spec.references());
        for command in &self.history {
            if let WorkAction::Revise { spec } = &command.action {
                refs.extend(spec.references());
            }
        }
        refs
    }
    pub fn apply(&mut self, command: WorkCommand) -> Result<WorkReceipt, WorkError> {
        if let Some((index, old)) = self
            .history
            .iter()
            .enumerate()
            .find(|(_, c)| c.id == command.id)
        {
            return if old == &command {
                Ok(WorkReceipt {
                    revision: Revision(index as u64 + 1),
                    replayed: true,
                })
            } else {
                Err(WorkError::IdempotencyConflict)
            };
        }
        if command.expected_revision != self.revision {
            return Err(WorkError::RevisionConflict);
        }
        if self.history.len() >= 128 {
            return Err(WorkError::ResourceLimit);
        }
        if command.at.0 < self.history.last().map_or(self.created_at.0, |c| c.at.0) {
            return Err(WorkError::InvalidTimestamp);
        }
        if matches!(command.actor, Actor::Worker(_)) {
            return Err(WorkError::PermissionDenied);
        }
        let next = match &command.action {
            WorkAction::Revise { spec } => {
                if !matches!(
                    self.state,
                    WorkState::Draft
                        | WorkState::Clarifying
                        | WorkState::AwaitingApproval
                        | WorkState::Approved
                ) {
                    return Err(WorkError::IllegalTransition);
                }
                spec.validate()?;
                if spec.id != self.spec.id
                    || spec.project_id != self.spec.project_id
                    || spec.role_id != self.spec.role_id
                {
                    return Err(WorkError::IdentityChanged);
                }
                WorkState::Draft
            }
            WorkAction::Clarify { question }
                if matches!(self.state, WorkState::Draft | WorkState::Clarifying) =>
            {
                if !bounded_text(question, 2048, true) {
                    return Err(WorkError::InvalidSpec);
                }
                WorkState::Clarifying
            }
            WorkAction::RequestApproval {}
                if matches!(self.state, WorkState::Draft | WorkState::Clarifying) =>
            {
                WorkState::AwaitingApproval
            }
            WorkAction::Approve {} if self.state == WorkState::AwaitingApproval => {
                if !matches!(command.actor, Actor::User(_)) {
                    return Err(WorkError::PermissionDenied);
                }
                WorkState::Approved
            }
            WorkAction::Start {} if self.state == WorkState::Approved => WorkState::Running,
            WorkAction::RequestCompletion {} if self.state == WorkState::Running => {
                WorkState::CompletionRequested
            }
            WorkAction::Complete { evidence } if self.state == WorkState::CompletionRequested => {
                let Actor::Host(host) = &command.actor else {
                    return Err(WorkError::PermissionDenied);
                };
                self.validate_evidence(evidence, host, command.at)?;
                WorkState::Completed
            }
            WorkAction::Cancel { reason }
                if !matches!(self.state, WorkState::Completed | WorkState::Cancelled) =>
            {
                if !bounded_text(reason, 2048, true) {
                    return Err(WorkError::InvalidSpec);
                }
                WorkState::Cancelled
            }
            WorkAction::Reopen { reason }
                if matches!(self.state, WorkState::Completed | WorkState::Cancelled) =>
            {
                if !bounded_text(reason, 2048, true) {
                    return Err(WorkError::InvalidSpec);
                }
                WorkState::Draft
            }
            _ => return Err(WorkError::IllegalTransition),
        };
        let mut candidate = self.clone();
        if let WorkAction::Revise { spec } = &command.action {
            candidate.spec = *spec.clone();
        }
        candidate.state = next;
        candidate.revision = Revision(self.revision.0 + 1);
        candidate.history.push(command);
        serde_json::to_writer(WorkSize(0, MAX_WORK_BYTES), &candidate)
            .map_err(|_| WorkError::ResourceLimit)?;
        *self = candidate;
        Ok(WorkReceipt {
            revision: self.revision,
            replayed: false,
        })
    }
    fn validate_evidence(
        &self,
        evidence: &WorkCompletionEvidence,
        host: &HostId,
        at: Timestamp,
    ) -> Result<(), WorkError> {
        if evidence.work != self.spec.reference()
            || evidence.revision != self.revision
            || &evidence.verified_by != host
            || evidence.verified_at.0 > at.0
            || evidence.verified_at.0 < self.history.last().map_or(self.created_at.0, |c| c.at.0)
            || self.spec.acceptance.is_empty()
            || evidence.acceptance.len() != self.spec.acceptance.len()
            || evidence.gates.len() != Gate::required().len()
        {
            return Err(WorkError::InvalidEvidence);
        }
        let mut ids = BTreeSet::new();
        let mut criteria = BTreeSet::new();
        let mut gates = BTreeSet::new();
        for item in &evidence.acceptance {
            if item.outcome != VerificationOutcome::Passed
                || usize::from(item.criterion) >= self.spec.acceptance.len()
                || !criteria.insert(item.criterion)
                || !ids.insert(&item.id)
            {
                return Err(WorkError::InvalidEvidence);
            }
        }
        for item in &evidence.gates {
            if item.outcome != VerificationOutcome::Passed
                || !gates.insert(item.gate)
                || !ids.insert(&item.id)
                || (item.gate == Gate::IndependentReview
                    && item.reviewer.as_ref().is_none_or(|r| r == self.role_id()))
            {
                return Err(WorkError::InvalidEvidence);
            }
        }
        if gates != Gate::required() {
            return Err(WorkError::InvalidEvidence);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    content = "work",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum TaskOrigin {
    Capability(WorkRef),
    Objective(WorkRef),
}
impl TaskOrigin {
    pub fn reference(&self) -> &WorkRef {
        self.work_ref()
    }
    pub fn work_ref(&self) -> &WorkRef {
        match self {
            Self::Capability(r) | Self::Objective(r) => r,
        }
    }
    pub fn validate_target(&self, item: &WorkItem) -> Result<(), WorkError> {
        if self.work_ref() != &item.spec.reference() {
            return Err(WorkError::InvalidTaskOrigin);
        }
        match self {
            Self::Capability(_) if matches!(item.id(), WorkId::Capability(_)) => Ok(()),
            Self::Objective(_)
                if matches!(item.id(), WorkId::Objective(_))
                    && matches!(
                        item.spec.objective_class,
                        Some(ObjectiveClass::Maintenance | ObjectiveClass::Operational)
                    ) =>
            {
                Ok(())
            }
            _ => Err(WorkError::InvalidTaskOrigin),
        }
    }
}

/// Validates current parent/dependency edges; historical edges remain observational.
/// Cross-Project references require explicit authorization by the Host before use.
pub fn validate_work_graph(items: &[WorkItem]) -> Result<(), WorkError> {
    if items.len() > 4096 {
        return Err(WorkError::ResourceLimit);
    }
    let mut index = BTreeMap::new();
    for (n, item) in items.iter().enumerate() {
        item.spec.validate()?;
        if index.insert(item.spec.reference(), n).is_some() {
            return Err(WorkError::InvalidSpec);
        }
    }
    let mut indegree = vec![0_usize; items.len()];
    let mut reverse = vec![Vec::new(); items.len()];
    for (n, item) in items.iter().enumerate() {
        let parent = item
            .spec
            .parent
            .as_ref()
            .map(|p| index.get(p).copied().ok_or(WorkError::MissingReference))
            .transpose()?;
        let valid = matches!(
            (item.id(), parent.map(|p| items[p].id())),
            (WorkId::Request(_), None)
                | (WorkId::Objective(_), None | Some(WorkId::Request(_)))
                | (
                    WorkId::Capability(_),
                    Some(WorkId::Request(_) | WorkId::Objective(_))
                )
                | (
                    WorkId::Plan(_),
                    Some(WorkId::Capability(_) | WorkId::Objective(_))
                )
                | (
                    WorkId::Milestone(_),
                    Some(WorkId::Plan(_) | WorkId::Capability(_) | WorkId::Objective(_)),
                )
                | (
                    WorkId::Checkpoint(_),
                    Some(WorkId::Plan(_) | WorkId::Milestone(_))
                )
                | (WorkId::FollowUp(_), Some(_))
        );
        if !valid {
            return Err(WorkError::ParentKindMismatch);
        }
        for reference in item.spec.references() {
            let target = *index.get(&reference).ok_or(WorkError::MissingReference)?;
            indegree[n] += 1;
            reverse[target].push(n);
        }
    }
    let mut ready: Vec<_> = indegree
        .iter()
        .enumerate()
        .filter_map(|(n, d)| (*d == 0).then_some(n))
        .collect();
    let mut visited = 0;
    while let Some(n) = ready.pop() {
        visited += 1;
        for target in &reverse[n] {
            indegree[*target] -= 1;
            if indegree[*target] == 0 {
                ready.push(*target);
            }
        }
    }
    if visited != items.len() {
        return Err(WorkError::Cycle);
    }
    Ok(())
}
