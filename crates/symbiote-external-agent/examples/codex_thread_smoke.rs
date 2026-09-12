//! Offline compatibility proof with fixture consent, not Host activation
//! authority: launch the pinned Codex App Server inside the Symbiote sandbox
//! and drive the real driver-level handshake — `ExternalSession::begin_thread`
//! over the production `CodexServerProcess` framing (initialize → pinned
//! version → initialized → thread/start), then stop. No turn starts, so no
//! model is called, no credential is read (fresh disposable HOME) and nothing
//! is spent. Usage:
//!   codex_thread_smoke <helper> <empty-worktree> <protected-dir>...
//! The driver session needs a dispatch; the smoke proof mints a fixture
//! dispatch with fixture consent exactly like the #483 discovery example.
//! This is protocol compatibility evidence, not Host activation authority.
use std::{collections::BTreeSet, path::PathBuf};
use symbiote_domain::*;
use symbiote_external_agent::ExternalSession;
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
    // The driver sets the harness's own policy from the contract's write
    // grant, so the outer profile must come from the same grant: the smoke's
    // binding grants `MutateStream`, so the reserved worktree is writable
    // here exactly as the production lane's is.
    let profile = Profile::WorktreeWrite;
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
            grants: BTreeSet::from([
                Permission::ReadRoot,
                Permission::ExecuteProcess,
                Permission::MutateStream,
            ]),
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
    let mut server = match launch_sandboxed(
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
        // The ceiling the pinned harness runs under: the smoke proof uses a
        // fixture dispatch, so the bound is the one its limits would reduce to.
        1 << 30,
    ) {
        Ok(server) => server,
        Err(error) => {
            eprintln!("launch_sandboxed error: {error:?}");
            return Err(error.into());
        }
    };
    // The driver path, not a hand-rolled handshake: a real ExternalSession
    // validates its contract, records Ready, performs the full handshake and
    // starts the thread. The sandbox mounts the worktree at /workspace and
    // the host path is invisible, so that is the cwd the driver must send.
    let task = Task::new(
        TaskId::new("smoke-task")?,
        ProjectId::new("smoke-project")?,
        RootId::new("smoke-root")?,
        RoleId::new("smoke-role")?,
        ChangeStreamId::new("smoke-stream")?,
        VersionedTaskContract {
            id: TaskContractId::new("smoke-contract")?,
            revision: Revision(1),
        },
    );
    let dispatch = smoke_dispatch(&task)?;
    let mut session = ExternalSession::new(&dispatch, Timestamp(2))?;
    session.begin_thread("/workspace", &mut server)?;
    let thread_id = session.harness_thread_id().unwrap_or_default().to_owned();
    let stopped = session.run_summary().stopped;
    let _cleanup = server.io().cancel(std::time::Duration::from_secs(2))?;
    if stopped.is_some() {
        return Err("driver reported a stop during the handshake".into());
    }
    println!(
        "{}",
        serde_json::json!({
            "server_version": format!(
                "symbiote/{}",
                symbiote_runtime_discovery::codex::CODEX_VERSION
            ),
            "handshake": "driver_begin_thread_ok",
            "events": session.events().len(),
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

/// The minimal external-harness dispatch for the smoke session. Fixture
/// identities throughout; no real Project, Role, binding or Host exists.
fn smoke_dispatch(task: &Task) -> Result<Dispatch, Box<dyn std::error::Error>> {
    let host_id = HostId::new("smoke-host")?;
    let profile = RuntimeProfile {
        id: RuntimeProfileId::new("smoke-profile")?,
        revision: Revision(0),
        runtime: RuntimeKind::ExternalHarness,
        adapter: AgentRuntimeAdapterId::new("codex-harness")?,
        installation: Some(InstallationId::new("codex-0-118-0")?),
        provider: ProviderConnectionId::new("smoke-provider")?,
        credential: CredentialReferenceId::new("smoke-credential")?,
        billing_entitlement: BillingEntitlementId::new("smoke-entitlement")?,
        model: ModelId::new("smoke-model")?,
        eligible_hosts: BTreeSet::from([host_id.clone()]),
    };
    let binding = WorkforceBinding {
        id: BindingId::new("smoke-binding")?,
        revision: Revision(0),
        project_id: task.project_id().clone(),
        role_id: task.role_id().clone(),
        profile_id: profile.id.clone(),
        profile_revision: profile.revision,
        protocol: VersionedProtocol {
            id: ProtocolId::new("smoke-protocol")?,
            revision: Revision(1),
        },
        access: AccessSnapshot {
            project_id: task.project_id().clone(),
            roots: BTreeSet::from([task.root_id().clone()]),
            grants: BTreeSet::from([Permission::ReadRoot, Permission::MutateStream]),
            policy_revision: Revision(1),
        },
        required_controls: BTreeSet::new(),
        context: ContextPolicy {
            bundle: ContextBundleId::new("smoke-context")?,
            revision: Revision(1),
            max_input_tokens: 1024,
            reserved_output_tokens: 1024,
        },
        required_tools: BTreeSet::new(),
        required_skills: BTreeSet::new(),
        escalation: EscalationPolicy::StopAndRequestHuman,
    };
    let host = Host {
        id: host_id,
        revision: Revision(0),
        device: DeviceId::new("smoke-device")?,
        fabric: None,
        supported_runtimes: vec![RuntimeKind::ExternalHarness],
        controls: BTreeSet::from([
            symbiote_domain::Control::Filesystem,
            symbiote_domain::Control::Cancellation,
            symbiote_domain::Control::CompletionAuthority,
        ])
        .into_iter()
        .map(|control| {
            (
                control,
                EnforcementClaim {
                    strength: EnforcementStrength::HostEnforced,
                    evidence: EvidenceId::new("smoke-evidence").expect("valid fixture id"),
                    verified_at: Timestamp(1),
                    expires_at: Timestamp(1_000_000),
                },
            )
        })
        .collect(),
    };
    let role = Role {
        id: task.role_id().clone(),
        project_id: task.project_id().clone(),
        revision: Revision(0),
        name: "Smoke".into(),
        operating_contract: VersionedRoleContract {
            id: RoleContractId::new("smoke-role-contract")?,
            revision: Revision(1),
        },
    };
    Ok(Dispatch::compile(
        DispatchId::new("smoke-dispatch")?,
        RuntimeContractId::new("smoke-rtc")?,
        symbiote_domain::DispatchInputs {
            task,
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
            minimum_enforcement: &std::collections::BTreeMap::new(),
            now: Timestamp(2),
        },
    )?)
}
