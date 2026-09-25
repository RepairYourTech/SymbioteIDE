use serde_json::json;
use symbiote_runtime_discovery::{
    DiagnosticAction, DiagnosticCode, DiagnosticInput, DiscoveryError, InstallationError,
    ProfileError, RuntimeDiagnostic, codex::CodexDiscoveryError, diagnose,
};

fn classified(input: DiagnosticInput) -> (DiagnosticCode, DiagnosticAction) {
    let diagnostic = diagnose(&input).unwrap();
    assert_eq!(diagnostic.schema_version(), 1);
    assert_eq!(
        serde_json::from_str::<RuntimeDiagnostic>(&serde_json::to_string(&diagnostic).unwrap())
            .unwrap(),
        diagnostic
    );
    (diagnostic.code(), diagnostic.action())
}

#[test]
fn the_five_required_failures_have_deterministic_non_authorizing_actions() {
    assert_eq!(
        classified(DiagnosticInput::Installation(
            InstallationError::MissingExecutable
        )),
        (
            DiagnosticCode::MissingBinary,
            DiagnosticAction::InstallBinaryWithConsent
        )
    );
    for input in [
        DiagnosticInput::Codex(CodexDiscoveryError::UnsupportedVersion),
        DiagnosticInput::Qualification(DiscoveryError::VersionMismatch),
    ] {
        assert_eq!(
            classified(input),
            (
                DiagnosticCode::UnsupportedVersion,
                DiagnosticAction::UpgradeWithConsent
            )
        );
    }
    assert_eq!(
        classified(DiagnosticInput::Qualification(
            DiscoveryError::AuthenticationExpired
        )),
        (
            DiagnosticCode::ExpiredAuthentication,
            DiagnosticAction::Reauthenticate
        )
    );
    assert_eq!(
        classified(DiagnosticInput::Qualification(DiscoveryError::RateLimited)),
        (DiagnosticCode::RateLimited, DiagnosticAction::RetryLater)
    );
    for error in [
        ProfileError::InvalidSpec,
        ProfileError::DuplicateProfile,
        ProfileError::DuplicateConfig,
        ProfileError::DuplicateEnvironment,
        ProfileError::DuplicatePath,
        ProfileError::InvalidPath,
    ] {
        assert_eq!(
            classified(DiagnosticInput::Profile(error)),
            (
                DiagnosticCode::IncompatibleConfiguration,
                DiagnosticAction::CorrectConfiguration
            )
        );
    }
}

#[test]
fn unrelated_refusals_remain_typed_errors_instead_of_becoming_diagnostic_guesses() {
    for input in [
        DiagnosticInput::Installation(InstallationError::OutsideTrustedRoot),
        DiagnosticInput::Installation(InstallationError::MutableExecutable),
        DiagnosticInput::Installation(InstallationError::Stale),
        DiagnosticInput::Profile(ProfileError::Stale),
        DiagnosticInput::Profile(ProfileError::ResourceLimit),
        DiagnosticInput::Qualification(DiscoveryError::IdentityMismatch),
        DiagnosticInput::Qualification(DiscoveryError::UnknownMandatoryFact),
        DiagnosticInput::Qualification(DiscoveryError::Unhealthy),
        DiagnosticInput::Codex(CodexDiscoveryError::Transport),
        DiagnosticInput::Codex(CodexDiscoveryError::MalformedFrame),
    ] {
        assert_eq!(diagnose(&input), None);
    }
}

#[test]
fn diagnostic_wire_rejects_version_drift_unknown_fields_and_inconsistent_actions() {
    let valid = json!({
        "schema_version": 1,
        "code": "missing_binary",
        "action": "install_binary_with_consent"
    });
    assert!(serde_json::from_value::<RuntimeDiagnostic>(valid.clone()).is_ok());
    for invalid in [
        json!({
            "schema_version": 2,
            "code": "missing_binary",
            "action": "install_binary_with_consent"
        }),
        json!({
            "schema_version": 1,
            "code": "missing_binary",
            "action": "reauthenticate"
        }),
        json!({
            "schema_version": 1,
            "code": "missing_binary",
            "action": "install_binary_with_consent",
            "path": "/private/account/path"
        }),
    ] {
        assert!(serde_json::from_value::<RuntimeDiagnostic>(invalid).is_err());
    }
}

#[test]
fn published_schema_enforces_the_same_version_and_code_action_pairs_as_deserialization() {
    let schema = serde_json::to_value(schemars::schema_for!(RuntimeDiagnostic)).unwrap();
    assert_eq!(schema["properties"]["schema_version"]["const"], json!(1));
    let variants = schema["oneOf"].as_array().unwrap();
    assert_eq!(variants.len(), 5);
    for diagnostic in [
        json!({"schema_version": 1, "code": "missing_binary", "action": "install_binary_with_consent"}),
        json!({"schema_version": 1, "code": "unsupported_version", "action": "upgrade_with_consent"}),
        json!({"schema_version": 1, "code": "expired_authentication", "action": "reauthenticate"}),
        json!({"schema_version": 1, "code": "rate_limited", "action": "retry_later"}),
        json!({"schema_version": 1, "code": "incompatible_configuration", "action": "correct_configuration"}),
    ] {
        assert!(variants.iter().any(|variant| {
            diagnostic["code"] == variant["properties"]["code"]["const"]
                && diagnostic["action"] == variant["properties"]["action"]["const"]
        }));
        assert!(serde_json::from_value::<RuntimeDiagnostic>(diagnostic).is_ok());
    }
    for invalid in [
        json!({"schema_version": 2, "code": "missing_binary", "action": "install_binary_with_consent"}),
        json!({"schema_version": 1, "code": "missing_binary", "action": "reauthenticate"}),
    ] {
        assert!(serde_json::from_value::<RuntimeDiagnostic>(invalid.clone()).is_err());
        let schema_accepts = invalid["schema_version"]
            == schema["properties"]["schema_version"]["const"]
            && variants.iter().any(|variant| {
                invalid["code"] == variant["properties"]["code"]["const"]
                    && invalid["action"] == variant["properties"]["action"]["const"]
            });
        assert!(!schema_accepts);
    }
}
