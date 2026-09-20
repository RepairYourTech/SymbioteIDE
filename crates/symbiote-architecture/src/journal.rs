//! The recorded history of this repository's architecture decisions (#173).
//!
//! A ledger record publishes a state and the draft it published from; it does
//! not say how the state was reached. This is the other half: the transitions
//! each decision went through, in order, each entry naming by SHA-256 the
//! content that transition published and chaining the entry recorded before it.
//! [`Journal::replay`] is the recovery path — a journal that does not rebuild
//! what the ledger publishes is refused rather than believed — and an entry
//! edited after it was recorded no longer hashes to the digest it carries.
//!
//! The chain is authored data, not a derivation. There is deliberately no way
//! to rebuild it from the ledger, because a history a checker regenerates from
//! the thing it checks pins nothing; [`Journal::append`] is the only writer, it
//! adds one entry at the end, and it hashes the content the ledger holds for
//! that transition, so a change to a decision is recorded rather than inserted.
//!
//! What this is not: durable transactional storage. A committed file is not a
//! database, no interrupted write is simulated here, and the Host that would
//! persist and recover this journal under crashes and concurrent writers is a
//! runtime integration its canonical owner holds.

use crate::ledger::{Ledger, proposal};
use crate::policy::DecisionRegistry;
use crate::{ContractError, Result, require};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// The journal this repository commits, relative to the workspace root.
pub const JOURNAL_PATH: &str = "docs/architecture/journal.json";

/// The only journal schema this loader replays. A future version is refused
/// rather than guessed at, exactly as a future ledger version is.
pub const JOURNAL_SCHEMA_VERSION: u32 = 1;

/// The `prev` of the first entry: no transition came before it.
pub const GENESIS: &str = "0000000000000000000000000000000000000000000000000000000000000000";

/// The transitions a decision's history is made of. Nothing else transitions a
/// decision: a fact and an artifact pin are recorded where they are, in the
/// ledger, and their revisions are the ledger's own business.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Transition {
    Proposed,
    Revised,
    Accepted,
}

/// The name this repository publishes for a transition, owned where the
/// transition is, for the same reason a state's name is.
impl std::fmt::Display for Transition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Transition::Proposed => "proposed",
            Transition::Revised => "revised",
            Transition::Accepted => "accepted",
        })
    }
}

/// One recorded transition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    /// The position in the chain, from 1, so a gap is visible.
    pub seq: u64,
    /// The decision this transition is about.
    pub subject: String,
    pub transition: Transition,
    pub from: u64,
    pub to: u64,
    /// Present exactly where this repository records a time: the acceptance's
    /// own `now`, which the content hash pins. Nothing else here has one, and a
    /// stamped entry cannot be told from an invented one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at: Option<u64>,
    /// The predecessor an acceptance supersedes, where it has one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub supersedes: Option<String>,
    /// The SHA-256 of the content this transition published: the proposal, the
    /// revised draft, or the acceptance, as the ledger holds it.
    pub content_sha256: String,
    /// The digest of the entry before this one, so an inserted or removed entry
    /// breaks every entry after it.
    pub prev: String,
    /// This entry's own digest, empty only while it is being computed.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub sha256: String,
}

impl Entry {
    /// The digest of this entry as recorded: the SHA-256 of its canonical JSON
    /// with `sha256` empty. One owner, so a writer and a reader cannot disagree
    /// about what was recorded.
    pub fn digest(&self) -> String {
        let mut body = self.clone();
        body.sha256 = String::new();
        sha256_json(&body)
    }
}

/// The committed history: every recorded transition, in the order it happened.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Journal {
    pub schema_version: u32,
    pub entries: Vec<Entry>,
}

