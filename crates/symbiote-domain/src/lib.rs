//! Version-one transport contracts and pure state transitions, not a Host implementation.
//! Aggregate deserialization validates replay and dispatch invariants. Cross-record
//! authentication/provenance remain Host responsibilities; wire data is never authorization.

mod dependency;
mod dispatch;
mod elevation;
mod entities;
mod foreign;
mod ids;
mod lease;
mod lifecycle;
mod limits;
mod model;
mod ontology;
mod preparation;
mod team;
mod work;

pub use dependency::*;
pub use dispatch::*;
pub use elevation::*;
pub use entities::*;
pub use foreign::*;
pub use ids::*;
pub use lease::*;
pub use lifecycle::*;
pub use limits::*;
pub use model::*;
pub use ontology::*;
pub use preparation::*;
pub use team::*;
pub use work::*;

/// Bump for incompatible wire changes. Version one has no persistent migration yet.
pub const SCHEMA_VERSION: u32 = 1;

/// A window's two ends, in milliseconds: `1 s–1 h`, `500 ms–30 s`.
#[cfg(test)]
fn window_ms(text: &str) -> (u64, u64) {
    use symbiote_contract_read::figure;

    let (low, high) = text.trim().split_once('–').expect("a window's two ends");
    let end = |end: &str| {
        let (value, unit) = end.trim().split_at(end.trim().len() - 1);
        let value: u64 = figure(value);
        match unit {
            "ms" => value,
            "s" => value * 1_000,
            "m" => value * 60_000,
            "h" => value * 3_600_000,
            other => panic!("an unknown window unit {other:?}"),
        }
    };
    (end(low), end(high))
}

/// The bounds `work-hierarchy.md` and `scheduling-leases.md` state are the ones this crate enforces:
/// the work-specification, aggregate, history and graph bounds, and the lease window's two ends.
/// Each figure is read from the sentence it is written in and compared to the declaration that
/// applies it, so a document that states a bound this crate has moved past fails here by name
/// rather than in prose nobody reads. Nothing is read out of the crate's source text: every bound
/// has a declaration a case can name.
///
/// What it does not read: the prose around the figures — the capacity refusals, the graph's kind
/// vocabulary, the lease state machine — which the crate's own cases drive, and the protocol's
/// response bound, which the protocol crate holds against its own constant.
///
/// The case lives at the crate root rather than in `tests/` because the work bounds are
/// crate-private facts: `work` names each one once and applies it where the sizes are checked, and
/// a case at the root reads those declarations while the test name the census records stays the one
/// it was.
#[cfg(test)]
#[test]
fn the_contracts_state_the_work_and_lease_bounds_this_crate_enforces() {
    use symbiote_contract_read::{bytes, figure, region};

    use crate::work::{MAX_GRAPH_ITEMS, MAX_SPEC_BYTES, MAX_WORK_BYTES, MAX_WORK_COMMANDS};
    use crate::{MAX_LEASE_MS, MIN_LEASE_MS};

    let hierarchy = include_str!("../../../docs/contracts/work-hierarchy.md");
    let leases = include_str!("../../../docs/contracts/scheduling-leases.md");

    for (label, stated, held) in [
        (
            "bytes in a work specification",
            bytes::<u64>(region(
                hierarchy,
                "Work specifications are bounded to ",
                ",",
            )),
            MAX_SPEC_BYTES as u64,
        ),
        (
            "bytes in an aggregate",
            bytes(region(hierarchy, "aggregates to ", ",")),
            MAX_WORK_BYTES as u64,
        ),
        (
            "commands in a history",
            figure(region(hierarchy, "histories to", " commands")),
            MAX_WORK_COMMANDS as u64,
        ),
        (
            "items in a graph",
            figure(region(hierarchy, "graph validation to ", " items")),
            MAX_GRAPH_ITEMS as u64,
        ),
    ] {
        assert_eq!(
            stated, held,
            "the contract states {stated} {label}, and this crate bounds {held}"
        );
    }

    let stated_window = window_ms(region(leases, "the declared ", " window"));
    let enforced_window = (MIN_LEASE_MS, MAX_LEASE_MS);
    assert_eq!(
        stated_window, enforced_window,
        "the contract states a {stated_window:?} lease window, and this crate declares {enforced_window:?}"
    );
}
