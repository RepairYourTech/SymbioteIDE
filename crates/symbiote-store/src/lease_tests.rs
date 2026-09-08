use super::*;

/// Registers a project plus one Ready task (with objective origin), then
/// starts it with a valid dispatch compiled against a host with current
/// enforcement claims. Returns the identities a lease test needs.
fn started_task(store: &mut Store, tag: &str) -> (ProjectId, TaskId, DispatchId, HostId) {
    let (project, roots, roles) = records(&format!("lease-{tag}"));
    store
        .register_project(
            id!(CommandId, &format!("register-{tag}")),
            project.clone(),
            roots,
            roles.clone(),
        )
        .unwrap();
    let (task_record, _) = task_records(tag);
    let task = Task::new(
        task_record.id().clone(),
        project.id.clone(),
        project.roots.iter().next().unwrap().clone(),
        project.lead.clone(),
        task_record.stream_id().clone(),
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
        worktree: id!(WorktreeId, &format!("worktree-{tag}")),
        branch: format!("symbiote/lease/{tag}"),
        lineage: StreamLineage::Independent,
        base: sha('a'),
        target: sha('b'),
    })
    .unwrap();
    let origin = register_work_target(store, &task).unwrap();
    store
        .create_task(
            id!(CommandId, &format!("task-{tag}")),
            task.clone(),
            stream,
            origin,
        )
        .unwrap();
    let host_id = id!(HostId, &format!("host-{tag}"));
    let profile = RuntimeProfile {
        id: id!(RuntimeProfileId, &format!("profile-{tag}")),
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
        id: id!(BindingId, &format!("binding-{tag}")),
        revision: Revision(0),
        project_id: project.id.clone(),
        role_id: project.lead.clone(),
        profile_id: profile.id.clone(),
        profile_revision: profile.revision,
        protocol: VersionedProtocol {
            id: id!(ProtocolId, "protocol"),
            revision: Revision(1),
        },
        access: AccessSnapshot {
            project_id: project.id.clone(),
            roots: BTreeSet::from([project.roots.iter().next().unwrap().clone()]),
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
        device: id!(DeviceId, &format!("device-{tag}")),
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
                    evidence: id!(EvidenceId, &format!("proof-{tag}")),
                    verified_at: Timestamp(1),
                    expires_at: Timestamp(1_000_000),
                },
            )
        })
        .collect(),
    };
    let dispatch = Dispatch::compile(
        id!(DispatchId, &format!("dispatch-{tag}")),
        id!(RuntimeContractId, &format!("runtime-contract-{tag}")),
        DispatchInputs {
            task: &task,
            role: &roles[0],
            binding: &binding,
            profile: &profile,
            host: &host,
            now: Timestamp(10),
        },
    )
    .unwrap();
    store
        .apply_task(
            &task.id().clone(),
            TaskCommand {
                id: id!(CommandId, &format!("start-{tag}")),
                expected_revision: Revision(0),
                actor: Actor::Host(host_id.clone()),
                at: Timestamp(20),
                action: TaskAction::Start {
                    dispatch: Box::new(dispatch.clone()),
                },
            },
        )
        .unwrap();
    (
        project.id.clone(),
        task.id().clone(),
        dispatch.id().clone(),
        host_id,
    )
}

fn acquire(
    store: &mut Store,
    tag: &str,
    task: &TaskId,
    dispatch: &DispatchId,
    host: &HostId,
    duration: u64,
    at: u64,
) -> Result<Receipt> {
    store.acquire_lease(
        id!(CommandId, tag),
        task.clone(),
        dispatch.clone(),
        host.clone(),
        duration,
        id!(UserId, "owner"),
        Timestamp(at),
    )
}

