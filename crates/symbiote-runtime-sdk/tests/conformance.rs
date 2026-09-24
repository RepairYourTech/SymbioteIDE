//! The conformance suite: one set of named rules run over an adapter, and the
//! two matrices a report publishes beside them.
use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet};
use symbiote_domain::*;
// `symbiote_domain` names a canonical `Capability` record of its own (#36); this
// test means the runtime adapter's declared capability.
use symbiote_runtime_sdk::Capability;
use symbiote_runtime_sdk::conformance::*;
use symbiote_runtime_sdk::projection::{CarrierDelivery, ContractProjection, ContractSurface};
use symbiote_runtime_sdk::*;

/// The Role's own prose. A failure must name the fact that moved and never this
/// text, which is what "a failure carries no task content" means.
const ROLE_PROSE: &str = "You are the engineer. Do not copy this sentence into a report.";

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

/// A structured runtime that declares every carrier surface this module places
/// and every control the contract can claim.
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

/// An abstract dispatch staffed with every permission the contract can claim a
/// control for, so every surface a contract can demand is demanded here.
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

/// The one fact a fixture breaks. Each variant is one rule's refusal, so a case
/// reads the rule it breaks by name rather than by watching a whole report move.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Breaks {
    Nothing,
    Descriptor,
    CapabilityEvidence,
    EmptyWindow,
    ControlMechanism,
    Header,
    Coverage,
    Carriers,
    Claims,
    Drift,
}

struct Fixture {
    descriptor: RuntimeDescriptor,
    breaks: Breaks,
    calls: Cell<usize>,
}

impl Fixture {
    fn new(breaks: Breaks) -> Self {
        let mut descriptor = descriptor();
        match breaks {
            Breaks::Descriptor => descriptor.sdk_version = 0,
            Breaks::CapabilityEvidence => {
                let Support::Supported { evidence } = descriptor
                    .capabilities
                    .get_mut(&Capability::Tools)
                    .expect("the fixture declares the tools carrier")
                else {
                    unreachable!("the fixture declares it supported")
                };
                evidence.model_id = ModelId::new("other-model").unwrap();
            }
            Breaks::EmptyWindow => {
                let Support::Supported { evidence } = descriptor
                    .capabilities
                    .get_mut(&Capability::Tools)
                    .expect("the fixture declares the tools carrier")
                else {
                    unreachable!("the fixture declares it supported")
                };
                let at = evidence.observed_at;
                evidence.expires_at = at;
            }
            Breaks::ControlMechanism => {
                descriptor
                    .controls
                    .get_mut(&Control::Filesystem)
                    .expect("the fixture declares the filesystem claim")
                    .mechanism = "   ".into();
            }
            _ => {}
        }
        Self {
            descriptor,
            breaks,
            calls: Cell::new(0),
        }
    }
}

impl AgentRuntimeAdapter for Fixture {
    fn descriptor(&self) -> &RuntimeDescriptor {
        &self.descriptor
    }
    fn launch(&mut self, permit: LaunchPermit, at: Timestamp) -> Result<SessionId, AdapterError> {
        permit.validate_at(&self.descriptor, at)?;
        Ok(SessionId::new("session").unwrap())
    }
    fn projection(&self, dispatch: &Dispatch) -> ContractProjection {
        let faithful = ContractProjection::project(dispatch, &self.descriptor);
        let call = self.calls.get();
        self.calls.set(call + 1);
        match self.breaks {
            Breaks::Nothing
            | Breaks::Descriptor
            | Breaks::CapabilityEvidence
            | Breaks::EmptyWindow
            | Breaks::ControlMechanism => faithful,
            Breaks::Header => {
                let mut projection = faithful;
                projection.model = ModelId::new("other-model").unwrap();
                projection
            }
            Breaks::Coverage => {
                let mut projection = faithful;
                projection.surfaces.pop();
                projection
            }
            Breaks::Carriers => {
                let mut projection = faithful;
                let entry = projection
                    .surfaces
                    .iter_mut()
                    .find(|entry| !entry.carried.is_empty())
                    .expect("a contract demands some carrier");
                let capability = entry
                    .carried
                    .keys()
                    .next()
                    .expect("the entry is not empty")
                    .clone();
                entry
                    .carried
                    .insert(capability, CarrierDelivery::Unsupported {});
                projection
            }
            Breaks::Claims => {
                let mut projection = faithful;
                let entry = projection
                    .surfaces
                    .iter_mut()
                    .find(|entry| !entry.preventive.is_empty())
                    .expect("a contract claims some control");
                let control = entry
                    .preventive
                    .keys()
                    .next()
                    .expect("the entry is not empty")
                    .clone();
                entry.preventive.insert(
                    control,
                    symbiote_runtime_sdk::projection::PreventiveDelivery::Missing {},
                );
                projection
            }
            // The first read is the one the suite compares against, the second
            // is the same read with one fact moved, so the suite is asked twice
            // and answers differently.
            Breaks::Drift => {
                if call == 0 {
                    faithful
                } else {
                    let mut projection = faithful;
                    projection.runtime = RuntimeKind::NativeSymbiote;
                    projection
                }
            }
        }
    }
}

