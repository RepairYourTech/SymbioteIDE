use super::*;
use symbiote_workforce::*;
fn fixture(store: &mut Store) -> (TeamConfiguration, BindingConfiguration) {
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
    let mut config = binding();
    config.binding.project_id = team.project_id.clone();
    config.binding.role_id = team.members[1].role_id.clone();
    config.primary.access = team.members[1].access.clone();
    config.binding.access = config.primary.access.clone();
    config.validate_team(&team).unwrap();
    (team, config)
}
fn replace(
    store: &mut Store,
    id: &str,
    expected: Option<Revision>,
    config: BindingConfiguration,
    at: u64,
) -> Result<Receipt> {
    store.replace_binding(
        id!(CommandId, id),
        expected,
        config,
        id!(UserId, "owner"),
        Timestamp(at),
    )
}
#[test]
fn bindings_recover_retry_and_survive_later_team_revisions_as_stale_intent() {
    let temp = Temporary::new();
    let mut store = Store::open(temp.database()).unwrap();
    let (mut team, config) = fixture(&mut store);
    let first = replace(&mut store, "initial", None, config.clone(), 11).unwrap();
    assert_eq!(
        store
            .binding_command_timestamp(&id!(CommandId, "initial"))
            .unwrap(),
        Some(Timestamp(11))
    );
    team.revision = Revision(1);
    store
        .replace_team(
            id!(CommandId, "team-update"),
            Some(Revision(0)),
            team.clone(),
            id!(UserId, "owner"),
            Timestamp(12),
        )
        .unwrap();
    drop(store);
    let mut store = Store::open(temp.database()).unwrap();
    assert_eq!(
        store
            .get_binding(&team.project_id, &config.binding.id)
            .unwrap(),
        config
    );
    assert!(config.validate_team(&team).is_err());
    let replay = replace(&mut store, "initial", None, config.clone(), 11).unwrap();
    assert!(replay.replayed);
    assert_eq!(replay.sequence, first.sequence);
    let mut update = config.clone();
    update.binding.revision = Revision(1);
    assert!(matches!(
        replace(&mut store, "stale", Some(Revision(0)), update.clone(), 13),
        Err(StoreError::InvalidBinding)
    ));
    update.team_revision = team.revision;
    replace(&mut store, "update", Some(Revision(0)), update.clone(), 13).unwrap();
    assert!(matches!(
        replace(&mut store, "racing", Some(Revision(0)), update.clone(), 13),
        Err(StoreError::BindingRevisionConflict)
    ));
    assert!(matches!(
        replace(&mut store, "initial", None, config, 14),
        Err(StoreError::IdempotencyConflict)
    ));
    drop(store);
    let store = Store::open(temp.database()).unwrap();
    assert_eq!(
        store
            .get_binding(&team.project_id, &update.binding.id)
            .unwrap(),
        update
    );
}
#[test]
fn binding_identity_role_uniqueness_access_and_attribution_are_enforced() {
    let mut store = Store::memory().unwrap();
    let (team, config) = fixture(&mut store);
    replace(&mut store, "initial", None, config.clone(), 11).unwrap();
    let mut replacement = config.clone();
    replacement.binding.id = id!(BindingId, "other-id");
    assert!(matches!(
        replace(&mut store, "other", None, replacement, 12),
        Err(StoreError::AlreadyExists)
    ));
    let mut changed = config.clone();
    changed.binding.revision = Revision(1);
    changed.binding.role_id = team.lead_role_id.clone();
    assert!(matches!(
        replace(&mut store, "role-move", Some(Revision(0)), changed, 12),
        Err(StoreError::InvalidBinding)
    ));
    let mut changed = config.clone();
    changed.binding.revision = Revision(1);
    assert!(matches!(
        replace(
            &mut store,
            "time-reverse",
            Some(Revision(0)),
            changed.clone(),
            10
        ),
        Err(StoreError::InvalidBinding)
    ));
    changed.primary.access.roots.insert(id!(RootId, "foreign"));
    changed.binding.access = changed.primary.access.clone();
    assert!(matches!(
        replace(&mut store, "foreign", Some(Revision(0)), changed, 12),
        Err(StoreError::InvalidBinding)
    ));
    assert!(matches!(
        store.get_binding(&id!(ProjectId, "other"), &config.binding.id),
        Err(StoreError::NotFound)
    ));
    assert!(matches!(
        store.replace_binding(
            id!(CommandId, "initial"),
            None,
            config.clone(),
            id!(UserId, "other"),
            Timestamp(11)
        ),
        Err(StoreError::IdempotencyConflict)
    ));
    store.integrity_check().unwrap();
    assert_eq!(
        store
            .get_binding(&team.project_id, &config.binding.id)
            .unwrap(),
        config
    );
}
#[test]
fn binding_concurrent_compare_and_swap_has_one_winner() {
    let temp = Temporary::new();
    let mut store = Store::open(temp.database()).unwrap();
    let (_, mut config) = fixture(&mut store);
    replace(&mut store, "initial", None, config.clone(), 11).unwrap();
    config.binding.revision = Revision(1);
    drop(store);
    let barrier = Arc::new(Barrier::new(2));
    let threads: Vec<_> = (0..2)
        .map(|i| {
            let path = temp.database();
            let barrier = barrier.clone();
            let config = config.clone();
            thread::spawn(move || {
                let mut store = Store::open(path).unwrap();
                barrier.wait();
                replace(
                    &mut store,
                    &format!("writer-{i}"),
                    Some(Revision(0)),
                    config,
                    12,
                )
            })
        })
        .collect();
    let results: Vec<_> = threads.into_iter().map(|t| t.join().unwrap()).collect();
    assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter(|r| matches!(r, Err(StoreError::BindingRevisionConflict)))
            .count(),
        1
    );
    Store::open(temp.database())
        .unwrap()
        .integrity_check()
        .unwrap();
}
#[test]
fn binding_tampered_missing_and_forged_materialization_fail_audit() {
    for sql in [
        "DELETE FROM workforce_bindings",
        "UPDATE workforce_bindings SET revision=9",
        "UPDATE workforce_bindings SET actor='forged'",
        "UPDATE workforce_bindings SET updated_at=999",
        "UPDATE workforce_bindings SET body=json_set(body,'$.team_revision',9)",
        "DROP TRIGGER journal_no_update; UPDATE journal SET request='{}' WHERE command_id='initial'; CREATE TRIGGER journal_no_update BEFORE UPDATE ON journal BEGIN SELECT RAISE(ABORT,'append-only journal'); END;",
    ] {
        let temp = Temporary::new();
        let mut store = Store::open(temp.database()).unwrap();
        let (_, config) = fixture(&mut store);
        replace(&mut store, "initial", None, config, 11).unwrap();
        store.connection.execute_batch(sql).unwrap();
        drop(store);
        assert!(Store::open(temp.database()).is_err());
    }
}
#[test]
fn v4_binding_migration_preserves_history_and_rolls_back_corrupt_input() {
    for corrupt in [false, true] {
        let temp = Temporary::new();
        let mut store = Store::open(temp.database()).unwrap();
        let (team, _) = fixture(&mut store);
        let before = store.events(&team.project_id, 0, 10).unwrap();
        store
            .connection
            .execute_batch(
                "DROP TABLE elevations; DROP TABLE model_descriptors; DROP TABLE billing_entitlements; DROP TABLE dispatch_preparations; DROP TABLE provider_connections; DROP TABLE task_leases; DROP TABLE task_dependencies; DROP TABLE work_routes; DROP TABLE workforce_bindings; PRAGMA user_version=4;",
            )
            .unwrap();
        if corrupt {
            store
                .connection
                .execute_batch("UPDATE team_configurations SET revision=9;")
                .unwrap();
        }
        drop(store);
        if corrupt {
            assert!(Store::open(temp.database()).is_err());
            let db = Connection::open(temp.database()).unwrap();
            assert_eq!(
                db.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
                    .unwrap(),
                4
            );
            assert_eq!(
                db.query_row(
                    "SELECT count(*) FROM sqlite_schema WHERE name='workforce_bindings'",
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
                store.get_binding(&team.project_id, &id!(BindingId, "binding")),
                Err(StoreError::NotFound)
            ));
        }
    }
}
fn access() -> AccessSnapshot {
    AccessSnapshot {
        project_id: ProjectId::new("project").unwrap(),
        roots: [RootId::new("root").unwrap()].into(),
        grants: [Permission::ReadRoot, Permission::ExecuteProcess].into(),
        policy_revision: Revision(0),
    }
}
fn candidate() -> StaffingCandidate {
    StaffingCandidate {
        profile: RuntimeProfile {
            id: RuntimeProfileId::new("native").unwrap(),
            revision: Revision(0),
            runtime: RuntimeKind::NativeSymbiote,
            adapter: AgentRuntimeAdapterId::new("native-adapter").unwrap(),
            installation: None,
            provider: ProviderConnectionId::new("provider").unwrap(),
            credential: CredentialReferenceId::new("credential").unwrap(),
            billing_entitlement: BillingEntitlementId::new("billing").unwrap(),
            model: ModelId::new("model").unwrap(),
            eligible_hosts: [HostId::new("host").unwrap()].into(),
        },
        config_identity: "native-profile".into(),
        access: access(),
        tools: ["read".into()].into(),
        skills: BTreeSet::new(),
        secret_scopes: BTreeSet::new(),
        context: ContextPolicy {
            bundle: ContextBundleId::new("context").unwrap(),
            revision: Revision(0),
            max_input_tokens: 100,
            reserved_output_tokens: 100,
        },
        limits: ResourceLimits {
            max_total_tokens: 1000,
            max_wall_time_ms: 1000,
            max_concurrency: 1,
            max_memory_bytes: 1000,
        },
    }
}
fn binding() -> BindingConfiguration {
    let primary = candidate();
    let policy = || PolicyReference {
        id: "policy".into(),
        revision: Revision(0),
    };
    BindingConfiguration {
        schema_version: BINDING_VERSION,
        binding: WorkforceBinding {
            id: BindingId::new("binding").unwrap(),
            revision: Revision(0),
            project_id: ProjectId::new("project").unwrap(),
            role_id: RoleId::new("worker").unwrap(),
            profile_id: primary.profile.id.clone(),
            profile_revision: primary.profile.revision,
            protocol: VersionedProtocol {
                id: ProtocolId::new("protocol").unwrap(),
                revision: Revision(0),
            },
            access: primary.access.clone(),
            required_controls: [Control::Filesystem].into(),
            context: primary.context.clone(),
            required_tools: primary.tools.clone(),
            required_skills: primary.skills.clone(),
            escalation: EscalationPolicy::StopAndRequestHuman,
        },
        team_revision: Revision(0),
        primary,
        fallbacks: vec![],
        policies: WorkforcePolicies {
            environment: policy(),
            worktree: policy(),
            verification: policy(),
            documentation: policy(),
            artifacts: policy(),
            mcp: policy(),
            escalation: policy(),
            root_effort: RootEffort::High,
            local_children: LocalChildPolicy::default(),
            fallback_consent: FallbackConsent::ExplicitRequired,
            minimum_enforcement: BTreeMap::from([(
                Control::Filesystem,
                EnforcementStrength::HostEnforced,
            )]),
            required_capabilities: BTreeSet::new(),
        },
    }
}
