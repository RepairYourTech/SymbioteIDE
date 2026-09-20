//! What the immutable store must hold for the artifacts a record cites.
//!
//! A store that holds no copy of an artifact a record rests on preserves
//! nothing; a copy whose bytes are not the digest it is named by is not
//! content-addressed; and an entry no record cites is data this repository has
//! no claim on. Each is a finding, so the store is exactly the content the
//! decisions rest on rather than a directory with files in it.

use super::Problem;
use crate::ledger::Ledger;
use crate::repository::digest;
use crate::store::{STORE_DIR, cited, entry, names};
use std::collections::BTreeSet;
use std::path::Path;

pub(super) fn store_problems(ledger: &Ledger, root: &Path, problems: &mut Vec<Problem>) {
    let cited = cited(ledger);
    for (artifact, recorded) in &cited {
        match entry(root, recorded) {
            None => problems.push(Problem::new(
                format!("{STORE_DIR}/{recorded}"),
                format!(
                    "the immutable store holds no copy of {artifact}, which a record cites at {recorded}"
                ),
            )),
            Some(bytes) if &digest(&bytes) != recorded => problems.push(Problem::new(
                format!("{STORE_DIR}/{recorded}"),
                format!(
                    "the store's copy for {artifact} is {}, not the content its name claims",
                    digest(&bytes)
                ),
            )),
            Some(_) => {}
        }
    }
    let known: BTreeSet<&str> = cited
        .iter()
        .map(|(_, recorded)| recorded.as_str())
        .collect();
    for name in names(root) {
        if !known.contains(name.as_str()) {
            problems.push(Problem::new(
                format!("{STORE_DIR}/{name}"),
                "the store holds an entry that no recorded decision cites",
            ));
        }
    }
}
