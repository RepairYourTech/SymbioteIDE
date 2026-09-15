//! How a path is spelled: canonical for comparison, relative to the workspace in
//! a record, and whether a file is one the scan reads.

use std::path::{Path, PathBuf};

pub(crate) fn canonical(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

/// A file's path as a record spells it: relative to the workspace, or absolute
/// when it lives outside it.
pub(crate) fn relative_to(workspace: &Path, source: &Path) -> PathBuf {
    source
        .strip_prefix(workspace)
        .map(Path::to_path_buf)
        .unwrap_or_else(|_| source.to_path_buf())
}

pub(crate) fn is_rust(source: &Path) -> bool {
    source
        .extension()
        .is_some_and(|extension| extension == "rs")
}
