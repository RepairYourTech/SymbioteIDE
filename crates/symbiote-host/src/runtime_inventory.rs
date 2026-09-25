//! This Host's own runtime inventory, published to and read from its private
//! state directory (#186).
//!
//! The [runtime discovery](../../../docs/contracts/runtime-discovery.md) contract
//! is a data contract: an inventory is observations, never authority. Something
//! has to move one from a discovery run into the Host that will serve it, and
//! until a Host-side probe exists that something is an operator publishing a
//! document this Host re-validates from scratch. So this module owns two
//! directions over one private file, and both apply the same discipline:
//!
//! * [`Published::publish`] is operator-side and writes. It parses the document
//!   with the discovery crate's own parser — never a local re-implementation —
//!   requires every record to name *this* Host, refuses a document past the
//!   published bound, and then installs it atomically at mode 0600 through a
//!   temporary file in the same directory, so a reader never observes a
//!   half-written document.
//! * [`Published::read`] is what the daemon serves. It opens the file the way
//!   the Host's own identity and policy files are opened — `O_NOFOLLOW`, then
//!   classified from the *opened handle*, so a path swapped for a symlink
//!   between a check and a read cannot slip a different document past the gate —
//!   and re-validates the whole document through the same parser on every read.
//!   Nothing is cached: the file is the durable record, and a read after a
//!   republish is the new document.
//!
//! Nothing here is a client write path. The wire surface is one owner-only read
//! (`get_runtime_inventory`); this publisher is a local operator action, and it
//! grants no activation, credential, installation or billing authority. A
//! record past its own `expires_at` is served, not filtered: expiry is the
//! reader's call, and dropping records on the way out would make the document
//! disagree with what this Host actually holds.

use std::{
    fs::{File, OpenOptions},
    io::{Read, Write},
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::{Path, PathBuf},
};
use symbiote_domain::HostId;
use symbiote_runtime_discovery::Inventory;

use crate::paths;

/// The published bound on the file this Host serves, and therefore on the
/// response it builds. It belongs to the discovery contract — the Host applies
/// that crate's figure rather than keeping a second one — and the assertion
/// below is why it is what it is: a document this Host will serve, plus the
/// response envelope carrying it, has to fit the protocol's response bound,
/// because the alternative is discovering at the transport that the answer did
/// not fit, after the read was already dispatched.
pub use symbiote_runtime_discovery::MAX_PUBLISHED_BYTES;
const _: () = assert!(
    MAX_PUBLISHED_BYTES + 64 * 1024 < symbiote_protocol::MAX_RESPONSE_BYTES,
    "a servable inventory plus its response envelope must fit the protocol's response bound"
);

/// Why this Host will not serve a runtime inventory, as a closed vocabulary.
/// The names are what an operator reads; they are never a path, a record, an
/// identity or a size the document said, and they do not distinguish *which*
/// private-file check failed, because a reader that could tell "wrong mode" from
/// "foreign-owned" would be an oracle for probing this Host's state directory.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// Nothing has been published into this Host's state directory.
    Absent,
    /// The path is not a private regular file of this user: a symlink, a
    /// directory, a device, a hard link, another user's file, or one with any
    /// group or other bit set.
    Unsafe,
    /// The file could not be read at all.
    Unreadable,
    /// The document is not one this Host's discovery contract speaks: a
    /// malformed body, a schema version it does not publish, a record that
    /// fails its own validation, or a duplicate identity.
    Unparseable,
    /// The document is larger than [`MAX_PUBLISHED_BYTES`].
    Oversized,
    /// A record names a Host other than this one. An inventory is this Host's
    /// own observation of this machine; a document carrying another Host's
    /// records is not this Host's document.
    ForeignHost,
}
impl Refusal {
    /// The refusal as it appears in a protocol error message. One owner for
    /// both the vocabulary and its spelling, so a case can assert the set.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Absent => "no_published_inventory",
            Self::Unsafe => "unsafe_inventory_file",
            Self::Unreadable => "unreadable_inventory",
            Self::Unparseable => "unparseable_inventory",
            Self::Oversized => "oversized_inventory",
            Self::ForeignHost => "foreign_host_inventory",
        }
    }
    /// Every refusal, in declaration order: the vocabulary a case pins.
    pub const fn all() -> [Self; 6] {
        [
            Self::Absent,
            Self::Unsafe,
            Self::Unreadable,
            Self::Unparseable,
            Self::Oversized,
            Self::ForeignHost,
        ]
    }
}
impl std::fmt::Display for Refusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}
impl std::error::Error for Refusal {}

