//! The Compatibility Dossier a pack publishes: one run per upstream version it
//! was really run against, and the refusals that stop it claiming otherwise.
use std::collections::{BTreeMap, BTreeSet};
use symbiote_domain::*;
// `symbiote_domain` names a canonical `Capability` record of its own (#36); this
// test means the runtime adapter's declared capability.
use symbiote_runtime_sdk::Capability;
use symbiote_runtime_sdk::conformance::{RULES, run_conformance};
use symbiote_runtime_sdk::dossier::*;
use symbiote_runtime_sdk::projection::{CarrierDelivery, ContractSurface};
use symbiote_runtime_sdk::*;

/// The Role's own prose. A refusal names the fact that made the publication
/// untrue, never this text.
const ROLE_PROSE: &str = "You are the engineer. Do not copy this sentence into a dossier.";

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

fn proof(runtime: RuntimeKind, pack: &Pack) -> ProbeEvidence {
    ProbeEvidence {
        artifact: EvidenceId::new("proof").unwrap(),
        adapter_id: AgentRuntimeAdapterId::new(pack.adapter).unwrap(),
        installation: profile(runtime).installation,
        profile_id: RuntimeProfileId::new("profile").unwrap(),
        profile_revision: Revision(1),
        model_id: ModelId::new("model").unwrap(),
        host_id: HostId::new("host").unwrap(),
        adapter_version: pack.adapter_version.into(),
        upstream_version: pack.upstream_version.into(),
        platform: pack.platform.into(),
        observed_at: Timestamp(1),
        expires_at: Timestamp(1000),
    }
}

/// One pack's own identity: the adapter, the driver it drives, and the upstream
/// version, adapter version and platform it is being described at. The evidence
/// a descriptor carries names these, so a fixture that changes one changes it
/// everywhere rather than describing a runtime its own proof does not.
#[derive(Clone, Copy)]
struct Pack {
    adapter: &'static str,
    driver: &'static str,
    upstream_version: &'static str,
    adapter_version: &'static str,
    platform: &'static str,
}

const DEFAULT: Pack = Pack {
    adapter: "adapter",
    driver: "fixture-driver",
    upstream_version: "fixture-1",
    adapter_version: "1.0",
    platform: "linux",
};

