//! This repository's own architecture decisions as data (#173), replayed
//! through [`crate::policy::DecisionRegistry`] rather than believed.
//!
//! A record holds the draft it published from, the state and revision it
//! publishes, and — when it is accepted — the acceptance that made it
//! accepted. [`Ledger::registry`] proposes, accepts and pins exactly what the
//! file records and then requires every record's published state and revision
//! to be the ones the registry reaches, so a record cannot claim a state the
//! policy crate's own rules never produce: Worker authority, missing measured
//! evidence on a high-reversal-cost choice, an unknown predecessor and a
//! revision the file invents all fail there.
//!
//! What needs the tree as well — the record's content hash, workspace
//! membership, the published gate status — is joined in [`crate::checks`].

use crate::policy::{
    Acceptance, CompatibilityFact, Decision, DecisionRegistry, DecisionState, GateStatus,
    ImplementationPins,
};
use crate::{ContractError, Result, require};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

/// The ledger this repository commits, relative to the workspace root.
pub const LEDGER_PATH: &str = "docs/architecture/decisions.json";

/// The only ledger schema this loader replays. A future version is refused
/// rather than guessed at, exactly as a future `Decision` version is.
pub const LEDGER_SCHEMA_VERSION: u32 = 1;

/// The committed decision ledger: every architecture decision this repository
/// records, the compatibility facts they pin, and the artifacts that pin them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ledger {
    pub schema_version: u32,
    /// In the order the repository published them, so a record that supersedes
    /// an earlier one may name it.
    pub decisions: Vec<DecisionRecord>,
    /// The oldest revision of each fact first.
    #[serde(default)]
    pub facts: Vec<CompatibilityFact>,
    /// One record per artifact that pins a decision.
    #[serde(default)]
    pub artifacts: Vec<ArtifactRecord>,
}

/// One decision, its normative text, and the state the repository publishes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DecisionRecord {
    /// The state and revision the repository publishes for this decision. It is
    /// a claim the replay checks, not the input that drives it.
    pub published: PublishedState,
    /// The draft the record published from, at the revision it was proposed at.
    pub draft: Decision,
    /// The accepted text this record interprets, relative to the workspace
    /// root, and the SHA-256 of that file as committed.
    pub record: String,
    pub record_sha256: String,
    /// Present exactly when the repository publishes an accepted or superseded
    /// state, because those states exist only through an acceptance.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub acceptance: Option<Acceptance>,
    /// The canonical issue that owns the proof this choice still needs. A
    /// record that is not accepted must name one: an unresolved choice with no
    /// owner is the state this ledger exists to prevent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blocking_issue: Option<u64>,
    /// The spike contract that says what this choice would be settled *by*, by
    /// identity. Naming one is what gives the choice a path from `investigating`
    /// to `accepted` that this repository can check: the contract and the record
    /// must name each other, and the run it points at must stand before the
    /// choice can be published accepted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proof_contract: Option<String>,
    /// The heading of the section of the accepted text that states what this
    /// choice's proof must do — the bar its contract answers. The choice names
    /// where its bar lives, not the contract: a contract that named the section
    /// itself could pick the clauses it is settled against out of the record
    /// belonging to the choice. Present exactly when the record names a proof
    /// contract, because a bar nobody answers is not a bar.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proof_section: Option<String>,
    /// The dependency names a dependent implementation would express this
    /// choice with, as cargo reports them. A workspace member that declares one
    /// of these must pin this decision, and a member that pins it must declare
    /// one of them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub proposed_dependencies: Vec<String>,
}

/// The state and revision a record publishes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublishedState {
    pub state: DecisionState,
    pub revision: u64,
}

/// One artifact's pins and the gate status the repository publishes for it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactRecord {
    pub pins: ImplementationPins,
    /// The status this repository publishes for the artifact. It is checked
    /// against [`DecisionRegistry::gate`], so a claim of readiness the registry
    /// does not make fails rather than reads.
    pub status: GateStatus,
    /// The canonical issue that owns what keeps this artifact short of settled.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blocking_issue: Option<u64>,
}

