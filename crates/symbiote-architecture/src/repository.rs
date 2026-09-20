//! What cargo reports about this workspace, and the SHA-256 of a committed
//! file.
//!
//! The member list and the dependency names are asked of cargo rather than
//! read out of manifests by hand, so the ledger is checked against the
//! workspace cargo actually builds: a member added to the workspace, or a
//! dependency a provisional choice would introduce, is visible here whether or
//! not anyone remembered to record it.
//!
//! Every read fails rather than passes when it finds nothing. A ledger checked
//! against a workspace that could not be read is not a checked ledger.

use crate::{ContractError, Result};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;

/// The workspace members cargo reports, each with the dependency names it
/// declares, keyed by its path relative to the workspace root.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Workspace {
    members: BTreeMap<String, BTreeSet<String>>,
}

impl Workspace {
    /// Ask cargo which members this workspace has and what they depend on.
    pub fn read(root: &Path) -> Result<Workspace> {
        let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
        let output = Command::new(&cargo)
            .args([
                "metadata",
                "--no-deps",
                "--locked",
                "--format-version",
                "1",
                "--manifest-path",
            ])
            .arg(root.join("Cargo.toml"))
            .output()
            .map_err(|error| {
                ContractError(format!(
                    "cargo could not be run to list the workspace members: {error}"
                ))
            })?;
        if !output.status.success() {
            return Err(ContractError(format!(
                "cargo metadata failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            )));
        }
        let metadata: Value = serde_json::from_slice(&output.stdout)
            .map_err(|error| ContractError(format!("cargo metadata was not readable: {error}")))?;
        Workspace::from_metadata(&metadata, root)
    }

    /// The same reading from a `cargo metadata --no-deps` document, so the rules
    /// over members and dependencies can be driven without running cargo.
    pub fn from_metadata(metadata: &Value, root: &Path) -> Result<Workspace> {
        let root = canonical(root);
        let mut members = BTreeMap::new();
        for package in metadata["packages"].as_array().into_iter().flatten() {
            let Some(manifest) = package["manifest_path"].as_str() else {
                continue;
            };
            let Some(directory) = Path::new(manifest).parent() else {
                continue;
            };
            let mut dependencies = BTreeSet::new();
            for dependency in package["dependencies"].as_array().into_iter().flatten() {
                if let Some(name) = dependency["name"].as_str() {
                    dependencies.insert(name.to_string());
                }
            }
            members.insert(relative(&root, directory), dependencies);
        }
        if members.is_empty() {
            return Err(ContractError(
                "cargo reported no workspace member to check the ledger against".into(),
            ));
        }
        Ok(Workspace { members })
    }

    /// Whether cargo reports this path as a workspace member.
    pub fn contains(&self, member: &str) -> bool {
        self.members.contains_key(member)
    }

    /// Every member path cargo reports, in order.
    pub fn paths(&self) -> impl Iterator<Item = &str> {
        self.members.keys().map(String::as_str)
    }

    /// Which of `names` this member declares, as cargo names them.
    pub fn declares(&self, member: &str, names: &[String]) -> Vec<String> {
        let Some(declared) = self.members.get(member) else {
            return Vec::new();
        };
        names
            .iter()
            .filter(|name| declared.contains(*name))
            .cloned()
            .collect()
    }
}

/// The SHA-256 of a repository-relative file, or `None` when the tree does not
/// hold it.
pub fn hash(root: &Path, path: &str) -> Option<String> {
    Some(digest(&std::fs::read(root.join(path)).ok()?))
}

/// The SHA-256 of a byte string: one owner for what a digest is here, so a file
/// in the tree, a stored copy and a recorded transition are all named the same
/// way.
pub fn digest(bytes: &[u8]) -> String {
    let mut digest = Sha256::new();
    digest.update(bytes);
    format!("{:x}", digest.finalize())
}

fn relative(root: &Path, directory: &Path) -> String {
    let directory = canonical(directory);
    directory
        .strip_prefix(root)
        .map(|path| path.to_string_lossy().replace('\\', "/"))
        .unwrap_or_else(|_| directory.to_string_lossy().into_owned())
}

fn canonical(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}
