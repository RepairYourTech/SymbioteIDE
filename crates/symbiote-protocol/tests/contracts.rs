use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};
use symbiote_contract_read::{bytes, region};
use symbiote_domain::*;
// `symbiote_domain` names a canonical `Request` record of its own (#36); this
// test means the protocol's wire request.
use symbiote_protocol::Request;
use symbiote_protocol::*;

fn project_id() -> ProjectId {
    ProjectId::new("project-a").unwrap()
}
fn principal() -> Principal {
    Principal::restricted(
        UserId::new("user-a").unwrap(),
        BTreeMap::from([(
            project_id(),
            BTreeSet::from([
                ProjectPermission::Register,
                ProjectPermission::Read,
                ProjectPermission::CreateTask,
                ProjectPermission::ReadJournal,
            ]),
        )]),
    )
}

#[test]
fn team_management_requires_its_own_project_grant_and_rejects_authority_injection() {
    let team: TeamConfiguration =
        serde_json::from_str(include_str!("../../../fixtures/project-team/team.json")).unwrap();
    let req = request(Operation::ReplaceTeam {
        expected_revision: None,
        team: Box::new(team.clone()),
    });
    let grants = |permissions| {
        Principal::restricted(
            UserId::new("staff-admin").unwrap(),
            BTreeMap::from([(team.project_id.clone(), permissions)]),
        )
    };
    let ordinary = grants(BTreeSet::from([
        ProjectPermission::Read,
        ProjectPermission::Register,
        ProjectPermission::ManageWork,
        ProjectPermission::CreateTask,
    ]));
    assert_eq!(
        authorize(&ordinary, &req).unwrap_err().code,
        ErrorCode::PermissionDenied
    );
    assert!(
        authorize(
            &ordinary,
            &request(Operation::GetTeam {
                project_id: team.project_id.clone()
            })
        )
        .is_ok()
    );
    let admin = grants(BTreeSet::from([ProjectPermission::ManageTeam]));
    assert!(authorize(&admin, &req).is_ok());
    let mut spoofed = serde_json::to_value(&req).unwrap();
    spoofed["operation"]["actor"] = json!("host");
    assert!(parse_request(&serde_json::to_vec(&spoofed).unwrap()).is_err());
    let mut foreign = req.clone();
    if let Operation::ReplaceTeam { team, .. } = &mut foreign.operation {
        team.project_id = ProjectId::new("foreign").unwrap();
        team.access_ceiling.project_id = team.project_id.clone();
        for member in &mut team.members {
            member.access.project_id = team.project_id.clone();
        }
    }
    assert_eq!(
        authorize(&admin, &foreign).unwrap_err().code,
        ErrorCode::PermissionDenied
    );
}

#[test]
fn project_grants_do_not_authorize_resource_consent_or_revocation() {
    let snapshot = serde_json::from_value(json!({"project_id":"project-a","role_id":"lead","profile_id":"profile",
        "host_id":"host","resource_ref":"tool","fingerprint":"a".repeat(64),
        "access":{"project_id":"project-a","roots":["root-a"],"grants":["read_root"],"policy_revision":1}})).unwrap();
    for operation in [
        Operation::RecordResourceConsent {
            snapshot,
            expires_at: Timestamp(100),
        },
        Operation::RevokeResourceConsent {
            project_id: project_id(),
            consent_id: CommandId::new("consent").unwrap(),
        },
    ] {
        let request = request(operation);
        assert_eq!(
            authorize(&principal(), &request).unwrap_err().code,
            ErrorCode::PermissionDenied
        );
        assert!(
            authorize(
                &Principal::local_owner(UserId::new("owner").unwrap()),
                &request
            )
            .is_ok()
        );
    }
}
fn request(operation: Operation) -> Request {
    Request {
        version: CURRENT_VERSION,
        correlation_id: RequestId::new("request-a").unwrap(),
        command_id: CommandId::new("command-a").unwrap(),
        operation,
    }
}

