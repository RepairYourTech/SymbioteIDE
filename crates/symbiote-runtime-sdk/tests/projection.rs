//! The Workforce Runtime Contract's projection surfaces and the runtime
//! handshake, against the same fixtures the adapter cases use.
use std::collections::{BTreeMap, BTreeSet};
use symbiote_domain::*;
// `symbiote_domain` names a canonical `Capability` record of its own (#36); this
// test means the runtime adapter's declared capability.
use symbiote_runtime_sdk::Capability;
use symbiote_runtime_sdk::projection::*;
use symbiote_runtime_sdk::*;

/// The Role's own prose. A projection must carry the role's identity and never
/// this text, so the case asserts the string is absent from the wire form.
const ROLE_PROSE: &str = "You are the engineer. Do not copy this sentence into a projection.";

fn profile(runtime: RuntimeKind) -> RuntimeProfile {
    RuntimeProfile {
        id: RuntimeProfileId::new("profile").unwrap(),
        revision: Revision(1),
        runtime,
        adapter: AgentRuntimeAdapterId::new("adapter").unwrap(),
        installation: (runtime == RuntimeKind::ExternalHarness)
            .then(|| InstallationId::new("installation").unwrap()),
        provider: ProviderConnectionId::new("provider").unwrap(),
        credential: CredentialReferenceId::new("credential").unwrap(),
        billing_entitlement: BillingEntitlementId::new("billing").unwrap(),
        model: ModelId::new("model").unwrap(),
        eligible_hosts: [HostId::new("host").unwrap()].into(),
    }
}

