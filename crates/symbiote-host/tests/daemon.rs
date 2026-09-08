use serde_json::{Value, json};
use std::io::Write;
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use symbiote_host::transport::{exchange, private_directory};

static NEXT: AtomicU64 = AtomicU64::new(0);
struct Host {
    directory: PathBuf,
    child: Option<Child>,
}
impl Host {
    fn new() -> Self {
        let directory = std::env::temp_dir().join(format!(
            "symbiote-daemon-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        assert!(!directory.exists());
        private_directory(&directory).unwrap();
        let mut host = Self {
            directory,
            child: None,
        };
        host.start();
        host
    }
    fn start(&mut self) {
        self.child = Some(
            Command::new(env!("CARGO_BIN_EXE_symbioted"))
                .arg("--state-dir")
                .arg(&self.directory)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .unwrap(),
        );
        let until = Instant::now() + Duration::from_secs(5);
        while !self.directory.join("host.sock").exists() {
            assert!(
                self.child.as_mut().unwrap().try_wait().unwrap().is_none(),
                "daemon failed startup"
            );
            assert!(Instant::now() < until, "daemon did not become ready");
            std::thread::sleep(Duration::from_millis(10));
        }
        // A stale socket from SIGKILL may exist briefly until the new lock holder binds.
        loop {
            if exchange(
                &self.directory,
                &serde_json::to_vec(&request("health", json!({"kind":"health"}))).unwrap(),
            )
            .is_ok()
            {
                break;
            }
            assert!(Instant::now() < until, "daemon did not answer health");
            std::thread::sleep(Duration::from_millis(10));
        }
    }
    fn crash(&mut self) {
        let mut child = self.child.take().unwrap();
        child.kill().unwrap();
        child.wait().unwrap();
    }
    fn call(&self, request: Value) -> Value {
        serde_json::from_slice(
            &exchange(&self.directory, &serde_json::to_vec(&request).unwrap()).unwrap(),
        )
        .unwrap()
    }
}
impl Drop for Host {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        let _ = std::fs::remove_dir_all(&self.directory);
    }
}
fn request(command: &str, operation: Value) -> Value {
    json!({"version":{"major":1,"minor":0},"correlation_id":"test-request","command_id":command,"operation":operation})
}
fn project(id: &str) -> Value {
    json!({"kind":"register_project","project":{"id":id,"name":id,"lead":format!("lead-{id}"),
        "roots":[{"id":format!("root-{id}"),"project_id":id,"revision":0,"repository":null,"host_paths":{}}],
        "roles":[{"id":format!("lead-{id}"),"project_id":id,"revision":0,"name":"Lead","operating_contract":{"id":"lead-contract","revision":1}}]}})
}
fn task(id: &str, project: &str) -> Value {
    json!({"kind":"create_task","task":{"id":id,"project_id":project,"root_id":format!("root-{project}"),"role_id":format!("lead-{project}"),
      "task_contract":{"id":"coding-contract","revision":1},"stream":{"id":format!("stream-{id}"),"originating_chat":"chat","worktree":format!("worktree-{id}"),
      "branch":format!("task/{id}"),"base":"a".repeat(40),"target":"a".repeat(40)}}})
}
fn ok(response: &Value) -> &Value {
    response
        .get("result")
        .unwrap()
        .get("Ok")
        .unwrap_or_else(|| panic!("unexpected response: {response}"))
}

#[test]
fn acknowledged_work_survives_sigkill_and_retries_keep_original_receipts() {
    let mut host = Host::new();
    let registration = request("register-alpha", project("alpha"));
    let first = host.call(registration.clone());
    assert_eq!(ok(&first)["kind"], "receipt");
    let created = host.call(request("create-task", task("task-one", "alpha")));
    assert_eq!(ok(&created)["data"]["sequence"], 2);
    host.crash();
    host.start();
    let mut retry = registration.clone();
    retry["correlation_id"] = json!("retry-after-crash");
    let replay = host.call(retry);
    assert_eq!(ok(&replay)["data"]["sequence"], 1);
    assert_eq!(ok(&replay)["data"]["replayed"], true);
    let task = host.call(request(
        "read-task",
        json!({"kind":"get_task","project_id":"alpha","task_id":"task-one"}),
    ));
    assert_eq!(ok(&task)["data"]["state"], "ready");
    let page = host.call(request(
        "read-journal",
        json!({"kind":"read_journal","project_id":"alpha","after":1,"limit":10}),
    ));
    assert_eq!(ok(&page)["data"]["events"].as_array().unwrap().len(), 1);
    assert_eq!(ok(&page)["data"]["next_cursor"], 2);
    let mut conflict = registration;
    conflict["operation"]["project"]["name"] = json!("different intent");
    assert_eq!(
        host.call(conflict)["result"]["Err"]["code"],
        "idempotency_conflict"
    );
}

#[test]
fn project_lineage_and_wire_authority_are_enforced_and_bad_input_does_not_stop_host() {
    let host = Host::new();
    ok(&host.call(request("alpha", project("alpha"))));
    ok(&host.call(request("beta", project("beta"))));
    let mut wrong = task("wrong", "alpha");
    wrong["task"]["root_id"] = json!("root-beta");
    assert_eq!(
        host.call(request("wrong", wrong))["result"]["Err"]["code"],
        "invalid_request"
    );
    ok(&host.call(request("create", task("one", "alpha"))));
    assert_eq!(
        host.call(request(
            "read",
            json!({"kind":"get_task","project_id":"beta","task_id":"one"})
        ))["result"]["Err"]["code"],
        "permission_denied"
    );
    let mut spoofed = request("spoof", json!({"kind":"health"}));
    spoofed["actor"] = json!({"kind":"host","id":"attacker"});
    assert_eq!(
        host.call(spoofed)["result"]["Err"]["code"],
        "invalid_request"
    );
    assert_eq!(
        host.call(request(
            "complete",
            json!({"kind":"complete","task_id":"one"})
        ))["result"]["Err"]["code"],
        "invalid_request"
    );
    ok(&host.call(request("health", json!({"kind":"health"}))));
}

#[test]
fn concurrent_retry_and_dropped_reply_commit_once() {
    let host = Host::new();
    let bytes = serde_json::to_vec(&request("register", project("alpha"))).unwrap();
    let mut abandoned = UnixStream::connect(host.directory.join("host.sock")).unwrap();
    abandoned.write_all(&bytes).unwrap();
    abandoned.write_all(b"\n").unwrap();
    drop(abandoned);
    let mut callers = Vec::new();
    for _ in 0..2 {
        let directory = host.directory.clone();
        let bytes = bytes.clone();
        callers.push(std::thread::spawn(move || {
            serde_json::from_slice::<Value>(&exchange(&directory, &bytes).unwrap()).unwrap()
        }));
    }
    for caller in callers {
        assert_eq!(ok(&caller.join().unwrap())["data"]["sequence"], 1);
    }
    let journal = host.call(request(
        "journal",
        json!({"kind":"read_journal","project_id":"alpha","after":0,"limit":100}),
    ));
    assert_eq!(ok(&journal)["data"]["events"].as_array().unwrap().len(), 1);
}

#[test]
fn cli_attaches_and_explicit_shutdown_exits_cleanly() {
    let mut host = Host::new();
    let mut cli = Command::new(env!("CARGO_BIN_EXE_symbiote"))
        .arg("--state-dir")
        .arg(&host.directory)
        .arg("request")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    cli.stdin
        .take()
        .unwrap()
        .write_all(
            &serde_json::to_vec_pretty(&request("health", json!({"kind":"health"}))).unwrap(),
        )
        .unwrap();
    let result = cli.wait_with_output().unwrap();
    assert!(result.status.success());
    assert_eq!(
        ok(&serde_json::from_slice::<Value>(&result.stdout).unwrap())["kind"],
        "hello"
    );
    ok(&host.call(request("shutdown", json!({"kind":"shutdown"}))));
    assert!(host.child.take().unwrap().wait().unwrap().success());
    assert!(!host.directory.join("host.sock").exists());
}

#[test]
fn cli_reports_rpc_failure_and_rejects_duplicate_fields_before_transmission() {
    let host = Host::new();
    let missing = serde_json::to_vec(&request(
        "missing",
        json!({"kind":"get_project","project_id":"missing"}),
    ))
    .unwrap();
    let duplicate = br#"{"version":{"major":1,"minor":0},"correlation_id":"one","command_id":"one","operation":{"kind":"health"},"operation":{"kind":"shutdown"}}"#.to_vec();
    for (input, rpc_response) in [(missing, true), (duplicate, false)] {
        let mut cli = Command::new(env!("CARGO_BIN_EXE_symbiote"))
            .arg("--state-dir")
            .arg(&host.directory)
            .arg("request")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        cli.stdin.take().unwrap().write_all(&input).unwrap();
        let result = cli.wait_with_output().unwrap();
        assert!(!result.status.success());
        if rpc_response {
            assert_eq!(
                serde_json::from_slice::<Value>(&result.stdout).unwrap()["result"]["Err"]["code"],
                "not_found"
            );
        } else {
            assert!(result.stdout.is_empty());
        }
    }
    ok(&host.call(request("still-alive", json!({"kind":"health"}))));
}