#[test]
fn binding_management_is_separate_from_team_work_and_read_permissions() {
    let config: symbiote_workforce::BindingConfiguration = serde_json::from_str(include_str!(
        "../../../fixtures/workforce-bindings/binding.json"
    ))
    .unwrap();
    let req = request(Operation::ReplaceBinding {
        expected_revision: None,
        configuration: Box::new(config.clone()),
    });
    let project = config.binding.project_id;
    let permissions = |set| {
        Principal::restricted(
            UserId::new("staff-admin").unwrap(),
            BTreeMap::from([(project.clone(), set)]),
        )
    };
    let reader = permissions(BTreeSet::from([
        ProjectPermission::ManageTeam,
        ProjectPermission::ManageWork,
        ProjectPermission::Read,
    ]));
    assert_eq!(
        authorize(&reader, &req).unwrap_err().code,
        ErrorCode::PermissionDenied
    );
    let admin = permissions(BTreeSet::from([ProjectPermission::ManageBindings]));
    assert!(authorize(&admin, &req).is_ok());
    let readiness = request(Operation::GetBindingReadiness {
        project_id: project.clone(),
        binding_id: config.binding.id.clone(),
    });
    assert_eq!(
        authorize(&reader, &readiness).unwrap_err().code,
        ErrorCode::PermissionDenied
    );
    assert!(
        authorize(
            &Principal::local_owner(UserId::new("owner").unwrap()),
            &readiness
        )
        .is_ok()
    );
    assert!(
        authorize(
            &reader,
            &request(Operation::GetBinding {
                project_id: project,
                binding_id: config.binding.id
            })
        )
        .is_ok()
    );
    let mut spoofed = serde_json::to_value(req).unwrap();
    spoofed["operation"]["actor"] = json!("host");
    assert!(parse_request(&serde_json::to_vec(&spoofed).unwrap()).is_err());
}

#[test]
fn host_pulse_requires_host_owner_even_with_project_permissions() {
    let req = request(Operation::GetHostPulse {});
    assert_eq!(
        authorize(&principal(), &req).unwrap_err().code,
        ErrorCode::PermissionDenied
    );
    assert!(authorize(&Principal::local_owner(UserId::new("owner").unwrap()), &req).is_ok());
    let mut spoof = serde_json::to_value(req).unwrap();
    spoof["operation"]["host_id"] = json!("another-host");
    assert!(parse_request(&serde_json::to_vec(&spoof).unwrap()).is_err());
}
fn project_draft() -> ProjectDraft {
    ProjectDraft {
        id: project_id(),
        name: "Project A".into(),
        lead: RoleId::new("lead").unwrap(),
        roots: vec![Root {
            id: RootId::new("root-a").unwrap(),
            project_id: project_id(),
            revision: Revision(0),
            repository: None,
            host_paths: BTreeMap::new(),
        }],
        roles: vec![Role {
            id: RoleId::new("lead").unwrap(),
            project_id: project_id(),
            revision: Revision(0),
            name: "Lead".into(),
            operating_contract: VersionedRoleContract {
                id: RoleContractId::new("role-contract").unwrap(),
                revision: Revision(1),
            },
        }],
    }
}
fn task_draft() -> TaskDraft {
    TaskDraft {
        origin: TaskOrigin::Objective(WorkRef {
            project_id: project_id(),
            id: WorkId::Objective(ObjectiveId::new("maintenance").unwrap()),
        }),
        id: TaskId::new("task-a").unwrap(),
        project_id: project_id(),
        root_id: RootId::new("root-a").unwrap(),
        role_id: RoleId::new("lead").unwrap(),
        task_contract: VersionedTaskContract {
            id: TaskContractId::new("task-contract").unwrap(),
            revision: Revision(1),
        },
        stream: InitialStream {
            id: ChangeStreamId::new("stream-a").unwrap(),
            originating_chat: ChatId::new("chat-a").unwrap(),
            worktree: WorktreeId::new("worktree-a").unwrap(),
            branch: "issue-181".into(),
            base: CommitSha::new("a".repeat(40)).unwrap(),
            target: CommitSha::new("b".repeat(40)).unwrap(),
        },
    }
}

fn work_spec() -> WorkSpec {
    WorkSpec {
        id: WorkId::Objective(ObjectiveId::new("maintenance").unwrap()),
        project_id: project_id(),
        role_id: RoleId::new("lead").unwrap(),
        title: "Maintenance".into(),
        description: "Explicit purpose".into(),
        utterance: None,
        objective_class: Some(ObjectiveClass::Maintenance),
        parent: None,
        dependencies: BTreeSet::new(),
        requirements: vec![],
        constraints: vec![],
        risks: vec![],
        acceptance: vec!["verified result".into()],
        priority: 2,
        budget: None,
        external_references: vec![],
    }
}

