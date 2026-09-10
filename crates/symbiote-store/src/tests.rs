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
#[path = "binding_tests.rs"]
mod binding_tests;
#[path = "dependency_tests.rs"]
mod dependency_tests;
#[path = "lease_tests.rs"]
mod lease_tests;
#[path = "preparation_tests.rs"]
mod preparation_tests;
#[path = "provider_tests.rs"]
mod provider_tests;
#[path = "route_tests.rs"]
mod route_tests;
fn sha(c: char) -> CommitSha {
    CommitSha::new(c.to_string().repeat(40)).unwrap()
}

pub(super) fn team_fixture(store: &mut Store) -> TeamConfiguration {
    let (project, roots, mut roles) = records("team");
    let mut worker = roles[0].clone();
    worker.id = id!(RoleId, "team-worker");
    worker.name = "General execution".into();
    roles.push(worker);
    store
        .register_project(
            id!(CommandId, "register-team"),
            project.clone(),
            roots,
            roles.clone(),
        )
        .unwrap();
    let access = AccessSnapshot {
        project_id: project.id.clone(),
        roots: project.roots.clone(),
        grants: BTreeSet::from([Permission::ReadRoot, Permission::ExecuteProcess]),
        policy_revision: Revision(1),
    };
    let members = roles
        .iter()
        .enumerate()
        .map(|(index, role)| RolePolicy {
            role_id: role.id.clone(),
            function: if index == 0 {
                RoleFunction::LeadOrchestrator
            } else {
                RoleFunction::GeneralExecution
            },
            responsibilities: vec!["Bounded fixture responsibility".into()],
            task_domains: BTreeSet::from(["implementation".into()]),
            access: access.clone(),
            context_policy_ref: "context-v1".into(),
            tool_policy_ref: "tools-v1".into(),
            skill_policy_ref: "skills-v1".into(),
            execution_policy_ref: "execution-v1".into(),
            independent_reviewers: BTreeSet::from([roles[1 - index].id.clone()]),
            fallbacks: vec![],
        })
        .collect();
    TeamConfiguration {
        schema_version: 1,
        project_id: project.id,
        revision: Revision(0),
        lead_role_id: project.lead,
        members,
        access_ceiling: access,
    }
}

#[test]
fn team_replacements_reopen_retry_and_preserve_role_records() {
    let temp = Temporary::new();
    let mut store = Store::open(temp.database()).unwrap();
    let team = team_fixture(&mut store);
    let before_roles: Vec<String> = store
        .connection
        .prepare("SELECT body FROM roles ORDER BY id")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .map(|r| r.unwrap())
        .collect();
    let first = store
        .replace_team(
            id!(CommandId, "team-initial"),
            None,
            team.clone(),
            id!(UserId, "owner"),
            Timestamp(10),
        )
        .unwrap();
    assert!(
        store
            .replace_team(
                id!(CommandId, "team-initial"),
                None,
                team.clone(),
                id!(UserId, "owner"),
                Timestamp(10)
            )
            .unwrap()
            .replayed
    );
    assert_eq!(
        store
            .team_command_timestamp(&id!(CommandId, "team-initial"))
            .unwrap(),
        Some(Timestamp(10))
    );
    let mut updated = team.clone();
    updated.revision = Revision(1);
    updated.members[0].responsibilities = vec!["Revised responsibility".into()];
    let second = store
        .replace_team(
            id!(CommandId, "team-update"),
            Some(Revision(0)),
            updated.clone(),
            id!(UserId, "another-owner"),
            Timestamp(11),
        )
        .unwrap();
    assert!(second.sequence > first.sequence);
    assert!(
        store
            .replace_team(
                id!(CommandId, "team-initial"),
                None,
                team.clone(),
                id!(UserId, "owner"),
                Timestamp(10)
            )
            .unwrap()
            .replayed
    );
    assert!(matches!(
        store.replace_team(
            id!(CommandId, "team-initial"),
            None,
            team,
            id!(UserId, "different"),
            Timestamp(10)
        ),
        Err(StoreError::IdempotencyConflict)
    ));
    drop(store);
    let store = Store::open(temp.database()).unwrap();
    assert_eq!(store.get_team(&updated.project_id).unwrap(), updated);
    let after_roles: Vec<String> = store
        .connection
        .prepare("SELECT body FROM roles ORDER BY id")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .map(|r| r.unwrap())
        .collect();
    assert_eq!(before_roles, after_roles);
    store.integrity_check().unwrap();
}

