//! The production external-harness launch (#218/#465): the Host runs the
//! operator's configured harness program inside the Linux sandbox (bubblewrap
//! via the trusted launcher), in the dispatch's reserved worktree, under the
//! address-space ceiling the dispatch's declared `max_memory_bytes` reduces to.
//! It is the external-lane counterpart of the sandboxed shell executor: the
//! sandbox provides OS containment and re-derives the command fingerprint, and
//! the Host — never a transport — decides the ceiling.
//!
//! Why the Host launches it: the external lane's readiness answer promises the
//! declared memory bound is enforceable. If a transport owned the launch, a
//! transport could start the harness under no ceiling while reporting that it
//! had applied one, and the Host would have only its word for it. Here the
//! Host builds the [`LaunchRequest`] itself, so there is nothing to report and
//! nothing to trust: the ceiling the dispatch declared is the ceiling the
//! launcher applies, or the launch refuses and no harness process exists.
//!
//! The worktree profile comes from the same fact the harness's own sandbox
//! policy does — the contract's `MutateStream` grant — so the outer sandbox
//! and the harness cannot disagree about whether the worktree may be written.
use std::path::PathBuf;

use symbiote_domain::{CommandId, Permission, Timestamp};
use symbiote_external_agent::CodexTransport;
use symbiote_runtime_transport::TransportLimits;
use symbiote_sandbox::{LaunchRequest, Profile};
use symbiote_trust::{ResourceConsent, ResourceSnapshot};

use crate::runner::ExternalHarnessInputs;

/// The operator's harness launch configuration. Every field is operator
/// authority, never client input: the program and its literal arguments, the
/// trusted sandbox launcher, and the Host directories the sandbox keeps out
/// of the harness's reach. The dispatch's declared ceiling and worktree
/// profile are NOT here — the Host supplies them per run, so the operator
/// cannot widen or omit them.
#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HarnessLaunchConfig {
    /// The trusted installed sandbox launcher (`symbiote-sandbox-launch`).
    pub launcher_path: PathBuf,
    /// Complete protected Host directories; nonempty, absolute, existing.
    pub protected_paths: Vec<PathBuf>,
    /// The harness program: an absolute `/usr/bin/...` executable.
    pub program: String,
    /// The program's literal arguments (never shell-expanded).
    #[serde(default)]
    pub args: Vec<String>,
}

/// The worktree profile the Host sandbox uses for this dispatch: writable
/// exactly when the contract's access snapshot grants stream mutation, which
/// is also what the sandbox itself requires for a writable bind. One fact
/// decides both the outer mount and the harness's own policy.
fn profile_for(inputs: &ExternalHarnessInputs<'_>) -> Profile {
    if inputs.access.grants.contains(&Permission::MutateStream) {
        Profile::WorktreeWrite
    } else {
        Profile::ReadOnly
    }
}

/// The command fingerprint the sandbox independently re-derives and compares
/// against the consent's snapshot: program, literal arguments, worktree, Root
/// and profile. A widened command fails the sandbox's own check rather than
/// executing.
fn consent(
    config: &HarnessLaunchConfig,
    inputs: &ExternalHarnessInputs<'_>,
    profile: Profile,
) -> Result<ResourceConsent, &'static str> {
    let fingerprint = symbiote_sandbox::fingerprint_command(
        inputs.root_id,
        inputs.worktree,
        profile,
        &config.program,
        &config.args,
    )
    .map_err(|_| "harness command rejected")?;
    Ok(ResourceConsent {
        id: CommandId::new(format!("harness-{}", inputs.project_id)).map_err(|_| "consent id")?,
        snapshot: ResourceSnapshot {
            project_id: inputs.project_id.clone(),
            role_id: inputs.role_id.clone(),
            profile_id: inputs.profile_id.clone(),
            host_id: inputs.host.clone(),
            resource_ref: "external-harness".into(),
            fingerprint,
            access: inputs.access.clone(),
        },
        user_id: inputs.user_id.clone(),
        issued_at: inputs.at,
        expires_at: Timestamp(inputs.at.0.saturating_add(300_000)),
        revoked_at: None,
    })
}

/// Launches the operator's harness program for one run, inside the sandbox,
/// under the dispatch's declared ceiling. The Host owns every authority-bearing
/// input ([`ExternalHarnessInputs`]); the sandbox refuses a launch it cannot
/// bound (`UnrepresentableLimit`) or cannot authorize (grants, fingerprint,
/// helper), so a failure here means no harness process exists.
pub fn launch(
    config: &HarnessLaunchConfig,
    inputs: &ExternalHarnessInputs<'_>,
) -> Result<Box<dyn CodexTransport>, &'static str> {
    let profile = profile_for(inputs);
    let consent = consent(config, inputs, profile)?;
    let process = symbiote_sandbox::launch(LaunchRequest {
        helper_path: &config.launcher_path,
        consent: &consent,
        snapshot: &consent.snapshot,
        policy: inputs.access,
        host: inputs.host,
        at: inputs.at,
        root_id: inputs.root_id,
        worktree: inputs.worktree,
        protected_paths: &config.protected_paths,
        profile,
        program: &config.program,
        args: &config.args,
        limits: TransportLimits::default(),
        // The ceiling the dispatch declared, applied by the launcher to the
        // harness process and everything it starts.
        address_space_bytes: inputs.bound.address_space_bytes,
    })
    .map_err(|_| "harness launch refused")?;
    Ok(Box::new(
        symbiote_external_agent::process::CodexServerProcess::new(process),
    ))
}
