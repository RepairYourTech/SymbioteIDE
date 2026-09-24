//! The surfaces a binding demands, read onto the runtime the Host observed,
//! before any Task Contract is compiled.
use std::collections::{BTreeMap, BTreeSet};
use symbiote_domain::*;
// `symbiote_domain` names a canonical `Capability` record of its own (#36); this
// test means the runtime adapter's declared capability.
use symbiote_runtime_sdk::Capability;
use symbiote_runtime_sdk::projection::*;
use symbiote_runtime_sdk::*;

const CONTROLS: [Control; 6] = [
    Control::Filesystem,
    Control::Process,
    Control::Network,
    Control::Credentials,
    Control::Cancellation,
    Control::CompletionAuthority,
];

fn proof() -> ProbeEvidence {
    ProbeEvidence {
        artifact: EvidenceId::new("proof").unwrap(),
        adapter_id: AgentRuntimeAdapterId::new("adapter").unwrap(),
        installation: None,
        profile_id: RuntimeProfileId::new("profile").unwrap(),
        profile_revision: Revision(1),
        model_id: ModelId::new("model").unwrap(),
        host_id: HostId::new("host").unwrap(),
        adapter_version: "1.0".into(),
        upstream_version: "fixture-1".into(),
        platform: "linux".into(),
        observed_at: Timestamp(1),
        expires_at: Timestamp(1000),
    }
}

fn profile() -> RuntimeProfile {
    RuntimeProfile {
        id: RuntimeProfileId::new("profile").unwrap(),
        revision: Revision(1),
        runtime: RuntimeKind::NativeSymbiote,
        adapter: AgentRuntimeAdapterId::new("adapter").unwrap(),
        installation: None,
        provider: ProviderConnectionId::new("provider").unwrap(),
        credential: CredentialReferenceId::new("credential").unwrap(),
        billing_entitlement: BillingEntitlementId::new("billing").unwrap(),
        model: ModelId::new("model").unwrap(),
        eligible_hosts: [HostId::new("host").unwrap()].into(),
    }
}

/// A runtime that declares every control the contract can claim and the tools and
/// skills carriers.
fn descriptor() -> RuntimeDescriptor {
    RuntimeDescriptor {
        sdk_version: SDK_VERSION,
        adapter_id: AgentRuntimeAdapterId::new("adapter").unwrap(),
        installation: None,
        profile_id: RuntimeProfileId::new("profile").unwrap(),
        profile_revision: Revision(1),
        model_id: ModelId::new("model").unwrap(),
        adapter_version: "1.0".into(),
        upstream_version: "fixture-1".into(),
        runtime: RuntimeKind::NativeSymbiote,
        owner: RuntimeOwner::SymbioteNative {},
        transport: Transport::NativeLoop,
        tier: IntegrationTier::Structured,
        host_id: HostId::new("host").unwrap(),
        platform: "linux".into(),
        capabilities: [Capability::Tools, Capability::Skills]
            .into_iter()
            .map(|capability| {
                (
                    capability,
                    Support::Supported {
                        evidence: Box::new(proof()),
                    },
                )
            })
            .collect(),
        controls: CONTROLS
            .into_iter()
            .map(|control| {
                (
                    control,
                    ControlSupport {
                        strength: EnforcementStrength::HostEnforced,
                        mechanism: "host sandbox".into(),
                        evidence: proof(),
                    },
                )
            })
            .collect(),
        tools: ["read_file".into()].into(),
        skills: ["review".into()].into(),
        context_limits: Some(RuntimeContextLimits {
            context_window_tokens: 8192,
            max_output_tokens: 2048,
        }),
    }
}

/// A binding that names one tool and one skill, with the grants its own validity
/// requires and no resources beyond those.
fn binding() -> WorkforceBinding {
    WorkforceBinding {
        id: BindingId::new("binding").unwrap(),
        revision: Revision(2),
        project_id: ProjectId::new("project").unwrap(),
        role_id: RoleId::new("engineer").unwrap(),
        profile_id: RuntimeProfileId::new("profile").unwrap(),
        profile_revision: Revision(1),
        protocol: VersionedProtocol {
            id: ProtocolId::new("protocol").unwrap(),
            revision: Revision(5),
        },
        access: AccessSnapshot {
            project_id: ProjectId::new("project").unwrap(),
            roots: [RootId::new("root").unwrap()].into(),
            grants: [Permission::ReadRoot, Permission::MutateStream].into(),
            policy_revision: Revision(1),
        },
        required_controls: BTreeSet::new(),
        context: ContextPolicy {
            bundle: ContextBundleId::new("context").unwrap(),
            revision: Revision(1),
            max_input_tokens: 4096,
            reserved_output_tokens: 1024,
        },
        required_tools: ["read_file".into()].into(),
        required_skills: ["review".into()].into(),
        escalation: EscalationPolicy::ReturnToLead,
    }
}

