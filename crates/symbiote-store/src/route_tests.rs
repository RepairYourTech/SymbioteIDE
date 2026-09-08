use super::*;
use symbiote_workforce::*;

fn staffed(store: &mut Store) -> TeamConfiguration {
    let team = team_fixture(store);
    store
        .replace_team(
            id!(CommandId, "team-config"),
            None,
            team.clone(),
            id!(UserId, "owner"),
            Timestamp(10),
        )
        .unwrap();
    team
}

fn work_spec(team: &TeamConfiguration) -> WorkSpec {
    WorkSpec {
        id: WorkId::Request(id!(RequestId, "route-request")),
        project_id: team.project_id.clone(),
        role_id: team.lead_role_id.clone(),
        title: "Route fixture".into(),
        description: "Fixture work item for routing".into(),
        utterance: Some(RequestUtterance::RequestedWork),
        objective_class: None,
        parent: None,
        dependencies: BTreeSet::new(),
        requirements: vec!["Implement the fixture".into()],
        constraints: vec![],
        risks: vec![],
        acceptance: vec!["Fixture verified".into()],
        priority: 1,
        budget: None,
        external_references: vec![],
    }
}

fn create_work(store: &mut Store, team: &TeamConfiguration) -> WorkItem {
    let item = WorkItem::new(work_spec(team), id!(UserId, "owner"), Timestamp(10)).unwrap();
    store
        .create_work_item(id!(CommandId, "work-route"), item.clone())
        .unwrap();
    item
}

fn classify(team: &TeamConfiguration) -> RouteDecision {
    let request = RouteRequest {
        project_id: team.project_id.clone(),
        work_id: WorkId::Request(id!(RequestId, "route-request")),
        requested: None,
        domains: BTreeSet::from(["implementation".into()]),
    };
    resolve_route(team, &request).unwrap()
}

fn explicit(team: &TeamConfiguration, role: &RoleId) -> RouteDecision {
    let request = RouteRequest {
        project_id: team.project_id.clone(),
        work_id: WorkId::Request(id!(RequestId, "route-request")),
        requested: Some(role.clone()),
        domains: BTreeSet::new(),
    };
    resolve_route(team, &request).unwrap()
}

#[test]
fn routes_persist_replay_and_reroute_with_journal_provenance() {
    let temp = Temporary::new();
    let mut store = Store::open(temp.database()).unwrap();
    let team = staffed(&mut store);
    create_work(&mut store, &team);
    let decision = classify(&team);
    assert_eq!(
        decision.resolved.as_ref().map(|r| r.as_str()),
        Some("team-worker")
    );
    let first = store
        .record_route(
            id!(CommandId, "route-1"),
            decision.clone(),
            id!(UserId, "owner"),
            Timestamp(11),
        )
        .unwrap();
    assert!(!first.replayed);
    assert_eq!(
        store
            .route_command_timestamp(&id!(CommandId, "route-1"))
            .unwrap(),
        Some(Timestamp(11))
    );
    let replay = store
        .record_route(
            id!(CommandId, "route-1"),
            decision.clone(),
            id!(UserId, "owner"),
            Timestamp(11),
        )
        .unwrap();
    assert!(replay.replayed);
    assert_eq!(replay.sequence, first.sequence);
    assert_eq!(
        store
            .get_route(
                &team.project_id,
                &WorkId::Request(id!(RequestId, "route-request"))
            )
            .unwrap(),
        decision
    );
    // Reassignment supersedes while the journal keeps the earlier decision,
    // and never rewinds the recorded decision time.
    let reassigned = explicit(&team, &team.lead_role_id);
    assert!(matches!(
        store.record_route(
            id!(CommandId, "route-early"),
            reassigned.clone(),
            id!(UserId, "owner"),
            Timestamp(10),
        ),
        Err(StoreError::InvalidRoute)
    ));
    store
        .record_route(
            id!(CommandId, "route-2"),
            reassigned.clone(),
            id!(UserId, "owner"),
            Timestamp(12),
        )
        .unwrap();
    assert_eq!(
        store
            .get_route(
                &team.project_id,
                &WorkId::Request(id!(RequestId, "route-request"))
            )
            .unwrap(),
        reassigned
    );
    drop(store);
    let mut store = Store::open(temp.database()).unwrap();
    assert_eq!(
        store
            .get_route(
                &team.project_id,
                &WorkId::Request(id!(RequestId, "route-request"))
            )
            .unwrap(),
        reassigned
    );
    // A replayed earlier decision must not resurrect superseded state.
    store
        .record_route(
            id!(CommandId, "route-1"),
            decision,
            id!(UserId, "owner"),
            Timestamp(11),
        )
        .unwrap();
    assert_eq!(
        store
            .get_route(
                &team.project_id,
                &WorkId::Request(id!(RequestId, "route-request"))
            )
            .unwrap(),
        reassigned
    );
}

