use super::*;
use std::{
    collections::BTreeMap,
    fs,
    process::Command,
    sync::{Arc, Barrier},
    thread,
    time::{Instant, SystemTime, UNIX_EPOCH},
};

macro_rules! id {
    ($kind:ident,$value:expr) => {
        $kind::new($value).unwrap()
    };
}
fn sha(c: char) -> CommitSha {
    CommitSha::new(c.to_string().repeat(40)).unwrap()
}

struct Temporary {
    directory: std::path::PathBuf,
}
impl Temporary {
    fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory =
            std::env::temp_dir().join(format!("symbiote-store-{}-{nonce}", std::process::id()));
        fs::create_dir(&directory).unwrap();
        Self { directory }
    }
    fn database(&self) -> std::path::PathBuf {
        self.directory.join("control.sqlite3")
    }
}
impl Drop for Temporary {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.directory);
    }
}

fn records(suffix: &str) -> (Project, Vec<Root>, Vec<Role>) {
    let project_id = id!(ProjectId, format!("project-{suffix}"));
    let root_id = id!(RootId, format!("root-{suffix}"));
    let role_id = id!(RoleId, format!("role-{suffix}"));
    let project = Project {
        id: project_id.clone(),
        revision: Revision(0),
        name: format!("Project {suffix}"),
        owner: id!(UserId, "owner"),
        roots: BTreeSet::from([root_id.clone()]),
        lead: role_id.clone(),
        disposition: RecordDisposition::Active,
        provenance: Provenance {
            created_at: Timestamp(10),
            updated_at: Timestamp(10),
            actor: Actor::User(id!(UserId, "owner")),
            external_references: vec![],
        },
    };
    let root = Root {
        id: root_id,
        project_id: project_id.clone(),
        revision: Revision(0),
        repository: None,
        host_paths: BTreeMap::new(),
    };
    let role = Role {
        id: role_id,
        project_id,
        revision: Revision(0),
        name: "Engineer".into(),
        operating_contract: VersionedRoleContract {
            id: id!(RoleContractId, "role-contract"),
            revision: Revision(1),
        },
    };
    (project, vec![root], vec![role])
}
fn register(store: &mut Store, suffix: &str) -> (Project, Vec<Root>, Vec<Role>) {
    let (p, r, s) = records(suffix);
    store
        .register_project(
            id!(CommandId, format!("register-{suffix}")),
            p.clone(),
            r.clone(),
            s.clone(),
        )
        .unwrap();
    (p, r, s)
}
fn task_records(suffix: &str) -> (Task, ChangeStream) {
    let project_id = id!(ProjectId, "project-one");
    let root_id = id!(RootId, "root-one");
    let task_id = id!(TaskId, format!("task-{suffix}"));
    let stream_id = id!(ChangeStreamId, format!("stream-{suffix}"));
    let task = Task::new(
        task_id.clone(),
        project_id.clone(),
        root_id.clone(),
        id!(RoleId, "role-one"),
        stream_id.clone(),
        VersionedTaskContract {
            id: id!(TaskContractId, "task-contract"),
            revision: Revision(1),
        },
    );
    let stream = ChangeStream::new(NewChangeStream {
        id: stream_id,
        project_id,
        root_id,
        tasks: BTreeSet::from([task_id]),
        originating_chat: id!(ChatId, format!("chat-{suffix}")),
        worktree: id!(WorktreeId, format!("worktree-{suffix}")),
        branch: format!("branch-{suffix}"),
        lineage: StreamLineage::Independent,
        base: sha('a'),
        target: sha('b'),
    })
    .unwrap();
    (task, stream)
}
fn start_command(task: &Task, role: &Role) -> TaskCommand {
    let host_id = id!(HostId, "host");
    let profile = RuntimeProfile {
        id: id!(RuntimeProfileId, "profile"),
        revision: Revision(0),
        runtime: RuntimeKind::NativeSymbiote,
        adapter: id!(AgentRuntimeAdapterId, "adapter"),
        installation: None,
        provider: id!(ProviderConnectionId, "provider"),
        credential: id!(CredentialReferenceId, "credential"),
        billing_entitlement: id!(BillingEntitlementId, "entitlement"),
        model: id!(ModelId, "model"),
        eligible_hosts: BTreeSet::from([host_id.clone()]),
    };
    let binding = WorkforceBinding {
        id: id!(BindingId, "binding"),
        revision: Revision(0),
        project_id: task.project_id().clone(),
        role_id: role.id.clone(),
        profile_id: profile.id.clone(),
        profile_revision: profile.revision,
        protocol: VersionedProtocol {
            id: id!(ProtocolId, "protocol"),
            revision: Revision(1),
        },
        access: AccessSnapshot {
            project_id: task.project_id().clone(),
            roots: BTreeSet::from([task.root_id().clone()]),
            grants: BTreeSet::from([Permission::MutateStream]),
            policy_revision: Revision(1),
        },
        required_controls: BTreeSet::new(),
        context: ContextPolicy {
            bundle: id!(ContextBundleId, "context"),
            revision: Revision(1),
            max_input_tokens: 100,
            reserved_output_tokens: 50,
        },
        required_tools: BTreeSet::new(),
        required_skills: BTreeSet::new(),
        escalation: EscalationPolicy::StopAndRequestHuman,
    };
    let host = Host {
        id: host_id.clone(),
        revision: Revision(0),
        device: id!(DeviceId, "device"),
        fabric: None,
        supported_runtimes: vec![RuntimeKind::NativeSymbiote],
        controls: [
            Control::Filesystem,
            Control::Cancellation,
            Control::CompletionAuthority,
        ]
        .into_iter()
        .map(|c| {
            (
                c,
                EnforcementClaim {
                    strength: EnforcementStrength::HostEnforced,
                    evidence: id!(EvidenceId, "proof"),
                    verified_at: Timestamp(1),
                    expires_at: Timestamp(1000),
                },
            )
        })
        .collect(),
    };
    let dispatch = Dispatch::compile(
        id!(DispatchId, "dispatch"),
        id!(RuntimeContractId, "runtime-contract"),
        DispatchInputs {
            task,
            role,
            binding: &binding,
            profile: &profile,
            host: &host,
            now: Timestamp(10),
        },
    )
    .unwrap();
    TaskCommand {
        id: id!(CommandId, "start"),
        expected_revision: Revision(0),
        actor: Actor::Host(host_id),
        at: Timestamp(20),
        action: TaskAction::Start {
            dispatch: Box::new(dispatch),
        },
    }
}