fn minimums(pairs: &[(Control, EnforcementStrength)]) -> BTreeMap<Control, EnforcementStrength> {
    pairs.iter().cloned().collect()
}

/// The demand is the binding's own, so an entry exists exactly where the binding
/// declares a minimum or names a resource — and the carriage covers every surface
/// in the module's own order.
#[test]
fn the_carriage_covers_every_surface_and_reads_only_the_bindings_own_demand() {
    let binding = binding();
    let carriage = binding_surfaces(
        &binding,
        &minimums(&[
            (Control::Filesystem, EnforcementStrength::HostEnforced),
            (
                Control::CompletionAuthority,
                EnforcementStrength::HostEnforced,
            ),
        ]),
        &BTreeSet::new(),
        &descriptor(),
    );
    assert_eq!(
        carriage
            .surfaces
            .iter()
            .map(|s| s.surface)
            .collect::<Vec<_>>(),
        ContractSurface::ALL.to_vec()
    );
    let with_entries: Vec<ContractSurface> = carriage
        .surfaces
        .iter()
        .filter(|entry| !entry.preventive.is_empty() || !entry.carried.is_empty())
        .map(|entry| entry.surface)
        .collect();
    assert_eq!(
        with_entries,
        vec![
            ContractSurface::Skills,
            ContractSurface::ToolsAndMcp,
            ContractSurface::PermissionsAndSandbox,
            ContractSurface::VerificationSignals,
        ]
    );
    // Tools and skills are demanded by the resources the binding names, and the
    // two controls by its own minimums — each on the one surface that carries it.
    let sandbox = carriage
        .surface(ContractSurface::PermissionsAndSandbox)
        .unwrap();
    assert_eq!(
        sandbox.preventive.keys().cloned().collect::<Vec<_>>(),
        vec![Control::Filesystem]
    );
    assert_eq!(
        carriage
            .surface(ContractSurface::ToolsAndMcp)
            .unwrap()
            .carried
            .keys()
            .cloned()
            .collect::<Vec<_>>(),
        vec![Capability::Tools]
    );
    // The instruction, hook, extension and environment carriers are a *compiled
    // contract's* demand — it places the protocol, the role brief and the task
    // brief on them — so a binding that names no such resource demands none of
    // them here, and this runtime's silence about them withholds nothing.
    for surface in [
        ContractSurface::WorkforceProtocol,
        ContractSurface::RoleOperatingContract,
        ContractSurface::TaskContract,
        ContractSurface::HooksAndEvents,
        ContractSurface::PluginsAndExtensions,
        ContractSurface::Environment,
        ContractSurface::SecretReferences,
        ContractSurface::Cleanup,
    ] {
        let entry = carriage.surface(surface).unwrap();
        assert!(
            entry.preventive.is_empty() && entry.carried.is_empty(),
            "{surface:?} carries nothing this binding demands"
        );
    }
    assert!(carriage.is_complete());
    assert_eq!(carriage.binding.as_str(), "binding");
    assert_eq!(carriage.role.as_str(), "engineer");
}

/// Nothing the runtime advertises becomes a requirement: a binding that declares
/// no minimum and names no resource demands nothing at all, however complete the
/// runtime's declaration is.
#[test]
fn a_runtime_advertising_what_the_binding_never_asked_for_is_not_a_demand() {
    let mut quiet = binding();
    quiet.required_tools.clear();
    quiet.required_skills.clear();
    let carriage = binding_surfaces(&quiet, &BTreeMap::new(), &BTreeSet::new(), &descriptor());
    assert!(carriage.is_complete());
    assert!(
        carriage
            .surfaces
            .iter()
            .all(|entry| entry.preventive.is_empty() && entry.carried.is_empty())
    );
    // The runtime declares six controls and two carriers; none of them is read.
    let empty = descriptor();
    assert_eq!(empty.controls.len(), 6);
}

/// A control the runtime never declares is withheld, and the refusal names the
/// surface that would have carried it rather than the control alone.
#[test]
fn a_control_the_runtime_never_declares_withholds_its_surface() {
    let mut missing = descriptor();
    missing.controls.remove(&Control::Filesystem);
    let carriage = binding_surfaces(
        &binding(),
        &minimums(&[(Control::Filesystem, EnforcementStrength::HostEnforced)]),
        &BTreeSet::new(),
        &missing,
    );
    assert_eq!(
        carriage.withheld().into_iter().collect::<Vec<_>>(),
        vec![ContractSurface::PermissionsAndSandbox]
    );
    assert!(!carriage.is_complete());
    assert!(matches!(
        carriage
            .surface(ContractSurface::PermissionsAndSandbox)
            .unwrap()
            .preventive[&Control::Filesystem],
        PreventiveDelivery::Missing {}
    ));
}