/// A publish that was refused. The document was not installed, so the Host keeps
/// serving whatever it held before.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PublishError {
    /// The document is not one this Host's discovery contract speaks.
    Unparseable,
    /// The document is larger than [`MAX_PUBLISHED_BYTES`].
    Oversized,
    /// A record names a Host other than this one.
    ForeignHost,
    /// The state directory refused the write.
    Unwritable,
}
impl PublishError {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Unparseable => Refusal::Unparseable.name(),
            Self::Oversized => Refusal::Oversized.name(),
            Self::ForeignHost => Refusal::ForeignHost.name(),
            Self::Unwritable => "inventory_not_installed",
        }
    }
    /// The refusal a read would report for the same document, so publishing and
    /// reading agree on why. `Unwritable` is publication-only and has no read
    /// counterpart: nothing was installed, so the previous document is what a
    /// read still answers.
    pub const fn as_refusal(self) -> Option<Refusal> {
        match self {
            Self::Unparseable => Some(Refusal::Unparseable),
            Self::Oversized => Some(Refusal::Oversized),
            Self::ForeignHost => Some(Refusal::ForeignHost),
            Self::Unwritable => None,
        }
    }
}
impl std::fmt::Display for PublishError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}
impl std::error::Error for PublishError {}

/// The published inventory of one Host state directory.
#[derive(Clone, Debug)]
pub struct Published {
    directory: PathBuf,
    host_id: HostId,
}

impl Published {
    pub fn new(directory: &Path, host_id: HostId) -> Self {
        Self {
            directory: directory.to_path_buf(),
            host_id,
        }
    }

    /// The file this Host publishes into and serves from.
    pub fn path(&self) -> PathBuf {
        self.directory.join(paths::RUNTIME_INVENTORY_FILE)
    }

    /// The Host identity every record in this Host's document must name.
    pub fn host_id(&self) -> &HostId {
        &self.host_id
    }

    /// This Host's inventory, re-validated from the file on every call.
    ///
    /// The order is deliberate and is the same one `identity::load` uses: open
    /// without following a symlink, classify the *opened handle*, then read
    /// bounded bytes from that same handle. A check-then-open path would be a
    /// TOCTOU on the one file whose contents are served to a client.
    pub fn read(&self) -> Result<Inventory, Refusal> {
        let file = match OpenOptions::new()
            .read(true)
            // `O_NOFOLLOW` refuses a final component that is a symlink, and
            // `O_NONBLOCK` keeps a FIFO at the path from parking this read
            // forever; the handle is classified as a regular file below either
            // way, so neither flag is a trust decision.
            .custom_flags(nix::libc::O_NOFOLLOW | nix::libc::O_NONBLOCK)
            .open(self.path())
        {
            Ok(file) => file,
            // A symlink is a discipline failure, not a missing document: the
            // two have different operator answers, so they have different names.
            Err(error) if error.raw_os_error() == Some(nix::libc::ELOOP) => {
                return Err(Refusal::Unsafe);
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Err(Refusal::Absent);
            }
            Err(_) => return Err(Refusal::Unreadable),
        };
        let metadata = file.metadata().map_err(|_| Refusal::Unreadable)?;
        if !metadata.file_type().is_file()
            || metadata.nlink() != 1
            || metadata.uid() != nix::unistd::geteuid().as_raw()
            || metadata.mode() & 0o777 != 0o600
        {
            return Err(Refusal::Unsafe);
        }
        // One byte past the bound, so "exactly at the bound" is served and "one
        // past it" is refused without trusting a length the file could change.
        let mut bytes = Vec::new();
        file.take(MAX_PUBLISHED_BYTES as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| Refusal::Unreadable)?;
        if bytes.len() > MAX_PUBLISHED_BYTES {
            return Err(Refusal::Oversized);
        }
        let text = String::from_utf8(bytes).map_err(|_| Refusal::Unparseable)?;
        // The discovery crate's own parser, so a document this Host accepts is
        // a document that crate validates: no second, laxer reader here.
        let inventory = Inventory::parse(&text).map_err(|_| Refusal::Unparseable)?;
        if !self.own_records(&inventory) {
            return Err(Refusal::ForeignHost);
        }
        Ok(inventory)
    }