#[test]
fn work_wire_never_accepts_host_completion_or_claimed_authority() {
    for edit in [
        json!({"kind":"complete","evidence":{}}),
        json!({"kind":"approve","actor":{"host":"forged"}}),
    ] {
        let bytes = serde_json::to_vec(&json!({"version":CURRENT_VERSION,"correlation_id":"request","command_id":"command",
            "operation":{"kind":"change_work","project_id":"project-a","id":{"kind":"objective","id":"maintenance"},"expected_revision":0,"edit":edit}})).unwrap();
        assert!(parse_request(&bytes).is_err());
    }
    let mut value = serde_json::to_value(task_draft()).unwrap();
    value.as_object_mut().unwrap().remove("origin");
    assert!(serde_json::from_value::<TaskDraft>(value).is_err());
}

#[test]
fn work_references_require_read_access_even_after_reference_is_removed() {
    let foreign = ProjectId::new("foreign").unwrap();
    let user = UserId::new("user-a").unwrap();
    let restricted = Principal::restricted(
        user.clone(),
        BTreeMap::from([(
            project_id(),
            BTreeSet::from([ProjectPermission::ManageWork, ProjectPermission::Read]),
        )]),
    );
    let mut spec = work_spec();
    spec.dependencies.insert(WorkRef {
        project_id: foreign.clone(),
        id: WorkId::Objective(ObjectiveId::new("foreign-objective").unwrap()),
    });
    assert_eq!(
        authorize_work_spec(&restricted, &spec, ProjectPermission::ManageWork)
            .unwrap_err()
            .code,
        ErrorCode::PermissionDenied
    );
    let linked = Principal::restricted(
        user.clone(),
        BTreeMap::from([
            (
                project_id(),
                BTreeSet::from([ProjectPermission::ManageWork, ProjectPermission::Read]),
            ),
            (foreign, BTreeSet::from([ProjectPermission::Read])),
        ]),
    );
    authorize_work_spec(&linked, &spec, ProjectPermission::ManageWork).unwrap();
    let mut item = WorkItem::new(spec.clone(), user.clone(), Timestamp(1)).unwrap();
    spec.dependencies.clear();
    item.apply(WorkCommand {
        id: CommandId::new("revise").unwrap(),
        expected_revision: Revision(0),
        actor: Actor::User(user),
        at: Timestamp(2),
        action: WorkAction::Revise {
            spec: Box::new(spec),
        },
    })
    .unwrap();
    authorize_work_resource(&linked, &project_id(), &item).unwrap();
    assert_eq!(
        authorize_work_resource(&restricted, &project_id(), &item)
            .unwrap_err()
            .code,
        ErrorCode::PermissionDenied
    );
}

#[test]
fn golden_request_and_response_remain_stable() {
    let fixture = r#"{"version":{"major":1,"minor":21},"correlation_id":"request-a","command_id":"command-a","operation":{"kind":"get_project","project_id":"project-a"}}"#;
    let parsed = parse_request(fixture.as_bytes()).unwrap();
    assert_eq!(serde_json::to_string(&parsed).unwrap(), fixture);
    let error = Response::failure(
        Some(parsed.correlation_id),
        ProtocolError::new(ErrorCode::NotFound),
    );
    assert_eq!(
        serde_json::to_value(error).unwrap(),
        json!({"version":{"major":1,"minor": 21},"correlation_id":"request-a","result":{"Err":{"code":"not_found","message":"resource not found"}}})
    );
}

#[test]
fn rejects_spoofed_identity_at_every_request_boundary() {
    let original = serde_json::to_value(request(Operation::RegisterProject {
        project: project_draft(),
    }))
    .unwrap();
    for key in ["principal", "actor", "owner", "host_id", "permissions"] {
        let mut wire = original.clone();
        wire[key] = json!({"kind":"host","id":"evil"});
        assert!(parse_request(wire.to_string().as_bytes()).is_err());
        let mut wire = original.clone();
        wire["operation"][key] = json!("evil");
        assert!(parse_request(wire.to_string().as_bytes()).is_err());
        let mut wire = original.clone();
        wire["operation"]["project"][key] = json!("evil");
        assert!(parse_request(wire.to_string().as_bytes()).is_err());
    }
    let (project, _, _) = project_draft()
        .into_records(&principal(), Timestamp(42))
        .unwrap();
    assert_eq!(project.owner, UserId::new("user-a").unwrap());
    assert_eq!(
        project.provenance.actor,
        Actor::User(UserId::new("user-a").unwrap())
    );
}

