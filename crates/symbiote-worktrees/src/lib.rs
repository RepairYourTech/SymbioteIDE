//! Reserved Change Stream worktree locations for the trusted Host provisioner.
//!
//! This slice covers deterministic collision-resistant naming plus filesystem
//! reservation and re-verification of the location that the Linux sandbox
//! requires its caller to reserve. It deliberately does not run Git, create
//! repository worktrees, or delete work: Git/repository identity belongs to
//! #190 and worktree cleanup with evidence remains pending #211 acceptance.
#![cfg(target_os = "linux")]

use nix::{
    fcntl::{OFlag, openat},
    sys::stat::{Mode, fstat},
    unistd::{geteuid, unlink},
};
use serde::{Deserialize, Serialize};
use std::{
    fmt, fs,
    os::unix::fs::MetadataExt,
    path::{Component, Path, PathBuf},
};

pub const RESERVATION_VERSION: u32 = 1;
pub const MARKER_NAME: &str = ".symbiote-reservation.json";
/// Bound on one path component this crate derives or accepts as a policy seed.
const MAX_SEED_BYTES: usize = 64;
const MAX_BASE_BYTES: usize = 3072;
const BRANCH_PREFIX: &str = "symbiote";
/// Branch suffix carries 96 bits of digest; the worktree id carries 64 bits.
/// Both are truncated hashes, not unique encodings.
const BRANCH_SUFFIX_DIGEST_BYTES: usize = 12;
const WORKTREE_ID_DIGEST_BYTES: usize = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorktreeError {
    InvalidIdentity,
    InvalidSeed,
    InvalidBase,
    UnsafePath,
    NotPrivate,
    AlreadyReserved,
    NotFound,
    IdentityMismatch,
    NotEmpty,
    Io,
}
impl fmt::Display for WorktreeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "worktree reservation rejected: {self:?}")
    }
}
impl std::error::Error for WorktreeError {}

/// Policy seed supplied by trusted Host state. Different seeds derive different
/// names; the same recorded seed and identities always derive the same names.
pub fn policy_seed(value: &str) -> Result<&str, WorktreeError> {
    let valid = !value.is_empty()
        && value.len() <= MAX_SEED_BYTES
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-');
    if valid {
        Ok(value)
    } else {
        Err(WorktreeError::InvalidSeed)
    }
}

/// Inputs are canonical identities already validated by the domain. The same
/// recorded identities and seed always yield the same derived names.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeriveInputs<'a> {
    pub project_id: &'a ProjectId,
    pub root_id: &'a RootId,
    pub stream_id: &'a ChangeStreamId,
    pub seed: &'a str,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Derived {
    pub project_id: ProjectId,
    pub root_id: RootId,
    pub stream_id: ChangeStreamId,
    pub seed: String,
    /// Canonical bounded WorktreeId newtype: a 64-bit digest truncation, never
    /// a filesystem path.
    pub worktree_id: WorktreeId,
    /// Git-ref-safe branch under the reserved `symbiote/` prefix namespace.
    pub branch: String,
}

fn digest_bytes(fingerprint: &symbiote_trust::Fingerprint) -> Vec<u8> {
    let hex = fingerprint.as_str();
    (0..hex.len() / 2)
        .map(|i| u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).unwrap_or(0))
        .collect()
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Git check-ref-format rules relevant to the derived charset, enforced so a
/// valid domain ID can never produce an invalid ref component.
fn ref_component(value: &str) -> bool {
    !value.is_empty()
        && !value.starts_with('.')
        && !value.ends_with(".lock")
        && !value.contains("..")
        && !value.contains("@{")
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.'))
}

