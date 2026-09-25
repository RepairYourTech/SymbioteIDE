use serde_json::{Value, json};
use std::{collections::VecDeque, time::Duration};
use symbiote_domain::*;
use symbiote_runtime_discovery::{
    AuthState, Authentication, ConfigRoot, ConfigRootSource, DiscoveryError, ExecutableIdentity,
    ExecutableInstallation, Fact, INSTALLATION_VERSION, InstallationChannel, InterfaceIdentity,
    Inventory, MethodAvailability, ObservationSource, ProbeProvenance, ProfileObservation,
    RuntimeInstanceIntent, UpdateAvailability,
    codex::{
        CodexAuthentication, CodexDiscoveryError, CodexModel, CodexProbeReport, DiscoveryTransport,
        discover,
    },
    codex_adapter::{CodexBindingError, bind_report},
};

fn provenance() -> ProbeProvenance {
    ProbeProvenance {
        probe_id: CommandId::new("codex-adapter-probe").unwrap(),
        source: ObservationSource::SandboxedProbe,
        adapter_revision: "v1".into(),
    }
}

const PROCESS_ID: u32 = 42;

struct Scripted(VecDeque<Value>);

impl DiscoveryTransport for Scripted {
    fn send(&mut self, _: &Value, _: Duration) -> Result<(), CodexDiscoveryError> {
        Ok(())
    }
    fn recv(&mut self, _: Duration) -> Result<Value, CodexDiscoveryError> {
        self.0.pop_front().ok_or(CodexDiscoveryError::Transport)
    }
    fn child_id(&self) -> Option<u32> {
        Some(PROCESS_ID)
    }
}

fn verified_report(authentication: CodexAuthentication) -> CodexProbeReport {
    let account = match authentication {
        CodexAuthentication::Required => json!({"requiresOpenaiAuth":true,"account":null}),
        CodexAuthentication::NotRequired => json!({"requiresOpenaiAuth":false,"account":null}),
        CodexAuthentication::ApiKeyReported => {
            json!({"requiresOpenaiAuth":false,"account":{"type":"apiKey"}})
        }
        CodexAuthentication::ChatGptReported => {
            json!({"requiresOpenaiAuth":false,"account":{"type":"chatgpt"}})
        }
        CodexAuthentication::Unknown => {
            json!({"requiresOpenaiAuth":false,"account":{"type":"other"}})
        }
    };
    let mut transport = Scripted(VecDeque::from([
        json!({"id":1,"result":{"userAgent":"symbiote/0.118.0 (Linux)","codexHome":"/home/agent/.codex"}}),
        json!({"id":2,"result":account}),
        json!({"id":3,"result":{"data":[{"id":"picker-one","model":"gpt-5.4","isDefault":true,"hidden":false,"defaultReasoningEffort":"medium","supportedReasoningEfforts":[{"reasoningEffort":"medium"}]}],"nextCursor":null}}),
    ]));
    discover(&mut transport, Duration::from_secs(1)).unwrap()
}

fn fixture() -> (
    RuntimeInstanceIntent,
    ProfileObservation,
    ExecutableInstallation,
    CodexProbeReport,
) {
    let profile_id = RuntimeProfileId::new("codex-personal").unwrap();
    let installation_id = InstallationId::new("codex-system").unwrap();
    let host_id = HostId::new("host-a").unwrap();
    let runtime_kind = RuntimeKind::ExternalHarness;
    let adapter_id = AgentRuntimeAdapterId::new("codex-harness").unwrap();
    let config_identity: String = "config_codex_personal".into();
    let instance_name: String = "Codex Personal".into();
    let profile = ProfileObservation {
        schema_version: 1,
        profile_id: profile_id.clone(),
        instance_name: instance_name.clone(),
        config_identity: config_identity.clone(),
        account_ref: None,
        path: "/home/agent/.codex".into(),
        source: ConfigRootSource::ProfileDefault,
        observed_at: Timestamp(10),
        expires_at: Timestamp(30),
        provenance: provenance(),
    };
    let installation = ExecutableInstallation {
        schema_version: INSTALLATION_VERSION,
        installation_id: installation_id.clone(),
        host_id: host_id.clone(),
        runtime_kind,
        adapter_id: adapter_id.clone(),
        channel: InstallationChannel::System,
        executable: ExecutableIdentity {
            requested_path: "/usr/bin/codex".into(),
            resolved_path: "/usr/bin/codex".into(),
        },
        version: Fact::Known("0.118.0".into()),
        sdk_version: Fact::Unknown,
        interface: Fact::Known(InterfaceIdentity {
            name: "app-server".into(),
            version: "0.118.0".into(),
        }),
        config_roots: vec![ConfigRoot {
            identity: config_identity.clone(),
            path: profile.path.clone(),
            source: profile.source,
        }],
        update_availability: UpdateAvailability::Unknown,
        observed_at: Timestamp(10),
        expires_at: Timestamp(30),
        provenance: provenance(),
    };
    let intent = RuntimeInstanceIntent {
        profile_id,
        installation_id,
        host_id,
        runtime_kind,
        adapter_id,
        instance_name,
        config_identity,
    };
    let report = verified_report(CodexAuthentication::Required);
    (intent, profile, installation, report)
}

