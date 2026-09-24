//! How a supervised transport session ended, and silence as evidence.
//!
//! `JsonlTransport` reports what it observed: frames, faults, diagnostics and a
//! wait status. This module is the one place those observations become the states
//! an owner must act on differently, so a caller cannot quietly read an unreaped
//! process or a faulted session as a clean one. Silence past a declared window is
//! reported with the process still owned: it is evidence, never an end state.
use crate::rpc::RpcSession;
use crate::{GroupSignalStatus, JsonlTransport, ProcessExit, TransportError};
use serde_json::Value;
use std::fmt;
use std::time::{Duration, Instant};

/// The shortest window silence may be judged over. A zero window would call every
/// session silent before it produced anything, so it is refused rather than
/// reinterpreted.
pub const MIN_LIVENESS_WINDOW: Duration = Duration::from_millis(1);

/// How an owned process terminated, read from the one evidence the platform gives:
/// a wait status. An exit code and a terminating signal are alternatives, and a
/// status carrying neither is reported as such instead of guessed at.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExitClass {
    Clean,
    Failed {
        code: i32,
    },
    Signalled {
        signal: i32,
    },
    /// The status carried neither an exit code nor a signal. The process ended
    /// and the platform did not say how, so the class stays unknown rather than
    /// being reported as a success.
    Indeterminate,
}

impl ExitClass {
    pub fn of(exit: ProcessExit) -> Self {
        match (exit.code, exit.signal) {
            (Some(0), _) => ExitClass::Clean,
            (Some(code), _) => ExitClass::Failed { code },
            (None, Some(signal)) => ExitClass::Signalled { signal },
            (None, None) => ExitClass::Indeterminate,
        }
    }

    /// Whether the process exited zero. A cancelled session classifies as
    /// `Signalled`, so this is false for anything the owner had to end.
    pub fn is_clean(self) -> bool {
        matches!(self, ExitClass::Clean)
    }
}

impl fmt::Display for ExitClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExitClass::Clean => f.write_str("exited 0"),
            ExitClass::Failed { code } => write!(f, "exited {code}"),
            ExitClass::Signalled { signal } => write!(f, "terminated by signal {signal}"),
            ExitClass::Indeterminate => f.write_str("ended without an exit code or a signal"),
        }
    }
}

/// Whether silence is judged, and over what window. An unwatched session measures
/// its silence and never rules on it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Liveness {
    Unwatched,
    /// A frame must arrive within this window of the last one the session
    /// received, or of the session's own start before the first frame.
    Watched {
        window: Duration,
    },
}

/// The silence a session has observed, judged against its window. One reading, so
/// a caller's own poll and an interleaved `recv` cannot disagree.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LivenessObservation {
    /// A frame arrived within the window, or no window is declared.
    WithinWindow { silence: Duration },
    /// Silence has reached the declared window while the session is still owned.
    Silent { silence: Duration },
}

/// Why a session ended, as one of the states an owner must act on differently.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SessionEnd {
    /// The owned process terminated on its own and no transport fault was seen.
    Exited,
    /// The owner ended the session and the process was reaped. The group signal
    /// status separates signalling a live group from a group that was already
    /// gone and a signal that could not be sent.
    Cancelled { group_signal: GroupSignalStatus },
    /// A transport fault ended the session.
    Faulted { error: TransportError },
    /// The owner's deadline passed before the process was reaped. The final state
    /// is unknown: it is not a clean session.
    Unreaped { error: TransportError },
}

/// The classified end of a session, with what could not be resolved on it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionOutcome {
    pub end: SessionEnd,
    /// How the owned process terminated, when it was reaped. `None` means the
    /// process had not been reaped when the session was retired.
    pub exit: Option<ProcessExit>,
    /// Request IDs this connection generation never saw answered. Their delivery
    /// and effects are unknown: they are named for reconciliation and are not
    /// permission to replay them.
    pub unacknowledged: Vec<String>,
}

impl SessionOutcome {
    /// The exit class, when the process was reaped. `None` is an unknown final
    /// state rather than a clean one.
    pub fn class(&self) -> Option<ExitClass> {
        self.exit.clone().map(ExitClass::of)
    }

    /// Whether the process terminated on its own with a zero exit code. A fault, a
    /// cancellation and an unreaped session are all false, whatever the status.
    pub fn is_clean(&self) -> bool {
        matches!(self.end, SessionEnd::Exited) && matches!(self.class(), Some(ExitClass::Clean))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SessionError {
    /// A liveness window below [`MIN_LIVENESS_WINDOW`].
    InvalidLiveness,
    Transport(TransportError),
    /// The child produced no frame within its declared window while the session
    /// was still owned. The process may still be running.
    Silence {
        observed: Duration,
    },
}

impl fmt::Display for SessionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SessionError::InvalidLiveness => {
                write!(f, "liveness window below {MIN_LIVENESS_WINDOW:?}")
            }
            SessionError::Transport(error) => write!(f, "transport: {error}"),
            SessionError::Silence { observed } => {
                write!(f, "no frame for {observed:?}, past the declared window")
            }
        }
    }
}