impl Derived {
    /// Pure derivation. Ids use the domain charset ([A-Za-z0-9_-]), so every
    /// derived component is ref-safe by construction; the seed mixes policy
    /// into the hash without entering the ref itself.
    pub fn derive(inputs: DeriveInputs<'_>) -> Result<Self, WorktreeError> {
        let seed = policy_seed(inputs.seed)?;
        let material = (
            seed,
            inputs.project_id.as_str(),
            inputs.root_id.as_str(),
            inputs.stream_id.as_str(),
            symbiote_trust::Fingerprint::of(
                format!(
                    "worktree-reservation-v{RESERVATION_VERSION}:{seed}:{}:{}:{}",
                    inputs.project_id.as_str(),
                    inputs.root_id.as_str(),
                    inputs.stream_id.as_str()
                )
                .as_bytes(),
            )
            .as_str()
            .to_owned(),
        );
        let fingerprint = symbiote_trust::Fingerprint::of(
            serde_json::to_vec(&material)
                .map_err(|_| WorktreeError::InvalidIdentity)?
                .as_slice(),
        );
        let digest = digest_bytes(&fingerprint);
        if digest.len() < BRANCH_SUFFIX_DIGEST_BYTES {
            return Err(WorktreeError::Io);
        }
        let suffix = hex(&digest[..BRANCH_SUFFIX_DIGEST_BYTES]);
        let identity = WorktreeId::new(format!("st-{}", hex(&digest[..WORKTREE_ID_DIGEST_BYTES])))
            .map_err(|_| WorktreeError::InvalidIdentity)?;
        let (project, stream) = (inputs.project_id.as_str(), inputs.stream_id.as_str());
        if !ref_component(project) || !ref_component(stream) {
            return Err(WorktreeError::InvalidIdentity);
        }
        let branch = format!("{BRANCH_PREFIX}/{project}/{stream}/{suffix}");
        // 512 covers the worst case (two 128-byte domain identities); the
        // protocol TaskDraft bound matches so derivation never produces a
        // branch the wire would reject.
        if branch.len() > 512 || !branch.split('/').all(ref_component) {
            return Err(WorktreeError::InvalidIdentity);
        }
        Ok(Self {
            project_id: inputs.project_id.clone(),
            root_id: inputs.root_id.clone(),
            stream_id: inputs.stream_id.clone(),
            seed: seed.to_owned(),
            worktree_id: identity,
            branch,
        })
    }

    pub fn worktree_path(&self, base: &Path) -> PathBuf {
        base.join(self.project_id.as_str())
            .join(self.worktree_id.as_str())
    }
}

/// Recorded reservation identity bound to the reserved location. The marker
/// file lives in the private project directory, never inside the worktree, so
/// later Git materialization keeps a clean tree.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Marker {
    pub version: u32,
    pub derived: Derived,
    pub created_at: Timestamp,
}

/// A verified reservation. Not a guard: reservations persist across Host
/// restarts by design; release requires explicit policy.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reservation {
    pub derived: Derived,
    pub base: PathBuf,
}

/// What release may do. Work left by a stream is never deleted recursively:
/// unmerged or uncommitted work removal requires separate evidence and
/// authorized confirmation per the Change Stream contract.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReleasePolicy {
    /// Keep the reserved location for recovery/inspection.
    Retain,
    /// Remove the marker and the directory only when it holds no entries.
    DeleteIfEmpty,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    Deleted,
    Retained,
}

fn validate_absolute(path: &Path, max_bytes: usize) -> Result<(), WorktreeError> {
    if path.as_os_str().len() > max_bytes
        || path.components().count() > 96
        || !path.is_absolute()
        || path
            .components()
            .any(|c| !matches!(c, Component::RootDir | Component::Normal(_)))
    {
        return Err(WorktreeError::UnsafePath);
    }
    Ok(())
}

fn euid() -> u32 {
    geteuid().as_raw()
}

