use serde_json::json;
use std::{
    collections::BTreeMap,
    ffi::OsString,
    fs,
    path::PathBuf,
    thread,
    time::{Duration, Instant},
};
use symbiote_runtime_transport::*;

fn spawn(script: &str, limits: TransportLimits) -> JsonlTransport {
    JsonlTransport::spawn(
        SpawnSpec {
            executable: PathBuf::from("/bin/sh"),
            args: vec![OsString::from("-c"), OsString::from(script)],
            cwd: std::env::temp_dir(),
            env: BTreeMap::new(),
        },
        limits,
    )
    .unwrap()
}
const SECOND: Duration = Duration::from_secs(2);
fn wait_until(mut condition: impl FnMut() -> bool) {
    let deadline = Instant::now() + SECOND;
    while !condition() {
        assert!(Instant::now() < deadline);
        thread::sleep(Duration::from_millis(2));
    }
}

#[test]
fn actual_echo_has_explicit_environment_and_cwd_and_separate_stderr() {
    let script = "printf '{\"home_empty\":%s,\"allowed\":\"%s\",\"cwd\":\"%s\"}\n' \"$(test -z \"$HOME\" && echo true || echo false)\" \"$ALLOWED\" \"$PWD\"; printf 'diagnostic only\n' >&2; exec 2>&-; while IFS= read -r line; do printf '%s\n' \"$line\"; done";
    let mut transport = JsonlTransport::spawn(
        SpawnSpec {
            executable: PathBuf::from("/bin/sh"),
            args: vec!["-c".into(), script.into()],
            cwd: std::env::temp_dir(),
            env: BTreeMap::from([("ALLOWED".into(), "explicit".into())]),
        },
        TransportLimits::default(),
    )
    .unwrap();
    assert_eq!(
        transport.recv(SECOND).unwrap(),
        json!({"home_empty":true,"allowed":"explicit","cwd":std::env::temp_dir().canonicalize().unwrap()})
    );
    let request = json!({"method":"echo","params":{"value":"hello"}});
    transport.send(&request, SECOND).unwrap();
    assert_eq!(transport.recv(SECOND).unwrap(), request);
    wait_until(|| transport.diagnostics_finished());
    let diagnostics = transport.diagnostics();
    assert_eq!(diagnostics.entries[0].text, "diagnostic only");
    assert!(!format!("{diagnostics:?}").contains("diagnostic only"));
    transport.cancel(SECOND).unwrap();
}

#[test]
fn malformed_duplicate_nested_duplicate_and_partial_frames_fail_without_desync() {
    for (script, expected) in [
        ("printf 'not-json\n{}\n'", TransportError::MalformedFrame),
        (
            "printf '{\"id\":1,\"id\":2}\n'",
            TransportError::MalformedFrame,
        ),
        (
            "printf '{\"result\":{\"x\":1,\"x\":2}}\n'",
            TransportError::MalformedFrame,
        ),
        (
            "printf '{\"unterminated\":true}'",
            TransportError::UnterminatedFrame,
        ),
    ] {
        let mut transport = spawn(script, TransportLimits::default());
        assert_eq!(transport.recv(SECOND), Err(expected.clone()));
        assert_eq!(transport.recv(SECOND), Err(expected));
    }
}

#[test]
fn oversized_input_and_output_are_explicit_and_queues_are_bounded() {
    let limits = TransportLimits {
        max_frame_bytes: 16,
        ..TransportLimits::default()
    };
    let mut transport = spawn("/bin/sleep 10", limits);
    assert_eq!(
        transport.send(&json!({"large":"x".repeat(1000)}), SECOND),
        Err(TransportError::FrameTooLarge)
    );
    transport.cancel(SECOND).unwrap();
    let mut oversized = spawn(
        "printf '{\"value\":\"xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx\"}\n'",
        limits,
    );
    assert_eq!(oversized.recv(SECOND), Err(TransportError::FrameTooLarge));
    let mut flooded = spawn(
        "i=0; while [ $i -lt 10 ]; do printf '{}\n'; i=$((i+1)); done",
        TransportLimits {
            frame_queue_capacity: 1,
            ..TransportLimits::default()
        },
    );
    wait_until(|| flooded.failure().is_some());
    assert_eq!(
        flooded.recv(SECOND),
        Err(TransportError::FrameQueueOverflow)
    );
}

#[test]
fn receive_deadline_is_nonfatal_and_stdin_backpressure_cannot_hang() {
    let mut delayed = spawn(
        "/bin/sleep 0.1; printf '{}\n'; /bin/sleep 1",
        TransportLimits::default(),
    );
    assert_eq!(
        delayed.recv(Duration::from_millis(5)),
        Err(TransportError::DeadlineExceeded)
    );
    assert_eq!(delayed.recv(SECOND).unwrap(), json!({}));
    let mut blocked = spawn(
        "/bin/sleep 10",
        TransportLimits {
            max_frame_bytes: 1_048_576,
            ..TransportLimits::default()
        },
    );
    let start = Instant::now();
    let result = blocked.send(&json!("x".repeat(524288)), Duration::from_secs(1));
    assert_eq!(result, Err(TransportError::PartialFrameWrite));
    assert!(start.elapsed() < Duration::from_secs(2));
    assert_eq!(
        blocked.send(&json!({}), SECOND),
        Err(TransportError::PartialFrameWrite)
    );
    blocked.cancel(SECOND).unwrap();
}

