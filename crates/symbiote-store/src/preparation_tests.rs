use super::*;

/// Registers a project, a Team, a workforce binding (with the profile's
/// provider connection registered in the #464 registry), an objective origin,
/// and a Ready task — everything prepare_dispatch composes from.
fn full_fixture(store: &mut Store, tag: &str) -> (ProjectId, TaskId) {
    let (project, roots, mut roles) = records(&format!("prep-{tag}"));
    roles.push(Role {
        id: id!(RoleId, &format!("worker-{tag}")),
        project_id: project.id.clone(),
        revision: Revision(0),
        name: "Engineer".into(),
        operating_contract: VersionedRoleContract {
            id: id!(RoleContractId, &format!("worker-contract-{tag}")),
            revision: Revision(1),
        },
    });
    store
        .register_project(
            id!(CommandId, &format!("register-{tag}")),
            project.clone(),
            roots,
            roles.clone(),
        )
        .unwrap();
    // Team: lead + general-executor engineer, reciprocal review.
    let access = AccessSnapshot {
        project_id: project.id.clone(),
        roots: project.roots.clone(),
        grants: BTreeSet::from([Permission::ReadRoot, Permission::ExecuteProcess]),
        policy_revision: Revision(1),
    };
    let lead = RolePolicy {
        role_id: project.lead.clone(),
        function: RoleFunction::LeadOrchestrator,
        responsibilities: vec!["Plan and integrate".into()],
        task_domains: BTreeSet::from(["coordination".into()]),
        access: access.clone(),
        context_policy_ref: "context".into(),
        tool_policy_ref: "tools".into(),
        skill_policy_ref: "skills".into(),
        execution_policy_ref: "execution".into(),
        independent_reviewers: BTreeSet::from([id!(RoleId, &format!("worker-{tag}"))]),
        fallbacks: vec![],
    };
    let mut worker_access = access.clone();
    worker_access.grants = BTreeSet::from([
        Permission::ReadRoot,
        Permission::ExecuteProcess,
        Permission::MutateStream,
    ]);
    let worker_access_for_binding = worker_access.clone();
    let worker = RolePolicy {
        role_id: id!(RoleId, &format!("worker-{tag}")),
        function: RoleFunction::GeneralExecution,
        responsibilities: vec!["Implement bounded tasks".into()],
        task_domains: BTreeSet::from(["coding".into()]),
        access: worker_access.clone(),
        context_policy_ref: "context".into(),
        tool_policy_ref: "tools".into(),
        skill_policy_ref: "skills".into(),
        execution_policy_ref: "execution".into(),
        independent_reviewers: BTreeSet::from([project.lead.clone()]),
        fallbacks: vec![],
    };
    let mut ceiling = access.clone();
    ceiling.grants.insert(Permission::MutateStream);
    let team = TeamConfiguration {
        schema_version: 1,
        project_id: project.id.clone(),
        revision: Revision(0),
        lead_role_id: project.lead.clone(),
        access_ceiling: ceiling,
        members: vec![lead, worker],
    };
    store
        .replace_team(
            id!(CommandId, &format!("team-{tag}")),
            None,
            team.clone(),
            id!(UserId, "owner"),
            Timestamp(10),
        )
        .unwrap();
    // Provider registry: connection referenced by the binding's profile.
    let connection = symbiote_domain::ProviderConnection {
        id: id!(ProviderConnectionId, &format!("provider-{tag}")),
        adapter: id!(InferenceProviderAdapterId, "adapter"),
        endpoint_reference: "https://api.openai.example/v1".into(),
        authentication: symbiote_domain::AuthenticationKind::ApiCredential,
    };
    let attribution = project.id.clone();
    store
        .replace_provider_connection(
            id!(CommandId, &format!("provider-{tag}")),
            attribution.clone(),
            connection.clone(),
            id!(UserId, "owner"),
            Timestamp(11),
        )
        .unwrap();
    // Workforce binding bound to the routed Role, profile pointing at the
    // registered provider connection.
    let profile = RuntimeProfile {
        id: id!(RuntimeProfileId, &format!("profile-{tag}")),
        revision: Revision(0),
        runtime: RuntimeKind::NativeSymbiote,
        adapter: id!(AgentRuntimeAdapterId, "adapter"),
        installation: None,
        provider: id!(ProviderConnectionId, &format!("provider-{tag}")),
        credential: id!(CredentialReferenceId, "credential"),
        billing_entitlement: id!(BillingEntitlementId, &format!("ent-{tag}")),
        model: id!(ModelId, &format!("model-{tag}")),
        eligible_hosts: BTreeSet::from([id!(HostId, &format!("host-{tag}"))]),
    };
    let primary = symbiote_workforce::StaffingCandidate {
        profile: profile.clone(),
        config_identity: "native-default".into(),
        access: worker_access.clone(),
        tools: BTreeSet::new(),
        skills: BTreeSet::new(),
        secret_scopes: BTreeSet::new(),
        context: symbiote_domain::ContextPolicy {
            bundle: id!(ContextBundleId, "context"),
            revision: Revision(1),
            max_input_tokens: 100,
            reserved_output_tokens: 50,
        },
        limits: symbiote_workforce::ResourceLimits {
            max_total_tokens: 1_000,
            max_wall_time_ms: 60_000,
            max_concurrency: 1,
            max_memory_bytes: 1 << 20,
        },
    };
    let policy = || symbiote_workforce::PolicyReference {
        id: "policy".into(),
        revision: Revision(0),
    };
    let mut configuration = symbiote_workforce::BindingConfiguration {
        schema_version: symbiote_workforce::BINDING_VERSION,
        binding: symbiote_domain::WorkforceBinding {
            id: id!(BindingId, &format!("binding-{tag}")),
            revision: Revision(0),
            project_id: project.id.clone(),
            role_id: id!(RoleId, &format!("worker-{tag}")),
            profile_id: profile.id.clone(),
            profile_revision: profile.revision,
            protocol: VersionedProtocol {
                id: id!(ProtocolId, "protocol"),
                revision: Revision(1),
            },
            access: worker_access_for_binding,
            required_controls: BTreeSet::from([symbiote_domain::Control::Filesystem]),
            context: primary.context.clone(),
            required_tools: BTreeSet::new(),
            required_skills: BTreeSet::new(),
            escalation: symbiote_domain::EscalationPolicy::StopAndRequestHuman,
        },
        team_revision: team.revision,
        primary,
        fallbacks: vec![],
        policies: symbiote_workforce::WorkforcePolicies {
            environment: policy(),
            worktree: policy(),
            verification: policy(),
            documentation: policy(),
            artifacts: policy(),
            mcp: policy(),
            escalation: policy(),
            root_effort: symbiote_workforce::RootEffort::Medium,
            local_children: symbiote_workforce::LocalChildPolicy::default(),
            fallback_consent: symbiote_domain::FallbackConsent::ExplicitRequired,
            minimum_enforcement: [
                symbiote_domain::Control::Filesystem,
                symbiote_domain::Control::Cancellation,
                symbiote_domain::Control::CompletionAuthority,
                symbiote_domain::Control::Process,
            ]
            .into_iter()
            .map(|c| (c, symbiote_domain::EnforcementStrength::HostEnforced))
            .collect(),
            required_capabilities: BTreeSet::new(),
        },
    };
    let worker_role_id = id!(RoleId, &format!("worker-{tag}"));
    configuration.binding.role_id = worker_role_id;
    configuration.validate_team(&team).unwrap();
    store
        .replace_binding(
            id!(CommandId, &format!("binding-{tag}")),
            None,
            configuration,
            id!(UserId, "owner"),
            Timestamp(12),
        )
        .unwrap();
    // Objective + task. register_work_target creates the classified
    // objective with a command id derived from the project identity.
    let (task, stream) = task_records(tag);
    let task = Task::new(
        task.id().clone(),
        project.id.clone(),
        project.roots.iter().next().unwrap().clone(),
        id!(RoleId, &format!("worker-{tag}")),
        id!(ChangeStreamId, &format!("stream-{tag}")),
        VersionedTaskContract {
            id: id!(TaskContractId, &format!("contract-{tag}")),
            revision: Revision(1),
        },
    );
    let stream = ChangeStream::new(NewChangeStream {
        id: id!(ChangeStreamId, &format!("stream-{tag}")),
        project_id: project.id.clone(),
        root_id: project.roots.iter().next().unwrap().clone(),
        tasks: BTreeSet::from([task.id().clone()]),
        originating_chat: id!(ChatId, &format!("chat-{tag}")),
        worktree: stream.worktree.clone(),
        branch: stream.branch.clone(),
        lineage: StreamLineage::Independent,
        base: sha('a'),
        target: sha('b'),
    })
    .unwrap();
    let origin = register_work_target(store, &task).unwrap();
    store
        .create_task(id!(CommandId, &format!("task-{tag}")), task, stream, origin)
        .unwrap();
    (project.id, id!(TaskId, &format!("task-{tag}")))
}

