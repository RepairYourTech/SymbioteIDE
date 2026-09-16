//! Versioned decision/proof contracts (#173). These are in-memory policy
//! mechanisms; the future Host must authenticate authority and evidence inputs.
//!
//! One concern per module. `policy` is the engine: the decision lifecycle,
//! authority, evidence, pins, facts and the registry that owns them, with no
//! file in sight. `ledger` and `spike` are this repository's own data read as
//! documents — its decisions, and the proof contracts that would settle them —
//! replayed through that engine. `repository` is what cargo reports about the
//! workspace, and `checks` is the join: the refusals this repository must
//! survive, one file per subject.
use std::path::{Path, PathBuf};

pub mod checks;
pub mod ledger;
pub mod policy;
pub mod repository;
pub mod spike;

// The vocabulary the crate publishes. Consumers name the engine and the
// contract shape at the crate root; each module below keeps its own path for
// the code that owns it.
pub use policy::*;
pub use spike::{Answered, Elsewhere, Measurement, Part, SpikeContract};

/// The repository this ledger belongs to.
pub fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractError(pub String);
impl std::fmt::Display for ContractError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for ContractError {}
pub(crate) type Result<T> = std::result::Result<T, ContractError>;
pub(crate) fn require(condition: bool, message: impl Into<String>) -> Result<()> {
    if condition {
        Ok(())
    } else {
        Err(ContractError(message.into()))
    }
}
pub(crate) fn text(value: &str) -> bool {
    !value.trim().is_empty()
}
