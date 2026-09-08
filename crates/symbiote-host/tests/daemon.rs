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
    telemetry: bool,
}
impl Host {
    fn new() -> Self {
        Self::with_telemetry(true)
    }
    fn with_telemetry(telemetry: bool) -> Self {
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
            telemetry,
        };
        host.start();
        host
    }
    fn start(&mut self) {
        let mut command = Command::new(env!("CARGO_BIN_EXE_symbioted"));
        command.arg("--state-dir").arg(&self.directory);
        if !self.telemetry {
            command.arg("--no-telemetry");
        }
        self.child = Some(
            command
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

#[test]
fn pulse_is_cached_identity_survives_restart_and_disabled_telemetry_stays_unknown() {
    let mut host = Host::new();
    let request_pulse = || request("pulse", json!({"kind":"get_host_pulse"}));
    let first = host.call(request_pulse());
    let pulse = ok(&first)["data"].clone();
    assert_eq!(ok(&first)["kind"], "host_pulse");
    assert_eq!(
        pulse["resources"]["effective_memory_available_bytes"]["status"],
        "unknown"
    );
    assert_eq!(
        pulse["resources"]["effective_cpu_millicores"]["status"],
        "unknown"
    );
    assert_eq!(ok(&host.call(request_pulse()))["data"], pulse);
    host.crash();
    host.start();
    let fresh = host.call(request_pulse());
    assert_eq!(ok(&fresh)["data"]["host_id"], pulse["host_id"]);
    assert_ne!(
        ok(&fresh)["data"]["observation_id"],
        pulse["observation_id"]
    );
    host.crash();
    host.telemetry = false;
    host.start();
    let disabled = host.call(request_pulse());
    assert_eq!(ok(&disabled)["data"]["telemetry"], "disabled");
    for fact in ok(&disabled)["data"]["resources"]
        .as_object()
        .unwrap()
        .values()
    {
        assert_eq!(fact["status"], "unknown");
    }
    assert_eq!(ok(&disabled)["data"]["host_id"], pulse["host_id"]);
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
    json!({"version":{"major":1,"minor":4},"correlation_id":"test-request","command_id":command,"operation":operation})
}

fn team(project: &str, revision: u64) -> Value {
    let access = json!({"project_id":project,"roots":[format!("root-{project}")],"grants":["read_root","execute_process"],"policy_revision":1});
    let member = |prefix: &str, function: &str, reviewer: &str| {
        json!({
            "role_id":format!("{prefix}-{project}"),"function":function,
            "responsibilities":["Implement and verify bounded work"],"task_domains":["coding"],
            "access":access,"context_policy_ref":"context","tool_policy_ref":"tools",
            "skill_policy_ref":"skills","execution_policy_ref":"execution",
            "independent_reviewers":[format!("{reviewer}-{project}")],"fallbacks":[]
        })
    };
    json!({"schema_version":1,"project_id":project,"revision":revision,
        "lead_role_id":format!("lead-{project}"),"access_ceiling":access,
        "members":[member("lead","lead_orchestrator","worker"),member("worker","general_execution","lead")]})
}

#[test]
fn team_revisions_survive_crash_and_do_not_leak_between_projects() {
    let mut host = Host::new();
    for id in ["staff-a", "staff-b"] {
        let mut registration = project(id);
        registration["project"]["roles"]
            .as_array_mut()
            .unwrap()
            .push(json!({
                "id":format!("worker-{id}"),"project_id":id,"revision":0,"name":"Engineer",
                "operating_contract":{"id":"worker-contract","revision":1}
            }));
        ok(&host.call(request(&format!("register-{id}"), registration)));
    }
    let replace = |command: &str, revision: Option<u64>, configuration: Value| {
        request(
            command,
            json!({"kind":"replace_team","expected_revision":revision,"team":configuration}),
        )
    };
    let first = replace("team-initial", None, team("staff-a", 0));
    ok(&host.call(first.clone()));
    host.crash();
    host.start();
    assert_eq!(ok(&host.call(first))["data"]["replayed"], true);
    assert_eq!(
        host.call(request(
            "other",
            json!({"kind":"get_team","project_id":"staff-b"})
        ))["result"]["Err"]["code"],
        "not_found"
    );
    let mut update = team("staff-a", 1);
    update["members"][1]["responsibilities"] = json!(["Review and implement scoped changes"]);
    ok(&host.call(replace("team-update", Some(0), update.clone())));
    assert_eq!(
        host.call(replace("stale-team", Some(0), update.clone()))["result"]["Err"]["code"],
        "stale_revision"
    );
    let mut foreign = team("staff-a", 2);
    foreign["access_ceiling"]["roots"] = json!(["root-staff-b"]);
    for member in foreign["members"].as_array_mut().unwrap() {
        member["access"]["roots"] = json!(["root-staff-b"]);
    }
    assert_eq!(
        host.call(replace("foreign-root", Some(1), foreign))["result"]["Err"]["code"],
        "invalid_request"
    );
    host.crash();
    host.start();
    let read = host.call(request(
        "read-team",
        json!({"kind":"get_team","project_id":"staff-a"}),
    ));
    assert_eq!(ok(&read)["data"], update);
    let journal = host.call(request(
        "team-events",
        json!({"kind":"read_journal","project_id":"staff-a","after":0,"limit":100}),
    ));
    assert_eq!(ok(&journal)["data"]["events"].as_array().unwrap().len(), 3);
}
fn project(id: &str) -> Value {
    json!({"kind":"register_project","project":{"id":id,"name":id,"lead":format!("lead-{id}"),
        "roots":[{"id":format!("root-{id}"),"project_id":id,"revision":0,"repository":null,"host_paths":{}}],
        "roles":[{"id":format!("lead-{id}"),"project_id":id,"revision":0,"name":"Lead","operating_contract":{"id":"lead-contract","revision":1}}]}})
}
fn task(id: &str, project: &str) -> Value {
    json!({"kind":"create_task","task":{"id":id,"project_id":project,"root_id":format!("root-{project}"),"role_id":format!("lead-{project}"),
      "origin":{"kind":"objective","work":{"project_id":project,"id":{"kind":"objective","id":format!("maintenance-{project}")}}},
      "task_contract":{"id":"coding-contract","revision":1},"stream":{"id":format!("stream-{id}"),"originating_chat":"chat","worktree":format!("worktree-{id}"),
      "branch":format!("task/{id}"),"base":"a".repeat(40),"target":"a".repeat(40)}}})
}
fn maintenance(project: &str) -> Value {
    json!({"kind":"create_work","work":{"id":{"kind":"objective","id":format!("maintenance-{project}")},
        "project_id":project,"role_id":format!("lead-{project}"),"title":"Maintenance fixture","description":"Explicit operational test work",
        "utterance":null,"objective_class":"maintenance","parent":null,"dependencies":[],"requirements":[],"constraints":[],"risks":[],
        "acceptance":["fixture verified"],"priority":2,"budget":null,"external_references":[]}})
}

#[test]
fn work_hierarchy_replays_after_restart_and_never_accepts_client_completion() {
    let mut host = Host::new();
    ok(&host.call(request("register-work", project("work"))));
    let create = request("create-maintenance", maintenance("work"));
    let first = host.call(create.clone());
    ok(&first);
    host.crash();
    host.start();
    assert_eq!(ok(&host.call(create))["data"]["replayed"], true);
    let id = json!({"kind":"objective","id":"maintenance-work"});
    let change = |command: &str, revision: u64, edit: Value| {
        request(
            command,
            json!({
                "kind":"change_work","project_id":"work","id":id,"expected_revision":revision,"edit":edit
            }),
        )
    };
    let approve_request = change("request-approval", 0, json!({"kind":"request_approval"}));
    ok(&host.call(approve_request.clone()));
    host.crash();
    host.start();
    assert_eq!(ok(&host.call(approve_request))["data"]["replayed"], true);
    assert_eq!(
        host.call(change("stale", 0, json!({"kind":"approve"})))["result"]["Err"]["code"],
        "stale_revision"
    );
    for (revision, edit) in [
        (1, json!({"kind":"approve"})),
        (2, json!({"kind":"start"})),
        (3, json!({"kind":"request_completion"})),
    ] {
        ok(&host.call(change(&format!("edit-{revision}"), revision, edit)));
    }
    assert_eq!(
        host.call(change(
            "forged-complete",
            4,
            json!({"kind":"complete","evidence":{}})
        ))["result"]["Err"]["code"],
        "invalid_request"
    );
    ok(&host.call(change(
        "cancel",
        4,
        json!({"kind":"cancel","reason":"scope changed"}),
    )));
    ok(&host.call(change(
        "reopen",
        5,
        json!({"kind":"reopen","reason":"new requirements"}),
    )));
    let result = host.call(request(
        "get-work",
        json!({"kind":"get_work","project_id":"work","id":id}),
    ));
    assert_eq!(ok(&result)["data"]["state"], "draft");
    assert_eq!(ok(&result)["data"]["revision"], 6);
    assert_eq!(ok(&result)["data"]["history"].as_array().unwrap().len(), 6);
    assert!(
        ok(&result)["data"]["created_by"]
            .as_str()
            .unwrap()
            .starts_with("local-uid-")
    );
    let mut forged = maintenance("work");
    forged["created_by"] = json!("forged");
    assert_eq!(
        host.call(request("forged-creator", forged))["result"]["Err"]["code"],
        "invalid_request"
    );
}
fn ok(response: &Value) -> &Value {
    response
        .get("result")
        .unwrap()
        .get("Ok")
        .unwrap_or_else(|| panic!("unexpected response: {response}"))
}

fn resource_consent(project: &str) -> Value {
    json!({"kind":"record_resource_consent","expires_at":4_102_444_800_000_u64,
        "snapshot":{"project_id":project,"role_id":format!("lead-{project}"),"profile_id":"profile-fixture",
        "host_id":"host-fixture","resource_ref":"tool-fixture","fingerprint":"a".repeat(64),
        "access":{"project_id":project,"roots":[format!("root-{project}")],"grants":["read_root"],"policy_revision":1}}})
}

#[test]
fn resource_consent_and_revocation_survive_daemon_death_with_original_receipts() {
    let mut host = Host::new();
    ok(&host.call(request(
        "register-consent-project",
        project("consent-project"),
    )));
    let command = request("approve-fixture", resource_consent("consent-project"));
    let first = host.call(command.clone());
    assert_eq!(ok(&first)["kind"], "receipt");
    host.crash();
    host.start();
    let replay = host.call(command);
    assert_eq!(
        ok(&replay)["data"]["sequence"],
        ok(&first)["data"]["sequence"]
    );
    assert_eq!(ok(&replay)["data"]["replayed"], true);
    let query = request(
        "read-consent",
        json!({"kind":"get_resource_consent","project_id":"consent-project","consent_id":"approve-fixture"}),
    );
    let stored = host.call(query.clone());
    assert_eq!(ok(&stored)["kind"], "resource_consent");
    assert!(ok(&stored)["data"]["issued_at"].as_u64().unwrap() > 0);
    assert!(ok(&stored)["data"]["user_id"].is_string());
    let revoke = request(
        "revoke-fixture",
        json!({"kind":"revoke_resource_consent","project_id":"consent-project","consent_id":"approve-fixture"}),
    );
    let revoked = host.call(revoke.clone());
    ok(&revoked);
    host.crash();
    host.start();
    assert_eq!(ok(&host.call(revoke))["data"]["replayed"], true);
    assert!(ok(&host.call(query))["data"]["revoked_at"].is_number());
    let journal = host.call(request(
        "read-consent-events",
        json!({"kind":"read_journal","project_id":"consent-project","after":0,"limit":10}),
    ));
    let events = ok(&journal)["data"]["events"].as_array().unwrap();
    assert_eq!(events.len(), 3);
    assert_eq!(events[1]["payload"]["kind"], "resource_consent_recorded");
    assert_eq!(events[2]["payload"]["kind"], "resource_consent_revoked");
    assert!(events[2]["payload"]["data"]["revoked_by"].is_string());
}

#[test]
fn resource_consent_rejects_spoofed_authority_cross_project_and_expired_requests() {
    let host = Host::new();
    ok(&host.call(request("register-rc-a", project("rc-a"))));
    ok(&host.call(request("register-rc-b", project("rc-b"))));
    for field in ["user_id", "issued_at", "revoked_at"] {
        let mut operation = resource_consent("rc-a");
        operation[field] = json!("forged");
        assert!(
            host.call(request(&format!("spoof-{field}"), operation))["result"]["Err"].is_object()
        );
    }
    let mut operation = resource_consent("rc-a");
    operation["snapshot"]["access"]["roots"] = json!(["root-rc-b"]);
    assert!(host.call(request("foreign-root", operation))["result"]["Err"].is_object());
    let mut expired = resource_consent("rc-a");
    expired["expires_at"] = json!(1);
    assert!(host.call(request("expired", expired))["result"]["Err"].is_object());
    ok(&host.call(request("rc-approved", resource_consent("rc-a"))));
    for kind in ["get_resource_consent", "revoke_resource_consent"] {
        assert!(
            host.call(request(
                kind,
                json!({"kind":kind,"project_id":"rc-b","consent_id":"rc-approved"})
            ))["result"]["Err"]
                .is_object()
        );
    }
}

#[test]
fn acknowledged_work_survives_sigkill_and_retries_keep_original_receipts() {
    let mut host = Host::new();
    let registration = request("register-alpha", project("alpha"));
    let first = host.call(registration.clone());
    assert_eq!(ok(&first)["kind"], "receipt");
    ok(&host.call(request("maintenance-alpha", maintenance("alpha"))));
    let created = host.call(request("create-task", task("task-one", "alpha")));
    assert_eq!(ok(&created)["data"]["sequence"], 3);
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
    assert_eq!(ok(&page)["data"]["events"].as_array().unwrap().len(), 2);
    assert_eq!(ok(&page)["data"]["next_cursor"], 3);
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
    ok(&host.call(request("maintenance-alpha", maintenance("alpha"))));
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
    let duplicate = br#"{"version":{"major":1,"minor":4},"correlation_id":"one","command_id":"one","operation":{"kind":"health"},"operation":{"kind":"shutdown"}}"#.to_vec();
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
