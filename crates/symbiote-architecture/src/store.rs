//! The immutable, content-addressed copy this repository keeps of the artifacts
//! its decisions rest on (#173).
//!
//! An accepted decision is the text it was accepted from and its evidence is
//! the bytes it cited. Both live in the tree where they can be edited; the
//! store holds the copy that is named by its own digest, so what was accepted
//! is *recovered* rather than described: an edit to the working copy shows up
//! as a difference from the store instead of silently changing what a decision
//! rests on, and the store cannot hold a byte string whose name it does not
//! hash to.
//!
//! This is a directory of files in the repository, not a service. Durability,
//! retention, replication and transactional writes belong to the Host that
//! would serve them, which is a runtime integration.

use crate::ledger::Ledger;
use crate::repository::hash;
use std::collections::BTreeMap;
use std::path::Path;

/// The store this repository commits, relative to the workspace root.
pub const STORE_DIR: &str = "docs/architecture/store";

/// Every artifact a record cites by path, with the digest it cites it at: the
/// accepted text a record interprets and the evidence it rests on. An `https://`
/// reference names something outside the tree, which this repository records
/// rather than stores.
pub fn cited(ledger: &Ledger) -> Vec<(String, String)> {
    let mut cited: BTreeMap<String, String> = BTreeMap::new();
    for record in &ledger.decisions {
        cited.insert(record.record_sha256.clone(), record.record.clone());
        for evidence in &record.draft.evidence {
            if !evidence.artifact.starts_with("https://") {
                cited.insert(evidence.sha256.clone(), evidence.artifact.clone());
            }
        }
    }
    cited
        .into_iter()
        .map(|(digest, artifact)| (artifact, digest))
        .collect()
}

/// The bytes the store holds at a digest, or `None` when it holds none.
pub fn entry(root: &Path, digest: &str) -> Option<Vec<u8>> {
    std::fs::read(root.join(STORE_DIR).join(digest)).ok()
}

/// Every digest the store's own directory names, in order. A store that cannot
/// be listed is reported as no store rather than as an empty one, so a missing
/// store refuses through the entries it should have held.
pub fn names(root: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(root.join(STORE_DIR))
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| entry.file_name().into_string().ok())
        .collect();
    names.sort();
    names
}

/// The digest a working copy of `artifact` has, or `None` when the tree does
/// not hold it.
pub fn working(root: &Path, artifact: &str) -> Option<String> {
    hash(root, artifact)
}