#[test]
fn team_relationship_time_and_journal_failures_never_commit() {
    let mut store = Store::memory().unwrap();
    let team = team_fixture(&mut store);
    let mut missing = team.clone();
    missing.members[1].role_id = id!(RoleId, "missing");
    missing.members[0].independent_reviewers = BTreeSet::from([missing.members[1].role_id.clone()]);
    assert!(
        store
            .replace_team(
                id!(CommandId, "missing"),
                None,
                missing,
                id!(UserId, "owner"),
                Timestamp(10)
            )
            .is_err()
    );
    let mut wrong_lead = team.clone();
    wrong_lead.lead_role_id = wrong_lead.members[1].role_id.clone();
    wrong_lead.members[0].function = RoleFunction::GeneralExecution;
    wrong_lead.members[1].function = RoleFunction::LeadOrchestrator;
    assert!(matches!(
        store.replace_team(
            id!(CommandId, "lead"),
            None,
            wrong_lead,
            id!(UserId, "owner"),
            Timestamp(10)
        ),
        Err(StoreError::RelationshipMismatch)
    ));
    store
        .replace_team(
            id!(CommandId, "initial"),
            None,
            team.clone(),
            id!(UserId, "owner"),
            Timestamp(10),
        )
        .unwrap();
    let mut updated = team.clone();
    updated.revision = Revision(1);
    assert!(matches!(
        store.replace_team(
            id!(CommandId, "old-time"),
            Some(Revision(0)),
            updated.clone(),
            id!(UserId, "owner"),
            Timestamp(9)
        ),
        Err(StoreError::InvalidTeam)
    ));
    store.connection.execute_batch("CREATE TRIGGER fail_team_journal BEFORE INSERT ON journal BEGIN SELECT RAISE(ABORT,'fixture journal failure'); END;").unwrap();
    assert!(
        store
            .replace_team(
                id!(CommandId, "journal-failure"),
                Some(Revision(0)),
                updated,
                id!(UserId, "owner"),
                Timestamp(11)
            )
            .is_err()
    );
    assert_eq!(store.get_team(&team.project_id).unwrap(), team);
    store.integrity_check().unwrap();
}

#[test]
fn team_compare_and_swap_has_one_concurrent_winner() {
    let temp = Temporary::new();
    let mut store = Store::open(temp.database()).unwrap();
    let team = team_fixture(&mut store);
    store
        .replace_team(
            id!(CommandId, "initial"),
            None,
            team.clone(),
            id!(UserId, "owner"),
            Timestamp(10),
        )
        .unwrap();
    let barrier = Arc::new(Barrier::new(2));
    let handles: Vec<_> = (0..2)
        .map(|n| {
            let path = temp.database();
            let mut team = team.clone();
            let barrier = barrier.clone();
            thread::spawn(move || {
                let mut store = Store::open(path).unwrap();
                team.revision = Revision(1);
                team.members[0].responsibilities = vec![format!("candidate{n}")];
                barrier.wait();
                store.replace_team(
                    id!(CommandId, format!("candidate{n}")),
                    Some(Revision(0)),
                    team,
                    id!(UserId, "owner"),
                    Timestamp(11),
                )
            })
        })
        .collect();
    let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter(|r| matches!(r, Err(StoreError::TeamRevisionConflict)))
            .count(),
        1
    );
    store.integrity_check().unwrap();
}

#[test]
fn team_materialization_missing_tampered_and_forged_rows_are_rejected() {
    for tamper in [
        "DELETE FROM team_configurations",
        "UPDATE team_configurations SET revision=99",
        "UPDATE team_configurations SET actor='forged'",
        "UPDATE team_configurations SET updated_at=99",
    ] {
        let temp = Temporary::new();
        let mut store = Store::open(temp.database()).unwrap();
        let team = team_fixture(&mut store);
        store
            .replace_team(
                id!(CommandId, "initial"),
                None,
                team,
                id!(UserId, "owner"),
                Timestamp(10),
            )
            .unwrap();
        store.connection.execute(tamper, []).unwrap();
        drop(store);
        assert!(Store::open(temp.database()).is_err());
    }
}

#[test]
fn team_v3_migration_preserves_data_and_corrupt_upgrade_rolls_back() {
    for corrupt in [false, true] {
        let temp = Temporary::new();
        let mut store = Store::open(temp.database()).unwrap();
        let team = team_fixture(&mut store);
        let before = store.events(&team.project_id, 0, 10).unwrap();
        store
            .connection
            .execute_batch("DROP TABLE elevation_requests; DROP TABLE elevations; DROP TABLE model_descriptors; DROP TABLE billing_entitlements; DROP TABLE dispatch_preparations; DROP TABLE provider_connections; DROP TABLE task_leases; DROP TABLE task_dependencies; DROP TABLE work_routes; DROP TABLE workforce_bindings; DROP TABLE team_configurations; PRAGMA user_version=3;")
            .unwrap();
        if corrupt {
            store
                .connection
                .execute("UPDATE projects SET revision=9", [])
                .unwrap();
        }
        drop(store);
        if corrupt {
            assert!(Store::open(temp.database()).is_err());
            let db = Connection::open(temp.database()).unwrap();
            assert_eq!(
                db.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
                    .unwrap(),
                3
            );
            assert_eq!(
                db.query_row(
                    "SELECT count(*) FROM sqlite_schema WHERE name='team_configurations'",
                    [],
                    |r| r.get::<_, i64>(0)
                )
                .unwrap(),
                0
            );
        } else {
            let store = Store::open(temp.database()).unwrap();
            assert_eq!(store.events(&team.project_id, 0, 10).unwrap(), before);
            assert!(matches!(
                store.get_team(&team.project_id),
                Err(StoreError::NotFound)
            ));
            store.integrity_check().unwrap();
        }
    }
}

