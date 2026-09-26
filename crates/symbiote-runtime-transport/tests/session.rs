//! A session's end state and its silence, driven against real subprocesses.
//!
//! Every case here runs a real child through `/bin/sh`. The normal path is a frame
//! then a stated exit; the boundary is which of two budgets is reported; the
//! failure paths are a fault, a signal and an unreaped child; interruption is a
//! cancellation with requests still in flight; recovery is a replacement
//! generation that inherits no pending request.
use serde_json::json;
use std::{
    collections::BTreeMap,
    ffi::OsString,
    path::PathBuf,
    time::{Duration, Instant},
};
use symbiote_runtime_transport::rpc::RpcSession;
use symbiote_runtime_transport::session::{
    ExitClass, Liveness, LivenessObservation, MIN_LIVENESS_WINDOW, Session, SessionEnd,
    SessionError, SessionOutcome,
};
use symbiote_runtime_transport::{
    GroupSignalStatus, JsonlTransport, ProcessExit, SpawnSpec, TransportError, TransportLimits,
};

const SECOND: Duration = Duration::from_secs(2);
const WINDOW: Duration = Duration::from_millis(400);

fn spawn(script: &str) -> JsonlTransport {
    JsonlTransport::spawn(
        SpawnSpec {
            executable: PathBuf::from("/bin/sh"),
            args: vec![OsString::from("-c"), OsString::from(script)],
            cwd: std::env::temp_dir(),
            env: BTreeMap::new(),
        },
        TransportLimits::default(),
    )
    .unwrap()
}

fn session(script: &str, liveness: Liveness) -> Session {
    Session::new(spawn(script), liveness).unwrap()
}

/// A wait status is read for what it says: an exit code, a signal, or neither.
/// Nothing here turns an absent code into a success.
#[test]
fn the_exit_classes_are_read_from_the_wait_status_and_never_guessed() {
    for (status, expected) in [
        (
            ProcessExit {
                code: Some(0),
                signal: None,
            },
            ExitClass::Clean,
        ),
        (
            ProcessExit {
                code: Some(3),
                signal: None,
            },
            ExitClass::Failed { code: 3 },
        ),
        (
            ProcessExit {
                code: None,
                signal: Some(15),
            },
            ExitClass::Signalled { signal: 15 },
        ),
        (
            ProcessExit {
                code: None,
                signal: None,
            },
            ExitClass::Indeterminate,
        ),
    ] {
        assert_eq!(ExitClass::of(status), expected);
    }
    assert!(ExitClass::Clean.is_clean());
    assert!(!ExitClass::Failed { code: 0 }.is_clean());
    assert!(!ExitClass::Signalled { signal: 9 }.is_clean());
    assert!(!ExitClass::Indeterminate.is_clean());
}