/// A weaker mechanism than the binding's minimum is a degradation with both
/// strengths kept, and it is **not** withheld: a minimum is what the binding
/// requires, and whether the runtime meets it is the capability check's refusal
/// rather than a second one read from this set.
#[test]
fn a_control_carried_weaker_than_the_minimum_keeps_both_strengths() {
    let mut observed = descriptor();
    observed
        .controls
        .get_mut(&Control::Filesystem)
        .unwrap()
        .strength = EnforcementStrength::ExternallyObserved;
    let carriage = binding_surfaces(
        &binding(),
        &minimums(&[(Control::Filesystem, EnforcementStrength::HostEnforced)]),
        &BTreeSet::new(),
        &observed,
    );
    assert!(matches!(
        carriage
            .surface(ContractSurface::PermissionsAndSandbox)
            .unwrap()
            .preventive[&Control::Filesystem],
        PreventiveDelivery::Degraded {
            claimed: EnforcementStrength::HostEnforced,
            declared: EnforcementStrength::ExternallyObserved,
            ..
        }
    ));
    // A degradation is not a withholding: the surface is named degraded with
    // both strengths, and whether a minimum is met is the capability check's
    // refusal — the carriage reports carriage, one fact per owner.
    assert!(carriage.withheld().is_empty());
    assert!(carriage.is_complete());
    // The same runtime satisfies a minimum it does realize.
    let carriage = binding_surfaces(
        &binding(),
        &minimums(&[(Control::Filesystem, EnforcementStrength::ExternallyObserved)]),
        &BTreeSet::new(),
        &observed,
    );
    assert!(carriage.is_complete());
}

/// The tools and skills the binding names are demanded as carriers — the dispatch
/// boundary requires the same capabilities for exactly those resources — and a
/// declaration that omits them withholds the surfaces that would carry them.
#[test]
fn the_resources_the_binding_names_demand_their_carrier_surfaces() {
    let mut bare = descriptor();
    bare.capabilities.remove(&Capability::Tools);
    bare.capabilities.remove(&Capability::Skills);
    let carriage = binding_surfaces(&binding(), &BTreeMap::new(), &BTreeSet::new(), &bare);
    assert_eq!(
        carriage.withheld().into_iter().collect::<Vec<_>>(),
        vec![ContractSurface::Skills, ContractSurface::ToolsAndMcp]
    );
    // A binding that names no resource demands neither, on the same runtime.
    let mut quiet = binding();
    quiet.required_tools.clear();
    quiet.required_skills.clear();
    assert!(
        binding_surfaces(&quiet, &BTreeMap::new(), &BTreeSet::new(), &bare).is_complete(),
        "a binding naming no resource demands no carrier"
    );
    // A capability the policy requires is demanded even where the binding names
    // no resource at all.
    let mut policy = BTreeSet::new();
    policy.insert(Capability::Hooks);
    let mut no_hooks = descriptor();
    no_hooks.capabilities.remove(&Capability::Hooks);
    assert_eq!(
        binding_surfaces(&quiet, &BTreeMap::new(), &policy, &no_hooks)
            .withheld()
            .into_iter()
            .collect::<Vec<_>>(),
        vec![ContractSurface::HooksAndEvents]
    );
}

/// The carriage names the runtime it was read from, and carries no task or
/// dispatch identity: it is a statement about a binding and an observation, not
/// about work that has been staffed.
#[test]
fn the_carriage_names_the_observation_and_no_work() {
    let carriage = binding_surfaces(
        &binding(),
        &minimums(&[(Control::Filesystem, EnforcementStrength::HostEnforced)]),
        &BTreeSet::new(),
        &descriptor(),
    );
    assert_eq!(carriage.adapter.as_str(), "adapter");
    assert_eq!(carriage.installation, None);
    assert_eq!(carriage.profile.as_str(), "profile");
    assert_eq!(carriage.profile_revision, Revision(1));
    assert_eq!(carriage.model.as_str(), "model");
    assert_eq!(carriage.host.as_str(), "host");
    assert_eq!(carriage.runtime, RuntimeKind::NativeSymbiote);
    assert_eq!(carriage.adapter_version, "1.0");
    assert_eq!(carriage.upstream_version, "fixture-1");
    assert_eq!(carriage.platform, "linux");
    // The wire form is closed and carried the surface vocabulary a third-party
    // operator reads.
    let wire = serde_json::to_string(&carriage).unwrap();
    assert!(wire.contains("permissions_and_sandbox") && wire.contains("tools_and_mcp"));
    // The only work-shaped names in the wire form are the surfaces a Task
    // Contract would be carried on; no dispatch, task or session identity is
    // here at all.
    assert!(!wire.contains("\"dispatch\"") && !wire.contains("\"task\""));
    let mut unknown: serde_json::Value = serde_json::from_str(&wire).unwrap();
    unknown["invented"] = serde_json::json!(true);
    assert!(serde_json::from_value::<BindingSurfaces>(unknown).is_err());
}