#[test]
fn project_relationships_and_idempotency_are_atomic() {
    let mut store = Store::memory().unwrap();
    let (project, roots, roles) = records("one");
    let mut wrong = roles.clone();
    wrong[0].project_id = id!(ProjectId, "wrong");
    assert!(matches!(
        store.register_project(
            id!(CommandId, "invalid"),
            project.clone(),
            roots.clone(),
            wrong
        ),
        Err(StoreError::RelationshipMismatch)
    ));
    assert!(matches!(
        store.project(&project.id),
        Err(StoreError::NotFound)
    ));
    let first = store
        .register_project(
            id!(CommandId, "register"),
            project.clone(),
            roots.clone(),
            roles.clone(),
        )
        .unwrap();
    let second = store
        .register_project(
            id!(CommandId, "register"),
            project.clone(),
            roots.clone(),
            roles.clone(),
        )
        .unwrap();
    assert_eq!(first.sequence, second.sequence);
    assert!(second.replayed);
    let mut changed = project.clone();
    changed.name = "Changed".into();
    assert!(matches!(
        store.register_project(id!(CommandId, "register"), changed, roots, roles),
        Err(StoreError::IdempotencyConflict)
    ));
    assert_eq!(store.project(&project.id).unwrap(), project);
    assert_eq!(
        store
            .registration_timestamp(&id!(CommandId, "register"))
            .unwrap(),
        Some(Timestamp(10))
    );
    assert_eq!(store.events(&project.id, 0, 10).unwrap().events.len(), 1);
}

