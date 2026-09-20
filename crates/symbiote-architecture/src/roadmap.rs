//! The repository's issue-generation artifact, read as data (#173).
//!
//! `planning/integrity/generated/registry.json` is what the roadmap importer
//! last wrote for the live issue program: one entry per issue the program
//! carries, with the canonical key, kind, state and the issue a reference
//! points at. A decision's `issue_refs` and `requirement_refs` are links into
//! that program, so checking them against this file is what makes them
//! queryable in the direction the issue-generation side reads: a link that
//! names an issue the program does not carry — retired, renumbered, or invented
//! — is refused rather than left to be discovered by a reader.
//!
//! What this is not: the live tracker. The file is a snapshot, and its own
//! header says ordering establishes nothing; the state it records is the state
//! the importer last captured, so an issue closed after the snapshot still
//! reads `open` here. Nothing offline can do better, and this layer does not
//! pretend to.

use crate::ledger::Ledger;
use crate::{ContractError, Result, require};
use serde::Deserialize;
use std::path::Path;

/// The registry the repository commits, relative to the workspace root.
pub const REGISTRY_PATH: &str = "planning/integrity/generated/registry.json";

/// The only registry schema this reader understands. A future version is
/// refused rather than guessed at.
pub const REGISTRY_SCHEMA_VERSION: u32 = 1;

/// One entry of the generated issue program. The registry carries more per
/// entry than a decision's links need — body hashes, labels, timestamps — and
/// this reads the fields a link resolves through, because the file belongs to
/// the roadmap side and adding a field there must not break this reader.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Entry {
    pub number: u64,
    pub key: String,
    pub kind: String,
    pub state: String,
    /// The canonical issue a reference entry points at, where it has one.
    #[serde(default)]
    pub canonical_issue: Option<u64>,
}

/// The generated issue program.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Roadmap {
    pub schema_version: u32,
    pub entries: Vec<Entry>,
}

impl Roadmap {
    /// Read the generated registry. A registry that cannot be read is an error
    /// rather than an empty program, because a link checked against no program
    /// is not a checked link.
    pub fn read(path: &Path) -> Result<Roadmap> {
        let source = std::fs::read_to_string(path).map_err(|error| {
            ContractError(format!(
                "issue-generation registry {}: {error}",
                path.display()
            ))
        })?;
        let roadmap: Roadmap = serde_json::from_str(&source).map_err(|error| {
            ContractError(format!(
                "issue-generation registry {}: {error}",
                path.display()
            ))
        })?;
        require(
            roadmap.schema_version == REGISTRY_SCHEMA_VERSION,
            "unsupported issue-generation registry schema: this reader understands version 1 and refuses to guess",
        )?;
        require(
            !roadmap.entries.is_empty(),
            "the issue-generation registry carries no entry, so it resolves no link",
        )?;
        Ok(roadmap)
    }

    /// The entry the program carries for an issue number.
    pub fn issue(&self, number: u64) -> Option<&Entry> {
        self.entries.iter().find(|entry| entry.number == number)
    }

    /// The entry the program carries for a canonical key.
    pub fn requirement(&self, key: &str) -> Option<&Entry> {
        self.entries.iter().find(|entry| entry.key == key)
    }

    /// The state the snapshot records for an issue, and whether it is closed.
    /// `None` when the program carries no such issue.
    pub fn closed(&self, number: u64) -> Option<bool> {
        self.issue(number)
            .map(|entry| entry.state.eq_ignore_ascii_case("closed"))
    }

    /// Whether the program carries this issue as canonical work rather than as a
    /// reference to somewhere else.
    pub fn canonical(&self, entry: &Entry) -> bool {
        !entry.kind.eq_ignore_ascii_case("reference")
    }
}

/// What one issue of the program is linked to: the decisions that name it, and
/// how many workspace artifacts pin those decisions. This is the direction the
/// issue-generation side asks its questions in — by issue, not by crate — so a
/// consumer reads the ledger's links without parsing the ledger.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Links {
    pub decisions: Vec<String>,
    pub artifacts: usize,
}

impl Links {
    pub fn is_empty(&self) -> bool {
        self.decisions.is_empty() && self.artifacts == 0
    }
}

/// The index for one issue of the program.
pub fn links(ledger: &Ledger, issue: u64) -> Links {
    let mut decisions = Vec::new();
    for record in &ledger.decisions {
        if record.draft.issue_refs.contains(&issue) {
            decisions.push(record.draft.id.clone());
        }
    }
    let artifacts = ledger
        .artifacts
        .iter()
        .filter(|artifact| {
            artifact.pins.decisions.iter().any(|pin| {
                ledger
                    .decision(&pin.id)
                    .is_some_and(|record| record.draft.issue_refs.contains(&issue))
            })
        })
        .count();
    Links {
        decisions,
        artifacts,
    }
}
