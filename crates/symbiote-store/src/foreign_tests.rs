use super::*;

/// Registers a project plus one Ready task (with objective origin) for the
/// foreign reference tests. Returns (project, task) identities.
fn register_task(store: &mut Store, tag: &str) -> (ProjectId, TaskId) {
    let (project, roots, roles) = records(&format!("foreign-{tag}"));
    store
        .register_project(
            id!(CommandId, &format!("foreign-register-{tag}")),
            project.clone(),
            roots,
            roles,
        )
        .unwrap();
    let (task, _) = task_records(tag);
    // task_records shares one project; build the task against this test's
    // project identity directly.
    let task = Task::new(
        task.id().clone(),
        project.id.clone(),
        project.roots.iter().next().unwrap().clone(),
        project.lead.clone(),
        task.stream_id().clone(),
        VersionedTaskContract {
            id: id!(TaskContractId, &format!("foreign-contract-{tag}")),
            revision: Revision(1),
        },
    );
    let stream = ChangeStream::new(NewChangeStream {
        id: task.stream_id().clone(),
        project_id: project.id.clone(),
        root_id: project.roots.iter().next().unwrap().clone(),
        tasks: BTreeSet::from([task.id().clone()]),
        originating_chat: id!(ChatId, &format!("foreign-chat-{tag}")),
        worktree: id!(WorktreeId, &format!("foreign-worktree-{tag}")),
        branch: format!("symbiote/foreign/{tag}"),
        lineage: StreamLineage::Independent,
        base: sha('a'),
        target: sha('b'),
    })
    .unwrap();
    let origin = register_work_target(store, &task).unwrap();
    store
        .create_task(
            id!(CommandId, &format!("foreign-task-{tag}")),
            task,
            stream,
            origin,
        )
        .unwrap();
    (project.id.clone(), id!(TaskId, &format!("task-{tag}")))
}

fn link(kind: ForeignItemKind, foreign_id: &str, session: Option<&str>) -> ForeignTaskLink {
    ForeignTaskLink {
        system: ExternalSystem::Harness,
        kind,
        foreign_id: foreign_id.to_owned(),
        foreign_session_id: session.map(str::to_owned),
        // Every fixture link reports what a foreign harness calls done: the
        // case that matters is what the canonical Task does with that.
        foreign_status: ForeignStatus::Complete,
        observed_at: Timestamp(10),
    }
}

fn session_link(foreign_id: &str) -> ForeignTaskLink {
    link(ForeignItemKind::Session, foreign_id, None)
}

fn set_links(
    store: &mut Store,
    command: &str,
    project: &ProjectId,
    task: &TaskId,
    links: &[ForeignTaskLink],
    at: u64,
) -> Result<Receipt> {
    store.set_task_foreign_links(
        id!(CommandId, command),
        project.clone(),
        task.clone(),
        links.iter().cloned().collect(),
        id!(UserId, "owner"),
        Timestamp(at),
    )
}

#[test]
fn foreign_link_sets_replace_replay_and_survive_restart() {
    let temp = Temporary::new();
    let mut store = Store::open(temp.database()).unwrap();
    let (project, task) = register_task(&mut store, "one");
    let links = vec![
        session_link("codex-session-01"),
        link(
            ForeignItemKind::Task,
            "codex-todo-7",
            Some("codex-session-01"),
        ),
    ];
    let receipt = set_links(&mut store, "foreign-initial", &project, &task, &links, 11).unwrap();
    assert!(!receipt.replayed);
    let replay = set_links(&mut store, "foreign-initial", &project, &task, &links, 11).unwrap();
    assert!(replay.replayed);
    assert_eq!(replay.sequence, receipt.sequence);
    assert_eq!(
        store.task_foreign_links(&project, &task).unwrap().len(),
        2,
        "an identical retry records nothing twice"
    );
    // Replacement supersedes the set; the journal keeps the first one.
    set_links(
        &mut store,
        "foreign-replace",
        &project,
        &task,
        &[session_link("codex-session-02")],
        12,
    )
    .unwrap();
    let current = store.task_foreign_links(&project, &task).unwrap();
    assert_eq!(current.len(), 1);
    assert_eq!(
        current.iter().next().unwrap().foreign_id,
        "codex-session-02"
    );
    let history = store
        .events(&project, 0, 100)
        .unwrap()
        .events
        .iter()
        .filter(|event| matches!(event.payload, EventPayload::TaskForeignLinksSet { .. }))
        .count();
    assert_eq!(history, 2, "both sets are readable as history");
    drop(store);
    let store = Store::open(temp.database()).unwrap();
    assert_eq!(
        store.task_foreign_links(&project, &task).unwrap().len(),
        1,
        "the durable record is what survived the restart"
    );
    assert_eq!(
        store
            .foreign_link_command_timestamp(&id!(CommandId, "foreign-initial"))
            .unwrap(),
        Some(Timestamp(11))
    );
    assert_eq!(
        store
            .foreign_link_command_timestamp(&id!(CommandId, "foreign-never"))
            .unwrap(),
        None
    );
}