/// What a session that wrote one frame and then ended must report, whichever of the
/// honest observations the runner wins. Returns the outcome so the caller can state the
/// claim its own script is about.
///
/// `recv` publishes a frame before it publishes end-of-stream, so a frame the child
/// wrote is delivered and never skipped. Once the stream is over it reports either the
/// exit it reaped (`ProcessExited`) or the stream closing before the exit became
/// reapable (`StdoutClosed`) — two honest answers to one observation, and which one a
/// loaded runner sees depends on how long `waitpid` takes. Measured under threefold
/// oversubscription: the second `recv` returns `StdoutClosed` often enough to fail a
/// case that demanded `ProcessExited`.
///
/// The owner's `end` is the same race one level up, and it has three honest answers, not
/// two. `cancel` re-reads the exit after signalling, so a child that finished between the
/// owner's signal and the reap is reaped with its own exit and reported as a
/// *cancellation* carrying that exit: `Cancelled { group_signal: Absent }` with
/// `class() == Some(Clean)` is reachable, and both halves of it are true — the owner did
/// end the session, and the child had already finished clean. Measured: it was the
/// failure an earlier version of this helper invented by assuming a cancellation always
/// carries a signal.
///
/// So the property is the one a caller depends on: the class is always the child's real
/// terminal state, or the signal that ended it, or nothing at all — never a fabricated
/// exit code — and the session is clean only when the child itself finished clean and
/// the session says it finished on its own.
fn assert_terminal_state(script: &str, reaped: ExitClass) -> SessionOutcome {
    let mut owned = session(script, Liveness::Unwatched);
    assert_eq!(
        owned.recv(SECOND).unwrap(),
        json!({}),
        "the frame the child wrote is published before the end of its stream, so it is \
         delivered even when the child has already exited"
    );
    let observed = owned.recv(SECOND);
    assert!(
        matches!(
            observed,
            Err(SessionError::Transport(
                TransportError::ProcessExited(_) | TransportError::StdoutClosed
            ))
        ),
        "the end of a session is evidence rather than a fault, and it arrives as the exit \
         or as the stream closing first, never as another frame and never as a hang: \
         {observed:?}"
    );
    let outcome = owned.end(None, SECOND);
    match outcome.end {
        SessionEnd::Exited => assert_eq!(
            outcome.class(),
            Some(reaped),
            "a session the child finished on its own reports the state it finished in"
        ),
        SessionEnd::Cancelled { .. } | SessionEnd::Unreaped { .. } => {
            let class = outcome.class();
            let honest = match &class {
                None => true,
                Some(ExitClass::Signalled { .. }) => true,
                Some(other) => *other == reaped,
            };
            assert!(
                honest,
                "a session the owner ended reports the child's own state if it had already \
                 finished, the signal that ended it, or nothing — never another exit code: \
                 {class:?}"
            );
        }
        other => {
            panic!("a session that ended reports Exited, Cancelled or Unreaped, not {other:?}")
        }
    }
    assert_eq!(
        outcome.is_clean(),
        matches!(outcome.end, SessionEnd::Exited) && reaped.is_clean(),
        "a session is clean only when the child finished clean by itself: {:?} reported \
         {outcome:?} for a child that finishes {reaped}",
        outcome.class()
    );
    outcome
}

/// The normal path: a clean exit is clean, a failing exit states its code.
#[test]
fn a_completed_session_is_classified_by_the_code_its_process_exited_with() {
    let clean = assert_terminal_state("printf '{}\\n'; exit 0", ExitClass::Clean);
    // One frame, then the end of the session, and nothing left unacknowledged on the
    // generation that carried it.
    if let SessionEnd::Exited = clean.end {
        assert!(clean.unacknowledged.is_empty());
    }

    let failed = assert_terminal_state("printf '{}\\n'; exit 3", ExitClass::Failed { code: 3 });
    assert_ne!(
        failed.class(),
        Some(ExitClass::Clean),
        "a child that exits 3 is never reported clean: {:?}",
        failed.class()
    );
    assert!(!failed.is_clean());
}

/// A signal is reported as a signal. It is never smoothed into an exit code and
/// never counted as a clean session.
#[test]
fn a_signalled_process_is_reported_as_a_signal_rather_than_an_exit_code() {
    let signalled = assert_terminal_state(
        "printf '{}\\n'; kill -TERM $$",
        ExitClass::Signalled { signal: 15 },
    );
    // A signalled child carries no code on any branch: the platform's own signal when
    // the exit is reaped, the owner's signal when it had to end the child, or nothing.
    // None of the three is an exit code, which is the claim this case exists to hold.
    let class = signalled.class();
    assert!(
        matches!(class, None | Some(ExitClass::Signalled { .. })),
        "a signalled child is reported as a signal, never as an exit code: {class:?}"
    );
    assert!(!signalled.is_clean());
}

/// An owned live process is cancelled, and the outcome separates the shared
/// property from the owner's act: the class is the signal that ended it, and the
/// end state says the owner asked.
#[test]
fn an_owned_live_process_is_cancelled_and_classified_cancelled() {
    let live = session("/bin/sleep 30", Liveness::Unwatched);
    assert!(live.child_id() > 0);
    let outcome = live.end(None, SECOND);
    assert_eq!(
        outcome.end,
        SessionEnd::Cancelled {
            group_signal: GroupSignalStatus::Sent
        }
    );
    assert_eq!(outcome.class(), Some(ExitClass::Signalled { signal: 9 }));
    assert!(!outcome.is_clean());
}

