//! Offline compatibility proof with fixture consent, not Host activation authority.
use serde_json::Value;
use std::{collections::BTreeSet, path::PathBuf, time::Duration};
use symbiote_domain::*;
use symbiote_runtime_discovery::codex::{self, CodexDiscoveryError, DiscoveryTransport};
use symbiote_runtime_transport::TransportLimits;
use symbiote_sandbox::{LaunchRequest, Profile, SandboxProcess, fingerprint_command, launch};
use symbiote_trust::*;

struct Probe(SandboxProcess);
impl DiscoveryTransport for Probe {
    fn send(&mut self, value: &Value, timeout: Duration) -> Result<(), CodexDiscoveryError> {
        self.0
            .send(value, timeout)
            .map_err(|_| CodexDiscoveryError::Transport)
    }
    fn recv(&mut self, timeout: Duration) -> Result<Value, CodexDiscoveryError> {
        let value = self
            .0
            .recv(timeout)
            .map_err(|_| CodexDiscoveryError::Transport)?;
        if value.pointer("/result/userAgent").is_some()
            && value.pointer("/result/codexHome").and_then(Value::as_str)
                != Some("/home/agent/.codex")
        {
            return Err(CodexDiscoveryError::MalformedFrame);
        }
        Ok(value)
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
    let mut probe = Probe(launch(LaunchRequest {
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
        program: "/usr/bin/codex",
        args: &args,
        limits: TransportLimits::default(),
    })?);
    let report = codex::discover(&mut probe, Duration::from_secs(20));
    let _cleanup = probe.0.cancel(Duration::from_secs(2))?;
    let report = report?;
    if report.authentication != codex::CodexAuthentication::Required {
        return Err("fresh isolated profile did not report authentication required".into());
    }
    println!(
        "{}",
        serde_json::json!({
            "server_version": report.server_version,
            "authentication": report.authentication,
            "listed_models": report.models.len(),
            "model_usability": "unknown",
            "model_turn_started": false,
            "profile": "fresh_disposable_home",
            "network": "denied_by_sandbox"
        })
    );
    Ok(())
}