#[test]
fn preparation_composes_routing_lease_worktree_and_provider() {
    let mut store = Store::memory().unwrap();
    // Fixture origin stream/worktree collide with the task's stream; use
    // task_records only for identity reuse with distinct streams.
    let (_project, task) = full_fixture(&mut store, "one");
    // No route recorded yet: preparation must refuse and record why.
    let (receipt, preparation) = store
        .prepare_dispatch(
            id!(CommandId, "prepare-1"),
            task.clone(),
            id!(UserId, "owner"),
            Timestamp(30),
        )
        .unwrap();
    assert!(!receipt.replayed);
    assert_eq!(preparation.outcome, PreparationOutcome::Refused);
    // Routing step recorded without resolution.
    assert!(
        preparation
            .steps
            .iter()
            .any(|step| matches!(step, CompositionStep::Routing { resolved: None }))
    );
    // Provider resolves through the routed Role's workforce binding, which
    // the fixture registered against the #464 registry.
    assert_eq!(
        preparation.provider_connection,
        Some(id!(ProviderConnectionId, "provider-one"))
    );
    // Worktree always composed from the stream record.
    assert!(preparation.worktree_id.is_some());
    assert!(preparation.branch.is_some());
    // Replay returns the same preparation.
    let (replay, replayed) = store
        .prepare_dispatch(
            id!(CommandId, "prepare-1"),
            task.clone(),
            id!(UserId, "owner"),
            Timestamp(30),
        )
        .unwrap();
    assert!(replay.replayed);
    assert_eq!(replayed, preparation);
    // Reading it back from storage matches.
    assert_eq!(store.dispatch_preparation(&task).unwrap(), preparation);
    // Preparing an unknown task is NotFound, never a fabricated record.
    assert!(matches!(
        store.prepare_dispatch(
            id!(CommandId, "prepare-unknown"),
            id!(TaskId, "ghost"),
            id!(UserId, "owner"),
            Timestamp(31),
        ),
        Err(StoreError::NotFound)
    ));
}

