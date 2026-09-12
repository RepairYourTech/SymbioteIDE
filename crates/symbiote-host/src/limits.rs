//! What a dispatch's declared resource limits mean for the execution it
//! produces. Declaring a limit is the candidate's intent; this is the Host's
//! answer to it, and it has exactly two shapes: the bound it applies to every
//! process the dispatch spawns, or a refusal that stops the run instead of
//! letting it proceed unbound.
//!
//! One kernel mechanism is available today, applied by the sandboxed launcher
//! ([`crate::shell_executor`]): an address-space ceiling (`RLIMIT_AS`) on the
//! process the dispatch's tools run in, inherited by everything it starts. The
//! declared wall-time, concurrency and token limits are NOT enforced here:
//! see `docs/contracts/workforce-bindings.md`.
use symbiote_domain::{ProcessBound, ResourceLimits, UnboundedLimit};

/// Why this dispatch's execution has no bound the Host can apply. Refusing is
/// the only other answer: a dispatch never runs past a limit it declared, and
/// it never runs without the bound its contract requires.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BoundRefusal {
    /// The contract records no limits at all — a dispatch journaled before
    /// limits were recorded. There is nothing to bind, so the run refuses
    /// rather than executing unbounded.
    Unrecorded,
    /// The contract declares a limit this Host has no kernel bound for.
    Declared(UnboundedLimit),
}

impl BoundRefusal {
    /// The operator-facing reason, naming the declared field where there is
    /// one. It never carries a value, path or account.
    pub fn refusal(self) -> &'static str {
        match self {
            Self::Unrecorded => {
                "the dispatch records no resource limits, so its execution cannot be bounded"
            }
            Self::Declared(limit) => match limit {
                UnboundedLimit::CpuRate => {
                    "the dispatch declares a CPU demand (max_cpu_millicores) this Host has no kernel bound for"
                }
            },
        }
    }
}

/// The bound this dispatch's execution must carry, or why the Host refuses it.
/// Every process a dispatch spawns is launched with the bound; a refusal stops
/// the run before any worktree or transport exists.
pub fn bounds_for(limits: Option<&ResourceLimits>) -> Result<ProcessBound, BoundRefusal> {
    let limits = limits.ok_or(BoundRefusal::Unrecorded)?;
    limits.process_bound().map_err(BoundRefusal::Declared)
}

#[cfg(test)]
mod tests {
    use super::*;
    use symbiote_domain::{MAX_CPU_MILLICORES, ResourceLimits};

    fn declared(cpu_millicores: Option<u64>) -> ResourceLimits {
        ResourceLimits {
            max_total_tokens: 100_000,
            max_wall_time_ms: 60_000,
            max_concurrency: 1,
            max_memory_bytes: 1 << 30,
            max_cpu_millicores: cpu_millicores,
        }
    }

    #[test]
    fn a_declared_memory_ceiling_is_the_bound_and_nothing_else_is() {
        assert_eq!(
            bounds_for(Some(&declared(None))),
            Ok(ProcessBound {
                address_space_bytes: 1 << 30
            })
        );
        // A contract with no recorded limits has no bound to apply, so it is
        // refused rather than executed unbounded.
        assert_eq!(bounds_for(None), Err(BoundRefusal::Unrecorded));
        assert_eq!(
            bounds_for(None).unwrap_err().refusal(),
            "the dispatch records no resource limits, so its execution cannot be bounded"
        );
        // A CPU rate has no kernel bound, so it refuses by name.
        let demanding = declared(Some(MAX_CPU_MILLICORES));
        assert_eq!(
            bounds_for(Some(&demanding)),
            Err(BoundRefusal::Declared(UnboundedLimit::CpuRate))
        );
        assert!(
            bounds_for(Some(&demanding))
                .unwrap_err()
                .refusal()
                .contains(UnboundedLimit::CpuRate.field())
        );
    }
}
