//! The `symbiote` CLI's authorization engine (#54): what it means for an
//! invocation to be *allowed* to send a dangerous operation, and the only
//! place a policy file is read or enforced.
//!
//! The policy document's shape is published by [`crate::cli_schema`]
//! (`POLICY_SCHEMA`, and the `authorize` enum that mirrors the dangerous
//! kinds). Keeping the schema and the code that honors it in one crate means a
//! change to either is made against the other, rather than in a binary the
//! published contract cannot see.
//!
//! Two properties this module exists to guarantee, both inherited from the
//! CLI's own posture:
//!
//! - **A policy is a pre-authorization, never a widening.** It is only ever
//!   consulted for an operation that is *already* dangerous, so it cannot make
//!   a read or a mutation allowed — those never needed authorization at all.
//! - **A policy that cannot be honored is refused, never ignored.** A missing,
//!   malformed, insecure or unclassifiable policy is an operator error that
//!   stops the invocation; silently treating it as "no policy" would let a
//!   typo downgrade a pre-authorization the operator believes is in force.
//!
//! The decision itself ([`authorize`]) is a pure function of its inputs, so
//! every branch is unit tested with no daemon, no terminal and no clock.
use std::collections::BTreeSet;
use std::io::Read;
use std::path::Path;

use crate::cli_schema::{POLICY_SCHEMA, Risk, known_risk_of_kind};

/// The environment variable consulted when `--policy` is absent. An empty
/// value means "no policy", like an unset one.
pub const POLICY_ENV: &str = "SYMBIOTE_CLI_POLICY";

/// A policy grants authority, so it is held to the same bound the Host holds
/// its own operator config to rather than being read whole.
const POLICY_LIMIT: u64 = 64 * 1024;

/// How a request came to be authorized. Kept as a value so the decision is
/// pure and every branch is unit tested without a daemon or a terminal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Authorization {
    /// The operation is not dangerous: nothing needed authorizing.
    NotRequired,
    /// `--yes` authorized this invocation.
    Flag,
    /// A configured policy pre-authorizes this operation kind.
    Policy,
    /// Ask on the terminal; only an explicit `yes` proceeds.
    Prompt,
    /// Refuse before connecting.
    Refused,
}

/// `--yes` outranks a policy (it is the narrower, per-invocation grant) and
/// both outrank the prompt. A policy is only ever consulted for an operation
/// that is already dangerous, so it can never make a read or a mutation
/// "allowed" — those never needed authorization to begin with.
pub fn authorize(risk: Risk, flag: bool, policy_grant: bool, interactive: bool) -> Authorization {
    if !risk.requires_authorization() {
        return Authorization::NotRequired;
    }
    if flag {
        return Authorization::Flag;
    }
    if policy_grant {
        return Authorization::Policy;
    }
    if interactive {
        Authorization::Prompt
    } else {
        Authorization::Refused
    }
}

/// Only the literal `yes` (trimmed, case-insensitive) authorizes. An empty
/// read (closed stdin) is a refusal, never a default.
pub fn confirmation_accepted(line: &str) -> bool {
    line.trim().eq_ignore_ascii_case("yes")
}

/// The parsed authorization policy: the operation kinds an operator has
/// pre-authorized for noninteractive runs, and the point after which that
/// pre-authorization lapses.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Policy {
    /// The kinds this policy names. Public because the gate distinguishes "the
    /// policy never named this kind" from "the policy named it and it lapsed"
    /// when it explains a refusal.
    pub authorize: BTreeSet<String>,
    /// Unix milliseconds after which nothing is granted, or unlimited.
    pub expires_at: Option<u64>,
}

impl Policy {
    /// A policy grants exactly the dangerous kinds it lists, and only until
    /// it expires. An expired policy is not an error: it simply grants
    /// nothing, so the ordinary refusal path reports it.
    pub fn authorizes(&self, kind: &str, now_ms: u64) -> bool {
        if self.expires_at.is_some_and(|expiry| now_ms > expiry) {
            return false;
        }
        self.authorize.contains(kind)
    }

