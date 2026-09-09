//! Offline compatibility proof with fixture consent, not Host activation
//! authority: launch the pinned Codex App Server inside the Symbiote sandbox,
//! complete the driver's real handshake (initialize → pinned version →
//! initialized → thread/start), and stop. No turn starts, so no model is
//! called, no credential is read (fresh disposable HOME) and nothing is
//! spent. Usage:
//!   codex_thread_smoke <helper> <empty-worktree> <protected-dir>...
use serde_json::json;
use std::{collections::BTreeSet, path::PathBuf};
use symbiote_domain::*;
use symbiote_external_agent::CodexTransport;
use symbiote_external_agent::process::{codex_launch_fingerprint, launch_sandboxed};
use symbiote_sandbox::Profile;
use symbiote_trust::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let inputs: Vec<_> = std::env::args_os().skip(1).map(PathBuf::from).collect();
    if inputs.len() != 3 {
        return Err(
            "expected absolute helper, private empty worktree, protected Host directory".into(),
        );
    }
    if std::fs::read_dir(&inputs[1])?.next().is_some() {
        return Err("thread smoke requires an empty worktree".into());
    }
    let root = RootId::new("smoke-root")?;
    let profile = Profile::ReadOnly;
    let snapshot = ResourceSnapshot {
        project_id: ProjectId::new("smoke-project")?,
        role_id: RoleId::new("smoke-role")?,
        profile_id: RuntimeProfileId::new("smoke-profile")?,
        host_id: HostId::new("smoke-host")?,
        resource_ref: "offline-codex-thread-smoke".into(),
        fingerprint: codex_launch_fingerprint(&root, &inputs[1], profile)?,
        access: AccessSnapshot {
            project_id: ProjectId::new("smoke-project")?,
            roots: BTreeSet::from([root.clone()]),
            grants: BTreeSet::from([Permission::ReadRoot, Permission::ExecuteProcess]),
            policy_revision: Revision(1),
        },
    };
    let consent = ResourceConsent {
        id: CommandId::new("smoke-consent")?,
        snapshot: snapshot.clone(),
        user_id: UserId::new("smoke-user")?,
        issued_at: Timestamp(1),
        expires_at: Timestamp(3),
        revoked_at: None,
    };
    let mut server = launch_sandboxed(
        &inputs[0],
        &consent,
        &snapshot.access,
        &snapshot.host_id,
        Timestamp(2),
        &root,
        &inputs[1],
        &inputs[2..],
        profile,
        symbiote_runtime_transport::TransportLimits::default(),
    )?;
    // Real handshake against the real binary through the real framing.
    let initialized = server.call(
        "initialize",
        &json!({"clientInfo": {"name": "symbiote", "version": "0.1.0", "title": "Symbiote"}}),
    )?;
    let user_agent = initialized
        .get("userAgent")
        .and_then(serde_json::Value::as_str)
        .ok_or("server did not report a user agent")?
        .to_owned();
    let expected = format!(
        "symbiote/{}",
        symbiote_runtime_discovery::codex::CODEX_VERSION
    );
    if user_agent.split_whitespace().next() != Some(&expected) {
        return Err(format!("unexpected server version: {user_agent}").into());
    }
    server.notify("initialized", &json!({}))?;
    let thread = server.call(
        "thread/start",
        &json!({
            "cwd": "/workspace",
            "sandbox": "read-only",
            "approvalPolicy": "never",
        }),
    )?;
    let thread_id = thread
        .get("thread")
        .and_then(|t| t.get("id"))
        .and_then(serde_json::Value::as_str)
        .ok_or("server did not report a thread id")?
        .to_owned();
    let _cleanup = server.io().cancel(std::time::Duration::from_secs(2))?;
    println!(
        "{}",
        serde_json::json!({
            "server_version": expected,
            "handshake": "initialize_initialized_ok",
            "thread_started": !thread_id.is_empty(),
            "turn_started": false,
            "model_called": false,
            "credentials_mounted": false,
            "profile": "fresh_disposable_home",
            "network": "denied_by_sandbox"
        })
    );
    Ok(())
}