/// The projection and the carriage withhold by one rule: the same runtime, the
/// same demanded control, and both report the same surface.
#[test]
fn the_projection_and_the_carriage_withhold_by_the_same_rule() {
    let binding = binding();
    let mut missing = descriptor();
    missing.controls.remove(&Control::Filesystem);
    let host = Host {
        id: HostId::new("host").unwrap(),
        revision: Revision(1),
        device: DeviceId::new("device").unwrap(),
        fabric: None,
        supported_runtimes: vec![RuntimeKind::NativeSymbiote],
        controls: [
            Control::Filesystem,
            Control::Cancellation,
            Control::CompletionAuthority,
        ]
        .iter()
        .map(|control| {
            (
                control.clone(),
                EnforcementClaim {
                    strength: EnforcementStrength::HostEnforced,
                    evidence: EvidenceId::new("proof").unwrap(),
                    verified_at: Timestamp(1),
                    expires_at: Timestamp(1000),
                },
            )
        })
        .collect(),
    };
    let task = Task::new(
        TaskId::new("task").unwrap(),
        ProjectId::new("project").unwrap(),
        RootId::new("root").unwrap(),
        RoleId::new("engineer").unwrap(),
        ChangeStreamId::new("stream").unwrap(),
        VersionedTaskContract {
            id: TaskContractId::new("task-contract").unwrap(),
            revision: Revision(7),
        },
    );
    let role = Role {
        id: RoleId::new("engineer").unwrap(),
        project_id: ProjectId::new("project").unwrap(),
        revision: Revision(1),
        name: "the engineer".into(),
        operating_contract: VersionedRoleContract {
            id: RoleContractId::new("role-contract").unwrap(),
            revision: Revision(3),
        },
    };
    let limits = ResourceLimits {
        max_total_tokens: 100_000,
        max_wall_time_ms: 60_000,
        max_concurrency: 1,
        max_memory_bytes: 1 << 30,
        max_cpu_millicores: None,
    };
    let dispatch = Dispatch::compile(
        DispatchId::new("dispatch").unwrap(),
        RuntimeContractId::new("runtime-contract").unwrap(),
        DispatchInputs {
            task: &task,
            role: &role,
            binding: &binding,
            profile: &profile(),
            host: &host,
            limits: &limits,
            minimum_enforcement: &BTreeMap::new(),
            now: Timestamp(2),
        },
    )
    .unwrap();
    let carriage = binding_surfaces(
        &binding,
        &minimums(&[(Control::Filesystem, EnforcementStrength::HostEnforced)]),
        &BTreeSet::new(),
        &missing,
    );
    let projection = ContractProjection::project(&dispatch, &missing);
    // The contract claims Filesystem, Cancellation and CompletionAuthority from
    // the Host's own advertised controls; the minimums declare Filesystem alone.
    // One rule, so the same missing control withholds the same surface in both.
    assert_eq!(
        carriage.withheld().into_iter().collect::<Vec<_>>(),
        vec![ContractSurface::PermissionsAndSandbox]
    );
    assert!(
        projection
            .withheld()
            .contains(&ContractSurface::PermissionsAndSandbox)
    );
    // The views differ in what they *demand*, never in what a delivery means: a
    // compiled contract also places the protocol, the role brief and the task
    // brief on the instruction surfaces and demands the hook, extension and
    // environment carriers, so those are withheld by the projection and demanded
    // by no binding.
    assert_eq!(
        projection
            .withheld()
            .difference(&carriage.withheld())
            .cloned()
            .collect::<Vec<_>>(),
        vec![
            ContractSurface::WorkforceProtocol,
            ContractSurface::RoleOperatingContract,
            ContractSurface::TaskContract,
            ContractSurface::HooksAndEvents,
            ContractSurface::PluginsAndExtensions,
            ContractSurface::Environment,
        ]
    );
    assert_eq!(
        carriage
            .surface(ContractSurface::PermissionsAndSandbox)
            .unwrap()
            .preventive[&Control::Filesystem],
        projection
            .surface(ContractSurface::PermissionsAndSandbox)
            .unwrap()
            .preventive[&Control::Filesystem]
    );
}