#[test]
fn tampered_route_rows_are_refused_on_reopen() {
    let temp = Temporary::new();
    let mut store = Store::open(temp.database()).unwrap();
    let team = staffed(&mut store);
    create_work(&mut store, &team);
    store
        .record_route(
            id!(CommandId, "route-tamper"),
            classify(&team),
            id!(UserId, "owner"),
            Timestamp(11),
        )
        .unwrap();
    drop(store);
    // Flip the indexed resolved flag without touching the journal; the body
    // still says resolved=team-worker, so reopen must refuse the database.
    let connection = Connection::open(temp.database()).unwrap();
    connection
        .execute(
            "UPDATE work_routes SET resolved=1-resolved WHERE work_key='request:route-request'",
            [],
        )
        .unwrap();
    drop(connection);
    assert!(matches!(
        Store::open(temp.database()),
        Err(StoreError::Integrity(_))
    ));
}

#[test]
fn no_route_diagnoses_are_recorded_like_resolutions() {
    let mut store = Store::memory().unwrap();
    let team = staffed(&mut store);
    create_work(&mut store, &team);
    let request = RouteRequest {
        project_id: team.project_id.clone(),
        work_id: WorkId::Request(id!(RequestId, "route-request")),
        requested: None,
        domains: BTreeSet::from(["frontend".into()]),
    };
    let decision = resolve_route(&team, &request).unwrap();
    assert_eq!(decision.resolved, None);
    assert!(matches!(
        decision.diagnosis,
        Some(RouteDiagnosis::NoExecutableRole { .. })
    ));
    store
        .record_route(
            id!(CommandId, "route-none"),
            decision.clone(),
            id!(UserId, "owner"),
            Timestamp(11),
        )
        .unwrap();
    assert_eq!(
        store
            .get_route(
                &team.project_id,
                &WorkId::Request(id!(RequestId, "route-request"))
            )
            .unwrap(),
        decision
    );
}

#[test]
fn route_records_reject_unknown_work_and_invalid_members() {
    let mut store = Store::memory().unwrap();
    let team = staffed(&mut store);
    create_work(&mut store, &team);
    let decision = classify(&team);
    // Unknown work item.
    let mut missing = decision.clone();
    missing.work_id = WorkId::Request(id!(RequestId, "missing"));
    assert!(matches!(
        store.record_route(
            id!(CommandId, "route-missing"),
            missing,
            id!(UserId, "owner"),
            Timestamp(11),
        ),
        Err(StoreError::RelationshipMismatch)
    ));
    // Project mismatch against the work item's stored project.
    let mut foreign = decision.clone();
    foreign.project_id = id!(ProjectId, "elsewhere");
    assert!(matches!(
        store.record_route(
            id!(CommandId, "route-foreign"),
            foreign,
            id!(UserId, "owner"),
            Timestamp(11),
        ),
        Err(StoreError::RelationshipMismatch)
    ));
    // Resolved Role outside the current Team.
    let mut ghost = decision.clone();
    ghost.resolved = Some(id!(RoleId, "ghost-role"));
    ghost.diagnosis = None;
    assert!(matches!(
        store.record_route(
            id!(CommandId, "route-ghost"),
            ghost,
            id!(UserId, "owner"),
            Timestamp(11),
        ),
        Err(StoreError::InvalidRoute)
    ));
    // Structurally invalid decision (resolution and diagnosis together).
    let mut broken = decision.clone();
    broken.diagnosis = Some(RouteDiagnosis::NoExecutableRole {
        domains: BTreeSet::from(["implementation".into()]),
    });
    assert!(matches!(
        store.record_route(
            id!(CommandId, "route-broken"),
            broken,
            id!(UserId, "owner"),
            Timestamp(11),
        ),
        Err(StoreError::InvalidRoute)
    ));
    assert_eq!(
        store
            .route_command_timestamp(&id!(CommandId, "route-never"))
            .unwrap(),
        None
    );
}