/// Walks each component without following symlinks and returns the final fd,
/// mirroring the sandbox's path-walk discipline.
fn walk_private(path: &Path) -> Result<(), WorktreeError> {
    use nix::sys::stat::SFlag;
    let mut fd = nix::fcntl::open(
        Path::new("/"),
        OFlag::O_RDONLY | OFlag::O_DIRECTORY | OFlag::O_NOFOLLOW | OFlag::O_CLOEXEC,
        Mode::empty(),
    )
    .map_err(|_| WorktreeError::UnsafePath)?;
    for component in path.components() {
        let name = match component {
            Component::Normal(name) => name,
            Component::RootDir => continue,
            _ => return Err(WorktreeError::UnsafePath),
        };
        fd = openat(
            &fd,
            name,
            OFlag::O_RDONLY | OFlag::O_DIRECTORY | OFlag::O_NOFOLLOW | OFlag::O_CLOEXEC,
            Mode::empty(),
        )
        .map_err(|_| WorktreeError::UnsafePath)?;
        let stat = fstat(&fd).map_err(|_| WorktreeError::UnsafePath)?;
        if SFlag::from_bits_truncate(stat.st_mode) & SFlag::S_IFMT != SFlag::S_IFDIR {
            return Err(WorktreeError::UnsafePath);
        }
        // Root-owned and sticky-bit components mirror the sandbox's accepted
        // walk rule (e.g. /tmp); anything group/other-writable without the
        // sticky bit is refused.
        if ![0, euid()].contains(&stat.st_uid)
            || (stat.st_mode & 0o022 != 0 && stat.st_mode & 0o1000 == 0)
        {
            return Err(WorktreeError::NotPrivate);
        }
    }
    Ok(())
}

fn sync_dir(path: &Path) -> Result<(), WorktreeError> {
    fs::File::open(path)
        .and_then(|handle| handle.sync_all())
        .map_err(|_| WorktreeError::Io)
}

/// Creates the directory with mode 0700, or accepts an existing directory that
/// already satisfies the private-premise checks. Racing creators fall through
/// to the checks instead of failing.
fn create_private_dir(path: &Path) -> Result<(), WorktreeError> {
    match nix::unistd::mkdir(path, Mode::from_bits_truncate(0o700)) {
        Ok(()) => {
            sync_dir(path.parent().ok_or(WorktreeError::UnsafePath)?)?;
            check_private_dir(path, true)
        }
        Err(nix::errno::Errno::EEXIST) => check_private_dir(path, true),
        Err(_) => Err(WorktreeError::UnsafePath),
    }
}

fn write_marker(marker_path: &Path, marker: &Marker) -> Result<(), WorktreeError> {
    let body = serde_json::to_vec(marker).map_err(|_| WorktreeError::Io)?;
    // O_EXCL makes reservation a single winner even across racing processes.
    let fd = nix::fcntl::open(
        marker_path,
        OFlag::O_WRONLY | OFlag::O_CREAT | OFlag::O_EXCL | OFlag::O_CLOEXEC,
        Mode::from_bits_truncate(0o600),
    )
    .map_err(|error| match error {
        nix::errno::Errno::EEXIST => WorktreeError::AlreadyReserved,
        _ => WorktreeError::UnsafePath,
    })?;
    let mut file = std::fs::File::from(fd);
    // Every failure after exclusive creation unlinks the marker we created:
    // a surviving marker would brick the identity, since reserve would hit
    // EEXIST while verify could never validate a half-written file.
    let mut persisted = std::io::Write::write_all(&mut file, &body).and_then(|_| file.sync_all());
    if persisted.is_ok() {
        persisted = match marker_path.parent().filter(|p| !p.as_os_str().is_empty()) {
            Some(parent) => sync_dir(parent).map_err(|_| std::io::Error::other("sync failed")),
            None => Err(std::io::Error::other("marker parent missing")),
        };
    }
    if persisted.is_err() {
        let _ = unlink(marker_path);
        return Err(WorktreeError::Io);
    }
    Ok(())
}

fn read_marker(marker_path: &Path) -> Result<Marker, WorktreeError> {
    let body = fs::read(marker_path).map_err(|_| WorktreeError::NotFound)?;
    let marker: Marker =
        serde_json::from_slice(&body).map_err(|_| WorktreeError::IdentityMismatch)?;
    if marker.version != RESERVATION_VERSION {
        return Err(WorktreeError::IdentityMismatch);
    }
    Ok(marker)
}

