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
            minimum_enforcement: BTreeMap::from([(
                symbiote_domain::Control::Filesystem,
                symbiote_domain::EnforcementStrength::HostEnforced,
            )]),
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
        project.lead.clone(),
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
    // Provider never resolved: the routed Role's binding is missing routing.
    assert_eq!(preparation.provider_connection, None);
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
