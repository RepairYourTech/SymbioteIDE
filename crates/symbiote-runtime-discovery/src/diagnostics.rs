use crate::{
    DiscoveryError, codex::CodexDiscoveryError, installation::InstallationError,
    profiles::ProfileError,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const DIAGNOSTIC_VERSION: u32 = 1;

/// A closed diagnostic classification. It carries no source error, path, account,
/// credential or adapter payload, so displaying it cannot disclose the observation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticCode {
    MissingBinary,
    UnsupportedVersion,
    ExpiredAuthentication,
    RateLimited,
    IncompatibleConfiguration,
}

/// The operator-facing next step. These values recommend UI action; this crate
/// never performs installation, upgrade, authentication or configuration changes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticAction {
    InstallBinaryWithConsent,
    UpgradeWithConsent,
    Reauthenticate,
    RetryLater,
    CorrectConfiguration,
}

impl DiagnosticCode {
    const fn all() -> [Self; 5] {
        [
            Self::MissingBinary,
            Self::UnsupportedVersion,
            Self::ExpiredAuthentication,
            Self::RateLimited,
            Self::IncompatibleConfiguration,
        ]
    }

    /// The one action the closed diagnostic contract permits for this code.
    pub const fn action(self) -> DiagnosticAction {
        match self {
            Self::MissingBinary => DiagnosticAction::InstallBinaryWithConsent,
            Self::UnsupportedVersion => DiagnosticAction::UpgradeWithConsent,
            Self::ExpiredAuthentication => DiagnosticAction::Reauthenticate,
            Self::RateLimited => DiagnosticAction::RetryLater,
            Self::IncompatibleConfiguration => DiagnosticAction::CorrectConfiguration,
        }
    }
}

/// A source refusal eligible for deterministic classification. Every variant is
/// an existing closed error enum; arbitrary strings and transport payloads cannot
/// become diagnostics.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DiagnosticInput {
    Installation(InstallationError),
    Profile(ProfileError),
    Qualification(DiscoveryError),
    Codex(CodexDiscoveryError),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "DiagnosticWire")]
pub struct RuntimeDiagnostic {
    schema_version: u32,
    code: DiagnosticCode,
    action: DiagnosticAction,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DiagnosticWire {
    schema_version: u32,
    code: DiagnosticCode,
    action: DiagnosticAction,
}

impl JsonSchema for RuntimeDiagnostic {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "RuntimeDiagnostic".into()
    }

    /// Publish the same closed version and code/action pairs enforced by the
    /// runtime parser, so a schema-only consumer cannot admit a broader value.
    fn json_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
        let one_of = DiagnosticCode::all()
            .into_iter()
            .map(|code| {
                serde_json::json!({
                    "properties": {
                        "code": {"const": serde_json::to_value(code).expect("diagnostic code serializes")},
                        "action": {"const": serde_json::to_value(code.action()).expect("diagnostic action serializes")}
                    }
                })
            })
            .collect::<Vec<_>>();
        schemars::json_schema!({
            "type": "object",
            "additionalProperties": false,
            "required": ["schema_version", "code", "action"],
            "properties": {
                "schema_version": {"type": "integer", "const": DIAGNOSTIC_VERSION},
                "code": {"type": "string"},
                "action": {"type": "string"}
            },
            "oneOf": one_of
        })
    }
}

impl TryFrom<DiagnosticWire> for RuntimeDiagnostic {
    type Error = DiagnosticError;

    fn try_from(wire: DiagnosticWire) -> Result<Self, Self::Error> {
        let diagnostic = Self {
            schema_version: wire.schema_version,
            code: wire.code,
            action: wire.action,
        };
        diagnostic.validate()?;
        Ok(diagnostic)
    }
}

impl RuntimeDiagnostic {
    /// The diagnostic schema version accepted by this build.
    pub const fn schema_version(&self) -> u32 {
        self.schema_version
    }

    /// The closed failure classification, without observation payloads.
    pub const fn code(&self) -> DiagnosticCode {
        self.code
    }

    /// The operator action recommended for this classification.
    pub const fn action(&self) -> DiagnosticAction {
        self.action
    }

    fn validate(&self) -> Result<(), DiagnosticError> {
        if self.schema_version != DIAGNOSTIC_VERSION {
            return Err(DiagnosticError::UnsupportedVersion);
        }
        if self.action != self.code.action() {
            return Err(DiagnosticError::InconsistentAction);
        }
        Ok(())
    }
}

/// Classify only the canonical failures with an operator action. Other refusals
/// remain their original typed error instead of being broadened into a guess.
pub fn diagnose(input: &DiagnosticInput) -> Option<RuntimeDiagnostic> {
    let code = match input {
        DiagnosticInput::Installation(InstallationError::MissingExecutable) => {
            DiagnosticCode::MissingBinary
        }
        DiagnosticInput::Codex(CodexDiscoveryError::UnsupportedVersion)
        | DiagnosticInput::Qualification(DiscoveryError::VersionMismatch) => {
            DiagnosticCode::UnsupportedVersion
        }
        DiagnosticInput::Qualification(DiscoveryError::AuthenticationExpired) => {
            DiagnosticCode::ExpiredAuthentication
        }
        DiagnosticInput::Qualification(DiscoveryError::RateLimited) => DiagnosticCode::RateLimited,
        DiagnosticInput::Profile(
            ProfileError::InvalidSpec
            | ProfileError::DuplicateProfile
            | ProfileError::DuplicateConfig
            | ProfileError::DuplicateEnvironment
            | ProfileError::DuplicatePath
            | ProfileError::InvalidPath,
        ) => DiagnosticCode::IncompatibleConfiguration,
        _ => return None,
    };
    Some(RuntimeDiagnostic {
        schema_version: DIAGNOSTIC_VERSION,
        code,
        action: code.action(),
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiagnosticError {
    UnsupportedVersion,
    InconsistentAction,
}

impl std::fmt::Display for DiagnosticError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "runtime diagnostic refused: {self:?}")
    }
}

impl std::error::Error for DiagnosticError {}
