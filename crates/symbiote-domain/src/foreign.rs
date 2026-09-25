//! Foreign runtime session and task references: a durable, bounded record of
//! the identifiers a *foreign* runtime uses for its own work, held against a
//! canonical Task so a reader can follow the work without the foreign system
//! becoming Project truth.
//!
//! The whole point of this module is what it cannot do. A link is an
//! observation: a harness's own session id, the harness's own todo id inside
//! that session, and the status the harness itself reports for them. Nothing
//! here creates a Task, moves a Task's state, satisfies a completion gate or
//! joins a dependency graph — there is no field a caller could set to make it
//! do so, and the `ForeignStatus::Complete` a harness reports is not the
//! canonical `TaskState::Completed` that only verified evidence produces. The
//! Task aggregate does not carry a link field either: a link is a separate
//! first-class record, exactly as a dependency edge is, because a Task's pure
//! `apply` cannot validate state owned by another system.
use crate::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// The links one canonical Task may carry. A Task is worked by a few runtimes
/// in a few sessions, not by thousands.
pub const MAX_FOREIGN_LINKS: usize = 64;
/// The bytes one foreign identifier may occupy. A harness's own identifier is
/// recorded verbatim — a locator is not reinterpreted as one of our IDs — and
/// this is the bound that keeps that verbatim recording bounded.
pub const MAX_FOREIGN_ID_BYTES: usize = 256;

/// What a reference names in the foreign system: one of that system's own
/// sessions, or one of its own tasks/todos within a session.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum ForeignItemKind {
    Session,
    Task,
}

/// What the foreign system says about its own item, in the foreign system's
/// own vocabulary. `Complete` is the harness saying its todo is done; it is
/// not `TaskState::Completed`, it is not evidence, and no gate reads it.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum ForeignStatus {
    Pending,
    Active,
    Complete,
    Failed,
    Unknown,
}

/// One foreign system's own identifier for one of its own items, held against
/// the canonical Task it was observed for.
///
/// The identifier is recorded as the foreign system gave it, not parsed into
/// one of this crate's identities: it is evidence that the foreign item exists,
/// not a canonical ID. A record that does name this Task is still not a Task,
/// and a `foreign_status` of `complete` is still not completion.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ForeignTaskLink {
    /// Which foreign system these identifiers belong to. A runtime session or
    /// task reference is a harness's own, so only `Harness` is admissible: a
    /// GitHub issue or a CI run is an external reference for another purpose
    /// and naming one here would claim a foreign runtime that does not exist.
    pub system: ExternalSystem,
    /// Whether this names the foreign system's session or its task.
    pub kind: ForeignItemKind,
    /// The foreign system's own identifier, verbatim.
    pub foreign_id: String,
    /// The foreign session the item belongs to, when the foreign system
    /// reports one. A session link does not name another session.
    pub foreign_session_id: Option<String>,
    /// The foreign system's own claim about the item, at `observed_at`.
    pub foreign_status: ForeignStatus,
    /// When the authority recorded this observation — the Host's clock, not
    /// the foreign system's and never a caller-set field on the wire.
    pub observed_at: Timestamp,
}

/// The full link set of one owning Task. Replacement semantics, like a
/// dependency edge set: a later `set` supersedes the previous set while the
/// journal keeps the history.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ForeignTaskLinks {
    pub project_id: ProjectId,
    pub task_id: TaskId,
    pub links: BTreeSet<ForeignTaskLink>,
}

/// One recorded identifier, checked against its bound and its shape. The two
/// refusals stay distinct: a locator too long to hold is a capacity refusal,
/// and a blank or control-character-bearing one is a shape refusal, because
/// the two say different things about what the foreign system reported.
fn check_id(value: &str) -> Result<(), DomainError> {
    if value.len() > MAX_FOREIGN_ID_BYTES {
        return Err(DomainError::ResourceLimit);
    }
    if value.trim().is_empty() || value.chars().any(char::is_control) {
        return Err(DomainError::InvalidForeignReference);
    }
    Ok(())
}

impl ForeignTaskLink {
    /// The bounds and refusals this link is checked against. Every one of them
    /// is a named error rather than a silent repair: a link that cannot be
    /// held is refused, never truncated, trimmed or stored as an observation
    /// that says something the foreign system did not say.
    pub fn validate(&self) -> Result<(), DomainError> {
        if self.system != ExternalSystem::Harness {
            return Err(DomainError::InvalidForeignReference);
        }
        check_id(&self.foreign_id)?;
        match &self.foreign_session_id {
            // A session is not inside another session: a link that claims one
            // is a reference whose own shape cannot be read, so it is refused
            // rather than stored as a session that contains itself.
            Some(_) if self.kind == ForeignItemKind::Session => {
                return Err(DomainError::InvalidForeignReference);
            }
            Some(session) => check_id(session)?,
            None => {}
        }
        Ok(())
    }
}

impl ForeignTaskLink {
    /// What names this item in the foreign system: the system, what kind of
    /// item it is, its own identifier, and the foreign session it sits in. A
    /// set carries one observation of an item, and a later observation of the
    /// same item supersedes the set rather than joining it.
    fn identity(&self) -> (ExternalSystem, ForeignItemKind, &str, &str) {
        (
            self.system,
            self.kind,
            self.foreign_id.as_str(),
            self.foreign_session_id.as_deref().unwrap_or(""),
        )
    }
}

impl ForeignTaskLinks {
    pub fn validate(&self) -> Result<(), DomainError> {
        if self.links.len() > MAX_FOREIGN_LINKS {
            return Err(DomainError::ResourceLimit);
        }
        for link in &self.links {
            link.validate()?;
        }
        // One observation per foreign item. Two entries naming the same item
        // with different statuses is a set that cannot be read — a reader
        // would have to guess which observation the caller meant — so it is
        // refused by name instead of resolved here.
        let mut seen = BTreeSet::new();
        for link in &self.links {
            if !seen.insert(link.identity()) {
                return Err(DomainError::ForeignObservationConflict);
            }
        }
        Ok(())
    }
}
