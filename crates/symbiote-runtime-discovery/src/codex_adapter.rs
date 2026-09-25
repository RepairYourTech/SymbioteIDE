//! Bind a completed Codex probe to the installation and profile it actually observed.
//! This is data composition only: it starts nothing, reads no credential, and cannot turn a
//! reported account type into verified authentication.

use crate::{
    AuthMode, AuthState, Authentication, DiscoveryError, DiscoveryFacts, DiscoveryRecord,
    ExecutableInstallation, Fact, Health, InstallationObservation, MethodAvailability,
    ModelListing, ProfileObservation, RuntimeInstanceIntent,
    codex::{CodexAuthentication, CodexProbeReport},
};
use std::{collections::BTreeMap, fmt};

fn bind_authentication(value: CodexAuthentication) -> Fact<Authentication> {
    match value {
        CodexAuthentication::Required => Fact::Known(Authentication {
            mode: AuthMode::None,
            state: AuthState::Required,
            account_ref: None,
        }),
        CodexAuthentication::NotRequired => Fact::Known(Authentication {
            mode: AuthMode::None,
            state: AuthState::NotRequired,
            account_ref: None,
        }),
        // Codex can report that an account object exists without proving that its
        // credential is valid or entitled. Keep the whole authentication fact unknown
        // instead of inventing a verified state or retaining the account payload.
        CodexAuthentication::ApiKeyReported
        | CodexAuthentication::ChatGptReported
        | CodexAuthentication::Unknown => Fact::Unknown,
    }
}

/// Compose the durable discovery record for one observed Codex instance.
///
/// Every identity is cross-checked against the concrete installation and profile. The
/// caller cannot pair a reachable Codex report with another executable, config root, Host,
/// adapter, or named profile and have the record validate.
pub fn bind_report(
    intent: RuntimeInstanceIntent,
    profile: &ProfileObservation,
    installation: &ExecutableInstallation,
    process_id: u32,
    report: &CodexProbeReport,
) -> Result<DiscoveryRecord, CodexBindingError> {
    profile
        .validate()
        .map_err(|_| CodexBindingError::InvalidObservation)?;
    installation
        .validate()
        .map_err(|_| CodexBindingError::InvalidObservation)?;
    let root_matches = installation.config_roots.iter().any(|root| {
        root.identity == profile.config_identity
            && root.path == profile.path
            && root.source == profile.source
    });
    if intent.profile_id != profile.profile_id
        || intent.config_identity != profile.config_identity
        || intent.instance_name != profile.instance_name
        || intent.installation_id != installation.installation_id
        || intent.host_id != installation.host_id
        || intent.runtime_kind != installation.runtime_kind
        || intent.adapter_id != installation.adapter_id
        || !root_matches
    {
        return Err(CodexBindingError::IdentityMismatch);
    }
    if report.process_id() != process_id {
        return Err(CodexBindingError::ProcessMismatch);
    }
    if report.config_root() != profile.path {
        return Err(CodexBindingError::ProfileMismatch);
    }
    if profile.observed_at > installation.observed_at
        || profile.expires_at != installation.expires_at
    {
        return Err(CodexBindingError::Stale);
    }
    let (Fact::Known(version), Fact::Known(interface)) =
        (&installation.version, &installation.interface)
    else {
        return Err(CodexBindingError::UnconfirmedProtocol);
    };
    if interface.name != "app-server"
        || interface.version != report.server_version
        || *version != report.server_version
    {
        return Err(CodexBindingError::InterfaceMismatch);
    }

    let mut upstream_models = std::collections::BTreeSet::new();
    let mut models = Vec::with_capacity(report.models.len());
    for model in &report.models {
        if !upstream_models.insert(model.provider_model.clone()) {
            return Err(CodexBindingError::DuplicateModel);
        }
        models.push(ModelListing {
            upstream_model: model.provider_model.clone(),
            canonical_model_id: None,
            context_tokens: model
                .context_tokens
                .map(Fact::Known)
                .unwrap_or(Fact::Unknown),
        });
    }
    let record = DiscoveryRecord {
        schema_version: crate::DISCOVERY_VERSION,
        observation: InstallationObservation {
            config_identity: profile.config_identity.clone(),
            profile_id: profile.profile_id.clone(),
            installation_id: installation.installation_id.clone(),
            host_id: installation.host_id.clone(),
            runtime_kind: installation.runtime_kind,
            adapter_id: installation.adapter_id.clone(),
            version: Fact::Known(version.clone()),
            interface: Fact::Known(interface.clone()),
            facts: DiscoveryFacts {
                health: Fact::Known(Health::Reachable),
                authentication: bind_authentication(report.authentication),
                models: Fact::Known(models),
                methods: BTreeMap::from([
                    ("account/read".into(), MethodAvailability::Available),
                    ("model/list".into(), MethodAvailability::Available),
                ]),
                isolation: Fact::Unknown,
            },
            observed_at: installation.observed_at,
            expires_at: installation.expires_at,
            provenance: installation.provenance.clone(),
        },
        intent,
    };
    record.validate().map_err(CodexBindingError::Discovery)?;
    Ok(record)
}

/// Why a real adapter report could not be composed into a validated record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CodexBindingError {
    ProcessMismatch,
    ProfileMismatch,
    IdentityMismatch,
    InterfaceMismatch,
    DuplicateModel,
    UnconfirmedProtocol,
    InvalidObservation,
    Stale,
    Discovery(DiscoveryError),
}

impl fmt::Display for CodexBindingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Codex discovery binding refused: {self:?}")
    }
}

impl std::error::Error for CodexBindingError {}