#[test]
fn task_creation_rejects_cross_project_foreign_keys_and_shared_workspace() {
    let mut store = Store::memory().unwrap();
    register(&mut store, "one");
    let (task, stream) = task_records("one");
    let mut wrong_stream = stream.clone();
    wrong_stream.worktree = id!(WorktreeId, "other");
    let wrong_task = Task::new(
        task.id().clone(),
        id!(ProjectId, "other"),
        task.root_id().clone(),
        task.role_id().clone(),
        task.stream_id().clone(),
        task.task_contract().clone(),
    );
    assert!(matches!(
        store.create_task(id!(CommandId, "bad"), wrong_task, wrong_stream),
        Err(StoreError::RelationshipMismatch)
    ));
    store
        .create_task(id!(CommandId, "create"), task.clone(), stream.clone())
        .unwrap();
    let (second, mut second_stream) = task_records("two");
    second_stream.worktree = stream.worktree;
    assert!(
        store
            .create_task(id!(CommandId, "shared"), second.clone(), second_stream)
            .is_err()
    );
    assert!(matches!(store.task(second.id()), Err(StoreError::NotFound)));
    assert_eq!(
        store.events(task.project_id(), 0, 10).unwrap().events.len(),
        2
    );
    assert!(matches!(
        store.registration_timestamp(&id!(CommandId, "create")),
        Err(StoreError::IdempotencyConflict)
    ));
}

#[test]
fn reconnect_persists_state_and_bounded_project_scoped_replay() {
    let temporary = Temporary::new();
    let project;
    {
        let mut store = Store::open(temporary.database()).unwrap();
        project = register(&mut store, "one").0;
        register(&mut store, "two");
        let (task, stream) = task_records("one");
        store
            .create_task(id!(CommandId, "create"), task, stream)
            .unwrap();
    }
    let store = Store::open(temporary.database()).unwrap();
    assert_eq!(store.project(&project.id).unwrap(), project);
    let first = store.events(&project.id, 0, 1).unwrap();
    assert!(first.has_more);
    assert_eq!(first.events.len(), 1);
    let second = store.events(&project.id, first.next_cursor, 1).unwrap();
    assert!(!second.has_more);
    assert_eq!(second.events.len(), 1);
    assert!(
        second.events[0].sequence > first.events[0].sequence + 1,
        "other Project's event must not leak"
    );
    assert!(store.events(&project.id, 0, MAX_EVENT_PAGE + 1).is_err());
    assert!(matches!(
        store.events(&project.id, 999, 1),
        Err(StoreError::InvalidPage)
    ));
    store.integrity_check().unwrap();
}

#[test]
fn lifecycle_updates_journal_once_and_denied_worker_does_not_mutate() {
    let mut store = Store::memory().unwrap();
    let (_, _, roles) = register(&mut store, "one");
    let (task, stream) = task_records("one");
    store
        .create_task(id!(CommandId, "create"), task.clone(), stream)
        .unwrap();
    let command = start_command(&task, &roles[0]);
    let first = store.apply_task(task.id(), command.clone()).unwrap();
    let retry = store.apply_task(task.id(), command).unwrap();
    assert_eq!(first.sequence, retry.sequence);
    assert!(retry.replayed);
    let denied = TaskCommand {
        id: id!(CommandId, "denied"),
        expected_revision: Revision(1),
        actor: Actor::Worker(id!(DispatchId, "dispatch")),
        at: Timestamp(21),
        action: TaskAction::BeginVerification,
    };
    assert!(matches!(
        store.apply_task(task.id(), denied),
        Err(StoreError::Domain(DomainError::PermissionDenied))
    ));
    assert_eq!(store.task(task.id()).unwrap().state(), &TaskState::Running);
    assert_eq!(
        store.events(task.project_id(), 0, 10).unwrap().events.len(),
        3
    );
}