impl Journal {
    /// Read and shape-check the journal, then verify the chain: a journal that
    /// cannot be read is an error rather than an empty history, because a
    /// recovery path with nothing to recover is not a recovery path.
    pub fn read(path: &Path) -> Result<Journal> {
        let source = std::fs::read_to_string(path).map_err(|error| {
            ContractError(format!("decision journal {}: {error}", path.display()))
        })?;
        let journal: Journal = serde_json::from_str(&source).map_err(|error| {
            ContractError(format!("decision journal {}: {error}", path.display()))
        })?;
        require(
            journal.schema_version == JOURNAL_SCHEMA_VERSION,
            "unsupported decision journal schema: this loader replays version 1 and refuses to guess",
        )?;
        require(
            !journal.entries.is_empty(),
            "the journal records no transition, so it accounts for no history",
        )?;
        journal.verify_chain()?;
        Ok(journal)
    }

    /// Write the journal back as the repository commits it, so the only writer
    /// and the committed bytes are the same shape.
    pub fn write(&self, path: &Path) -> Result<()> {
        let bytes = serde_json::to_vec_pretty(self)
            .map_err(|error| ContractError(format!("the journal does not serialize: {error}")))?;
        std::fs::write(path, [bytes, b"\n".to_vec()].concat())
            .map_err(|error| ContractError(format!("decision journal {}: {error}", path.display())))
    }

    /// Sequences ascend from 1 with no gap, every entry continues the chain, and
    /// every entry still hashes to the digest it carries.
    fn verify_chain(&self) -> Result<()> {
        let mut prev = GENESIS.to_string();
        for (index, entry) in self.entries.iter().enumerate() {
            let seq = index as u64 + 1;
            require(
                entry.seq == seq,
                format!(
                    "entry {seq} is recorded at position {}: the journal's transitions are out of order",
                    entry.seq
                ),
            )?;
            require(
                entry.prev == prev,
                format!(
                    "entry {seq} ({}) does not continue the chain: it names {prev} as the entry before it",
                    entry.subject
                ),
            )?;
            require(
                entry.sha256 == entry.digest(),
                format!(
                    "entry {seq} ({}, {}) was changed after it was recorded: it no longer hashes to the digest it carries",
                    entry.subject, entry.transition
                ),
            )?;
            prev = entry.sha256.clone();
        }
        Ok(())
    }

    /// Replay the recorded transitions through the policy registry — the
    /// recovery path. Each entry is checked against the content the ledger
    /// holds for that transition, so the journal recovers the published state
    /// only from the content it actually recorded.
    pub fn replay(&self, ledger: &Ledger) -> Result<DecisionRegistry> {
        let mut registry = DecisionRegistry::default();
        for entry in &self.entries {
            let subject = entry.subject.as_str();
            let record = ledger.decision(subject).ok_or_else(|| {
                ContractError(format!(
                    "entry {} records a transition for {subject}, which the ledger does not hold",
                    entry.seq
                ))
            })?;
            match entry.transition {
                Transition::Proposed => {
                    require(
                        entry.from == 0 && entry.to == 1 && record.draft.revision == 1,
                        format!(
                            "entry {}: a decision is proposed once, from nothing to revision 1",
                            entry.seq
                        ),
                    )?;
                    let content = proposal(&record.draft);
                    require(
                        entry.content_sha256 == sha256_json(&content),
                        format!(
                            "entry {} records proposing {subject} and the ledger holds different content for it, so the history is not of this decision",
                            entry.seq
                        ),
                    )?;
                    registry.propose(content)?;
                }
                Transition::Revised => {
                    require(
                        entry.to == entry.from + 1,
                        format!(
                            "entry {}: a draft revision advances exactly once",
                            entry.seq
                        ),
                    )?;
                    require(
                        entry.content_sha256 == sha256_json(&record.draft),
                        format!(
                            "entry {} records revising {subject} to the revision the ledger holds, and the content differs",
                            entry.seq
                        ),
                    )?;
                    registry.revise_draft(entry.from, record.draft.clone())?;
                }
                Transition::Accepted => {
                    let acceptance = record.acceptance.as_ref().ok_or_else(|| {
                        ContractError(format!(
                            "entry {} records an acceptance of {subject} and the ledger holds none",
                            entry.seq
                        ))
                    })?;
                    require(
                        entry.to == entry.from + 1,
                        format!(
                            "entry {}: an acceptance advances exactly one revision",
                            entry.seq
                        ),
                    )?;
                    require(
                        entry.content_sha256 == sha256_json(acceptance),
                        format!(
                            "entry {} records accepting {subject} on an approval the ledger does not hold",
                            entry.seq
                        ),
                    )?;
                    require(
                        entry.at == Some(acceptance.now),
                        format!(
                            "entry {} records the acceptance at {:?} and the ledger records {}",
                            entry.seq, entry.at, acceptance.now
                        ),
                    )?;
                    require(
                        entry.supersedes == record.draft.supersedes,
                        format!(
                            "entry {} records superseding {:?} where the ledger records {:?}",
                            entry.seq, entry.supersedes, record.draft.supersedes
                        ),
                    )?;
                    registry.accept(subject, entry.from, acceptance.clone())?;
                }
            }
            let reached = registry
                .decision(subject)
                .expect("a decision the replay just transitioned");
            require(
                reached.revision == entry.to,
                format!(
                    "entry {} records {subject} reaching revision {} and the replay reaches {}",
                    entry.seq, entry.to, reached.revision
                ),
            )?;
        }
        for record in &ledger.decisions {
            let id = record.draft.id.as_str();
            let reached = registry.decision(id).ok_or_else(|| {
                ContractError(format!("the recovered registry holds no decision {id}"))
            })?;
            require(
                reached.state == record.published.state
                    && reached.revision == record.published.revision,
                format!(
                    "the journal recovers {id} as {} at revision {}, and the ledger publishes {} at revision {}",
                    reached.state,
                    reached.revision,
                    record.published.state,
                    record.published.revision
                ),
            )?;
        }
        Ok(registry)
    }

