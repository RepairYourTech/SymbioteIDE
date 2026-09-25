//! Named config-root resolution for runtime instances. The resolver is pure:
//! it reads a supplied environment map, never changes process environment,
//! never creates a directory, and never reads credentials.

use crate::{ProbeProvenance, installation::ConfigRootSource};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    path::{Component, Path, PathBuf},
};
use symbiote_domain::*;

pub const PROFILE_VERSION: u32 = 1;
pub const MAX_PROFILE_SPECS: usize = 64;
const MAX_NAME_BYTES: usize = 128;
const MAX_PATH_BYTES: usize = 4096;
const MAX_OBSERVATION_AGE: u64 = 86_400_000;

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProfileSpec {
    pub profile_id: RuntimeProfileId,
    /// Display label only; it is never used for adapter selection.
    pub instance_name: String,
    /// Symbolic, opaque config identity. It is never an account name or path.
    pub config_identity: String,
    /// Adapter-declared environment variable, such as CODEX_HOME.
    pub environment_variable: String,
    /// Relative path beneath the supplied Home when the override is absent.
    pub default_relative_path: String,
    /// Optional Host-generated opaque account reference, never an email or token.
    pub account_ref: Option<String>,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(try_from = "ProfileObservationWire")]
pub struct ProfileObservation {
    pub schema_version: u32,
    pub profile_id: RuntimeProfileId,
    pub instance_name: String,
    pub config_identity: String,
    pub account_ref: Option<String>,
    /// Exact local path, retained only in the Host-local observation.
    pub path: String,
    pub source: ConfigRootSource,
    pub observed_at: Timestamp,
    pub expires_at: Timestamp,
    pub provenance: ProbeProvenance,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ProfileObservationWire {
    schema_version: u32,
    profile_id: RuntimeProfileId,
    instance_name: String,
    config_identity: String,
    account_ref: Option<String>,
    path: String,
    source: ConfigRootSource,
    observed_at: Timestamp,
    expires_at: Timestamp,
    provenance: ProbeProvenance,
}

impl TryFrom<ProfileObservationWire> for ProfileObservation {
    type Error = ProfileError;
    fn try_from(wire: ProfileObservationWire) -> Result<Self, Self::Error> {
        let value = Self {
            schema_version: wire.schema_version,
            profile_id: wire.profile_id,
            instance_name: wire.instance_name,
            config_identity: wire.config_identity,
            account_ref: wire.account_ref,
            path: wire.path,
            source: wire.source,
            observed_at: wire.observed_at,
            expires_at: wire.expires_at,
            provenance: wire.provenance,
        };
        value.validate()?;
        Ok(value)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProfileError {
    InvalidSpec,
    DuplicateProfile,
    DuplicateConfig,
    DuplicateEnvironment,
    DuplicatePath,
    InvalidPath,
    ResourceLimit,
    Stale,
}

impl fmt::Display for ProfileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "profile resolution refused: {self:?}")
    }
}
impl std::error::Error for ProfileError {}

fn valid_absolute(path: &Path) -> bool {
    path.is_absolute()
        && path.as_os_str().len() <= MAX_PATH_BYTES
        && path
            .components()
            .all(|component| matches!(component, Component::RootDir | Component::Normal(_)))
}

fn valid_relative(value: &str) -> bool {
    let path = Path::new(value);
    !value.is_empty()
        && value.len() <= MAX_PATH_BYTES
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

fn valid_symbol(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_NAME_BYTES
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte))
}

fn valid_environment_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_NAME_BYTES
        && value.bytes().enumerate().all(|(index, byte)| match byte {
            b'A'..=b'Z' | b'0'..=b'9' => true,
            b'_' => index > 0,
            _ => false,
        })
}

fn valid_instance_name(value: &str) -> bool {
    !value.trim().is_empty()
        && value.len() <= MAX_NAME_BYTES
        && !value.chars().any(char::is_control)
}

fn valid_account_ref(value: &str) -> bool {
    value.starts_with("account_") && valid_symbol(value)
}

fn validate_timestamp(observed_at: Timestamp, expires_at: Timestamp) -> Result<(), ProfileError> {
    if expires_at.0 <= observed_at.0 || expires_at.0 - observed_at.0 > MAX_OBSERVATION_AGE {
        return Err(ProfileError::Stale);
    }
    Ok(())
}