#[test]
fn every_foreign_link_refusal_writes_nothing_and_names_itself() {
    let temp = Temporary::new();
    let mut store = Store::open(temp.database()).unwrap();
    let (project, task) = register_task(&mut store, "refuse");
    let (other_project, other_task) = register_task(&mut store, "refuse2");
    set_links(
        &mut store,
        "foreign-keep",
        &project,
        &task,
        &[session_link("codex-session-01")],
        11,
    )
    .unwrap();
    let before = store.task_foreign_links(&project, &task).unwrap();
    let journal_before = store.events(&project, 0, 100).unwrap().events.len();

    let mut wrong_system = session_link("codex-session-02");
    wrong_system.system = ExternalSystem::Ci;
    let mut nested = link(ForeignItemKind::Session, "codex-session-03", Some("other"));
    nested.foreign_id = "codex-session-03".into();
    let mut over = session_link(&"s".repeat(symbiote_domain::MAX_FOREIGN_ID_BYTES + 1));
    over.foreign_id = "s".repeat(symbiote_domain::MAX_FOREIGN_ID_BYTES + 1);
    let mut conflict = session_link("codex-session-04");
    conflict.foreign_status = ForeignStatus::Active;
    let many: Vec<ForeignTaskLink> = (0..symbiote_domain::MAX_FOREIGN_LINKS + 1)
        .map(|index| session_link(&format!("codex-session-{index:03}")))
        .collect();
    for (command, links) in [
        ("foreign-system", vec![wrong_system]),
        ("foreign-nested", vec![nested]),
        ("foreign-over", vec![over]),
        (
            "foreign-conflict",
            vec![session_link("codex-session-04"), conflict],
        ),
        ("foreign-many", many),
    ] {
        assert!(
            matches!(
                set_links(&mut store, command, &project, &task, &links, 12),
                Err(StoreError::InvalidForeignLink)
            ),
            "{command} must be refused as InvalidForeignLink"
        );
    }
    // A Task the Project does not hold is NotFound on both paths, and the
    // other Project's Task is not reachable by naming this Project instead.
    assert!(matches!(
        set_links(
            &mut store,
            "foreign-ghost",
            &project,
            &id!(TaskId, "ghost"),
            &[session_link("codex-session-05")],
            12
        ),
        Err(StoreError::NotFound)
    ));
    assert!(matches!(
        set_links(
            &mut store,
            "foreign-elsewhere",
            &project,
            &other_task,
            &[session_link("codex-session-06")],
            12
        ),
        Err(StoreError::NotFound)
    ));
    assert!(matches!(
        store.task_foreign_links(&project, &id!(TaskId, "ghost")),
        Err(StoreError::NotFound)
    ));
    assert!(matches!(
        store.task_foreign_links(&project, &other_task),
        Err(StoreError::NotFound)
    ));
    assert!(
        store
            .task_foreign_links(&other_project, &other_task)
            .unwrap()
            .is_empty()
    );
    // Every refusal wrote no row and no journal event: the recorded set is
    // exactly what the last accepted command said.
    assert_eq!(
        store.task_foreign_links(&project, &task).unwrap(),
        before,
        "a refused set leaves the recorded set untouched"
    );
    assert_eq!(
        store.events(&project, 0, 100).unwrap().events.len(),
        journal_before
    );
    store.integrity_check().unwrap();
}