#[test]
fn two_connections_racing_terminal_transitions_have_one_winner() {
    let temporary = Temporary::new();
    let mut store = Store::open(temporary.database()).unwrap();
    let (_, _, roles) = register(&mut store, "one");
    let (task, stream) = task_records("one");
    store
        .create_task(id!(CommandId, "create"), task.clone(), stream)
        .unwrap();
    store
        .apply_task(task.id(), start_command(&task, &roles[0]))
        .unwrap();
    let barrier = Arc::new(Barrier::new(2));
    let handles: Vec<_> = [
        TaskAction::Cancel {
            reason: "cancel".into(),
        },
        TaskAction::Fail {
            reason: "failure".into(),
        },
    ]
    .into_iter()
    .enumerate()
    .map(|(i, action)| {
        let database = temporary.database();
        let barrier = barrier.clone();
        let task_id = task.id().clone();
        thread::spawn(move || {
            let mut store = Store::open(database).unwrap();
            barrier.wait();
            store.apply_task(
                &task_id,
                TaskCommand {
                    id: id!(CommandId, format!("race-{i}")),
                    expected_revision: Revision(1),
                    actor: Actor::Host(id!(HostId, "host")),
                    at: Timestamp(21),
                    action,
                },
            )
        })
    })
    .collect();
    let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter(|r| matches!(r, Err(StoreError::Domain(DomainError::RevisionConflict))))
            .count(),
        1
    );
    assert_eq!(store.task(task.id()).unwrap().revision(), Revision(2));
    assert_eq!(
        store.events(task.project_id(), 0, 10).unwrap().events.len(),
        4
    );
}

#[test]
fn journal_refuses_update_delete_and_failed_late_insert_rolls_back_current_state() {
    let mut store = Store::memory().unwrap();
    register(&mut store, "one");
    assert!(store.connection.execute("DELETE FROM journal", []).is_err());
    assert!(
        store
            .connection
            .execute("UPDATE journal SET revision=99", [])
            .is_err()
    );
    store.connection.execute_batch("CREATE TRIGGER fail_new_event BEFORE INSERT ON journal BEGIN SELECT RAISE(ABORT,'injected journal failure'); END;").unwrap();
    let (task, stream) = task_records("one");
    assert!(
        store
            .create_task(id!(CommandId, "create"), task.clone(), stream)
            .is_err()
    );
    assert!(matches!(store.task(task.id()), Err(StoreError::NotFound)));
    assert_eq!(
        store
            .connection
            .query_row("SELECT count(*) FROM streams", [], |r| sql_u64(r, 0))
            .unwrap(),
        0
    );
}

#[test]
fn corruption_and_future_or_unknown_schema_are_refused_without_reset() {
    let temporary = Temporary::new();
    fs::write(temporary.database(), b"not a database").unwrap();
    assert!(Store::open(temporary.database()).is_err());
    assert_eq!(fs::read(temporary.database()).unwrap(), b"not a database");
    fs::remove_file(temporary.database()).unwrap();
    let store = Store::open(temporary.database()).unwrap();
    store
        .connection
        .pragma_update(None, "user_version", 2)
        .unwrap();
    drop(store);
    assert!(matches!(
        Store::open(temporary.database()),
        Err(StoreError::UnsupportedVersion)
    ));
    let connection = Connection::open(temporary.database()).unwrap();
    assert_eq!(
        connection
            .pragma_query_value(None, "user_version", |r| sql_u64(r, 0))
            .unwrap(),
        2
    );
}

#[test]
fn invalid_domain_snapshot_is_detected_on_reopen() {
    let temporary = Temporary::new();
    let mut store = Store::open(temporary.database()).unwrap();
    register(&mut store, "one");
    let (task, stream) = task_records("one");
    store
        .create_task(id!(CommandId, "create"), task.clone(), stream)
        .unwrap();
    store
        .connection
        .execute(
            "UPDATE tasks SET body=json_set(body,'$.state','completed')",
            [],
        )
        .unwrap();
    drop(store);
    assert!(matches!(
        Store::open(temporary.database()),
        Err(StoreError::Json(_))
    ));
}

