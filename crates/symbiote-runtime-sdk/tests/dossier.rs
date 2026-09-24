//! The Compatibility Dossier a pack publishes: one run per upstream version it
//! was really run against, and the refusals that stop it claiming otherwise.
use std::collections::{BTreeMap, BTreeSet};
use symbiote_domain::*;
// `symbiote_domain` names a canonical `Capability` record of its own (#36); this
// test means the runtime adapter's declared capability.
use symbiote_runtime_sdk::Capability;
use symbiote_runtime_sdk::conformance::{
    RULES, placed_capabilities, placed_controls, run_conformance,
};
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

/// The operator's form of the same runtime: the identity and the facts, with
/// the observation identity, the evidence artifact and the window left to the
/// Host. This is the form a Host actually holds, and therefore the one a reader
/// holds a published record against.
fn declared_at(pack: &Pack) -> DeclaredRuntime {
    let descriptor = descriptor_at(pack);
    DeclaredRuntime {
        adapter_id: descriptor.adapter_id.clone(),
        installation: descriptor.installation.clone(),
        profile_id: descriptor.profile_id.clone(),
        profile_revision: descriptor.profile_revision,
        model_id: descriptor.model_id.clone(),
        adapter_version: descriptor.adapter_version.clone(),
        upstream_version: descriptor.upstream_version.clone(),
        runtime: descriptor.runtime,
        owner: descriptor.owner.clone(),
        transport: descriptor.transport.clone(),
        tier: descriptor.tier.clone(),
        platform: descriptor.platform.clone(),
        capabilities: BTreeSet::new(),
        controls: BTreeMap::new(),
        tools: descriptor.tools.clone(),
        skills: descriptor.skills.clone(),
        context_limits: descriptor.context_limits.clone(),
    }
}

/// The declaration the pack every case describes.
fn declared() -> DeclaredRuntime {
    declared_at(&DEFAULT)
}

