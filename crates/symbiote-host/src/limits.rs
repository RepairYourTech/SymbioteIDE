//! What a dispatch's declared resource limits mean for the execution it
//! produces. Declaring a limit is the candidate's intent; this is the Host's
//! answer to it, and it has exactly two shapes: the bound it applies to every
//! process the dispatch spawns, or the declared limit it has no bound for —
//! which refuses the run instead of letting it proceed unbounded.
//!
//! One kernel mechanism is available today, applied by the sandboxed launcher
//! ([`crate::shell_executor`]): an address-space ceiling (`RLIMIT_AS`) on the
//! process the dispatch's tools run in, inherited by everything it starts. The
//! declared wall-time, concurrency and token limits are NOT enforced here:
//! see `docs/contracts/workforce-bindings.md`.
use symbiote_domain::{ProcessBound, ResourceLimits, UnboundedLimit};

/// The declared limit the Host cannot bind, named for the operator. The refusal
/// message never carries a value, path or account.
pub fn refusal(limit: UnboundedLimit) -> &'static str {
    match limit {
        UnboundedLimit::CpuRate => {
            "the dispatch declares a CPU demand (max_cpu_millicores) this Host has no kernel bound for"
        }
    }
}

/// The bound this dispatch's execution must carry, or the declared limit that
/// makes the execution unbounded. Every process a dispatch spawns is launched
/// with this bound; a declared limit outside it refuses the run.
pub fn bounds_for(limits: &ResourceLimits) -> Result<ProcessBound, UnboundedLimit> {
    limits.process_bound()
}