/// Verifies that the base directory satisfies the provisioner premises and
/// creates it with mode 0700 when absent. A base that exists as a symlink, is
/// foreign-owned, or is group/other-writable is refused.
fn ensure_base(base: &Path) -> Result<(), WorktreeError> {
    validate_absolute(base, MAX_BASE_BYTES)?;
    for protected in [
        "/usr", "/etc", "/dev", "/proc", "/sys", "/run", "/boot", "/var",
    ] {
        if base.starts_with(protected) {
            return Err(WorktreeError::InvalidBase);
        }
    }
    match fs::symlink_metadata(base) {
        Ok(meta) => {
            if !meta.is_dir() || meta.file_type().is_symlink() {
                return Err(WorktreeError::InvalidBase);
            }
            if meta.uid() != euid() || meta.mode() & 0o022 != 0 {
                return Err(WorktreeError::NotPrivate);
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let parent = base
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .ok_or(WorktreeError::InvalidBase)?;
            walk_private(parent)?;
            // A racing creator may have made it exist between the metadata
            // check and mkdir; create_private_dir falls through to the same
            // private-premise checks for EEXIST instead of failing.
            create_private_dir(base)?;
        }
        Err(_) => return Err(WorktreeError::UnsafePath),
    }
    walk_private(base)
}

/// Reserves the derived location: `<base>/<project>/<worktree_id>` with a
/// private project directory (0700) and an exclusive marker binding identity.
/// The worktree directory itself is created empty with mode 0700; later Git
/// materialization is explicitly out of this slice.
pub fn reserve(
    derived: &Derived,
    base: &Path,
    created_at: Timestamp,
) -> Result<Reservation, WorktreeError> {
    if derived
        != &Derived::derive(DeriveInputs {
            project_id: &derived.project_id,
            root_id: &derived.root_id,
            stream_id: &derived.stream_id,
            seed: &derived.seed,
        })?
    {
        return Err(WorktreeError::InvalidIdentity);
    }
    ensure_base(base)?;
    let project_dir = base.join(derived.project_id.as_str());
    match fs::symlink_metadata(&project_dir) {
        Ok(meta) if meta.is_dir() && !meta.file_type().is_symlink() => {
            if meta.uid() != euid() || meta.mode() & 0o7777 != 0o700 {
                return Err(WorktreeError::NotPrivate);
            }
        }
        Ok(_) => return Err(WorktreeError::UnsafePath),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            // EEXIST from a racing sibling stream's creator falls through to
            // the same checks rather than failing this reservation.
            create_private_dir(&project_dir)?;
        }
        Err(_) => return Err(WorktreeError::UnsafePath),
    }
    let worktree = derived.worktree_path(base);
    // The identity-specific location is exclusive: an existing directory is
    // reclaimable only while it is empty (an orphan from an interrupted
    // attempt, or a Retain-released location with no data). Any content means
    // retained or materialized work and is never reclaimed.
    let created = match nix::unistd::mkdir(&worktree, Mode::from_bits_truncate(0o700)) {
        Ok(()) => {
            sync_dir(&project_dir)?;
            true
        }
        Err(nix::errno::Errno::EEXIST) => {
            check_private_dir(&worktree, true)?;
            let mut entries = fs::read_dir(&worktree).map_err(|_| WorktreeError::NotFound)?;
            if entries.next().is_some() {
                return Err(WorktreeError::NotEmpty);
            }
            false
        }
        Err(_) => return Err(WorktreeError::UnsafePath),
    };
    let marker = Marker {
        version: RESERVATION_VERSION,
        derived: derived.clone(),
        created_at,
    };
    match write_marker(
        &project_dir.join(format!("{}.json", derived.worktree_id)),
        &marker,
    ) {
        Ok(()) => Ok(Reservation {
            derived: derived.clone(),
            base: base.to_path_buf(),
        }),
        Err(error) => {
            // Never touch a location owned by another reservation: only this
            // call's own freshly created directory is cleaned up.
            if created {
                let _ = fs::remove_dir(&worktree);
            }
            Err(error)
        }
    }
}