#[test]
fn start_from_preparation_compiles_dispatch_and_transitions_to_running() {
    let temp = Temporary::new();
    let mut store = Store::open(temp.database()).unwrap();
    let (project, task) = full_fixture(&mut store, "start");
    // Record a route so the preparation's routing step resolves. The route is
    // an explicit assignment of the task's own Role (classification never
    // routes to the Lead, and the task carries the worker Role here).
    let task_role = store.task(&task).unwrap().role_id().clone();
    let request = symbiote_workforce::RouteRequest {
        project_id: project.clone(),
        work_id: WorkId::Objective(id!(ObjectiveId, "project-prep-start")),
        requested: Some(task_role.clone()),
        domains: BTreeSet::new(),
    };
    let team = store.get_team(&project).unwrap();
    let decision = symbiote_workforce::resolve_route(&team, &request).unwrap();
    store
        .record_route(
            id!(CommandId, "route-start"),
            decision,
            id!(UserId, "owner"),
            Timestamp(20),
        )
        .unwrap();
    // Prepare: routing, worktree, and provider all resolve; the lease step
    // is informational (no lease held before the first start).
    let (_, preparation) = store
        .prepare_dispatch(
            id!(CommandId, "prepare-start"),
            task.clone(),
            id!(UserId, "owner"),
            Timestamp(30),
        )
        .unwrap();
    assert_eq!(preparation.outcome, PreparationOutcome::Ready);
    let routed_role = preparation.role_id.clone().unwrap();
    // Compile the dispatch the Host would start against the host's claims.
    let host = Host {
        id: id!(HostId, "host-start"),
        revision: Revision(0),
        device: id!(DeviceId, "device-start"),
        fabric: None,
        supported_runtimes: vec![RuntimeKind::NativeSymbiote],
        controls: [
            symbiote_domain::Control::Filesystem,
            symbiote_domain::Control::Cancellation,
            symbiote_domain::Control::CompletionAuthority,
            symbiote_domain::Control::Process,
        ]
        .into_iter()
        .map(|c| {
            (
                c,
                symbiote_domain::EnforcementClaim {
                    strength: symbiote_domain::EnforcementStrength::HostEnforced,
                    evidence: id!(EvidenceId, "proof"),
                    verified_at: Timestamp(1),
                    expires_at: Timestamp(1_000_000),
                },
            )
        })
        .collect(),
    };
    let binding_body: String = store
        .connection
        .query_row(
            "SELECT body FROM workforce_bindings WHERE project_id=?1 AND role_id=?2",
            params![project.as_str(), routed_role.as_str()],
            |r| r.get(0),
        )
        .unwrap();
    let binding: symbiote_workforce::BindingConfiguration =
        serde_json::from_str(&binding_body).unwrap();
    let task_record = store.task(&task).unwrap();
    let role_body: String = store
        .connection
        .query_row(
            "SELECT body FROM roles WHERE id=?1 AND project_id=?2",
            params![routed_role.as_str(), project.as_str()],
            |r| r.get(0),
        )
        .unwrap();
    let role: Role = serde_json::from_str(&role_body).unwrap();
    let dispatch = symbiote_domain::Dispatch::compile(
        id!(DispatchId, "dispatch-start"),
        id!(RuntimeContractId, "contract-start"),
        symbiote_domain::DispatchInputs {
            task: &task_record,
            role: &role,
            binding: &binding.binding,
            profile: &binding.primary.profile,
            host: &host,
            minimum_enforcement: &std::collections::BTreeMap::new(),
            now: Timestamp(35),
        },
    )
    .unwrap();
    // Start the prepared task: the store compiles its own dispatch from the
    // same inputs, transitions the task to Running, and acquires the
    // governing lease transactionally.
    let (receipt, started) = store
        .start_prepared_task(
            id!(CommandId, "start-1"),
            task.clone(),
            dispatch.id().clone(),
            id!(RuntimeContractId, "contract-start-2"),
            &host,
            id!(UserId, "owner"),
            Timestamp(50),
        )
        .unwrap();
    assert!(!receipt.replayed);
    assert_eq!(started.id(), dispatch.id());
    let started_task = store.task(&task).unwrap();
    assert_eq!(started_task.state(), &TaskState::Running);
    assert_eq!(
        started_task.current_dispatch().map(|d| d.id().clone()),
        Some(dispatch.id().clone())
    );
    // The governing lease is held with token 1 and binds the started dispatch.
    let lease = store.task_lease(&task).unwrap();
    assert_eq!(lease.state, symbiote_domain::LeaseState::Held);
    assert_eq!(lease.fencing_token, 1);
    assert_eq!(lease.dispatch_id, *dispatch.id());
    // The journal request bytes must satisfy the audit (P0 regression from
    // the PR #494 review): integrity check passes and reopen replays.
    store.integrity_check().unwrap();
    // Idempotent retry of the committed start replays its receipt.
    let (retry_receipt, retry_dispatch) = store
        .start_prepared_task(
            id!(CommandId, "start-1"),
            task.clone(),
            dispatch.id().clone(),
            id!(RuntimeContractId, "contract-start-2"),
            &host,
            id!(UserId, "owner"),
            Timestamp(51),
        )
        .unwrap();
    assert!(retry_receipt.replayed);
    assert_eq!(retry_receipt.sequence, receipt.sequence);
    assert_eq!(retry_dispatch.id(), dispatch.id());
    // A start under the reserved sweep namespace is refused.
    assert!(matches!(
        store.start_prepared_task(
            id!(CommandId, "symbiote-sweep-poison"),
            task.clone(),
            dispatch.id().clone(),
            id!(RuntimeContractId, "contract-x"),
            &host,
            id!(UserId, "owner"),
            Timestamp(52),
        ),
        Err(StoreError::InvalidLease)
    ));
    // Reopen: the started task, lease, and preparation all replay (the P0
    // regression — a committed start must never brick the store).
    drop(store);
    let mut store = Store::open(temp.database()).unwrap();
    assert_eq!(store.task(&task).unwrap().state(), &TaskState::Running);
    assert_eq!(
        store.task_lease(&task).unwrap().state,
        symbiote_domain::LeaseState::Held
    );
    store.integrity_check().unwrap();
    // Prepare again after the start: the task is Running, so the scheduling
    // step records the refusal — preparation precedes start, never follows.
    let (_, preparation) = store
        .prepare_dispatch(
            id!(CommandId, "prepare-start-2"),
            task.clone(),
            id!(UserId, "owner"),
            Timestamp(55),
        )
        .unwrap();
    assert_eq!(preparation.outcome, PreparationOutcome::Refused);
    assert!(
        preparation
            .steps
            .iter()
            .any(|step| matches!(step, CompositionStep::Scheduling { schedulable: false }))
    );
    // A second start is refused: the lease is already held (or, equivalently,
    // the task is no longer Ready for another Start).
    let second = store.start_prepared_task(
        id!(CommandId, "start-2"),
        task.clone(),
        dispatch.id().clone(),
        id!(RuntimeContractId, "contract-start-3"),
        &host,
        id!(UserId, "owner"),
        Timestamp(60),
    );
    // The second start must be refused. Because prepare-start-2 superseded
    // the stored preparation with a Refused record, the start sees
    // PreparationRefused; a stale-Ready path would yield StillHeld or
    // IllegalTransition. All three are refusals, never a second Running.
    assert!(
        matches!(
            &second,
            Err(StoreError::PreparationRefused)
                | Err(StoreError::LeaseConflict(
                    symbiote_domain::LeaseError::StillHeld
                ))
                | Err(StoreError::Domain(DomainError::IllegalTransition))
        ),
        "second start must be refused, got {second:?}"
    );
}