fn work_spec(project: ProjectId, role: RoleId, name: &str) -> WorkSpec {
    WorkSpec {
        id: WorkId::Objective(id!(ObjectiveId, name)),
        project_id: project,
        role_id: role,
        title: "Explicit fixture maintenance".into(),
        description: "Classified fixture task origin".into(),
        utterance: None,
        objective_class: Some(ObjectiveClass::Maintenance),
        parent: None,
        dependencies: BTreeSet::new(),
        requirements: vec![],
        constraints: vec![],
        risks: vec![],
        acceptance: vec!["fixture verified".into()],
        priority: 1,
        budget: None,
        external_references: vec![],
    }
}
fn register_work_target(store: &mut Store, task: &Task) -> Result<TaskOrigin> {
    let spec = work_spec(
        task.project_id().clone(),
        task.role_id().clone(),
        task.project_id().as_str(),
    );
    let origin = TaskOrigin::Objective(spec.reference());
    let item = WorkItem::new(spec, id!(UserId, "owner"), Timestamp(10))?;
    store.create_work_item(
        id!(CommandId, format!("work-{}", task.project_id().as_str())),
        item,
    )?;
    Ok(origin)
}
trait FixtureTaskCreation {
    fn create_fixture_task(
        &mut self,
        id: CommandId,
        task: Task,
        stream: ChangeStream,
    ) -> Result<Receipt>;
}
impl FixtureTaskCreation for Store {
    fn create_fixture_task(
        &mut self,
        id: CommandId,
        task: Task,
        stream: ChangeStream,
    ) -> Result<Receipt> {
        let origin = register_work_target(self, &task)?;
        self.create_task(id, task, stream, origin)
    }
}

fn new_work(project: &Project, role: &Role, name: &str) -> WorkItem {
    WorkItem::new(
        work_spec(project.id.clone(), role.id.clone(), name),
        id!(UserId, "owner"),
        Timestamp(10),
    )
    .unwrap()
}
fn revise_work(item: &WorkItem, name: &str, spec: WorkSpec) -> WorkCommand {
    WorkCommand {
        id: id!(CommandId, name),
        expected_revision: item.revision(),
        actor: Actor::User(id!(UserId, "owner")),
        at: Timestamp(11),
        action: WorkAction::Revise {
            spec: Box::new(spec),
        },
    }
}

#[test]
fn work_graph_reopens_retries_and_rejects_global_cycles_and_missing_references() {
    let temp = Temporary::new();
    let mut store = Store::open(temp.database()).unwrap();
    let (project, _, roles) = register(&mut store, "one");
    let (other, _, other_roles) = register(&mut store, "two");
    let a = new_work(&project, &roles[0], "a");
    let first = store
        .create_work_item(id!(CommandId, "work-a"), a.clone())
        .unwrap();
    assert!(
        store
            .create_work_item(id!(CommandId, "work-a"), a.clone())
            .unwrap()
            .replayed
    );
    assert_eq!(
        store
            .work_command_timestamp(&id!(CommandId, "work-a"))
            .unwrap(),
        Some(Timestamp(10))
    );
    let mut b_spec = work_spec(other.id.clone(), other_roles[0].id.clone(), "b");
    b_spec.dependencies.insert(a.spec().reference());
    let b = WorkItem::new(b_spec, id!(UserId, "owner"), Timestamp(10)).unwrap();
    store
        .create_work_item(id!(CommandId, "work-b"), b.clone())
        .unwrap();
    let mut cycle = a.spec().clone();
    cycle.dependencies.insert(b.spec().reference());
    assert!(matches!(
        store.apply_work_command(&project.id, a.id(), revise_work(&a, "cycle", cycle)),
        Err(StoreError::Work(WorkError::Cycle))
    ));
    let mut missing = a.spec().clone();
    missing.dependencies.insert(WorkRef {
        project_id: project.id.clone(),
        id: WorkId::Objective(id!(ObjectiveId, "absent")),
    });
    assert!(matches!(
        store.apply_work_command(&project.id, a.id(), revise_work(&a, "missing", missing)),
        Err(StoreError::Work(WorkError::MissingReference))
    ));
    let mut changed = a.spec().clone();
    changed.title = "Changed".into();
    let command = revise_work(&a, "revise", changed);
    let receipt = store
        .apply_work_command(&project.id, a.id(), command.clone())
        .unwrap();
    assert!(receipt.sequence > first.sequence);
    assert!(
        store
            .apply_work_command(&project.id, a.id(), command)
            .unwrap()
            .replayed
    );
    assert!(matches!(
        store.work_item(&other.id, a.id()),
        Err(StoreError::NotFound)
    ));
    drop(store);
    let store = Store::open(temp.database()).unwrap();
    assert_eq!(
        store.work_item(&project.id, a.id()).unwrap().spec().title,
        "Changed"
    );
    store.integrity_check().unwrap();
}