#[test]
fn sequence_gaps_and_zero_are_refused_without_pruning_or_repair() {
    for sequence in [0, 100] {
        let temporary = Temporary::new();
        let mut store = Store::open(temporary.database()).unwrap();
        register(&mut store, "one");
        store
            .connection
            .execute_batch("DROP TRIGGER journal_no_update; PRAGMA ignore_check_constraints=ON;")
            .unwrap();
        store
            .connection
            .execute("UPDATE journal SET sequence=?1", [sequence])
            .unwrap();
        store.connection.execute_batch("CREATE TRIGGER journal_no_update BEFORE UPDATE ON journal BEGIN SELECT RAISE(ABORT,'append-only journal'); END;").unwrap();
        drop(store);
        assert!(matches!(
            Store::open(temporary.database()),
            Err(StoreError::Integrity(_))
        ));
    }
}

#[cfg(unix)]
#[test]
fn existing_database_symlinks_are_refused() {
    let temporary = Temporary::new();
    let target = temporary.directory.join("target");
    fs::write(&target, b"untouched").unwrap();
    std::os::unix::fs::symlink(&target, temporary.database()).unwrap();
    assert!(Store::open(temporary.database()).is_err());
    assert_eq!(fs::read(target).unwrap(), b"untouched");
}

#[test]
fn signed_sqlite_values_are_checked_before_unsigned_conversion() {
    let connection = Connection::open_in_memory().unwrap();
    assert!(
        connection
            .query_row("SELECT -1", [], |row| sql_u64(row, 0))
            .is_err()
    );
    assert!(
        connection
            .query_row("SELECT -1", [], |row| sql_usize(row, 0))
            .is_err()
    );
    assert_eq!(
        connection
            .query_row("SELECT 9223372036854775807", [], |row| sql_u64(row, 0))
            .unwrap(),
        i64::MAX as u64
    );
}

pub(super) fn fault_boundary(stage: &str) {
    if std::env::var("SYMBIOTE_STORE_CRASH_STAGE").ok().as_deref() == Some(stage) {
        fs::write(std::env::var("SYMBIOTE_STORE_CRASH_READY").unwrap(), stage).unwrap();
        loop {
            thread::sleep(Duration::from_millis(50));
        }
    }
}

#[test]
fn crash_child() {
    // Normal suite invocation is intentionally a no-op; child mode is selected
    // solely by the parent crash test's environment, never by production code.
    let Ok(database) = std::env::var("SYMBIOTE_STORE_CRASH_DATABASE") else {
        return;
    };
    let mut store = Store::open(database).unwrap();
    register(&mut store, "crash");
}

#[test]
fn process_kill_at_migration_state_journal_and_commit_boundaries_is_atomic() {
    for stage in [
        "migration_before_commit",
        "state_before_journal",
        "journal_before_commit",
        "after_commit",
    ] {
        let temporary = Temporary::new();
        if stage != "migration_before_commit" {
            drop(Store::open(temporary.database()).unwrap());
        }
        let ready = temporary.directory.join("ready");
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "tests::crash_child", "--nocapture"])
            .env("SYMBIOTE_STORE_CRASH_DATABASE", temporary.database())
            .env("SYMBIOTE_STORE_CRASH_STAGE", stage)
            .env("SYMBIOTE_STORE_CRASH_READY", &ready)
            .stdout(std::process::Stdio::null())
            .spawn()
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        while !ready.exists() {
            if Instant::now() >= deadline {
                let _ = child.kill();
                let _ = child.wait();
                panic!("fault boundary {stage} not reached");
            }
            thread::sleep(Duration::from_millis(10));
        }
        child.kill().unwrap();
        child.wait().unwrap();
        let mut store = Store::open(temporary.database()).unwrap();
        store.integrity_check().unwrap();
        let (p, r, s) = records("crash");
        if stage == "after_commit" {
            assert_eq!(store.project(&p.id).unwrap(), p);
            assert!(
                store
                    .register_project(id!(CommandId, "register-crash"), p, r, s)
                    .unwrap()
                    .replayed
            );
        } else {
            assert!(matches!(store.project(&p.id), Err(StoreError::NotFound)));
            assert_eq!(
                store
                    .connection
                    .query_row("SELECT count(*) FROM journal", [], |r| sql_u64(r, 0))
                    .unwrap(),
                0
            );
        }
    }
}