    /// What the repository recorded about one decision, in order: the query a
    /// reader asks of the history.
    pub fn entries_for(&self, subject: &str) -> Vec<&Entry> {
        self.entries
            .iter()
            .filter(|entry| entry.subject == subject)
            .collect()
    }

    /// Whether the history reaches this revision through a named transition.
    pub fn records(&self, subject: &str, transition: Transition) -> bool {
        self.entries_for(subject)
            .iter()
            .any(|entry| entry.transition == transition)
    }

    /// Append one transition to the end of the chain, naming the content the
    /// ledger holds for it. The only writer, and it never rewrites an entry.
    pub fn append(
        &mut self,
        ledger: &Ledger,
        transition: Transition,
        subject: &str,
        from: u64,
    ) -> Result<Entry> {
        let record = ledger.decision(subject).ok_or_else(|| {
            ContractError(format!(
                "the ledger holds no decision {subject}, so its history cannot record a transition for it"
            ))
        })?;
        let (to, at, supersedes, content_sha256) = match transition {
            Transition::Proposed => (1, None, None, sha256_json(&proposal(&record.draft))),
            Transition::Revised => (from + 1, None, None, sha256_json(&record.draft)),
            Transition::Accepted => {
                let acceptance = record.acceptance.as_ref().ok_or_else(|| {
                    ContractError(format!(
                        "the ledger records no acceptance of {subject} to record"
                    ))
                })?;
                (
                    from + 1,
                    Some(acceptance.now),
                    record.draft.supersedes.clone(),
                    sha256_json(acceptance),
                )
            }
        };
        let mut entry = Entry {
            seq: self.entries.len() as u64 + 1,
            subject: subject.to_string(),
            transition,
            from,
            to,
            at,
            supersedes,
            content_sha256,
            prev: self
                .entries
                .last()
                .map_or_else(|| GENESIS.to_string(), |entry| entry.sha256.clone()),
            sha256: String::new(),
        };
        entry.sha256 = entry.digest();
        self.entries.push(entry.clone());
        Ok(entry)
    }
}

/// The SHA-256 of a value's canonical JSON: how a recorded transition names the
/// content it published, and the same reading [`Entry::digest`] hashes.
pub(crate) fn sha256_json<T: Serialize>(value: &T) -> String {
    crate::repository::digest(&serde_json::to_vec(value).expect("a recorded value serializes"))
}