impl std::error::Error for SessionError {}

/// A transport session with its liveness measured from the frames it receives.
pub struct Session {
    transport: JsonlTransport,
    liveness: Liveness,
    started: Instant,
    last_frame: Option<Instant>,
}

impl Session {
    pub fn new(transport: JsonlTransport, liveness: Liveness) -> Result<Self, SessionError> {
        if let Liveness::Watched { window } = liveness {
            if window < MIN_LIVENESS_WINDOW {
                return Err(SessionError::InvalidLiveness);
            }
        }
        Ok(Self {
            transport,
            liveness,
            started: Instant::now(),
            last_frame: None,
        })
    }

    /// The directly owned child's process ID, for diagnostics that must name the
    /// process rather than the session.
    pub fn child_id(&self) -> u32 {
        self.transport.child_id()
    }

    /// This session's silence, judged against its declared window.
    pub fn liveness(&self) -> LivenessObservation {
        let silence = self.silence();
        match self.liveness {
            Liveness::Watched { window } if silence >= window => {
                LivenessObservation::Silent { silence }
            }
            _ => LivenessObservation::WithinWindow { silence },
        }
    }

    pub fn send(&mut self, value: &Value, timeout: Duration) -> Result<(), SessionError> {
        self.transport
            .send(value, timeout)
            .map_err(SessionError::Transport)
    }

    /// Receive one frame within `timeout`, and within the declared liveness window
    /// when one is declared. The earlier of the two budgets is the one reported:
    /// a receive deadline stays a deadline, and silence is reported as silence
    /// only once the window is actually reached.
    ///
    /// `TransportError::ProcessExited` is how a session that ended on its own is
    /// observed from here. It is evidence, and [`Session::end`] turns it into a
    /// classified outcome.
    pub fn recv(&mut self, timeout: Duration) -> Result<Value, SessionError> {
        let deadline = Instant::now()
            .checked_add(timeout)
            .ok_or(SessionError::Transport(TransportError::DeadlineExceeded))?;
        loop {
            let budget = match self.liveness() {
                LivenessObservation::Silent { silence } => {
                    return Err(SessionError::Silence { observed: silence });
                }
                LivenessObservation::WithinWindow { silence } => {
                    let remaining = deadline.saturating_duration_since(Instant::now());
                    match self.liveness {
                        Liveness::Unwatched => remaining,
                        Liveness::Watched { window } => {
                            remaining.min(window.saturating_sub(silence))
                        }
                    }
                }
            };
            if budget.is_zero() {
                return Err(SessionError::Transport(TransportError::DeadlineExceeded));
            }
            match self.transport.recv(budget) {
                Ok(frame) => {
                    self.last_frame = Some(Instant::now());
                    return Ok(frame);
                }
                // Re-measure rather than assume which budget expired: the window
                // is judged by the same reading every other caller sees.
                Err(TransportError::DeadlineExceeded) if Instant::now() < deadline => {}
                Err(error) => return Err(SessionError::Transport(error)),
            }
        }
    }

    /// Retire the session and classify how it ended. Consuming both the session and
    /// the connection generation's `RpcSession` is the discipline: a replacement
    /// connection is a new generation, and the requests this one never saw answered
    /// come back named instead of being replayed.
    ///
    /// Every path ends the process within `timeout`. A fault can arrive while the
    /// child is still running, so a faulted session is still an ended one: the exit
    /// returned in that case is the cancellation's own, and the process is not left
    /// to `Drop`'s later bounded attempt.
    pub fn end(mut self, rpc: Option<RpcSession>, timeout: Duration) -> SessionOutcome {
        let unacknowledged = rpc.map(RpcSession::into_unresolved).unwrap_or_default();
        if let Some(error) = self.transport.failure() {
            let exit = match self.transport.try_wait() {
                Ok(Some(exit)) => Some(exit),
                _ => {
                    let ended = self
                        .transport
                        .cancel(timeout)
                        .ok()
                        .map(|report| report.exit);
                    ended.or_else(|| self.transport.try_wait().ok().flatten())
                }
            };
            return SessionOutcome {
                end: SessionEnd::Faulted { error },
                exit,
                unacknowledged,
            };
        }
        if let Ok(Some(exit)) = self.transport.try_wait() {
            return SessionOutcome {
                end: SessionEnd::Exited,
                exit: Some(exit),
                unacknowledged,
            };
        }
        match self.transport.cancel(timeout) {
            Ok(report) => SessionOutcome {
                end: SessionEnd::Cancelled {
                    group_signal: report.group_signal,
                },
                exit: Some(report.exit),
                unacknowledged,
            },
            Err(error) => SessionOutcome {
                end: SessionEnd::Unreaped { error },
                exit: self.transport.try_wait().ok().flatten(),
                unacknowledged,
            },
        }
    }

    fn silence(&self) -> Duration {
        self.last_frame.unwrap_or(self.started).elapsed()
    }
}
