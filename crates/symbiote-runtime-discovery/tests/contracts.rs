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
