//! The resource limits a staffing candidate declares, and what a Host can
//! actually enforce for them. Declaring a limit is an intent; the Host must
//! bind it to the execution the dispatch produces or refuse that execution —
//! it never runs past a limit it was given.
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The contract's own ceilings. They bound the *declaration*; what the Host
/// enforces is [`ResourceLimits::process_bound`].
pub const MAX_TOTAL_TOKENS: u64 = 1_000_000_000;
pub const MAX_WALL_TIME_MS: u64 = 604_800_000;
pub const MAX_CONCURRENCY: u16 = 64;
pub const MAX_MEMORY_BYTES: u64 = 1 << 50;
pub const MAX_CPU_MILLICORES: u64 = 64_000;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ResourceLimits {
    pub max_total_tokens: u64,
    pub max_wall_time_ms: u64,
    pub max_concurrency: u16,
    pub max_memory_bytes: u64,
    /// The CPU the candidate's dispatch needs, in millicores. Absent means the
    /// candidate declares no CPU demand and the readiness report asks the Host
    /// for none: concurrency is not a CPU reservation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_cpu_millicores: Option<u64>,
}

/// The declared limit a Host has no kernel bound for. A declared limit outside
/// the enforced set is a refusal, never a silent omission.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum UnboundedLimit {
    /// A CPU *rate* has no bound on Linux without cgroup delegation: RLIMIT_CPU
    /// caps cumulative CPU time, not a rate, so the Host refuses rather than
    /// substituting a different limit than the one declared.
    CpuRate,
}

impl UnboundedLimit {
    /// The declared field this limit names. A stable identifier, never an
    /// input value, path or account.
    pub fn field(self) -> &'static str {
        match self {
            Self::CpuRate => "max_cpu_millicores",
        }
    }
}

/// The bound a Host applies to every process a dispatch spawns. Today one
/// kernel mechanism is available: an address-space ceiling (RLIMIT_AS), which
/// bounds allocation for the process and everything it starts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProcessBound {
    pub address_space_bytes: u64,
}

impl ResourceLimits {
    /// The contract's own bounds, independent of any Host. A limit outside them
    /// is not a limit any Host could honour.
    pub fn validate(&self) -> bool {
        self.max_total_tokens > 0
            && self.max_total_tokens <= MAX_TOTAL_TOKENS
            && self.max_wall_time_ms > 0
            && self.max_wall_time_ms <= MAX_WALL_TIME_MS
            && self.max_concurrency > 0
            && self.max_concurrency <= MAX_CONCURRENCY
            && self.max_memory_bytes > 0
            && self.max_memory_bytes <= MAX_MEMORY_BYTES
            && self
                .max_cpu_millicores
                .is_none_or(|millicores| millicores > 0 && millicores <= MAX_CPU_MILLICORES)
    }

    /// The bound this declaration reduces to for the processes a dispatch
    /// spawns, or the declared limit the Host cannot bind. Every limit in this
    /// set is enforced; a limit outside it refuses the execution that would
    /// have run unbounded.
    pub fn process_bound(&self) -> Result<ProcessBound, UnboundedLimit> {
        if self.max_cpu_millicores.is_some() {
            return Err(UnboundedLimit::CpuRate);
        }
        Ok(ProcessBound {
            address_space_bytes: self.max_memory_bytes,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn limits() -> ResourceLimits {
        ResourceLimits {
            max_total_tokens: 100_000,
            max_wall_time_ms: 60_000,
            max_concurrency: 1,
            max_memory_bytes: 1 << 30,
            max_cpu_millicores: None,
        }
    }

    #[test]
    fn contract_bounds_are_the_declaration_ceiling() {
        assert!(limits().validate());
        let broken = |name: &str, f: &dyn Fn(&mut ResourceLimits)| {
            let mut value = limits();
            f(&mut value);
            assert!(!value.validate(), "{name}");
        };
        broken("tokens", &|l| l.max_total_tokens = MAX_TOTAL_TOKENS + 1);
        broken("wall", &|l| l.max_wall_time_ms = 0);
        broken("concurrency", &|l| l.max_concurrency = MAX_CONCURRENCY + 1);
        broken("memory", &|l| l.max_memory_bytes = 0);
        broken("memory ceiling", &|l| {
            l.max_memory_bytes = MAX_MEMORY_BYTES + 1
        });
        broken("cpu", &|l| l.max_cpu_millicores = Some(0));
        broken("cpu ceiling", &|l| {
            l.max_cpu_millicores = Some(MAX_CPU_MILLICORES + 1)
        });
    }

    #[test]
    fn the_process_bound_is_what_the_host_can_enforce_and_no_more() {
        // A memory ceiling is a bound a Host can apply, byte for byte.
        assert_eq!(
            limits().process_bound(),
            Ok(ProcessBound {
                address_space_bytes: 1 << 30
            })
        );
        // A CPU rate is not: it refuses, naming the declared field rather than
        // substituting a different limit.
        let mut demanding = limits();
        demanding.max_cpu_millicores = Some(1_000);
        assert_eq!(demanding.process_bound(), Err(UnboundedLimit::CpuRate));
        assert_eq!(UnboundedLimit::CpuRate.field(), "max_cpu_millicores");
    }
}