#[test]
fn a_real_report_becomes_a_validated_inventory_without_becoming_usable() {
    let (intent, profile, installation, report) = fixture();
    let record = bind_report(intent.clone(), &profile, &installation, PROCESS_ID, &report).unwrap();
    assert_eq!(record.intent, intent);
    assert_eq!(
        record.observation.facts.health,
        Fact::Known(symbiote_runtime_discovery::Health::Reachable)
    );
    assert_eq!(
        record.observation.facts.authentication,
        Fact::Known(Authentication {
            mode: symbiote_runtime_discovery::AuthMode::None,
            state: AuthState::Required,
            account_ref: None,
        })
    );
    assert_eq!(
        record.observation.facts.methods["model/list"],
        MethodAvailability::Available
    );
    let Fact::Known(models) = &record.observation.facts.models else {
        panic!("the adapter must publish the models it observed");
    };
    assert_eq!(models[0].context_tokens, Fact::Unknown);
    assert_eq!(record.observation.facts.isolation, Fact::Unknown);
    let inventory = Inventory::new(vec![record]).unwrap();
    let encoded = serde_json::to_string(&inventory).unwrap();
    assert_eq!(Inventory::parse(&encoded).unwrap(), inventory);
    assert!(!encoded.contains("private-token"));
    assert!(!encoded.contains("someone@example.test"));
}

#[test]
fn reported_account_types_remain_unknown_authentication() {
    for authentication in [
        CodexAuthentication::ApiKeyReported,
        CodexAuthentication::ChatGptReported,
        CodexAuthentication::Unknown,
    ] {
        let (intent, profile, installation, mut report) = fixture();
        report.authentication = authentication;
        let record = bind_report(intent, &profile, &installation, PROCESS_ID, &report).unwrap();
        assert_eq!(record.observation.facts.authentication, Fact::Unknown);
    }
}

#[test]
fn another_profile_root_or_installation_cannot_be_paired_with_the_report() {
    let (mut intent, profile, installation, report) = fixture();
    intent.instance_name = "Codex Work".into();
    assert_eq!(
        bind_report(intent, &profile, &installation, PROCESS_ID, &report),
        Err(CodexBindingError::IdentityMismatch)
    );
    let (intent, mut profile, mut installation, report) = fixture();
    profile.path = "/home/agent/.other".into();
    installation.config_roots[0].path = profile.path.clone();
    assert_eq!(
        bind_report(intent, &profile, &installation, PROCESS_ID, &report),
        Err(CodexBindingError::ProfileMismatch)
    );
    let (intent, profile, installation, report) = fixture();
    assert_eq!(
        bind_report(intent, &profile, &installation, PROCESS_ID + 1, &report),
        Err(CodexBindingError::ProcessMismatch)
    );
    let (intent, profile, mut installation, report) = fixture();
    installation.host_id = HostId::new("host-b").unwrap();
    assert_eq!(
        bind_report(intent, &profile, &installation, PROCESS_ID, &report),
        Err(CodexBindingError::IdentityMismatch)
    );
}

#[test]
fn stale_unconfirmed_and_duplicate_report_facts_are_named_refusals() {
    let (intent, mut profile, installation, report) = fixture();
    profile.observed_at = Timestamp(11);
    assert_eq!(
        bind_report(intent, &profile, &installation, PROCESS_ID, &report),
        Err(CodexBindingError::Stale)
    );
    let (intent, profile, mut installation, report) = fixture();
    installation.expires_at = Timestamp(31);
    assert_eq!(
        bind_report(intent, &profile, &installation, PROCESS_ID, &report),
        Err(CodexBindingError::Stale)
    );
    let (intent, profile, mut installation, report) = fixture();
    installation.interface = Fact::Unknown;
    assert_eq!(
        bind_report(intent, &profile, &installation, PROCESS_ID, &report),
        Err(CodexBindingError::UnconfirmedProtocol)
    );
    let (intent, profile, mut installation, report) = fixture();
    installation.interface = Fact::Known(InterfaceIdentity {
        name: "other-interface".into(),
        version: "0.118.0".into(),
    });
    assert_eq!(
        bind_report(intent, &profile, &installation, PROCESS_ID, &report),
        Err(CodexBindingError::InterfaceMismatch)
    );
    let (intent, profile, mut installation, report) = fixture();
    installation.interface = Fact::Known(InterfaceIdentity {
        name: "app-server".into(),
        version: "0.118.1".into(),
    });
    assert_eq!(
        bind_report(intent, &profile, &installation, PROCESS_ID, &report),
        Err(CodexBindingError::InterfaceMismatch)
    );
    let (intent, profile, mut installation, report) = fixture();
    installation.version = Fact::Known("0.118.1".into());
    assert_eq!(
        bind_report(intent, &profile, &installation, PROCESS_ID, &report),
        Err(CodexBindingError::InterfaceMismatch)
    );
    let (intent, profile, installation, mut report) = fixture();
    report.models.push(CodexModel {
        listing_id: "picker-two".into(),
        ..report.models[0].clone()
    });
    assert_eq!(
        bind_report(intent, &profile, &installation, PROCESS_ID, &report),
        Err(CodexBindingError::DuplicateModel)
    );
}

#[test]
fn invalid_source_observations_and_report_facts_stay_distinct_refusals() {
    let (intent, mut profile, installation, report) = fixture();
    profile.expires_at = profile.observed_at;
    assert_eq!(
        bind_report(intent, &profile, &installation, PROCESS_ID, &report),
        Err(CodexBindingError::InvalidObservation)
    );
    let (intent, profile, mut installation, report) = fixture();
    installation.expires_at = installation.observed_at;
    assert_eq!(
        bind_report(intent, &profile, &installation, PROCESS_ID, &report),
        Err(CodexBindingError::InvalidObservation)
    );
    let (intent, profile, installation, mut report) = fixture();
    report.models[0].context_tokens = Some(0);
    assert_eq!(
        bind_report(intent, &profile, &installation, PROCESS_ID, &report),
        Err(CodexBindingError::Discovery(DiscoveryError::InvalidRecord))
    );
}