#[test]
fn start_recompiles_the_durable_binding_and_refuses_when_policy_floor_exceeds_host_claims() {
    let mut store = Store::memory().unwrap();
    let (project_id, task) = full_fixture(&mut store, "floor");
    // Record the task's route so the preparation's routing step resolves.
    let task_role = store.task(&task).unwrap().role_id().clone();
    let request = symbiote_workforce::RouteRequest {
        project_id: project_id.clone(),
        work_id: WorkId::Objective(id!(ObjectiveId, "project-prep-floor")),
        requested: Some(task_role.clone()),
        domains: BTreeSet::new(),
    };
    let team = store.get_team(&project_id).unwrap();
    let decision = symbiote_workforce::resolve_route(&team, &request).unwrap();
    store
        .record_route(
            id!(CommandId, "route-floor"),
            decision,
            id!(UserId, "owner"),
            Timestamp(20),
        )
        .unwrap();
    // The fixture's binding floors Filesystem at HostEnforced and the host
    // claims exactly that: prepare and the compiled contract agree.
    let (_, preparation) = store
        .prepare_dispatch(
            id!(CommandId, "prepare-floor"),
            task.clone(),
            id!(UserId, "owner"),
            Timestamp(30),
        )
        .unwrap();
    assert_eq!(preparation.outcome, PreparationOutcome::Ready);
    // Between prepare and start, the binding's enforcement policy is
    // raised to demand NATIVE filesystem enforcement. Re-staffing and
    // mid-policy change never inherit the previous authorization: start
    // recompiles from the durable binding and must refuse.
    let mut raised = store
        .get_binding(&project_id, &id!(BindingId, "binding-floor"))
        .unwrap();
    assert_eq!(raised.binding.revision, Revision(0));
    raised.policies.minimum_enforcement.insert(
        symbiote_domain::Control::Filesystem,
        symbiote_domain::EnforcementStrength::Native,
    );
    raised.binding.revision = Revision(1);
    store
        .replace_binding(
            id!(CommandId, "binding-floor-raise"),
            Some(Revision(0)),
            raised,
            id!(UserId, "owner"),
            Timestamp(32),
        )
        .unwrap();
    // The host claims HostEnforced for the required controls — exactly the
    // strength the raise now refuses to accept.
    let host = Host {
        id: id!(HostId, "host-floor"),
        revision: Revision(0),
        device: id!(DeviceId, "device-floor"),
        fabric: None,
        supported_runtimes: vec![RuntimeKind::NativeSymbiote],
        controls: [
            symbiote_domain::Control::Filesystem,
            symbiote_domain::Control::Cancellation,
            symbiote_domain::Control::CompletionAuthority,
            symbiote_domain::Control::Process,
        ]
        .into_iter()
        .map(|c| {
            (
                c,
                symbiote_domain::EnforcementClaim {
                    strength: symbiote_domain::EnforcementStrength::HostEnforced,
                    evidence: id!(EvidenceId, "proof"),
                    verified_at: Timestamp(1),
                    expires_at: Timestamp(1_000_000),
                },
            )
        })
        .collect(),
    };
    let refused = store.start_prepared_task(
        id!(CommandId, "start-floor"),
        task,
        id!(DispatchId, "dispatch-floor"),
        id!(RuntimeContractId, "contract-floor"),
        &host,
        id!(UserId, "owner"),
        Timestamp(35),
    );
    assert!(
        matches!(
            refused,
            Err(StoreError::Domain(DomainError::UnsupportedControl))
        ),
        "start must refuse under an unmeetable enforcement floor, got {refused:?}"
    );
}

