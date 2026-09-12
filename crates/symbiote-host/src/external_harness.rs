//! The production external-harness transport (#218/#465): runs the operator's
//! configured harness program inside the Linux sandbox (bubblewrap via the
//! trusted launcher), in the dispatch's reserved worktree, under the
//! address-space ceiling the dispatch's declared `max_memory_bytes` reduces
//! to. It is the external-lane counterpart of the sandboxed shell executor:
//! the sandbox provides OS containment and re-derives the command
//! fingerprint, and the Host — never the caller — decides the ceiling.
//!
//! Why this exists: the external lane's readiness answer promises the declared
//! memory bound is enforceable. A caller-supplied transport that starts the
//! harness as a process could quietly run it under no ceiling, so the Host
//! hands the transport the declared [`ProcessBound`](symbiote_domain::ProcessBound)
//! and requires it to report what it did with the harness process
//! ([`RunnerError::ExternalBoundRefused`] otherwise). This factory is the one
//! production transport that starts a process, and it starts it under exactly
//! that ceiling.
use std::path::PathBuf;

use symbiote_domain::{CommandId, Timestamp};
use symbiote_runtime_transport::TransportLimits;
use symbiote_sandbox::{LaunchRequest, Profile};
use symbiote_trust::{ResourceConsent, ResourceSnapshot};

use crate::runner::{
    ExternalHarness, ExternalHarnessInputs, ExternalTransportFactory, HarnessProcess,
};

/// The operator's harness launch configuration. Every field is operator
/// authority, never client input: the program and its literal arguments, the
/// trusted sandbox launcher, and the Host directories the sandbox keeps out
/// of the harness's reach. The dispatch's declared ceiling is NOT here — the
/// Host supplies it per run, so the operator cannot widen or omit it.
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

/// Builds one sandboxed external harness transport per run, under the
/// dispatch's declared ceiling.
pub struct SandboxedHarnessFactory {
    pub config: HarnessLaunchConfig,
}

impl SandboxedHarnessFactory {
    pub fn new(config: HarnessLaunchConfig) -> Self {
        Self { config }
    }

    /// The command fingerprint the sandbox independently re-derives and
    /// compares against the consent's snapshot: program, literal arguments,
    /// worktree, Root and profile. A widened command fails the sandbox's own
    /// check rather than executing.
    fn consent(&self, inputs: &ExternalHarnessInputs<'_>) -> Result<ResourceConsent, &'static str> {
        let fingerprint = symbiote_sandbox::fingerprint_command(
            inputs.root_id,
            inputs.worktree,
            Profile::ReadOnly,
            &self.config.program,
            &self.config.args,
        )
        .map_err(|_| "harness command rejected")?;
        Ok(ResourceConsent {
            id: CommandId::new(format!("harness-{}", inputs.project_id))
                .map_err(|_| "consent id")?,
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
}

impl ExternalTransportFactory for SandboxedHarnessFactory {
    fn build(
        &mut self,
        inputs: ExternalHarnessInputs<'_>,
    ) -> Result<ExternalHarness, &'static str> {
        let consent = self.consent(&inputs)?;
        let ceiling = inputs.bound.address_space_bytes;
        let process = symbiote_sandbox::launch(LaunchRequest {
            helper_path: &self.config.launcher_path,
            consent: &consent,
            snapshot: &consent.snapshot,
            policy: inputs.access,
            host: inputs.host,
            at: inputs.at,
            root_id: inputs.root_id,
            worktree: inputs.worktree,
            protected_paths: &self.config.protected_paths,
            profile: Profile::ReadOnly,
            program: &self.config.program,
            args: &self.config.args,
            limits: TransportLimits::default(),
            address_space_bytes: ceiling,
        })
        .map_err(|_| "harness launch refused")?;
        Ok(ExternalHarness {
            transport: Box::new(symbiote_external_agent::process::CodexServerProcess::new(
                process,
            )),
            // The harness runs as an OS process under exactly the dispatch's
            // declared ceiling — the value the runner verifies.
            process: HarnessProcess::Ceiling(ceiling),
        })
    }
}