/// A child the owner cannot reap in time is never reported clean, and the owner's
/// deadline bounds the wait rather than the child's lifetime.
///
/// The two ends this asserts are the two the code promises, and *which* one a run
/// reaches is a race between the child dying from the group signal and the deadline
/// expiring: `cancel` reads the exit before it reads the clock, so a child that dies
/// inside the window is reaped (`Cancelled`) and one that does not is not (`Unreaped`).
/// Asserting a branch made this case fail under a loaded runner — once in CI on the
/// 1.85.0 leg, with `Cancelled { group_signal: Sent }` where this test had asserted
/// `Unreaped` — while both are honest. The properties are the ones a caller depends
/// on, and they are stated by the code's own documentation: a reaped cancel reports
/// an exit and an unreaped one does not, neither is clean, and the child is reaped on
/// the way out so a timed-out session leaves no zombie.
#[test]
fn a_child_the_owner_cannot_reap_in_time_is_unreaped_and_not_clean() {
    let live = session("/bin/sleep 30", Liveness::Unwatched);
    let pid = live.child_id();
    assert!(pid > 0);
    let started = Instant::now();
    let outcome = live.end(None, Duration::ZERO);
    // The child lives 30 seconds; a wait that tracked the child rather than the
    // deadline would be visible here as most of that.
    let waited = started.elapsed();
    assert!(
        waited < Duration::from_secs(2),
        "a zero deadline still waited {waited:?}, so the deadline bounds the wait only \
         by accident"
    );
    match outcome.end {
        SessionEnd::Cancelled { ref group_signal } => {
            assert!(
                outcome.exit.is_some(),
                "a cancelled end says the process was reaped, so it must carry the exit: \
                 {group_signal:?}"
            );
        }
        SessionEnd::Unreaped { ref error } => {
            assert_eq!(
                *error,
                TransportError::DeadlineExceeded,
                "an unreaped end is the deadline passing, not another failure"
            );
            assert!(
                outcome.exit.is_none(),
                "an unreaped end means the process had not been reaped when the session was \
                 retired, so it cannot carry an exit"
            );
        }
        other => panic!("a session the owner ended reports Cancelled or Unreaped, not {other:?}"),
    }
    assert!(!outcome.is_clean(), "{:?} is not a clean end", outcome.end);
    assert!(
        !matches!(outcome.class(), Some(ExitClass::Clean)),
        "a child killed on the owner's ask never exits zero: {:?}",
        outcome.class()
    );
    // `end` consumed the session, so the transport's own drop has run its reaper
    // with a budget of its own. A child left as a zombie would keep a `/proc` entry
    // for as long as this process lives, which is the leak this asserts is absent.
    let proc = PathBuf::from(format!("/proc/{pid}"));
    let deadline = Instant::now() + SECOND;
    while proc.exists() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(
        !proc.exists(),
        "the child {pid} is still in /proc {:?} after the session was retired, so an \
         unreaped session leaves a zombie behind",
        std::fs::read_to_string(proc.join("stat")).ok()
    );
}

/// A faulted session is faulted even when the child exited zero: a status is not
/// an acquittal for a protocol the transport could not decode.
#[test]
fn a_transport_fault_is_classified_faulted_and_never_a_clean_exit() {
    let mut faulty = session("printf 'not-json\\n'", Liveness::Unwatched);
    assert_eq!(
        faulty.recv(SECOND),
        Err(SessionError::Transport(TransportError::MalformedFrame))
    );
    let outcome = faulty.end(None, SECOND);
    assert_eq!(
        outcome.end,
        SessionEnd::Faulted {
            error: TransportError::MalformedFrame
        }
    );
    assert!(!outcome.is_clean());
    assert!(outcome.unacknowledged.is_empty());
}