#[test]
fn task_creation_cannot_import_history_evidence_or_host_authority() {
    let base = serde_json::to_value(request(Operation::CreateTask { task: task_draft() })).unwrap();
    for key in [
        "actor", "state", "revision", "history", "dispatch", "evidence",
    ] {
        let mut wire = base.clone();
        wire["operation"]["task"][key] = json!([]);
        assert!(parse_request(wire.to_string().as_bytes()).is_err());
    }
    for kind in [
        "apply_task",
        "complete_task",
        "append_event",
        "request_completion",
    ] {
        let mut wire = base.clone();
        wire["operation"] = json!({"kind":kind});
        assert!(parse_request(wire.to_string().as_bytes()).is_err());
    }
    let (task, stream) = task_draft().into_records().unwrap();
    assert_eq!(task.state(), &TaskState::Ready);
    assert!(task.history().is_empty());
    assert_eq!(stream.state(), &StreamState::Active);
    assert!(stream.history().is_empty());
}

#[test]
fn bounds_and_malformed_frames_fail_without_reflecting_input() {
    assert_eq!(
        parse_request(&vec![b' '; MAX_REQUEST_BYTES + 1])
            .unwrap_err()
            .code,
        ErrorCode::RequestTooLarge
    );
    let valid = serde_json::to_vec(&request(Operation::Health {})).unwrap();
    let mut exact = valid.clone();
    exact.resize(MAX_REQUEST_BYTES, b' ');
    assert!(parse_request(&exact).is_ok());
    for bad in [
        b"".as_slice(),
        b"null",
        b"{}{}",
        b"{\"secret-value\":",
        &[0xff],
    ] {
        let error = parse_request(bad).unwrap_err();
        assert_eq!(error.code, ErrorCode::InvalidRequest);
        assert!(!error.message.contains("secret-value"));
    }
    let duplicate = String::from_utf8(valid).unwrap().replace(
        "\"kind\":\"health\"",
        "\"kind\":\"health\",\"kind\":\"shutdown\"",
    );
    assert!(parse_request(duplicate.as_bytes()).is_err());
}

#[test]
fn versions_negotiate_only_explicitly_supported_versions() {
    assert_eq!(
        negotiate(&[ProtocolVersion { major: 2, minor: 0 }, CURRENT_VERSION])
            .unwrap()
            .version,
        CURRENT_VERSION
    );
    for version in [
        ProtocolVersion { major: 0, minor: 9 },
        ProtocolVersion { major: 1, minor: 0 },
        ProtocolVersion { major: 1, minor: 1 },
        ProtocolVersion { major: 1, minor: 2 },
        ProtocolVersion { major: 1, minor: 3 },
        ProtocolVersion { major: 1, minor: 4 },
        ProtocolVersion { major: 1, minor: 5 },
        ProtocolVersion { major: 1, minor: 6 },
        ProtocolVersion { major: 1, minor: 7 },
        ProtocolVersion { major: 1, minor: 8 },
        ProtocolVersion { major: 1, minor: 9 },
        ProtocolVersion {
            major: 1,
            minor: 10,
        },
        ProtocolVersion {
            major: 1,
            minor: 11,
        },
        ProtocolVersion {
            major: 1,
            minor: 12,
        },
        ProtocolVersion {
            major: 1,
            minor: 13,
        },
        ProtocolVersion {
            major: 1,
            minor: 14,
        },
        ProtocolVersion {
            major: 1,
            minor: 15,
        },
        ProtocolVersion {
            major: 1,
            minor: 16,
        },
        ProtocolVersion {
            major: 1,
            minor: 17,
        },
        ProtocolVersion {
            major: 1,
            minor: 18,
        },
        ProtocolVersion {
            major: 1,
            minor: 19,
        },
        ProtocolVersion {
            major: 1,
            minor: 20,
        },
        ProtocolVersion { major: 2, minor: 0 },
    ] {
        assert_eq!(
            negotiate(&[version]).unwrap_err().code,
            ErrorCode::UnsupportedVersion
        );
        let mut req = request(Operation::Health {});
        req.version = version;
        assert_eq!(
            parse_request(&serde_json::to_vec(&req).unwrap())
                .unwrap_err()
                .code,
            ErrorCode::UnsupportedVersion
        );
    }
    assert!(negotiate(&[]).is_err());
    assert!(negotiate(&[CURRENT_VERSION; 17]).is_err());
}

