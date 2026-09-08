use serde_json::{Value, json};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

struct Probe {
    directory: PathBuf,
    daemon: Option<Child>,
}

fn request(directory: &Path, value: Value) -> Value {
    let mut socket = UnixStream::connect(directory.join("host.sock")).unwrap();
    socket
        .set_read_timeout(Some(Duration::from_secs(8)))
        .unwrap();
    serde_json::to_writer(&mut socket, &value).unwrap();
    socket.write_all(b"\n").unwrap();
    let mut response = String::new();
    BufReader::new(socket).read_line(&mut response).unwrap();
    serde_json::from_str(&response).unwrap()
}

fn wait_until(mut predicate: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(6);
    loop {
        if predicate() {
            return;
        }
        assert!(Instant::now() < deadline, "condition timed out");
        thread::sleep(Duration::from_millis(10));
    }
}

impl Probe {
    fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory =
            std::env::temp_dir().join(format!("symbiote-host-{}-{nonce}", std::process::id()));
        fs::create_dir(&directory).unwrap();
        fs::set_permissions(&directory, fs::Permissions::from_mode(0o700)).unwrap();
        Self {
            directory,
            daemon: None,
        }
    }
    fn launch(&mut self) {
        self.daemon = Some(
            Command::new(env!("CARGO_BIN_EXE_symbiote-host-lifetime-probe"))
                .arg("serve")
                .arg(&self.directory)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::inherit())
                .spawn()
                .unwrap(),
        );
        wait_until(|| UnixStream::connect(self.directory.join("host.sock")).is_ok());
        assert_eq!(self.call(json!({"command":"status"}))["ok"], true);
    }
    fn call(&self, value: Value) -> Value {
        request(&self.directory, value)
    }
    fn crash(&mut self) {
        let daemon = self.daemon.as_mut().unwrap();
        daemon.kill().unwrap();
        daemon.wait().unwrap();
        self.daemon = None;
    }
    fn stop(&mut self) {
        assert_eq!(self.call(json!({"command":"shutdown"}))["ok"], true);
        self.daemon.as_mut().unwrap().wait().unwrap();
        self.daemon = None;
    }
}
impl Drop for Probe {
    fn drop(&mut self) {
        if let Some(mut daemon) = self.daemon.take() {
            // Only the directly owned Child is killed. Its workload guardian owns
            // any descendant cleanup; tests never scan and kill global PIDs.
            let _ = daemon.kill();
            let _ = daemon.wait();
            thread::sleep(Duration::from_millis(100));
        }
        let _ = fs::remove_dir_all(&self.directory);
    }
}

fn token_processes(token: &str) -> Vec<u32> {
    fs::read_dir("/proc")
        .unwrap()
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let pid: u32 = entry.file_name().to_str()?.parse().ok()?;
            let bytes = fs::read(entry.path().join("cmdline")).ok()?;
            let args: Vec<_> = bytes.split(|b| *b == 0).collect();
            (args.get(1) == Some(&b"workload".as_slice()) && args.get(3) == Some(&token.as_bytes()))
                .then_some(pid)
        })
        .collect()
}

#[test]
fn two_clients_disconnect_reconnect_and_cancel_entire_fixed_process_tree() {
    let mut probe = Probe::new();
    probe.launch();
    let initial = probe.call(json!({"command":"events","after":0}));
    let cursor = initial["cursor"].as_u64().unwrap();
    let start = probe.call(json!({"command":"start"}));
    assert_eq!(start["ok"], true, "{start}");
    let token = start["group"]["token"].as_str().unwrap().to_string();
    assert_eq!(token_processes(&token).len(), 3);
    // The start client's connection is already closed. Two independent clients
    // connect concurrently and see one canonical ongoing fixed workload.
    let first = probe.directory.clone();
    let second = probe.directory.clone();
    let a = thread::spawn(move || request(&first, json!({"command":"status"})));
    let b = thread::spawn(move || request(&second, json!({"command":"events","after":cursor})));
    assert_eq!(a.join().unwrap()["status"], "running");
    let replay = b.join().unwrap();
    assert_eq!(replay["events"][0]["sequence"], cursor + 1);
    assert_eq!(replay["events"][1]["kind"], "workload_ready");
    thread::sleep(Duration::from_millis(75));
    assert_eq!(
        token_processes(&token).len(),
        3,
        "work must survive all client disconnects"
    );
    assert_eq!(probe.call(json!({"command":"start"}))["ok"], false);
    assert_eq!(probe.call(json!({"command":"cancel"}))["ok"], true);
    wait_until(|| token_processes(&token).is_empty());
    let last = probe.call(json!({"command":"events","after":replay["cursor"]}));
    assert_eq!(last["events"][0]["kind"], "workload_cancelled");
    assert_eq!(last["status"], "idle");
    probe.stop();
}