/// A fault can arrive while the child is still running. The session is still the
/// owner's to end, so a faulted outcome ends the process inside the owner's own
/// deadline and reports the exit it was reaped with, rather than leaving it to
/// `Drop` and reporting an unknown final state.
#[test]
fn a_faulted_session_still_ends_its_process_within_the_owners_deadline() {
    let mut faulty = session("printf 'not-json\\n'; /bin/sleep 30", Liveness::Unwatched);
    assert_eq!(
        faulty.recv(SECOND),
        Err(SessionError::Transport(TransportError::MalformedFrame))
    );
    let started = Instant::now();
    let outcome = faulty.end(None, SECOND);
    assert_eq!(
        outcome.end,
        SessionEnd::Faulted {
            error: TransportError::MalformedFrame
        }
    );
    assert_eq!(outcome.class(), Some(ExitClass::Signalled { signal: 9 }));
    assert!(!outcome.is_clean());
    assert!(
        started.elapsed() < SECOND,
        "the faulted session was ended inside the owner's deadline"
    );
}

/// Silence before the first frame is measured from the session's own start, so a
/// child that never speaks is judged the same as one that stopped speaking.
#[test]
fn silence_before_the_first_frame_is_measured_from_the_session_start() {
    let mut quiet = session(
        "/bin/sleep 30",
        Liveness::Watched {
            window: Duration::from_millis(200),
        },
    );
    assert!(matches!(
        quiet.liveness(),
        LivenessObservation::WithinWindow { .. }
    ));
    match quiet.recv(SECOND) {
        Err(SessionError::Silence { observed }) => {
            assert!(observed >= Duration::from_millis(200), "{observed:?}")
        }
        other => panic!("expected silence, got {other:?}"),
    }
}

/// Silence past the window is evidence with the process still owned. Retiring the
/// session for that reason is a cancellation, which is only possible because the
/// process was still there.
#[test]
fn silence_past_the_window_is_evidence_and_leaves_the_process_owned() {
    let mut silent = session("/bin/sleep 30", Liveness::Watched { window: WINDOW });
    match silent.recv(SECOND) {
        Err(SessionError::Silence { observed }) => assert!(observed >= WINDOW, "{observed:?}"),
        other => panic!("expected silence, got {other:?}"),
    }
    assert!(matches!(
        silent.liveness(),
        LivenessObservation::Silent { .. }
    ));
    let outcome = silent.end(None, SECOND);
    assert_eq!(
        outcome.end,
        SessionEnd::Cancelled {
            group_signal: GroupSignalStatus::Sent
        }
    );
}

/// The boundary between the two budgets, taken from both sides. The window is the
/// earlier one when no frame arrives and it expires; the receive deadline is the
/// earlier one when the caller's timeout is far shorter than the window.
#[test]
fn the_earlier_of_the_receive_deadline_and_the_window_is_the_one_reported() {
    let promised = Duration::from_millis(150);
    let mut quiet = session("/bin/sleep 30", Liveness::Watched { window: promised });
    match quiet.recv(SECOND) {
        Err(SessionError::Silence { observed }) => assert!(observed >= promised, "{observed:?}"),
        other => panic!("expected the window's silence, got {other:?}"),
    }
    assert_eq!(
        quiet.end(None, SECOND).end,
        SessionEnd::Cancelled {
            group_signal: GroupSignalStatus::Sent
        }
    );

    // A window far longer than the caller's timeout: a deadline stays a deadline.
    let mut bounded = session(
        "/bin/sleep 30",
        Liveness::Watched {
            window: Duration::from_millis(2500),
        },
    );
    let started = Instant::now();
    assert_eq!(
        bounded.recv(Duration::from_millis(100)),
        Err(SessionError::Transport(TransportError::DeadlineExceeded))
    );
    let elapsed = started.elapsed();
    assert!(
        elapsed < Duration::from_secs(1),
        "the receive deadline expired after {elapsed:?}, not the declared window"
    );
    assert!(matches!(
        bounded.liveness(),
        LivenessObservation::WithinWindow { .. }
    ));
    assert_eq!(
        bounded.end(None, SECOND).end,
        SessionEnd::Cancelled {
            group_signal: GroupSignalStatus::Sent
        }
    );
}

