use crate::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DomainError {
    UnsupportedSchemaVersion,
    RevisionConflict,
    IdempotencyConflict,
    RevisionExhausted,
    IllegalTransition,
    PermissionDenied,
    LineageMismatch,
    IneligibleHost,
    UnsupportedControl,
    InvalidContextBudget,
    EvidenceMissing,
    EvidenceRejected,
    StaleEvidence,
    IndependentReviewRequired,
    StreamNotReady,
    InvalidTimestamp,
    InvalidStream,
    EmptyReport,
}
impl fmt::Display for DomainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for DomainError {}

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum Gate {
    Tests,
    Security,
    Impact,
    Documentation,
    IndependentReview,
    Delivery,
    CapabilityClosure,
}
impl Gate {
    pub fn required() -> BTreeSet<Self> {
        [
            Self::Tests,
            Self::Security,
            Self::Impact,
            Self::Documentation,
            Self::IndependentReview,
            Self::Delivery,
            Self::CapabilityClosure,
        ]
        .into_iter()
        .collect()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum VerificationOutcome {
    Passed,
    Failed,
    Inconclusive,
}

/// A verifier-produced claim whose authenticity must be checked by the Host. This
/// value alone grants no authority. Each gate binds to a run, artifact, dispatch and
/// exact source/target pair; a runtime-local success flag is not evidence.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct VerificationEvidence {
    pub id: EvidenceId,
    pub run_id: VerificationRunId,
    pub artifact_id: ArtifactId,
    pub project_id: ProjectId,
    pub task_id: TaskId,
    pub dispatch_id: DispatchId,
    pub stream_id: ChangeStreamId,
    pub gate: Gate,
    pub outcome: VerificationOutcome,
    pub head: CommitSha,
    pub target: CommitSha,
    pub verified_by: HostId,
    pub reviewer: Option<RoleId>,
    pub recorded_at: Timestamp,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TaskState {
    Ready,
    Running,
    CompletionRequested,
    Verifying,
    Completed,
    Failed,
    Cancelled,
    Interrupted,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Task {
    id: TaskId,
    project_id: ProjectId,
    root_id: RootId,
    role_id: RoleId,
    stream_id: ChangeStreamId,
    task_contract: VersionedTaskContract,
    revision: Revision,
    state: TaskState,
    dispatch: Option<Dispatch>,
    history: Vec<TaskEvent>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TaskAction {
    Start {
        dispatch: Box<Dispatch>,
    },
    RequestCompletion {
        dispatch_id: DispatchId,
        report: String,
    },
    BeginVerification,
    Complete {
        stream: Box<ChangeStream>,
        evidence: Vec<VerificationEvidence>,
    },
    Fail {
        reason: String,
    },
    Cancel {
        reason: String,
    },
    Interrupt {
        reason: String,
    },
    Recover,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TaskCommand {
    pub id: CommandId,
    pub expected_revision: Revision,
    pub actor: Actor,
    pub at: Timestamp,
    pub action: TaskAction,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TaskEvent {
    pub command: TaskCommand,
    pub resulting_revision: Revision,
    pub from: TaskState,
    pub to: TaskState,
}

impl Task {
    pub fn new(
        id: TaskId,
        project_id: ProjectId,
        root_id: RootId,
        role_id: RoleId,
        stream_id: ChangeStreamId,
        task_contract: VersionedTaskContract,
    ) -> Self {
        Self {
            id,
            project_id,
            root_id,
            role_id,
            stream_id,
            task_contract,
            revision: Revision(0),
            state: TaskState::Ready,
            dispatch: None,
            history: Vec::new(),
        }
    }
    pub fn id(&self) -> &TaskId {
        &self.id
    }
    pub fn project_id(&self) -> &ProjectId {
        &self.project_id
    }
    pub fn root_id(&self) -> &RootId {
        &self.root_id
    }
    pub fn role_id(&self) -> &RoleId {
        &self.role_id
    }
    pub fn stream_id(&self) -> &ChangeStreamId {
        &self.stream_id
    }
    pub fn revision(&self) -> Revision {
        self.revision
    }
    pub fn state(&self) -> &TaskState {
        &self.state
    }
    pub fn task_contract(&self) -> &VersionedTaskContract {
        &self.task_contract
    }
    pub fn history(&self) -> &[TaskEvent] {
        &self.history
    }

    /// A duplicate returns its original resulting revision, even after later work.
    /// Reusing a command ID with different bytes conflicts. Failed commands leave
    /// state untouched and are not recorded; durable rejection audit is Host-owned.
    pub fn apply(&mut self, command: TaskCommand) -> Result<Revision, DomainError> {
        if let Some(event) = self.history.iter().find(|e| e.command.id == command.id) {
            return if event.command == command {
                Ok(event.resulting_revision)
            } else {
                Err(DomainError::IdempotencyConflict)
            };
        }
        if command.expected_revision != self.revision {
            return Err(DomainError::RevisionConflict);
        }
        if self
            .history
            .last()
            .is_some_and(|e| command.at < e.command.at)
        {
            return Err(DomainError::InvalidTimestamp);
        }
        let next_revision = Revision(
            self.revision
                .0
                .checked_add(1)
                .ok_or(DomainError::RevisionExhausted)?,
        );
        let host_authorized = match (&command.actor, &self.dispatch) {
            (Actor::Host(host), Some(dispatch)) => host == dispatch.contract().host_id(),
            _ => false,
        };
        let next_state = match &command.action {
            TaskAction::Start { dispatch } => {
                if self.state != TaskState::Ready {
                    return Err(DomainError::IllegalTransition);
                }
                let contract = dispatch.contract();
                contract.validate_at(command.at)?;
                if command.actor != Actor::Host(contract.host_id().clone()) {
                    return Err(DomainError::PermissionDenied);
                }
                if contract.task_id() != &self.id
                    || contract.root_id() != &self.root_id
                    || contract.stream_id() != &self.stream_id
                    || contract.task_contract() != &self.task_contract
                    || contract.task_revision() != self.revision
                    || contract.role().id != self.role_id
                    || contract.role().project_id != self.project_id
                {
                    return Err(DomainError::LineageMismatch);
                }
                TaskState::Running
            }
            TaskAction::RequestCompletion {
                dispatch_id,
                report,
            } => {
                if self.state != TaskState::Running {
                    return Err(DomainError::IllegalTransition);
                }
                if self.dispatch.as_ref().map(Dispatch::id) != Some(dispatch_id)
                    || command.actor != Actor::Worker(dispatch_id.clone())
                {
                    return Err(DomainError::PermissionDenied);
                }
                if report.trim().is_empty() {
                    return Err(DomainError::EmptyReport);
                }
                TaskState::CompletionRequested
            }
            TaskAction::BeginVerification => {
                if !host_authorized {
                    return Err(DomainError::PermissionDenied);
                }
                if self.state != TaskState::CompletionRequested {
                    return Err(DomainError::IllegalTransition);
                }
                TaskState::Verifying
            }
            TaskAction::Complete { stream, evidence } => {
                if !host_authorized {
                    return Err(DomainError::PermissionDenied);
                }
                if self.state != TaskState::Verifying {
                    return Err(DomainError::IllegalTransition);
                }
                self.validate_evidence(stream, evidence, command.at)?;
                TaskState::Completed
            }
            TaskAction::Fail { reason }
            | TaskAction::Cancel { reason }
            | TaskAction::Interrupt { reason } => {
                if !host_authorized {
                    return Err(DomainError::PermissionDenied);
                }
                if !matches!(
                    self.state,
                    TaskState::Running | TaskState::CompletionRequested | TaskState::Verifying
                ) {
                    return Err(DomainError::IllegalTransition);
                }
                if reason.trim().is_empty() {
                    return Err(DomainError::EmptyReport);
                }
                match &command.action {
                    TaskAction::Fail { .. } => TaskState::Failed,
                    TaskAction::Cancel { .. } => TaskState::Cancelled,
                    _ => TaskState::Interrupted,
                }
            }
            TaskAction::Recover => {
                if !host_authorized {
                    return Err(DomainError::PermissionDenied);
                }
                if !matches!(self.state, TaskState::Failed | TaskState::Interrupted) {
                    return Err(DomainError::IllegalTransition);
                }
                TaskState::Ready
            }
        };
        if let TaskAction::Start { dispatch } = &command.action {
            self.dispatch = Some(*dispatch.clone());
        }
        if matches!(command.action, TaskAction::Recover) {
            self.dispatch = None;
        }
        let from = std::mem::replace(&mut self.state, next_state.clone());
        self.revision = next_revision;
        self.history.push(TaskEvent {
            command,
            resulting_revision: next_revision,
            from,
            to: next_state,
        });
        Ok(next_revision)
    }

    fn validate_evidence(
        &self,
        stream: &ChangeStream,
        evidence: &[VerificationEvidence],
        at: Timestamp,
    ) -> Result<(), DomainError> {
        let dispatch = self
            .dispatch
            .as_ref()
            .ok_or(DomainError::IllegalTransition)?;
        if stream.id != self.stream_id
            || stream.project_id != self.project_id
            || stream.root_id != self.root_id
            || !stream.tasks.contains(&self.id)
        {
            return Err(DomainError::LineageMismatch);
        }
        if stream.state != StreamState::Validated
            || stream.last_validated_target.as_ref() != Some(&stream.target)
            || stream.last_validated_head.as_ref() != Some(&stream.head)
        {
            return Err(DomainError::StreamNotReady);
        }
        let mut covered = BTreeSet::new();
        let mut ids = BTreeSet::new();
        for item in evidence {
            if item.task_id != self.id
                || item.project_id != self.project_id
                || item.stream_id != self.stream_id
                || &item.dispatch_id != dispatch.id()
                || &item.verified_by != dispatch.contract().host_id()
            {
                return Err(DomainError::LineageMismatch);
            }
            if item.head != stream.head || item.target != stream.target {
                return Err(DomainError::StaleEvidence);
            }
            if item.recorded_at > at
                || self
                    .history
                    .last()
                    .is_some_and(|e| item.recorded_at < e.command.at)
            {
                return Err(DomainError::InvalidTimestamp);
            }
            if item.outcome != VerificationOutcome::Passed
                || !ids.insert(item.id.clone())
                || !covered.insert(item.gate)
            {
                return Err(DomainError::EvidenceRejected);
            }
            if item.gate == Gate::IndependentReview
                && (item.reviewer.is_none() || item.reviewer.as_ref() == Some(&self.role_id))
            {
                return Err(DomainError::IndependentReviewRequired);
            }
        }
        if covered != *dispatch.contract().gates() {
            return Err(DomainError::EvidenceMissing);
        }
        Ok(())
    }
}

impl<'de> Deserialize<'de> for Task {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Wire {
            id: TaskId,
            project_id: ProjectId,
            root_id: RootId,
            role_id: RoleId,
            stream_id: ChangeStreamId,
            task_contract: VersionedTaskContract,
            revision: Revision,
            state: TaskState,
            dispatch: Option<Dispatch>,
            history: Vec<TaskEvent>,
        }
        let w = Wire::deserialize(deserializer)?;
        let mut task = Self::new(
            w.id,
            w.project_id,
            w.root_id,
            w.role_id,
            w.stream_id,
            w.task_contract,
        );
        for event in &w.history {
            if task.state != event.from {
                return Err(serde::de::Error::custom(
                    "event source state does not match replay",
                ));
            }
            let revision = task
                .apply(event.command.clone())
                .map_err(serde::de::Error::custom)?;
            if revision != event.resulting_revision || task.state != event.to {
                return Err(serde::de::Error::custom(
                    "event outcome does not match replay",
                ));
            }
        }
        if task.revision != w.revision
            || task.state != w.state
            || task.dispatch != w.dispatch
            || task.history != w.history
        {
            return Err(serde::de::Error::custom(
                "task snapshot does not match replay",
            ));
        }
        Ok(task)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum StreamLineage {
    Independent,
    Stacked {
        parent: ChangeStreamId,
        parent_head: CommitSha,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum StreamState {
    Active,
    NeedsRevalidation,
    Collided,
    Validated,
    Integrated,
    Cancelled,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ChangeStream {
    id: ChangeStreamId,
    project_id: ProjectId,
    root_id: RootId,
    tasks: BTreeSet<TaskId>,
    pub originating_chat: ChatId,
    pub sessions: BTreeSet<SessionId>,
    pub objective: Option<ObjectiveId>,
    pub worktree: WorktreeId,
    pub branch: String,
    lineage: StreamLineage,
    base: CommitSha,
    head: CommitSha,
    target: CommitSha,
    initial_target: CommitSha,
    last_validated_target: Option<CommitSha>,
    last_validated_head: Option<CommitSha>,
    state: StreamState,
    revision: Revision,
    history: Vec<StreamEvent>,
}

pub struct NewChangeStream {
    pub id: ChangeStreamId,
    pub project_id: ProjectId,
    pub root_id: RootId,
    pub tasks: BTreeSet<TaskId>,
    pub originating_chat: ChatId,
    pub worktree: WorktreeId,
    pub branch: String,
    pub lineage: StreamLineage,
    pub base: CommitSha,
    pub target: CommitSha,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum StreamAction {
    Advance {
        head: CommitSha,
        target: CommitSha,
    },
    Collision {
        other: ChangeStreamId,
    },
    Reconcile {
        head: CommitSha,
        target: CommitSha,
    },
    Validate {
        head: CommitSha,
        target: CommitSha,
        run: VerificationRunId,
    },
    Integrate {
        head: CommitSha,
        target: CommitSha,
    },
    Cancel,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StreamCommand {
    pub id: CommandId,
    pub expected_revision: Revision,
    pub host_id: HostId,
    pub at: Timestamp,
    pub action: StreamAction,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StreamEvent {
    pub command: StreamCommand,
    pub resulting_revision: Revision,
}

impl<'de> Deserialize<'de> for ChangeStream {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Wire {
            id: ChangeStreamId,
            project_id: ProjectId,
            root_id: RootId,
            tasks: BTreeSet<TaskId>,
            originating_chat: ChatId,
            sessions: BTreeSet<SessionId>,
            objective: Option<ObjectiveId>,
            worktree: WorktreeId,
            branch: String,
            lineage: StreamLineage,
            base: CommitSha,
            head: CommitSha,
            target: CommitSha,
            initial_target: CommitSha,
            last_validated_target: Option<CommitSha>,
            last_validated_head: Option<CommitSha>,
            state: StreamState,
            revision: Revision,
            history: Vec<StreamEvent>,
        }
        let w = Wire::deserialize(deserializer)?;
        let access = AccessSnapshot {
            project_id: w.project_id.clone(),
            roots: BTreeSet::from([w.root_id.clone()]),
            grants: BTreeSet::from([Permission::MutateStream]),
            policy_revision: Revision(0),
        };
        let mut stream = Self::new(NewChangeStream {
            id: w.id,
            project_id: w.project_id,
            root_id: w.root_id,
            tasks: w.tasks,
            originating_chat: w.originating_chat,
            worktree: w.worktree,
            branch: w.branch,
            lineage: w.lineage,
            base: w.base,
            target: w.initial_target,
        })
        .map_err(serde::de::Error::custom)?;
        stream.sessions = w.sessions;
        stream.objective = w.objective;
        for event in &w.history {
            if stream
                .apply(event.command.clone(), &access)
                .map_err(serde::de::Error::custom)?
                != event.resulting_revision
            {
                return Err(serde::de::Error::custom(
                    "stream event revision does not match replay",
                ));
            }
        }
        if stream.revision != w.revision
            || stream.state != w.state
            || stream.head != w.head
            || stream.target != w.target
            || stream.last_validated_head != w.last_validated_head
            || stream.last_validated_target != w.last_validated_target
            || stream.history != w.history
        {
            return Err(serde::de::Error::custom(
                "stream snapshot does not match replay",
            ));
        }
        Ok(stream)
    }
}

impl ChangeStream {
    pub fn new(input: NewChangeStream) -> Result<Self, DomainError> {
        if input.branch.trim().is_empty()
            || input.tasks.is_empty()
            || matches!(&input.lineage, StreamLineage::Stacked { parent, .. } if parent == &input.id)
        {
            return Err(DomainError::InvalidStream);
        }
        Ok(Self {
            id: input.id,
            project_id: input.project_id,
            root_id: input.root_id,
            tasks: input.tasks,
            originating_chat: input.originating_chat,
            sessions: BTreeSet::new(),
            objective: None,
            worktree: input.worktree,
            branch: input.branch,
            lineage: input.lineage,
            head: input.base.clone(),
            base: input.base,
            initial_target: input.target.clone(),
            target: input.target,
            last_validated_target: None,
            last_validated_head: None,
            state: StreamState::Active,
            revision: Revision(0),
            history: Vec::new(),
        })
    }
    pub fn id(&self) -> &ChangeStreamId {
        &self.id
    }
    pub fn project_id(&self) -> &ProjectId {
        &self.project_id
    }
    pub fn root_id(&self) -> &RootId {
        &self.root_id
    }
    pub fn tasks(&self) -> &BTreeSet<TaskId> {
        &self.tasks
    }
    pub fn head(&self) -> &CommitSha {
        &self.head
    }
    pub fn target(&self) -> &CommitSha {
        &self.target
    }
    pub fn state(&self) -> &StreamState {
        &self.state
    }
    pub fn revision(&self) -> Revision {
        self.revision
    }
    pub fn lineage(&self) -> &StreamLineage {
        &self.lineage
    }
    pub fn history(&self) -> &[StreamEvent] {
        &self.history
    }

    /// Permission snapshot must come from the authenticated Host. This pure layer
    /// does not prove filesystem isolation or authenticate the caller/verification run.
    pub fn apply(
        &mut self,
        command: StreamCommand,
        access: &AccessSnapshot,
    ) -> Result<Revision, DomainError> {
        if access.project_id != self.project_id
            || !access.roots.contains(&self.root_id)
            || !access.grants.contains(&Permission::MutateStream)
        {
            return Err(DomainError::PermissionDenied);
        }
        if let Some(event) = self.history.iter().find(|e| e.command.id == command.id) {
            return if event.command == command {
                Ok(event.resulting_revision)
            } else {
                Err(DomainError::IdempotencyConflict)
            };
        }
        if self.revision != command.expected_revision {
            return Err(DomainError::RevisionConflict);
        }
        if self
            .history
            .last()
            .is_some_and(|e| command.at < e.command.at)
        {
            return Err(DomainError::InvalidTimestamp);
        }
        if matches!(self.state, StreamState::Integrated | StreamState::Cancelled) {
            return Err(DomainError::IllegalTransition);
        }
        let revision = Revision(
            self.revision
                .0
                .checked_add(1)
                .ok_or(DomainError::RevisionExhausted)?,
        );
        match &command.action {
            StreamAction::Advance { head, target } | StreamAction::Reconcile { head, target } => {
                if self.state == StreamState::Collided
                    && matches!(&command.action, StreamAction::Advance { .. })
                {
                    return Err(DomainError::IllegalTransition);
                }
                if self.state != StreamState::Collided
                    && matches!(&command.action, StreamAction::Reconcile { .. })
                {
                    return Err(DomainError::IllegalTransition);
                }
                self.head = head.clone();
                self.target = target.clone();
                self.state = StreamState::NeedsRevalidation;
                self.last_validated_head = None;
                self.last_validated_target = None;
            }
            StreamAction::Collision { other } => {
                if other == &self.id {
                    return Err(DomainError::InvalidStream);
                }
                self.state = StreamState::Collided;
                self.last_validated_head = None;
                self.last_validated_target = None;
            }
            StreamAction::Validate { head, target, .. } => {
                if self.state == StreamState::Collided {
                    return Err(DomainError::StreamNotReady);
                }
                if head != &self.head || target != &self.target {
                    return Err(DomainError::StaleEvidence);
                }
                self.last_validated_head = Some(head.clone());
                self.last_validated_target = Some(target.clone());
                self.state = StreamState::Validated;
            }
            StreamAction::Integrate { head, target } => {
                if self.state != StreamState::Validated {
                    return Err(DomainError::StreamNotReady);
                }
                if head != &self.head || target != &self.target {
                    return Err(DomainError::StaleEvidence);
                }
                self.state = StreamState::Integrated;
            }
            StreamAction::Cancel => self.state = StreamState::Cancelled,
        }
        self.revision = revision;
        self.history.push(StreamEvent {
            command,
            resulting_revision: revision,
        });
        Ok(revision)
    }
}