    /// Install a discovery document as this Host's inventory.
    ///
    /// The write is atomic: a temporary file in the same directory, synced, then
    /// renamed over the published name. A reader — this Host or an operator
    /// reading the directory — sees the old document or the new one, never a
    /// partial write, and a failed publish leaves the previous document in
    /// place.
    pub fn publish(&self, document: &str) -> Result<(), PublishError> {
        if document.len() > MAX_PUBLISHED_BYTES {
            return Err(PublishError::Oversized);
        }
        let inventory = Inventory::parse(document).map_err(|_| PublishError::Unparseable)?;
        if !self.own_records(&inventory) {
            return Err(PublishError::ForeignHost);
        }
        let path = self.path();
        // The temporary name carries the published name, so an operator looking
        // for a half-finished publish can see one, and it is created new in a
        // directory only this user can write.
        let temporary = self
            .directory
            .join(format!(".{}.new", paths::RUNTIME_INVENTORY_FILE));
        // A leftover temporary from an interrupted publish is removed rather
        // than reopened: `.mode(0o600)` only applies to a file this open
        // *creates*, so reusing one would install a document at whatever mode
        // an earlier run left behind.
        let _ = std::fs::remove_file(&temporary);
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(&temporary)
            .map_err(|_| PublishError::Unwritable)?;
        file.write_all(document.as_bytes())
            .and_then(|()| file.sync_all())
            .map_err(|_| PublishError::Unwritable)?;
        drop(file);
        std::fs::rename(&temporary, &path).map_err(|_| PublishError::Unwritable)?;
        File::open(&self.directory)
            .and_then(|directory| directory.sync_all())
            .map_err(|_| PublishError::Unwritable)?;
        Ok(())
    }

