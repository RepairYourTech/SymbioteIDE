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

/// The dossier read answers from the Host's own operator provisioning — which
/// pack it has installed a record for — so it is owner authority and no Project
/// grant, not even the one that manages this Project's bindings. The record it
/// serves is the pack's own document with the run the Host held named beside it,
/// so a client is never left making the Host's comparison for itself.
#[test]
fn the_compatibility_dossier_read_is_the_hosts_own_record_not_a_projects() {
    let operation = Operation::GetCompatibilityDossier {
        project_id: project_id(),
        binding_id: BindingId::new("binding").unwrap(),
    };
    let request = request(operation);
    assert_eq!(
        authorize(&principal(), &request).unwrap_err().code,
        ErrorCode::PermissionDenied
    );
    let binding_admin = Principal::restricted(
        UserId::new("staff-admin").unwrap(),
        BTreeMap::from([(
            project_id(),
            BTreeSet::from([ProjectPermission::ManageBindings]),
        )]),
    );
    assert_eq!(
        authorize(&binding_admin, &request).unwrap_err().code,
        ErrorCode::PermissionDenied,
        "authority over a Project's bindings is not authority over this Host's records"
    );
    assert!(
        authorize(
            &Principal::local_owner(UserId::new("owner").unwrap()),
            &request
        )
        .is_ok()
    );
    // A read: it changes no state and starts and ends nothing.
    assert!(!request.operation.is_mutation());
    assert_eq!(request.operation.project_id(), Some(&project_id()));
    assert!(
        negotiate(&[CURRENT_VERSION])
            .unwrap()
            .capabilities
            .contains(&symbiote_protocol::Capability::CompatibilityDossier)
    );
    // The body is the record as published, with the run this Host held named by
    // the pair a reader looks it up with — the run is inside the record, not
    // copied out of it beside itself.
    let dossier: symbiote_runtime_sdk::dossier::CompatibilityDossier = serde_json::from_value(
        json!({"schema_version": 1, "adapter": "native-agent", "driver": null, "runs": []}),
    )
    .unwrap();
    let body = ResponseBody::CompatibilityDossier(Box::new(DossierHolding {
        dossier: Box::new(dossier.clone()),
        upstream_version: "0.1.0".into(),
        platform: "linux".into(),
    }));
    let wire = serde_json::to_value(&body).unwrap();
    assert_eq!(wire["kind"], "compatibility_dossier");
    assert_eq!(wire["data"]["dossier"]["adapter"], "native-agent");
    assert_eq!(wire["data"]["upstream_version"], "0.1.0");
    assert_eq!(wire["data"]["platform"], "linux");
    assert!(wire["data"]["dossier"].get("runs").is_some());
    assert_eq!(
        serde_json::from_value::<ResponseBody>(wire).unwrap(),
        body,
        "the served record round-trips as itself"
    );
    // The request round-trips too, and a request that tried to carry the record
    // in the envelope is not a request: the record arrives in the response only.
    let sent = serde_json::to_string(&request).unwrap();
    assert!(sent.contains("get_compatibility_dossier"));
    assert_eq!(parse_request(sent.as_bytes()).unwrap(), request);
    let mut spoofed = serde_json::to_value(&request).unwrap();
    spoofed["operation"]["dossier"] = json!({"schema_version": 1});
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
/// The runtime inventory read is owner authority over this Host's own machine,
/// not a Project record: a Project Read grant is not authority over what an
/// operator installed here, and the response carries the discovery document
/// whole rather than a filtered view of it. A read is not a mutation, and the
/// record cannot be smuggled in through the request envelope.
#[test]
fn the_runtime_inventory_read_is_owner_authority_and_serves_the_document_whole() {
    let req = request(Operation::GetRuntimeInventory {});
    assert_eq!(
        authorize(&principal(), &req).unwrap_err().code,
        ErrorCode::PermissionDenied,
        "a Project Read grant does not reach this Host's own inventory"
    );
    assert!(authorize(&Principal::local_owner(UserId::new("owner").unwrap()), &req).is_ok());
    assert!(!req.operation.is_mutation(), "a read changes nothing");
    assert!(req.operation.project_id().is_none(), "no Project scopes it");
    assert!(
        negotiate(&[CURRENT_VERSION])
            .unwrap()
            .capabilities
            .contains(&symbiote_protocol::Capability::RuntimeInventoryRead),
        "a client negotiates the capability it needs to ask"
    );
    // The identity a request would try to name is refused at the envelope: the
    // Host is the only source of which Host this is.
    for key in ["host_id", "inventory", "records", "principal"] {
        let mut spoof = serde_json::to_value(&req).unwrap();
        spoof["operation"][key] = json!("another-host");
        assert!(
            parse_request(&serde_json::to_vec(&spoof).unwrap()).is_err(),
            "{key} is not a field this operation carries"
        );
    }
    // The body is the document the Host published, served as itself: a stale
    // record is still a record, and the reader sees its own `expires_at`.
    let observation = json!({
        "config_identity": "codex-default",
        "profile_id": "profile-1",
        "installation_id": "installation-1",
        "host_id": "host-a",
        "runtime_kind": "EXTERNAL_HARNESS",
        "adapter_id": "adapter-1",
        "version": {"status": "known", "value": "0.118.0"},
        "interface": {"status": "known", "value": {"name": "app_server", "version": "0.118.0"}},
        "facts": {
            "health": {"status": "known", "value": "reachable"},
            "authentication": {"status": "known", "value": {"mode": "none", "state": "required", "account_ref": null}},
            "models": {"status": "unknown"},
            "methods": {"initialize": "available"},
            "isolation": {"status": "unknown"}
        },
        "observed_at": 1000,
        "expires_at": 2000,
        "provenance": {"probe_id": "probe-1", "source": "sandboxed_probe", "adapter_revision": "0.118.0"}
    });
    let document = json!({
        "schema_version": 1,
        "records": [{
            "schema_version": 1,
            "intent": {
                "profile_id": "profile-1",
                "installation_id": "installation-1",
                "host_id": "host-a",
                "runtime_kind": "EXTERNAL_HARNESS",
                "adapter_id": "adapter-1",
                "instance_name": "Codex",
                "config_identity": "codex-default"
            },
            "observation": observation
        }]
    });
    let body = ResponseBody::RuntimeInventory(Box::new(
        symbiote_runtime_discovery::Inventory::parse(&document.to_string()).unwrap(),
    ));
    let response = Response::success(&req, body.clone());
    let bytes = encode_response(&response).unwrap();
    assert!(
        bytes.len() < MAX_RESPONSE_BYTES,
        "a served inventory is inside the response bound the transport frames"
    );
    let wire = serde_json::to_value(&response).unwrap();
    assert_eq!(wire["result"]["Ok"]["kind"], "runtime_inventory");
    assert_eq!(wire["result"]["Ok"]["data"]["schema_version"], 1);
    assert_eq!(
        wire["result"]["Ok"]["data"]["records"][0]["observation"]["expires_at"], 2000,
        "a record past its own expiry is served with the expiry it carries"
    );
    assert_eq!(
        serde_json::from_value::<ResponseBody>(wire["result"]["Ok"].clone()).unwrap(),
        body,
        "the served document round-trips as itself"
    );
    // The request round-trips, and the wire kind is the one the operation
    // table and the schema publish.
    let sent = serde_json::to_string(&req).unwrap();
    assert!(sent.contains("get_runtime_inventory"));
    assert_eq!(parse_request(sent.as_bytes()).unwrap(), req);
    let schema = schema_json();
    assert!(schema.contains("get_runtime_inventory"));
    assert!(schema.contains("runtime_inventory_read"));
    // A version the Host does not publish is not admitted by the response body
    // either: the served document is validated, not echoed.
    let mut wrong = document.clone();
    wrong["schema_version"] = json!(2);
    assert!(symbiote_runtime_discovery::Inventory::parse(&wrong.to_string()).is_err());
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
/// One canonical Project record with a stable identity and no roots: enough
/// for a registry listing, which carries records rather than placements.
fn project_record(id: &str) -> Project {
    Project {
        id: ProjectId::new(id).unwrap(),
        revision: Revision(0),
        name: format!("Project {id}"),
        owner: UserId::new("user-a").unwrap(),
        roots: BTreeSet::new(),
        lead: RoleId::new(format!("lead-{id}")).unwrap(),
        disposition: RecordDisposition::Active,
        provenance: Provenance {
            created_at: Timestamp(1),
            updated_at: Timestamp(1),
            actor: Actor::User(UserId::new("user-a").unwrap()),
            external_references: vec![],
        },
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
    let fixture = r#"{"version":{"major":1,"minor":26},"correlation_id":"request-a","command_id":"command-a","operation":{"kind":"get_project","project_id":"project-a"}}"#;
    let parsed = parse_request(fixture.as_bytes()).unwrap();
    assert_eq!(serde_json::to_string(&parsed).unwrap(), fixture);
    let error = Response::failure(
        Some(parsed.correlation_id),
        ProtocolError::new(ErrorCode::NotFound),
    );
    assert_eq!(
        serde_json::to_value(error).unwrap(),
        json!({"version":{"major":1,"minor": 26},"correlation_id":"request-a","result":{"Err":{"code":"not_found","message":"resource not found"}}})
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
fn the_registry_listing_is_scoped_by_the_callers_own_read_grants() {
    let records = vec![
        project_record("registry-one"),
        project_record("registry-two"),
        project_record("registry-three"),
    ];
    // The local owner sees every record, in the order it supplied them.
    assert_eq!(
        authorized_projects(
            &Principal::local_owner(UserId::new("local").unwrap()),
            records.clone()
        ),
        records
    );
    // A restricted caller sees exactly the Projects it may read: Read on the
    // second record admits that record alone, and the order is preserved.
    let reader = Principal::restricted(
        UserId::new("reader").unwrap(),
        BTreeMap::from([(
            records[1].id.clone(),
            BTreeSet::from([ProjectPermission::Read]),
        )]),
    );
    assert_eq!(
        authorized_projects(&reader, records.clone()),
        vec![records[1].clone()]
    );
    // A caller that may manage a Project but not read it does not see it in
    // its registry: the listing applies the same Read rule GetProject applies
    // to one record, so authorization cannot be widened by asking for a list.
    let manager = Principal::restricted(
        UserId::new("manager").unwrap(),
        BTreeMap::from([(
            records[2].id.clone(),
            BTreeSet::from([ProjectPermission::ManageWork]),
        )]),
    );
    assert!(authorized_projects(&manager, records.clone()).is_empty());
    // A caller with no grants sees an empty registry, never every Project.
    let stranger = Principal::restricted(UserId::new("stranger").unwrap(), BTreeMap::new());
    assert!(authorized_projects(&stranger, records).is_empty());
}

#[test]
fn the_registry_listing_is_admitted_and_carries_no_identity_to_claim() {
    // The gate admits the operation for any authenticated caller; the scoping
    // is the per-record read rule above, never the connection.
    for principal in [
        principal(),
        Principal::restricted(UserId::new("stranger").unwrap(), BTreeMap::new()),
    ] {
        assert!(authorize(&principal, &request(Operation::ListProjects {})).is_ok());
    }
    // The request names no Project and no identity: there is no field in which
    // a caller could claim to read another's registry.
    let wire = serde_json::to_value(request(Operation::ListProjects {})).unwrap();
    assert_eq!(wire["operation"], json!({"kind": "list_projects"}));
    assert_eq!(wire["operation"].as_object().unwrap().len(), 1);
    // The response is the declared `projects` kind carrying canonical records.
    let body = Response::success(
        &request(Operation::ListProjects {}),
        ResponseBody::Projects {
            projects: vec![project_record("registry-one")],
        },
    );
    let encoded = serde_json::to_value(body).unwrap();
    assert_eq!(encoded["result"]["Ok"]["kind"], json!("projects"));
    assert_eq!(
        encoded["result"]["Ok"]["data"]["projects"][0]["id"],
        json!("registry-one")
    );
}

#[test]
fn the_snapshot_is_gated_by_the_projects_own_read_rule_and_names_its_cursor() {
    let project = project_record("snapshot-one");
    // The gate is the same per-Project Read rule `GetProject` applies to one
    // record: a caller that may read the Project may read its snapshot, and a
    // caller that may only manage it may not.
    let reader = Principal::restricted(
        UserId::new("reader").unwrap(),
        BTreeMap::from([(
            project.id.clone(),
            BTreeSet::from([ProjectPermission::Read]),
        )]),
    );
    assert!(
        authorize(
            &reader,
            &request(Operation::Snapshot {
                project_id: project.id.clone()
            })
        )
        .is_ok()
    );
    let manager = Principal::restricted(
        UserId::new("manager").unwrap(),
        BTreeMap::from([(
            project.id.clone(),
            BTreeSet::from([ProjectPermission::ManageWork]),
        )]),
    );
    assert!(matches!(
        authorize(
            &manager,
            &request(Operation::Snapshot {
                project_id: project.id.clone()
            })
        ),
        Err(error) if error.code == ErrorCode::PermissionDenied
    ));
    let stranger = Principal::restricted(UserId::new("stranger").unwrap(), BTreeMap::new());
    assert!(
        authorize(
            &stranger,
            &request(Operation::Snapshot {
                project_id: project.id.clone()
            })
        )
        .is_err()
    );
    // The request names exactly one Project and nothing else: it carries no
    // cursor, limit or scope a caller could use to ask for a partial snapshot
    // or a point of its own choosing.
    let wire = serde_json::to_value(request(Operation::Snapshot {
        project_id: project.id.clone(),
    }))
    .unwrap();
    assert_eq!(
        wire["operation"],
        json!({"kind": "snapshot", "project_id": "snapshot-one"})
    );
    assert_eq!(wire["operation"].as_object().unwrap().len(), 2);
    // The response is the declared `snapshot` kind carrying the canonical
    // records with the cursor they were read at.
    let body = Response::success(
        &request(Operation::Snapshot {
            project_id: project.id.clone(),
        }),
        ResponseBody::Snapshot(Box::new(ProjectSnapshot {
            project: project.clone(),
            roots: vec![],
            roles: vec![],
            tasks: vec![],
            cursor: JournalCursor(9),
        })),
    );
    let encoded = serde_json::to_value(body).unwrap();
    assert_eq!(encoded["result"]["Ok"]["kind"], json!("snapshot"));
    assert_eq!(
        encoded["result"]["Ok"]["data"]["project"]["id"],
        json!("snapshot-one")
    );
    assert_eq!(encoded["result"]["Ok"]["data"]["cursor"], json!(9));
    assert_eq!(encoded["result"]["Ok"]["data"]["tasks"], json!([]));
    assert_eq!(encoded["result"]["Ok"]["data"]["roots"], json!([]));
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

/// The example targets this crate's manifest declares, as (name, path) pairs: the owner of the
/// names every document that tells a reader to run one is compared against, read here rather than
/// restated, so a target renamed or removed in the manifest moves every reading of it in the same
/// change.
fn declared_examples(manifest: &str) -> Vec<(String, String)> {
    fn quoted(line: &str, key: &str) -> Option<String> {
        let anchor = format!("{key} = \"");
        let start = line.find(&anchor)? + anchor.len();
        let rest = &line[start..];
        Some(rest[..rest.find('"')?].to_string())
    }
    let mut examples: Vec<(String, String)> = Vec::new();
    let mut current: Option<(Option<String>, Option<String>)> = None;
    let close = |current: &mut Option<(Option<String>, Option<String>)>,
                 examples: &mut Vec<(String, String)>| {
        if let Some((Some(name), Some(path))) = current.take() {
            examples.push((name, path));
        }
    };
    for line in manifest.lines() {
        let line = line.trim();
        if line == "[[example]]" {
            close(&mut current, &mut examples);
            current = Some((None, None));
            continue;
        }
        if line.starts_with('[') {
            close(&mut current, &mut examples);
            continue;
        }
        if let Some((name, path)) = current.as_mut() {
            if name.is_none() {
                *name = quoted(line, "name");
            } else if path.is_none() {
                *path = quoted(line, "path");
            }
        }
    }
    close(&mut current, &mut examples);
    examples
}

/// Every markdown file this repository ships, found by walking it rather than listed here, so a
/// document added anywhere in the tree is read by whatever case scans the surface it states.
fn markdown_documents(directory: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut found = Vec::new();
    let Ok(entries) = std::fs::read_dir(directory) else {
        return found;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if path.is_dir() {
            if !matches!(name.as_str(), ".git" | "target" | "node_modules") {
                found.extend(markdown_documents(&path));
            }
        } else if name.ends_with(".md") {
            found.push(path);
        }
    }
    found
}

/// The drift check the document tells a reader to run is the comparison this crate makes: it
/// accepts the committed artifact and names the first line an edited one differs at, so
/// `protocol.md`'s `--check` sentence is held by a case rather than by the example's own code.
///
/// What it does not read: which line a *shortened* artifact should report beyond the first line
/// the two do not share, which is the drift report's own choice and not a contract.
#[test]
fn the_drift_check_accepts_the_committed_artifact_and_names_the_line_an_edited_one_moved() {
    let root = symbiote_protocol::workspace_root();
    assert_eq!(
        schema_drift(&root.join(SCHEMA_PATH)),
        Ok(()),
        "the committed artifact is the document these types generate"
    );

    // The line the report must name is the line this case edits, read from the generated document
    // rather than from the drift check's own arithmetic, so the two are not the same measurement.
    let generated = schema_json();
    let titled = "\"title\": \"Telemetry\"";
    let line = generated
        .lines()
        .position(|one| one.contains(titled))
        .map(|index| index + 1)
        .expect("the generated document titles the telemetry schema");

    let edited = root.join(format!(
        "target/probe-181-drift-{}.json",
        std::process::id()
    ));
    std::fs::create_dir_all(edited.parent().expect("a parent directory")).unwrap();
    std::fs::write(
        &edited,
        generated.replacen(titled, "\"title\": \"TelemetryV2\"", 1),
    )
    .unwrap();
    let drift = schema_drift(&edited).expect_err("an edited artifact is drift");
    std::fs::remove_file(&edited).unwrap();
    assert!(
        drift.contains(&format!("differs at line {line}")),
        "the drift report names the line that moved ({line}): {drift}"
    );

    assert!(
        schema_drift(&root.join("docs/contracts/not-there.json")).is_err(),
        "an unreadable artifact is reported rather than accepted"
    );
}

/// The canonical schema is published where the document says it is (#181's "generate or validate
/// typed clients and contract tests from a canonical schema"): `protocol.md` names the generator
/// and the artifact, this crate's manifest declares that example target and the file it points at,
/// and the committed artifact is byte-for-byte the document these types generate. A renamed
/// target, a moved or edited artifact, or a wire type that changes without the artifact each reds
/// here by name, instead of leaving a reader a command whose output nothing compares.
///
/// What it does not read: the artifact's own content beyond equality with [`schema_json`] — which
/// nested record a schema describes is the wire types' business, and each record's own case is
/// what drives its shape.
#[test]
fn the_published_schema_artifact_is_the_document_these_types_generate() {
    let contract = include_str!("../../../docs/contracts/protocol.md");
    let generator = region(contract, "generated by `", "`");
    let target = generator
        .rsplit("--example ")
        .next()
        .unwrap_or_default()
        .trim()
        .to_string();
    assert!(
        generator.starts_with("cargo run -p symbiote-protocol"),
        "the document names the command that generates the schema: {generator}"
    );
    let artifact = region(contract, "committed at `", "`");
    assert_eq!(
        artifact, SCHEMA_PATH,
        "the document names the artifact this crate publishes"
    );

    let manifest = include_str!("../Cargo.toml");
    let declared = declared_examples(manifest);
    let (_, path) = declared
        .iter()
        .find(|(name, _)| name == &target)
        .unwrap_or_else(|| {
            panic!("the manifest declares no example target called {target}: {declared:?}")
        });
    let root = symbiote_protocol::workspace_root();
    assert!(
        root.join("crates/symbiote-protocol").join(path).is_file(),
        "the manifest's example target {target} points at {path}, which is not a file"
    );

    let committed = std::fs::read_to_string(root.join(SCHEMA_PATH)).unwrap_or_else(|error| {
        panic!("the committed artifact {SCHEMA_PATH} is readable: {error}")
    });
    assert!(
        committed == schema_json(),
        "the committed artifact {SCHEMA_PATH} is not the document these types generate; \
         regenerate it with `cargo run -p symbiote-protocol --example protocol_schema -- --write`"
    );
}

/// The generator command is stated by three documents — `protocol.md` as the artifact's owner,
/// `host.md` where a reader prepares request files, and the repository `README.md` as the entry
/// point — and a reader arriving at any of them must get a command this crate can actually run.
/// The manifest owns the target names, the walk finds the documents rather than listing them, and
/// the document set is the figure this case holds: a target renamed in any copy, or a copy added
/// or dropped, moves in the same change instead of leaving a reader a command nothing checks.
///
/// What it does not read: the flags each copy adds beyond the target (host.md passes `--locked`),
/// which belong to the command the document is teaching, and whether three copies is the right
/// number — that the documents keep the readers' entry points is the editors' judgement, the
/// figure here only refuses to let one move silently.
#[test]
fn every_document_that_states_the_schema_command_names_a_declared_example_target() {
    const COMMAND: &str = "cargo run -p symbiote-protocol --example ";
    let root = symbiote_protocol::workspace_root();
    let declared = declared_examples(include_str!("../Cargo.toml"));
    assert!(
        !declared.is_empty(),
        "this crate declares no example target, so no documented command can run"
    );

    let mut stating = Vec::new();
    for document in markdown_documents(&root) {
        let Ok(text) = std::fs::read_to_string(&document) else {
            continue;
        };
        let named = document
            .strip_prefix(&root)
            .unwrap_or(&document)
            .display()
            .to_string();
        for rest in text.split(COMMAND).skip(1) {
            let target: String = rest
                .chars()
                .take_while(|one| !one.is_whitespace() && *one != '`')
                .collect();
            assert!(
                declared.iter().any(|(name, _)| name == &target),
                "{named} tells a reader to run `{COMMAND}{target}`, and the manifest declares \
                 {declared:?}"
            );
            if !stating.contains(&named) {
                stating.push(named.clone());
            }
        }
    }
    stating.sort();
    assert_eq!(
        stating,
        [
            "README.md",
            "docs/contracts/host.md",
            "docs/contracts/protocol.md"
        ],
        "the documents stating the schema command moved; each is a reader's entry point, so this \
         figure moves with them"
    );
}

/// #189 foreign runtime references: a Project edit to record and a Project
/// read to serve, with nothing on the wire that could move canonical state.
#[test]
fn foreign_runtime_links_are_a_project_edit_and_a_project_read() {
    let link = ForeignTaskLink {
        system: ExternalSystem::Harness,
        kind: ForeignItemKind::Task,
        foreign_id: "codex-todo-7".into(),
        foreign_session_id: Some("codex-session-01".into()),
        foreign_status: ForeignStatus::Complete,
        observed_at: Timestamp(10),
    };
    let set = request(Operation::SetTaskForeignLinks {
        project_id: project_id(),
        task_id: TaskId::new("task-a").unwrap(),
        links: vec![link.clone()],
    });
    let get = request(Operation::GetTaskForeignLinks {
        project_id: project_id(),
        task_id: TaskId::new("task-a").unwrap(),
    });
    let grants = |permissions| {
        Principal::restricted(
            UserId::new("user-a").unwrap(),
            BTreeMap::from([(project_id(), permissions)]),
        )
    };
    let manager = grants(BTreeSet::from([
        ProjectPermission::Read,
        ProjectPermission::ManageWork,
    ]));
    let reader = grants(BTreeSet::from([ProjectPermission::Read]));
    assert!(authorize(&manager, &set).is_ok());
    assert!(authorize(&manager, &get).is_ok());
    // Read is not ManageWork: a reader can follow a reference but cannot record
    // what a foreign system claims.
    assert_eq!(
        authorize(&reader, &set).unwrap_err().code,
        ErrorCode::PermissionDenied
    );
    assert!(authorize(&reader, &get).is_ok());
    // ManageWork on another Project is not a grant on this one.
    let elsewhere = Principal::restricted(
        UserId::new("user-a").unwrap(),
        BTreeMap::from([(
            ProjectId::new("project-b").unwrap(),
            BTreeSet::from([ProjectPermission::Read, ProjectPermission::ManageWork]),
        )]),
    );
    assert_eq!(
        authorize(&elsewhere, &set).unwrap_err().code,
        ErrorCode::PermissionDenied
    );
    // The write is classified as a mutation and attributed to its Project; the
    // read is neither.
    assert!(set.operation.is_mutation());
    assert!(!get.operation.is_mutation());
    assert_eq!(set.operation.project_id(), Some(&project_id()));
    assert_eq!(get.operation.project_id(), Some(&project_id()));
    // Both capabilities are advertised, so a client can tell before it sends.
    let hello = negotiate(&[CURRENT_VERSION]).unwrap();
    assert!(
        hello
            .capabilities
            .contains(&symbiote_protocol::Capability::ForeignLinkWrite)
    );
    assert!(
        hello
            .capabilities
            .contains(&symbiote_protocol::Capability::ForeignLinkRead)
    );
    // The wire form of a link has no canonical status, actor or timestamp to
    // forge: `observed_at` is the only time on the wire and it is the
    // authority's own to set, and a `task_state` field is refused rather than
    // ignored.
    let wire = serde_json::to_value(&link).unwrap();
    let fields: BTreeSet<&str> = wire
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(
        fields,
        BTreeSet::from([
            "foreign_id",
            "foreign_session_id",
            "foreign_status",
            "kind",
            "observed_at",
            "system",
        ])
    );
    let forged = json!({
        "system": "harness", "kind": "task", "foreign_id": "codex-todo-7",
        "foreign_status": "complete", "observed_at": 10, "task_state": "completed"
    });
    assert!(serde_json::from_value::<ForeignTaskLink>(forged).is_err());
    // The response serves the foreign claim as itself: the body is a list of
    // links, not a Task and not a status.
    let body = ResponseBody::TaskForeignLinks(vec![link.clone()]);
    let served = serde_json::to_value(&body).unwrap();
    assert_eq!(served["kind"], json!("task_foreign_links"));
    assert_eq!(served["data"][0]["foreign_status"], json!("complete"));
    assert!(
        !served.to_string().contains("task_state"),
        "a served link set never looks like canonical state: {served}"
    );
    // An operation whose links the domain refuses is refused before it is
    // authorized: the wire validator holds the same bounds the store does.
    let over = request(Operation::SetTaskForeignLinks {
        project_id: project_id(),
        task_id: TaskId::new("task-a").unwrap(),
        links: vec![ForeignTaskLink {
            foreign_id: "s".repeat(MAX_FOREIGN_ID_BYTES + 1),
            ..link
        }],
    });
    assert_eq!(
        parse_request(serde_json::to_string(&over).unwrap().as_bytes())
            .unwrap_err()
            .code,
        ErrorCode::InvalidRequest
    );
}
