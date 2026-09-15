//! A test-shaped file that a binding check must refuse: one entry is skipped,
//! and the other is a helper that is not a test at all. If either passed, the
//! ledger could cite a check the workspace never runs.

#[test]
#[ignore]
pub fn a_skipped_check() {}

pub fn a_helper_mistaken_for_a_test() {}
