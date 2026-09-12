use symbiote_domain::{CommandId, HostId, Timestamp};
use symbiote_host_inventory::*;
fn pulse() -> HostPulse {
    HostPulse::new(
        HostId::new("host-a").unwrap(),
        CommandId::new("sample-a").unwrap(),
        Timestamp(1_000),
        Timestamp(6_000),
        TelemetryMode::Enabled,
        Fact::Known("linux".into()),
        Fact::Known("x86_64".into()),
        ResourceObservation {
            logical_cpu_count: Fact::Known(64),
            physical_memory_total_bytes: Fact::Known(1_000_000),
            physical_memory_available_bytes: Fact::Known(900_000),
            ..Default::default()
        },
        PulseProvenance {
            source: PulseSource::OperatingSystem,
            probe_version: "test-v1".into(),
        },
    )
    .unwrap()
}
fn query() -> PulseRequirements {
    PulseRequirements {
        host_id: HostId::new("host-a").unwrap(),
        minimum_cpu_millicores: None,
        minimum_available_memory_bytes: None,
        capabilities: vec![],
    }
}
#[test]
fn physical_capacity_never_substitutes_for_effective_capacity() {
    let mut p = pulse();
    let mut q = query();
    q.minimum_cpu_millicores = Some(1);
    assert_eq!(p.qualify(&q, Timestamp(2_000)), Err(PulseError::Unknown));
    p.resources.effective_cpu_millicores = Fact::Known(0);
    assert_eq!(
        p.qualify(&q, Timestamp(2_000)),
        Err(PulseError::Insufficient)
    );
    p.resources.effective_cpu_millicores = Fact::Known(500);
    assert!(p.qualify(&q, Timestamp(2_000)).is_ok());
    q.minimum_available_memory_bytes = Some(1);
    assert_eq!(p.qualify(&q, Timestamp(2_000)), Err(PulseError::Unknown));
}
#[test]
fn exact_host_freshness_and_disabled_fail_closed() {
    let p = pulse();
    let mut q = query();
    assert_eq!(p.qualify(&q, Timestamp(999)), Err(PulseError::Stale));
    assert_eq!(p.qualify(&q, Timestamp(6_000)), Err(PulseError::Stale));
    q.host_id = HostId::new("host-b").unwrap();
    assert_eq!(
        p.qualify(&q, Timestamp(2_000)),
        Err(PulseError::HostMismatch)
    );
    let mut disabled =
        HostPulse::telemetry_disabled(p.host_id, p.observation_id, Timestamp(2_000)).unwrap();
    assert_eq!(
        disabled.qualify(&query(), Timestamp(2_001)),
        Err(PulseError::Disabled)
    );
    disabled.resources.logical_cpu_count = Fact::Known(1);
    assert_eq!(disabled.validate(), Err(PulseError::Invalid));
}
#[test]
fn installation_does_not_imply_health_or_authentication() {
    let mut p = pulse();
    let mut q = query();
    q.capabilities.push(CapabilityRequirement {
        kind: CapabilityKind::Container,
        version: Some("1.2".into()),
        authentication_required: true,
    });
    p.capabilities.push(CapabilityObservation {
        kind: CapabilityKind::Container,
        installed: Fact::Known(true),
        authentication: Fact::Unknown,
        health: Fact::Unknown,
        version: Fact::Known("1.2".into()),
        observed_at: Timestamp(1_000),
        expires_at: Timestamp(4_000),
        source: CapabilitySource::HealthCheck,
    });
    assert_eq!(p.qualify(&q, Timestamp(2_000)), Err(PulseError::Unknown));
    p.capabilities[0].health = Fact::Known(CapabilityHealth::Healthy);
    assert_eq!(p.qualify(&q, Timestamp(2_000)), Err(PulseError::Unknown));
    p.capabilities[0].authentication = Fact::Known(CapabilityAuthentication::Expired);
    assert_eq!(
        p.qualify(&q, Timestamp(2_000)),
        Err(PulseError::CapabilityUnavailable)
    );
    p.capabilities[0].authentication = Fact::Known(CapabilityAuthentication::Authenticated);
    assert!(p.qualify(&q, Timestamp(2_000)).is_ok());
    assert_eq!(p.qualify(&q, Timestamp(4_000)), Err(PulseError::Stale));
    q.capabilities[0].version = Some("2.0".into());
    assert_eq!(
        p.qualify(&q, Timestamp(2_000)),
        Err(PulseError::CapabilityUnavailable)
    );
}
#[test]
fn invalid_bounds_versions_and_wire_fields_rejected() {
    let p = pulse();
    let json = serde_json::to_string(&p).unwrap();
    assert_eq!(HostPulse::parse(&json).unwrap(), p);
    let mut invalid = p.clone();
    invalid.resources.physical_memory_available_bytes = Fact::Known(2_000_000);
    assert_eq!(invalid.validate(), Err(PulseError::Invalid));
    invalid = p.clone();
    invalid.schema_version = 2;
    assert_eq!(invalid.validate(), Err(PulseError::UnsupportedVersion));
    assert!(serde_json::from_str::<HostPulse>(&serde_json::to_string(&invalid).unwrap()).is_err());
    invalid = p;
    invalid.expires_at = Timestamp(31_001);
    assert_eq!(invalid.validate(), Err(PulseError::Invalid));
    assert_eq!(
        HostPulse::parse(&" ".repeat(65537)),
        Err(PulseError::Capacity)
    );
    assert!(HostPulse::parse(&json.replacen("{", "{\"credentials\":\"forbidden\",", 1)).is_err());
}
#[test]
fn probe_failures_are_preserved_and_contradictions_rejected() {
    use symbiote_host_inventory::linux::*;
    let p = pulse();
    let observation = LinuxObservation {
        os: p.os.clone(),
        architecture: p.architecture.clone(),
        resources: ResourceObservation::default(),
        observed_at: Timestamp(10),
        failures: vec![ProbeFailure {
            resource: ProbeResource::PhysicalMemory,
            reason: ProbeError::Unreadable,
        }],
    };
    let mut result =
        HostPulse::from_observation(p.host_id.clone(), p.observation_id.clone(), observation)
            .unwrap();
    assert_eq!(result.probe_failures.len(), 1);
    assert_eq!(result.expires_at, Timestamp(5_010));
    result.resources.physical_memory_total_bytes = Fact::Known(100);
    assert_eq!(result.validate(), Err(PulseError::Invalid));

    // The same rule for the effective facts: a failure that explains absent
    // effective availability cannot coexist with an observed value, while a
    // finite ceiling observed on its own is consistent — the failure explains
    // the charge against the ceiling, not the ceiling.
    let mut effective = p.clone();
    effective.probe_failures = vec![
        ProbeFailure {
            resource: ProbeResource::EffectiveMemory,
            reason: ProbeError::Unreadable,
        },
        ProbeFailure {
            resource: ProbeResource::EffectiveCpu,
            reason: ProbeError::UnsupportedHierarchy,
        },
    ];
    assert_eq!(effective.validate(), Ok(()));
    effective.resources.effective_memory_available_bytes = Fact::Known(10);
    assert_eq!(effective.validate(), Err(PulseError::Invalid));
    effective.resources.effective_memory_available_bytes = Fact::Unknown;
    effective.resources.effective_cpu_millicores = Fact::Known(10);
    assert_eq!(effective.validate(), Err(PulseError::Invalid));
    effective.resources.effective_cpu_millicores = Fact::Unknown;
    effective.resources.effective_memory_limit_bytes = Fact::Known(100);
    assert_eq!(effective.validate(), Ok(()));
    // At most one failure per resource, and a bounded total.
    effective.probe_failures = vec![
        ProbeFailure {
            resource: ProbeResource::EffectiveMemory,
            reason: ProbeError::Unreadable,
        },
        ProbeFailure {
            resource: ProbeResource::EffectiveMemory,
            reason: ProbeError::Malformed,
        },
    ];
    assert_eq!(effective.validate(), Err(PulseError::Invalid));
    effective.probe_failures = vec![effective.probe_failures[0].clone(); 5];
    assert_eq!(effective.validate(), Err(PulseError::Capacity));
}

#[test]
fn unsupported_platform_never_claims_operating_system_probe_success() {
    use symbiote_host_inventory::linux::*;
    let p = pulse();
    let observation = LinuxObservation {
        os: Fact::Known("macos".into()),
        architecture: Fact::Known("aarch64".into()),
        resources: ResourceObservation::default(),
        observed_at: Timestamp(10),
        failures: vec![
            ProbeFailure {
                resource: ProbeResource::PhysicalMemory,
                reason: ProbeError::UnsupportedPlatform,
            },
            ProbeFailure {
                resource: ProbeResource::LogicalCpuCount,
                reason: ProbeError::UnsupportedPlatform,
            },
        ],
    };
    let result = HostPulse::from_observation(p.host_id, p.observation_id, observation).unwrap();
    assert_eq!(result.provenance.source, PulseSource::UnsupportedPlatform);
    assert_eq!(result.os, Fact::Unknown);
    assert_eq!(result.probe_failures.len(), 2);
    assert_eq!(
        result.qualify(&query(), Timestamp(11)),
        Err(PulseError::Unknown)
    );
}