impl Ledger {
    /// Read and shape-check the ledger. A ledger that cannot be read is an
    /// error rather than an empty one, because a check with nothing to read is
    /// not a passing check.
    pub fn read(path: &Path) -> Result<Ledger> {
        let source = std::fs::read_to_string(path).map_err(|error| {
            ContractError(format!("decision ledger {}: {error}", path.display()))
        })?;
        let ledger: Ledger = serde_json::from_str(&source).map_err(|error| {
            ContractError(format!("decision ledger {}: {error}", path.display()))
        })?;
        require(
            ledger.schema_version == LEDGER_SCHEMA_VERSION,
            "unsupported decision ledger schema: this loader replays version 1 and refuses to guess",
        )?;
        require(
            !ledger.decisions.is_empty(),
            "the ledger records no decision, so it accounts for nothing",
        )?;
        Ok(ledger)
    }

    /// Replay the ledger through the policy registry: propose every record,
    /// accept the ones that carry an acceptance, register every pin, and
    /// require the registry to reach exactly the state each record publishes.
    ///
    /// Nothing here is a new rule: the transitions are the crate's own, so a
    /// record that could only exist by breaking them cannot load.
    pub fn registry(&self) -> Result<DecisionRegistry> {
        let mut registry = DecisionRegistry::default();
        for record in &self.decisions {
            let id = record.draft.id.as_str();
            require(
                record.draft.revision == 1,
                format!("{id}: the draft a record holds must be the revision it was proposed at"),
            )?;
            match record.published.state {
                DecisionState::Proposed => require(
                    record.acceptance.is_none(),
                    format!("{id}: a proposed record cannot carry an acceptance"),
                )?,
                DecisionState::Accepted | DecisionState::Superseded => require(
                    record.acceptance.is_some(),
                    format!(
                        "{id}: an accepted record must hold the acceptance that made it accepted"
                    ),
                )?,
                DecisionState::Investigating | DecisionState::Rejected | DecisionState::Retired => {
                    registry.propose(proposal(&record.draft))?;
                    registry.revise_draft(1, record.draft.clone())?;
                    // The revision is this record's own business; the state
                    // check below is what pins it to the replay.
                    continue;
                }
            }
            registry.propose(proposal(&record.draft))?;
            if let Some(acceptance) = &record.acceptance {
                registry.accept(id, 1, acceptance.clone())?;
            }
        }
        let mut revisions: BTreeMap<String, u64> = BTreeMap::new();
        for fact in &self.facts {
            registry.put_fact(revisions.get(&fact.id).copied(), fact.clone())?;
            revisions.insert(fact.id.clone(), fact.revision);
        }
        for artifact in &self.artifacts {
            registry.register_implementation(artifact.pins.clone())?;
        }
        for record in &self.decisions {
            let id = record.draft.id.as_str();
            let reached = registry.decision(id).ok_or_else(|| {
                ContractError(format!("{id}: the registry holds no such decision"))
            })?;
            require(
                reached.state == record.published.state
                    && reached.revision == record.published.revision,
                format!(
                    "{id}: the record publishes {:?} at revision {}, the replay reaches {:?} at revision {}",
                    record.published.state,
                    record.published.revision,
                    reached.state,
                    reached.revision
                ),
            )?;
        }
        Ok(registry)
    }

    /// The record for a decision, so a caller can ask what the repository says
    /// about an identity it holds.
    pub fn decision(&self, id: &str) -> Option<&DecisionRecord> {
        self.decisions.iter().find(|record| record.draft.id == id)
    }
}

/// A proposal as the registry requires one: the draft's content, proposed at
/// revision 1. A record publishes a state; it never publishes the lifecycle.
fn proposal(draft: &Decision) -> Decision {
    let mut proposal = draft.clone();
    proposal.state = DecisionState::Proposed;
    proposal.revision = 1;
    proposal
}
