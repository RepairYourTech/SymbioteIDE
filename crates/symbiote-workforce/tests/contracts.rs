use std::collections::{BTreeMap, BTreeSet};
use symbiote_domain::*;
use symbiote_workforce::*;
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
            max_cpu_millicores: None,
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
fn team() -> TeamConfiguration {
    let lead = RoleId::new("lead").unwrap();
    let worker = RoleId::new("worker").unwrap();
    let mut a = RoleTemplate::LeadOrchestrator.policy(lead.clone(), access());
    let mut b = RoleTemplate::GeneralExecution.policy(worker.clone(), access());
    for member in [&mut a, &mut b] {
        member.context_policy_ref = "context".into();
        member.tool_policy_ref = "tools".into();
        member.skill_policy_ref = "skills".into();
        member.execution_policy_ref = "execution".into();
    }
    a.independent_reviewers.insert(worker);
    b.independent_reviewers.insert(lead.clone());
    TeamConfiguration {
        schema_version: 1,
        project_id: ProjectId::new("project").unwrap(),
        revision: Revision(0),
        lead_role_id: lead,
        members: vec![a, b],
        access_ceiling: access(),
    }
}
#[test]
fn roundtrip_preserves_canonical_identity_and_strict_metadata() {
    let value = binding();
    value.validate_team(&team()).unwrap();
    let json = serde_json::to_string(&value).unwrap();
    assert_eq!(BindingConfiguration::parse(&json).unwrap(), value);
    let mut invalid = value.clone();
    invalid.binding.profile_revision = Revision(1);
    assert_eq!(invalid.validate(), Err(BindingError::ProfileMismatch));
    assert!(
        serde_json::from_str::<BindingConfiguration>(&serde_json::to_string(&invalid).unwrap())
            .is_err()
    );
    assert!(
        serde_json::from_str::<LocalChildPolicy>(r#"{"kind":"disabled","enabled":true}"#).is_err()
    );
}
#[test]
fn fallback_has_its_own_access_context_account_and_budget() {
    let mut value = binding();
    let mut fallback = candidate();
    fallback.profile.id = RuntimeProfileId::new("external").unwrap();
    fallback.profile.runtime = RuntimeKind::ExternalHarness;
    fallback.profile.installation = Some(InstallationId::new("external-install").unwrap());
    fallback.profile.credential = CredentialReferenceId::new("external-credential").unwrap();
    fallback.access.grants.remove(&Permission::ExecuteProcess);
    fallback.tools.clear();
    fallback.context.max_input_tokens = 50;
    fallback.limits.max_total_tokens = 500;
    value.fallbacks.push(fallback.clone());
    value.validate_team(&team()).unwrap();
    assert_eq!(value.candidate(&fallback.profile.id), Some(&fallback));
    assert_ne!(
        value.primary.profile.credential,
        fallback.profile.credential
    );
    value.fallbacks[0].access.grants.insert(Permission::Network);
    assert_eq!(
        value.validate_team(&team()),
        Err(BindingError::PrivilegeEscalation)
    );
}
#[test]
fn shared_profile_does_not_change_role_identity_or_share_permissions() {
    let worker = binding();
    let mut lead = worker.clone();
    lead.binding.id = BindingId::new("lead-binding").unwrap();
    lead.binding.role_id = RoleId::new("lead").unwrap();
    lead.primary
        .access
        .grants
        .remove(&Permission::ExecuteProcess);
    lead.binding.access = lead.primary.access.clone();
    worker.validate_team(&team()).unwrap();
    lead.validate_team(&team()).unwrap();
    assert_eq!(worker.primary.profile.id, lead.primary.profile.id);
    assert_ne!(worker.binding.role_id, lead.binding.role_id);
    assert_ne!(worker.binding.access, lead.binding.access);
}
#[test]
fn stale_team_foreign_root_and_symbolic_secret_paths_are_rejected() {
    let mut value = binding();
    value.team_revision = Revision(1);
    assert_eq!(
        value.validate_team(&team()),
        Err(BindingError::TeamMismatch)
    );
    value = binding();
    value
        .primary
        .access
        .roots
        .insert(RootId::new("foreign").unwrap());
    value.binding.access = value.primary.access.clone();
    assert_eq!(
        value.validate_team(&team()),
        Err(BindingError::PrivilegeEscalation)
    );
    value = binding();
    value.primary.config_identity = "/home/user/.codex".into();
    assert_eq!(value.validate(), Err(BindingError::InvalidReference));
    value = binding();
    value.primary.secret_scopes.insert("secret".into());
    assert_eq!(value.validate(), Err(BindingError::InvalidPolicy));
}
#[test]
fn unknown_or_unbounded_requirements_cannot_be_stored() {
    let mut value = binding();
    value.primary.limits.max_total_tokens = 0;
    assert_eq!(value.validate(), Err(BindingError::ResourceLimit));
    // A declared CPU demand is bounded like every other resource: zero demands
    // nothing and is refused, and the contract's own ceiling is the maximum.
    value = binding();
    value.primary.limits.max_cpu_millicores = Some(0);
    assert_eq!(value.validate(), Err(BindingError::ResourceLimit));
    value = binding();
    value.primary.limits.max_cpu_millicores = Some(64_001);
    assert_eq!(value.validate(), Err(BindingError::ResourceLimit));
    value = binding();
    value.primary.limits.max_cpu_millicores = Some(64_000);
    assert_eq!(value.validate(), Ok(()));
    value = binding();
    value.policies.minimum_enforcement.clear();
    assert_eq!(value.validate(), Err(BindingError::InvalidPolicy));
    value = binding();
    value.fallbacks.push(value.primary.clone());
    assert_eq!(value.validate(), Err(BindingError::DuplicateProfile));
    assert!(BindingConfiguration::parse(&" ".repeat(MAX_BINDING_BYTES + 1)).is_err());
}

#[test]
fn duplicate_enforcement_controls_cannot_silently_replace_requirements() {
    let json = serde_json::to_string(&binding()).unwrap();
    let duplicate = json.replace(
        "\"filesystem\":\"host_enforced\"",
        "\"filesystem\":\"native\",\"filesystem\":\"host_enforced\"",
    );
    assert_ne!(duplicate, json);
    assert!(BindingConfiguration::parse(&duplicate).is_err());
}

#[test]
fn real_prerequisite_checks_never_imply_activation_permission() {
    use symbiote_host_inventory::{
        Fact, HostPulse, PulseProvenance, PulseSource, ResourceObservation, TelemetryMode,
    };
    use symbiote_runtime_sdk::*;
    let b = binding();
    let t = team();
    let resolved = ProviderResolution {
        connection: b.primary.profile.provider.clone(),
        model: b.primary.profile.model.clone(),
        refusal: None,
    };
    let missing = assess_readiness(&b, &t, None, None, None, Timestamp(10));
    assert_eq!(missing.status, PrerequisiteStatus::NotReady);
    assert_eq!(missing.checks[1].result, CheckResult::MissingObservation);
    // The provider prerequisite observes exactly what the registry resolved:
    // no resolution is a missing observation, a refused one is a rejection
    // that names the refusal, and a usable one is satisfied.
    assert_eq!(
        missing.checks[2].prerequisite,
        Prerequisite::ProviderRegistration
    );
    assert_eq!(missing.checks[2].result, CheckResult::MissingObservation);
    assert_eq!(missing.checks[2].provider_refusal, None);
    let p = &b.primary.profile;
    let pulse = HostPulse::new(
        HostId::new("host").unwrap(),
        CommandId::new("pulse").unwrap(),
        Timestamp(1),
        Timestamp(100),
        TelemetryMode::Enabled,
        Fact::Known("linux".into()),
        Fact::Known("x86_64".into()),
        ResourceObservation {
            effective_memory_available_bytes: Fact::Known(10_000),
            effective_memory_limit_bytes: Fact::Known(10_000),
            ..Default::default()
        },
        PulseProvenance {
            source: PulseSource::OperatingSystem,
            probe_version: "test".into(),
        },
    )
    .unwrap();
    let evidence = ProbeEvidence {
        artifact: EvidenceId::new("proof").unwrap(),
        adapter_id: p.adapter.clone(),
        installation: p.installation.clone(),
        profile_id: p.id.clone(),
        profile_revision: p.revision,
        model_id: p.model.clone(),
        host_id: pulse.host_id.clone(),
        adapter_version: "1".into(),
        upstream_version: "1".into(),
        platform: "linux".into(),
        observed_at: Timestamp(1),
        expires_at: Timestamp(100),
    };
    let mut descriptor = RuntimeDescriptor {
        sdk_version: SDK_VERSION,
        adapter_id: p.adapter.clone(),
        installation: p.installation.clone(),
        profile_id: p.id.clone(),
        profile_revision: p.revision,
        model_id: p.model.clone(),
        adapter_version: "1".into(),
        upstream_version: "1".into(),
        runtime: p.runtime,
        owner: RuntimeOwner::SymbioteNative {},
        transport: Transport::NativeLoop,
        tier: IntegrationTier::Detected,
        host_id: pulse.host_id.clone(),
        platform: "linux".into(),
        capabilities: BTreeMap::new(),
        controls: BTreeMap::from([(
            Control::Filesystem,
            ControlSupport {
                strength: EnforcementStrength::HostEnforced,
                mechanism: "fixture-proof".into(),
                evidence,
            },
        )]),
        tools: b.primary.tools.clone(),
        skills: b.primary.skills.clone(),
        context_limits: Some(RuntimeContextLimits {
            context_window_tokens: 1_000,
            max_output_tokens: 100,
        }),
    };
    let report = assess_readiness(
        &b,
        &t,
        Some(&pulse),
        Some(&descriptor),
        Some(&resolved),
        Timestamp(10),
    );
    assert_eq!(report.status, PrerequisiteStatus::ReadyForPreflight);
    assert_eq!(report.activation_pending.len(), 6);
    assert_eq!(report.checks[2].result, CheckResult::Satisfied);
    assert_eq!(report.checks[2].provider_refusal, None);
    assert_eq!(
        report.checks[4].prerequisite,
        Prerequisite::RuntimeResources
    );
    // A registry refusal is a rejection that names itself, whatever the
    // runtime observation says — the report never implies a run that the
    // execution boundary would refuse.
    let refused = ProviderResolution {
        refusal: Some(ProviderRefusal::ExpiredEntitlement),
        ..resolved.clone()
    };
    let denied = assess_readiness(
        &b,
        &t,
        Some(&pulse),
        Some(&descriptor),
        Some(&refused),
        Timestamp(10),
    );
    assert_eq!(denied.status, PrerequisiteStatus::NotReady);
    assert_eq!(denied.checks[2].result, CheckResult::Rejected);
    assert_eq!(
        denied.checks[2].provider_refusal,
        Some(ProviderRefusal::ExpiredEntitlement)
    );
    let mut observed_binding = b.clone();
    observed_binding
        .policies
        .minimum_enforcement
        .insert(Control::Filesystem, EnforcementStrength::ExternallyObserved);
    descriptor
        .controls
        .get_mut(&Control::Filesystem)
        .unwrap()
        .strength = EnforcementStrength::ExternallyObserved;
    assert_eq!(
        assess_readiness(
            &observed_binding,
            &t,
            Some(&pulse),
            Some(&descriptor),
            Some(&resolved),
            Timestamp(10)
        )
        .status,
        PrerequisiteStatus::ReadyForPreflight
    );
    assert_eq!(
        assess_readiness(
            &b,
            &t,
            Some(&pulse),
            Some(&descriptor),
            Some(&resolved),
            Timestamp(10)
        )
        .status,
        PrerequisiteStatus::NotReady
    );
    descriptor
        .controls
        .get_mut(&Control::Filesystem)
        .unwrap()
        .evidence
        .expires_at = Timestamp(9);
    assert_eq!(
        assess_readiness(
            &observed_binding,
            &t,
            Some(&pulse),
            Some(&descriptor),
            Some(&resolved),
            Timestamp(10)
        )
        .status,
        PrerequisiteStatus::NotReady
    );
    descriptor
        .controls
        .get_mut(&Control::Filesystem)
        .unwrap()
        .strength = EnforcementStrength::HostEnforced;
    descriptor
        .controls
        .get_mut(&Control::Filesystem)
        .unwrap()
        .evidence
        .expires_at = Timestamp(100);
    assert_eq!(
        assess_readiness(
            &b,
            &t,
            Some(&pulse),
            Some(&descriptor),
            Some(&resolved),
            Timestamp(100)
        )
        .status,
        PrerequisiteStatus::NotReady
    );
    descriptor.profile_revision = Revision(2);
    assert_eq!(
        assess_readiness(
            &b,
            &t,
            Some(&pulse),
            Some(&descriptor),
            Some(&resolved),
            Timestamp(10)
        )
        .status,
        PrerequisiteStatus::NotReady
    );
    // Every prerequisite classifies the same way: a fact the Host has not
    // observed is an absence, a fact it observed that does not meet the
    // requirement is a rejection, and the two are never swapped.
    let mut observed = descriptor.clone();
    observed.profile_revision = p.revision;
    let report = |pulse: &HostPulse, descriptor: &RuntimeDescriptor| {
        assess_readiness(
            &b,
            &t,
            Some(pulse),
            Some(descriptor),
            Some(&resolved),
            Timestamp(10),
        )
    };
    // Capacity: unobserved effective availability is not a rejection, and an
    // observed availability below the profile's limit is not an absence.
    let mut unobserved = pulse.clone();
    unobserved.resources.effective_memory_available_bytes = Fact::Unknown;
    assert_eq!(
        report(&unobserved, &observed).checks[1].result,
        CheckResult::MissingObservation
    );
    let mut exhausted = pulse.clone();
    exhausted.resources.effective_memory_available_bytes = Fact::Known(1);
    assert_eq!(
        report(&exhausted, &observed).checks[1].result,
        CheckResult::Rejected
    );
    // Runtime capabilities: a capability the observation itself reports as
    // unknown is an absence; one the observation says the runtime lacks is a
    // rejection; evidence that expired is an absence again.
    let mut requiring = b.clone();
    requiring
        .policies
        .required_capabilities
        .insert(Capability::Tools);
    let mut unknown = observed.clone();
    unknown
        .capabilities
        .insert(Capability::Tools, Support::Unknown);
    assert_eq!(
        assess_readiness(
            &requiring,
            &t,
            Some(&pulse),
            Some(&unknown),
            Some(&resolved),
            Timestamp(10)
        )
        .checks[3]
            .result,
        CheckResult::MissingObservation
    );
    assert_eq!(
        assess_readiness(
            &requiring,
            &t,
            Some(&pulse),
            Some(&observed),
            Some(&resolved),
            Timestamp(10)
        )
        .checks[3]
            .result,
        CheckResult::Rejected
    );
    let mut stale = observed.clone();
    stale.capabilities.insert(
        Capability::Tools,
        Support::Supported {
            evidence: Box::new(ProbeEvidence {
                artifact: EvidenceId::new("proof").unwrap(),
                adapter_id: p.adapter.clone(),
                installation: p.installation.clone(),
                profile_id: p.id.clone(),
                profile_revision: p.revision,
                model_id: p.model.clone(),
                host_id: pulse.host_id.clone(),
                adapter_version: "1".into(),
                upstream_version: "1".into(),
                platform: "linux".into(),
                observed_at: Timestamp(1),
                expires_at: Timestamp(2),
            }),
        },
    );
    assert_eq!(
        assess_readiness(
            &requiring,
            &t,
            Some(&pulse),
            Some(&stale),
            Some(&resolved),
            Timestamp(10)
        )
        .checks[3]
            .result,
        CheckResult::MissingObservation
    );
    // Runtime resources: absent context bounds are an absence, observed but
    // insufficient bounds are a rejection.
    let mut unbounded = observed.clone();
    unbounded.context_limits = None;
    assert_eq!(
        report(&pulse, &unbounded).checks[4].result,
        CheckResult::MissingObservation
    );
    let mut narrow = observed.clone();
    narrow.context_limits = Some(RuntimeContextLimits {
        context_window_tokens: 1,
        max_output_tokens: 1,
    });
    assert_eq!(
        report(&pulse, &narrow).checks[4].result,
        CheckResult::Rejected
    );
}

/// A candidate that declares a CPU demand is judged against the effective CPU
/// the Host observed — the CPUs this process may actually run on, capped by any
/// enforced quota — through the same classification every other prerequisite
/// uses: a measurement below the demand is a rejection, and a Host that
/// observed no effective CPU at all is an absence rather than a judgement.
#[test]
fn a_declared_cpu_demand_is_judged_against_the_observed_effective_cpu() {
    use symbiote_host_inventory::{
        Fact, HostPulse, PulseProvenance, PulseSource, ResourceObservation, TelemetryMode,
    };
    let base = binding();
    let t = team();
    let resolved = ProviderResolution {
        connection: base.primary.profile.provider.clone(),
        model: base.primary.profile.model.clone(),
        refusal: None,
    };
    let pulse = HostPulse::new(
        HostId::new("host").unwrap(),
        CommandId::new("pulse").unwrap(),
        Timestamp(1),
        Timestamp(100),
        TelemetryMode::Enabled,
        Fact::Known("linux".into()),
        Fact::Known("x86_64".into()),
        ResourceObservation {
            effective_cpu_millicores: Fact::Known(4_000),
            effective_memory_available_bytes: Fact::Known(10_000),
            effective_memory_limit_bytes: Fact::Known(10_000),
            ..Default::default()
        },
        PulseProvenance {
            source: PulseSource::OperatingSystem,
            probe_version: "test".into(),
        },
    )
    .unwrap();
    let capacity = |pulse: &HostPulse, millicores: Option<u64>| {
        let mut b = binding();
        b.primary.limits.max_cpu_millicores = millicores;
        assess_readiness(&b, &t, Some(pulse), None, Some(&resolved), Timestamp(10)).checks[1]
            .result
            .clone()
    };
    // A candidate that declares no CPU demand asks the Host for none, so the
    // prerequisite is decided by the memory requirement it does declare.
    assert_eq!(capacity(&pulse, None), CheckResult::Satisfied);
    // The measured bound is judged exactly: meeting it is not a rejection.
    assert_eq!(capacity(&pulse, Some(4_000)), CheckResult::Satisfied);
    assert_eq!(capacity(&pulse, Some(4_001)), CheckResult::Rejected);
    // An unobserved effective CPU is an absence even for a candidate that
    // demands one — the requirement never turns a missing measurement into a
    // judgement against the Host.
    let mut blind = pulse.clone();
    blind.resources.effective_cpu_millicores = Fact::Unknown;
    assert_eq!(capacity(&blind, Some(1)), CheckResult::MissingObservation);
    assert_eq!(
        capacity(&blind, None),
        CheckResult::Satisfied,
        "a candidate that declares no CPU demand is unaffected by an unobserved CPU"
    );
}
/// Whether the candidate's declared limits can be bound at all is a
/// prerequisite of its own, decided by the same function the execution boundary
/// uses (`ResourceLimits::process_bound`): a declared CPU rate has no kernel
/// bound here, so the candidate is not usable — whatever capacity the Host
/// measured — and the report names the reason the run will refuse on. A
/// consumable limit is satisfied, so the report cannot promise a run the start
/// would refuse.
#[test]
fn declared_limits_the_host_cannot_bind_are_not_usable() {
    use symbiote_host_inventory::{
        Fact, HostPulse, PulseProvenance, PulseSource, ResourceObservation, TelemetryMode,
    };
    let t = team();
    let resolved = ProviderResolution {
        connection: ProviderConnectionId::new("provider").unwrap(),
        model: ModelId::new("model").unwrap(),
        refusal: None,
    };
    let pulse = HostPulse::new(
        HostId::new("host").unwrap(),
        CommandId::new("pulse").unwrap(),
        Timestamp(1),
        Timestamp(100),
        TelemetryMode::Enabled,
        Fact::Known("linux".into()),
        Fact::Known("x86_64".into()),
        ResourceObservation {
            effective_cpu_millicores: Fact::Known(4_000),
            effective_memory_available_bytes: Fact::Known(10_000),
            effective_memory_limit_bytes: Fact::Known(10_000),
            ..Default::default()
        },
        PulseProvenance {
            source: PulseSource::OperatingSystem,
            probe_version: "test".into(),
        },
    )
    .unwrap();
    let report = |limits: ResourceLimits| {
        let mut b = binding();
        b.primary.limits = limits;
        assess_readiness(&b, &t, Some(&pulse), None, Some(&resolved), Timestamp(10))
    };
    // The fixture's memory ceiling is a bound the Host applies, so the
    // candidate is usable and the check names no refusal.
    let consumable = binding().primary.limits.clone();
    let usable = report(consumable.clone());
    assert_eq!(
        usable.checks[5].prerequisite,
        Prerequisite::EnforceableLimits
    );
    assert_eq!(usable.checks[5].result, CheckResult::Satisfied);
    assert_eq!(usable.checks[5].limit_refusal, None);
    // A declared CPU rate is not: the same candidate is not usable, and the
    // reason is the declared limit activation refuses on.
    let mut demanding = consumable.clone();
    demanding.max_cpu_millicores = Some(1_000);
    let refused = report(demanding.clone());
    assert_eq!(
        refused.checks[5].prerequisite,
        Prerequisite::EnforceableLimits
    );
    assert_eq!(refused.checks[5].result, CheckResult::Rejected);
    assert_eq!(
        refused.checks[5].limit_refusal,
        Some(UnboundedLimit::CpuRate)
    );
    assert_eq!(refused.status, PrerequisiteStatus::NotReady);
    // One decision, not two opinions: the report's refusal IS the domain's own
    // answer for the same declared limits — the function activation calls.
    assert_eq!(
        demanding.process_bound(),
        Err(UnboundedLimit::CpuRate),
        "the report and the execution boundary read the same decision"
    );
    assert_eq!(
        refused.checks[5].limit_refusal,
        demanding.process_bound().err()
    );
}

/// Both lanes answer the ceiling question the same way, because both now
/// honour it: a bindable memory ceiling is `enforceable_limits: satisfied` on
/// the external-harness lane exactly as on the native lane, so the report
/// neither weakens for the lane nor promises a bound the lane's process does
/// not carry (the sandboxed harness applies it; the in-process fixture starts
/// no process to bind).
#[test]
fn a_bindable_ceiling_is_enforceable_on_both_lanes() {
    use symbiote_domain::RuntimeKind;
    let t = team();
    for runtime in [RuntimeKind::NativeSymbiote, RuntimeKind::ExternalHarness] {
        let mut b = binding();
        b.primary.profile.runtime = runtime;
        let report = assess_readiness(&b, &t, None, None, None, Timestamp(10));
        let check = report
            .checks
            .iter()
            .find(|c| c.prerequisite == Prerequisite::EnforceableLimits)
            .expect("every report carries the enforceable-limits check");
        assert_eq!(check.result, CheckResult::Satisfied, "{runtime:?}");
        assert_eq!(check.limit_refusal, None, "{runtime:?}");
    }
}