/// Runs the suite once over one fixture. A failure must name the rule's own
/// subject, never the role's prose and never a blank string.
fn report_of(breaks: Breaks) -> ConformanceReport {
    let adapter = Fixture::new(breaks);
    let dispatch = dispatch();
    let report = run_conformance(&adapter, &[&dispatch]);
    for outcome in &report.rules {
        if let Some(failure) = &outcome.failure {
            assert!(
                !failure.contains(ROLE_PROSE),
                "a failure carries no task content: {failure}"
            );
            assert!(!failure.trim().is_empty());
        }
    }
    assert_eq!(
        report
            .rules
            .iter()
            .map(|outcome| outcome.rule)
            .collect::<Vec<_>>(),
        RULES.to_vec(),
        "a report lists the suite's rules in the suite's own order"
    );
    report
}

/// The one rule a fixture breaks, asserted by name.
fn breaks_one(breaks: Breaks, rule: &str) {
    assert_ne!(breaks, Breaks::Nothing);
    assert_eq!(report_of(breaks).failures(), vec![rule]);
}

#[test]
fn the_suite_names_every_rule_once_and_no_rule_is_unaccounted_for() {
    let mut ids: Vec<&str> = RULES.iter().map(|rule| rule.id).collect();
    assert!(!ids.is_empty(), "the suite is not empty");
    let named = ids.len();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), named, "every rule has its own name");
    for rule in RULES {
        assert!(!rule.holds.trim().is_empty(), "{} states nothing", rule.id);
    }
    // A rule that nothing evaluates would be a name a report never carries, so
    // the report the suite builds is the same length as the suite itself.
    let report = report_of(Breaks::Nothing);
    assert_eq!(report.rules.len(), RULES.len());
    assert!(report.passed());
    assert!(report.failures().is_empty());
}

#[test]
fn a_conformant_adapter_passes_and_publishes_both_matrices() {
    let report = report_of(Breaks::Nothing);
    assert!(report.passed(), "the faithful adapter passes every rule");
    assert_eq!(report.adapter.as_str(), "adapter");
    assert_eq!(report.tier, IntegrationTier::Structured);

    // One entry per dispatch, each covering the module's own surfaces and
    // withholding none of them.
    assert_eq!(report.dispatches.len(), 1);
    assert_eq!(report.dispatches[0].surfaces, ContractSurface::ALL.len());
    assert!(report.dispatches[0].withheld.is_empty());
    assert!(report.withheld().is_empty());

    // The capability matrix: one row per capability this module places, with the
    // declaration, the demand and what the projection carries for it.
    assert_eq!(report.capabilities.len(), 6);
    let tools = report
        .capability(&Capability::Tools)
        .expect("the tools carrier is placed on a surface");
    assert_eq!(tools.declared, CarrierDelivery::Declared {});
    assert!(tools.demanded, "the binding names a tool");
    assert!(tools.carried);
    let skills = report.capability(&Capability::Skills).unwrap();
    assert!(skills.demanded && skills.carried);
    // The matrix covers what the module places, not the runtime's whole
    // advertised set: a capability no surface carries a claim about is not a row.
    assert!(report.capability(&Capability::Reasoning).is_none());

    // The enforcement matrix: one row per claim, with the one surface that
    // carries it and the strength the descriptor declares.
    assert_eq!(report.controls.len(), 6);
    let filesystem = report.control(&Control::Filesystem).unwrap();
    assert_eq!(filesystem.surface, ContractSurface::PermissionsAndSandbox);
    assert_eq!(filesystem.declared, Some(EnforcementStrength::HostEnforced));
    assert_eq!(filesystem.mechanism.as_deref(), Some("host sandbox"));
    assert!(filesystem.demanded && filesystem.realized);
    assert_eq!(
        report
            .control(&Control::CompletionAuthority)
            .unwrap()
            .surface,
        ContractSurface::VerificationSignals
    );
    assert_eq!(
        report.control(&Control::Credentials).unwrap().surface,
        ContractSurface::SecretReferences
    );
    assert_eq!(
        report.control(&Control::Cancellation).unwrap().surface,
        ContractSurface::Cleanup
    );
    // Every rule's outcome names the rule it is about, and one that held
    // carries no failure at all.
    for outcome in &report.rules {
        assert!(outcome.held());
        assert!(outcome.failure.is_none());
        assert!(report.rule(outcome.rule.id).is_some());
    }
}