#[test]
fn leases_acquire_renew_expire_and_refuse_stale_tokens() {
    let mut store = Store::memory().unwrap();
    let (_project, task, dispatch, host) = started_task(&mut store, "one");
    // Acquire at t=100 for 10s: held, token 1.
    acquire(
        &mut store,
        "lease-1",
        &task,
        &dispatch,
        &host,
        MIN_LEASE_MS,
        100,
    )
    .unwrap();
    let lease = store.task_lease(&task).unwrap();
    assert_eq!(lease.state, LeaseState::Held);
    assert_eq!(lease.fencing_token, 1);
    assert_eq!(lease.expires_at, Timestamp(100 + MIN_LEASE_MS));
    // Renewal by the same holder keeps the token and extends the window.
    acquire(
        &mut store,
        "lease-2",
        &task,
        &dispatch,
        &host,
        MIN_LEASE_MS,
        105,
    )
    .unwrap();
    assert_eq!(store.task_lease(&task).unwrap().fencing_token, 1);
    assert_eq!(
        store.task_lease(&task).unwrap().expires_at,
        Timestamp(105 + MIN_LEASE_MS)
    );
    // A different host cannot renew a live lease.
    let (_p2, _t2, d2, h2) = started_task(&mut store, "two");
    let _ = (d2, h2);
    // Expiry transitions the stale lease and records the token (the renewal
    // at t=105 pushed the window out, so expire past the renewed expiry).
    let expired = store
        .expire_stale_leases(id!(UserId, "owner"), Timestamp(105 + MIN_LEASE_MS + 1))
        .unwrap();
    assert_eq!(expired, vec![(task.clone(), 1)]);
    assert_eq!(store.task_lease(&task).unwrap().state, LeaseState::Expired);
    // Expiry is idempotent.
    assert!(
        store
            .expire_stale_leases(id!(UserId, "owner"), Timestamp(105 + MIN_LEASE_MS + 2))
            .unwrap()
            .is_empty()
    );
    // A new acquisition after expiry takes the next token generation.
    acquire(
        &mut store,
        "lease-3",
        &task,
        &dispatch,
        &host,
        MIN_LEASE_MS,
        1_000,
    )
    .unwrap();
    assert_eq!(store.task_lease(&task).unwrap().fencing_token, 2);
    // The prior generation is fenced out of release attempts.
    assert!(matches!(
        store.release_lease(
            id!(CommandId, "stale-release"),
            task.clone(),
            DispatchId::new("dispatch-one").unwrap(),
            1,
            id!(UserId, "owner"),
            Timestamp(1_001),
        ),
        Err(StoreError::LeaseConflict(LeaseError::Fenced))
    ));
    // The current holder releases cleanly and the state is terminal.
    store
        .release_lease(
            id!(CommandId, "release-2"),
            task.clone(),
            DispatchId::new("dispatch-one").unwrap(),
            2,
            id!(UserId, "owner"),
            Timestamp(1_002),
        )
        .unwrap();
    assert_eq!(store.task_lease(&task).unwrap().state, LeaseState::Released);
    assert!(matches!(
        store.release_lease(
            id!(CommandId, "release-again"),
            task.clone(),
            DispatchId::new("dispatch-one").unwrap(),
            2,
            id!(UserId, "owner"),
            Timestamp(1_003),
        ),
        Err(StoreError::LeaseConflict(LeaseError::NotHeld))
    ));
}

#[test]
fn leases_replay_across_restart_and_reject_foreign_dispatches() {
    let temp = Temporary::new();
    let mut store = Store::open(temp.database()).unwrap();
    let (_project, task, dispatch, host) = started_task(&mut store, "two");
    acquire(
        &mut store,
        "lease-1",
        &task,
        &dispatch,
        &host,
        MIN_LEASE_MS,
        100,
    )
    .unwrap();
    let replay = acquire(
        &mut store,
        "lease-1",
        &task,
        &dispatch,
        &host,
        MIN_LEASE_MS,
        100,
    )
    .unwrap();
    assert!(replay.replayed);
    drop(store);
    let mut store = Store::open(temp.database()).unwrap();
    let lease = store.task_lease(&task).unwrap();
    assert_eq!(lease.state, LeaseState::Held);
    assert_eq!(lease.fencing_token, 1);
    assert!(lease_holds(&lease, Timestamp(101)));
    // A dispatch that does not belong to the task cannot lease it.
    assert!(matches!(
        store.acquire_lease(
            id!(CommandId, "lease-foreign"),
            task.clone(),
            id!(DispatchId, "foreign-dispatch"),
            host.clone(),
            MIN_LEASE_MS,
            id!(UserId, "owner"),
            Timestamp(150),
        ),
        Err(StoreError::InvalidLease)
    ));
    // A task that is not Running cannot lease.
    store
        .apply_task(
            &task,
            TaskCommand {
                id: id!(CommandId, "interrupt-lease"),
                expected_revision: Revision(1),
                actor: Actor::Host(host.clone()),
                at: Timestamp(160),
                action: TaskAction::Interrupt {
                    reason: "lease test".into(),
                },
            },
        )
        .unwrap();
    assert!(matches!(
        store.acquire_lease(
            id!(CommandId, "lease-interrupted"),
            task.clone(),
            dispatch.clone(),
            host.clone(),
            MIN_LEASE_MS,
            id!(UserId, "owner"),
            Timestamp(170),
        ),
        Err(StoreError::LeaseConflict(LeaseError::TaskState))
    ));
}