#[test]
fn concurrent_work_revisions_commit_once() {
    let temp = Temporary::new();
    let mut store = Store::open(temp.database()).unwrap();
    let (project, _, roles) = register(&mut store, "one");
    let item = new_work(&project, &roles[0], "work");
    store
        .create_work_item(id!(CommandId, "work"), item.clone())
        .unwrap();
    let barrier = Arc::new(Barrier::new(2));
    let handles: Vec<_> = (0..2)
        .map(|n| {
            let path = temp.database();
            let barrier = barrier.clone();
            let item = item.clone();
            thread::spawn(move || {
                let mut store = Store::open(path).unwrap();
                let mut spec = item.spec().clone();
                spec.title = format!("winner{n}");
                let command = revise_work(&item, &format!("revise{n}"), spec);
                barrier.wait();
                store.apply_work_command(item.project_id(), item.id(), command)
            })
        })
        .collect();
    let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter(|r| matches!(r, Err(StoreError::Work(WorkError::RevisionConflict))))
            .count(),
        1
    );
    store.integrity_check().unwrap();
}

fn insert_legacy_task(store: &mut Store, task: &Task, stream: &ChangeStream) {
    let tx = store.connection.transaction().unwrap();
    tx.execute("INSERT INTO streams(id,project_id,root_id,worktree_id,branch,body) VALUES (?1,?2,?3,?4,?5,?6)",params![stream.id().as_str(),task.project_id().as_str(),task.root_id().as_str(),stream.worktree.as_str(),stream.branch,serde_json::to_string(stream).unwrap()]).unwrap();
    tx.execute("INSERT INTO tasks(id,project_id,root_id,role_id,stream_id,revision,body) VALUES (?1,?2,?3,?4,?5,0,?6)",params![task.id().as_str(),task.project_id().as_str(),task.root_id().as_str(),task.role_id().as_str(),task.stream_id().as_str(),serde_json::to_string(task).unwrap()]).unwrap();
    let event = EventPayload::TaskCreated {
        task: Box::new(task.clone()),
        stream: Box::new(stream.clone()),
        origin: None,
    };
    append(
        &tx,
        task.project_id(),
        &id!(CommandId, "legacy-task"),
        Revision(0),
        &serde_json::to_string(&event).unwrap(),
        &event,
    )
    .unwrap();
    tx.commit().unwrap();
}

#[test]
fn v2_legacy_tasks_remain_unclassified_until_explicit_durable_assignment() {
    let temp = Temporary::new();
    let mut store = Store::open(temp.database()).unwrap();
    let (project, _, roles) = register(&mut store, "one");
    let (task, stream) = task_records("one");
    insert_legacy_task(&mut store, &task, &stream);
    let before = store.events(&project.id, 0, 10).unwrap();
    store
        .connection
        .execute_batch("DROP TABLE elevation_requests; DROP TABLE elevations; DROP TABLE model_descriptors; DROP TABLE billing_entitlements; DROP TABLE dispatch_preparations; DROP TABLE provider_connections; DROP TABLE task_leases; DROP TABLE task_dependencies; DROP TABLE work_routes; DROP TABLE workforce_bindings; DROP TABLE team_configurations; DROP TABLE task_origins; DROP TABLE work_items; PRAGMA user_version=2;")
        .unwrap();
    drop(store);
    let mut store = Store::open(temp.database()).unwrap();
    assert_eq!(store.events(&project.id, 0, 10).unwrap(), before);
    assert_eq!(store.task_origin(&project.id, task.id()).unwrap(), None);
    assert!(matches!(
        store.apply_task(task.id(), start_command(&task, &roles[0])),
        Err(StoreError::UnclassifiedTask)
    ));
    let origin = register_work_target(&mut store, &task).unwrap();
    let receipt = store
        .assign_task_origin(
            id!(CommandId, "classify"),
            &project.id,
            task.id(),
            origin.clone(),
            id!(UserId, "owner"),
            Timestamp(12),
        )
        .unwrap();
    assert!(
        store
            .assign_task_origin(
                id!(CommandId, "classify"),
                &project.id,
                task.id(),
                origin.clone(),
                id!(UserId, "owner"),
                Timestamp(12)
            )
            .unwrap()
            .replayed
    );
    assert!(matches!(
        store.assign_task_origin(
            id!(CommandId, "replace"),
            &project.id,
            task.id(),
            origin.clone(),
            id!(UserId, "owner"),
            Timestamp(13)
        ),
        Err(StoreError::AlreadyExists)
    ));
    assert_eq!(receipt.revision, Revision(0));
    store
        .apply_task(task.id(), start_command(&task, &roles[0]))
        .unwrap();
    drop(store);
    let store = Store::open(temp.database()).unwrap();
    assert_eq!(
        store.task_origin(&project.id, task.id()).unwrap(),
        Some(origin)
    );
    store.integrity_check().unwrap();
}