#[test]
fn authorization_is_per_operation_project_and_loaded_resource() {
    let principal = principal();
    assert!(
        authorize(
            &principal,
            &request(Operation::GetProject {
                project_id: project_id()
            })
        )
        .is_ok()
    );
    let other = ProjectId::new("project-b").unwrap();
    assert_eq!(
        authorize(
            &principal,
            &request(Operation::GetProject {
                project_id: other.clone()
            })
        )
        .unwrap_err()
        .code,
        ErrorCode::PermissionDenied
    );
    let (mut_draft, _) = task_draft().into_records().unwrap();
    assert!(authorize_task_resource(&principal, &other, &mut_draft).is_err());
    let reader = Principal::restricted(
        UserId::new("reader").unwrap(),
        [(project_id(), [ProjectPermission::Read].into())].into(),
    );
    assert!(
        authorize(
            &reader,
            &request(Operation::CreateTask { task: task_draft() })
        )
        .is_err()
    );
    assert!(
        authorize(
            &reader,
            &request(Operation::ReadJournal {
                project_id: project_id(),
                after: JournalCursor(0),
                limit: 1
            })
        )
        .is_err()
    );
    assert!(authorize(&reader, &request(Operation::Shutdown {})).is_err());
    assert!(
        authorize(
            &Principal::local_owner(UserId::new("local").unwrap()),
            &request(Operation::Shutdown {})
        )
        .is_ok()
    );
}

#[test]
fn registration_rejects_cross_project_duplicates_machine_state_and_stale_revisions() {
    let mut draft = project_draft();
    draft.roots[0].project_id = ProjectId::new("other").unwrap();
    assert!(draft.validate().is_err());
    let mut draft = project_draft();
    draft.roots.push(draft.roots[0].clone());
    assert!(draft.validate().is_err());
    let mut draft = project_draft();
    draft.roles.push(draft.roles[0].clone());
    assert!(draft.validate().is_err());
    let mut draft = project_draft();
    draft.roles[0].revision = Revision(1);
    assert!(draft.validate().is_err());
    let mut draft = project_draft();
    draft.roots[0]
        .host_paths
        .insert(HostId::new("host").unwrap(), "/private".into());
    assert!(draft.validate().is_err());
    let mut draft = project_draft();
    draft.lead = RoleId::new("missing").unwrap();
    assert!(draft.validate().is_err());
}

#[test]
fn pagination_bounds_sequence_and_project_lineage() {
    for limit in [0, MAX_PAGE_SIZE + 1] {
        assert!(
            parse_request(
                &serde_json::to_vec(&request(Operation::ReadJournal {
                    project_id: project_id(),
                    after: JournalCursor(0),
                    limit
                }))
                .unwrap()
            )
            .is_err()
        );
    }
    let (project, roots, roles) = project_draft()
        .into_records(&principal(), Timestamp(1))
        .unwrap();
    let event = JournalEvent {
        sequence: 3,
        project_id: project_id(),
        command_id: CommandId::new("event-command").unwrap(),
        revision: Revision(0),
        payload: EventPayload::ProjectRegistered {
            project,
            roots,
            roles,
        },
    };
    let page = JournalPage {
        events: vec![event],
        next_cursor: JournalCursor(3),
        has_more: false,
    };
    assert!(page.validate(&project_id(), JournalCursor(0), 10).is_ok());
    assert!(page.validate(&project_id(), JournalCursor(3), 10).is_err());
    assert!(
        page.validate(&ProjectId::new("other").unwrap(), JournalCursor(0), 10)
            .is_err()
    );
    let mut wrong = page.clone();
    wrong.next_cursor = JournalCursor(4);
    assert!(wrong.validate(&project_id(), JournalCursor(0), 10).is_err());
    let empty = JournalPage {
        events: vec![],
        next_cursor: JournalCursor(3),
        has_more: false,
    };
    assert!(empty.validate(&project_id(), JournalCursor(3), 10).is_ok());
}

