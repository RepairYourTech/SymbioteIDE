//! Fixture consent only: this is not an authenticated Host approval flow.
//! Usage: sandbox_probe ABS_HELPER ABS_PRIVATE_WORKTREE ABS_PROTECTED_HOST_DIR
use std::{collections::BTreeSet, path::PathBuf, time::Duration};
use symbiote_domain::*;
use symbiote_runtime_transport::TransportLimits;
use symbiote_sandbox::*;
use symbiote_trust::*;
fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let inputs: Vec<_> = std::env::args_os().skip(1).map(PathBuf::from).collect();
    if inputs.len() != 3 {
        return Err("expected helper, private worktree, protected Host directory".into());
    }
    let project = ProjectId::new("fixture-project")?;
    let root = RootId::new("fixture-root")?;
    let args=vec!["-c".into(),"import json,os\ntry:\n open('/workspace/denied-write','w');denied=False\nexcept OSError:denied=True\nprint(json.dumps({'sandbox':True,'write_denied':denied,'home':os.environ['HOME']}),flush=True)".into()];
    let snapshot = ResourceSnapshot {
        project_id: project.clone(),
        role_id: RoleId::new("fixture-role")?,
        profile_id: RuntimeProfileId::new("fixture-runtime")?,
        host_id: HostId::new("fixture-host")?,
        resource_ref: "sandbox-probe".into(),
        fingerprint: fingerprint_command(
            &root,
            &inputs[1],
            Profile::ReadOnly,
            "/usr/bin/python3",
            &args,
        )?,
        access: AccessSnapshot {
            project_id: project,
            roots: BTreeSet::from([root.clone()]),
            grants: BTreeSet::from([Permission::ReadRoot, Permission::ExecuteProcess]),
            policy_revision: Revision(1),
        },
    };
    let consent = ResourceConsent {
        id: CommandId::new("fixture-consent")?,
        snapshot: snapshot.clone(),
        user_id: UserId::new("fixture-user")?,
        issued_at: Timestamp(1),
        expires_at: Timestamp(3),
        revoked_at: None,
    };
    let mut process = launch(LaunchRequest {
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
        // The declared memory bound of the dispatch this probe stands in for.
        address_space_bytes: 1 << 30,
        program: "/usr/bin/python3",
        args: &args,
        limits: TransportLimits::default(),
    })?;
    let evidence = process.recv(Duration::from_secs(5))?;
    if evidence != serde_json::json!({"sandbox":true,"write_denied":true,"home":"/home/agent"}) {
        return Err("sandbox probe returned unexpected evidence".into());
    }
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(exit) = process.try_wait()? {
            if exit.code != Some(0) || exit.signal.is_some() {
                return Err("sandbox probe failed".into());
            }
            break;
        }
        if std::time::Instant::now() >= deadline {
            return Err("sandbox probe exit deadline exceeded".into());
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    println!("{evidence}");
    Ok(())
}