#[test]
fn materialization_tampering_and_corrupt_v2_upgrade_are_not_repaired() {
    let temp = Temporary::new();
    let mut store = Store::open(temp.database()).unwrap();
    let (project, _, roles) = register(&mut store, "one");
    let item = new_work(&project, &roles[0], "work");
    store
        .create_work_item(id!(CommandId, "work"), item.clone())
        .unwrap();
    store
        .connection
        .execute("UPDATE work_items SET revision=42", [])
        .unwrap();
    drop(store);
    assert!(Store::open(temp.database()).is_err());
    let temp = Temporary::new();
    let mut store = Store::open(temp.database()).unwrap();
    register(&mut store, "one");
    store.connection.execute_batch("DROP TABLE elevation_requests; DROP TABLE elevations; DROP TABLE model_descriptors; DROP TABLE billing_entitlements; DROP TABLE dispatch_preparations; DROP TABLE provider_connections; DROP TABLE task_leases; DROP TABLE task_dependencies; DROP TABLE work_routes; DROP TABLE workforce_bindings; DROP TABLE team_configurations; DROP TABLE task_origins; DROP TABLE work_items; PRAGMA user_version=2; UPDATE projects SET revision=9;").unwrap();
    drop(store);
    assert!(Store::open(temp.database()).is_err());
    let connection = Connection::open(temp.database()).unwrap();
    assert_eq!(
        connection
            .pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
            .unwrap(),
        2
    );
    assert_eq!(
        connection
            .query_row(
                "SELECT count(*) FROM sqlite_schema WHERE name='work_items'",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
        0
    );
}

#[test]
fn root_placement_observation_advances_replays_and_audits() {
    let temp = Temporary::new();
    let mut store = Store::open(temp.database()).unwrap();
    let (project, roots, roles) = records("placement");
    store
        .register_project(
            id!(CommandId, "register-placement"),
            project.clone(),
            roots,
            roles,
        )
        .unwrap();
    let root_id = id!(RootId, "root-placement");
    let host = id!(HostId, "host-placement");
    let actor = id!(UserId, "owner");
    let observe = |store: &mut Store, command: &str, expected: Revision, path: &str, at: u64| {
        store.observe_root_placement(
            id!(CommandId, command),
            &project.id,
            &root_id,
            &host,
            path,
            expected,
            &actor,
            Timestamp(at),
        )
    };
    // First observation: revision 0 → 1, with the server time recoverable
    // for identical retries.
    let first = observe(&mut store, "observe-1", Revision(0), "/repos/placement", 20).unwrap();
    assert_eq!(first.revision, Revision(1));
    assert!(!first.replayed);
    assert_eq!(
        store
            .placement_command_timestamp(&id!(CommandId, "observe-1"))
            .unwrap(),
        Some(Timestamp(20))
    );
    // An exact retry replays the recorded receipt; a different intent under
    // the same command id cannot.
    let replay = observe(&mut store, "observe-1", Revision(0), "/repos/placement", 20).unwrap();
    assert!(replay.replayed);
    assert_eq!(replay.sequence, first.sequence);
    assert!(matches!(
        observe(&mut store, "observe-1", Revision(0), "/repos/changed", 20).unwrap_err(),
        StoreError::IdempotencyConflict
    ));
    // A stale expected revision refuses without recording; the placement
    // moves only from the current revision.
    assert!(matches!(
        observe(&mut store, "observe-stale", Revision(0), "/repos/next", 21).unwrap_err(),
        StoreError::Domain(DomainError::RevisionConflict)
    ));
    let moved = observe(&mut store, "observe-2", Revision(1), "/repos/next", 22).unwrap();
    assert_eq!(moved.revision, Revision(2));
    // A placement under a foreign project identity refuses.
    assert!(matches!(
        store.observe_root_placement(
            id!(CommandId, "observe-foreign"),
            &id!(ProjectId, "project-other"),
            &root_id,
            &host,
            "/repos/next",
            Revision(2),
            &actor,
            Timestamp(23),
        ),
        Err(StoreError::RelationshipMismatch)
    ));
    // Reopening replays the journal audit across the observed placements.
    drop(store);
    let reopened = Store::open(temp.database()).unwrap();
    let body: String = reopened
        .connection
        .query_row(
            "SELECT body FROM roots WHERE id=?1",
            [root_id.as_str()],
            |r| r.get::<_, String>(0),
        )
        .unwrap();
    let root: Root = serde_json::from_str(&body).unwrap();
    assert_eq!(root.revision, Revision(2));
    assert_eq!(
        root.host_paths.get(&host).map(String::as_str),
        Some("/repos/next")
    );
    // The journal is append-only at the schema level: even a same-uid
    // process cannot rewrite a revision cell to forge a receipt or a
    // replayed one. The audit's row-revision check is the belt to this
    // braces: it rejects any row that slips past the trigger's reach.
    let tampered = reopened;
    let rewrite = tampered.connection.execute(
        "UPDATE journal SET revision=5 WHERE command_id=?1",
        [id!(CommandId, "observe-2").as_str()],
    );
    assert!(matches!(rewrite, Err(rusqlite::Error::SqliteFailure(_, _))));
}

#[test]
fn task_origin_target_cannot_be_invalidated_or_forged() {
    let temp = Temporary::new();
    let mut store = Store::open(temp.database()).unwrap();
    let (project, _, _) = register(&mut store, "one");
    let (task, stream) = task_records("one");
    store
        .create_fixture_task(id!(CommandId, "task"), task.clone(), stream)
        .unwrap();
    let origin = store.task_origin(&project.id, task.id()).unwrap().unwrap();
    let item = store.work_item(&project.id, &origin.work_ref().id).unwrap();
    let mut spec = item.spec().clone();
    spec.objective_class = Some(ObjectiveClass::Outcome);
    assert!(matches!(
        store.apply_work_command(
            &project.id,
            item.id(),
            revise_work(&item, "invalidate", spec)
        ),
        Err(StoreError::Work(WorkError::InvalidTaskOrigin))
    ));
    store
        .connection
        .execute("DELETE FROM task_origins", [])
        .unwrap();
    drop(store);
    assert!(Store::open(temp.database()).is_err());
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
            minimum_enforcement: &std::collections::BTreeMap::new(),
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
        store.create_fixture_task(id!(CommandId, "bad"), wrong_task, wrong_stream),
        Err(StoreError::RelationshipMismatch)
    ));
    store
        .create_fixture_task(id!(CommandId, "create"), task.clone(), stream.clone())
        .unwrap();
    let (second, mut second_stream) = task_records("two");
    second_stream.worktree = stream.worktree;
    assert!(
        store
            .create_fixture_task(id!(CommandId, "shared"), second.clone(), second_stream)
            .is_err()
    );
    assert!(matches!(store.task(second.id()), Err(StoreError::NotFound)));
    assert_eq!(
        store.events(task.project_id(), 0, 10).unwrap().events.len(),
        3
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
            .create_fixture_task(id!(CommandId, "create"), task, stream)
            .unwrap();
    }
    let store = Store::open(temporary.database()).unwrap();
    assert_eq!(store.project(&project.id).unwrap(), project);
    let first = store.events(&project.id, 0, 1).unwrap();
    assert!(first.has_more);
    assert_eq!(first.events.len(), 1);
    let second = store.events(&project.id, first.next_cursor, 1).unwrap();
    assert!(second.has_more);
    let third = store.events(&project.id, second.next_cursor, 1).unwrap();
    assert!(!third.has_more);
    assert!(matches!(
        third.events[0].payload,
        EventPayload::TaskCreated { .. }
    ));
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
        .create_fixture_task(id!(CommandId, "create"), task.clone(), stream)
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
        4
    );
}

