//! What `docs/contracts/runtime-discovery.md` states about this crate, held to the crate.
//!
//! The document writes the pinned Codex release three times — the tested release, the client
//! name the probe reports, and the version the installed binary returned — and each is read from
//! the sentence it is written in and held to `CODEX_VERSION`, the one constant this crate
//! enforces. A document that names a release the crate has moved past fails here by name, and so
//! does a constant moved without the document: the figures are compared in both directions.
//!
//! What it does not read: the prose around the figures — the exact npm artifact, the schema
//! generation, the observed authentication state — which the codex and inventory cases drive, and
//! the historical narration in `docs/proofs/` and `docs/agent-takeover.md`, which states what a
//! past run observed rather than what this crate enforces.
use symbiote_contract_read::region;

/// The pinned release the document states is the one `CODEX_VERSION` carries, in every sentence
/// that writes it.
#[test]
fn the_contract_states_the_release_this_crate_pins() {
    let contract = include_str!("../../../docs/contracts/runtime-discovery.md");
    let pinned = symbiote_runtime_discovery::codex::CODEX_VERSION;

    for (label, stated) in [
        (
            "the tested release",
            region(contract, "The tested release is `codex-cli ", "`"),
        ),
        (
            "the client name the probe reports",
            region(contract, "reports `symbiote/", "`"),
        ),
        (
            "the version the installed binary returned",
            region(contract, "returned version `", "`"),
        ),
    ] {
        assert_eq!(
            stated, pinned,
            "the contract's {label} states {stated}, and this crate pins {pinned}"
        );
    }
}

/// The installation slice is held to the names and the deliberately unknown update boundary in
/// the document, so prose cannot make the metadata pass sound like an installer or updater.
#[test]
fn the_installation_boundary_is_named_and_does_not_claim_update_authority() {
    let contract = include_str!("../../../docs/contracts/runtime-discovery.md");
    for name in [
        "ExecutableInstallation::inspect",
        "InstallationChannel",
        "ProtocolEvidence",
        "UpdateAvailability::Unknown",
        "OutsideTrustedRoot",
        "MutableExecutable",
        "confirm_protocol",
    ] {
        assert!(contract.contains(name), "the contract must name {name}");
    }
    let source = include_str!("../src/installation.rs");
    assert!(source.contains("std::fs::symlink_metadata"));
    assert!(source.contains("std::fs::canonicalize"));
    assert!(!source.contains("Command::new"));
}

/// The named-profile boundary is held to the document and to the absence of
/// process-environment mutation, so alternate config roots cannot quietly become
/// global HOME or credential authority.
#[test]
fn the_named_profile_boundary_is_named_and_pure() {
    let contract = include_str!("../../../docs/contracts/runtime-discovery.md");
    for name in [
        "profiles::resolve_profiles",
        "ProfileSpec",
        "ProfileObservation",
        "EnvironmentOverride",
        "ProfileDefault",
        "account_ref",
        "DuplicatePath",
    ] {
        assert!(contract.contains(name), "the contract must name {name}");
    }
    let source = include_str!("../src/profiles.rs");
    assert!(!source.contains("set_var"));
    assert!(!source.contains("remove_dir_all"));
    assert!(source.contains("environment.get"));
}

/// The real-adapter composition boundary is held to the document and to its
/// data-only, privacy-preserving source. A reported account object cannot become
/// verified authentication or smuggle an account payload into durable inventory.
#[test]
fn the_codex_adapter_record_is_named_bounded_and_unverified() {
    let contract = include_str!("../../../docs/contracts/runtime-discovery.md");
    for name in [
        "codex_adapter::bind_report",
        "CodexProbeReport",
        "DiscoveryRecord",
        "ProcessMismatch",
        "ProfileMismatch",
        "UnconfirmedProtocol",
        "DuplicateModel",
    ] {
        assert!(contract.contains(name), "the contract must name {name}");
    }
    let source = include_str!("../src/codex_adapter.rs");
    assert!(source.contains("let root_matches"));
    assert!(source.contains("report.process_id() != process_id"));
    assert!(source.contains("report.config_root() != profile.path"));
    assert!(source.contains("Fact::Unknown"));
    assert!(!source.contains("Command::new"));
    assert!(!source.contains("std::env"));
    assert!(!source.contains("account.email"));
}