    /// Every record names this Host. The intent and the observation are both
    /// checked: the discovery contract requires a record's *isolation evidence*
    /// to name the observing Host but does not itself require the intent and the
    /// observation to agree, so a document whose intent names this machine while
    /// its observation names another is refused here rather than served as this
    /// Host's own.
    fn own_records(&self, inventory: &Inventory) -> bool {
        inventory.records().iter().all(|record| {
            record.intent.host_id == self.host_id && record.observation.host_id == self.host_id
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT: AtomicU64 = AtomicU64::new(0);

    /// A state directory private to this user, so the Host's own directory
    /// refusal cannot be what a case is measuring.
    fn state() -> PathBuf {
        let directory = std::env::temp_dir().join(format!(
            "symbiote-runtime-inventory-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        crate::transport::private_directory(&directory).unwrap();
        directory
    }

    fn published(directory: &Path) -> Published {
        Published::new(directory, HostId::new("host_published").unwrap())
    }

    /// A minimal valid record for `host_id`, written as the discovery contract
    /// writes it. Field values are the ones the record's own validation demands.
    pub(crate) fn record(host_id: &str, profile: &str) -> String {
        serde_json::json!({
            "schema_version": 1,
            "intent": {
                "profile_id": profile,
                "installation_id": "installation-1",
                "host_id": host_id,
                "runtime_kind": "NATIVE_SYMBIOTE",
                "adapter_id": "adapter-1",
                "instance_name": "Codex",
                "config_identity": "codex-default"
            },
            "observation": {
                "config_identity": "codex-default",
                "profile_id": profile,
                "installation_id": "installation-1",
                "host_id": host_id,
                "runtime_kind": "NATIVE_SYMBIOTE",
                "adapter_id": "adapter-1",
                "version": {"status": "known", "value": "0.118.0"},
                "interface": {"status": "known", "value": {"name": "app_server", "version": "0.118.0"}},
                "facts": {
                    "health": {"status": "known", "value": "reachable"},
                    "authentication": {"status": "known", "value": {"mode": "none", "state": "required", "account_ref": null}},
                    "models": {"status": "unknown"},
                    "methods": {"initialize": "available"},
                    "isolation": {"status": "unknown"}
                },
                "observed_at": 1000,
                "expires_at": 2000,
                "provenance": {
                    "probe_id": "probe-1",
                    "source": "sandboxed_probe",
                    "adapter_revision": "0.118.0"
                }
            }
        })
        .to_string()
    }

    /// A one-record inventory document naming `host_id`.
    pub(crate) fn document(host_id: &str) -> String {
        serde_json::json!({
            "schema_version": 1,
            "records": [serde_json::from_str::<serde_json::Value>(&record(host_id, "profile-1")).unwrap()],
        })
        .to_string()
    }

    #[test]
    fn an_unpublished_inventory_is_absent_and_a_published_one_is_served_whole() {
        let directory = state();
        let published = published(&directory);
        assert_eq!(published.read().unwrap_err(), Refusal::Absent);
        published.publish(&document("host_published")).unwrap();
        let inventory = published.read().unwrap();
        assert_eq!(inventory.records().len(), 1);
        assert_eq!(
            inventory.records()[0].intent.profile_id.as_str(),
            "profile-1"
        );
        assert_eq!(
            published.path(),
            directory.join(paths::RUNTIME_INVENTORY_FILE)
        );
        // The installed file is the private file the read requires.
        let metadata = std::fs::symlink_metadata(published.path()).unwrap();
        assert_eq!(metadata.mode() & 0o777, 0o600);
        assert_eq!(metadata.nlink(), 1);
        assert!(published.read().is_ok());
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn a_republish_replaces_the_document_rather_than_appending_to_it() {
        let directory = state();
        let published = published(&directory);
        published.publish(&document("host_published")).unwrap();
        // A longer second document, so a reader that appended or concatenated
        // would see two records or fail to parse.
        let two = serde_json::json!({
            "schema_version": 1,
            "records": [
                serde_json::from_str::<serde_json::Value>(&record("host_published", "profile-1")).unwrap(),
                serde_json::from_str::<serde_json::Value>(&record("host_published", "profile-2")).unwrap(),
            ],
        })
        .to_string();
        published.publish(&two).unwrap();
        assert_eq!(published.read().unwrap().records().len(), 2);
        // No temporary file survives a successful publish.
        assert!(
            !directory
                .join(format!(".{}.new", paths::RUNTIME_INVENTORY_FILE))
                .exists()
        );
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn a_record_past_its_own_expiry_is_served_rather_than_dropped() {
        let directory = state();
        let published = published(&directory);
        published.publish(&document("host_published")).unwrap();
        // Read far past `expires_at`: the document is what this Host observed,
        // and the reader decides staleness from the record it is served.
        let inventory = published.read().unwrap();
        assert_eq!(inventory.records()[0].observation.expires_at.0, 2000);
        assert!(published.read().is_ok());
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn a_file_that_is_not_a_private_regular_file_of_this_user_is_refused() {
        use std::os::unix::fs::{PermissionsExt, symlink};
        let directory = state();
        let published = published(&directory);
        let path = published.path();

        // A symlink at the published name.
        symlink(directory.join("elsewhere"), &path).unwrap();
        assert_eq!(published.read().unwrap_err(), Refusal::Unsafe);
        std::fs::remove_file(&path).unwrap();

        // A readable file with a group bit set.
        std::fs::write(&path, document("host_published")).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o640)).unwrap();
        assert_eq!(published.read().unwrap_err(), Refusal::Unsafe);
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
        assert!(published.read().is_ok());

        // A second name for the same file: one writer, two readers of it.
        std::fs::hard_link(&path, directory.join("copy.json")).unwrap();
        assert_eq!(published.read().unwrap_err(), Refusal::Unsafe);
        std::fs::remove_file(directory.join("copy.json")).unwrap();
        std::fs::remove_file(&path).unwrap();

        // A FIFO at the published name would park a reader forever without
        // `O_NONBLOCK`; it is refused as what it is, not waited on.
        nix::unistd::mkfifo(
            &path,
            nix::sys::stat::Mode::S_IRUSR | nix::sys::stat::Mode::S_IWUSR,
        )
        .unwrap();
        assert_eq!(published.read().unwrap_err(), Refusal::Unsafe);
        std::fs::remove_file(&path).unwrap();

        // A directory at the published name.
        std::fs::create_dir(&path).unwrap();
        assert_eq!(published.read().unwrap_err(), Refusal::Unsafe);
        std::fs::remove_dir(&path).unwrap();
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn a_document_this_crate_does_not_speak_is_refused_by_name() {
        let directory = state();
        let published = published(&directory);
        for (document, expected) in [
            ("not json at all", Refusal::Unparseable),
            ("{}", Refusal::Unparseable),
            (
                r#"{"schema_version": 2, "records": []}"#,
                Refusal::Unparseable,
            ),
            (
                r#"{"schema_version": 1, "records": [{"schema_version": 1}]}"#,
                Refusal::Unparseable,
            ),
            (
                r#"{"schema_version": 1, "records": [], "extra": true}"#,
                Refusal::Unparseable,
            ),
            // A valid document that names another Host, in the intent, in the
            // observation, and in both at once.
            (&document("host_other"), Refusal::ForeignHost),
        ] {
            // Written directly rather than published, so a publish refusal does
            // not hide the read refusal this case is measuring.
            std::fs::write(published.path(), document).unwrap();
            std::fs::set_permissions(
                published.path(),
                <std::fs::Permissions as std::os::unix::fs::PermissionsExt>::from_mode(0o600),
            )
            .unwrap();
            assert_eq!(published.read().unwrap_err(), expected, "{document}");
            std::fs::remove_file(published.path()).unwrap();
        }
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn a_record_whose_intent_names_this_host_but_whose_observation_does_not_is_foreign() {
        let directory = state();
        let published = published(&directory);
        let mut value: serde_json::Value =
            serde_json::from_str(&document("host_published")).unwrap();
        value["records"][0]["observation"]["host_id"] = serde_json::json!("host_other");
        // The parser accepts it — the discovery contract does not require the
        // two to agree — so the refusal has to be this Host's own.
        Inventory::parse(&value.to_string()).unwrap();
        std::fs::write(published.path(), value.to_string()).unwrap();
        std::fs::set_permissions(
            published.path(),
            <std::fs::Permissions as std::os::unix::fs::PermissionsExt>::from_mode(0o600),
        )
        .unwrap();
        assert_eq!(published.read().unwrap_err(), Refusal::ForeignHost);
        std::fs::remove_dir_all(directory).unwrap();
    }

    /// A record carrying the most model listing the discovery contract allows
    /// (256), each with a 128-character upstream identifier — the largest shape
    /// a single record can take, and therefore the way to fill a document up to
    /// near the published bound without inventing a field the parser allows.
    fn fat_record(host_id: &str, profile: &str) -> serde_json::Value {
        let mut value: serde_json::Value = serde_json::from_str(&record(host_id, profile)).unwrap();
        let models: Vec<serde_json::Value> = (0..256)
            .map(|index| {
                serde_json::json!({
                    "upstream_model": format!("model-{index:03}-{}", "x".repeat(115)),
                    "canonical_model_id": null,
                    "context_tokens": {"status": "known", "value": 4096},
                })
            })
            .collect();
        value["observation"]["facts"]["models"] =
            serde_json::json!({ "status": "known", "value": models });
        value
    }

    #[test]
    fn a_document_past_the_published_bound_is_refused_and_a_real_one_is_served() {
        let directory = state();
        let published = published(&directory);
        // A real inventory near the bound: the published file is large enough
        // that the response envelope has to fit around it, and this is the
        // shape the bound has to leave room for rather than a padded string.
        let records: Vec<serde_json::Value> = (0..6)
            .map(|index| fat_record("host_published", &format!("profile-{index}")))
            .collect();
        let document = serde_json::json!({"schema_version": 1, "records": records}).to_string();
        assert!(
            document.len() > 256 * 1024 && document.len() < MAX_PUBLISHED_BYTES,
            "{} bytes",
            document.len()
        );
        published.publish(&document).unwrap();
        assert_eq!(published.read().unwrap().records().len(), 6);

        // Past the bound, both directions refuse by name, and the read refuses
        // before parsing: the bytes need not be a document at all, because the
        // bound is applied to the file rather than to something the file claims
        // about itself.
        let oversized = format!("\"{}\"", "y".repeat(MAX_PUBLISHED_BYTES + 1));
        assert_eq!(
            published.publish(&oversized).unwrap_err(),
            PublishError::Oversized
        );
        // The refused publish never touched the file, so the document this Host
        // held is still the document it serves.
        assert_eq!(published.read().unwrap().records().len(), 6);
        assert_eq!(
            PublishError::Oversized.as_refusal(),
            Some(Refusal::Oversized)
        );
        // Now the file itself is past the bound.
        std::fs::write(published.path(), &oversized).unwrap();
        std::fs::set_permissions(
            published.path(),
            <std::fs::Permissions as std::os::unix::fs::PermissionsExt>::from_mode(0o600),
        )
        .unwrap();
        assert_eq!(published.read().unwrap_err(), Refusal::Oversized);
        std::fs::remove_dir_all(directory).unwrap();
    }

    /// The read is bounded *while it reads*, not only judged after: a file far
    /// larger than the bound must never be read into memory whole to discover
    /// that it is too large. The refusal below is observable; the allocation
    /// bound is not distinguishable by an assertion over a fixture this size, so
    /// it is held here as a source pin rather than left to rot.
    #[test]
    fn the_read_is_bounded_while_it_reads_and_the_refusal_is_the_length_check() {
        let source = include_str!("runtime_inventory.rs");
        assert!(
            source.contains("file.take(MAX_PUBLISHED_BYTES as u64 + 1)"),
            "the read is taken with the published bound plus one byte"
        );
    }

    #[test]
    fn every_refusal_name_is_distinct_and_publish_reports_the_read_refusal() {
        let names: Vec<&str> = Refusal::all().iter().map(|r| r.name()).collect();
        let mut sorted = names.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), names.len(), "{names:?}");
        for refusal in Refusal::all() {
            assert!(
                refusal
                    .name()
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b == b'_')
            );
        }
        assert_eq!(PublishError::Unwritable.as_refusal(), None);
    }
}
