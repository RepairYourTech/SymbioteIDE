//! The native lane's [`AgentRuntimeAdapter`]: the session boundary a native
//! worker is launched through, and where every refusal on that boundary is.
//!
//! The model turn is **not** this module's. `InferenceTransport` owns inference,
//! and an adapter that also owned it would be a second loop over the same turn.
//! What is here is everything around the turn: the permit check, the dispatch
//! binding, the session identity, bounded events, lifecycle control and
//! disposal — all of it over a [`HarnessProcess`] the caller installs.
//!
//! The posture is the SDK's own. `NativeAgentAdapter::new` installs **no**
//! harness, and every session method on such an adapter refuses with
//! [`AdapterError::Unavailable`]: an adapter with nothing to drive must not
//! simulate a turn, report a session it did not start, or return a handshake it
//! did not receive. The caller installs a concrete harness in production
//! integration; these cases install a mock. Which one is installed is never a
//! fact this module claims.
//!
//! Three invariants are enforced here rather than documented and hoped for:
//!
//! * **The permit is rechecked at activation.** `launch` calls
//!   [`LaunchPermit::validate_at`] against the descriptor the adapter actually
//!   holds, so a descriptor that moved, or a permit older than the clock,
//!   refuses before any process is started.
//! * **Every later call is bound to a session this adapter started.** `send`,
//!   `poll_events`, `control`, `dispose` and `handshake` resolve the session
//!   through the adapter's own table and refuse
//!   [`AdapterError::SessionMismatch`] for any identity it did not mint, so a
//!   harness cannot answer for a session that was never launched. A handle
//!   returned by a mismatched start is released before the refusal unless it is
//!   the exact handle already owned for a live session.
//! * **An event is bound to the session it was polled for.** `poll_events`
//!   refuses any event whose own binding names another session, dispatch, Host
//!   or runtime kind, or whose sequence does not advance the caller's cursor,
//!   rather than passing it on. A harness that answers for the wrong lane fails
//!   here instead of at the consumer.
use std::collections::BTreeMap;

use symbiote_domain::{Dispatch, DispatchId, Session, SessionId, Timestamp};
use symbiote_runtime_sdk::events::RuntimeEvent;
use symbiote_runtime_sdk::projection::{ContractProjection, RuntimeHandshake};
use symbiote_runtime_sdk::{
    AdapterError, AdapterOperation, AgentRuntimeAdapter, LaunchPermit, RuntimeDescriptor,
    SessionControl, TurnInput,
};

/// What a harness is given at launch, and nothing more: the session identity the
/// adapter minted, the exact dispatch the permit authorized, the descriptor it
/// was qualified against, and the projection of the two. The dispatch is the
/// original immutable contract record, not a copy made here; the projection adds
/// only the identities and revisions it references and their carriage, never
/// contract prose or a second place for the contract to be wrong.
pub struct LaunchRequest<'a> {
    pub session: &'a SessionId,
    pub dispatch: &'a Dispatch,
    pub descriptor: &'a RuntimeDescriptor,
    pub projection: &'a ContractProjection,
}

/// A harness that has been started: the identity every later call is bound to,
/// and the generation that tells a resumed or forked successor from the session
/// it came from. The adapter stores this and hands it back; it never trusts a
/// harness to remember which session a call is about.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HarnessSession {
    pub session: SessionId,
    pub generation: u64,
}