#[test]
fn root_placement_observation_needs_registration_grant_and_absolute_paths() {
    let operation = |path: &str| Operation::ObserveRootPlacement {
        project_id: project_id(),
        root_id: RootId::new("root-a").unwrap(),
        host_id: HostId::new("host-a").unwrap(),
        path: path.into(),
        expected_revision: Revision(0),
    };
    // Only absolute, bounded, control-free paths parse.
    assert!(
        parse_request(&serde_json::to_vec(&request(operation("/repos/demo"))).unwrap()).is_ok()
    );
    for path in ["", " ", "relative/path", "/no\0nul"] {
        assert!(parse_request(&serde_json::to_vec(&request(operation(path))).unwrap()).is_err());
    }
    // Registration authority: a read-only principal cannot record a
    // placement, and the Host separately verifies the host identity and
    // repository before storing anything.
    let reader = Principal::restricted(
        UserId::new("reader").unwrap(),
        BTreeMap::from([(project_id(), BTreeSet::from([ProjectPermission::Read]))]),
    );
    let placed = request(operation("/repos/demo"));
    assert_eq!(
        authorize(&reader, &placed).unwrap_err().code,
        ErrorCode::PermissionDenied
    );
    assert!(authorize(&principal(), &placed).is_ok());
    assert!(matches!(
        placed.operation,
        Operation::ObserveRootPlacement { .. }
    ));
    assert!(placed.operation.is_mutation());
    assert_eq!(placed.operation.project_id(), Some(&project_id()));
}

#[test]
fn placement_events_replay_lineage_with_exact_revision_transitions() {
    let host = HostId::new("host-a").unwrap();
    let actor = UserId::new("owner").unwrap();
    let mut root = project_draft().roots.remove(0);
    root.revision = Revision(1);
    root.host_paths
        .insert(host.clone(), "/repos/demo".to_string());
    let event = |root: &Root| JournalEvent {
        sequence: 5,
        project_id: project_id(),
        command_id: CommandId::new("placement-command").unwrap(),
        revision: Revision(1),
        payload: EventPayload::RootPlacementObserved {
            root: Box::new(root.clone()),
            expected_revision: Revision(0),
            host_id: host.clone(),
            path: "/repos/demo".into(),
            actor: actor.clone(),
            at: Timestamp(20),
        },
    };
    let page = JournalPage {
        events: vec![event(&root)],
        next_cursor: JournalCursor(5),
        has_more: false,
    };
    assert!(page.validate(&project_id(), JournalCursor(0), 10).is_ok());
    // A revision jump that does not match expected+1 is corrupt lineage.
    let mut stale = event(&root);
    stale.payload = match stale.payload {
        EventPayload::RootPlacementObserved {
            root,
            expected_revision: _,
            host_id,
            path,
            actor,
            at,
        } => EventPayload::RootPlacementObserved {
            root,
            expected_revision: Revision(3),
            host_id,
            path,
            actor,
            at,
        },
        other => other,
    };
    let page = JournalPage {
        events: vec![stale],
        next_cursor: JournalCursor(5),
        has_more: false,
    };
    assert!(page.validate(&project_id(), JournalCursor(0), 10).is_err());
    // The recorded path must be the entry actually present on the Root.
    let mut absent = event(&root);
    absent.payload = match absent.payload {
        EventPayload::RootPlacementObserved {
            root,
            expected_revision,
            host_id,
            actor,
            at,
            ..
        } => EventPayload::RootPlacementObserved {
            root,
            expected_revision,
            host_id,
            path: "/repos/elsewhere".into(),
            actor,
            at,
        },
        other => other,
    };
    let page = JournalPage {
        events: vec![absent],
        next_cursor: JournalCursor(5),
        has_more: false,
    };
    assert!(page.validate(&project_id(), JournalCursor(0), 10).is_err());
}

#[test]
fn telemetry_is_not_an_accepted_domain_command() {
    let telemetry = Telemetry::QueueDepth {
        project_id: project_id(),
        pending: 10,
    };
    assert!(parse_request(&serde_json::to_vec(&telemetry).unwrap()).is_err());
    let mut wire = serde_json::to_value(request(Operation::Health {})).unwrap();
    wire["operation"] = serde_json::to_value(telemetry).unwrap();
    assert!(parse_request(wire.to_string().as_bytes()).is_err());
}

#[test]
fn response_serialization_is_bounded_before_allocating_full_payload() {
    let ordinary = Response::failure(None, ProtocolError::new(ErrorCode::NotFound));
    assert_eq!(
        encode_response(&ordinary).unwrap(),
        serde_json::to_vec(&ordinary).unwrap()
    );
    let mut oversized = ordinary;
    oversized.result = Err(ProtocolError {
        code: ErrorCode::Internal,
        message: "x".repeat(MAX_RESPONSE_BYTES),
    });
    assert_eq!(
        encode_response(&oversized).unwrap_err().code,
        ErrorCode::ResourceExhausted
    );
}