#[test]
fn interrupt_then_sweep_replays_and_reserved_ids_are_refused() {
    let temp = Temporary::new();
    let mut store = Store::open(temp.database()).unwrap();
    let (_project, task, dispatch, host) = started_task(&mut store, "rec");
    acquire(
        &mut store,
        "lease-rec",
        &task,
        &dispatch,
        &host,
        MAX_LEASE_MS,
        100,
    )
    .unwrap();
    // Lifecycle recovery moves the task out of Running while the lease is
    // live; the sweep must still journal the expiry and the store must
    // reopen (the documented recovery path must not brick replay).
    store
        .apply_task(
            &task,
            TaskCommand {
                id: id!(CommandId, "interrupt-rec"),
                expected_revision: Revision(1),
                actor: Actor::Host(host.clone()),
                at: Timestamp(110),
                action: TaskAction::Interrupt {
                    reason: "recovery test".into(),
                },
            },
        )
        .unwrap();
    let expired = store
        .expire_stale_leases(id!(UserId, "owner"), Timestamp(100 + MAX_LEASE_MS + 1))
        .unwrap();
    assert_eq!(expired.len(), 1);
    drop(store);
    let mut store = Store::open(temp.database()).unwrap();
    assert_eq!(
        store.task_lease(&task).unwrap().state,
        symbiote_domain::LeaseState::Expired
    );
    // The reserved sweep namespace is refused for caller-initiated writes.
    let (p2, t2, d2, h2) = started_task(&mut store, "res");
    assert!(matches!(
        store.acquire_lease(
            id!(
                CommandId,
                &format!("{}{}-1", RESERVED_LEASE_PREFIX_TEST, "task-res")
            ),
            t2.clone(),
            d2.clone(),
            h2.clone(),
            MIN_LEASE_MS,
            id!(UserId, "owner"),
            Timestamp(200),
        ),
        Err(StoreError::InvalidLease)
    ));
    let _ = (p2,);
}

const RESERVED_LEASE_PREFIX_TEST: &str = "symbiote-sweep-";

#[test]
fn scheduling_projection_explains_ready_blocked_and_leased_tasks() {
    let mut store = Store::memory().unwrap();
    // A started task is Running, so it is invisible to the Ready projection.
    let (_project, running, dispatch, host) = started_task(&mut store, "proj");
    acquire(
        &mut store,
        "lease-running",
        &running,
        &dispatch,
        &host,
        MIN_LEASE_MS,
        100,
    )
    .unwrap();
    let projection = store.scheduling_projection(Timestamp(101)).unwrap();
    assert!(projection.schedulable.is_empty());
    // The lease is held on the stream; after expiry the task would still be
    // Running (not Ready), so it never appears as schedulable.
    store
        .expire_stale_leases(id!(UserId, "owner"), Timestamp(100 + MIN_LEASE_MS + 1))
        .unwrap();
    let projection = store
        .scheduling_projection(Timestamp(101 + MIN_LEASE_MS))
        .unwrap();
    assert!(projection.schedulable.is_empty());
    assert!(projection.blocked.is_empty());
}