fn check_private_dir(path: &Path, exact: bool) -> Result<(), WorktreeError> {
    let meta = fs::symlink_metadata(path).map_err(|_| WorktreeError::NotFound)?;
    if !meta.is_dir() || meta.file_type().is_symlink() {
        return Err(WorktreeError::UnsafePath);
    }
    if meta.uid() != euid() || meta.mode() & 0o022 != 0 || (exact && meta.mode() & 0o7777 != 0o700)
    {
        return Err(WorktreeError::NotPrivate);
    }
    Ok(())
}

/// Re-derives and re-verifies a reservation after restart: path layout, marker
/// identity binding, ownership and private modes. This is the provisioner-side
/// premise the sandbox launcher relies on; the launcher still revalidates.
pub fn verify(reservation: &Reservation) -> Result<Marker, WorktreeError> {
    let base = &reservation.base;
    // Verification never creates state: a missing or non-private base fails.
    validate_absolute(base, MAX_BASE_BYTES)?;
    for protected in [
        "/usr", "/etc", "/dev", "/proc", "/sys", "/run", "/boot", "/var",
    ] {
        if base.starts_with(protected) {
            return Err(WorktreeError::InvalidBase);
        }
    }
    walk_private(base)?;
    let project_dir = base.join(reservation.derived.project_id.as_str());
    check_private_dir(&project_dir, true)?;
    let worktree = reservation.derived.worktree_path(base);
    check_private_dir(&worktree, true)?;
    let marker =
        read_marker(&project_dir.join(format!("{}.json", reservation.derived.worktree_id)))?;
    if marker.derived != reservation.derived {
        return Err(WorktreeError::IdentityMismatch);
    }
    // The reserved worktree is empty until Git materialization populates it;
    // any other content means someone else used this location.
    let mut entries = fs::read_dir(&worktree).map_err(|_| WorktreeError::NotFound)?;
    if entries.next().is_some() {
        return Err(WorktreeError::NotEmpty);
    }
    Ok(marker)
}

/// Releases per the explicit policy. Only an empty reserved directory is ever
/// removed; any content (including materialized Git state) is retained and
/// reported rather than deleted.
pub fn release(reservation: &Reservation, policy: ReleasePolicy) -> Result<Outcome, WorktreeError> {
    let project_dir = reservation
        .base
        .join(reservation.derived.project_id.as_str());
    let worktree = reservation.derived.worktree_path(&reservation.base);
    let marker_path = project_dir.join(format!("{}.json", reservation.derived.worktree_id));
    // Confirm the marker still binds this identity before touching anything:
    // a syntactically valid marker for a different identity is rejected here
    // exactly as verify rejects it.
    let marker = read_marker(&marker_path)?;
    if marker.derived != reservation.derived {
        return Err(WorktreeError::IdentityMismatch);
    }
    match policy {
        ReleasePolicy::Retain => {
            unlink(&marker_path).map_err(|_| WorktreeError::Io)?;
            // The directory is retained; a released directory without a marker
            // is treated as abandoned location, never as removable work.
            Ok(Outcome::Retained)
        }
        ReleasePolicy::DeleteIfEmpty => {
            let mut entries = fs::read_dir(&worktree).map_err(|_| WorktreeError::NotFound)?;
            if entries.next().is_some() {
                return Err(WorktreeError::NotEmpty);
            }
            // Remove the directory first: remove_dir fails closed on any
            // content, and a failure here keeps the marker as the source of
            // truth instead of downgrading the reservation to abandoned.
            fs::remove_dir(&worktree).map_err(|_| WorktreeError::Io)?;
            if unlink(&marker_path).is_err() {
                // Roll back to a consistent marker-plus-empty-directory
                // state so a retry can proceed instead of leaving a stale
                // marker that reserves collide with and release cannot read.
                let _ = nix::unistd::mkdir(&worktree, Mode::from_bits_truncate(0o700));
                let _ = check_private_dir(&worktree, true);
                return Err(WorktreeError::Io);
            }
            sync_dir(&project_dir)?;
            // Removing the project directory is best-effort; concurrent
            // reservations of the same project keep it alive.
            let _ = fs::remove_dir(&project_dir);
            Ok(Outcome::Deleted)
        }
    }
}