#[test]
fn provider_registry_writes_and_reads_stay_owner_authority_only() {
    let connection = symbiote_domain::ProviderConnection {
        id: ProviderConnectionId::new("provider-a").unwrap(),
        adapter: InferenceProviderAdapterId::new("adapter").unwrap(),
        endpoint_reference: "https://provider.example/v1".into(),
        authentication: symbiote_domain::AuthenticationKind::ApiCredential,
    };
    let entitlement = symbiote_domain::BillingEntitlement {
        id: BillingEntitlementId::new("entitlement-a").unwrap(),
        provider: ProviderConnectionId::new("provider-a").unwrap(),
        kind: symbiote_domain::BillingKind::MeteredApi,
        verification_evidence: EvidenceId::new("evidence").unwrap(),
        expires_at: Timestamp(1_000),
    };
    let descriptor = symbiote_runtime_sdk::provider::ModelDescriptor {
        schema_version: symbiote_runtime_sdk::provider::PROVIDER_CONTRACT_VERSION,
        id: ModelId::new("model-a").unwrap(),
        provider_id: ProviderConnectionId::new("provider-a").unwrap(),
        context_window_tokens: 1_000,
        max_output_tokens: 500,
        capabilities: symbiote_runtime_sdk::provider::ModelCapabilities {
            reasoning_efforts: BTreeSet::new(),
            tools: false,
            images: false,
            streaming: false,
        },
    };
    for operation in [
        Operation::ReplaceProviderConnection {
            attribution: project_id(),
            connection,
        },
        Operation::ReplaceBillingEntitlement {
            attribution: project_id(),
            entitlement: Box::new(entitlement),
        },
        Operation::ReplaceModelDescriptor {
            attribution: project_id(),
            descriptor: Box::new(descriptor),
        },
        Operation::GetProviderConnection {
            provider_id: ProviderConnectionId::new("provider-a").unwrap(),
        },
        Operation::GetBillingEntitlement {
            entitlement_id: BillingEntitlementId::new("entitlement-a").unwrap(),
        },
        Operation::GetModelDescriptor {
            model_id: ModelId::new("model-a").unwrap(),
        },
    ] {
        let request = request(operation.clone());
        assert_eq!(
            authorize(&principal(), &request).unwrap_err().code,
            ErrorCode::PermissionDenied
        );
        assert!(
            authorize(
                &Principal::local_owner(UserId::new("owner").unwrap()),
                &request
            )
            .is_ok()
        );
    }
}

/// The `major.minor` a dotted figure states.
fn dotted(text: &str) -> (u16, u16) {
    let (major, minor) = text.trim().split_once('.').expect("a dotted version");
    (
        major.parse().expect("a major version"),
        minor.parse().expect("a minor version"),
    )
}

