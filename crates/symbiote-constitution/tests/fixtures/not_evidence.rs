//! A test-shaped file that a binding check must refuse: one entry is skipped,
//! one is a helper that is not a test, and one is a plain `#[test]` in a file
//! the workspace never compiles. If any of them passed, the ledger could cite a
//! check that never runs. This file is deliberately not a cargo test target:
//! cargo discovers only the direct children of a crate's `tests/` directory.

#[test]
#[ignore]
pub fn a_skipped_check() {}

pub fn a_helper_mistaken_for_a_test() {}

#[test]
pub fn a_test_the_harness_never_compiles() {}