/// Lists reservation markers recorded under one project of a base directory.
/// Marker integrity is not re-verified here; call `verify` per reservation.
pub fn list(base: &Path, project: &ProjectId) -> Result<Vec<Marker>, WorktreeError> {
    let project_dir = base.join(project.as_str());
    let mut markers = Vec::new();
    for entry in fs::read_dir(&project_dir).map_err(|_| WorktreeError::NotFound)? {
        let entry = entry.map_err(|_| WorktreeError::Io)?;
        let name = entry.file_name();
        let name = name.to_str().ok_or(WorktreeError::UnsafePath)?;
        let Some(id) = name.strip_suffix(".json") else {
            continue;
        };
        if id.is_empty() || !id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-') {
            continue;
        }
        // One unreadable or foreign marker does not hide the others; callers
        // verify individual reservations for integrity.
        if let Ok(marker) = read_marker(&entry.path()) {
            markers.push(marker);
        }
    }
    markers.sort_by(|a, b| a.derived.worktree_id.cmp(&b.derived.worktree_id));
    Ok(markers)
}

use symbiote_domain::{ChangeStreamId, ProjectId, RootId, Timestamp, WorktreeId};

#[cfg(test)]
mod tests {
    use super::*;
    use symbiote_domain::ProjectId;

    fn derived(stream: &str, seed: &str) -> Derived {
        Derived::derive(DeriveInputs {
            project_id: &ProjectId::new("demo").unwrap(),
            root_id: &symbiote_domain::RootId::new("root").unwrap(),
            stream_id: &symbiote_domain::ChangeStreamId::new(stream).unwrap(),
            seed,
        })
        .unwrap()
    }

    fn temp_base(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "symbiote-worktrees-{}-{}-{tag}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn derivation_is_deterministic_and_collision_resistant() {
        let a = derived("stream-a", "policy-1");
        let a_again = derived("stream-a", "policy-1");
        assert_eq!(a, a_again);
        assert_eq!(a.branch, a_again.branch);
        assert_eq!(a.worktree_id, a_again.worktree_id);
        // Different identities or seeds never collide on names.
        assert_ne!(a, derived("stream-b", "policy-1"));
        assert_ne!(a, derived("stream-a", "policy-2"));
        // The branch is reserved-namespace and every component is ref-safe.
        assert!(a.branch.starts_with("symbiote/demo/stream-a/"));
        for component in a.branch.split('/') {
            assert!(ref_component(component), "bad component: {component}");
        }
        assert!(a.worktree_id.as_str().starts_with("st-"));
        assert!(a.worktree_id.as_str().len() == 3 + 2 * WORKTREE_ID_DIGEST_BYTES);
        assert_eq!(
            a.branch.len() - a.project_id.as_str().len() - a.stream_id.as_str().len(),
            {
                // The invariant part of the branch (prefix + separators + suffix) is fixed-length.
                let again = derived("stream-a", "policy-1");
                again.branch.len()
                    - again.project_id.as_str().len()
                    - again.stream_id.as_str().len()
            }
        );
    }

    #[test]
    fn invalid_seeds_and_identities_are_rejected() {
        for bad in [
            "",
            "sp ace",
            "slash/seed",
            "seed!",
            &"x".repeat(MAX_SEED_BYTES + 1),
        ] {
            assert_eq!(
                Derived::derive(DeriveInputs {
                    project_id: &ProjectId::new("demo").unwrap(),
                    root_id: &symbiote_domain::RootId::new("root").unwrap(),
                    stream_id: &symbiote_domain::ChangeStreamId::new("s").unwrap(),
                    seed: bad,
                })
                .unwrap_err(),
                WorktreeError::InvalidSeed
            );
        }
        assert!(!ref_component(".starts-with-dot"));
        assert!(!ref_component("ends.lock"));
        assert!(!ref_component("a..b"));
        assert!(!ref_component("a@{b"));
    }

