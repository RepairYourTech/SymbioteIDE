//! What this repository's `repository.md` states about this crate, held to the crate.
use symbiote_contract_read::{bytes, figure, region};
use symbiote_repo::{MAX_GIT_OUTPUT_BYTES, MAX_UNTRACKED_FILE_BYTES, MAX_UNTRACKED_FILES};

/// The three bounds `repository.md` states are the ones this crate enforces: the git invocation's
/// captured-output bound, and the untracked file head count and per-file bound. Each figure is read
/// from the sentence it is written in, so a document that states a bound this crate has moved past
/// fails here by name rather than in prose nobody reads.
///
/// What it does not read: the prose around the figures — the total-run-diff degradation, the
/// watchdog's process-group kill, the temporary-index discipline — which the crate's own tests drive
/// against a real git, and the crates the document links to for other surfaces.
#[test]
fn the_contract_states_the_bounds_this_crate_enforces() {
    let contract = include_str!("../../../docs/contracts/repository.md");

    let stated_output: usize = bytes(region(contract, "captured through a ", " bound"));
    assert_eq!(
        stated_output, MAX_GIT_OUTPUT_BYTES,
        "the contract states a {stated_output}-byte capture bound, and this crate bounds {MAX_GIT_OUTPUT_BYTES}"
    );

    let stated_heads: usize = figure(region(
        contract,
        "plus up to",
        " untracked file content heads",
    ));
    assert_eq!(
        stated_heads, MAX_UNTRACKED_FILES,
        "the contract states {stated_heads} untracked file heads, and this crate reads {MAX_UNTRACKED_FILES}"
    );

    let stated_head: usize = bytes(region(contract, "content heads of at most ", " each"));
    assert_eq!(
        stated_head, MAX_UNTRACKED_FILE_BYTES,
        "the contract states {stated_head} bytes per untracked head, and this crate reads {MAX_UNTRACKED_FILE_BYTES}"
    );
}
