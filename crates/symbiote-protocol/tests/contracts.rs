use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};
use symbiote_domain::*;
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
fn request(operation: Operation) -> Request {
    Request {
        version: CURRENT_VERSION,
        correlation_id: RequestId::new("request-a").unwrap(),
        command_id: CommandId::new("command-a").unwrap(),
        operation,
    }
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

#[test]
fn golden_request_and_response_remain_stable() {
    let fixture = r#"{"version":{"major":1,"minor":0},"correlation_id":"request-a","command_id":"command-a","operation":{"kind":"get_project","project_id":"project-a"}}"#;
    let parsed = parse_request(fixture.as_bytes()).unwrap();
    assert_eq!(serde_json::to_string(&parsed).unwrap(), fixture);
    let error = Response::failure(
        Some(parsed.correlation_id),
        ProtocolError::new(ErrorCode::NotFound),
    );
    assert_eq!(
        serde_json::to_value(error).unwrap(),
        json!({"version":{"major":1,"minor":0},"correlation_id":"request-a","result":{"Err":{"code":"not_found","message":"resource not found"}}})
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
        ProtocolVersion { major: 1, minor: 1 },
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