/// A structured runtime that declares every carrier surface this crate places
/// and every control the contract can claim.
fn descriptor_at(pack: &Pack) -> RuntimeDescriptor {
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
        adapter_id: AgentRuntimeAdapterId::new(pack.adapter).unwrap(),
        installation: profile(runtime).installation,
        profile_id: RuntimeProfileId::new("profile").unwrap(),
        profile_revision: Revision(1),
        model_id: ModelId::new("model").unwrap(),
        adapter_version: pack.adapter_version.into(),
        upstream_version: pack.upstream_version.into(),
        runtime,
        owner: RuntimeOwner::External {
            driver: HarnessDriverId::new(pack.driver).unwrap(),
        },
        transport: Transport::StructuredRpc,
        tier: IntegrationTier::Structured,
        host_id: HostId::new("host").unwrap(),
        platform: pack.platform.into(),
        capabilities: capabilities
            .into_iter()
            .map(|capability| {
                (
                    capability,
                    Support::Supported {
                        evidence: Box::new(proof(runtime, pack)),
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
                    evidence: proof(runtime, pack),
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

/// The pack every case describes unless it is about another one.
fn descriptor() -> RuntimeDescriptor {
    descriptor_at(&DEFAULT)
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

struct Fixture {
    descriptor: RuntimeDescriptor,
}

impl Fixture {
    fn new(descriptor: RuntimeDescriptor) -> Self {
        Self { descriptor }
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
}

/// The report the suite makes of one runtime over one dispatch.
fn report_of(
    descriptor: &RuntimeDescriptor,
) -> symbiote_runtime_sdk::conformance::ConformanceReport {
    let adapter = Fixture::new(descriptor.clone());
    let dispatch = dispatch();
    run_conformance(&adapter, &[&dispatch])
}

/// The run a pack publishes for the runtime the suite just read.
fn run_of(descriptor: &RuntimeDescriptor) -> Result<ConformanceRun, DossierError> {
    ConformanceRun::of(descriptor, &report_of(descriptor))
}

#[test]
fn a_dossier_publishes_the_runs_it_was_built_from() {
    let declared = descriptor();
    let report = report_of(&declared);
    let run = ConformanceRun::of(&declared, &report).expect("a passing run publishes");
    let dossier = CompatibilityDossier::publish(vec![run.clone()]).expect("one run publishes");

    // The versions and the platform are the descriptor's own, never the pack's.
    assert_eq!(dossier.schema_version, DOSSIER_SCHEMA_VERSION);
    assert_eq!(dossier.adapter.as_str(), "adapter");
    assert_eq!(
        dossier.driver.as_ref().map(HarnessDriverId::as_str),
        Some("fixture-driver")
    );
    assert_eq!(run.upstream_version, "fixture-1");
    assert_eq!(run.adapter_version, "1.0");
    assert_eq!(run.platform, "linux");
    assert_eq!(run.tier, IntegrationTier::Structured);

    // The outcomes are the suite's, by its own names and in its own order, and
    // the rule prose is not here: a name is what a reader looks up.
    assert_eq!(
        run.rules
            .iter()
            .map(|rule| rule.id.as_str())
            .collect::<Vec<_>>(),
        RULES.iter().map(|rule| rule.id).collect::<Vec<_>>()
    );
    assert!(
        run.rules
            .iter()
            .all(|rule| rule.held && rule.failure.is_none())
    );

    // The matrices are the report's rows, unchanged, and the withheld set is
    // the suite's own reading of the same dispatches.
    assert_eq!(run.capabilities, report.capabilities);
    assert_eq!(run.controls, report.controls);
    assert_eq!(run.dispatches, report.dispatches);
    assert_eq!(run.withheld, report.withheld());
    assert!(run.withheld.is_empty(), "this runtime carries every demand");

    // The two ways in: the version list, and a lookup that misses cleanly.
    assert_eq!(dossier.versions(), vec!["fixture-1".to_string()]);
    assert_eq!(dossier.run("fixture-1", "linux"), Some(&run));
    assert_eq!(dossier.run("fixture-2", "linux"), None);
    assert_eq!(dossier.run("fixture-1", "macos"), None);
}

#[test]
fn two_runs_of_one_pack_publish_both_versions() {
    let first = run_of(&descriptor()).expect("the first version publishes");
    let newer = Pack {
        upstream_version: "fixture-2",
        adapter_version: "1.1",
        ..DEFAULT
    };
    let second = run_of(&descriptor_at(&newer)).expect("the second version publishes");
    let dossier =
        CompatibilityDossier::publish(vec![first.clone(), second.clone()]).expect("two publish");

    assert_eq!(
        dossier.versions(),
        vec!["fixture-1".to_string(), "fixture-2".to_string()]
    );
    // One pack, two versions, and the adapter's own version is per run: a pack
    // that updated its adapter between runs says so rather than averaging.
    assert_eq!(dossier.adapter.as_str(), "adapter");
    assert_eq!(dossier.runs[0].adapter_version, "1.0");
    assert_eq!(dossier.runs[1].adapter_version, "1.1");
    assert_eq!(dossier.run("fixture-2", "linux"), Some(&second));
    assert_eq!(dossier.runs.len(), 2);
}

#[test]
fn a_run_of_another_runtime_is_refused() {
    let other = Pack {
        adapter: "other-adapter",
        ..DEFAULT
    };
    let report = report_of(&descriptor_at(&other));
    let refused = ConformanceRun::of(&descriptor(), &report).expect_err("another pack's run");
    assert_eq!(
        refused,
        DossierError::TheRunNamesAnotherRuntime {
            declared: "adapter".into(),
            reported: "other-adapter".into(),
        }
    );
    let text = refused.to_string();
    assert!(text.contains("other-adapter") && text.contains("adapter"));
    assert!(
        !text.contains(ROLE_PROSE),
        "a refusal carries no task content"
    );
}

#[test]
fn a_run_that_failed_a_rule_is_refused_and_names_the_rule() {
    let declared = descriptor();
    let mut report = report_of(&declared);
    // A report whose own outcome was edited is exactly what the refusal is for.
    let broken = "every_projection_covers_every_surface_in_the_modules_order";
    let outcome = report
        .rules
        .iter_mut()
        .find(|outcome| outcome.rule.id == broken)
        .expect("the suite ran that rule");
    outcome.failure = Some("a projection omitted a surface".into());
    let refused = ConformanceRun::of(&declared, &report).expect_err("a failed run publishes");
    assert_eq!(
        refused,
        DossierError::RulesFailed {
            rules: vec![broken.to_string()]
        }
    );
    assert!(refused.to_string().contains(broken));
}

#[test]
fn a_run_with_no_dispatch_is_refused() {
    let declared = descriptor();
    let adapter = Fixture::new(declared.clone());
    let report = run_conformance(&adapter, &[]);
    assert!(report.passed(), "an empty run breaks no rule");
    let refused = ConformanceRun::of(&declared, &report).expect_err("an empty run proves nothing");
    assert_eq!(refused, DossierError::NoDispatches);
    assert!(refused.to_string().contains("no dispatch"));
}

#[test]
fn a_capability_matrix_row_against_the_declaration_is_refused() {
    let declared = descriptor();
    let mut report = report_of(&declared);
    let row = report
        .capabilities
        .iter_mut()
        .find(|row| row.capability == Capability::Tools)
        .expect("the matrix has a tools row");
    assert_eq!(row.declared, CarrierDelivery::Declared {});
    row.declared = CarrierDelivery::Unsupported {};
    let refused = ConformanceRun::of(&declared, &report).expect_err("an edited row publishes");
    assert_eq!(
        refused,
        DossierError::MatrixAgainstTheDeclaration {
            carrier: "tools".into(),
            stated: "Unsupported".into(),
            declared: "Declared".into(),
        }
    );
    assert!(refused.to_string().contains("tools"));
}

#[test]
fn an_enforcement_matrix_row_against_the_declaration_is_refused() {
    let declared = descriptor();
    let mut report = report_of(&declared);
    let row = report
        .controls
        .iter_mut()
        .find(|row| row.control == Control::Filesystem)
        .expect("the matrix has a filesystem row");
    assert_eq!(row.mechanism.as_deref(), Some("host sandbox"));
    row.mechanism = Some("a mechanism the runtime never declared".into());
    let refused = ConformanceRun::of(&declared, &report).expect_err("an edited row publishes");
    assert_eq!(
        refused,
        DossierError::MatrixAgainstTheDeclaration {
            carrier: "filesystem".into(),
            stated: "Some(HostEnforced)".into(),
            declared: "Some(HostEnforced)".into(),
        }
    );
    // The refusal names the carrier, not the fact: the mechanism is the row that
    // disagrees, and the row's own strength is the one the declaration has.
    assert!(refused.to_string().contains("filesystem"));
}

#[test]
fn a_matrix_that_omits_a_carrier_is_refused() {
    let declared = descriptor();
    let mut report = report_of(&declared);
    let omitted = report.capabilities.pop().expect("the matrix has rows");
    let refused = ConformanceRun::of(&declared, &report).expect_err("a short matrix publishes");
    assert_eq!(
        refused,
        DossierError::MatrixOmitsACarrier {
            missing: vec![
                serde_json::to_string(&omitted.capability)
                    .unwrap()
                    .trim_matches('"')
                    .to_string()
            ]
        }
    );
    assert!(refused.to_string().contains("omits"));

    // A control row is held on the same terms: a matrix that drops a claim is
    // hiding it rather than declaring it absent.
    let mut report = report_of(&declared);
    report.controls.pop();
    assert!(matches!(
        ConformanceRun::of(&declared, &report),
        Err(DossierError::MatrixOmitsACarrier { .. })
    ));
}

#[test]
fn a_dossier_with_no_run_certifies_nothing() {
    assert_eq!(
        CompatibilityDossier::publish(Vec::new()).expect_err("an empty dossier"),
        DossierError::NoRuns
    );
    assert!(
        CompatibilityDossier::publish(Vec::new())
            .expect_err("an empty dossier")
            .to_string()
            .contains("certifies nothing")
    );
}

#[test]
fn the_same_version_twice_is_refused() {
    let first = run_of(&descriptor()).expect("the run publishes");
    let refused = CompatibilityDossier::publish(vec![first.clone(), first.clone()])
        .expect_err("a version twice");
    assert_eq!(
        refused,
        DossierError::TheSameVersionTwice {
            upstream_version: "fixture-1".into(),
            platform: "linux".into()
        }
    );
    assert!(refused.to_string().contains("twice"));

    // The same upstream version on another platform is a different claim, and it
    // is published: one row per version and platform, never a merged one.
    let macos = Pack {
        platform: "macos",
        ..DEFAULT
    };
    let other = run_of(&descriptor_at(&macos)).expect("the second platform publishes");
    let dossier = CompatibilityDossier::publish(vec![first, other]).expect("both platforms");
    assert_eq!(dossier.versions(), vec!["fixture-1", "fixture-1"]);
    assert!(dossier.run("fixture-1", "macos").is_some());
}

#[test]
fn a_dossier_naming_two_packs_is_refused() {
    let first = run_of(&descriptor()).expect("the first pack publishes");
    let other = Pack {
        adapter: "other-adapter",
        driver: "other-driver",
        ..DEFAULT
    };
    let second = run_of(&descriptor_at(&other)).expect("the second pack publishes");
    let refused = CompatibilityDossier::publish(vec![first, second]).expect_err("two packs");
    assert_eq!(
        refused,
        DossierError::MoreThanOnePack {
            first: "adapter/fixture-driver".into(),
            other: "other-adapter/other-driver".into(),
        }
    );
    assert!(refused.to_string().contains("other-adapter/other-driver"));
}

/// The dossier is a published document: a pack writes it, a reader parses it,
/// and the schema a third-party adapter builds against names its fields.
#[test]
fn a_dossier_travels_as_a_published_document() {
    let declared = descriptor();
    let dossier = CompatibilityDossier::publish(vec![run_of(&declared).unwrap()]).unwrap();
    let wire = serde_json::to_string(&dossier).unwrap();
    assert_eq!(
        serde_json::from_str::<CompatibilityDossier>(&wire).unwrap(),
        dossier,
        "a dossier round-trips through the wire without losing a field"
    );
    for field in [
        "schema_version",
        "adapter",
        "driver",
        "runs",
        "upstream_version",
        "adapter_version",
        "platform",
        "tier",
        "rules",
        "capabilities",
        "controls",
        "dispatches",
        "withheld",
    ] {
        assert!(wire.contains(field), "the published form names {field}");
    }
    // A published rule row is a name, an outcome and a failure — and nothing
    // else: the suite's own prose for a rule does not travel, so a reworded rule
    // cannot make a published dossier read as a different rule.
    let value = serde_json::to_value(&dossier).unwrap();
    let first = value["runs"][0]["rules"][0]
        .as_object()
        .expect("a published rule row is an object");
    let mut keys: Vec<&str> = first.keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        vec!["failure", "held", "id"],
        "a published rule row carries a name, an outcome and a failure"
    );
    for rule in RULES {
        assert!(
            !wire.contains(rule.holds),
            "the published form carries the prose of {}",
            rule.id
        );
    }

    // And the schema a third-party adapter builds against is the one this crate
    // publishes, so the document a pack writes is the document a reader expects.
    let example = include_str!("../examples/schema.rs");
    assert!(
        example.contains("compatibility_dossier"),
        "the schema example publishes the dossier, which is how a third-party pack learns it"
    );
    let schema = serde_json::to_string(&schemars::schema_for!(CompatibilityDossier)).unwrap();
    for field in ["schema_version", "runs", "upstream_version", "capabilities"] {
        assert!(schema.contains(field), "the schema names {field}");
    }
    for rule in RULES {
        assert!(
            !schema.contains(rule.holds),
            "no schema carries the prose of {}",
            rule.id
        );
    }
}

#[test]
fn the_contract_document_names_the_dossier_and_its_refusals() {
    let contract = include_str!("../../../docs/contracts/runtime-sdk.md");
    for name in [
        "CompatibilityDossier",
        "ConformanceRun",
        "run_conformance",
        "DOSSIER_SCHEMA_VERSION",
    ] {
        assert!(contract.contains(name), "the document does not name {name}");
    }
    for refusal in [
        "TheRunNamesAnotherRuntime",
        "RulesFailed",
        "NoDispatches",
        "MatrixAgainstTheDeclaration",
        "MatrixOmitsACarrier",
        "NoRuns",
        "MoreThanOnePack",
        "TheSameVersionTwice",
    ] {
        assert!(
            contract.contains(refusal),
            "the document does not name {refusal}"
        );
    }
    // The surface vocabulary is the same twelve the projection and the carriage
    // cover: a dossier reports the same places, not a third list of its own.
    assert!(
        contract.contains("`CompatibilityDossier::run` and `versions` are the only ways in"),
        "the document states the only ways into what a pack publishes"
    );
    let surfaces: Vec<ContractSurface> = ContractSurface::ALL.to_vec();
    assert_eq!(surfaces.len(), 12);
}