    /// The instant this policy lapsed, if it has. An expired policy that never
    /// named a kind must not be reported as a lapsed grant *for* it, so the
    /// caller pairs this with [`Policy::authorize`].
    pub fn expired_at(&self, now_ms: u64) -> Option<u64> {
        self.expires_at.filter(|expiry| now_ms > *expiry)
    }
}

/// The policy file as written by an operator. `deny_unknown_fields` matters:
/// a misspelled key (`authorise`, `expires`) would otherwise be ignored and
/// the operator would believe they had configured something they had not.
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct PolicyFile {
    schema: String,
    authorize: Vec<String>,
    #[serde(default)]
    expires_at: Option<u64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PolicyError {
    /// Missing, unreadable, a directory, or larger than the bound.
    Unreadable,
    /// Not a regular file, or readable/writable by group or other.
    Insecure,
    /// Not the policy schema, or an unknown/empty/non-dangerous `authorize`.
    Invalid,
}

impl std::fmt::Display for PolicyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            PolicyError::Unreadable => "cannot read the policy as a regular file",
            PolicyError::Insecure => {
                "the policy must be a regular file, owned by this user, with no group/other \
                 permission bits (chmod 600)"
            }
            PolicyError::Invalid => "the policy is not a valid symbiote.cli-policy/v1 document",
        })
    }
}

impl std::error::Error for PolicyError {}

/// Loads and validates a policy with the discipline the Host applies to its
/// own private files: a symlink, a shared file, a foreign owner, an oversized
/// file or a schema that does not name only *known dangerous* kinds is
/// refused rather than interpreted. A policy that cannot be honored is never
/// silently downgraded to "no policy".
pub fn load_policy(path: &Path) -> Result<Policy, PolicyError> {
    use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
    // `O_NOFOLLOW` and classify from the *opened handle*, the way the Host's
    // own transport opens its lock file: the file that is validated is then
    // the file that is read, so a path swapped for a symlink between a check
    // and a read cannot slip a different document past this gate.
    let file = match std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(nix::fcntl::OFlag::O_NOFOLLOW.bits())
        .open(path)
    {
        Ok(file) => file,
        // `ELOOP` means the final component is a symlink: a discipline
        // failure, not a missing file.
        Err(error) if error.raw_os_error() == Some(nix::libc::ELOOP) => {
            return Err(PolicyError::Insecure);
        }
        Err(_) => return Err(PolicyError::Unreadable),
    };
    let metadata = file.metadata().map_err(|_| PolicyError::Unreadable)?;
    if !metadata.file_type().is_file()
        || metadata.uid() != nix::unistd::geteuid().as_raw()
        || metadata.permissions().mode() & 0o077 != 0
    {
        return Err(PolicyError::Insecure);
    }
    let mut bytes = Vec::new();
    (&file)
        .take(POLICY_LIMIT + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| PolicyError::Unreadable)?;
    if bytes.len() as u64 > POLICY_LIMIT {
        return Err(PolicyError::Unreadable);
    }
    let parsed: PolicyFile = serde_json::from_slice(&bytes).map_err(|_| PolicyError::Invalid)?;
    if parsed.schema != POLICY_SCHEMA {
        return Err(PolicyError::Invalid);
    }
    if parsed.authorize.is_empty() {
        return Err(PolicyError::Invalid);
    }
    let mut authorize = BTreeSet::new();
    for kind in parsed.authorize {
        // Only a kind that is *known and dangerous* can be named. A read or a
        // mutation is already ungated, so listing it would be a no-op the
        // operator could mistake for coverage; an unknown kind is one the
        // daemon may not even define, and this CLI refuses to pre-authorize
        // what it cannot classify. A wildcard is deliberately not a syntax.
        if known_risk_of_kind(&kind) != Some(Risk::Dangerous) {
            return Err(PolicyError::Invalid);
        }
        authorize.insert(kind);
    }
    Ok(Policy {
        authorize,
        expires_at: parsed.expires_at,
    })
}