/// The process behind a session. The methods are the session boundary this crate
/// owns; what a turn *is* belongs to the implementation, and an implementation
/// that cannot do one says so rather than returning silence.
pub trait HarnessProcess {
    /// Starts the session. Called once per session, after the permit recheck.
    fn start(&mut self, request: &LaunchRequest<'_>) -> Result<HarnessSession, AdapterError>;
    /// Hands one validated turn to the session.
    fn send(&mut self, session: &HarnessSession, input: &TurnInput) -> Result<(), AdapterError>;
    /// The session's events after `after`, at most `limit` of them. A harness
    /// with nothing new yet reports an empty list: absence is a fact, not an
    /// unavailability.
    fn events(
        &mut self,
        session: &HarnessSession,
        after: u64,
        limit: u16,
    ) -> Result<Vec<RuntimeEvent>, AdapterError>;
    /// One lifecycle command, applied to this session only.
    fn control(
        &mut self,
        session: &HarnessSession,
        command: &SessionControl,
    ) -> Result<(), AdapterError>;
    /// Ends the session's resources. A second call is the caller's error.
    fn dispose(&mut self, session: &HarnessSession) -> Result<(), AdapterError>;
    /// The runtime's own account of the surfaces this session took, as the
    /// harness received it from the process it started — never as this adapter
    /// would have projected it.
    fn handshake(&self, session: &HarnessSession) -> Result<RuntimeHandshake, AdapterError>;
}

/// One live session: the harness's handle and the dispatch it is bound to.
#[derive(Clone, Debug, PartialEq, Eq)]
struct LiveSession {
    harness: HarnessSession,
    dispatch: DispatchId,
}

/// The native lane's adapter. Holds the descriptor it was declared with and the
/// harness it drives, and nothing else: no projection cache, no report cache, no
/// completion path. An adapter method completes no canonical task.
pub struct NativeAgentAdapter {
    descriptor: RuntimeDescriptor,
    harness: Option<Box<dyn HarnessProcess>>,
    sessions: BTreeMap<SessionId, LiveSession>,
    /// The last session number this adapter minted. It never moves backwards,
    /// even when a session is disposed, so a stale report cannot be credited to
    /// a later process that happens to get the same dispatch.
    next_generation: u64,
}

impl NativeAgentAdapter {
    /// An adapter with no harness behind it. Every session method refuses with
    /// [`AdapterError::Unavailable`]; the descriptor and the projection remain
    /// readable, so an operator can see what a native runtime could carry before
    /// anything is launched.
    pub fn new(descriptor: RuntimeDescriptor) -> Self {
        Self {
            descriptor,
            harness: None,
            sessions: BTreeMap::new(),
            next_generation: 0,
        }
    }

    /// An adapter over an installed harness implementation.
    pub fn with_harness(descriptor: RuntimeDescriptor, harness: Box<dyn HarnessProcess>) -> Self {
        Self {
            descriptor,
            harness: Some(harness),
            sessions: BTreeMap::new(),
            next_generation: 0,
        }
    }

    /// The sessions this adapter has started and not yet disposed, by identity.
    pub fn live_sessions(&self) -> Vec<SessionId> {
        self.sessions.keys().cloned().collect()
    }

    /// The handle for a session this adapter started, or the refusal. An
    /// identity it never minted is not a session, and a disposed one stays gone.
    fn live(&self, session: &SessionId) -> Result<LiveSession, AdapterError> {
        self.sessions
            .get(session)
            .cloned()
            .ok_or(AdapterError::SessionMismatch)
    }

    /// The availability check for methods that do not need the harness out.
    /// Session binding comes after it so an adapter with no harness reports the
    /// one fact that owns the refusal rather than blaming a session it could
    /// never have started.
    fn require_harness(&self) -> Result<(), AdapterError> {
        if self.harness.is_none() {
            Err(AdapterError::Unavailable)
        } else {
            Ok(())
        }
    }

    /// Takes the harness out for the length of one call and puts it back on every
    /// path out, so a launch that refuses leaves the adapter exactly as it was:
    /// an adapter that lost its harness to a refusal would be unusable for the
    /// next attempt, which is a different failure from the one reported.
    fn through_harness<R>(
        &mut self,
        call: impl FnOnce(&mut dyn HarnessProcess) -> Result<R, AdapterError>,
    ) -> Result<R, AdapterError> {
        let mut harness = self.harness.take().ok_or(AdapterError::Unavailable)?;
        let outcome = call(harness.as_mut());
        self.harness = Some(harness);
        outcome
    }
}

impl AgentRuntimeAdapter for NativeAgentAdapter {
    fn descriptor(&self) -> &RuntimeDescriptor {
        &self.descriptor
    }

    fn launch(&mut self, permit: LaunchPermit, at: Timestamp) -> Result<SessionId, AdapterError> {
        // The permit is rechecked against the descriptor this adapter holds,
        // immediately before activation: preparation latency cannot keep a moved
        // descriptor or a lapsed proof valid.
        permit.validate_at(&self.descriptor, at)?;
        let dispatch = permit.dispatch().id().clone();
        // The projection is the descriptor's own carriage of the contract, and
        // it is computed before the harness is borrowed: the harness receives it
        // as a reference and keeps nothing.
        let projection = self.projection(permit.dispatch());
        let generation = self
            .next_generation
            .checked_add(1)
            .ok_or(AdapterError::ContractMismatch)?;
        self.next_generation = generation;
        let session = SessionId::new(format!("{dispatch}-{generation}"))
            .map_err(|_| AdapterError::ContractMismatch)?;
        if self.sessions.contains_key(&session) {
            return Err(AdapterError::ContractMismatch);
        }
        // No harness, no launch. Never a simulated session. The descriptor is
        // read into the request before the harness is taken out, so the adapter
        // is not borrowed twice at once.
        let descriptor = self.descriptor.clone();
        let started = self.through_harness(|harness| {
            harness.start(&LaunchRequest {
                session: &session,
                dispatch: permit.dispatch(),
                descriptor: &descriptor,
                projection: &projection,
            })
        })?;
        // A harness that started a session under another identity has not
        // started *this* one, and the adapter records nothing it cannot bind.
        if started.session != session {
            // `start` returning a handle transfers that handle to the adapter,
            // even when the harness filled the wrong identity in it. Release a
            // newly returned handle before reporting the mismatch. Compare the
            // complete handle: a harness that returns one already in our table
            // has returned an existing resource, and disposing that would end
            // the live session rather than clean up a failed launch.
            let already_owned = self.sessions.values().any(|live| live.harness == started);
            if !already_owned {
                let _ = self.through_harness(|harness| harness.dispose(&started));
            }
            return Err(AdapterError::SessionMismatch);
        }
        self.sessions.insert(
            session.clone(),
            LiveSession {
                harness: started,
                dispatch,
            },
        );
        Ok(session)
    }