/// The version and bounds the protocol's documents state are the ones this crate speaks and
/// enforces: the version in `protocol.md` (its title, the envelope's exact version, the two
/// figures in the `hello` sentence, the fixture sentence) and in `host-inventory.md`, and the
/// three bounds in `protocol.md`, `host.md` and `client-sdk.md`, and the response bound
/// `work-hierarchy.md` restates, each read from the sentence it is written in. A document that
/// states a figure `CURRENT_VERSION` or the constants have moved past fails here by name rather
/// than in prose nobody reads.
///
/// What it does not read: the changelog's history (`v1.20`, `v1.5` …), which states what an
/// older revision did and stays true when the current one moves.
#[test]
fn the_documents_state_the_version_and_bounds_this_crate_enforces() {
    let protocol = include_str!("../../../docs/contracts/protocol.md");
    let host = include_str!("../../../docs/contracts/host.md");
    let inventory = include_str!("../../../docs/contracts/host-inventory.md");
    let sdk = include_str!("../../../docs/contracts/client-sdk.md");
    let hierarchy = include_str!("../../../docs/contracts/work-hierarchy.md");
    let current = (CURRENT_VERSION.major, CURRENT_VERSION.minor);

    for (label, stated) in [
        (
            "the protocol document's title",
            dotted(region(protocol, "# Client/Host protocol v", " foundation")),
        ),
        (
            "the protocol document's envelope version",
            (
                region(protocol, "exact `version: {major: ", ",")
                    .trim()
                    .parse()
                    .expect("a major version"),
                region(protocol, ", minor: ", "}`")
                    .trim()
                    .parse()
                    .expect("a minor version"),
            ),
        ),
        (
            "the protocol document's hello offer",
            dotted(region(
                protocol,
                "`hello` offers the explicit protocol version it supports: v",
                ". Only v",
            )),
        ),
        (
            "the protocol document's supported version",
            dotted(region(protocol, "Only v", " is supported")),
        ),
        (
            "the protocol document's fixture sentence",
            dotted(region(protocol, "Fixtures verify v", " and rejection")),
        ),
        (
            "the host inventory document",
            dotted(region(inventory, "for protocol v", ". This batch")),
        ),
    ] {
        assert_eq!(
            stated, current,
            "{label} states v{}.{}, and this crate speaks v{}.{}",
            stated.0, stated.1, current.0, current.1
        );
    }

    for (label, stated, enforced) in [
        (
            "the protocol document's request bound",
            bytes::<usize>(region(protocol, "Requests are limited to ", " bytes")),
            MAX_REQUEST_BYTES,
        ),
        (
            "the protocol document's response cap",
            bytes(region(
                protocol,
                "`encode_response` caps output at ",
                " before larger allocation",
            )),
            MAX_RESPONSE_BYTES,
        ),
        (
            "the protocol document's page bound",
            bytes(region(protocol, "`limit` in 1–", ".")),
            MAX_PAGE_SIZE as usize,
        ),
        (
            "the host document's frame bound",
            bytes(region(host, "Frames have a ", " request bound")),
            MAX_REQUEST_BYTES,
        ),
        (
            "the host document's response bound",
            bytes(region(host, "Responses have a ", " serialization bound")),
            MAX_RESPONSE_BYTES,
        ),
        (
            "the client SDK document's bound",
            bytes(region(sdk, "`RequestTooLarge` for the ", " bound")),
            MAX_REQUEST_BYTES,
        ),
        (
            "the work hierarchy document's response bound",
            bytes(region(hierarchy, "Server responses retain the", " bound")),
            MAX_RESPONSE_BYTES,
        ),
    ] {
        assert_eq!(
            stated, enforced,
            "{label} states {stated}, and this crate enforces {enforced}"
        );
    }
}

/// The kind names `protocol.md`'s response sentence writes are ones this crate's `ResponseBody`
/// declares: the enum's adjacently tagged variants are the response kinds, so a document naming a
/// kind the crate cannot produce — or a variant renamed or removed without the document — fails
/// here by name rather than in prose nobody reads. The tag rule the enum's own attribute states is
/// read too: if it stops being `snake_case`, the variant names are no longer the kinds, and the
/// case refuses rather than judging against a rule that no longer holds.
///
/// What it does not read: the sentence's own framing (`include`), which does not claim the list is
/// complete, so a kind the document does not name is not a failure here.
#[test]
fn the_contract_names_only_response_kinds_this_crate_declares() {
    let protocol = include_str!("../../../docs/contracts/protocol.md");
    let source = include_str!("../src/lib.rs");

    let header = &source[..source
        .find("pub enum ResponseBody {")
        .expect("the response body enum")];
    assert!(
        header[header.len().saturating_sub(300)..].contains("rename_all = \"snake_case\""),
        "the response kinds are the variant names only while the enum's tag rule is snake_case"
    );

    let body = region(source, "pub enum ResponseBody {", "\n}\n");
    let declared: Vec<String> = body
        .lines()
        .filter_map(|line| {
            let variant = line.trim_start();
            variant
                .starts_with(|c: char| c.is_ascii_uppercase())
                .then(|| {
                    variant
                        .chars()
                        .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                        .collect::<String>()
                })
        })
        .map(|variant| {
            variant.chars().fold(String::new(), |mut snake, c| {
                if c.is_ascii_uppercase() {
                    if !snake.is_empty() {
                        snake.push('_');
                    }
                    snake.push(c.to_ascii_lowercase());
                } else {
                    snake.push(c);
                }
                snake
            })
        })
        .collect();
    assert!(
        declared.len() > 1,
        "the response body must declare its variants, and this read found {declared:?}"
    );

    let sentence = region(protocol, "Successful response kinds include ", " alongside");
    let named: Vec<&str> = sentence
        .split('`')
        .skip(1)
        .step_by(2)
        .filter(|name| !name.is_empty())
        .collect();
    assert!(
        !named.is_empty(),
        "the response sentence must name the kind it adds, and it names none"
    );
    for kind in named {
        assert!(
            declared.iter().any(|declared| declared == kind),
            "the protocol document names {kind} as a response kind, and ResponseBody declares {declared:?}"
        );
    }
}