#[test]
fn diagnostics_have_bounded_lines_and_loss_is_visible() {
    let mut transport = spawn(
        "printf 'longdiagnostic\na\nb\nc\n' >&2; exec 2>&-; /bin/sleep 1",
        TransportLimits {
            max_diagnostic_bytes: 4,
            diagnostic_queue_capacity: 4,
            ..TransportLimits::default()
        },
    );
    wait_until(|| transport.diagnostics_finished());
    let first = transport.diagnostics();
    assert_eq!(first.entries.len(), 4);
    assert!(first.entries[0].truncated);
    assert_eq!(first.entries[0].text, "long");
    let mut overflow = spawn(
        "printf 'a\nb\nc\n' >&2; exec 2>&-; /bin/sleep 1",
        TransportLimits {
            diagnostic_queue_capacity: 1,
            ..TransportLimits::default()
        },
    );
    wait_until(|| overflow.diagnostics_finished());
    let second = overflow.diagnostics();
    assert_eq!(second.entries.len(), 1);
    assert_eq!(second.dropped, 2);
    let mut invalid = spawn(
        "printf '\\377\\377\\377\\377\\n' >&2",
        TransportLimits {
            max_diagnostic_bytes: 4,
            ..TransportLimits::default()
        },
    );
    wait_until(|| invalid.diagnostics_finished());
    let third = invalid.diagnostics();
    assert!(third.entries[0].text.len() <= 4);
    assert!(third.entries[0].truncated);
}

fn live(pid: u32) -> bool {
    fs::read_to_string(format!("/proc/{pid}/stat"))
        .ok()
        .and_then(|stat| {
            stat.rsplit_once(')')
                .map(|(_, tail)| tail.trim_start().starts_with('Z'))
        })
        .is_some_and(|zombie| !zombie)
}
fn group_members(group: u32) -> Vec<u32> {
    fs::read_dir("/proc")
        .unwrap()
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let pid = entry.file_name().to_str()?.parse::<u32>().ok()?;
            let stat = fs::read_to_string(entry.path().join("stat")).ok()?;
            let (_, tail) = stat.rsplit_once(')')?;
            let fields: Vec<_> = tail.split_whitespace().collect();
            (fields.get(2)?.parse::<u32>().ok()? == group && live(pid)).then_some(pid)
        })
        .collect()
}
#[test]
fn cancellation_and_drop_terminate_owned_unescaped_process_groups() {
    for explicit in [true, false] {
        let mut transport = spawn(
            "/bin/sh -c '/bin/sleep 10 & wait' & printf '{}\n'; wait",
            TransportLimits::default(),
        );
        transport.recv(SECOND).unwrap();
        let group = transport.child_id();
        let deadline = Instant::now() + SECOND;
        let members = loop {
            let members = group_members(group);
            if members.len() >= 3 {
                break members;
            }
            assert!(Instant::now() < deadline);
            thread::sleep(Duration::from_millis(5));
        };
        if explicit {
            let report = transport.cancel(SECOND).unwrap();
            assert!(report.exit.signal.is_some());
            assert_eq!(report.group_signal, GroupSignalStatus::Sent);
            assert_eq!(report.descendant_cleanup, DescendantCleanup::Unknown);
        }
        drop(transport);
        let deadline = Instant::now() + SECOND;
        while members.iter().copied().any(live) {
            assert!(Instant::now() < deadline);
            thread::sleep(Duration::from_millis(5));
        }
    }
}

#[test]
fn child_exit_and_closed_transport_are_distinct_from_frames() {
    let mut transport = spawn("printf '{}\n'; exit 7", TransportLimits::default());
    assert_eq!(transport.recv(SECOND).unwrap(), json!({}));
    let deadline = Instant::now() + SECOND;
    while transport.try_wait().unwrap().is_none() {
        assert!(Instant::now() < deadline);
        thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(
        transport.recv(SECOND),
        Err(TransportError::ProcessExited(ProcessExit {
            code: Some(7),
            signal: None
        }))
    );
    let report = transport.cancel(SECOND).unwrap();
    assert_eq!(report.group_signal, GroupSignalStatus::LeaderAlreadyReaped);
    assert_eq!(report.descendant_cleanup, DescendantCleanup::Unknown);
    assert_eq!(transport.recv(SECOND), Err(TransportError::Closed));
}

#[test]
fn already_reaped_leader_reports_unknown_while_unescaped_child_survives() {
    let mut transport = spawn(
        "/bin/sleep 3 >/dev/null 2>&1 & printf '{}\n'; exit 0",
        TransportLimits::default(),
    );
    transport.recv(SECOND).unwrap();
    let group = transport.child_id();
    wait_until(|| transport.try_wait().unwrap().is_some());
    let survivors = group_members(group);
    assert!(!survivors.is_empty());
    let report = transport.cancel(SECOND).unwrap();
    assert_eq!(report.group_signal, GroupSignalStatus::LeaderAlreadyReaped);
    assert_eq!(report.descendant_cleanup, DescendantCleanup::Unknown);
    assert!(survivors.iter().copied().any(live));
    // The fixed child exits by itself. No stale group/PID is signalled by cleanup.
    let deadline = Instant::now() + Duration::from_secs(5);
    while survivors.iter().copied().any(live) {
        assert!(Instant::now() < deadline);
        thread::sleep(Duration::from_millis(5));
    }
}