/// The suite and its document agree: every rule a report can name is a rule the
/// contract document names, so a rule renamed without its document fails here
/// rather than leaving a document that describes a suite nobody runs.
#[test]
fn the_document_names_every_rule_the_suite_runs() {
    let contract = include_str!("../../../docs/contracts/runtime-sdk.md");
    for rule in RULES {
        assert!(
            contract.contains(rule.id),
            "the contract document does not name {}",
            rule.id
        );
    }
    for name in ["RULES", "run_conformance", "ConformanceReport"] {
        assert!(contract.contains(name), "the document does not name {name}");
    }
}

#[test]
fn a_descriptor_that_does_not_validate_breaks_its_own_rule() {
    breaks_one(Breaks::Descriptor, "descriptor_identity_is_valid");
}

#[test]
fn a_capability_declared_on_evidence_of_another_runtime_breaks_its_rule() {
    breaks_one(
        Breaks::CapabilityEvidence,
        "every_declared_capability_names_evidence_of_its_own",
    );
}

/// A window that opens and closes at the same instant names no observation, and
/// the rule reads that without a clock: freshness is qualification's fact, not
/// this one's.
#[test]
fn a_capability_declared_on_an_empty_window_breaks_its_rule() {
    breaks_one(
        Breaks::EmptyWindow,
        "every_declared_capability_names_evidence_of_its_own",
    );
}

#[test]
fn a_control_with_no_mechanism_breaks_its_rule() {
    breaks_one(
        Breaks::ControlMechanism,
        "every_declared_control_names_evidence_of_its_own",
    );
}

#[test]
fn a_projection_that_names_another_runtime_breaks_its_rule() {
    breaks_one(
        Breaks::Header,
        "the_adapters_projection_names_the_runtime_its_own_descriptor_names",
    );
}

#[test]
fn a_projection_that_drops_a_surface_breaks_its_rule() {
    let report = report_of(Breaks::Coverage);
    assert_eq!(
        report.failures(),
        vec!["every_projection_covers_every_surface_in_the_modules_order"]
    );
    // The failure names the surface that went missing rather than the count it
    // left behind, and the report still names every dispatch it read.
    let failure = report
        .rule("every_projection_covers_every_surface_in_the_modules_order")
        .unwrap()
        .failure
        .as_deref()
        .expect("the rule that broke carries its failure");
    assert!(failure.contains("Cleanup"), "{failure}");
    assert_eq!(
        report.dispatches[0].surfaces,
        ContractSurface::ALL.len() - 1
    );
}

#[test]
fn a_projection_that_reads_a_carrier_against_its_declaration_breaks_its_rule() {
    breaks_one(
        Breaks::Carriers,
        "every_demanded_capability_is_carried_exactly_as_it_is_declared",
    );
}

#[test]
fn a_projection_that_reads_a_claim_against_the_declared_strength_breaks_its_rule() {
    breaks_one(
        Breaks::Claims,
        "every_demanded_control_is_read_at_the_strength_the_descriptor_declares",
    );
}

#[test]
fn a_projection_that_answers_differently_the_second_time_breaks_its_rule() {
    let report = report_of(Breaks::Drift);
    assert_eq!(
        report.failures(),
        vec!["reading_the_same_dispatch_twice_gives_the_same_projection"]
    );
    let failure = report
        .rule("reading_the_same_dispatch_twice_gives_the_same_projection")
        .unwrap()
        .failure
        .as_deref()
        .unwrap();
    assert!(failure.contains("dispatch"), "{failure}");
}

/// A demand the declaration does not answer is a matrix row, not a broken rule:
/// whether this runtime may run this dispatch is qualification's refusal, and a
/// suite that refused it here would be a second owner of that decision.
#[test]
fn an_unanswered_demand_is_a_matrix_row_and_not_a_broken_rule() {
    let mut bare = descriptor();
    bare.controls.remove(&Control::Filesystem);
    bare.capabilities.remove(&Capability::Tools);
    let adapter = Fixture {
        descriptor: bare,
        breaks: Breaks::Nothing,
        calls: Cell::new(0),
    };
    let dispatch = dispatch();
    let report = run_conformance(&adapter, &[&dispatch]);
    assert!(
        report.passed(),
        "the adapter read its own declaration faithfully: {:?}",
        report.failures()
    );

    let filesystem = report.control(&Control::Filesystem).unwrap();
    assert_eq!(filesystem.declared, None);
    assert_eq!(filesystem.mechanism, None);
    assert!(filesystem.demanded);
    assert!(!filesystem.realized);
    let tools = report.capability(&Capability::Tools).unwrap();
    assert_eq!(tools.declared, CarrierDelivery::Absent {});
    assert!(tools.demanded);
    assert!(!tools.carried);

    // And the projection says where the runtime is short, by surface, so a
    // reader who wants the refusal finds it one step away in qualification.
    assert_eq!(
        report.withheld(),
        vec![
            ContractSurface::ToolsAndMcp,
            ContractSurface::PermissionsAndSandbox
        ]
    );
    assert!(
        qualify_dispatch(
            &dispatch,
            &adapter.descriptor,
            &BTreeSet::new(),
            Timestamp(2)
        )
        .is_err()
    );
}