/// The deterministic-diagnostic boundary is held to the document, its closed source
/// vocabulary, and its data-only non-authorizing surface.
#[test]
fn the_diagnostic_boundary_is_named_deterministic_and_non_authorizing() {
    let contract = include_str!("../../../docs/contracts/runtime-discovery.md");
    for name in [
        "diagnostics::diagnose",
        "DiagnosticInput",
        "RuntimeDiagnostic",
        "DiagnosticCode",
        "DiagnosticAction",
        "MissingBinary",
        "UnsupportedVersion",
        "ExpiredAuthentication",
        "RateLimited",
        "IncompatibleConfiguration",
        "InstallBinaryWithConsent",
        "UpgradeWithConsent",
        "Reauthenticate",
        "RetryLater",
        "CorrectConfiguration",
    ] {
        assert!(contract.contains(name), "the contract must name {name}");
    }
    let source = include_str!("../src/diagnostics.rs");
    for source_error in [
        "InstallationError::MissingExecutable",
        "CodexDiscoveryError::UnsupportedVersion",
        "DiscoveryError::VersionMismatch",
        "DiscoveryError::AuthenticationExpired",
        "DiscoveryError::RateLimited",
        "ProfileError::InvalidSpec",
    ] {
        assert!(
            source.contains(source_error),
            "the mapping must name {source_error}"
        );
    }
    assert!(source.contains("self.action != self.code.action()"));
    assert!(source.contains("\"const\": DIAGNOSTIC_VERSION"));
    assert!(source.contains("\"oneOf\": one_of"));
    assert!(!source.contains("Command::new"));
    assert!(!source.contains("std::env"));
    assert!(!source.contains("std::fs"));
    assert!(!source.contains("account_ref"));
}

/// The user agent this crate accepts is built from the pinned release rather than restated beside
/// it, so a moved constant moves both the accepted name and the document's figures together.
#[test]
fn the_accepted_client_name_is_built_from_the_pinned_release() {
    let source = include_str!("../src/codex.rs");
    let check = region(
        source,
        "let expected = ",
        "return Err(CodexDiscoveryError::UnsupportedVersion);",
    );
    assert!(
        check.contains("CODEX_VERSION"),
        "the accepted client name must be built from CODEX_VERSION, and this read found {check:?}"
    );
    assert!(
        !source.contains("symbiote/0."),
        "the accepted client name must not be restated as a version figure beside the constant"
    );
}

/// The durable read surface this document publishes is the one the Host
/// implements: the file name, the bound, the whole refusal vocabulary, the local
/// command that installs a document, and the protocol version the read rides on.
/// Each figure is read from the sentence that writes it and compared with the
/// code, so a moved constant or a renamed file reds here by name.
#[test]
fn the_durable_inventory_read_surface_is_named_and_bound() {
    let contract = include_str!("../../../docs/contracts/runtime-discovery.md");
    let section = region(
        contract,
        "## Durable Host inventory read surface",
        "\n## Deterministic diagnostics",
    );
    // The refusal vocabulary is closed: the document names every name the Host
    // can answer with, and answers with no other.
    let host = include_str!("../../symbiote-host/src/runtime_inventory.rs");
    for refusal in [
        "Absent",
        "Unsafe",
        "Unreadable",
        "Unparseable",
        "Oversized",
        "ForeignHost",
    ] {
        assert!(host.contains(refusal), "the Host module declares {refusal}");
    }
    for name in [
        "no_published_inventory",
        "unsafe_inventory_file",
        "unreadable_inventory",
        "unparseable_inventory",
        "oversized_inventory",
        "foreign_host_inventory",
    ] {
        assert!(section.contains(name), "the section must name {name}");
    }
    // The bound, the file name and the command are stated once each and are the
    // ones the code uses.
    assert!(section.contains("`runtime-inventory.json`"));
    assert!(
        include_str!("../../symbiote-host/src/paths.rs")
            .contains(r#"RUNTIME_INVENTORY_FILE: &str = "runtime-inventory.json""#),
        "the Host publishes the file name the document states"
    );
    let stated: usize = region(section, "The published bound is ", " KiB")
        .trim()
        .parse()
        .expect("a KiB figure");
    assert_eq!(
        stated * 1024,
        symbiote_runtime_discovery::MAX_PUBLISHED_BYTES,
        "the document's bound is the one this crate carries and the Host applies"
    );
    assert!(section.contains("`symbiote publish-runtime-inventory DOCUMENT --state-dir DIR`"));
    assert!(section.contains("`get_runtime_inventory` (protocol v1.25)"));
    // The Host-side reader carries the discipline the section describes, and no
    // write path a client could reach.
    for discipline in [
        "O_NOFOLLOW",
        "metadata.nlink() != 1",
        "metadata.uid() != nix::unistd::geteuid().as_raw()",
        "metadata.mode() & 0o777 != 0o600",
    ] {
        assert!(
            host.contains(discipline),
            "the read must apply {discipline}"
        );
    }
    assert!(!host.contains("Command::new"));
    // The install is atomic: a rename over the published name, never a
    // truncating write of the same name, so a reader sees the old document or
    // the new one and never a partial write.
    assert!(
        host.contains("std::fs::rename(&temporary, &path)"),
        "the publisher installs by rename"
    );
    assert!(!host.contains("fs::copy"));
}
