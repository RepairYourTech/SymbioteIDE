use std::collections::{BTreeMap, BTreeSet};
use symbiote_domain::*;
use symbiote_runtime_discovery::*;
fn record() -> DiscoveryRecord {
    let intent = RuntimeInstanceIntent {
        profile_id: RuntimeProfileId::new("profile").unwrap(),
        installation_id: InstallationId::new("installation").unwrap(),
        host_id: HostId::new("host").unwrap(),
        runtime_kind: RuntimeKind::ExternalHarness,
        adapter_id: AgentRuntimeAdapterId::new("adapter").unwrap(),
        instance_name: "Named coding worker".into(),
        config_identity: "private-config".into(),
    };
    DiscoveryRecord {
        schema_version: 1,
        observation: InstallationObservation {
            config_identity: intent.config_identity.clone(),
            profile_id: intent.profile_id.clone(),
            installation_id: intent.installation_id.clone(),
            host_id: intent.host_id.clone(),
            runtime_kind: intent.runtime_kind,
            adapter_id: intent.adapter_id.clone(),
            version: Fact::Known("0.118.0".into()),
            interface: Fact::Known(InterfaceIdentity {
                name: "app-server".into(),
                version: "0.118.0".into(),
            }),
            facts: DiscoveryFacts {
                health: Fact::Known(Health::Reachable),
                authentication: Fact::Known(Authentication {
                    mode: AuthMode::None,
                    state: AuthState::Required,
                    account_ref: None,
                }),
                models: Fact::Known(vec![ModelListing {
                    upstream_model: "provider/model-1.2".into(),
                    canonical_model_id: None,
                    context_tokens: Fact::Unknown,
                }]),
                methods: BTreeMap::from([("model/list".into(), MethodAvailability::Available)]),
                isolation: Fact::Unknown,
            },
            observed_at: Timestamp(100),
            expires_at: Timestamp(200),
            provenance: ProbeProvenance {
                probe_id: CommandId::new("probe").unwrap(),
                source: ObservationSource::SandboxedProbe,
                adapter_revision: "v1".into(),
            },
        },
        intent,
    }
}
fn query(r: &DiscoveryRecord) -> EligibilityQuery {
    EligibilityQuery {
        config_identity: r.intent.config_identity.clone(),
        profile_id: r.intent.profile_id.clone(),
        installation_id: r.intent.installation_id.clone(),
        host_id: r.intent.host_id.clone(),
        runtime_kind: r.intent.runtime_kind,
        adapter_id: r.intent.adapter_id.clone(),
        version: "0.118.0".into(),
        interface: InterfaceIdentity {
            name: "app-server".into(),
            version: "0.118.0".into(),
        },
        required_methods: BTreeSet::from(["model/list".into()]),
        requires_authenticated: false,
        upstream_model: Some("provider/model-1.2".into()),
        minimum_context_tokens: None,
        requires_isolation: false,
    }
}
#[test]
fn named_intent_and_fresh_observed_facts_roundtrip_without_inferred_auth_or_context() {
    let r = record();
    let q = query(&r);
    let inventory = Inventory::new(vec![r.clone()]).unwrap();
    assert!(inventory.qualify(&q, Timestamp(150)).is_ok());
    assert_eq!(
        Inventory::parse(&serde_json::to_string(&inventory).unwrap()).unwrap(),
        inventory
    );
    let mut renamed = r.clone();
    renamed.intent.instance_name = "Native sounding display label".into();
    assert!(renamed.qualify(&q, Timestamp(150)).is_ok());
    let mut authenticated = q.clone();
    authenticated.requires_authenticated = true;
    assert_eq!(
        r.qualify(&authenticated, Timestamp(150)),
        Err(DiscoveryError::AuthenticationRequired)
    );
    let mut context = q.clone();
    context.minimum_context_tokens = Some(1);
    assert_eq!(
        r.qualify(&context, Timestamp(150)),
        Err(DiscoveryError::UnknownMandatoryFact)
    );
    let mut isolation = q;
    isolation.requires_isolation = true;
    assert_eq!(
        r.qualify(&isolation, Timestamp(150)),
        Err(DiscoveryError::IsolationUnverified)
    );
}
#[test]
fn stale_and_all_identity_version_interface_mismatches_are_rejected() {
    let r = record();
    let q = query(&r);
    for at in [99, 200, 201] {
        assert_eq!(r.qualify(&q, Timestamp(at)), Err(DiscoveryError::Stale));
    }
    let mut variants = Vec::new();
    let mut v = r.clone();
    v.observation.host_id = HostId::new("other").unwrap();
    variants.push(v);
    let mut v = r.clone();
    v.observation.profile_id = RuntimeProfileId::new("other").unwrap();
    variants.push(v);
    let mut v = r.clone();
    v.observation.installation_id = InstallationId::new("other").unwrap();
    variants.push(v);
    let mut v = r.clone();
    v.observation.runtime_kind = RuntimeKind::NativeSymbiote;
    variants.push(v);
    let mut v = r.clone();
    v.observation.adapter_id = AgentRuntimeAdapterId::new("other").unwrap();
    variants.push(v);
    let mut v = r.clone();
    v.observation.config_identity = "other".into();
    variants.push(v);
    for v in variants {
        assert_eq!(
            v.qualify(&q, Timestamp(150)),
            Err(DiscoveryError::IdentityMismatch)
        );
    }
    let mut v = r.clone();
    v.observation.version = Fact::Known("0.119.0".into());
    assert_eq!(
        v.qualify(&q, Timestamp(150)),
        Err(DiscoveryError::VersionMismatch)
    );
    let mut v = r;
    v.observation.interface = Fact::Known(InterfaceIdentity {
        name: "other".into(),
        version: "0.118.0".into(),
    });
    assert_eq!(
        v.qualify(&q, Timestamp(150)),
        Err(DiscoveryError::VersionMismatch)
    );
}
#[test]
fn unknown_none_expired_rate_limits_and_model_absence_stay_distinct() {
    let mut r = record();
    let mut q = query(&r);
    r.observation.facts.models = Fact::Unknown;
    assert_eq!(
        r.qualify(&q, Timestamp(150)),
        Err(DiscoveryError::UnknownMandatoryFact)
    );
    r.observation.facts.models = Fact::Known(vec![]);
    assert_eq!(
        r.qualify(&q, Timestamp(150)),
        Err(DiscoveryError::ModelNotListed)
    );
    q.upstream_model = None;
    q.requires_authenticated = true;
    r.observation.facts.authentication = Fact::Unknown;
    assert_eq!(
        r.qualify(&q, Timestamp(150)),
        Err(DiscoveryError::UnknownMandatoryFact)
    );
    r.observation.facts.authentication = Fact::Known(Authentication {
        mode: AuthMode::ChatGpt,
        state: AuthState::Expired,
        account_ref: Some("account_local".into()),
    });
    assert_eq!(
        r.qualify(&q, Timestamp(150)),
        Err(DiscoveryError::AuthenticationExpired)
    );
    r.observation.facts.authentication = Fact::Known(Authentication {
        mode: AuthMode::ApiKey,
        state: AuthState::RateLimited,
        account_ref: None,
    });
    assert_eq!(
        r.qualify(&q, Timestamp(150)),
        Err(DiscoveryError::RateLimited)
    );
    q.requires_authenticated = false;
    r.observation
        .facts
        .methods
        .insert("model/list".into(), MethodAvailability::Unknown);
    assert_eq!(
        r.qualify(&q, Timestamp(150)),
        Err(DiscoveryError::UnknownMandatoryFact)
    );
    r.observation
        .facts
        .methods
        .insert("model/list".into(), MethodAvailability::Unsupported);
    assert_eq!(
        r.qualify(&q, Timestamp(150)),
        Err(DiscoveryError::UnsupportedMethod)
    );
}
#[test]
fn malformed_duplicate_and_oversized_inventory_is_rejected() {
    let r = record();
    assert!(Inventory::new(vec![r.clone(); 257]).is_err());
    assert!(Inventory::new(vec![r.clone(), r.clone()]).is_err());
    let mut value = serde_json::to_value(&r).unwrap();
    value["schema_version"] = serde_json::json!(2);
    assert!(serde_json::from_value::<DiscoveryRecord>(value).is_err());
    let mut invalid = r.clone();
    invalid.observation.facts.authentication = Fact::Known(Authentication {
        mode: AuthMode::ChatGpt,
        state: AuthState::Authenticated,
        account_ref: Some("someone@example.com".into()),
    });
    assert!(invalid.validate().is_err());
    let mut invalid = r.clone();
    invalid.intent.config_identity = "/home/user/.codex".into();
    assert!(invalid.validate().is_err());
    let mut invalid = r.clone();
    invalid.observation.expires_at = Timestamp(100);
    assert!(invalid.validate().is_err());
    let encoded = serde_json::to_string(&r).unwrap().replace(
        "\"model/list\":\"available\"",
        "\"model/list\":\"available\",\"model/list\":\"unsupported\"",
    );
    assert!(serde_json::from_str::<DiscoveryRecord>(&encoded).is_err());
    let mut invalid = r;
    invalid.observation.facts.models = Fact::Known(vec![ModelListing {
        upstream_model: "m".into(),
        canonical_model_id: None,
        context_tokens: Fact::Known(0),
    }]);
    assert!(invalid.validate().is_err());
}
#[test]
fn native_runtime_uses_same_contract_without_provider_adapter_conflation() {
    let mut r = record();
    r.intent.runtime_kind = RuntimeKind::NativeSymbiote;
    r.observation.runtime_kind = RuntimeKind::NativeSymbiote;
    r.intent.adapter_id = AgentRuntimeAdapterId::new("native-rust-loop").unwrap();
    r.observation.adapter_id = r.intent.adapter_id.clone();
    let q = query(&r);
    assert!(r.qualify(&q, Timestamp(150)).is_ok());
    let external = query(&record());
    assert_eq!(
        r.qualify(&external, Timestamp(150)),
        Err(DiscoveryError::IdentityMismatch)
    );
}

#[test]
fn unknown_fact_cannot_silently_discard_claimed_payload() {
    for input in [
        r#"{"status":"unknown","value":{"account":"discarded"}}"#,
        r#"{"status":"unknown","value":"discarded"}"#,
    ] {
        assert!(serde_json::from_str::<Fact<String>>(input).is_err());
    }
}
