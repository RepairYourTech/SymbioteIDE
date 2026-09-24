use std::{
    fs,
    os::unix::fs::{PermissionsExt, symlink},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};
use symbiote_domain::*;
use symbiote_runtime_discovery::{
    ConfigRoot, ConfigRootSource, ExecutableInspection, ExecutableInstallation,
    INSTALLATION_VERSION, InstallationChannel, InstallationError, InterfaceIdentity,
    ProbeProvenance, ProtocolEvidence, UpdateAvailability,
};

static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);

fn root() -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "symbiote-installation-{}-{}",
        std::process::id(),
        NEXT_ROOT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&path).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
    path
}

fn executable(root: &Path, name: &str, mode: u32) -> PathBuf {
    let target = root.join(format!("{name}.real"));
    fs::write(&target, b"not an executable payload").unwrap();
    fs::set_permissions(&target, fs::Permissions::from_mode(mode)).unwrap();
    let link = root.join(name);
    symlink(&target, &link).unwrap();
    link
}

fn provenance() -> ProbeProvenance {
    ProbeProvenance {
        probe_id: CommandId::new("installation-probe").unwrap(),
        source: symbiote_runtime_discovery::ObservationSource::TrustedHostProbe,
        adapter_revision: "v1".into(),
    }
}

fn request<'a>(root: &'a Path, path: &'a Path) -> ExecutableInspection<'a> {
    ExecutableInspection {
        installation_id: InstallationId::new("installation").unwrap(),
        host_id: HostId::new("host").unwrap(),
        runtime_kind: RuntimeKind::ExternalHarness,
        adapter_id: AgentRuntimeAdapterId::new("adapter").unwrap(),
        requested_path: path,
        trusted_root: root,
        channel: InstallationChannel::System,
        observed_at: Timestamp(10),
        expires_at: Timestamp(20),
        provenance: provenance(),
    }
}

fn config_root() -> ConfigRoot {
    ConfigRoot {
        identity: "config_codex_default".into(),
        path: "/home/agent/.codex".into(),
        source: ConfigRootSource::IsolatedHome,
    }
}