#[test]
fn a_foreign_complete_completes_nothing_canonically() {
    let temp = Temporary::new();
    let mut store = Store::open(temp.database()).unwrap();
    let (project, task) = register_task(&mut store, "authority");
    let (other_project, other_task) = register_task(&mut store, "authority2");
    // The foreign harness says its todo is done, for both tasks.
    set_links(
        &mut store,
        "foreign-complete-a",
        &project,
        &task,
        &[link(
            ForeignItemKind::Task,
            "codex-todo-7",
            Some("codex-session-01"),
        )],
        11,
    )
    .unwrap();
    set_links(
        &mut store,
        "foreign-complete-b",
        &other_project,
        &other_task,
        &[session_link("codex-session-02")],
        11,
    )
    .unwrap();
    // The canonical records are byte-for-byte what they were: recording a
    // foreign completion writes no task event, advances no revision and moves
    // no state.
    let before = store.task(&task).unwrap();
    let other_before = store.task(&other_task).unwrap();
    assert_eq!(before.state(), &TaskState::Ready);
    assert_eq!(other_before.state(), &TaskState::Ready);
    let events: BTreeSet<String> = store
        .events(&project, 0, 100)
        .unwrap()
        .events
        .iter()
        .map(|event| format!("{:?}", event.payload))
        .collect();
    assert!(
        !events.iter().any(|payload| payload.contains("TaskChanged")),
        "a foreign link set writes no task transition: {events:?}"
    );
    // A dependency on the task whose harness says it is done is still
    // unresolved: the gate reads canonical state, never a foreign claim.
    store
        .set_task_dependencies(
            id!(CommandId, "foreign-requires"),
            project.clone(),
            task.clone(),
            BTreeSet::from([TaskDependencyEdge {
                kind: TaskDependencyKind::Requires,
                target: TaskDependencyTarget {
                    project_id: other_project.clone(),
                    task_id: other_task.clone(),
                },
            }]),
            id!(UserId, "owner"),
            Timestamp(12),
        )
        .unwrap();
    assert!(matches!(
        store.dependencies_satisfied_for_completion(&project, &task),
        Err(StoreError::DependenciesUnresolved)
    ));
    // And the link set itself is not a completion gate of any kind: a Task
    // with nothing but a foreign `complete` is still Ready.
    assert_eq!(store.task_foreign_links(&project, &task).unwrap().len(), 1);
    assert_eq!(store.task(&task).unwrap().state(), &TaskState::Ready);
}
#[test]
fn tampered_and_forged_foreign_link_rows_are_refused_on_reopen() {
    // Four ways the stored record and its journal can disagree, each caught by
    // one check: a row whose indexed column no longer matches its body, a row
    // with no journal event behind it, a journal event whose recorded request
    // is not the one its payload implies, and a journal event whose links
    // differ from the rows it produced. All four are corruption rather than a
    // set to serve.
    for tamper in [
        "indexed-column",
        "forged-row",
        "journal-request",
        "journal-payload",
    ] {
        let temp = Temporary::new();
        let mut store = Store::open(temp.database()).unwrap();
        let (project, task) = register_task(&mut store, &format!("tamper-{tamper}"));
        set_links(
            &mut store,
            "foreign-tamper",
            &project,
            &task,
            &[session_link("codex-session-01")],
            11,
        )
        .unwrap();
        match tamper {
            // The indexed columns are the row key; a body that no longer
            // projects onto them is a row nobody wrote.
            "indexed-column" => {
                store
                    .connection
                    .execute(
                        "UPDATE task_foreign_links SET foreign_id='codex-session-99' WHERE project_id=?1",
                        params![project.as_str()],
                    )
                    .unwrap();
            }
            // A whole well-formed link with no journal event behind it: its
            // columns and body agree with each other, so only the provenance
            // direction can refuse it.
            "forged-row" => {
                let forged = link(ForeignItemKind::Session, "codex-session-99", None);
                store
                    .connection
                    .execute(
                        "INSERT INTO task_foreign_links(project_id,task_id,system,item_kind,foreign_id,foreign_session_id,body) VALUES (?1,?2,?3,?4,?5,'',?6)",
                        params![
                            project.as_str(),
                            task.as_str(),
                            serde_json::to_string(&forged.system).unwrap(),
                            serde_json::to_string(&forged.kind).unwrap(),
                            forged.foreign_id,
                            serde_json::to_string(&forged).unwrap()
                        ],
                    )
                    .unwrap();
            }
            // The recorded request is not the request the payload implies.
            "journal-request" => {
                store
                    .connection
                    .execute_batch("DROP TRIGGER journal_no_update; UPDATE journal SET request=json_set(request,'$.data.links[0].foreign_status','active') WHERE command_id='foreign-tamper'; CREATE TRIGGER journal_no_update BEFORE UPDATE ON journal BEGIN SELECT RAISE(ABORT, 'append-only journal'); END;")
                    .unwrap();
            }
            // The payload's links differ from the rows that event produced.
            _ => {
                store
                    .connection
                    .execute_batch("DROP TRIGGER journal_no_update; UPDATE journal SET payload=json_set(payload,'$.data.links[0].foreign_status','active') WHERE command_id='foreign-tamper'; CREATE TRIGGER journal_no_update BEFORE UPDATE ON journal BEGIN SELECT RAISE(ABORT, 'append-only journal'); END;")
                    .unwrap();
            }
        }
        drop(store);
        assert!(
            matches!(Store::open(temp.database()), Err(StoreError::Integrity(_))),
            "{tamper} must be refused on reopen"
        );
    }
}

#[test]
fn a_row_edited_while_the_store_is_open_is_refused_by_the_next_read() {
    // The reopen audit catches a record that disagrees with its journal, but a
    // running store is holding its own connection: a row edited underneath it
    // is re-checked on the read that would serve it, not at the next restart.
    let temp = Temporary::new();
    let mut store = Store::open(temp.database()).unwrap();
    let (project, task) = register_task(&mut store, "live");
    set_links(
        &mut store,
        "foreign-live",
        &project,
        &task,
        &[session_link("codex-session-01")],
        11,
    )
    .unwrap();
    let mut blank = session_link("codex-session-01");
    blank.foreign_id = "   ".into();
    store
        .connection
        .execute(
            "UPDATE task_foreign_links SET foreign_id=?1, body=?2 WHERE project_id=?3",
            params![
                blank.foreign_id,
                serde_json::to_string(&blank).unwrap(),
                project.as_str()
            ],
        )
        .unwrap();
    assert!(
        matches!(
            store.task_foreign_links(&project, &task),
            Err(StoreError::Domain(DomainError::InvalidForeignReference))
        ),
        "an edited row is refused by the read that would serve it"
    );
}