    #[test]
    fn reserve_verify_release_round_trip() {
        let base = temp_base("roundtrip");
        let d = derived("stream-rt", "seed");
        let reservation = reserve(&d, &base, Timestamp(100)).unwrap();
        let path = d.worktree_path(&base);
        assert!(path.is_dir());
        let mode = fs::metadata(&path).unwrap().mode();
        assert_eq!(mode & 0o7777, 0o700);
        let project_mode = fs::metadata(base.join("demo")).unwrap().mode();
        assert_eq!(project_mode & 0o7777, 0o700);
        // Verify returns the bound marker identity.
        let marker = verify(&reservation).unwrap();
        assert_eq!(marker.derived, d);
        assert_eq!(marker.created_at, Timestamp(100));
        // Idempotent verification after "restart".
        verify(&reservation).unwrap();
        // DeleteIfEmpty removes the empty reservation; Retain keeps it.
        assert!(matches!(
            release(&reservation, ReleasePolicy::DeleteIfEmpty),
            Ok(Outcome::Deleted)
        ));
        assert!(!path.exists());
        let retained = reserve(&d, &base, Timestamp(101)).unwrap();
        assert!(matches!(
            release(&retained, ReleasePolicy::Retain),
            Ok(Outcome::Retained)
        ));
        assert!(path.is_dir());
        assert!(read_marker(&base.join("demo").join(format!("{}.json", d.worktree_id))).is_err());
    }

    #[test]
    fn double_reservation_is_a_single_winner() {
        let base = temp_base("double");
        let d = derived("stream-2", "seed");
        reserve(&d, &base, Timestamp(1)).unwrap();
        assert_eq!(
            reserve(&d, &base, Timestamp(2)).unwrap_err(),
            WorktreeError::AlreadyReserved
        );
    }

    #[test]
    fn verify_detects_tampering_and_foreign_use() {
        let base = temp_base("tamper");
        let d = derived("stream-3", "seed");
        let reservation = reserve(&d, &base, Timestamp(1)).unwrap();
        // Foreign content inside the reserved directory fails verification.
        fs::write(d.worktree_path(&base).join("sneaky"), b"x").unwrap();
        assert_eq!(verify(&reservation).unwrap_err(), WorktreeError::NotEmpty);
        fs::remove_file(d.worktree_path(&base).join("sneaky")).unwrap();
        // Swapped marker identity fails.
        let marker_path = base.join("demo").join(format!("{}.json", d.worktree_id));
        let mut marker: Marker = serde_json::from_slice(&fs::read(&marker_path).unwrap()).unwrap();
        marker.derived.stream_id = symbiote_domain::ChangeStreamId::new("other").unwrap();
        fs::write(&marker_path, serde_json::to_vec(&marker).unwrap()).unwrap();
        assert_eq!(
            verify(&reservation).unwrap_err(),
            WorktreeError::IdentityMismatch
        );
        // A missing base is not recreated by verification.
        let elsewhere = temp_base("missing");
        let dangling = Reservation {
            derived: d.clone(),
            base: elsewhere.join("does").join("not").join("exist"),
        };
        assert_eq!(verify(&dangling).unwrap_err(), WorktreeError::UnsafePath);
    }

    #[test]
    fn release_never_deletes_nonempty_work() {
        let base = temp_base("nonempty");
        let d = derived("stream-4", "seed");
        let reservation = reserve(&d, &base, Timestamp(1)).unwrap();
        fs::write(d.worktree_path(&base).join("uncommitted.txt"), b"work").unwrap();
        assert_eq!(
            release(&reservation, ReleasePolicy::DeleteIfEmpty).unwrap_err(),
            WorktreeError::NotEmpty
        );
        // Work and reservation survive the refused release.
        assert!(d.worktree_path(&base).join("uncommitted.txt").is_file());
        assert!(verify(&reservation).is_err()); // content present
        assert!(read_marker(&base.join("demo").join(format!("{}.json", d.worktree_id))).is_ok());
    }

