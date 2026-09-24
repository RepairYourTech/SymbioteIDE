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
    SessionError,
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

/// The normal path: a clean exit is clean, a failing exit states its code.
#[test]
fn a_completed_session_is_classified_by_the_code_its_process_exited_with() {
    let mut clean = session("printf '{}\\n'; exit 0", Liveness::Unwatched);
    assert_eq!(clean.recv(SECOND).unwrap(), json!({}));
    // The end of the child is observed from recv as evidence, not as a fault.
    assert!(matches!(
        clean.recv(SECOND),
        Err(SessionError::Transport(TransportError::ProcessExited(_)))
    ));
    let outcome = clean.end(None, SECOND);
    assert_eq!(outcome.end, SessionEnd::Exited);
    assert_eq!(outcome.class(), Some(ExitClass::Clean));
    assert!(outcome.is_clean());
    assert!(outcome.unacknowledged.is_empty());

    let mut failed = session("printf '{}\\n'; exit 3", Liveness::Unwatched);
    assert_eq!(failed.recv(SECOND).unwrap(), json!({}));
    let outcome = failed.end(None, SECOND);
    assert_eq!(outcome.end, SessionEnd::Exited);
    assert_eq!(outcome.class(), Some(ExitClass::Failed { code: 3 }));
    assert!(!outcome.is_clean());
}

/// A signal is reported as a signal. It is never smoothed into an exit code and
/// never counted as a clean session.
#[test]
fn a_signalled_process_is_reported_as_a_signal_rather_than_an_exit_code() {
    let mut signalled = session("printf '{}\\n'; kill -TERM $$", Liveness::Unwatched);
    assert_eq!(signalled.recv(SECOND).unwrap(), json!({}));
    let outcome = signalled.end(None, SECOND);
    assert_eq!(outcome.end, SessionEnd::Exited);
    assert_eq!(outcome.class(), Some(ExitClass::Signalled { signal: 15 }));
    assert!(!outcome.is_clean());
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

/// A child that cannot be reaped inside the owner's deadline is `Unreaped`. The
/// process may still be running, so the final state is unknown rather than clean.
#[test]
fn a_child_the_owner_cannot_reap_in_time_is_unreaped_and_not_clean() {
    let live = session("/bin/sleep 30", Liveness::Unwatched);
    let outcome = live.end(None, Duration::ZERO);
    assert_eq!(
        outcome.end,
        SessionEnd::Unreaped {
            error: TransportError::DeadlineExceeded
        }
    );
    assert!(!outcome.is_clean());
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
