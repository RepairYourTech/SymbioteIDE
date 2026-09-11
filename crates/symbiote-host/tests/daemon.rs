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
    json!({"version":{"major":1,"minor":16},"correlation_id":"test-request","command_id":command,"operation":operation})
}

#[test]
fn bindings_replay_after_crash_and_survive_team_drift_as_desired_history() {
    let mut host = Host::new();
    for input in [
        include_str!("../../../fixtures/project-team/register.json"),
        include_str!("../../../fixtures/project-team/configure.json"),
    ] {
        ok(&host.call(serde_json::from_str(input).unwrap()));
    }
    let create: Value = serde_json::from_str(include_str!(
        "../../../fixtures/workforce-bindings/configure.json"
    ))
    .unwrap();
    ok(&host.call(create.clone()));
    host.crash();
    host.start();
    assert_eq!(ok(&host.call(create.clone()))["data"]["replayed"], true);
    let mut update = create.clone();
    update["command_id"] = json!("binding-update");
    update["operation"]["expected_revision"] = json!(0);
    update["operation"]["configuration"]["binding"]["revision"] = json!(1);
    update["operation"]["configuration"]["policies"]["root_effort"] = json!("high");
    ok(&host.call(update.clone()));
    update["command_id"] = json!("binding-stale");
    assert_eq!(host.call(update)["result"]["Err"]["code"], "stale_revision");
    let mut team: Value = serde_json::from_str(include_str!(
        "../../../fixtures/project-team/configure.json"
    ))
    .unwrap();
    team["command_id"] = json!("team-drift");
    team["operation"]["expected_revision"] = json!(0);
    team["operation"]["team"]["revision"] = json!(1);
    ok(&host.call(team));
    host.crash();
    host.start();
    let read: Value = serde_json::from_str(include_str!(
        "../../../fixtures/workforce-bindings/read.json"
    ))
    .unwrap();
    let result = host.call(read);
    assert_eq!(ok(&result)["data"]["team_revision"], 0);
    assert_eq!(ok(&result)["data"]["binding"]["revision"], 1);
    let readiness: Value = serde_json::from_str(include_str!(
        "../../../fixtures/workforce-bindings/readiness.json"
    ))
    .unwrap();
    let status = host.call(readiness);
    assert_eq!(ok(&status)["data"]["status"], "not_ready");
    assert!(
        !ok(&status)["data"]["activation_pending"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(ok(&host.call(create))["data"]["replayed"], true);
    let wrong = request(
        "wrong-project",
        json!({"kind":"get_binding","project_id":"other","binding_id":"engineer-binding"}),
    );
    assert_eq!(host.call(wrong)["result"]["Err"]["code"], "not_found");
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
fn ok_step<'a>(step: &'static str, response: &'a Value) -> &'a Value {
    response
        .get("result")
        .unwrap()
        .get("Ok")
        .unwrap_or_else(|| panic!("step {step} failed: {response}"))
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
    let result = Command::new(env!("CARGO_BIN_EXE_symbiote"))
        .arg("--state-dir")
        .arg(&host.directory)
        .arg("health")
        .stdout(Stdio::piped())
        .output()
        .unwrap();
    assert!(result.status.success());
    let body: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(body["kind"], "hello");
    // Host pulse is owner-authorized over the same socket.
    let pulse = Command::new(env!("CARGO_BIN_EXE_symbiote"))
        .arg("--state-dir")
        .arg(&host.directory)
        .arg("host-pulse")
        .stdout(Stdio::piped())
        .output()
        .unwrap();
    assert!(pulse.status.success());
    let body: Value = serde_json::from_slice(&pulse.stdout).unwrap();
    assert_eq!(body["kind"], "host_pulse");
    ok(&host.call(request("shutdown", json!({"kind":"shutdown"}))));
    assert!(host.child.take().unwrap().wait().unwrap().success());
    assert!(!host.directory.join("host.sock").exists());
}

#[test]
fn cli_exit_codes_distinguish_daemon_refusal_from_usage_and_transport() {
    let host = Host::new();
    // Daemon refusal (unknown project): exit 2 with the typed error on
    // stderr, nothing on stdout.
    let refused = Command::new(env!("CARGO_BIN_EXE_symbiote"))
        .arg("--state-dir")
        .arg(&host.directory)
        .args(["get-project", "missing"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .unwrap();
    assert_eq!(refused.status.code(), Some(2));
    assert!(refused.stdout.is_empty());
    let error: Value = serde_json::from_slice(&refused.stderr).unwrap();
    assert_eq!(error["code"], "not_found");
    // Usage failure (unknown command, missing args): exit 1 before any
    // connection attempt.
    for arguments in [
        vec![
            "--state-dir".to_string(),
            host.directory.display().to_string(),
            "no-such-command".to_string(),
        ],
        vec![
            "--state-dir".to_string(),
            host.directory.display().to_string(),
            "get-task".to_string(),
        ],
        vec![
            "--state-dir".to_string(),
            host.directory.display().to_string(),
            "get-task".to_string(),
            "p".to_string(),
        ],
    ] {
        let result = Command::new(env!("CARGO_BIN_EXE_symbiote"))
            .args(&arguments)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .unwrap();
        assert_eq!(result.status.code(), Some(1));
        assert!(result.stdout.is_empty());
    }
    // Raw mode types through: a valid operation file succeeds.
    let duplicate = host.directory.join("dup.json");
    std::fs::write(&duplicate, br#"{"kind":"health"}"#).unwrap();
    let raw_ok = Command::new(env!("CARGO_BIN_EXE_symbiote"))
        .arg("--state-dir")
        .arg(&host.directory)
        .args(["raw"])
        .arg(&duplicate)
        .stdout(Stdio::piped())
        .output()
        .unwrap();
    assert!(raw_ok.status.success());
    ok(&host.call(request("still-alive", json!({"kind":"health"}))));
}

#[test]
fn routes_resolve_deterministically_and_survive_restart_with_provenance() {
    let mut host = Host::new();
    for input in [
        include_str!("../../../fixtures/role-resolution/register.json"),
        include_str!("../../../fixtures/role-resolution/team.json"),
        include_str!("../../../fixtures/role-resolution/work.json"),
    ] {
        ok(&host.call(serde_json::from_str(input).unwrap()));
    }
    let classify: Value = serde_json::from_str(include_str!(
        "../../../fixtures/role-resolution/resolve-classify.json"
    ))
    .unwrap();
    let resolved = ok(&host.call(classify.clone()))["data"].clone();
    assert_eq!(resolved["resolved"], "routing-engineer");
    assert_eq!(resolved["reason"], "exact_domain_match");
    assert_eq!(resolved["confidence"], "deterministic");
    assert_eq!(resolved["requested"], Value::Null);
    // Deterministic: the same request yields the same decision.
    assert_eq!(ok(&host.call(classify.clone()))["data"], resolved);
    let explicit: Value = serde_json::from_str(include_str!(
        "../../../fixtures/role-resolution/resolve-explicit.json"
    ))
    .unwrap();
    let explicit_decision = ok(&host.call(explicit.clone()))["data"].clone();
    assert_eq!(explicit_decision["resolved"], "routing-engineer");
    assert_eq!(explicit_decision["reason"], "explicit_request");
    assert_eq!(explicit_decision["requested"], "routing-engineer");
    // An explicit Lead assignment is honored regardless of domain coverage.
    let mut lead_assignment = explicit.clone();
    lead_assignment["command_id"] = json!("resolve-lead");
    lead_assignment["operation"]["request"]["requested"] = json!("routing-lead");
    let lead_decision = ok(&host.call(lead_assignment))["data"].clone();
    assert_eq!(lead_decision["resolved"], "routing-lead");
    assert_eq!(lead_decision["reason"], "explicit_request");
    // Unknown Roles are diagnosed, never silently substituted.
    let mut unknown = explicit.clone();
    unknown["command_id"] = json!("resolve-unknown");
    unknown["operation"]["request"]["requested"] = json!("ghost");
    let diagnosed = ok(&host.call(unknown))["data"].clone();
    assert_eq!(diagnosed["resolved"], Value::Null);
    assert_eq!(diagnosed["diagnosis"]["kind"], "requested_role_not_member");
    // No executable Role covers frontend; the diagnosis names the gap.
    let noroute: Value = serde_json::from_str(include_str!(
        "../../../fixtures/role-resolution/resolve-noroute.json"
    ))
    .unwrap();
    let diagnosed = ok(&host.call(noroute))["data"].clone();
    assert_eq!(diagnosed["resolved"], Value::Null);
    assert_eq!(diagnosed["diagnosis"]["kind"], "no_executable_role");
    assert_eq!(
        diagnosed["diagnosis"]["domains"].as_array().unwrap().len(),
        1
    );
    // Recording persists the host-computed decision.
    let record: Value = serde_json::from_str(include_str!(
        "../../../fixtures/role-resolution/record.json"
    ))
    .unwrap();
    let first_receipt = ok(&host.call(record.clone()))["data"].clone();
    assert_eq!(first_receipt["replayed"], false);
    assert_eq!(ok(&host.call(record))["data"]["replayed"], true);
    host.crash();
    host.start();
    // Reassignment supersedes the classified route with an explicit one.
    let reassign: Value = serde_json::from_str(include_str!(
        "../../../fixtures/role-resolution/record-reassign.json"
    ))
    .unwrap();
    ok(&host.call(reassign));
    let get: Value =
        serde_json::from_str(include_str!("../../../fixtures/role-resolution/get.json")).unwrap();
    let decision = ok(&host.call(get.clone()))["data"].clone();
    assert_eq!(decision["resolved"], "routing-lead");
    assert_eq!(decision["reason"], "explicit_request");
    assert_eq!(decision["requested"], "routing-lead");
    host.crash();
    host.start();
    assert_eq!(ok(&host.call(get))["data"], decision);
}

#[test]
fn task_dependencies_persist_block_completion_and_reject_cycles() {
    let mut host = Host::new();
    ok(&host.call(request("register-dag", project("dag"))));
    ok(&host.call(request("dag-maintenance", maintenance("dag"))));
    ok(&host.call(request("dag-task-b", task("task-b", "dag"))));
    ok(&host.call(request("dag-task-a", task("task-a", "dag"))));
    // A requires B: recorded, readable, and replayed after restart.
    let set = |command: &str, task_id: &str, deps: Value| {
        request(
            command,
            json!({"kind":"set_task_dependencies","project_id":"dag","task_id":task_id,"dependencies":deps}),
        )
    };
    let set_a = set(
        "dep-a-requires-b",
        "task-a",
        json!([{"kind":"requires","target":{"project_id":"dag","task_id":"task-b"}}]),
    );
    ok(&host.call(set_a.clone()));
    host.crash();
    host.start();
    assert_eq!(ok(&host.call(set_a))["data"]["replayed"], true);
    let read = request(
        "dep-read-a",
        json!({"kind":"get_task_dependencies","project_id":"dag","task_id":"task-a"}),
    );
    let edges = ok(&host.call(read))["data"].as_array().unwrap().clone();
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0]["kind"], "requires");
    assert_eq!(edges[0]["target"]["task_id"], "task-b");
    // Closing the cycle is rejected with a typed error and the Host keeps serving.
    let closing = set(
        "dep-b-requires-a",
        "task-b",
        json!([{"kind":"requires","target":{"project_id":"dag","task_id":"task-a"}}]),
    );
    assert_eq!(host.call(closing)["result"]["Err"]["code"], "conflict");
    ok(&host.call(request("dag-alive", json!({"kind":"health"}))));
    // Self-edges are invalid requests.
    let self_edge = set(
        "dep-self",
        "task-a",
        json!([{"kind":"requires","target":{"project_id":"dag","task_id":"task-a"}}]),
    );
    assert_eq!(
        host.call(self_edge)["result"]["Err"]["code"],
        "invalid_request"
    );
    // Non-blocking kinds are recorded without gating completion, but they
    // still participate in DAG acyclicity (task-b waits on task-a here while
    // task-a requires task-b would be a genuine cycle), so use a fresh pair.
    ok(&host.call(request("dag-task-c", task("task-c", "dag"))));
    ok(&host.call(set(
        "dep-b-followup-c",
        "task-b",
        json!([{"kind":"follow_up_to","target":{"project_id":"dag","task_id":"task-c"}}]),
    )));
    let read_b = request(
        "dep-read-b",
        json!({"kind":"get_task_dependencies","project_id":"dag","task_id":"task-b"}),
    );
    let edges_b = ok(&host.call(read_b))["data"].as_array().unwrap().clone();
    assert_eq!(edges_b.len(), 1);
    assert_eq!(edges_b[0]["kind"], "follow_up_to");
    assert_eq!(edges_b[0]["target"]["task_id"], "task-c");
    // Journal surfaces the dependency event with lineage intact.
    let journal_response = host.call(request(
        "dag-journal",
        json!({"kind":"read_journal","project_id":"dag","after":0,"limit":100}),
    ));
    let journal = ok(&journal_response);
    let kinds: Vec<&str> = journal["data"]["events"]
        .as_array()
        .unwrap()
        .iter()
        .map(|event| event["payload"]["kind"].as_str().unwrap())
        .collect();
    assert!(kinds.contains(&"task_dependencies_set"));
}

#[test]
fn leases_fence_stale_owners_and_the_projection_is_explainable() {
    let mut host = Host::new();
    ok(&host.call(request("register-lease", project("lease"))));
    ok(&host.call(request("lease-maintenance", maintenance("lease"))));
    ok(&host.call(request("lease-task", task("lease-task", "lease"))));
    // The projection over a fresh Ready task explains it as schedulable.
    let sweep = |command: &str| request(command, json!({"kind":"get_scheduling_projection"}));
    let first_projection = host.call(sweep("project-1"));
    let schedulable = ok(&first_projection)["data"]["schedulable"]
        .as_array()
        .unwrap()
        .clone();
    assert_eq!(schedulable.len(), 1);
    assert_eq!(schedulable[0]["task_id"], "lease-task");
    assert_eq!(schedulable[0]["reason"], "no_blocking_dependencies");
    // Without an active dispatch no lease can be acquired yet; the attempt
    // fails as an invalid request and the host keeps serving.
    let lease_attempt = request(
        "lease-too-early",
        json!({"kind":"acquire_task_lease","task_id":"lease-task","dispatch_id":"no-dispatch",
               "host_id":"lease","duration_ms":60000}),
    );
    assert_eq!(
        host.call(lease_attempt)["result"]["Err"]["code"],
        "invalid_request"
    );
    // The sweep endpoint expires nothing and still reports the projection.
    let sweep_response = host.call(request("sweep-1", json!({"kind":"expire_stale_leases"})));
    let swept = ok(&sweep_response);
    assert_eq!(swept["data"]["expired"].as_array().unwrap().len(), 0);
    assert_eq!(swept["data"]["schedulable"].as_array().unwrap().len(), 1);
    host.crash();
    host.start();
    let second_projection = host.call(sweep("project-2"));
    assert_eq!(
        ok(&second_projection)["data"]["schedulable"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn dispatch_preparation_records_composition_and_refusals() {
    let mut host = Host::new();
    ok(&host.call(request("register-disp", project("disp"))));
    ok(&host.call(request("disp-maintenance", maintenance("disp"))));
    ok(&host.call(request("disp-task", task("disp-task", "disp"))));
    // The projection explains the task as schedulable; the preparation then
    // records the composition steps against durable state.
    let projection_response = host.call(request(
        "disp-projection",
        json!({"kind":"get_scheduling_projection"}),
    ));
    let sweep = ok(&projection_response);
    assert_eq!(sweep["data"]["schedulable"].as_array().unwrap().len(), 1);
    let prepare = request(
        "disp-prepare-1",
        json!({"kind":"prepare_dispatch","task_id":"disp-task"}),
    );
    let first_response = host.call(prepare.clone());
    let first = ok(&first_response)["data"].clone();
    // Without routing and a lease the composition is recorded as refused.
    assert_eq!(first["outcome"], "refused");
    let steps = first["steps"].as_array().unwrap();
    assert!(
        steps
            .iter()
            .any(|s| s["kind"] == "scheduling" && s["schedulable"] == true)
    );
    assert!(
        steps
            .iter()
            .any(|s| s["kind"] == "routing" && s["resolved"] == Value::Null)
    );
    // Replay returns the identical composition.
    let replay_response = host.call(prepare);
    let replay = ok(&replay_response)["data"].clone();
    assert_eq!(replay, first);
    // Restart: the preparation replays from the journal.
    host.crash();
    host.start();
    let read_response = host.call(request(
        "disp-prepare-read",
        json!({"kind":"get_dispatch_preparation","task_id":"disp-task"}),
    ));
    assert_eq!(ok(&read_response)["data"], first);
    // The journal carries the dispatch_prepared event.
    let journal_response = host.call(request(
        "disp-journal",
        json!({"kind":"read_journal","project_id":"disp","after":0,"limit":100}),
    ));
    let journal = ok(&journal_response);
    let kinds: Vec<&str> = journal["data"]["events"]
        .as_array()
        .unwrap()
        .iter()
        .map(|event| event["payload"]["kind"].as_str().unwrap())
        .collect();
    assert!(kinds.contains(&"dispatch_prepared"));
}

#[test]
fn start_prepared_task_transitions_running_and_holds_the_lease() {
    let host = Host::new();
    ok(&host.call(request("register-run", project("run"))));
    ok(&host.call(request("run-maintenance", maintenance("run"))));
    ok(&host.call(request("run-task", task("run-task", "run"))));
    // A start before any preparation exists is NotFound.
    let early = request(
        "run-start-early",
        json!({"kind":"start_prepared_task","task_id":"run-task","host_id":"unknown-host"}),
    );
    assert_eq!(
        host.call(early)["result"]["Err"]["code"],
        "permission_denied"
    );
    // Prepare records the composition (refused: no route/binding/provider).
    let prepare = request(
        "run-prepare",
        json!({"kind":"prepare_dispatch","task_id":"run-task"}),
    );
    let first = ok(&host.call(prepare.clone()))["data"].clone();
    assert_eq!(first["outcome"], "refused");
    // Starting from a refused preparation fails closed.
    let host_id = ok(&host.call(request(
        "run-host-pulse",
        json!({"kind":"get_host_pulse"}),
    )))["data"]["host_id"]
        .clone();
    let start_refused = request(
        "run-start-refused",
        json!({"kind":"start_prepared_task","task_id":"run-task","host_id":host_id}),
    );
    assert_eq!(
        host.call(start_refused)["result"]["Err"]["code"],
        "failed_precondition"
    );
    // The task remains Ready and no lease exists.
    let task_response = host.call(request(
        "run-task-read",
        json!({"kind":"get_task","project_id":"run","task_id":"run-task"}),
    ));
    assert_eq!(ok(&task_response)["data"]["state"], "ready");
}

#[test]
fn worker_completion_request_is_evidence_not_completion() {
    let host = Host::new();
    ok(&host.call(request("register-comp", project("comp"))));
    ok(&host.call(request("comp-maintenance", maintenance("comp"))));
    ok(&host.call(request("comp-task", task("comp-task", "comp"))));
    // A completion request against a task that was never started is an
    // illegal transition, and the Host keeps serving.
    let request_completion = request(
        "comp-request",
        json!({"kind":"request_task_completion","task_id":"comp-task",
               "dispatch_id":"ghost-dispatch","report":"done"}),
    );
    let response = host.call(request_completion);
    assert_eq!(response["result"]["Err"]["code"], "invalid_request");
    ok(&host.call(request("comp-alive", json!({"kind":"health"}))));
    // The task remains Ready: worker evidence never bypasses verification.
    let task_response = host.call(request(
        "comp-task-read",
        json!({"kind":"get_task","project_id":"comp","task_id":"comp-task"}),
    ));
    assert_eq!(ok(&task_response)["data"]["state"], "ready");
}

#[test]
fn run_started_dispatch_refuses_closed_and_never_executes_without_transport() {
    let mut host = Host::new();
    // Full composition for staffing-demo: team, binding, provider, model,
    // classified objective origin, task, route, preparation.
    let register: Value =
        serde_json::from_str(include_str!("../../../fixtures/project-team/register.json")).unwrap();
    let register_response = host.call(register);
    if register_response["result"].get("Err").is_some() {
        panic!("fixture step register failed: {register_response}");
    }
    // The team grants the engineer's role stream mutation (the dispatch
    // compiles MutateStream from the binding, which must stay within the
    // member's access and the ceiling).
    let mut team: Value = serde_json::from_str(include_str!(
        "../../../fixtures/project-team/configure.json"
    ))
    .unwrap();
    team["command_id"] = json!("activation-team");
    for pointer in ["/access_ceiling/grants", "/members/1/access/grants"] {
        let mut grants: Vec<Value> = team["operation"]["team"]
            .pointer_mut(pointer)
            .unwrap()
            .as_array()
            .unwrap()
            .clone();
        grants.push(json!("mutate_stream"));
        *team["operation"]["team"].pointer_mut(pointer).unwrap() = json!(grants);
    }
    let team_response = host.call(team);
    if team_response["result"].get("Err").is_some() {
        panic!("fixture step team failed: {team_response}");
    }
    // The binding must name the live Host as eligible: resolve the real
    // inventory identity first, then configure the workforce binding with
    // it (the checked-in fixture pins a placeholder host).
    let pulse_response = host.call(request(
        "activation-pulse-early",
        json!({"kind":"get_host_pulse"}),
    ));
    let host_id = pulse_response["result"]["Ok"]["data"]["host_id"].clone();
    let mut binding: Value = serde_json::from_str(include_str!(
        "../../../fixtures/workforce-bindings/configure.json"
    ))
    .unwrap();
    binding["command_id"] = json!("activation-binding");
    binding["operation"]["configuration"]["primary"]["profile"]["eligible_hosts"] =
        json!([host_id]);
    for path in [
        ["operation", "configuration", "primary", "access", "grants"],
        ["operation", "configuration", "binding", "access", "grants"],
    ] {
        let mut grants: Vec<Value> = binding
            .pointer_mut(&format!("/{}", path.join("/")))
            .unwrap()
            .as_array()
            .unwrap()
            .clone();
        grants.push(json!("mutate_stream"));
        *binding
            .pointer_mut(&format!("/{}", path.join("/")))
            .unwrap() = json!(grants);
    }
    let binding_response = host.call(binding);
    if binding_response["result"].get("Err").is_some() {
        panic!("fixture step binding failed: {binding_response}");
    }
    // Provider connection and model descriptor the binding's profile needs.
    ok_step(
        "activation-provider",
        &host.call(request(
            "activation-provider",
            json!({"kind":"replace_provider_connection","attribution":"staffing-demo",
            "connection":{"id":"native-openai","adapter":"openai-responses",
            "endpoint_reference":"https://api.openai.example/v1","authentication":"api_credential"}}),
        ),
    ));
    ok_step(
        "activation-model",
        &host.call(request(
            "activation-model",
            json!({"kind":"replace_model_descriptor","attribution":"staffing-demo",
            "descriptor":{"schema_version":1,"id":"coding-model","provider_id":"native-openai",
            "context_window_tokens":8192,"max_output_tokens":4096,
            "capabilities":{"reasoning_efforts":[],"tools":true,"images":false,"streaming":false}}}),
        ),
    ));
    // Classified origin the engineer's task can hang off.
    ok_step(
        "activation-objective",
        &host.call(request(
            "activation-objective",
            json!({"kind":"create_work","work":{"id":{"kind":"objective","id":"staffing-objective"},
            "project_id":"staffing-demo","role_id":"engineer","title":"Implement a bounded change",
            "description":"Implement the bounded change described by this objective.",
            "utterance":null,"objective_class":"maintenance","parent":null,"dependencies":[],
            "requirements":[],"constraints":[],"risks":[],"acceptance":["done"],"priority":2,
            "budget":null,"external_references":[]}}),
        )),
    );
    ok_step(
        "activation-task",
        &host.call(request(
            "activation-task",
            json!({"kind":"create_task","task":{"id":"staffing-task","project_id":"staffing-demo",
            "root_id":"staffing-root","role_id":"engineer",
            "origin":{"kind":"objective","work":{"project_id":"staffing-demo",
                "id":{"kind":"objective","id":"staffing-objective"}}},
            "task_contract":{"id":"coding-contract","revision":1},
            "stream":{"id":"staffing-stream","originating_chat":"chat","worktree":"staffing-worktree",
            "branch":"task/staffing","base":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "target":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"}}}),
        ),
    ));
    // Route the task to the engineer explicitly.
    ok_step(
        "activation-route",
        &host.call(request(
            "activation-route",
            json!({"kind":"record_route",
            "request":{"project_id":"staffing-demo","work_id":{"kind":"objective","id":"staffing-objective"},
            "requested":"engineer","domains":[]}}),
        ),
    ));
    let prepare_response = host.call(request(
        "activation-prepare",
        json!({"kind":"prepare_dispatch","task_id":"staffing-task"}),
    ));
    if prepare_response["result"].get("Err").is_some() {
        panic!("activation-prepare failed: {prepare_response}");
    }
    let preparation = prepare_response["result"]["Ok"]["data"].clone();
    assert_eq!(preparation["outcome"], "ready");
    // Start requires the Host's own inventory identity; a start against an
    // unknown Host is permission-denied, so resolve the real one.
    let start_response = host.call(request(
        "activation-start",
        json!({"kind":"start_prepared_task","task_id":"staffing-task","host_id":host_id}),
    ));
    if start_response["result"].get("Err").is_some() {
        panic!("activation-start failed: {start_response}");
    }
    let started = start_response["result"]["Ok"]["data"].clone();
    assert_eq!(started["task_id"], "staffing-task");
    // Activation with the production transport configuration (none
    // configured) refuses with a typed precondition and files nothing:
    // live execution requires explicit authorization for credentials and
    // billing. The task stays Running.
    let refused = host.call(request(
        "activation-run",
        json!({"kind":"run_started_dispatch","task_id":"staffing-task",
            "dispatch_id":started["dispatch_id"]}),
    ));
    assert_eq!(refused["result"]["Err"]["code"], "failed_precondition");
    // Provisioning runs BEFORE the transport build: the production daemon
    // configures no reservation base, so the provisioning stage refuses
    // first (the no-transport refusal is pinned in the runner tests).
    assert_eq!(
        refused["result"]["Err"]["message"],
        "no worktree reservation base configured"
    );
    let task_read = request(
        "activation-task-read",
        json!({"kind":"get_task","project_id":"staffing-demo","task_id":"staffing-task"}),
    );
    let task_response = host.call(task_read);
    let task = ok(&task_response);
    assert_eq!(task["data"]["state"], "running");
    // Activation against a foreign dispatch id is refused before anything
    // (message pins the ordering: the binding check precedes any transport
    // build, so this can never read as a no-transport refusal).
    let foreign = host.call(request(
        "activation-foreign",
        json!({"kind":"run_started_dispatch","task_id":"staffing-task","dispatch_id":"disp_ghost"}),
    ));
    assert_eq!(foreign["result"]["Err"]["code"], "failed_precondition");
    assert_eq!(
        foreign["result"]["Err"]["message"],
        "task is not running under the requested dispatch"
    );
    host.crash();
    host.start();
    // The refusal left no residue: state replays identically.
    let task_read = request(
        "activation-task-read-2",
        json!({"kind":"get_task","project_id":"staffing-demo","task_id":"staffing-task"}),
    );
    let task_response = host.call(task_read);
    let task = ok(&task_response);
    assert_eq!(task["data"]["state"], "running");
}

#[test]
fn cli_administration_flow_uses_typed_commands_end_to_end() {
    let host = Host::new();
    let run = |arguments: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_symbiote"))
            .arg("--state-dir")
            .arg(&host.directory)
            .args(arguments)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .unwrap()
    };
    // Register via raw (full typed draft), then read back through the
    // typed command.
    let project_json = host.directory.join("project.json");
    let operation = serde_json::to_value(project("cli")).unwrap();
    std::fs::write(
        &project_json,
        serde_json::to_vec_pretty(&operation).unwrap(),
    )
    .unwrap();
    let registered = run(&["raw", project_json.to_str().unwrap()]);
    assert!(
        registered.status.success(),
        "raw register: {}",
        String::from_utf8_lossy(&registered.stderr)
    );
    let read = run(&["get-project", "cli"]);
    assert!(
        read.status.success(),
        "get-project: {}",
        String::from_utf8_lossy(&read.stderr)
    );
    let body: Value = serde_json::from_slice(&read.stdout).unwrap();
    assert_eq!(body["kind"], "project");
    assert_eq!(body["data"]["id"], "cli");

    // Journal read through the typed command shows the registration event.
    let journal = run(&["read-journal", "cli", "0", "100"]);
    assert!(
        journal.status.success(),
        "read-journal: {}",
        String::from_utf8_lossy(&journal.stderr)
    );
    let body: Value = serde_json::from_slice(&journal.stdout).unwrap();
    assert_eq!(body["kind"], "journal");
    assert_eq!(body["data"]["events"].as_array().unwrap().len(), 1);

    // Team is absent for this project until configured: the daemon refusal
    // is a typed error with exit 2 (still an exercised typed command).
    let team = run(&["get-team", "cli"]);
    assert_eq!(team.status.code(), Some(2));
    let error: Value = serde_json::from_slice(&team.stderr).unwrap();
    assert_eq!(error["code"], "not_found");

    // Scheduling projection explains an empty queue.
    let projection = run(&["scheduling-projection"]);
    assert!(projection.status.success());
    let body: Value = serde_json::from_slice(&projection.stdout).unwrap();
    assert_eq!(body["kind"], "scheduler_sweep");

    // Dispatch preparation against a real task: create one via raw, then
    // exercise prepare/get-preparation/get-task/get-task-origin through the
    // typed commands.
    let objective = host.directory.join("objective.json");
    std::fs::write(
        &objective,
        serde_json::to_vec_pretty(&serde_json::json!({
            "kind":"create_work","work":{"id":{"kind":"objective","id":"cli-objective"},
            "project_id":"cli","role_id":"lead-cli","title":"CLI objective",
            "description":"Owns the CLI task","utterance":null,"objective_class":"maintenance",
            "parent":null,"dependencies":[],"requirements":[],"constraints":[],"risks":[],
            "acceptance":["done"],"priority":2,"budget":null,"external_references":[]}}))
        .unwrap(),
    )
    .unwrap();
    let created = run(&["raw", objective.to_str().unwrap()]);
    assert!(
        created.status.success(),
        "create_work: {}",
        String::from_utf8_lossy(&created.stderr)
    );
    let task_json = host.directory.join("task.json");
    std::fs::write(
        &task_json,
        serde_json::to_vec_pretty(&serde_json::json!({
            "id":"cli-task","project_id":"cli",
            "root_id":"root-cli","role_id":"lead-cli",
            "origin":{"kind":"objective","work":{"project_id":"cli",
                "id":{"kind":"objective","id":"cli-objective"}}},
            "task_contract":{"id":"coding-contract","revision":1},
            "stream":{"id":"cli-stream","originating_chat":"chat","worktree":"cli-worktree",
            "branch":"task/cli","base":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "target":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"}}))
        .unwrap(),
    )
    .unwrap();
    let created = run(&["create-task", task_json.to_str().unwrap()]);
    assert!(
        created.status.success(),
        "create-task: {}",
        String::from_utf8_lossy(&created.stderr)
    );
    let read = run(&["get-task", "cli", "cli-task"]);
    assert!(
        read.status.success(),
        "get-task: {}",
        String::from_utf8_lossy(&read.stderr)
    );
    let body: Value = serde_json::from_slice(&read.stdout).unwrap();
    assert_eq!(body["kind"], "task");
    assert_eq!(body["data"]["state"], "ready");
    let origin = run(&["get-task-origin", "cli", "cli-task"]);
    assert!(
        origin.status.success(),
        "get-task-origin: {}",
        String::from_utf8_lossy(&origin.stderr)
    );
    let body: Value = serde_json::from_slice(&origin.stdout).unwrap();
    assert_eq!(body["kind"], "task_origin");
    let preparation = run(&["prepare-dispatch", "cli-task"]);
    assert!(
        preparation.status.success(),
        "prepare-dispatch: {}",
        String::from_utf8_lossy(&preparation.stderr)
    );
    let body: Value = serde_json::from_slice(&preparation.stdout).unwrap();
    assert_eq!(body["kind"], "dispatch_preparation");
    // Without routing/binding the composition is recorded as refused — the
    // recorded outcome is machine-readable in the response.
    assert_eq!(body["data"]["outcome"], "refused");
    let read_back = run(&["get-dispatch-preparation", "cli-task"]);
    assert!(
        read_back.status.success(),
        "get-dispatch-preparation: {}",
        String::from_utf8_lossy(&read_back.stderr)
    );
    let body: Value = serde_json::from_slice(&read_back.stdout).unwrap();
    assert_eq!(body["data"]["outcome"], "refused");

    // run-started-dispatch and request-task-completion and
    // start-prepared-task against this not-started task: typed refusals
    // with exit 2 (the commands are on the wire and authorized; the state
    // machine refuses the transition).
    // run-started-dispatch and start-prepared-task refuse with the
    // precondition code; request-task-completion against an unstarted task
    // maps the domain's illegal transition to invalid_request. All are exit
    // 2 with typed errors — the commands are on the wire and authorized,
    // the state machine refuses the transition. The dangerous operations
    // carry their explicit `--yes` so what is observed here is the daemon's
    // refusal, not the CLI's own authorization gate (exit 3).
    for (arguments, code) in [
        (
            vec!["run-started-dispatch", "cli-task", "disp_ghost", "--yes"],
            "failed_precondition",
        ),
        (
            vec![
                "request-task-completion",
                "cli-task",
                "disp_ghost",
                "report",
            ],
            "invalid_request",
        ),
        (
            vec![
                "request-elevation",
                "missing-task",
                "disp_ghost",
                "use_credential",
                "need-secret",
            ],
            "not_found",
        ),
        (
            vec!["start-prepared-task", "cli-task", "ghost-host", "--yes"],
            "permission_denied",
        ),
    ] {
        let result = run(&arguments);
        assert_eq!(result.status.code(), Some(2), "{arguments:?}");
        let error: Value = serde_json::from_slice(&result.stderr).unwrap();
        assert_eq!(error["code"], code, "{arguments:?}");
    }

    // Transport failure (dead state dir) is exit 1, not 2.
    let dead = Command::new(env!("CARGO_BIN_EXE_symbiote"))
        .arg("--state-dir")
        .arg("/nonexistent-symbiote-state")
        .arg("health")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .unwrap();
    assert_eq!(dead.status.code(), Some(1));

    // Help exits 0 with the command table. It is answered locally and honors
    // no flags, so it is invoked without the `--state-dir` the `run` closure
    // prepends: a flag a command cannot honor is a usage error, never a silent
    // no-op (see `flags_a_command_cannot_honor_are_usage_errors` in
    // `cli_schema_contract`).
    let help = Command::new(env!("CARGO_BIN_EXE_symbiote"))
        .arg("help")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .unwrap();
    assert_eq!(help.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&help.stdout).contains("run-started-dispatch"));
}