#[test]
fn two_connections_racing_terminal_transitions_have_one_winner() {
    let temporary = Temporary::new();
    let mut store = Store::open(temporary.database()).unwrap();
    let (_, _, roles) = register(&mut store, "one");
    let (task, stream) = task_records("one");
    store
        .create_fixture_task(id!(CommandId, "create"), task.clone(), stream)
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
        5
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
    let (task, stream) = task_records("one");
    register_work_target(&mut store, &task).unwrap();
    store.connection.execute_batch("CREATE TRIGGER fail_new_event BEFORE INSERT ON journal BEGIN SELECT RAISE(ABORT,'injected journal failure'); END;").unwrap();
    assert!(
        store
            .create_fixture_task(id!(CommandId, "create"), task.clone(), stream)
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
        .pragma_update(None, "user_version", 13)
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
        13
    );
}

fn consent_fixture(name: &str) -> ResourceConsent {
    ResourceConsent {
        id: id!(CommandId, name),
        snapshot: symbiote_trust::ResourceSnapshot {
            project_id: id!(ProjectId, "project-one"),
            role_id: id!(RoleId, "role-one"),
            profile_id: id!(RuntimeProfileId, "profile-one"),
            host_id: id!(HostId, "host-one"),
            resource_ref: "fixture-skill".into(),
            fingerprint: symbiote_trust::Fingerprint::of(b"fixture"),
            access: AccessSnapshot {
                project_id: id!(ProjectId, "project-one"),
                roots: BTreeSet::from([id!(RootId, "root-one")]),
                grants: BTreeSet::new(),
                policy_revision: Revision(0),
            },
        },
        user_id: id!(UserId, "owner"),
        issued_at: Timestamp(10),
        expires_at: Timestamp(100),
        revoked_at: None,
    }
}

