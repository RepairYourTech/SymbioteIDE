//! Normalized runtime observations. Nothing in this module completes canonical work.
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt;
use symbiote_domain::{CommandId, DispatchId, HostId, RequestId, RuntimeKind, SessionId};

pub const MAX_EVENT_TEXT_BYTES: usize = 16_384;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct EventText(String);
impl JsonSchema for EventText {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "EventText".into()
    }
    fn json_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
        schemars::json_schema!({"type":"string","maxLength":16384,"description":"At most 16384 UTF-8 bytes; runtime also enforces this stricter byte limit."})
    }
}
impl EventText {
    pub fn new(text: impl Into<String>) -> Result<Self, EventError> {
        let text = text.into();
        if text.len() > MAX_EVENT_TEXT_BYTES {
            return Err(EventError::TextTooLarge);
        }
        Ok(Self(text))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl TryFrom<String> for EventText {
    type Error = EventError;
    fn try_from(text: String) -> Result<Self, Self::Error> {
        Self::new(text)
    }
}
impl From<EventText> for String {
    fn from(text: EventText) -> Self {
        text.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SessionBinding {
    pub session_id: SessionId,
    pub dispatch_id: DispatchId,
    pub host_id: HostId,
    pub runtime: RuntimeKind,
}

/// Absence and a measured zero have different wire representations.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum UsageMeasurement {
    Unknown { reason: UnknownUsageReason },
    Measured { value: u64 },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum UnknownUsageReason {
    NotReported,
    Unsupported,
    PartialVisibility,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum UsageCoverage {
    RootOnly,
    RootAndChildren,
    AggregateOnly,
    Unknown,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RuntimeUsage {
    pub input_tokens: UsageMeasurement,
    pub output_tokens: UsageMeasurement,
    pub cached_input_tokens: UsageMeasurement,
    pub billed_micro_units: UsageMeasurement,
    pub coverage: UsageCoverage,
}
impl RuntimeUsage {
    pub fn validate(&self) -> Result<(), EventError> {
        if let (
            UsageMeasurement::Measured { value: input },
            UsageMeasurement::Measured { value: cached },
        ) = (&self.input_tokens, &self.cached_input_tokens)
        {
            if cached > input {
                return Err(EventError::InvalidUsage);
            }
        }
        if let (
            UsageMeasurement::Measured { value: input },
            UsageMeasurement::Measured { value: output },
        ) = (&self.input_tokens, &self.output_tokens)
        {
            if input.checked_add(*output).is_none() {
                return Err(EventError::InvalidUsage);
            }
        }
        Ok(())
    }
}
impl<'de> Deserialize<'de> for RuntimeUsage {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Wire {
            input_tokens: UsageMeasurement,
            output_tokens: UsageMeasurement,
            cached_input_tokens: UsageMeasurement,
            billed_micro_units: UsageMeasurement,
            coverage: UsageCoverage,
        }
        let wire = Wire::deserialize(deserializer)?;
        let usage = Self {
            input_tokens: wire.input_tokens,
            output_tokens: wire.output_tokens,
            cached_input_tokens: wire.cached_input_tokens,
            billed_micro_units: wire.billed_micro_units,
            coverage: wire.coverage,
        };
        usage.validate().map_err(serde::de::Error::custom)?;
        Ok(usage)
    }
}

/// Retain a namespaced reference to separately governed/redacted vendor evidence,
/// not an unbounded raw payload or a copy of authentication material.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct VendorEventReference {
    pub namespace: EventText,
    pub id: RequestId,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ObservationKind {
    FilesystemMutation,
    ShellProcess,
    Git,
    Network,
    McpAccess,
    SecretAccess,
    PostEdit,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum RuntimeEventKind {
    Ready {},
    Message {
        text: EventText,
    },
    ToolProposed {
        tool_call_id: RequestId,
        name: EventText,
        arguments: EventText,
    },
    ToolStarted {
        tool_call_id: RequestId,
    },
    ToolCompleted {
        tool_call_id: RequestId,
        output: EventText,
    },
    ToolFailed {
        tool_call_id: RequestId,
        error: EventText,
    },
    ApprovalRequested {
        request_id: RequestId,
        prompt: EventText,
    },
    WorkerQuestion {
        request_id: RequestId,
        prompt: EventText,
    },
    Observation {
        resource: ObservationKind,
        detail: EventText,
    },
    Diagnostic {
        message: EventText,
    },
    Usage {
        usage: RuntimeUsage,
    },
    CompletionRequested {
        report: EventText,
    },
    Exit {
        code: Option<i32>,
    },
    Disconnected {
        reason: EventText,
    },
    Reconnected {},
    Crashed {
        reason: EventText,
    },
    CancellationRequested {
        reason: EventText,
    },
    CancelAcknowledged {},
}
impl RuntimeEventKind {
    fn is_mutation_observation(&self) -> bool {
        matches!(
            self,
            Self::ToolProposed { .. }
                | Self::ToolStarted { .. }
                | Self::ToolCompleted { .. }
                | Self::ToolFailed { .. }
                | Self::Observation { .. }
        )
    }
}

/// The adapter assigns positive contiguous sequence numbers per immutable binding.
/// This envelope proves structure, not that the sender is an authenticated adapter.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RuntimeEvent {
    id: CommandId,
    #[schemars(range(min = 1))]
    sequence: u64,
    binding: SessionBinding,
    payload: RuntimeEventKind,
    vendor: Option<VendorEventReference>,
}
impl RuntimeEvent {
    pub fn new(
        id: CommandId,
        sequence: u64,
        binding: SessionBinding,
        payload: RuntimeEventKind,
    ) -> Result<Self, EventError> {
        if sequence == 0 {
            return Err(EventError::InvalidSequence);
        }
        if let RuntimeEventKind::Usage { usage } = &payload {
            usage.validate()?;
        }
        if matches!(&payload, RuntimeEventKind::ToolProposed { name, .. } if name.as_str().trim().is_empty())
        {
            return Err(EventError::EmptyToolName);
        }
        Ok(Self {
            id,
            sequence,
            binding,
            payload,
            vendor: None,
        })
    }
    pub fn with_vendor_reference(
        mut self,
        vendor: VendorEventReference,
    ) -> Result<Self, EventError> {
        if vendor.namespace.as_str().trim().is_empty() {
            return Err(EventError::InvalidVendorReference);
        }
        self.vendor = Some(vendor);
        Ok(self)
    }
    pub fn id(&self) -> &CommandId {
        &self.id
    }
    pub fn sequence(&self) -> u64 {
        self.sequence
    }
    pub fn binding(&self) -> &SessionBinding {
        &self.binding
    }
    pub fn payload(&self) -> &RuntimeEventKind {
        &self.payload
    }
    pub fn vendor_reference(&self) -> Option<&VendorEventReference> {
        self.vendor.as_ref()
    }
}
impl<'de> Deserialize<'de> for RuntimeEvent {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Wire {
            id: CommandId,
            sequence: u64,
            binding: SessionBinding,
            payload: RuntimeEventKind,
            vendor: Option<VendorEventReference>,
        }
        let wire = Wire::deserialize(deserializer)?;
        let mut event = Self::new(wire.id, wire.sequence, wire.binding, wire.payload)
            .map_err(serde::de::Error::custom)?;
        if let Some(vendor) = wire.vendor {
            event = event
                .with_vendor_reference(vendor)
                .map_err(serde::de::Error::custom)?;
        }
        Ok(event)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SessionState {
    AwaitingReady,
    Running,
    Disconnected,
    Cancelling,
    Cancelled,
    Exited,
    Crashed,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ResidualEffects {
    Unassessed,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EventReceipt {
    pub sequence: u64,
    pub state: SessionState,
    pub replayed: bool,
    pub residual_effects: ResidualEffects,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TrackerLimits {
    pub replay_capacity: usize,
    pub max_tool_calls: usize,
    pub max_events: usize,
}
impl Default for TrackerLimits {
    fn default() -> Self {
        Self {
            replay_capacity: 128,
            max_tool_calls: 256,
            max_events: 4096,
        }
    }
}
impl TrackerLimits {
    fn validate(self) -> Result<Self, EventError> {
        if !(1..=1024).contains(&self.replay_capacity)
            || !(1..=256).contains(&self.max_tool_calls)
            || !(1..=65536).contains(&self.max_events)
            || self.replay_capacity > self.max_events
        {
            return Err(EventError::InvalidLimits);
        }
        Ok(self)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "error", rename_all = "snake_case", deny_unknown_fields)]
pub enum EventError {
    TextTooLarge,
    InvalidUsage,
    EmptyToolName,
    InvalidVendorReference,
    InvalidSequence,
    BindingMismatch,
    IdempotencyConflict,
    SequenceConflict,
    SequenceGap { expected: u64, received: u64 },
    ReplayUnavailable { floor: u64 },
    InvalidTransition,
    UnknownTool,
    ToolAlreadyExists,
    ToolTransitionRejected,
    CapacityExceeded,
    InvalidLimits,
    ResidualEffectUnknown,
}
impl fmt::Display for EventError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for EventError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
enum ToolState {
    Proposed,
    Started,
    Completed,
    Failed,
}
#[derive(Clone, Debug, Serialize)]
struct RetainedEvent {
    event: RuntimeEvent,
    receipt: EventReceipt,
}

/// Serialize for inspection only; deliberately no Deserialize authority shortcut.
/// A new tracker must consume its event stream in order. When history has expired,
/// recovery requires an authoritative adapter/store handoff, not a guessed replay.
#[derive(Clone, Debug, Serialize)]
pub struct SessionTracker {
    binding: SessionBinding,
    limits: TrackerLimits,
    state: SessionState,
    last_sequence: u64,
    cancellation_requested: bool,
    before_disconnect: Option<SessionState>,
    residual_effects: ResidualEffects,
    history: VecDeque<RetainedEvent>,
    seen_ids: BTreeSet<CommandId>,
    tools: BTreeMap<RequestId, ToolState>,
}
impl SessionTracker {
    pub fn new(binding: SessionBinding, limits: TrackerLimits) -> Result<Self, EventError> {
        Ok(Self {
            binding,
            limits: limits.validate()?,
            state: SessionState::AwaitingReady,
            last_sequence: 0,
            cancellation_requested: false,
            before_disconnect: None,
            residual_effects: ResidualEffects::Unassessed,
            history: VecDeque::new(),
            seen_ids: BTreeSet::new(),
            tools: BTreeMap::new(),
        })
    }
    pub fn binding(&self) -> &SessionBinding {
        &self.binding
    }
    pub fn state(&self) -> SessionState {
        self.state
    }
    pub fn next_sequence(&self) -> u64 {
        self.last_sequence + 1
    }
    pub fn replay_floor(&self) -> u64 {
        self.history.front().map_or(1, |entry| entry.event.sequence)
    }
    pub fn residual_effects(&self) -> ResidualEffects {
        self.residual_effects
    }
    pub fn retained_event_count(&self) -> usize {
        self.history.len()
    }
    pub fn tracked_tool_count(&self) -> usize {
        self.tools.len()
    }

    pub fn apply(&mut self, event: RuntimeEvent) -> Result<EventReceipt, EventError> {
        if event.binding != self.binding {
            return Err(EventError::BindingMismatch);
        }
        if event.sequence < self.replay_floor() {
            return Err(EventError::ReplayUnavailable {
                floor: self.replay_floor(),
            });
        }
        if let Some(previous) = self.history.iter().find(|entry| entry.event.id == event.id) {
            return if previous.event == event {
                let mut receipt = previous.receipt.clone();
                receipt.replayed = true;
                Ok(receipt)
            } else {
                Err(EventError::IdempotencyConflict)
            };
        }
        if self.seen_ids.contains(&event.id) {
            return Err(EventError::IdempotencyConflict);
        }
        if event.sequence <= self.last_sequence {
            return Err(EventError::SequenceConflict);
        }
        if event.sequence != self.next_sequence() {
            return Err(EventError::SequenceGap {
                expected: self.next_sequence(),
                received: event.sequence,
            });
        }
        if self.cancellation_requested && event.payload.is_mutation_observation() {
            self.residual_effects = ResidualEffects::Unknown;
            return Err(EventError::ResidualEffectUnknown);
        }
        if self.seen_ids.len() >= self.limits.max_events {
            return Err(EventError::CapacityExceeded);
        }
        self.transition(&event.payload)?;
        self.last_sequence = event.sequence;
        self.seen_ids.insert(event.id.clone());
        let receipt = EventReceipt {
            sequence: event.sequence,
            state: self.state,
            replayed: false,
            residual_effects: self.residual_effects,
        };
        if self.history.len() == self.limits.replay_capacity {
            self.history.pop_front();
        }
        self.history.push_back(RetainedEvent {
            event,
            receipt: receipt.clone(),
        });
        Ok(receipt)
    }

    fn transition(&mut self, payload: &RuntimeEventKind) -> Result<(), EventError> {
        use RuntimeEventKind as E;
        match payload {
            E::Usage { .. } | E::Diagnostic { .. } => {}
            E::Ready {} => {
                if self.state != SessionState::AwaitingReady {
                    return Err(EventError::InvalidTransition);
                }
                self.state = SessionState::Running;
            }
            E::Message { .. }
            | E::ApprovalRequested { .. }
            | E::WorkerQuestion { .. }
            | E::Observation { .. }
            | E::CompletionRequested { .. } => {
                self.require_running()?;
            }
            E::ToolProposed { tool_call_id, .. } => {
                self.require_running()?;
                if self.tools.contains_key(tool_call_id) {
                    return Err(EventError::ToolAlreadyExists);
                }
                if self.tools.len() >= self.limits.max_tool_calls {
                    return Err(EventError::CapacityExceeded);
                }
                self.tools.insert(tool_call_id.clone(), ToolState::Proposed);
            }
            E::ToolStarted { tool_call_id } => {
                self.tool_transition(tool_call_id, ToolState::Proposed, ToolState::Started)?;
            }
            E::ToolCompleted { tool_call_id, .. } => {
                self.tool_transition(tool_call_id, ToolState::Started, ToolState::Completed)?;
            }
            E::ToolFailed { tool_call_id, .. } => {
                self.tool_transition(tool_call_id, ToolState::Started, ToolState::Failed)?;
            }
            E::Disconnected { .. } => {
                if !matches!(
                    self.state,
                    SessionState::AwaitingReady | SessionState::Running | SessionState::Cancelling
                ) {
                    return Err(EventError::InvalidTransition);
                }
                self.residual_effects = ResidualEffects::Unknown;
                self.before_disconnect = Some(self.state);
                self.state = SessionState::Disconnected;
            }
            E::Reconnected {} => {
                if self.state != SessionState::Disconnected {
                    return Err(EventError::InvalidTransition);
                }
                self.state = if self.cancellation_requested {
                    SessionState::Cancelling
                } else {
                    self.before_disconnect
                        .unwrap_or(SessionState::AwaitingReady)
                };
                self.before_disconnect = None;
            }
            E::CancellationRequested { .. } => {
                if !matches!(
                    self.state,
                    SessionState::AwaitingReady
                        | SessionState::Running
                        | SessionState::Disconnected
                ) {
                    return Err(EventError::InvalidTransition);
                }
                self.cancellation_requested = true;
                self.residual_effects = ResidualEffects::Unknown;
                if self.state != SessionState::Disconnected {
                    self.state = SessionState::Cancelling;
                }
            }
            E::CancelAcknowledged {} => {
                if !self.cancellation_requested {
                    return Err(EventError::InvalidTransition);
                }
                self.state = SessionState::Cancelled;
            }
            E::Exit { .. } | E::Crashed { .. } => {
                if matches!(self.state, SessionState::Exited | SessionState::Crashed) {
                    return Err(EventError::InvalidTransition);
                }
                if matches!(payload, E::Crashed { .. })
                    || self
                        .tools
                        .values()
                        .any(|state| *state == ToolState::Started)
                {
                    self.residual_effects = ResidualEffects::Unknown;
                }
                self.state = if self.cancellation_requested {
                    SessionState::Cancelled
                } else if matches!(payload, E::Crashed { .. }) {
                    SessionState::Crashed
                } else {
                    SessionState::Exited
                };
            }
        }
        Ok(())
    }
    fn require_running(&self) -> Result<(), EventError> {
        if self.state == SessionState::Running {
            Ok(())
        } else {
            Err(EventError::InvalidTransition)
        }
    }
    fn tool_transition(
        &mut self,
        id: &RequestId,
        expected: ToolState,
        next: ToolState,
    ) -> Result<(), EventError> {
        self.require_running()?;
        let state = self.tools.get_mut(id).ok_or(EventError::UnknownTool)?;
        if *state != expected {
            return Err(EventError::ToolTransitionRejected);
        }
        *state = next;
        Ok(())
    }
}
