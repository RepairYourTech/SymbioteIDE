//! Operator provisioning for the daemon (#54 desktop-workflow path): the
//! bounded, explicit configuration that turns `run_started_dispatch` from
//! "always refuses" into a working execution path.
//!
//! Every field is operator authority, never client input: the file lives
//! outside the store, is read once at daemon start, must be 0600, and its
//! contents never enter the journal. The fixture model transport is
//! EXPLICITLY labeled — it is a scripted model turn for demonstrating the
//! dispatch/provisioning/sandbox/completion mechanics without spending; a
//! live model turn still requires the real Responses-API transport, which
//! remains gated on explicit user authorization for credentials and
//! billing. Nothing here attempts live execution.
use serde::Deserialize;
use std::path::PathBuf;

/// The daemon's operator configuration file schema. Unknown fields are
/// refused so a typo can never silently disable a control.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct OperatorConfig {
    /// The Host's reserved-location base for derived worktrees (#211).
    pub reservation_base: PathBuf,
    /// The fixture model transport for native runs: turn one proposes the
    /// optional tool call, turn two completes with `echo_text`. Clearly a
    /// fixture — the model turn is simulated; everything the daemon does
    /// around it (provisioning, sandboxed tool execution, completion
    /// evidence) is real.
    pub native_fixture: Option<NativeFixture>,
    /// The fixture harness conversation for external runs: the Codex
    /// App-Server exchange is scripted (no binary, no model turn, no
    /// spending) while everything the daemon does around it — runtime-kind
    /// strictness, harness approval refusal, worktree provisioning,
    /// completion evidence — is real. Clearly a fixture, symmetric with
    /// `native_fixture`; a live Codex turn remains gated on explicit user
    /// authorization for credentials and billing.
    pub external_fixture: Option<ExternalFixture>,
    /// The operator-provisioned sandboxed external harness: the program the
    /// external lane launches inside the sandbox under the dispatch's
    /// declared memory ceiling. When set it replaces the in-process fixture
    /// transport, so the lane runs a real, bounded process. Values and paths
    /// stay operator state. The ceiling is never here: the Host supplies the
    /// dispatch's declared bound per run.
    pub external_harness: Option<crate::external_harness::HarnessLaunchConfig>,
    /// Credential registrations for the #217 broker. Values live in this
    /// operator-owned 0600 file only — never in the store or journal.
    /// Real secret material via keychain remains pending #217; these
    /// registrations are for fixture/demo credentials.
    #[serde(default)]
    pub credential_broker: Vec<BrokerRegistration>,
    /// The sandboxed shell executor (#465/#466): the launcher path plus
    /// the operator's program allowlist — the consent authority the #507
    /// seam consults per tool invocation. An absent shell executor leaves
    /// declared shell tools propose-only.
    pub shell_executor: Option<ShellExecutorConfig>,
    /// The runtime facts the operator asserts for this Host, one declaration
    /// per runtime profile the Host may be asked to assess. The identity and
    /// the observation window of the resulting descriptor are Host-owned and
    /// never appear here; an absent declaration leaves the binding readiness
    /// report's runtime prerequisites unobserved rather than assumed.
    #[serde(default)]
    pub runtime_declarations: Vec<symbiote_runtime_sdk::DeclaredRuntime>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct NativeFixture {
    /// The turn-two completion text (the run's worker report).
    pub echo_text: String,
    /// Optional turn-one tool proposal. The binding's required_tools must
    /// declare `shell` for it to execute.
    pub tool: Option<FixtureToolCall>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ExternalFixture {
    /// The scripted thread id the harness conversation runs in.
    pub thread_id: String,
    /// The agent message the scripted turn completes with (the run's
    /// worker report).
    pub agent_message: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct FixtureToolCall {
    pub call_id: String,
    pub arguments: serde_json::Value,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BrokerRegistration {
    pub reference: String,
    pub project: String,
    pub environment: String,
    pub value: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ShellExecutorConfig {
    pub launcher_path: PathBuf,
    pub protected_paths: Vec<PathBuf>,
    /// Bare program names the consent authority may approve (the #466
    /// allowlist flow, operator-authored). Empty means the executor is
    /// attached but refuses every tool.
    pub allowed_programs: Vec<String>,
}

/// The file must exist, be bounded (64 KiB), and carry no group/other
/// permission bits — operator discipline enforced at load, not trusted
/// from convention.
pub fn load(path: &std::path::Path) -> Result<OperatorConfig, OperatorConfigError> {
    use std::os::unix::fs::PermissionsExt;
    let meta = std::fs::metadata(path).map_err(|_| OperatorConfigError::Unreadable)?;
    if !meta.is_file() || meta.permissions().mode() & 0o077 != 0 {
        return Err(OperatorConfigError::Permissions);
    }
    let file = std::fs::File::open(path).map_err(|_| OperatorConfigError::Unreadable)?;
    let mut bytes = Vec::new();
    (&file)
        .take(64 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| OperatorConfigError::Unreadable)?;
    if bytes.len() > 64 * 1024 {
        return Err(OperatorConfigError::Overbound);
    }
    serde_json::from_slice(&bytes).map_err(|_| OperatorConfigError::Invalid)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OperatorConfigError {
    Unreadable,
    Permissions,
    Overbound,
    Invalid,
}

impl std::fmt::Display for OperatorConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "operator config: {self:?}")
    }
}
impl std::error::Error for OperatorConfigError {}

use std::io::Read;
