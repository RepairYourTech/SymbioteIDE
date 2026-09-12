//! A declared runtime is a Host-provisioned input, never a self-authenticating
//! observation: the Host owns the identity and the evidence window, and the
//! declaration owns only the facts it asserts.
use std::collections::{BTreeMap, BTreeSet};
use symbiote_domain::*;
use symbiote_runtime_sdk::*;

fn profile() -> RuntimeProfile {
    RuntimeProfile {
        id: RuntimeProfileId::new("native-worker").unwrap(),
        revision: Revision(1),
        runtime: RuntimeKind::NativeSymbiote,
        adapter: AgentRuntimeAdapterId::new("native-agent").unwrap(),
        installation: None,
        provider: ProviderConnectionId::new("provider").unwrap(),
        credential: CredentialReferenceId::new("credential").unwrap(),
        billing_entitlement: BillingEntitlementId::new("entitlement").unwrap(),
        model: ModelId::new("model").unwrap(),
        eligible_hosts: [HostId::new("host-a").unwrap()].into(),
    }
}

fn declaration() -> DeclaredRuntime {
    DeclaredRuntime {
        adapter_id: profile().adapter,
        installation: None,
        profile_id: profile().id,
        profile_revision: profile().revision,
        model_id: profile().model,
        adapter_version: "1.0.0".into(),
        upstream_version: "1.0.0".into(),
        runtime: RuntimeKind::NativeSymbiote,
        owner: RuntimeOwner::SymbioteNative {},
        transport: Transport::NativeLoop,
        tier: IntegrationTier::Detected,
        platform: "linux".into(),
        capabilities: [Capability::Tools].into(),
        controls: BTreeMap::from([(
            Control::Filesystem,
            DeclaredControl {
                strength: EnforcementStrength::HostEnforced,
                mechanism: "operator-provisioned sandbox".into(),
            },
        )]),
        tools: ["read".into()].into(),
        skills: BTreeSet::new(),
        context_limits: Some(RuntimeContextLimits {
            context_window_tokens: 1_000,
            max_output_tokens: 100,
        }),
    }
}

fn required_capabilities(
    capabilities: impl IntoIterator<Item = Capability>,
) -> BTreeSet<Capability> {
    capabilities.into_iter().collect()
}

fn minimums(minimum: EnforcementStrength) -> BTreeMap<Control, EnforcementStrength> {
    BTreeMap::from([(Control::Filesystem, minimum)])
}

#[test]
fn the_host_owns_the_observation_identity_artifact_and_window() {
    let host = HostId::new("host-a").unwrap();
    let at = Timestamp(5_000);
    let descriptor = declaration().observe(&host, at).unwrap();
    assert_eq!(descriptor.host_id, host);
    assert_eq!(descriptor.sdk_version, SDK_VERSION);
    assert_eq!(descriptor.profile_id, declaration().profile_id);
    assert_eq!(descriptor.tools, declaration().tools);
    // Every declared capability and control carries one fresh window the
    // declaration never supplied, and the artifact names the observing Host.
    let capability = match descriptor.capabilities.get(&Capability::Tools).unwrap() {
        Support::Supported { evidence } => evidence,
        other => panic!("declared capability is not supported: {other:?}"),
    };
    let control = descriptor.controls.get(&Control::Filesystem).unwrap();
    assert_eq!(control.evidence, **capability);
    assert_eq!(capability.host_id, host);
    assert_eq!(capability.observed_at, at);
    assert_eq!(
        capability.expires_at,
        Timestamp(at.0 + DeclaredRuntime::EVIDENCE_WINDOW_MS)
    );
    assert!(capability.artifact.as_str().contains(host.as_str()));
    descriptor.validate().unwrap();
}

#[test]
fn the_declaration_is_the_runtime_the_profile_asks_for_or_it_is_refused() {
    let host = HostId::new("host-a").unwrap();
    let at = Timestamp(5_000);
    let descriptor = declaration().observe(&host, at).unwrap();
    let profile = profile();
    qualify_profile_with_minimums(
        &profile,
        &descriptor,
        &required_capabilities([Capability::Tools]),
        &minimums(EnforcementStrength::HostEnforced),
        at,
    )
    .unwrap();
    // A capability the declaration omits is missing, not assumed supported.
    assert_eq!(
        qualify_profile_with_minimums(
            &profile,
            &descriptor,
            &required_capabilities([Capability::Images]),
            &minimums(EnforcementStrength::HostEnforced),
            at,
        ),
        Err(QualificationError::MissingCapability(Capability::Images))
    );
    // A control weaker than the profile's minimum cannot qualify.
    assert_eq!(
        qualify_profile_with_minimums(
            &profile,
            &descriptor,
            &required_capabilities([Capability::Tools]),
            &minimums(EnforcementStrength::Native),
            at,
        ),
        Err(QualificationError::InsufficientControl(Control::Filesystem))
    );
    // The window is bounded: the same observation expires.
    assert_eq!(
        qualify_profile_with_minimums(
            &profile,
            &descriptor,
            &required_capabilities([Capability::Tools]),
            &minimums(EnforcementStrength::HostEnforced),
            Timestamp(at.0 + DeclaredRuntime::EVIDENCE_WINDOW_MS),
        ),
        Err(QualificationError::StaleEvidence)
    );
    // A declaration for another profile identity is not an observation of this
    // one, and neither is one whose runtime/owner pairing is incoherent.
    let mut other = declaration();
    other.model_id = ModelId::new("other-model").unwrap();
    assert_eq!(
        qualify_profile_with_minimums(
            &profile,
            &other.observe(&host, at).unwrap(),
            &required_capabilities([Capability::Tools]),
            &minimums(EnforcementStrength::HostEnforced),
            at,
        ),
        Err(QualificationError::AdapterMismatch)
    );
    let mut incoherent = declaration();
    incoherent.owner = RuntimeOwner::External {
        driver: HarnessDriverId::new("driver").unwrap(),
    };
    assert_eq!(
        incoherent.validate(),
        Err(QualificationError::RuntimeOwnerMismatch)
    );
    let mut unbounded = declaration();
    unbounded.context_limits = Some(RuntimeContextLimits {
        context_window_tokens: 10,
        max_output_tokens: 100,
    });
    assert_eq!(
        unbounded.validate(),
        Err(QualificationError::InvalidDescriptor)
    );
}