fn validate_spec(spec: &ProfileSpec) -> Result<(), ProfileError> {
    if !valid_instance_name(&spec.instance_name)
        || !valid_symbol(&spec.config_identity)
        || !valid_environment_name(&spec.environment_variable)
        || !valid_relative(&spec.default_relative_path)
        || spec
            .account_ref
            .as_ref()
            .is_some_and(|value| !valid_account_ref(value))
    {
        return Err(ProfileError::InvalidSpec);
    }
    Ok(())
}

fn text_path(path: &Path) -> Result<String, ProfileError> {
    if !valid_absolute(path) {
        return Err(ProfileError::InvalidPath);
    }
    let canonical: PathBuf = path.components().collect();
    canonical
        .to_str()
        .map(str::to_owned)
        .ok_or(ProfileError::InvalidPath)
}

impl ProfileObservation {
    pub fn validate(&self) -> Result<(), ProfileError> {
        if self.schema_version != PROFILE_VERSION
            || !valid_instance_name(&self.instance_name)
            || !valid_symbol(&self.config_identity)
            || self
                .account_ref
                .as_ref()
                .is_some_and(|value| !valid_account_ref(value))
            || !valid_absolute(Path::new(&self.path))
            || !valid_symbol(&self.provenance.adapter_revision)
        {
            return Err(ProfileError::InvalidSpec);
        }
        validate_timestamp(self.observed_at, self.expires_at)
    }
}

impl fmt::Debug for ProfileObservation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProfileObservation")
            .field("schema_version", &self.schema_version)
            .field("profile_id", &self.profile_id)
            .field("instance_name", &self.instance_name)
            .field("config_identity", &self.config_identity)
            .field("account_ref", &self.account_ref)
            .field("path", &"<redacted>")
            .field("source", &self.source)
            .field("observed_at", &self.observed_at)
            .field("expires_at", &self.expires_at)
            .finish()
    }
}

/// Resolve declared profiles against an explicitly supplied environment map.
/// No process environment is read or changed, and no path is created.
pub fn resolve_profiles(
    specs: &[ProfileSpec],
    environment: &BTreeMap<String, std::ffi::OsString>,
    home: &Path,
    observed_at: Timestamp,
    expires_at: Timestamp,
    provenance: ProbeProvenance,
) -> Result<Vec<ProfileObservation>, ProfileError> {
    if specs.len() > MAX_PROFILE_SPECS {
        return Err(ProfileError::ResourceLimit);
    }
    validate_timestamp(observed_at, expires_at)?;
    if !valid_absolute(home) {
        return Err(ProfileError::InvalidPath);
    }
    let mut profile_ids = BTreeSet::new();
    let mut config_ids = BTreeSet::new();
    let mut variables = BTreeSet::new();
    let mut defaults: BTreeSet<PathBuf> = BTreeSet::new();
    for spec in specs {
        validate_spec(spec)?;
        if !profile_ids.insert(spec.profile_id.clone()) {
            return Err(ProfileError::DuplicateProfile);
        }
        if !config_ids.insert(spec.config_identity.clone()) {
            return Err(ProfileError::DuplicateConfig);
        }
        if !variables.insert(spec.environment_variable.clone()) {
            return Err(ProfileError::DuplicateEnvironment);
        }
        if !defaults.insert(
            Path::new(&spec.default_relative_path)
                .components()
                .collect(),
        ) {
            return Err(ProfileError::DuplicatePath);
        }
    }
    let mut observations = Vec::with_capacity(specs.len());
    let mut paths = BTreeSet::new();
    for spec in specs {
        let (path, source) = match environment.get(&spec.environment_variable) {
            Some(value) => {
                let path = PathBuf::from(value);
                if !valid_absolute(&path) {
                    return Err(ProfileError::InvalidPath);
                }
                (path, ConfigRootSource::EnvironmentOverride)
            }
            None => (
                home.join(&spec.default_relative_path),
                ConfigRootSource::ProfileDefault,
            ),
        };
        let path_text = text_path(&path)?;
        if !paths.insert(path_text.clone()) {
            return Err(ProfileError::DuplicatePath);
        }
        observations.push(ProfileObservation {
            schema_version: PROFILE_VERSION,
            profile_id: spec.profile_id.clone(),
            instance_name: spec.instance_name.clone(),
            config_identity: spec.config_identity.clone(),
            account_ref: spec.account_ref.clone(),
            path: path_text,
            source,
            observed_at,
            expires_at,
            provenance: provenance.clone(),
        });
    }
    for observation in &observations {
        observation.validate()?;
    }
    Ok(observations)
}