#[test]
fn resource_consents_reopen_revoke_and_exact_retries_are_durable() {
    let temporary = Temporary::new();
    let mut store = Store::open(temporary.database()).unwrap();
    register(&mut store, "one");
    let consent = consent_fixture("consent-one");
    let receipt = store
        .record_resource_consent(consent.id.clone(), consent.clone())
        .unwrap();
    assert_eq!(receipt.revision, Revision(0));
    assert!(
        store
            .record_resource_consent(consent.id.clone(), consent.clone())
            .unwrap()
            .replayed
    );
    assert_eq!(
        store.consent_command_timestamp(&consent.id).unwrap(),
        Some(Timestamp(10))
    );
    let mut changed = consent.clone();
    changed.expires_at = Timestamp(101);
    assert!(matches!(
        store.record_resource_consent(changed.id.clone(), changed),
        Err(StoreError::IdempotencyConflict)
    ));
    drop(store);
    let mut store = Store::open(temporary.database()).unwrap();
    assert_eq!(
        store
            .resource_consent(&consent.snapshot.project_id, &consent.id)
            .unwrap(),
        consent
    );
    let revoke = id!(CommandId, "revoke-one");
    let actor = id!(UserId, "revoker");
    assert_eq!(
        store
            .revoke_resource_consent(
                revoke.clone(),
                &consent.snapshot.project_id,
                &consent.id,
                &actor,
                Timestamp(20)
            )
            .unwrap()
            .revision,
        Revision(1)
    );
    assert!(
        store
            .revoke_resource_consent(
                revoke.clone(),
                &consent.snapshot.project_id,
                &consent.id,
                &actor,
                Timestamp(20)
            )
            .unwrap()
            .replayed
    );
    assert!(matches!(
        store.revoke_resource_consent(
            revoke.clone(),
            &consent.snapshot.project_id,
            &consent.id,
            &consent.user_id,
            Timestamp(20)
        ),
        Err(StoreError::IdempotencyConflict)
    ));
    assert_eq!(
        store.consent_command_timestamp(&revoke).unwrap(),
        Some(Timestamp(20))
    );
    drop(store);
    let store = Store::open(temporary.database()).unwrap();
    assert_eq!(
        store
            .resource_consent(&consent.snapshot.project_id, &consent.id)
            .unwrap()
            .revoked_at,
        Some(Timestamp(20))
    );
    let events = store.events(&consent.snapshot.project_id, 0, 10).unwrap();
    assert_eq!(events.events.len(), 3);
    assert!(
        matches!(&events.events[2].payload,EventPayload::ResourceConsentRevoked{revoked_by,..} if revoked_by==&actor)
    );
}

#[test]
fn consent_relationships_invalid_times_and_cross_project_revocation_roll_back() {
    let mut store = Store::memory().unwrap();
    register(&mut store, "one");
    register(&mut store, "two");
    let consent = consent_fixture("consent-one");
    let mut invalid = consent.clone();
    invalid.snapshot.access.roots = BTreeSet::from([id!(RootId, "root-two")]);
    assert!(matches!(
        store.record_resource_consent(invalid.id.clone(), invalid),
        Err(StoreError::RelationshipMismatch)
    ));
    let mut invalid = consent.clone();
    invalid.snapshot.role_id = id!(RoleId, "role-two");
    assert!(matches!(
        store.record_resource_consent(invalid.id.clone(), invalid),
        Err(StoreError::RelationshipMismatch)
    ));
    let mut invalid = consent.clone();
    invalid.expires_at = invalid.issued_at;
    assert!(matches!(
        store.record_resource_consent(invalid.id.clone(), invalid),
        Err(StoreError::InvalidConsent)
    ));
    assert!(matches!(
        store.record_resource_consent(id!(CommandId, "different"), consent.clone()),
        Err(StoreError::InvalidConsent)
    ));
    assert_eq!(
        store
            .events(&consent.snapshot.project_id, 0, 10)
            .unwrap()
            .events
            .len(),
        1
    );
    store
        .record_resource_consent(consent.id.clone(), consent.clone())
        .unwrap();
    let revoke = id!(CommandId, "revoke");
    assert!(matches!(
        store.revoke_resource_consent(
            revoke.clone(),
            &id!(ProjectId, "project-two"),
            &consent.id,
            &consent.user_id,
            Timestamp(20)
        ),
        Err(StoreError::NotFound)
    ));
    assert!(matches!(
        store.revoke_resource_consent(
            revoke.clone(),
            &consent.snapshot.project_id,
            &consent.id,
            &consent.user_id,
            Timestamp(9)
        ),
        Err(StoreError::InvalidConsent)
    ));
    assert_eq!(store.consent_command_timestamp(&revoke).unwrap(), None);
    assert_eq!(
        store
            .resource_consent(&consent.snapshot.project_id, &consent.id)
            .unwrap(),
        consent
    );
    store.integrity_check().unwrap();
}

#[test]
fn v1_migration_preserves_existing_records_and_journal() {
    let temporary = Temporary::new();
    let mut store = Store::open(temporary.database()).unwrap();
    register(&mut store, "one");
    let before = store.events(&id!(ProjectId, "project-one"), 0, 10).unwrap();
    // Exact v1 layout: v2 only adds this table and bumps user_version.
    store
        .connection
        .execute_batch("DROP TABLE elevation_requests; DROP TABLE elevations; DROP TABLE model_descriptors; DROP TABLE billing_entitlements; DROP TABLE dispatch_preparations; DROP TABLE provider_connections; DROP TABLE task_leases; DROP TABLE task_dependencies; DROP TABLE work_routes; DROP TABLE workforce_bindings; DROP TABLE team_configurations; DROP TABLE task_origins; DROP TABLE work_items; DROP TABLE resource_consents; PRAGMA user_version=1;")
        .unwrap();
    drop(store);
    let mut store = Store::open(temporary.database()).unwrap();
    assert_eq!(
        store.events(&id!(ProjectId, "project-one"), 0, 10).unwrap(),
        before
    );
    assert_eq!(
        store
            .connection
            .pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
            .unwrap(),
        12
    );
    let consent = consent_fixture("consent-after-migration");
    store
        .record_resource_consent(consent.id.clone(), consent)
        .unwrap();
    store.integrity_check().unwrap();
}