#[test]
fn elevation_leases_are_attributable_bounded_and_sticky() {
    let temp = Temporary::new();
    let mut store = Store::open(temp.database()).unwrap();
    let (project, task) = full_fixture(&mut store, "elev");
    // Route, prepare, and start: the elevated dispatch must be live.
    let task_role = store.task(&task).unwrap().role_id().clone();
    let request = symbiote_workforce::RouteRequest {
        project_id: project.clone(),
        work_id: WorkId::Objective(id!(ObjectiveId, "project-prep-elev")),
        requested: Some(task_role.clone()),
        domains: BTreeSet::new(),
    };
    let team = store.get_team(&project).unwrap();
    let decision = symbiote_workforce::resolve_route(&team, &request).unwrap();
    store
        .record_route(
            id!(CommandId, "route-elev"),
            decision,
            id!(UserId, "owner"),
            Timestamp(20),
        )
        .unwrap();
    store
        .prepare_dispatch(
            id!(CommandId, "prepare-elev"),
            task.clone(),
            id!(UserId, "owner"),
            Timestamp(30),
        )
        .unwrap();
    let host = Host {
        id: id!(HostId, "host-elev"),
        revision: Revision(0),
        device: id!(DeviceId, "device-elev"),
        fabric: None,
        supported_runtimes: vec![RuntimeKind::NativeSymbiote],
        controls: [
            symbiote_domain::Control::Filesystem,
            symbiote_domain::Control::Cancellation,
            symbiote_domain::Control::CompletionAuthority,
            symbiote_domain::Control::Process,
        ]
        .into_iter()
        .map(|c| {
            (
                c,
                symbiote_domain::EnforcementClaim {
                    strength: symbiote_domain::EnforcementStrength::HostEnforced,
                    evidence: id!(EvidenceId, "proof"),
                    verified_at: Timestamp(1),
                    expires_at: Timestamp(1_000_000),
                },
            )
        })
        .collect(),
    };
    let dispatch_id = id!(DispatchId, "dispatch-elev");
    store
        .start_prepared_task(
            id!(CommandId, "start-elev"),
            task.clone(),
            dispatch_id.clone(),
            id!(RuntimeContractId, "contract-elev"),
            &host,
            id!(UserId, "owner"),
            Timestamp(40),
        )
        .unwrap();
    let lease = |command: &str| symbiote_domain::ElevationLease {
        id: id!(CommandId, command),
        project_id: project.clone(),
        task_id: task.clone(),
        dispatch_id: dispatch_id.clone(),
        permission: Permission::UseCredential,
        reason: "the run must read the operator's configured secret".into(),
        approved: true,
        approved_by: id!(UserId, "owner"),
        decided_at: Timestamp(50),
        expires_at: Timestamp(50 + 300_000),
        revoked_at: None,
    };

    // The ceiling is law even before anything else: UseCredential is not
    // in it yet, so the decision refuses.
    assert!(matches!(
        store.decide_elevation(id!(CommandId, "elevate-early"), lease("elevate-early")),
        Err(StoreError::ElevationCeiling)
    ));
    // A permission with no enforcement consumer refuses outright (this
    // slice's only consumer is the broker's UseCredential gate).
    let mut unconsumed = lease("elevate-beyond");
    unconsumed.permission = Permission::Network;
    assert!(matches!(
        store.decide_elevation(id!(CommandId, "elevate-beyond"), unconsumed),
        Err(StoreError::InvalidElevation)
    ));
    // Raise the ceiling to admit UseCredential (the binding grants stay
    // without it — that is what makes this an ELEVATION).
    let mut raised = store.get_team(&project).unwrap();
    raised
        .access_ceiling
        .grants
        .insert(Permission::UseCredential);
    raised.revision = Revision(1);
    store
        .replace_team(
            id!(CommandId, "team-elev-raise"),
            Some(Revision(0)),
            raised,
            id!(UserId, "owner"),
            Timestamp(45),
        )
        .unwrap();
    // A permission the binding already grants is not an elevation.
    let mut regrant = lease("elevate-regrant");
    regrant.permission = Permission::MutateStream;
    assert!(matches!(
        store.decide_elevation(id!(CommandId, "elevate-regrant"), regrant),
        Err(StoreError::InvalidElevation)
    ));
    // The approval licenses exactly this dispatch, until it does not:
    // expiry is the automatic revocation, checked at every read.
    let approved = lease("elevate-usecredential");
    store
        .decide_elevation(id!(CommandId, "elevate-usecredential"), approved.clone())
        .unwrap();
    assert!(
        store
            .active_elevation(&dispatch_id, &Permission::UseCredential, Timestamp(60))
            .unwrap()
    );
    assert!(
        !store
            .active_elevation(
                &dispatch_id,
                &Permission::UseCredential,
                Timestamp(50 + 300_000)
            )
            .unwrap()
    );
    assert!(
        !store
            .active_elevation(
                &id!(DispatchId, "dispatch-other"),
                &Permission::UseCredential,
                Timestamp(60)
            )
            .unwrap()
    );
    // Sticky manual revocation kills the lease inside its window.
    store
        .revoke_elevation(
            id!(CommandId, "revoke-elev"),
            &project,
            &id!(CommandId, "elevate-usecredential"),
            &id!(UserId, "owner"),
            Timestamp(70),
        )
        .unwrap();
    assert!(
        !store
            .active_elevation(&dispatch_id, &Permission::UseCredential, Timestamp(71))
            .unwrap()
    );
    // Revocation is one-way.
    assert!(matches!(
        store.revoke_elevation(
            id!(CommandId, "revoke-elev-2"),
            &project,
            &id!(CommandId, "elevate-usecredential"),
            &id!(UserId, "owner"),
            Timestamp(72),
        ),
        Err(StoreError::InvalidElevation)
    ));
    // An exact retry replays; different intent under the same id cannot.
    assert!(
        store
            .decide_elevation(id!(CommandId, "elevate-usecredential"), approved)
            .unwrap()
            .replayed
    );
    // Reopen replays the audit over the elevation lifecycle.
    store.integrity_check().unwrap();
    drop(store);
    Store::open(temp.database()).unwrap();
}