/// The record as a reader receives it: parsed back out of the published bytes,
/// so a case edits a *document* — the only form a Host ever holds a pack's
/// claim in — rather than an in-memory value nobody had to write down.
fn parsed(dossier: &CompatibilityDossier) -> CompatibilityDossier {
    serde_json::from_str(&serde_json::to_string(dossier).unwrap()).unwrap()
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
/// The reader's side: a record parsed back out of its published bytes, held
/// against the runtime this Host actually has. The holding names the run inside
/// the record that certifies this runtime and the declaration it was held
/// against, and serves the record whole.
#[test]
fn a_held_record_names_the_run_that_certifies_this_runtime() {
    let declared = declared();
    let dossier = parsed(&publish_one(&descriptor()).expect("the run publishes"));
    // The record is this pack's, read the one way the Host both chooses its
    // installed file and refuses a foreign one.
    assert!(dossier.publishes(
        &declared.adapter_id,
        match &declared.owner {
            RuntimeOwner::External { driver } => Some(driver),
            RuntimeOwner::SymbioteNative {} => None,
        }
    ));
    let held = dossier
        .hold(&declared)
        .expect("this pack certified this runtime");
    assert_eq!(held.declared.adapter_id, declared.adapter_id);
    assert_eq!(held.run.upstream_version, "fixture-1");
    assert_eq!(held.run.platform, "linux");
    assert_eq!(held.run.adapter_version, "1.0");
    // The run is the one inside the record, not a copy of it: the same rows.
    assert_eq!(Some(held.run), dossier.run("fixture-1", "linux"));
    assert!(
        held.run.rules.iter().all(|rule| rule.held),
        "a held run shows every rule as held"
    );
    // A native runtime is not this external pack, even at the same version: the
    // pack identity includes the driver a pack drives.
    let mut native = declared.clone();
    native.owner = RuntimeOwner::SymbioteNative {};
    native.runtime = RuntimeKind::NativeSymbiote;
    native.installation = None;
    assert_eq!(
        dossier
            .hold(&native)
            .expect_err("a native runtime is not an external pack"),
        HoldError::AnotherPack {
            stated: "adapter/fixture-driver".into(),
            held: "adapter".into(),
        }
    );
}

/// A record that is another pack's, or that names a shape this crate does not
/// speak, is refused before any of its evidence is read: a reader cannot hold a
/// document whose own identity is not the runtime it has.
#[test]
fn a_record_that_is_another_packs_or_another_shape_is_refused() {
    let declared = declared();
    let mut foreign = parsed(&publish_one(&descriptor()).expect("the run publishes"));
    foreign.adapter = AgentRuntimeAdapterId::new("other-adapter").unwrap();
    assert_eq!(
        foreign.hold(&declared).expect_err("another pack's record"),
        HoldError::AnotherPack {
            stated: "other-adapter/fixture-driver".into(),
            held: "adapter/fixture-driver".into(),
        }
    );
    // A driver is part of the pack's identity, so the same adapter under another
    // driver is another pack too.
    let mut other_driver = foreign.clone();
    other_driver.driver = Some(HarnessDriverId::new("other-driver").unwrap());
    assert!(matches!(
        other_driver.hold(&declared),
        Err(HoldError::AnotherPack { .. })
    ));
    // A shape this crate does not publish cannot be read as one it does: the
    // fields would be somebody else's, whatever they are named.
    let mut shape = parsed(&publish_one(&descriptor()).expect("the run publishes"));
    shape.schema_version = DOSSIER_SCHEMA_VERSION + 1;
    assert_eq!(
        shape.hold(&declared).expect_err("another shape"),
        HoldError::UnknownShape {
            stated: DOSSIER_SCHEMA_VERSION + 1,
        }
    );
    assert!(
        shape
            .hold(&declared)
            .unwrap_err()
            .to_string()
            .contains("shape")
    );
    // And a document whose own runs are not its pack's is a document that mixes
    // packs, refused before any run is read as this one.
    let mut mixed = parsed(&publish_one(&descriptor()).expect("the run publishes"));
    mixed.runs[0].adapter = AgentRuntimeAdapterId::new("other-adapter").unwrap();
    assert_eq!(
        mixed.hold(&declared).expect_err("a mixed document"),
        HoldError::TheRunNamesAnotherPack {
            stated: "other-adapter/fixture-driver".into(),
            pack: "adapter/fixture-driver".into(),
        }
    );
}

/// The evidence a record carries about outcomes is the one thing a reader cannot
/// observe for itself, so every way of weakening it is the same refusal: a rule
/// this suite runs that the record does not show as held.
#[test]
fn a_rule_the_record_does_not_show_as_held_is_refused_by_name() {
    let declared = declared();
    let first = RULES[0].id.to_string();
    for edit in [
        // A row that reports the rule as failed.
        Box::new(|run: &mut ConformanceRun| run.rules[0].held = false)
            as Box<dyn Fn(&mut ConformanceRun)>,
        // A row that claims an outcome beside a failure: the two contradict, and
        // neither reading of it is evidence.
        Box::new(|run: &mut ConformanceRun| {
            run.rules[0].failure = Some("a failure the record also calls held".into())
        }),
        // No row at all. A row under another name is refused earlier and by
        // another name — `UnknownRules`, because a row the suite does not publish
        // is not a weakened outcome but a rule nobody here runs.
        Box::new(|run: &mut ConformanceRun| {
            run.rules.remove(0);
        }),
        // A second row beside the held one: two rows are not one outcome, and a
        // rule that appears twice is not shown as held by either of them.
        Box::new(|run: &mut ConformanceRun| {
            let mut second = run.rules[0].clone();
            second.held = false;
            run.rules.push(second);
        }),
    ] {
        let mut dossier = parsed(&publish_one(&descriptor()).expect("the run publishes"));
        edit(&mut dossier.runs[0]);
        let refused = dossier.hold(&declared).expect_err("an unheld rule");
        assert_eq!(
            refused,
            HoldError::ARuleDidNotHold {
                rules: vec![first.clone()]
            },
            "one rule is not shown as held, and the refusal names it"
        );
        let text = refused.to_string();
        assert!(text.contains(&first), "{text}");
        assert!(
            !text.contains(ROLE_PROSE),
            "a refusal carries no task content"
        );
        assert_eq!(refused.name(), "a_rule_did_not_hold");
    }
    // Two weakened rules are named together, and a record with every rule held
    // is not refused for the rules at all.
    let mut two = parsed(&publish_one(&descriptor()).expect("the run publishes"));
    two.runs[0].rules[0].held = false;
    two.runs[0].rules[1].failure = Some("also failed".into());
    assert_eq!(
        two.hold(&declared)
            .expect_err("two rules are not shown as held"),
        HoldError::ARuleDidNotHold {
            rules: vec![first, RULES[1].id.to_string()]
        }
    );
}

/// A record carries its own vocabulary, so a rule row the suite does not publish
/// is a row about a rule nobody here runs — and a record with the same version
/// and platform in two runs cannot say which of the two is the claim. Both are
/// refused before any of the record's outcomes is believed.
#[test]
fn a_rule_the_suite_does_not_publish_and_a_version_twice_are_refused() {
    let declared = declared();
    let mut unknown = parsed(&publish_one(&descriptor()).expect("the run publishes"));
    unknown.runs[0].rules.push(RuleRead {
        id: "a_rule_this_suite_does_not_run".into(),
        held: true,
        failure: None,
    });
    assert_eq!(
        unknown.hold(&declared).expect_err("an unknown rule"),
        HoldError::UnknownRules {
            rules: vec!["a_rule_this_suite_does_not_run".into()]
        }
    );
    // A known rule's row renamed is the same refusal: the record no longer shows
    // the suite's rule, and the name it shows instead is one nobody here runs.
    let mut renamed = parsed(&publish_one(&descriptor()).expect("the run publishes"));
    renamed.runs[0].rules[0].id = "some_other_rule".into();
    assert_eq!(
        renamed.hold(&declared).expect_err("a renamed rule row"),
        HoldError::UnknownRules {
            rules: vec!["some_other_rule".into()]
        }
    );
    // Two runs for one version and platform: the pair a reader looks a run up
    // with would name either of them, and a second run that disagrees is exactly
    // what the publisher refuses to publish.
    let mut twice = parsed(&publish_one(&descriptor()).expect("the run publishes"));
    let mut second = twice.runs[0].clone();
    second.adapter_version = "0.9".into();
    twice.runs.push(second);
    assert_eq!(
        twice.hold(&declared).expect_err("the same version twice"),
        HoldError::TheSameRunTwice {
            upstream_version: "fixture-1".into(),
            platform: "linux".into(),
        }
    );
    // The same upstream version on another platform is a different claim, so it
    // is served rather than refused: only the pair may appear once.
    let mut platforms = parsed(&publish_one(&descriptor()).expect("the run publishes"));
    let mut macos = platforms.runs[0].clone();
    macos.platform = "macos".into();
    platforms.runs.push(macos);
    assert!(platforms.hold(&declared).is_ok());
}

/// The two matrices are what a record says was demanded and realized, so a
/// matrix that is not the one this suite publishes is evidence for nothing:
/// whether a row is missing, extra, duplicated or on another surface, the reader
/// is being shown a claim the suite would not have written.
#[test]
fn a_matrix_that_is_not_the_suites_is_refused() {
    let declared = declared();
    for (what, edit) in [
        (
            "capabilities",
            Box::new(|run: &mut ConformanceRun| {
                run.capabilities.pop();
            }) as Box<dyn Fn(&mut ConformanceRun)>,
        ),
        (
            "capabilities",
            Box::new(|run: &mut ConformanceRun| {
                run.capabilities[0] = run.capabilities[1].clone();
            }),
        ),
        (
            "controls",
            Box::new(|run: &mut ConformanceRun| {
                run.controls.pop();
            }),
        ),
        (
            "controls",
            Box::new(|run: &mut ConformanceRun| {
                if let Some(row) = run.controls.first_mut() {
                    row.surface = ContractSurface::WorkforceProtocol;
                }
            }),
        ),
    ] {
        let mut dossier = parsed(&publish_one(&descriptor()).expect("the run publishes"));
        edit(&mut dossier.runs[0]);
        assert_eq!(
            dossier.hold(&declared).expect_err("another matrix"),
            HoldError::TheMatricesAreNotTheSuites { what },
            "the refusal names which matrix, not a count"
        );
    }
    // The shape both matrices are read against is the suite's own, so a case
    // cannot hold a record against a shape the suite would not publish, and a
    // held run carries one row per placed capability and one per placed control.
    assert_eq!(placed_capabilities().len(), 6);
    let whole = parsed(&publish_one(&descriptor()).expect("the run publishes"));
    let held = whole.hold(&declared).expect("the whole record is held");
    assert_eq!(held.run.capabilities.len(), placed_capabilities().len());
    assert_eq!(held.run.controls.len(), placed_controls().len());
    assert_eq!(
        held.run.controls[0].surface,
        placed_controls()[0].0,
        "a control row is the surface this module places that control on"
    );
    // What a row *says* is the pack's claim rather than a fact this reader can
    // recompute — a record carries each dispatch's summary, not the work the
    // suite read — so a value is served as published. The shape is what the
    // reader holds, and this says so rather than leaving it implied.
    let mut edited_value = parsed(&publish_one(&descriptor()).expect("the run publishes"));
    edited_value.runs[0].capabilities[0].carried = false;
    assert!(
        edited_value.hold(&declared).is_ok(),
        "a row's value is the pack's claim; only its shape is the reader's check"
    );
}

/// A run over no dispatch proves nothing about any work, exactly as it cannot be
/// published: a document that claims one anyway is refused rather than served.
#[test]
fn a_run_that_read_no_dispatch_is_refused() {
    let declared = declared();
    let mut dossier = parsed(&publish_one(&descriptor()).expect("the run publishes"));
    dossier.runs[0].dispatches.clear();
    assert_eq!(
        dossier.hold(&declared).expect_err("a run over no work"),
        HoldError::TheRunReadNoDispatch
    );
}

/// The version and the platform are the pair a reader looks a run up with, so a
/// runtime at a version the pack published no run for is refused by name rather
/// than served the nearest run it does have.
#[test]
fn a_version_or_platform_the_dossier_does_not_publish_is_refused() {
    let declared = declared();
    let dossier = parsed(&publish_one(&descriptor()).expect("the run publishes"));
    let mut other_version = declared.clone();
    other_version.upstream_version = "fixture-2".into();
    assert_eq!(
        dossier
            .hold(&other_version)
            .expect_err("an unpublished version"),
        HoldError::NoRun {
            upstream_version: "fixture-2".into(),
            platform: "linux".into(),
        }
    );
    let mut other_platform = declared.clone();
    other_platform.platform = "macos".into();
    assert_eq!(
        dossier
            .hold(&other_platform)
            .expect_err("an unpublished platform"),
        HoldError::NoRun {
            upstream_version: "fixture-1".into(),
            platform: "macos".into(),
        }
    );
    // A second published version is a run the reader may hold, and the holding
    // names that one rather than the first.
    let newer = Pack {
        upstream_version: "fixture-2",
        adapter_version: "1.1",
        ..DEFAULT
    };
    let second = Fixture::new(descriptor_at(&newer));
    let dispatch = dispatch();
    let both = CompatibilityDossier::publish(vec![
        PackRun {
            adapter: &Fixture::new(descriptor()),
            dispatches: vec![&dispatch],
        },
        PackRun {
            adapter: &second,
            dispatches: vec![&dispatch],
        },
    ])
    .expect("two versions publish");
    let mut newer_declaration = declared_at(&newer);
    newer_declaration.adapter_version = "1.1".into();
    let held = both
        .hold(&newer_declaration)
        .expect("the newer version is published");
    assert_eq!(held.run.upstream_version, "fixture-2");
    assert_eq!(held.run.adapter_version, "1.1");
    // The older run is still the refusal this declaration asks about, not the
    // newer one that happens to exist.
    assert!(matches!(
        dossier.hold(&declared),
        Ok(held) if held.run.upstream_version == "fixture-1"
    ));
    // A version this record does publish is held by the declaration that names
    // it — and by that run's own adapter version, not a neighbour's.
    other_version.adapter_version = "1.1".into();
    assert!(matches!(
        both.hold(&other_version),
        Ok(held) if held.run.upstream_version == "fixture-2"
    ));
}

/// The adapter's own version is per run: a run made with another build of the
/// adapter is not evidence about this one, however well it passed.
#[test]
fn a_run_of_another_adapter_version_is_refused() {
    let mut declared = declared();
    declared.adapter_version = "1.1".into();
    let dossier = parsed(&publish_one(&descriptor()).expect("the run publishes"));
    assert_eq!(
        dossier.hold(&declared).expect_err("another build's run"),
        HoldError::AnotherAdapterVersion {
            stated: "1.0".into(),
            held: "1.1".into(),
        }
    );
}

/// A boundary that has to name a refusal without repeating the document gets
/// the refusal's own name and nothing else: no version, no platform, no identity
/// and no path the file said. The names are stable for the same reason the
/// published shape is.
#[test]
fn every_refusal_has_a_stable_name_that_echoes_nothing() {
    let mut another_shape = parsed(&publish_one(&descriptor()).expect("the run publishes"));
    another_shape.schema_version = DOSSIER_SCHEMA_VERSION + 1;
    let refusals = [
        (
            another_shape.hold(&declared()).unwrap_err(),
            "unknown_shape",
        ),
        (
            HoldError::AnotherPack {
                stated: "other/fixture-driver".into(),
                held: "adapter/fixture-driver".into(),
            },
            "another_pack",
        ),
        (
            HoldError::TheRunNamesAnotherPack {
                stated: "other/fixture-driver".into(),
                pack: "adapter/fixture-driver".into(),
            },
            "the_run_names_another_pack",
        ),
        (
            HoldError::UnknownRules {
                rules: vec!["a_rule_this_suite_does_not_run".into()],
            },
            "unknown_rules",
        ),
        (
            HoldError::TheSameRunTwice {
                upstream_version: "fixture-1".into(),
                platform: "linux".into(),
            },
            "the_same_run_twice",
        ),
        (
            HoldError::ARuleDidNotHold {
                rules: vec!["descriptor_identity_is_valid".into()],
            },
            "a_rule_did_not_hold",
        ),
        (
            HoldError::TheMatricesAreNotTheSuites { what: "controls" },
            "the_matrices_are_not_the_suites",
        ),
        (HoldError::TheRunReadNoDispatch, "the_run_read_no_dispatch"),
        (
            HoldError::NoRun {
                upstream_version: "fixture-2".into(),
                platform: "linux".into(),
            },
            "no_run",
        ),
        (
            HoldError::AnotherAdapterVersion {
                stated: "1.0".into(),
                held: "1.1".into(),
            },
            "another_adapter_version",
        ),
    ];
    for (refusal, name) in &refusals {
        assert_eq!(refusal.name(), *name);
        // A name is a closed lowercase identifier: no digit, no dot, no slash —
        // nothing a document could have put into it.
        assert!(
            !refusal
                .name()
                .contains(['0', '1', '2', '3', '4', '5', '6', '7', '8', '9', '.', '/']),
            "the refusal's name is not a closed identifier: {}",
            refusal.name()
        );
        // The full text is for an operator reading the Host's own logs and for a
        // case; it names the facts, and never the role's prose.
        assert!(!refusal.to_string().contains(ROLE_PROSE));
    }
    // Every refusal the module states is named here, so a variant added without
    // a wire name reds by name rather than reaching a boundary unnamed.
    assert_eq!(refusals.len(), 10);
}

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
        "publishes",
        "hold",
        "DossierHolding",
        "HoldError",
    ] {
        assert!(contract.contains(name), "the document does not name {name}");
    }
    for refusal in [
        "RulesFailed",
        "NoDispatches",
        "NoRuns",
        "MoreThanOnePack",
        "TheSameVersionTwice",
        "UnknownShape",
        "AnotherPack",
        "TheRunNamesAnotherPack",
        "UnknownRules",
        "TheSameRunTwice",
        "ARuleDidNotHold",
        "TheMatricesAreNotTheSuites",
        "TheRunReadNoDispatch",
        "NoRun",
        "AnotherAdapterVersion",
    ] {
        assert!(
            contract.contains(refusal),
            "the document does not name {refusal}"
        );
    }
    // The ways in are the only ways in, and a version absent from them is a
    // version the dossier says nothing about.
    assert!(
        contract
            .contains("`CompatibilityDossier::run`, `versions` and `hold` are the only ways in"),
        "the document states the only ways into what a pack publishes"
    );
    // What the reader's check is not is stated in the document too, so a served
    // record cannot read as a qualification or as authenticated proof.
    for limit in [
        "neither qualification nor authentication",
        "what a row *says* about a run is the pack's claim",
    ] {
        assert!(
            contract.contains(limit),
            "the document does not state the limit: {limit}"
        );
    }
    let surfaces: Vec<ContractSurface> = ContractSurface::ALL.to_vec();
    assert_eq!(surfaces.len(), 12);
}