/// A frame inside the window restarts it. A stream whose gaps are each inside the
/// window stays open even where the stream as a whole outlives it, so the window is
/// measured from what the session received rather than from when it started.
#[test]
fn frames_inside_the_window_keep_it_open_for_the_whole_stream() {
    let mut steady = session(
        "i=0; while [ $i -lt 12 ]; do printf '{}\\n'; sleep 0.05; i=$((i+1)); done; /bin/sleep 30",
        Liveness::Watched { window: WINDOW },
    );
    for frame in 0..12 {
        assert_eq!(
            steady.recv(Duration::from_millis(800)).unwrap(),
            json!({}),
            "frame {frame} of a stream that outlives its own window"
        );
    }
    assert!(matches!(
        steady.liveness(),
        LivenessObservation::WithinWindow { .. }
    ));
    assert_eq!(
        steady.end(None, SECOND).end,
        SessionEnd::Cancelled {
            group_signal: GroupSignalStatus::Sent
        }
    );
}

/// An unwatched session measures its silence and never judges it: the same silence
/// that is `Silent` under a window stays a receive deadline here.
#[test]
fn silence_is_measured_but_never_judged_without_a_window() {
    let mut unwatched = session("/bin/sleep 30", Liveness::Unwatched);
    assert_eq!(
        unwatched.recv(Duration::from_millis(60)),
        Err(SessionError::Transport(TransportError::DeadlineExceeded))
    );
    match unwatched.liveness() {
        LivenessObservation::WithinWindow { silence } => {
            assert!(silence >= Duration::from_millis(60), "{silence:?}")
        }
        other => panic!("expected an unjudged observation, got {other:?}"),
    }
}

/// A window below the floor is refused rather than read as always silent.
#[test]
fn a_window_below_the_floor_is_refused() {
    for window in [
        Duration::ZERO,
        MIN_LIVENESS_WINDOW - Duration::from_nanos(1),
    ] {
        assert_eq!(
            Session::new(spawn("/bin/sleep 30"), Liveness::Watched { window }).err(),
            Some(SessionError::InvalidLiveness)
        );
    }
    assert!(
        Session::new(
            spawn("/bin/sleep 30"),
            Liveness::Watched {
                window: MIN_LIVENESS_WINDOW
            }
        )
        .is_ok()
    );
}

/// Interruption: requests still in flight when the session is retired come back
/// named, because a write that was not answered leaves its delivery unknown.
#[test]
fn requests_left_in_flight_are_named_for_reconciliation_and_not_replayed() {
    let mut live = session("/bin/sleep 30", Liveness::Unwatched);
    let mut rpc = RpcSession::new(4).unwrap();
    let mut sent = Vec::new();
    for method in ["first", "second"] {
        let request = rpc.request(method, None).unwrap();
        sent.push(request["id"].as_str().unwrap().to_owned());
        live.send(&request, SECOND).unwrap();
    }
    assert_eq!(rpc.pending_count(), 2);
    let outcome = live.end(Some(rpc), SECOND);
    assert_eq!(
        outcome.end,
        SessionEnd::Cancelled {
            group_signal: GroupSignalStatus::Sent
        }
    );
    assert_eq!(outcome.unacknowledged, sent);
}

/// Recovery: a replacement connection is a new generation. It inherits no pending
/// request from the retired one, and its own identity starts where that
/// generation's did.
#[test]
fn a_replacement_session_is_a_new_generation_that_inherits_nothing() {
    let mut first = session(
        "printf '{\"turn\":1}\\n'; /bin/sleep 30",
        Liveness::Unwatched,
    );
    assert_eq!(first.recv(SECOND).unwrap(), json!({"turn": 1}));
    let mut rpc = RpcSession::new(2).unwrap();
    let request = rpc.request("work", None).unwrap();
    let id = request["id"].as_str().unwrap().to_owned();
    first.send(&request, SECOND).unwrap();
    let outcome = first.end(Some(rpc), SECOND);
    assert_eq!(outcome.unacknowledged, vec![id]);

    let mut second = session(
        "printf '{\"turn\":1}\\n'; /bin/sleep 30",
        Liveness::Unwatched,
    );
    let mut fresh = RpcSession::new(2).unwrap();
    assert_eq!(fresh.pending_count(), 0);
    assert_eq!(fresh.request("work", None).unwrap()["id"], "1");
    assert_eq!(second.recv(SECOND).unwrap(), json!({"turn": 1}));
    assert_eq!(
        second.end(None, SECOND).end,
        SessionEnd::Cancelled {
            group_signal: GroupSignalStatus::Sent
        }
    );
}
