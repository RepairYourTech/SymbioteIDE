//! Host-local installation observations. Filesystem inspection is metadata-only:
//! it never runs a candidate, infers an account, or grants update authority. A
//! later protocol probe may confirm version/interface facts; unknown SDK and
//! update facts stay unknown.

use crate::{Fact, InterfaceIdentity, ProbeProvenance};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    fmt,
    path::{Component, Path, PathBuf},
};
use symbiote_domain::*;

pub const INSTALLATION_VERSION: u32 = 1;
const MAX_CONFIG_ROOTS: usize = 32;
const MAX_PATH_BYTES: usize = 4096;
const MAX_OBSERVATION_AGE: u64 = 86_400_000;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExecutableIdentity {
    /// The path the Host was asked to inspect, retained without executing it.
    pub requested_path: String,
    /// The canonical target after symlink resolution, still inside the trusted root.
    pub resolved_path: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum InstallationChannel {
    System,
    User,
    Vendor,
    Portable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ConfigRootSource {
    EnvironmentOverride,
    ProfileDefault,
    IsolatedHome,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ConfigRoot {
    /// Host-owned opaque identity; never an account name, token or email.
    pub identity: String,
    /// Exact local path, retained only in the Host-local observation.
    pub path: String,
    pub source: ConfigRootSource,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum UpdateAvailability {
    /// This slice has no authenticated update channel or consent flow.
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(try_from = "InstallationWire")]
pub struct ExecutableInstallation {
    pub schema_version: u32,
    pub installation_id: InstallationId,
    pub host_id: HostId,
    pub runtime_kind: RuntimeKind,
    pub adapter_id: AgentRuntimeAdapterId,
    pub channel: InstallationChannel,
    pub executable: ExecutableIdentity,
    pub version: Fact<String>,
    pub sdk_version: Fact<String>,
    pub interface: Fact<InterfaceIdentity>,
    pub config_roots: Vec<ConfigRoot>,
    pub update_availability: UpdateAvailability,
    pub observed_at: Timestamp,
    pub expires_at: Timestamp,
    pub provenance: ProbeProvenance,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct InstallationWire {
    schema_version: u32,
    installation_id: InstallationId,
    host_id: HostId,
    runtime_kind: RuntimeKind,
    adapter_id: AgentRuntimeAdapterId,
    channel: InstallationChannel,
    executable: ExecutableIdentity,
    version: Fact<String>,
    sdk_version: Fact<String>,
    interface: Fact<InterfaceIdentity>,
    config_roots: Vec<ConfigRoot>,
    update_availability: UpdateAvailability,
    observed_at: Timestamp,
    expires_at: Timestamp,
    provenance: ProbeProvenance,
}

impl TryFrom<InstallationWire> for ExecutableInstallation {
    type Error = InstallationError;
    fn try_from(wire: InstallationWire) -> Result<Self, Self::Error> {
        let value = Self {
            schema_version: wire.schema_version,
            installation_id: wire.installation_id,
            host_id: wire.host_id,
            runtime_kind: wire.runtime_kind,
            adapter_id: wire.adapter_id,
            channel: wire.channel,
            executable: wire.executable,
            version: wire.version,
            sdk_version: wire.sdk_version,
            interface: wire.interface,
            config_roots: wire.config_roots,
            update_availability: wire.update_availability,
            observed_at: wire.observed_at,
            expires_at: wire.expires_at,
            provenance: wire.provenance,
        };
        value.validate()?;
        Ok(value)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InstallationError {
    InvalidPath,
    MissingExecutable,
    NotDirectory,
    OutsideTrustedRoot,
    NotExecutable,
    MutableExecutable,
    InvalidObservation,
    InvalidProtocolEvidence,
    ResourceLimit,
    Stale,
}

impl fmt::Display for InstallationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "installation observation refused: {self:?}")
    }
}
impl std::error::Error for InstallationError {}

#[derive(Clone, Debug)]
pub struct ExecutableInspection<'a> {
    pub installation_id: InstallationId,
    pub host_id: HostId,
    pub runtime_kind: RuntimeKind,
    pub adapter_id: AgentRuntimeAdapterId,
    pub requested_path: &'a Path,
    pub trusted_root: &'a Path,
    pub channel: InstallationChannel,
    pub observed_at: Timestamp,
    pub expires_at: Timestamp,
    pub provenance: ProbeProvenance,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProtocolEvidence {
    pub version: String,
    pub interface: InterfaceIdentity,
    pub sdk_version: Option<String>,
    pub config_roots: Vec<ConfigRoot>,
}

fn valid_path(path: &str) -> bool {
    let path = Path::new(path);
    !path.as_os_str().is_empty()
        && path.is_absolute()
        && path.as_os_str().len() <= MAX_PATH_BYTES
        && path
            .components()
            .all(|component| matches!(component, Component::RootDir | Component::Normal(_)))
}

fn version(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._+-".contains(&byte))
}

fn symbol(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte))
}

fn text_path(path: &Path) -> Result<String, InstallationError> {
    let value = path.to_str().ok_or(InstallationError::InvalidPath)?;
    if valid_path(value) {
        Ok(value.to_owned())
    } else {
        Err(InstallationError::InvalidPath)
    }
}

fn validate_roots(roots: &[ConfigRoot]) -> Result<(), InstallationError> {
    if roots.len() > MAX_CONFIG_ROOTS {
        return Err(InstallationError::ResourceLimit);
    }
    let mut identities = BTreeSet::new();
    let mut paths = BTreeSet::new();
    for root in roots {
        if !root.identity.starts_with("config_")
            || !symbol(&root.identity)
            || !valid_path(&root.path)
            || !identities.insert(&root.identity)
            || !paths.insert(&root.path)
        {
            return Err(InstallationError::InvalidProtocolEvidence);
        }
    }
    Ok(())
}

fn validate_timestamp(
    observed_at: Timestamp,
    expires_at: Timestamp,
) -> Result<(), InstallationError> {
    if expires_at.0 <= observed_at.0 || expires_at.0 - observed_at.0 > MAX_OBSERVATION_AGE {
        return Err(InstallationError::InvalidObservation);
    }
    Ok(())
}

#[cfg(unix)]
fn mutable_metadata(path: &Path) -> Result<bool, InstallationError> {
    use std::os::unix::fs::PermissionsExt;
    let metadata = std::fs::metadata(path).map_err(|_| InstallationError::InvalidPath)?;
    Ok(metadata.permissions().mode() & 0o022 != 0)
}

#[cfg(not(unix))]
fn mutable_metadata(_path: &Path) -> Result<bool, InstallationError> {
    Ok(false)
}

fn validate_executable(path: &Path, trusted_root: &Path) -> Result<PathBuf, InstallationError> {
    let _requested = text_path(path)?;
    let root = path
        .parent()
        .ok_or(InstallationError::InvalidPath)?
        .to_path_buf();
    if !root.is_absolute() {
        return Err(InstallationError::InvalidPath);
    }
    let metadata = std::fs::symlink_metadata(path).map_err(|error| match error.kind() {
        std::io::ErrorKind::NotFound => InstallationError::MissingExecutable,
        _ => InstallationError::InvalidPath,
    })?;
    if !metadata.file_type().is_symlink() && !metadata.is_file() {
        return Err(InstallationError::NotExecutable);
    }
    let resolved = std::fs::canonicalize(path).map_err(|_| InstallationError::MissingExecutable)?;
    if !resolved.is_absolute() || !text_path(&resolved).is_ok() {
        return Err(InstallationError::InvalidPath);
    }
    let target = std::fs::metadata(&resolved).map_err(|_| InstallationError::MissingExecutable)?;
    if !target.is_file() {
        return Err(InstallationError::NotExecutable);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = target.permissions().mode();
        if mode & 0o111 == 0 {
            return Err(InstallationError::NotExecutable);
        }
    }
    if mutable_metadata(&resolved)? {
        return Err(InstallationError::MutableExecutable);
    }
    let mut parent = resolved.parent().ok_or(InstallationError::InvalidPath)?;
    while parent.starts_with(trusted_root) {
        if !parent.is_dir() || mutable_metadata(parent)? {
            return Err(InstallationError::MutableExecutable);
        }
        if parent == trusted_root {
            return Ok(resolved);
        }
        parent = parent
            .parent()
            .ok_or(InstallationError::OutsideTrustedRoot)?;
    }
    Err(InstallationError::OutsideTrustedRoot)
}

impl ExecutableInstallation {
    /// Inspect metadata without starting the candidate. The trusted root is a
    /// Host-owned allowlist; a symlink escaping it is refused before any use.
    pub fn inspect(request: ExecutableInspection<'_>) -> Result<Self, InstallationError> {
        validate_timestamp(request.observed_at, request.expires_at)?;
        if !request.trusted_root.is_absolute()
            || !request.trusted_root.is_dir()
            || std::fs::symlink_metadata(request.trusted_root)
                .map(|metadata| !metadata.is_dir())
                .unwrap_or(true)
        {
            return Err(InstallationError::NotDirectory);
        }
        let trusted_root = std::fs::canonicalize(request.trusted_root)
            .map_err(|_| InstallationError::NotDirectory)?;
        if !trusted_root.is_absolute() {
            return Err(InstallationError::InvalidPath);
        }
        if !request.requested_path.starts_with(&trusted_root) {
            return Err(InstallationError::OutsideTrustedRoot);
        }
        let resolved = validate_executable(request.requested_path, &trusted_root)?;
        if !resolved.starts_with(&trusted_root) {
            return Err(InstallationError::OutsideTrustedRoot);
        }
        let observation = Self {
            schema_version: INSTALLATION_VERSION,
            installation_id: request.installation_id,
            host_id: request.host_id,
            runtime_kind: request.runtime_kind,
            adapter_id: request.adapter_id,
            channel: request.channel,
            executable: ExecutableIdentity {
                requested_path: text_path(request.requested_path)?,
                resolved_path: text_path(&resolved)?,
            },
            version: Fact::Unknown,
            sdk_version: Fact::Unknown,
            interface: Fact::Unknown,
            config_roots: Vec::new(),
            update_availability: UpdateAvailability::Unknown,
            observed_at: request.observed_at,
            expires_at: request.expires_at,
            provenance: request.provenance,
        };
        observation.validate()?;
        Ok(observation)
    }

    /// Merge facts from a separately executed, bounded protocol probe. This
    /// method does not run a probe and never turns a version into an update right.
    pub fn confirm_protocol(
        mut self,
        evidence: ProtocolEvidence,
        provenance: ProbeProvenance,
        observed_at: Timestamp,
        expires_at: Timestamp,
    ) -> Result<Self, InstallationError> {
        self.validate()?;
        validate_timestamp(observed_at, expires_at)?;
        if observed_at.0 < self.observed_at.0 {
            return Err(InstallationError::Stale);
        }
        if !version(&evidence.version)
            || !symbol(&evidence.interface.name)
            || !version(&evidence.interface.version)
            || evidence
                .sdk_version
                .as_ref()
                .is_some_and(|value| !version(value))
        {
            return Err(InstallationError::InvalidProtocolEvidence);
        }
        validate_roots(&evidence.config_roots)?;
        self.version = Fact::Known(evidence.version);
        self.interface = Fact::Known(evidence.interface);
        self.sdk_version = evidence
            .sdk_version
            .map(Fact::Known)
            .unwrap_or(Fact::Unknown);
        self.config_roots = evidence.config_roots;
        self.provenance = provenance;
        self.observed_at = observed_at;
        self.expires_at = expires_at;
        self.validate()?;
        Ok(self)
    }

    pub fn validate(&self) -> Result<(), InstallationError> {
        if self.schema_version != INSTALLATION_VERSION
            || !valid_path(&self.executable.requested_path)
            || !valid_path(&self.executable.resolved_path)
            || !version(&self.provenance.adapter_revision)
        {
            return Err(InstallationError::InvalidObservation);
        }
        validate_timestamp(self.observed_at, self.expires_at)?;
        if let Fact::Known(value) = &self.version {
            if !version(value) {
                return Err(InstallationError::InvalidObservation);
            }
        }
        if let Fact::Known(value) = &self.sdk_version {
            if !version(value) {
                return Err(InstallationError::InvalidObservation);
            }
        }
        if let Fact::Known(interface) = &self.interface {
            if !symbol(&interface.name) || !version(&interface.version) {
                return Err(InstallationError::InvalidObservation);
            }
        }
        validate_roots(&self.config_roots)?;
        Ok(())
    }
}
