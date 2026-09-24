//! The Compatibility Dossier a pack publishes: one run per upstream version it
//! was really run against, and the refusals that stop it claiming otherwise.
use std::collections::{BTreeMap, BTreeSet};
use symbiote_domain::*;
// `symbiote_domain` names a canonical `Capability` record of its own (#36); this
// test means the runtime adapter's declared capability.
use symbiote_runtime_sdk::Capability;
use symbiote_runtime_sdk::conformance::{RULES, run_conformance};
use symbiote_runtime_sdk::dossier::*;
use symbiote_runtime_sdk::projection::{ContractProjection, ContractSurface};
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
    /// Whether the adapter's own projection agrees with the descriptor it holds.
    /// A pack whose projection disagrees with its own declaration fails the
    /// suite, and that is how a refusal is reached here: by a real run rather
    /// than by a record somebody edited.
    truthful: bool,
}

impl Fixture {
    fn new(descriptor: RuntimeDescriptor) -> Self {
        Self {
            descriptor,
            truthful: true,
        }
    }

    fn lying(descriptor: RuntimeDescriptor) -> Self {
        Self {
            descriptor,
            truthful: false,
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
        if self.truthful {
            return faithful;
        }
        // A projection of another runtime than the one the Host holds: the
        // suite's own header rule is what refuses this, by name.
        let mut projection = faithful;
        projection.runtime = RuntimeKind::NativeSymbiote;
        projection
    }
}

/// The suite's own report of one runtime over one dispatch, so a case can hold a
/// published run against what the suite actually read.
fn report_of(
    descriptor: &RuntimeDescriptor,
) -> symbiote_runtime_sdk::conformance::ConformanceReport {
    let adapter = Fixture::new(descriptor.clone());
    let dispatch = dispatch();
    run_conformance(&adapter, &[&dispatch])
}

/// One runtime, one dispatch, published.
fn publish_one(descriptor: &RuntimeDescriptor) -> Result<CompatibilityDossier, DossierError> {
    let adapter = Fixture::new(descriptor.clone());
    let dispatch = dispatch();
    CompatibilityDossier::publish(vec![PackRun {
        adapter: &adapter,
        dispatches: vec![&dispatch],
    }])
}

#[test]
fn a_dossier_publishes_the_runs_it_was_built_from() {
    let declared = descriptor();
    let report = report_of(&declared);
    let dossier = publish_one(&declared).expect("a passing run publishes");
    let run = &dossier.runs[0];

    // The versions, the platform, the tier and the driver are the descriptor's
    // own: a caller has no parameter to publish anything else with.
    assert_eq!(dossier.schema_version, DOSSIER_SCHEMA_VERSION);
    assert_eq!(dossier.adapter.as_str(), "adapter");
    assert_eq!(
        dossier.driver.as_ref().map(HarnessDriverId::as_str),
        Some("fixture-driver")
    );
    assert_eq!(run.adapter.as_str(), "adapter");
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

    // The matrices, the dispatch reads and the withheld set are the report's
    // own rows, unchanged.
    assert_eq!(run.capabilities, report.capabilities);
    assert_eq!(run.controls, report.controls);
    assert_eq!(run.dispatches, report.dispatches);
    assert_eq!(run.withheld, report.withheld());
    assert!(run.withheld.is_empty(), "this runtime carries every demand");

    // The two ways in: the version list, and a lookup that misses cleanly.
    assert_eq!(dossier.versions(), vec!["fixture-1".to_string()]);
    assert_eq!(dossier.run("fixture-1", "linux"), Some(run));
    assert_eq!(dossier.run("fixture-2", "linux"), None);
    assert_eq!(dossier.run("fixture-1", "macos"), None);
}

#[test]
fn two_runs_of_one_pack_publish_both_versions() {
    let first_adapter = Fixture::new(descriptor());
    let newer = Pack {
        upstream_version: "fixture-2",
        adapter_version: "1.1",
        ..DEFAULT
    };
    let second_adapter = Fixture::new(descriptor_at(&newer));
    let dispatch = dispatch();
    let dossier = CompatibilityDossier::publish(vec![
        PackRun {
            adapter: &first_adapter,
            dispatches: vec![&dispatch],
        },
        PackRun {
            adapter: &second_adapter,
            dispatches: vec![&dispatch],
        },
    ])
    .expect("two versions publish");

    assert_eq!(
        dossier.versions(),
        vec!["fixture-1".to_string(), "fixture-2".to_string()]
    );
    // One pack, two versions, and the adapter's own version is per run: a pack
    // that updated its adapter between runs says so rather than averaging.
    assert_eq!(dossier.adapter.as_str(), "adapter");
    assert_eq!(dossier.runs[0].adapter_version, "1.0");
    assert_eq!(dossier.runs[1].adapter_version, "1.1");
    assert!(dossier.run("fixture-2", "linux").is_some());
    assert_eq!(dossier.runs.len(), 2);
}

#[test]
fn a_run_that_failed_a_rule_is_refused_and_names_the_rule() {
    // An adapter whose own projection disagrees with its own declaration: the
    // suite refuses it by name, and the dossier refuses to publish the run.
    let adapter = Fixture::lying(descriptor());
    let dispatch = dispatch();
    let refused = CompatibilityDossier::publish(vec![PackRun {
        adapter: &adapter,
        dispatches: vec![&dispatch],
    }])
    .expect_err("a failed run publishes");
    assert_eq!(
        refused,
        DossierError::RulesFailed {
            rules: vec![
                "the_adapters_projection_names_the_runtime_its_own_descriptor_names".to_string()
            ]
        }
    );
    let text = refused.to_string();
    assert!(
        text.contains("the_adapters_projection_names_the_runtime_its_own_descriptor_names"),
        "{text}"
    );
    assert!(
        !text.contains(ROLE_PROSE),
        "a refusal carries no task content"
    );
}

#[test]
fn a_run_with_no_dispatch_is_refused() {
    let adapter = Fixture::new(descriptor());
    let refused = CompatibilityDossier::publish(vec![PackRun {
        adapter: &adapter,
        dispatches: Vec::new(),
    }])
    .expect_err("an empty run proves nothing");
    assert_eq!(refused, DossierError::NoDispatches);
    assert!(refused.to_string().contains("no dispatch"));
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
    let first = Fixture::new(descriptor());
    let second = Fixture::new(descriptor());
    let dispatch = dispatch();
    let refused = CompatibilityDossier::publish(vec![
        PackRun {
            adapter: &first,
            dispatches: vec![&dispatch],
        },
        PackRun {
            adapter: &second,
            dispatches: vec![&dispatch],
        },
    ])
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
    let other = Fixture::new(descriptor_at(&macos));
    let dossier = CompatibilityDossier::publish(vec![
        PackRun {
            adapter: &first,
            dispatches: vec![&dispatch],
        },
        PackRun {
            adapter: &other,
            dispatches: vec![&dispatch],
        },
    ])
    .expect("both platforms publish");
    assert_eq!(dossier.versions(), vec!["fixture-1", "fixture-1"]);
    assert!(dossier.run("fixture-1", "macos").is_some());
    assert!(dossier.run("fixture-1", "linux").is_some());
}

#[test]
fn a_dossier_naming_two_packs_is_refused() {
    let first = Fixture::new(descriptor());
    let other = Pack {
        adapter: "other-adapter",
        driver: "other-driver",
        ..DEFAULT
    };
    let second = Fixture::new(descriptor_at(&other));
    let dispatch = dispatch();
    let refused = CompatibilityDossier::publish(vec![
        PackRun {
            adapter: &first,
            dispatches: vec![&dispatch],
        },
        PackRun {
            adapter: &second,
            dispatches: vec![&dispatch],
        },
    ])
    .expect_err("two packs");
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
    let dossier = publish_one(&declared).expect("the run publishes");
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
    // else: the suite's own prose for a rule does not travel, so a reworded
    // rule cannot make a published dossier read as a different rule.
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
        "PackRun",
        "run_conformance",
        "DOSSIER_SCHEMA_VERSION",
    ] {
        assert!(contract.contains(name), "the document does not name {name}");
    }
    for refusal in [
        "RulesFailed",
        "NoDispatches",
        "NoRuns",
        "MoreThanOnePack",
        "TheSameVersionTwice",
    ] {
        assert!(
            contract.contains(refusal),
            "the document does not name {refusal}"
        );
    }
    // The two ways in are the only ways in, and a version absent from them is a
    // version the dossier says nothing about.
    assert!(
        contract.contains("`CompatibilityDossier::run` and `versions` are the only ways in"),
        "the document states the only ways into what a pack publishes"
    );
    let surfaces: Vec<ContractSurface> = ContractSurface::ALL.to_vec();
    assert_eq!(surfaces.len(), 12);
}