#[test]
fn metadata_inspection_does_not_execute_the_candidate_and_roundtrips_unknowns() {
    let root = root();
    let path = executable(&root, "tool", 0o755);
    let before = fs::read(&path).unwrap();
    let record = ExecutableInstallation::inspect(request(&root, &path)).unwrap();
    assert_eq!(record.schema_version, INSTALLATION_VERSION);
    assert_eq!(record.executable.requested_path, path.to_str().unwrap());
    assert!(record.executable.resolved_path.ends_with("tool.real"));
    assert_eq!(record.channel, InstallationChannel::System);
    assert_eq!(record.version, symbiote_runtime_discovery::Fact::Unknown);
    assert_eq!(
        record.sdk_version,
        symbiote_runtime_discovery::Fact::Unknown
    );
    assert_eq!(record.interface, symbiote_runtime_discovery::Fact::Unknown);
    assert_eq!(record.update_availability, UpdateAvailability::Unknown);
    assert_eq!(fs::read(&path).unwrap(), before);
    let encoded = serde_json::to_string(&record).unwrap();
    assert_eq!(
        serde_json::from_str::<ExecutableInstallation>(&encoded).unwrap(),
        record
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn config_roots_are_bounded_before_a_protocol_confirmation() {
    let root = root();
    let path = executable(&root, "tool", 0o755);
    let record = ExecutableInstallation::inspect(request(&root, &path)).unwrap();
    let roots = (0..=32)
        .map(|index| ConfigRoot {
            identity: format!("config_root_{index}"),
            path: format!("/tmp/config-root-{index}"),
            source: ConfigRootSource::ProfileDefault,
        })
        .collect();
    assert_eq!(
        record.confirm_protocol(
            ProtocolEvidence {
                version: "1.0.0".into(),
                interface: InterfaceIdentity {
                    name: "app-server".into(),
                    version: "1".into()
                },
                sdk_version: None,
                config_roots: roots,
            },
            provenance(),
            Timestamp(11),
            Timestamp(30),
        ),
        Err(InstallationError::ResourceLimit)
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn protocol_confirmation_publishes_only_the_facts_the_probe_supplied() {
    let root = root();
    let path = executable(&root, "tool", 0o755);
    let record = ExecutableInstallation::inspect(request(&root, &path)).unwrap();
    let record = record
        .confirm_protocol(
            ProtocolEvidence {
                version: "0.118.0".into(),
                interface: InterfaceIdentity {
                    name: "app-server".into(),
                    version: "0.118.0".into(),
                },
                sdk_version: None,
                config_roots: vec![config_root()],
            },
            provenance(),
            Timestamp(11),
            Timestamp(30),
        )
        .unwrap();
    assert_eq!(
        record.version,
        symbiote_runtime_discovery::Fact::Known("0.118.0".into())
    );
    assert_eq!(
        record.sdk_version,
        symbiote_runtime_discovery::Fact::Unknown
    );
    assert_eq!(record.config_roots, vec![config_root()]);
    assert_eq!(record.observed_at, Timestamp(11));
    assert_eq!(record.expires_at, Timestamp(30));
    assert!(
        serde_json::to_string(&record)
            .unwrap()
            .contains("config_codex_default")
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn outside_escape_mutable_and_missing_candidates_are_named_refusals() {
    let root = root();
    let outside = root
        .parent()
        .unwrap()
        .join("symbiote-installation-outside-target");
    fs::write(&outside, b"outside").unwrap();
    fs::set_permissions(&outside, fs::Permissions::from_mode(0o755)).unwrap();
    let outside_link = root.join("outside");
    symlink(&outside, &outside_link).unwrap();
    assert_eq!(
        ExecutableInstallation::inspect(request(&root, &outside_link)),
        Err(InstallationError::OutsideTrustedRoot)
    );
    fs::remove_file(&outside).unwrap();
    let inside = executable(&root, "inside", 0o755);
    let external_alias = root
        .parent()
        .unwrap()
        .join("symbiote-installation-alias-target");
    symlink(&inside, &external_alias).unwrap();
    assert_eq!(
        ExecutableInstallation::inspect(request(&root, &external_alias)),
        Err(InstallationError::OutsideTrustedRoot)
    );
    fs::remove_file(&external_alias).unwrap();

    let escape = root.join("escape");
    let escape_target = root
        .parent()
        .unwrap()
        .join("symbiote-installation-escape-target");
    fs::write(&escape_target, b"outside").unwrap();
    fs::set_permissions(&escape_target, fs::Permissions::from_mode(0o755)).unwrap();
    symlink(&escape_target, &escape).unwrap();
    assert_eq!(
        ExecutableInstallation::inspect(request(&root, &escape)),
        Err(InstallationError::OutsideTrustedRoot)
    );
    fs::remove_file(&escape).unwrap();
    fs::remove_file(&escape_target).unwrap();

    let mutable = executable(&root, "mutable", 0o777);
    assert_eq!(
        ExecutableInstallation::inspect(request(&root, &mutable)),
        Err(InstallationError::MutableExecutable)
    );
    let non_executable = executable(&root, "data", 0o644);
    assert_eq!(
        ExecutableInstallation::inspect(request(&root, &non_executable)),
        Err(InstallationError::NotExecutable)
    );
    let mutable_parent = root.join("mutable-parent");
    fs::create_dir(&mutable_parent).unwrap();
    fs::set_permissions(&mutable_parent, fs::Permissions::from_mode(0o777)).unwrap();
    let parent_executable = mutable_parent.join("tool.real");
    fs::write(&parent_executable, b"child").unwrap();
    fs::set_permissions(&parent_executable, fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(
        ExecutableInstallation::inspect(request(&root, &parent_executable)),
        Err(InstallationError::MutableExecutable)
    );
    let directory = root.join("directory");
    fs::create_dir(&directory).unwrap();
    assert_eq!(
        ExecutableInstallation::inspect(request(&root, &directory)),
        Err(InstallationError::NotExecutable)
    );
    assert_eq!(
        ExecutableInstallation::inspect(request(&root, &root.join("missing"))),
        Err(InstallationError::MissingExecutable)
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn protocol_evidence_and_wire_validation_refuse_claims_that_were_not_observed() {
    let root = root();
    let path = executable(&root, "tool", 0o755);
    let record = ExecutableInstallation::inspect(request(&root, &path)).unwrap();
    for evidence in [
        ProtocolEvidence {
            version: "not a version".into(),
            interface: InterfaceIdentity {
                name: "app-server".into(),
                version: "1".into(),
            },
            sdk_version: None,
            config_roots: vec![],
        },
        ProtocolEvidence {
            version: "1.0.0".into(),
            interface: InterfaceIdentity {
                name: "bad interface".into(),
                version: "1".into(),
            },
            sdk_version: None,
            config_roots: vec![],
        },
        ProtocolEvidence {
            version: "1.0.0".into(),
            interface: InterfaceIdentity {
                name: "app-server".into(),
                version: "1".into(),
            },
            sdk_version: Some("bad sdk".into()),
            config_roots: vec![],
        },
    ] {
        assert_eq!(
            record
                .clone()
                .confirm_protocol(evidence, provenance(), Timestamp(11), Timestamp(30)),
            Err(InstallationError::InvalidProtocolEvidence)
        );
    }
    let duplicate = ProtocolEvidence {
        version: "1.0.0".into(),
        interface: InterfaceIdentity {
            name: "app-server".into(),
            version: "1".into(),
        },
        sdk_version: None,
        config_roots: vec![config_root(), config_root()],
    };
    assert_eq!(
        record
            .clone()
            .confirm_protocol(duplicate, provenance(), Timestamp(11), Timestamp(30)),
        Err(InstallationError::InvalidProtocolEvidence)
    );
    assert_eq!(
        record.clone().confirm_protocol(
            ProtocolEvidence {
                version: "1.0.0".into(),
                interface: InterfaceIdentity {
                    name: "app-server".into(),
                    version: "1".into()
                },
                sdk_version: None,
                config_roots: vec![ConfigRoot {
                    identity: "account_secret".into(),
                    path: "/tmp/secret".into(),
                    source: ConfigRootSource::ProfileDefault,
                }],
            },
            provenance(),
            Timestamp(11),
            Timestamp(30),
        ),
        Err(InstallationError::InvalidProtocolEvidence)
    );
    let valid = ProtocolEvidence {
        version: "1.0.0".into(),
        interface: InterfaceIdentity {
            name: "app-server".into(),
            version: "1".into(),
        },
        sdk_version: None,
        config_roots: vec![config_root()],
    };
    assert_eq!(
        record
            .clone()
            .confirm_protocol(valid.clone(), provenance(), Timestamp(9), Timestamp(30)),
        Err(InstallationError::Stale)
    );
    assert_eq!(
        record
            .clone()
            .confirm_protocol(valid, provenance(), Timestamp(11), Timestamp(11)),
        Err(InstallationError::InvalidObservation)
    );
    let mut wire = serde_json::to_value(&record).unwrap();
    wire["schema_version"] = serde_json::json!(2);
    assert!(serde_json::from_value::<ExecutableInstallation>(wire).is_err());
    let mut wire = serde_json::to_value(&record).unwrap();
    wire["update_availability"] = serde_json::json!("current");
    assert!(serde_json::from_value::<ExecutableInstallation>(wire).is_err());
    fs::remove_dir_all(root).unwrap();
}