#[test]
fn daemon_sigkill_closes_guardian_lease_and_restart_preserves_event_cursor() {
    let mut probe = Probe::new();
    probe.launch();
    let start = probe.call(json!({"command":"start"}));
    assert_eq!(start["ok"], true);
    let token = start["group"]["token"].as_str().unwrap().to_string();
    let cursor = start["cursor"].as_u64().unwrap();
    let journal_before: Value =
        serde_json::from_slice(&fs::read(probe.directory.join("journal.json")).unwrap()).unwrap();
    assert!(journal_before["active"].is_object());
    probe.crash();
    wait_until(|| token_processes(&token).is_empty());
    probe.launch();
    let replay = probe.call(json!({"command":"events","after":cursor}));
    assert_eq!(replay["events"][0]["kind"], "recovery_confirmed_exit");
    assert_eq!(replay["events"][0]["sequence"], cursor + 1);
    assert_eq!(replay["status"], "idle");
    assert!(replay["group"].is_null());
    let new_run = probe.call(json!({"command":"start"}));
    assert_eq!(new_run["ok"], true);
    assert_ne!(new_run["group"]["token"], start["group"]["token"]);
    probe.stop();
}

#[test]
fn stale_journal_identity_never_signals_an_unrelated_live_process() {
    let mut probe = Probe::new();
    // Our own test runner PID is deliberately stored with a bogus old identity.
    // A blind recovery kill would kill the tests; recovery must only observe.
    let journal = json!({"version":1,"active":{"pid":std::process::id(),"start_ticks":1,"token":"not-an-owned-workload"},
        "events":[{"sequence":1,"kind":"workload_started","detail":"stale identity fixture"}]});
    fs::write(
        probe.directory.join("journal.json"),
        serde_json::to_vec(&journal).unwrap(),
    )
    .unwrap();
    probe.launch();
    assert_eq!(probe.call(json!({"command":"status"}))["status"], "idle");
    assert!(Path::new(&format!("/proc/{}", std::process::id())).exists());
    let events = probe.call(json!({"command":"events","after":1}));
    assert_eq!(events["events"][0]["kind"], "recovery_confirmed_exit");
    probe.stop();
}

#[test]
fn malformed_requests_and_future_cursors_do_not_launch_work() {
    let mut probe = Probe::new();
    probe.launch();
    for value in [
        json!({"command":"exec","program":"sh"}),
        json!({"command":"start","program":"sh"}),
        json!({"command":"events","after":9999}),
    ] {
        let result = probe.call(value);
        assert_eq!(result["ok"], false);
        assert_eq!(result["status"], "idle");
    }
    probe.stop();
}

#[test]
fn invalid_journal_is_preserved_and_second_daemon_cannot_take_socket() {
    let mut probe = Probe::new();
    probe.launch();
    let second = Command::new(env!("CARGO_BIN_EXE_symbiote-host-lifetime-probe"))
        .arg("serve")
        .arg(&probe.directory)
        .output()
        .unwrap();
    assert!(!second.status.success());
    assert_eq!(probe.call(json!({"command":"status"}))["ok"], true);
    probe.stop();
    fs::write(probe.directory.join("journal.json"), b"incomplete journal").unwrap();
    let failure = Command::new(env!("CARGO_BIN_EXE_symbiote-host-lifetime-probe"))
        .arg("serve")
        .arg(&probe.directory)
        .output()
        .unwrap();
    assert!(!failure.status.success());
    assert_eq!(
        fs::read(probe.directory.join("journal.json")).unwrap(),
        b"incomplete journal"
    );
}

#[test]
fn nonprivate_directory_is_rejected() {
    let probe = Probe::new();
    fs::set_permissions(&probe.directory, fs::Permissions::from_mode(0o755)).unwrap();
    let failure = Command::new(env!("CARGO_BIN_EXE_symbiote-host-lifetime-probe"))
        .arg("serve")
        .arg(&probe.directory)
        .output()
        .unwrap();
    assert!(!failure.status.success());
    assert!(!probe.directory.join("host.sock").exists());
}

#[test]
fn unresponsive_guardian_blocks_recovery_without_blind_cleanup() {
    let mut probe = Probe::new();
    probe.launch();
    let start = probe.call(json!({"command":"start","fault":"ignore_guardian_eof"}));
    let root = start["group"]["pid"].as_u64().unwrap() as i32;
    let token = start["group"]["token"].as_str().unwrap().to_string();
    assert!(token_processes(&token).contains(&(root as u32)));
    // The explicit fixed fault disables only this workload's EOF guardian. This
    // test owns the live group and cleans it up; daemon recovery never signals it.
    struct CleanupGroup(PathBuf);
    impl Drop for CleanupGroup {
        fn drop(&mut self) {
            let _ = fs::write(&self.0, b"stop");
        }
    }
    let cleanup = CleanupGroup(probe.directory.join(format!("stop-{token}")));
    probe.crash();
    probe.launch();
    let status = probe.call(json!({"command":"status"}));
    assert_eq!(status["status"], "recovery_required");
    assert_eq!(
        token_processes(&token).len(),
        3,
        "recovery must not signal journal process IDs"
    );
    assert_eq!(probe.call(json!({"command":"start"}))["ok"], false);
    assert_eq!(probe.call(json!({"command":"cancel"}))["ok"], false);
    drop(cleanup);
    wait_until(|| token_processes(&token).is_empty());
    probe.stop();
    probe.launch();
    assert_eq!(probe.call(json!({"command":"status"}))["status"], "idle");
    probe.stop();
}