fn proof(runtime: RuntimeKind) -> ProbeEvidence {
    ProbeEvidence {
        artifact: EvidenceId::new("proof").unwrap(),
        adapter_id: AgentRuntimeAdapterId::new("adapter").unwrap(),
        installation: profile(runtime).installation,
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

/// A structured runtime that declares every carrier surface this module names,
/// at Host-enforced strength for every control the contract can claim.
fn descriptor() -> RuntimeDescriptor {
    let runtime = RuntimeKind::ExternalHarness;
    let capabilities = [
        Capability::StructuredMessages,
        Capability::ContextLimits,
        Capability::Tools,
        Capability::Skills,
        Capability::Instructions,
        Capability::Hooks,
        Capability::Extensions,
        Capability::ProfileIsolation,
    ];
    RuntimeDescriptor {
        sdk_version: SDK_VERSION,
        adapter_id: AgentRuntimeAdapterId::new("adapter").unwrap(),
        installation: profile(runtime).installation,
        profile_id: RuntimeProfileId::new("profile").unwrap(),
        profile_revision: Revision(1),
        model_id: ModelId::new("model").unwrap(),
        adapter_version: "1.0".into(),
        upstream_version: "fixture-1".into(),
        runtime,
        owner: RuntimeOwner::External {
            driver: HarnessDriverId::new("fixture-driver").unwrap(),
        },
        transport: Transport::StructuredRpc,
        tier: IntegrationTier::Structured,
        host_id: HostId::new("host").unwrap(),
        platform: "linux".into(),
        capabilities: capabilities
            .into_iter()
            .map(|capability| {
                (
                    capability,
                    Support::Supported {
                        evidence: Box::new(proof(runtime)),
                    },
                )
            })
            .collect(),
        controls: [
            Control::Filesystem,
            Control::Process,
            Control::Network,
            Control::Credentials,
            Control::Cancellation,
            Control::CompletionAuthority,
        ]
        .into_iter()
        .map(|control| {
            (
                control,
                ControlSupport {
                    strength: EnforcementStrength::HostEnforced,
                    mechanism: "host sandbox".into(),
                    evidence: proof(runtime),
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

/// The same abstract dispatch as the adapter cases, staffed with every
/// permission the contract can claim a control for, so all surfaces that a
/// contract can demand are demanded here.
fn dispatch() -> Dispatch {
    let runtime = RuntimeKind::ExternalHarness;
    let project = ProjectId::new("project").unwrap();
    let root = RootId::new("root").unwrap();
    let task = Task::new(
        TaskId::new("task").unwrap(),
        project.clone(),
        root.clone(),
        RoleId::new("engineer").unwrap(),
        ChangeStreamId::new("stream").unwrap(),
        VersionedTaskContract {
            id: TaskContractId::new("task-contract").unwrap(),
            revision: Revision(7),
        },
    );
    let role = Role {
        id: RoleId::new("engineer").unwrap(),
        project_id: project.clone(),
        revision: Revision(1),
        name: ROLE_PROSE.into(),
        operating_contract: VersionedRoleContract {
            id: RoleContractId::new("role-contract").unwrap(),
            revision: Revision(3),
        },
    };
    let profile = profile(runtime);
    let binding = WorkforceBinding {
        id: BindingId::new("binding").unwrap(),
        revision: Revision(2),
        project_id: project.clone(),
        role_id: role.id.clone(),
        profile_id: profile.id.clone(),
        profile_revision: profile.revision,
        protocol: VersionedProtocol {
            id: ProtocolId::new("protocol").unwrap(),
            revision: Revision(5),
        },
        access: AccessSnapshot {
            project_id: project,
            roots: [root].into(),
            grants: [
                Permission::ReadRoot,
                Permission::MutateStream,
                Permission::ExecuteProcess,
                Permission::Network,
                Permission::UseCredential,
            ]
            .into(),
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
    };
    let host = Host {
        id: HostId::new("host").unwrap(),
        revision: Revision(1),
        device: DeviceId::new("device").unwrap(),
        fabric: None,
        supported_runtimes: vec![runtime],
        controls: [
            Control::Filesystem,
            Control::Process,
            Control::Network,
            Control::Credentials,
            Control::Cancellation,
            Control::CompletionAuthority,
        ]
        .into_iter()
        .map(|control| {
            (
                control,
                EnforcementClaim {
                    strength: EnforcementStrength::HostEnforced,
                    evidence: EvidenceId::new("proof").unwrap(),
                    verified_at: Timestamp(1),
                    expires_at: Timestamp(1000),
                },
            )
        })
        .collect::<BTreeMap<_, _>>(),
    };
    Dispatch::compile(
        DispatchId::new("dispatch").unwrap(),
        RuntimeContractId::new("runtime-contract").unwrap(),
        DispatchInputs {
            task: &task,
            role: &role,
            binding: &binding,
            profile: &profile,
            host: &host,
            limits: &ResourceLimits {
                max_total_tokens: 100_000,
                max_wall_time_ms: 60_000,
                max_concurrency: 1,
                max_memory_bytes: 1 << 30,
                max_cpu_millicores: None,
            },
            minimum_enforcement: &BTreeMap::new(),
            now: Timestamp(2),
        },
    )
    .unwrap()
}

fn session(kind: RuntimeKind) -> Session {
    Session {
        id: SessionId::new("session").unwrap(),
        dispatch_id: DispatchId::new("dispatch").unwrap(),
        kind,
        foreign_reference: None,
    }
}

/// A generic terminal runtime: it launches, and it declares no carrier at all.
fn pty() -> RuntimeDescriptor {
    let mut pty = descriptor();
    pty.transport = Transport::Pty;
    pty.tier = IntegrationTier::TerminalCompatible;
    pty.capabilities.clear();
    pty.controls.clear();
    pty.tools.clear();
    pty.skills.clear();
    pty.context_limits = None;
    pty
}

/// What the runtime says it took, echoing a projection exactly: every demanded
/// control at the strength that was delivered, every demanded capability taken.
fn echo(projection: &ContractProjection, mechanism: &str) -> RuntimeHandshake {
    let mut surfaces = Vec::new();
    for entry in &projection.surfaces {
        surfaces.push(SurfaceReport {
            surface: entry.surface,
            preventive: entry
                .preventive
                .iter()
                .filter_map(|(control, delivery)| {
                    delivery
                        .realized()
                        .map(|strength| (control.clone(), strength))
                })
                .collect(),
            carried: entry
                .carried
                .iter()
                .filter(|(_, delivery)| matches!(delivery, CarrierDelivery::Declared {}))
                .map(|(capability, _)| capability.clone())
                .collect(),
            mechanism: mechanism.into(),
        });
    }
    RuntimeHandshake {
        dispatch: projection.dispatch.clone(),
        surfaces,
    }
}

/// A report that mentions nothing at all: the echo with its surfaces dropped.
fn silent(projection: &ContractProjection) -> RuntimeHandshake {
    RuntimeHandshake {
        dispatch: projection.dispatch.clone(),
        surfaces: Vec::new(),
    }
}

/// Every value a `const` states and every string an `enum` array names, at any
/// depth: how a JSON Schema states the variants of a Rust enum, whether it is
/// written as one enum list or as one tagged alternative per variant.
fn collect_names(value: &serde_json::Value, into: &mut BTreeSet<String>) {
    let string = |value: &serde_json::Value, into: &mut BTreeSet<String>| {
        if let serde_json::Value::String(name) = value {
            into.insert(name.clone());
        }
    };
    match value {
        serde_json::Value::Object(map) => {
            for (key, entry) in map {
                match key.as_str() {
                    "const" => string(entry, into),
                    "enum" => {
                        if let serde_json::Value::Array(names) = entry {
                            for name in names {
                                string(name, into);
                            }
                        }
                    }
                    _ => collect_names(entry, into),
                }
            }
        }
        serde_json::Value::Array(entries) => {
            for entry in entries {
                collect_names(entry, into);
            }
        }
        _ => {}
    }
}

/// One surface's report, for the states that edit a report in place.
fn report_mut(report: &mut RuntimeHandshake, surface: ContractSurface) -> &mut SurfaceReport {
    report
        .surfaces
        .iter_mut()
        .find(|entry| entry.surface == surface)
        .unwrap()
}

#[test]
fn a_projection_covers_every_surface_and_copies_no_contract_content() {
    let dispatch = dispatch();
    let projection = ContractProjection::project(&dispatch, &descriptor());
    assert_eq!(projection.surfaces.len(), ContractSurface::ALL.len());
    assert_eq!(
        projection
            .surfaces
            .iter()
            .map(|entry| entry.surface)
            .collect::<Vec<_>>(),
        ContractSurface::ALL.to_vec(),
        "the entries keep the declared order, and each surface appears once"
    );
    // The projection names the canonical facts by identity and revision.
    assert_eq!(
        projection.references.protocol.revision,
        Revision(5),
        "the protocol revision is the binding's own"
    );
    assert_eq!(projection.references.role_contract.revision, Revision(3));
    assert_eq!(projection.references.task_contract.revision, Revision(7));
    assert_eq!(projection.references.role.as_str(), "engineer");
    assert_eq!(projection.references.binding.as_str(), "binding");
    assert_eq!(projection.references.task.as_str(), "task");
    assert_eq!(projection.references.root.as_str(), "root");
    assert_eq!(projection.references.stream.as_str(), "stream");
    // And never their content: the Role's prose is not in the wire form at all,
    // because no field of a projection holds contract text.
    let wire = serde_json::to_string(&projection).unwrap();
    assert!(
        !wire.contains(ROLE_PROSE),
        "a projection that carried the role's prose would be a second copy of the contract"
    );
    let fields = serde_json::to_value(&projection).unwrap();
    let mut names = fields
        .as_object()
        .unwrap()
        .keys()
        .cloned()
        .collect::<Vec<_>>();
    names.sort();
    assert_eq!(
        names,
        [
            "adapter",
            "adapter_version",
            "dispatch",
            "host",
            "installation",
            "model",
            "platform",
            "profile",
            "profile_revision",
            "references",
            "runtime",
            "surfaces",
            "upstream_version",
        ]
        .map(String::from)
        .to_vec(),
        "the projection's fields are exactly identity, references and surfaces"
    );
}

#[test]
fn each_control_the_contract_claims_lands_on_exactly_one_surface() {
    let dispatch = dispatch();
    let projection = ContractProjection::project(&dispatch, &descriptor());
    let claimed = dispatch
        .contract()
        .enforcement()
        .keys()
        .cloned()
        .collect::<BTreeSet<_>>();
    assert_eq!(
        claimed,
        [
            Control::Filesystem,
            Control::Process,
            Control::Network,
            Control::Credentials,
            Control::Cancellation,
            Control::CompletionAuthority,
        ]
        .into_iter()
        .collect::<BTreeSet<_>>(),
        "this fixture staffs every control a contract can claim"
    );
    let mut seen = BTreeSet::new();
    for entry in &projection.surfaces {
        for control in entry.preventive.keys() {
            assert!(
                seen.insert(control.clone()),
                "{control:?} is carried on more than one surface"
            );
            assert!(
                claimed.contains(control),
                "{:?} reports {control:?}, which the contract does not claim",
                entry.surface
            );
        }
    }
    assert_eq!(
        seen, claimed,
        "every claimed control is carried on exactly one surface"
    );
    // The table covers every control the domain has, once each.
    let mut table = Vec::new();
    for surface in ContractSurface::ALL {
        table.extend(surface.controls().iter().cloned());
    }
    let unique = table.iter().cloned().collect::<BTreeSet<_>>();
    assert_eq!(
        table.len(),
        unique.len(),
        "a control named on two surfaces would let each report the other's delivery"
    );
    assert_eq!(
        unique,
        [
            Control::Filesystem,
            Control::Process,
            Control::Network,
            Control::Credentials,
            Control::Cancellation,
            Control::CompletionAuthority,
        ]
        .into_iter()
        .collect::<BTreeSet<_>>(),
        "every control has one surface that carries it"
    );
}

#[test]
fn a_demand_a_runtime_cannot_carry_is_named_rather_than_omitted() {
    let dispatch = dispatch();
    let projection = ContractProjection::project(&dispatch, &pty());
    assert_eq!(
        projection.surfaces.len(),
        ContractSurface::ALL.len(),
        "a generic terminal runtime still gets a surface for every fact"
    );
    assert_eq!(
        projection.withheld(),
        ContractSurface::ALL.into_iter().collect::<BTreeSet<_>>(),
        "a runtime that declares no carrier withholds every demanded surface"
    );
    let permissions = projection
        .surface(ContractSurface::PermissionsAndSandbox)
        .unwrap();
    assert_eq!(
        permissions.preventive.get(&Control::Filesystem),
        Some(&PreventiveDelivery::Missing {}),
        "a control the descriptor does not list is missing, not absent-supported"
    );
    assert_eq!(PreventiveDelivery::Missing {}.realized(), None);
    let environment = projection.surface(ContractSurface::Environment).unwrap();
    assert_eq!(
        environment.carried.get(&Capability::ProfileIsolation),
        Some(&CarrierDelivery::Absent {}),
        "the worker's environment demand is named even for a runtime that has no profile of its own"
    );
    // The same abstract dispatch, projected onto the structured runtime, has no
    // withheld surface: the difference the two runtimes expose is the point.
    assert!(
        ContractProjection::project(&dispatch, &descriptor())
            .withheld()
            .is_empty()
    );
}

#[test]
fn declared_support_declared_unsupported_declared_unknown_and_absence_stay_four_facts() {
    let dispatch = dispatch();
    let supported = |mutator: fn(&mut RuntimeDescriptor)| {
        let mut descriptor = descriptor();
        mutator(&mut descriptor);
        ContractProjection::project(&dispatch, &descriptor)
            .surface(ContractSurface::WorkforceProtocol)
            .unwrap()
            .carried
            .get(&Capability::Instructions)
            .copied()
    };
    assert_eq!(supported(|_| {}), Some(CarrierDelivery::Declared {}));
    assert_eq!(
        supported(|descriptor| {
            descriptor.capabilities.remove(&Capability::Instructions);
        }),
        Some(CarrierDelivery::Absent {}),
        "a capability the descriptor never mentions is absent, never assumed"
    );
    assert_eq!(
        supported(|descriptor| {
            descriptor
                .capabilities
                .insert(Capability::Instructions, Support::Unknown);
        }),
        Some(CarrierDelivery::Unknown {}),
        "an unestablished capability is unknown"
    );
    assert_eq!(
        supported(|descriptor| {
            descriptor
                .capabilities
                .insert(Capability::Instructions, Support::Unsupported);
        }),
        Some(CarrierDelivery::Unsupported {}),
        "a declared-unsupported capability is its own fact"
    );
}

#[test]
fn a_weaker_control_is_a_named_degradation_and_a_stronger_one_is_delivered() {
    let dispatch = dispatch();
    let mut weaker = descriptor();
    weaker
        .controls
        .get_mut(&Control::Filesystem)
        .unwrap()
        .strength = EnforcementStrength::ExternallyObserved;
    let projection = ContractProjection::project(&dispatch, &weaker);
    let entry = projection
        .surface(ContractSurface::PermissionsAndSandbox)
        .unwrap()
        .preventive
        .get(&Control::Filesystem)
        .unwrap();
    assert_eq!(
        entry,
        &PreventiveDelivery::Degraded {
            claimed: EnforcementStrength::HostEnforced,
            declared: EnforcementStrength::ExternallyObserved,
            mechanism: "host sandbox".into(),
        },
        "an observed mechanism where the contract claims prevention is a degradation, both strengths named"
    );
    assert_eq!(
        entry.realized(),
        Some(EnforcementStrength::ExternallyObserved),
        "a degraded control is realized at its weaker strength, not at the claim"
    );
    assert!(
        !projection
            .withheld()
            .contains(&ContractSurface::PermissionsAndSandbox),
        "withheld means nothing carries the surface; a degradation carries it weakly"
    );
    let mut stronger = descriptor();
    stronger
        .controls
        .get_mut(&Control::Filesystem)
        .unwrap()
        .strength = EnforcementStrength::Native;
    assert_eq!(
        ContractProjection::project(&dispatch, &stronger)
            .surface(ContractSurface::PermissionsAndSandbox)
            .unwrap()
            .preventive
            .get(&Control::Filesystem),
        Some(&PreventiveDelivery::Delivered {
            strength: EnforcementStrength::Native,
            mechanism: "host sandbox".into(),
        }),
        "a runtime stronger than the claim is delivered at what it actually declares"
    );
}

#[test]
fn the_handshake_reconciles_every_surface_and_treats_silence_as_unreported() {
    let dispatch = dispatch();
    let projection = ContractProjection::project(&dispatch, &descriptor());
    let session = session(RuntimeKind::ExternalHarness);
    let outcome = projection
        .reconcile(&session, &echo(&projection, "rpc handshake"))
        .unwrap();
    assert_eq!(outcome.session.as_str(), "session");
    assert_eq!(outcome.surfaces.len(), ContractSurface::ALL.len());
    assert!(outcome.is_exact(), "an echo of the projection is exact");
    assert_eq!(
        outcome
            .surfaces
            .iter()
            .map(|state| state.surface)
            .collect::<Vec<_>>(),
        ContractSurface::ALL.to_vec()
    );
    for state in &outcome.surfaces {
        assert_eq!(
            state.reported_mechanism.as_deref(),
            Some("rpc handshake"),
            "{:?} retained the mechanism the runtime named",
            state.surface
        );
        for (control, carrier) in &state.preventive {
            assert_eq!(
                carrier.agreement,
                Agreement::Exact,
                "{:?}/{control:?} was reported at what was projected",
                state.surface
            );
        }
        for (capability, carrier) in &state.carried {
            assert!(carrier.reported, "{capability:?} was reported taken");
            assert_eq!(carrier.agreement, Agreement::Exact);
        }
    }
    // Silence is not agreement: a runtime that reports nothing for a surface
    // leaves every demanded carrier unreported and the outcome inexact.
    let outcome = projection
        .reconcile(&session, &silent(&projection))
        .unwrap();
    assert!(!outcome.is_exact());
    assert_eq!(outcome.surfaces.len(), ContractSurface::ALL.len());
    let cleanup = outcome.surface(ContractSurface::Cleanup).unwrap();
    assert_eq!(
        cleanup.preventive[&Control::Cancellation].agreement,
        Agreement::Unreported
    );
    assert_eq!(
        outcome
            .surface(ContractSurface::Environment)
            .unwrap()
            .carried[&Capability::ProfileIsolation]
            .agreement,
        Agreement::Unreported
    );
    assert_eq!(
        cleanup.reported_mechanism, None,
        "a surface the runtime never mentioned names no mechanism"
    );
}

#[test]
fn a_report_cannot_upgrade_widen_or_malform_what_was_projected() {
    let dispatch = dispatch();
    let projection = ContractProjection::project(&dispatch, &descriptor());
    let session = session(RuntimeKind::ExternalHarness);
    let refused = |report: &RuntimeHandshake| projection.reconcile(&session, report);
    let mut report = echo(&projection, "rpc handshake");
    // A report names each surface once, so two accounts of one surface have no
    // reading and refuse.
    let mut twice = echo(&projection, "rpc handshake");
    let first = twice.surfaces[0].clone();
    twice.surfaces.push(first);
    assert_eq!(
        refused(&twice),
        Err(AdapterError::ContractMismatch),
        "two reports of one surface have no reading"
    );
    // Stronger than what was delivered.
    report_mut(&mut report, ContractSurface::Cleanup)
        .preventive
        .insert(Control::Cancellation, EnforcementStrength::Native);
    assert_eq!(
        refused(&report),
        Err(AdapterError::ContractMismatch),
        "a report cannot mint enforcement the Host never delivered"
    );
    // Incomparable: observed where emulated was delivered is a mismatch, not a
    // quiet degradation.
    let mut emulated = descriptor();
    emulated
        .controls
        .get_mut(&Control::Cancellation)
        .unwrap()
        .strength = EnforcementStrength::Emulated;
    let emulated_projection = ContractProjection::project(&dispatch, &emulated);
    let mut report = echo(&emulated_projection, "emulated hook");
    report_mut(&mut report, ContractSurface::Cleanup)
        .preventive
        .insert(
            Control::Cancellation,
            EnforcementStrength::ExternallyObserved,
        );
    assert_eq!(
        emulated_projection.reconcile(&session, &report),
        Err(AdapterError::ContractMismatch),
        "observation and emulation are incomparable, and neither is the other"
    );
    // A carrier the surface does not demand.
    let mut report = echo(&projection, "rpc handshake");
    report_mut(&mut report, ContractSurface::HooksAndEvents)
        .preventive
        .insert(Control::Filesystem, EnforcementStrength::Native);
    assert_eq!(refused(&report), Err(AdapterError::ContractMismatch));
    // A report may not claim a carrier the projection found withheld, on either
    // kind of surface: nothing in a terminal runtime carries the claim.
    let pty_projection = ContractProjection::project(&dispatch, &pty());
    let mut claimed = echo(&pty_projection, "terminal");
    report_mut(&mut claimed, ContractSurface::PermissionsAndSandbox)
        .preventive
        .insert(Control::Filesystem, EnforcementStrength::Native);
    assert_eq!(
        pty_projection.reconcile(&session, &claimed),
        Err(AdapterError::ContractMismatch),
        "a terminal runtime cannot report a control the projection found missing"
    );
    let mut claimed = echo(&pty_projection, "terminal");
    report_mut(&mut claimed, ContractSurface::WorkforceProtocol)
        .carried
        .insert(Capability::Instructions);
    assert_eq!(
        pty_projection.reconcile(&session, &claimed),
        Err(AdapterError::ContractMismatch),
        "a terminal runtime cannot report instructions it never declared it reads"
    );
    // Mechanism validity.
    for mechanism in ["", " ", "\u{7}bell"] {
        let mut report = echo(&projection, "rpc handshake");
        report_mut(&mut report, ContractSurface::Cleanup).mechanism = mechanism.into();
        assert_eq!(
            refused(&report),
            Err(AdapterError::InvalidInput),
            "{mechanism:?} is not a mechanism a report may name"
        );
    }
    let mut oversized = echo(&projection, "rpc handshake");
    report_mut(&mut oversized, ContractSurface::Cleanup).mechanism =
        "m".repeat(MECHANISM_MAX_BYTES + 1);
    assert_eq!(refused(&oversized), Err(AdapterError::InvalidInput));
    // The bound is the same one the descriptor's own mechanism is held to.
    let mut long = descriptor();
    long.controls
        .get_mut(&Control::Filesystem)
        .unwrap()
        .mechanism = "m".repeat(MECHANISM_MAX_BYTES + 1);
    assert_eq!(
        qualify_profile(
            &profile(RuntimeKind::ExternalHarness),
            &long,
            &BTreeSet::new(),
            &[Control::Filesystem].into(),
            Timestamp(10)
        ),
        Err(QualificationError::InsufficientControl(Control::Filesystem)),
        "one bound owns both places a mechanism is named"
    );
}

#[test]
fn a_weaker_report_is_named_and_never_smoothed_into_the_projection() {
    let dispatch = dispatch();
    let projection = ContractProjection::project(&dispatch, &descriptor());
    let mut report = echo(&projection, "rpc handshake");
    report_mut(&mut report, ContractSurface::SecretReferences)
        .preventive
        .insert(
            Control::Credentials,
            EnforcementStrength::ExternallyObserved,
        );
    let outcome = projection
        .reconcile(&session(RuntimeKind::ExternalHarness), &report)
        .unwrap();
    let state = &outcome
        .surface(ContractSurface::SecretReferences)
        .unwrap()
        .preventive[&Control::Credentials];
    assert_eq!(state.agreement, Agreement::Weaker);
    assert_eq!(
        state.reported,
        Some(EnforcementStrength::ExternallyObserved)
    );
    assert_eq!(
        state.delivered,
        PreventiveDelivery::Delivered {
            strength: EnforcementStrength::HostEnforced,
            mechanism: "host sandbox".into(),
        },
        "the projection still says what was delivered: the report does not rewrite it"
    );
    assert!(!outcome.is_exact());
}

#[test]
fn the_handshake_refuses_another_dispatch_and_another_runtime() {
    let dispatch = dispatch();
    let projection = ContractProjection::project(&dispatch, &descriptor());
    let report = echo(&projection, "rpc handshake");
    let mut elsewhere = report.clone();
    elsewhere.dispatch = DispatchId::new("another-dispatch").unwrap();
    assert_eq!(
        projection.reconcile(&session(RuntimeKind::ExternalHarness), &elsewhere),
        Err(AdapterError::ContractMismatch)
    );
    let mut other_session = session(RuntimeKind::ExternalHarness);
    other_session.dispatch_id = DispatchId::new("another-dispatch").unwrap();
    assert_eq!(
        projection.reconcile(&other_session, &report),
        Err(AdapterError::SessionMismatch),
        "a handshake belongs to the dispatch its session was launched under"
    );
    assert_eq!(
        projection.reconcile(&session(RuntimeKind::NativeSymbiote), &report),
        Err(AdapterError::SessionMismatch),
        "a native session cannot reconcile an external runtime's projection"
    );
}

#[test]
fn the_wire_forms_are_closed_and_carry_no_canonical_completion() {
    let dispatch = dispatch();
    let projection = ContractProjection::project(&dispatch, &descriptor());
    let report = echo(&projection, "rpc handshake");
    let outcome = projection
        .reconcile(&session(RuntimeKind::ExternalHarness), &report)
        .unwrap();
    fn round_trips<T>(value: &T)
    where
        T: serde::Serialize + serde::de::DeserializeOwned,
    {
        let wire = serde_json::to_value(value).unwrap();
        let back: T = serde_json::from_value(wire.clone()).unwrap();
        assert_eq!(
            wire,
            serde_json::to_value(&back).unwrap(),
            "the wire form round trips"
        );
    }
    round_trips(&projection);
    round_trips(&report);
    round_trips(&outcome);
    // There is no field a completion could arrive in: a handshake is an
    // observation, and the events module owns the completion request.
    let mut completion = serde_json::to_value(&report).unwrap();
    completion
        .as_object_mut()
        .unwrap()
        .insert("completed".into(), serde_json::Value::Bool(true));
    assert!(
        serde_json::from_value::<RuntimeHandshake>(completion).is_err(),
        "a handshake carrying a completion claim does not deserialize"
    );
    let mut task_state = serde_json::to_value(&report).unwrap();
    task_state
        .as_object_mut()
        .unwrap()
        .insert("task_state".into(), serde_json::json!("completed"));
    assert!(serde_json::from_value::<RuntimeHandshake>(task_state).is_err());
    let mut unknown_surface = serde_json::to_value(&report).unwrap();
    unknown_surface["surfaces"][0]["surface"] = serde_json::json!("gpu");
    assert!(
        serde_json::from_value::<RuntimeHandshake>(unknown_surface).is_err(),
        "a surface outside the published vocabulary does not deserialize"
    );
    let mut untyped = serde_json::to_value(&report).unwrap();
    untyped["surfaces"] = serde_json::json!({ "cleanup": {} });
    assert!(
        serde_json::from_value::<RuntimeHandshake>(untyped).is_err(),
        "the surfaces are a list of named entries, not a map of untyped keys"
    );
    assert!(
        serde_json::from_value::<SurfaceReport>(serde_json::json!({
            "preventive": {},
            "carried": [],
            "mechanism": "x",
            "strength": "native"
        }))
        .is_err(),
        "a report's fields are exactly the ones this module names"
    );
    assert!(
        serde_json::from_value::<PreventiveDelivery>(serde_json::json!({
            "kind": "assumed",
            "strength": "native"
        }))
        .is_err(),
        "there is no 'assumed' delivery"
    );
}

#[test]
fn every_capability_a_surface_names_is_one_the_contract_can_demand() {
    let binding = dispatch().contract().binding().clone();
    let always = [
        Capability::Instructions,
        Capability::Hooks,
        Capability::Extensions,
        Capability::ProfileIsolation,
    ];
    for surface in ContractSurface::ALL {
        for capability in surface.capabilities() {
            assert!(
                demanded(capability.clone(), &binding),
                "{surface:?} names {capability:?}, which no contract can demand"
            );
            assert!(
                always.contains(capability)
                    || matches!(capability, Capability::Skills | Capability::Tools),
                "{surface:?} names a capability the demand rule does not decide"
            );
        }
    }
    // A binding that names no resource demands no skill or tool carrier, and a
    // surface with no demand is still reported rather than omitted.
    let mut bare = binding.clone();
    bare.required_skills.clear();
    bare.required_tools.clear();
    for capability in [Capability::Skills, Capability::Tools] {
        assert!(!demanded(capability, &bare));
    }
    for capability in always {
        assert!(demanded(capability, &bare));
    }
}

#[test]
fn the_published_schemas_carry_every_surface_and_the_example_publishes_them() {
    let example = include_str!("../examples/schema.rs");
    for name in [
        "contract_projection",
        "runtime_handshake",
        "handshake_outcome",
    ] {
        assert!(
            example.contains(name),
            "the schema example publishes {name}, which is how a third-party adapter reads the contract"
        );
    }
    let projection = serde_json::to_string(&schemars::schema_for!(ContractProjection)).unwrap();
    for surface in ContractSurface::ALL {
        let wire = serde_json::to_string(&surface).unwrap();
        let name = wire.trim_matches('"');
        assert!(
            projection.contains(name),
            "the published projection schema names the {name} surface"
        );
    }
    // The census of the vocabulary: the names a reader of the schema sees and
    // the names `ALL` covers are one set, so a surface added to the enum without
    // being added to the list — or a list entry that names no variant — fails
    // here instead of leaving a surface a projection silently never covers.
    let mut published = BTreeSet::new();
    collect_names(
        &serde_json::to_value(schemars::schema_for!(ContractSurface)).unwrap(),
        &mut published,
    );
    let declared = ContractSurface::ALL
        .iter()
        .map(|surface| {
            serde_json::to_string(surface)
                .unwrap()
                .trim_matches('"')
                .to_owned()
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(declared.len(), ContractSurface::ALL.len());
    assert_eq!(
        published, declared,
        "the published vocabulary and the covered list are the same surfaces"
    );
    let delivery = serde_json::to_string(&schemars::schema_for!(PreventiveDelivery)).unwrap();
    for name in ["delivered", "degraded", "missing"] {
        assert!(
            delivery.contains(name),
            "the schema names the {name} outcome"
        );
    }
    assert!(
        !serde_json::to_string(&schemars::schema_for!(CarrierDelivery))
            .unwrap()
            .contains("assumed"),
        "no schema offers an assumed carrier"
    );
    let handshake = serde_json::to_string(&schemars::schema_for!(RuntimeHandshake)).unwrap();
    assert!(!handshake.contains("completed") && !handshake.contains("task_state"));
    let outcome = serde_json::to_string(&schemars::schema_for!(HandshakeOutcome)).unwrap();
    assert!(
        outcome.contains("unreported") && outcome.contains("weaker"),
        "the outcome's own vocabulary is published beside the agreement it names"
    );
}

#[test]
fn the_adapter_hooks_read_the_descriptor_the_host_holds() {
    struct Fixture {
        descriptor: RuntimeDescriptor,
    }
    impl AgentRuntimeAdapter for Fixture {
        fn descriptor(&self) -> &RuntimeDescriptor {
            &self.descriptor
        }
        fn launch(
            &mut self,
            permit: LaunchPermit,
            at: Timestamp,
        ) -> Result<SessionId, AdapterError> {
            permit.validate_at(&self.descriptor, at)?;
            Ok(SessionId::new("session").unwrap())
        }
    }
    let dispatch = dispatch();
    let adapter = Fixture {
        descriptor: descriptor(),
    };
    assert_eq!(
        adapter.projection(&dispatch),
        ContractProjection::project(&dispatch, &adapter.descriptor),
        "the hook projects the descriptor the Host holds, and adds no authority"
    );
    assert_eq!(
        adapter.handshake(&session(RuntimeKind::ExternalHarness)),
        Err(AdapterError::Unsupported {
            operation: AdapterOperation::Handshake
        }),
        "a runtime with no handshake says so instead of reporting an empty agreement"
    );
}