    fn send(&mut self, session: &SessionId, input: TurnInput) -> Result<(), AdapterError> {
        self.require_harness()?;
        // The input is validated here as well as by the harness's own consumer:
        // an unbounded turn must not reach a process at all.
        input.validate()?;
        let live = self.live(session)?;
        self.through_harness(|harness| harness.send(&live.harness, &input))
    }

    fn poll_events(
        &mut self,
        session: &SessionId,
        after: u64,
        limit: u16,
    ) -> Result<Vec<RuntimeEvent>, AdapterError> {
        self.require_harness()?;
        let live = self.live(session)?;
        let events = self.through_harness(|harness| harness.events(&live.harness, after, limit))?;
        if events.len() > limit as usize {
            return Err(AdapterError::ContractMismatch);
        }
        // Every event must be bound to the complete session this adapter holds
        // and must advance the caller's cursor. A harness answering for another
        // lane, Host or runtime kind fails here rather than at whichever consumer
        // happened to read it.
        let mut previous = after;
        for event in &events {
            let binding = event.binding();
            if binding.session_id != *session
                || binding.dispatch_id != live.dispatch
                || binding.host_id != self.descriptor.host_id
                || binding.runtime != self.descriptor.runtime
            {
                return Err(AdapterError::SessionMismatch);
            }
            if event.sequence() <= previous {
                return Err(AdapterError::ContractMismatch);
            }
            previous = event.sequence();
        }
        Ok(events)
    }

    fn control(
        &mut self,
        session: &SessionId,
        command: SessionControl,
    ) -> Result<(), AdapterError> {
        self.require_harness()?;
        let live = self.live(session)?;
        // This reference adapter owns process interruption and cancellation, but
        // no continuation store or successor handle. Forwarding resume/fork to a
        // harness would report a lifecycle method the adapter cannot bind or
        // recover, so those methods are explicitly unsupported until that state
        // has one owner.
        match &command {
            SessionControl::Interrupt {} | SessionControl::Cancel {} => {}
            SessionControl::Resume {} => {
                return Err(AdapterError::Unsupported {
                    operation: AdapterOperation::Resume,
                });
            }
            SessionControl::Fork { .. } => {
                return Err(AdapterError::Unsupported {
                    operation: AdapterOperation::Fork,
                });
            }
        }
        self.through_harness(|harness| harness.control(&live.harness, &command))
    }

    fn dispose(&mut self, session: &SessionId) -> Result<(), AdapterError> {
        self.require_harness()?;
        let live = self.live(session)?;
        self.through_harness(|harness| harness.dispose(&live.harness))?;
        // The record goes only after the harness says it ended: an adapter that
        // forgot a session whose disposal failed would answer for a process that
        // is still running.
        self.sessions.remove(session);
        Ok(())
    }

    fn handshake(&self, session: &Session) -> Result<RuntimeHandshake, AdapterError> {
        self.require_harness()?;
        let live = self.live(&session.id)?;
        // The caller's record is checked before the harness is asked. A caller
        // cannot use a live session id to make the process produce a report for
        // another dispatch or runtime kind.
        if session.dispatch_id != live.dispatch || session.kind != self.descriptor.runtime {
            return Err(AdapterError::SessionMismatch);
        }
        let harness = self.harness.as_deref().ok_or(AdapterError::Unavailable)?;
        let report = harness.handshake(&live.harness)?;
        // The report is of *this* session and *this* dispatch. A report about
        // another one is refused here, before any caller can reconcile it against
        // a projection it does not belong to.
        if report.session != session.id {
            return Err(AdapterError::SessionMismatch);
        }
        if report.dispatch != live.dispatch {
            return Err(AdapterError::ContractMismatch);
        }
        Ok(report)
    }
}
