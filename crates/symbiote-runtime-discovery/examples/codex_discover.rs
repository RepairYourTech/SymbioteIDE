//! Offline compatibility proof with fixture consent, not Host activation authority.
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    time::Duration,
};
use symbiote_domain::*;
use symbiote_runtime_discovery::{
    ConfigRoot, ExecutableInspection, ExecutableInstallation, InstallationChannel,
    InterfaceIdentity, Inventory, ProbeProvenance, ProfileSpec, ProtocolEvidence,
    RuntimeInstanceIntent,
    codex::{self, CodexDiscoveryError, DiscoveryTransport},
    codex_adapter, resolve_profiles,
};
use symbiote_runtime_transport::TransportLimits;
use symbiote_sandbox::{LaunchRequest, Profile, SandboxProcess, fingerprint_command, launch};
use symbiote_trust::*;

struct Probe {
    process: SandboxProcess,
}
impl DiscoveryTransport for Probe {
    fn send(&mut self, value: &Value, timeout: Duration) -> Result<(), CodexDiscoveryError> {
        self.process
            .send(value, timeout)
            .map_err(|_| CodexDiscoveryError::Transport)
    }
    fn recv(&mut self, timeout: Duration) -> Result<Value, CodexDiscoveryError> {
        self.process
            .recv(timeout)
            .map_err(|_| CodexDiscoveryError::Transport)
    }
    fn child_id(&self) -> Option<u32> {
        Some(self.process.child_id())
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let inputs: Vec<_> = std::env::args_os().skip(1).map(PathBuf::from).collect();
    if inputs.len() != 3 {
        return Err(
            "expected absolute helper, private empty worktree, protected Host directory".into(),
        );
    }
    if std::fs::read_dir(&inputs[1])?.next().is_some() {
        return Err("discovery proof requires an empty worktree".into());
    }
    let root = RootId::new("discovery-root")?;
    let args = vec!["app-server".into(), "--listen".into(), "stdio://".into()];
    let snapshot = ResourceSnapshot {
        project_id: ProjectId::new("discovery-project")?,
        role_id: RoleId::new("discovery-role")?,
        profile_id: RuntimeProfileId::new("discovery-profile")?,
        host_id: HostId::new("discovery-host")?,
        resource_ref: "offline-codex-discovery".into(),
        fingerprint: fingerprint_command(
            &root,
            &inputs[1],
            Profile::ReadOnly,
            "/usr/bin/codex",
            &args,
        )?,
        access: AccessSnapshot {
            project_id: ProjectId::new("discovery-project")?,
            roots: BTreeSet::from([root.clone()]),
            grants: BTreeSet::from([Permission::ReadRoot, Permission::ExecuteProcess]),
            policy_revision: Revision(1),
        },
    };
    let consent = ResourceConsent {
        id: CommandId::new("discovery-consent")?,
        snapshot: snapshot.clone(),
        user_id: UserId::new("discovery-user")?,
        issued_at: Timestamp(1),
        expires_at: Timestamp(3),
        revoked_at: None,
    };
    let profile = resolve_profiles(
        &[ProfileSpec {
            profile_id: RuntimeProfileId::new("discovery-profile")?,
            instance_name: "Disposable Codex profile".into(),
            config_identity: "config_codex_disposable".into(),
            environment_variable: "CODEX_HOME".into(),
            default_relative_path: ".codex".into(),
            account_ref: None,
        }],
        &BTreeMap::new(),
        Path::new("/home/agent"),
        Timestamp(1),
        Timestamp(3),
        ProbeProvenance {
            probe_id: CommandId::new("codex-profile-resolution")?,
            source: symbiote_runtime_discovery::ObservationSource::TrustedHostProbe,
            adapter_revision: "v1".into(),
        },
    )?
    .into_iter()
    .next()
    .ok_or("profile resolution returned no profile")?;
    let installation = ExecutableInstallation::inspect(ExecutableInspection {
        installation_id: InstallationId::new("codex-system-installation")?,
        host_id: HostId::new("discovery-host")?,
        runtime_kind: RuntimeKind::ExternalHarness,
        adapter_id: AgentRuntimeAdapterId::new("codex-harness")?,
        requested_path: std::path::Path::new("/usr/bin/codex"),
        trusted_root: std::path::Path::new("/usr"),
        channel: InstallationChannel::System,
        observed_at: Timestamp(1),
        expires_at: Timestamp(3),
        provenance: ProbeProvenance {
            probe_id: CommandId::new("codex-file-inspection")?,
            source: symbiote_runtime_discovery::ObservationSource::TrustedHostProbe,
            adapter_revision: "v1".into(),
        },
    })?;
    let mut probe = Probe {
        process: launch(LaunchRequest {
            helper_path: &inputs[0],
            consent: &consent,
            snapshot: &snapshot,
            policy: &snapshot.access,
            host: &snapshot.host_id,
            at: Timestamp(2),
            root_id: &root,
            worktree: &inputs[1],
            protected_paths: &inputs[2..],
            profile: Profile::ReadOnly,
            // An explicit ceiling for this read-only probe, not a claim about a
            // dispatch limit. Codex's Node launcher reserves address space before
            // the native App Server starts, so the proof needs more virtual room
            // than the smallest synthetic dispatch fixture.
            address_space_bytes: 2 << 30,
            program: "/usr/bin/codex",
            args: &args,
            limits: TransportLimits::default(),
        })?,
    };
    let report = codex::discover(&mut probe, Duration::from_secs(20));
    let _cleanup = probe.process.cancel(Duration::from_secs(2))?;
    let report = report?;
    let installation = installation.confirm_protocol(
        ProtocolEvidence {
            version: report.server_version.clone(),
            interface: InterfaceIdentity {
                name: "app-server".into(),
                version: report.server_version.clone(),
            },
            sdk_version: None,
            config_roots: vec![ConfigRoot {
                identity: profile.config_identity.clone(),
                path: profile.path.clone(),
                source: profile.source,
            }],
        },
        ProbeProvenance {
            probe_id: CommandId::new("codex-app-server")?,
            source: symbiote_runtime_discovery::ObservationSource::SandboxedProbe,
            adapter_revision: "v1".into(),
        },
        Timestamp(2),
        Timestamp(3),
    )?;
    if report.authentication != codex::CodexAuthentication::Required {
        return Err("fresh isolated profile did not report authentication required".into());
    }
    let intent = RuntimeInstanceIntent {
        profile_id: profile.profile_id.clone(),
        installation_id: installation.installation_id.clone(),
        host_id: installation.host_id.clone(),
        runtime_kind: installation.runtime_kind,
        adapter_id: installation.adapter_id.clone(),
        instance_name: profile.instance_name.clone(),
        config_identity: profile.config_identity.clone(),
    };
    let inventory = Inventory::new(vec![codex_adapter::bind_report(
        intent,
        &profile,
        &installation,
        probe.process.child_id(),
        &report,
    )?])?;
    println!(
        "{}",
        serde_json::json!({
            "server_version": report.server_version,
            "executable_path": installation.executable.requested_path,
            "resolved_executable_path": installation.executable.resolved_path,
            "executable_version": installation.version,
            "sdk_version": installation.sdk_version,
            "interface": installation.interface,
            "config_root_identity": installation.config_roots[0].identity,
            "config_root_path": installation.config_roots[0].path,
            "install_channel": installation.channel,
            "update_availability": installation.update_availability,
            "authentication": report.authentication,
            "listed_models": report.models.len(),
            "inventory_records": inventory.records().len(),
            "inventory_schema_version": inventory.schema_version(),
            "model_usability": "unknown",
            "model_turn_started": false,
            "profile": "fresh_disposable_home",
            "network": "denied_by_sandbox"
        })
    );
    Ok(())
}