#[test]
fn consent_state_and_revocation_journal_tampering_are_refused() {
    for journal_tamper in [false, true] {
        let temporary = Temporary::new();
        let mut store = Store::open(temporary.database()).unwrap();
        register(&mut store, "one");
        let consent = consent_fixture("consent-one");
        store
            .record_resource_consent(consent.id.clone(), consent.clone())
            .unwrap();
        store
            .revoke_resource_consent(
                id!(CommandId, "revoke"),
                &consent.snapshot.project_id,
                &consent.id,
                &consent.user_id,
                Timestamp(20),
            )
            .unwrap();
        if journal_tamper {
            store.connection.execute_batch("DROP TRIGGER journal_no_update; UPDATE journal SET payload=json_set(payload,'$.data.consent.revoked_at',21) WHERE command_id='revoke'; CREATE TRIGGER journal_no_update BEFORE UPDATE ON journal BEGIN SELECT RAISE(ABORT, 'append-only journal'); END;").unwrap();
        } else {
            store.connection.execute("UPDATE resource_consents SET body=json_set(body,'$.expires_at',101) WHERE id=?1",[consent.id.as_str()]).unwrap();
        }
        drop(store);
        assert!(Store::open(temporary.database()).is_err());
    }
}

#[test]
fn consent_journal_failure_rolls_back_insert_and_revocation() {
    let mut store = Store::memory().unwrap();
    register(&mut store, "one");
    let consent = consent_fixture("consent-one");
    store.connection.execute_batch("CREATE TRIGGER deny_consent BEFORE INSERT ON journal WHEN NEW.command_id='consent-one' BEGIN SELECT RAISE(ABORT,'fixture'); END;").unwrap();
    assert!(
        store
            .record_resource_consent(consent.id.clone(), consent.clone())
            .is_err()
    );
    assert!(matches!(
        store.resource_consent(&consent.snapshot.project_id, &consent.id),
        Err(StoreError::NotFound)
    ));
    store
        .connection
        .execute_batch("DROP TRIGGER deny_consent")
        .unwrap();
    store
        .record_resource_consent(consent.id.clone(), consent.clone())
        .unwrap();
    store.connection.execute_batch("CREATE TRIGGER deny_revoke BEFORE INSERT ON journal WHEN NEW.command_id='revoke' BEGIN SELECT RAISE(ABORT,'fixture'); END;").unwrap();
    assert!(
        store
            .revoke_resource_consent(
                id!(CommandId, "revoke"),
                &consent.snapshot.project_id,
                &consent.id,
                &consent.user_id,
                Timestamp(20)
            )
            .is_err()
    );
    assert_eq!(
        store
            .resource_consent(&consent.snapshot.project_id, &consent.id)
            .unwrap(),
        consent
    );
    assert_eq!(
        store
            .consent_command_timestamp(&id!(CommandId, "revoke"))
            .unwrap(),
        None
    );
    store.integrity_check().unwrap();
}

#[test]
fn corrupt_v1_migration_rolls_back_schema_and_version() {
    for semantic_only in [false, true] {
        let temporary = Temporary::new();
        let mut store = Store::open(temporary.database()).unwrap();
        register(&mut store, "one");
        store
            .connection
            .execute_batch("DROP TABLE elevation_requests; DROP TABLE elevations; DROP TABLE model_descriptors; DROP TABLE billing_entitlements; DROP TABLE dispatch_preparations; DROP TABLE provider_connections; DROP TABLE task_leases; DROP TABLE task_dependencies; DROP TABLE work_routes; DROP TABLE workforce_bindings; DROP TABLE team_configurations; DROP TABLE task_origins; DROP TABLE work_items; DROP TABLE resource_consents; PRAGMA user_version=1;")
            .unwrap();
        if semantic_only {
            store
                .connection
                .execute_batch("UPDATE projects SET body=json_set(body,'$.name','changed');")
                .unwrap();
        } else {
            store.connection.execute_batch("PRAGMA foreign_keys=OFF; INSERT INTO roots(id,project_id,body) VALUES('orphan','missing','{}');").unwrap();
        }
        drop(store);
        assert!(Store::open(temporary.database()).is_err());
        let connection = Connection::open(temporary.database()).unwrap();
        assert_eq!(
            connection
                .pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
                .unwrap(),
            1
        );
        assert_eq!(
            connection
                .query_row(
                    "SELECT count(*) FROM sqlite_schema WHERE name='resource_consents'",
                    [],
                    |r| r.get::<_, i64>(0)
                )
                .unwrap(),
            0
        );
        assert_eq!(
            connection
                .query_row("SELECT count(*) FROM journal", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            1
        );
    }
}

#[test]
fn invalid_domain_snapshot_is_detected_on_reopen() {
    let temporary = Temporary::new();
    let mut store = Store::open(temporary.database()).unwrap();
    register(&mut store, "one");
    let (task, stream) = task_records("one");
    store
        .create_fixture_task(id!(CommandId, "create"), task.clone(), stream)
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