    #[test]
    fn retained_and_populated_locations_are_never_reclaimed() {
        let base = temp_base("reclaim");
        let d = derived("stream-6", "seed");
        let reservation = reserve(&d, &base, Timestamp(1)).unwrap();
        // Retain detaches the marker but keeps the directory.
        release(&reservation, ReleasePolicy::Retain).unwrap();
        // Retained content is never silently reclaimed by a fresh reservation.
        fs::write(d.worktree_path(&base).join("recovery"), b"data").unwrap();
        assert_eq!(
            reserve(&d, &base, Timestamp(2)).unwrap_err(),
            WorktreeError::NotEmpty
        );
        assert!(d.worktree_path(&base).join("recovery").is_file());
        // An empty orphan location may be reclaimed by its identity.
        fs::remove_file(d.worktree_path(&base).join("recovery")).unwrap();
        let reclaimed = reserve(&d, &base, Timestamp(3)).unwrap();
        verify(&reclaimed).unwrap();
    }

    #[test]
    fn failed_reservation_cleanup_never_touches_a_live_reservation() {
        let base = temp_base("live");
        let d = derived("stream-7", "seed");
        let live = reserve(&d, &base, Timestamp(1)).unwrap();
        // A racing duplicate must not delete the live reservation's directory.
        assert_eq!(
            reserve(&d, &base, Timestamp(2)).unwrap_err(),
            WorktreeError::AlreadyReserved
        );
        verify(&live).unwrap();
        // A swapped marker cannot release someone else's reservation.
        let other = derived("stream-8", "seed");
        reserve(&other, &base, Timestamp(3)).unwrap();
        let marker_path = base
            .join("demo")
            .join(format!("{}.json", other.worktree_id));
        let mut marker: Marker = serde_json::from_slice(&fs::read(&marker_path).unwrap()).unwrap();
        marker.derived.stream_id = symbiote_domain::ChangeStreamId::new("stream-7").unwrap();
        fs::write(&marker_path, serde_json::to_vec(&marker).unwrap()).unwrap();
        let swapped = Reservation {
            derived: other,
            base: base.clone(),
        };
        assert!(matches!(
            release(&swapped, ReleasePolicy::Retain).unwrap_err(),
            WorktreeError::IdentityMismatch
        ));
        // Both live reservations survive.
        verify(&live).unwrap();
    }

    #[test]
    fn unsafe_bases_are_refused() {
        let d = derived("stream-5", "seed");
        // System locations.
        for protected in ["/usr/demo", "/etc/demo", "/var/demo", "/proc/demo"] {
            assert_eq!(
                reserve(&d, Path::new(protected), Timestamp(1)).unwrap_err(),
                WorktreeError::InvalidBase,
                "base {protected} must be refused"
            );
        }
        // Relative paths.
        assert_eq!(
            reserve(&d, Path::new("relative/base"), Timestamp(1)).unwrap_err(),
            WorktreeError::UnsafePath
        );
        // Group/other-writable existing base.
        let base = temp_base("writable");
        chmod(&base, 0o777);
        assert_eq!(
            reserve(&d, &base, Timestamp(1)).unwrap_err(),
            WorktreeError::NotPrivate
        );
        // Symlinked base component.
        let link_base = temp_base("link");
        let target = temp_base("link-target");
        std::os::unix::fs::symlink(&target, link_base.join("link")).unwrap();
        assert_eq!(
            reserve(&d, &link_base.join("link").join("deeper"), Timestamp(1)).unwrap_err(),
            WorktreeError::UnsafePath
        );
    }

    fn chmod(path: &Path, mode: u32) {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(path).unwrap().permissions();
        permissions.set_mode(mode);
        fs::set_permissions(path, permissions).unwrap();
    }

    #[test]
    fn concurrent_reservations_of_different_streams_do_not_collide() {
        let base = temp_base("parallel");
        let handles: Vec<_> = (0..8)
            .map(|n| {
                let base = base.clone();
                std::thread::spawn(move || {
                    let d = derived(&format!("stream-p{n}"), "seed");
                    reserve(&d, &base, Timestamp(1)).unwrap();
                    d
                })
            })
            .collect();
        for handle in handles {
            let d = handle.join().unwrap();
            verify(&Reservation {
                derived: d,
                base: base.clone(),
            })
            .unwrap();
        }
        assert_eq!(
            list(&base, &ProjectId::new("demo").unwrap()).unwrap().len(),
            8
        );
    }
}
