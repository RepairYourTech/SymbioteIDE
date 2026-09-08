use crate::{Capability, QualificationError, RuntimeDescriptor, qualify_dispatch};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use symbiote_domain::*;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AdapterOperation {
    Detect,
    Probe,
    PollEvents,
    Authenticate,
    DiscoverProfiles,
    ProjectConfiguration,
    Launch,
    Handshake,
    Send,
    Steer,
    Interrupt,
    Cancel,
    Resume,
    Fork,
    Health,
    Usage,
    Dispose,
    Revoke,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum AdapterError {
    Unsupported { operation: AdapterOperation },
    Qualification { reason: QualificationError },
    AuthorizationNotPersisted,
    ContractMismatch,
    InvalidInput,
    SessionMismatch,
    Unavailable,
    UnknownEffect,
}
impl std::fmt::Display for AdapterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for AdapterError {}
impl From<QualificationError> for AdapterError {
    fn from(reason: QualificationError) -> Self {
        Self::Qualification { reason }
    }
}

/// A Host journal receipt, never deserialized from an agent/tool event.
#[derive(Clone, Debug)]
pub struct ActivationReceipt {
    pub command_id: CommandId,
    pub revision: Revision,
    pub dispatch: Dispatch,
    pub descriptor: RuntimeDescriptor,
    pub authorized_at: Timestamp,
}

/// Implemented inside the trusted Host. Returning success means the exact
/// immutable dispatch and qualification facts are committed before activation.
pub trait ActivationJournal {
    fn persist_authorization(
        &mut self,
        dispatch: &Dispatch,
        descriptor: &RuntimeDescriptor,
        at: Timestamp,
    ) -> Result<ActivationReceipt, AdapterError>;
}

/// Preparation is data-only: no project hooks, code, providers or credentials run.
pub struct PreparedLaunch {
    dispatch: Dispatch,
    descriptor: RuntimeDescriptor,
    required_capabilities: BTreeSet<Capability>,
}
impl PreparedLaunch {
    pub fn new(
        dispatch: Dispatch,
        descriptor: RuntimeDescriptor,
        required_capabilities: BTreeSet<Capability>,
        at: Timestamp,
    ) -> Result<Self, AdapterError> {
        qualify_dispatch(&dispatch, &descriptor, &required_capabilities, at)?;
        Ok(Self {
            dispatch,
            descriptor,
            required_capabilities,
        })
    }

    pub fn authorize(
        self,
        journal: &mut impl ActivationJournal,
        at: Timestamp,
    ) -> Result<LaunchPermit, AdapterError> {
        qualify_dispatch(
            &self.dispatch,
            &self.descriptor,
            &self.required_capabilities,
            at,
        )?;
        let receipt = journal.persist_authorization(&self.dispatch, &self.descriptor, at)?;
        if receipt.dispatch != self.dispatch
            || receipt.descriptor != self.descriptor
            || receipt.authorized_at != at
            || receipt.revision.0 == 0
        {
            return Err(AdapterError::ContractMismatch);
        }
        Ok(LaunchPermit {
            receipt,
            required_capabilities: self.required_capabilities,
        })
    }
}

/// Non-serializable, no public constructor. A worker JSON payload cannot mint it.
/// Consumed at launch; actual one-time fencing across crashes remains Host-owned.
pub struct LaunchPermit {
    receipt: ActivationReceipt,
    required_capabilities: BTreeSet<Capability>,
}
impl LaunchPermit {
    pub fn dispatch(&self) -> &Dispatch {
        &self.receipt.dispatch
    }
    pub fn receipt(&self) -> &ActivationReceipt {
        &self.receipt
    }
    /// Adapters must recheck immediately before resource activation; preparation
    /// or journal latency cannot keep expired enforcement proof valid.
    pub fn validate_at(
        &self,
        current_descriptor: &RuntimeDescriptor,
        at: Timestamp,
    ) -> Result<(), AdapterError> {
        if current_descriptor != &self.receipt.descriptor || at < self.receipt.authorized_at {
            return Err(AdapterError::ContractMismatch);
        }
        qualify_dispatch(
            &self.receipt.dispatch,
            current_descriptor,
            &self.required_capabilities,
            at,
        )?;
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TurnInput {
    pub text: String,
    pub artifacts: Vec<ArtifactId>,
}
impl TurnInput {
    pub fn validate(&self) -> Result<(), AdapterError> {
        if self.text.trim().is_empty() || self.text.len() > 64 * 1024 || self.artifacts.len() > 64 {
            return Err(AdapterError::InvalidInput);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SessionControl {
    Interrupt {},
    Cancel {},
    Resume {},
    Fork { successor: SessionId },
}

/// Full agent-loop adapters, separate from inference-only providers. Unsupported
/// methods are explicit errors, never successful no-ops or PTY parsing guesses.
/// A concrete adapter must enforce session binding, input/event bounds and the
/// permit check. Actual process containment is a prerequisite outside this SDK.
pub trait AgentRuntimeAdapter {
    fn descriptor(&self) -> &RuntimeDescriptor;
    fn launch(&mut self, permit: LaunchPermit, at: Timestamp) -> Result<SessionId, AdapterError>;
    fn send(&mut self, _session: &SessionId, _input: TurnInput) -> Result<(), AdapterError> {
        Err(AdapterError::Unsupported {
            operation: AdapterOperation::Send,
        })
    }
    fn poll_events(
        &mut self,
        _session: &SessionId,
        _after: u64,
        _limit: u16,
    ) -> Result<Vec<crate::events::RuntimeEvent>, AdapterError> {
        Err(AdapterError::Unsupported {
            operation: AdapterOperation::PollEvents,
        })
    }
    fn control(
        &mut self,
        _session: &SessionId,
        command: SessionControl,
    ) -> Result<(), AdapterError> {
        let operation = match command {
            SessionControl::Interrupt {} => AdapterOperation::Interrupt,
            SessionControl::Cancel {} => AdapterOperation::Cancel,
            SessionControl::Resume {} => AdapterOperation::Resume,
            SessionControl::Fork { .. } => AdapterOperation::Fork,
        };
        Err(AdapterError::Unsupported { operation })
    }
    fn dispose(&mut self, _session: &SessionId) -> Result<(), AdapterError> {
        Err(AdapterError::Unsupported {
            operation: AdapterOperation::Dispose,
        })
    }
}
