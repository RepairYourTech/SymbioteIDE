//! Passive, bounded Linux observations. Physical memory and online CPU counts
//! do not establish cgroup, affinity, reservation or schedulable capacity.
use crate::{Fact, ResourceObservation};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use symbiote_domain::Timestamp;

pub const MAX_MEMINFO_BYTES: usize = 65_536;
pub const MAX_STAT_BYTES: usize = 1_048_576;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProbeError {
    UnsupportedPlatform,
    Unreadable,
    TooLarge,
    InvalidEncoding,
    Malformed,
    MissingField,
    DuplicateField,
    Overflow,
    Inconsistent,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProbeResource {
    PhysicalMemory,
    LogicalCpuCount,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProbeFailure {
    pub resource: ProbeResource,
    pub reason: ProbeError,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LinuxObservation {
    pub os: Fact<String>,
    pub architecture: Fact<String>,
    pub resources: ResourceObservation,
    pub observed_at: Timestamp,
    pub failures: Vec<ProbeFailure>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PhysicalMemory {
    pub total_bytes: u64,
    /// Kernel estimate from MemAvailable, not an allocation guarantee.
    pub available_bytes: u64,
}

fn number(value: &str) -> Result<u64, ProbeError> {
    if value.is_empty() || !value.bytes().all(|b| b.is_ascii_digit()) {
        return Err(ProbeError::Malformed);
    }
    value.parse().map_err(|_| ProbeError::Overflow)
}

/// Parses only the two documented memory observations; unrelated fields are
/// ignored, including their values. No hostname, path or raw input is retained.
pub fn parse_meminfo(input: &str) -> Result<PhysicalMemory, ProbeError> {
    if input.len() > MAX_MEMINFO_BYTES {
        return Err(ProbeError::TooLarge);
    }
    let mut total = None;
    let mut available = None;
    for line in input.lines() {
        let mut fields = line.split_ascii_whitespace();
        let target = match fields.next() {
            Some("MemTotal:") => &mut total,
            Some("MemAvailable:") => &mut available,
            _ => continue,
        };
        if target.is_some() {
            return Err(ProbeError::DuplicateField);
        }
        let value = number(fields.next().ok_or(ProbeError::Malformed)?)?;
        if fields.next() != Some("kB") || fields.next().is_some() {
            return Err(ProbeError::Malformed);
        }
        *target = Some(value.checked_mul(1024).ok_or(ProbeError::Overflow)?);
    }
    let total_bytes = total.ok_or(ProbeError::MissingField)?;
    let available_bytes = available.ok_or(ProbeError::MissingField)?;
    if total_bytes == 0 || available_bytes > total_bytes {
        return Err(ProbeError::Inconsistent);
    }
    Ok(PhysicalMemory {
        total_bytes,
        available_bytes,
    })
}

/// Counts distinct cpuN rows, not the aggregate cpu row and not process affinity.
/// Validates numeric counters but does not infer utilization from one sample.
pub fn parse_cpu_stat(input: &str) -> Result<u32, ProbeError> {
    if input.len() > MAX_STAT_BYTES {
        return Err(ProbeError::TooLarge);
    }
    let mut cpus = BTreeSet::new();
    let mut aggregate = false;
    for line in input.lines() {
        let mut fields = line.split_ascii_whitespace();
        let Some(label) = fields.next() else { continue };
        let Some(id) = label.strip_prefix("cpu") else {
            continue;
        };
        if id.is_empty() {
            if aggregate {
                return Err(ProbeError::DuplicateField);
            }
            aggregate = true;
        } else {
            let id = number(id)?;
            if id > u64::from(u32::MAX) {
                return Err(ProbeError::Overflow);
            }
            if !cpus.insert(id) {
                return Err(ProbeError::DuplicateField);
            }
        }
        let mut counters = 0;
        for value in fields {
            number(value)?;
            counters += 1;
            if counters > 10 {
                return Err(ProbeError::Malformed);
            }
        }
        if counters < 4 {
            return Err(ProbeError::Malformed);
        }
    }
    if !aggregate || cpus.is_empty() {
        return Err(ProbeError::MissingField);
    }
    u32::try_from(cpus.len()).map_err(|_| ProbeError::Overflow)
}

#[cfg(target_os = "linux")]
fn read_bounded(path: &str, maximum: usize) -> Result<String, ProbeError> {
    use std::io::Read;
    let file = std::fs::File::open(path).map_err(|_| ProbeError::Unreadable)?;
    let mut bytes = Vec::new();
    file.take((maximum + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| ProbeError::Unreadable)?;
    if bytes.len() > maximum {
        return Err(ProbeError::TooLarge);
    }
    String::from_utf8(bytes).map_err(|_| ProbeError::InvalidEncoding)
}

/// Read-only synchronous sample of fixed procfs files. It never launches tools,
/// reads accounts/configuration, probes networks, or guesses effective capacity.
pub fn probe(at: Timestamp) -> LinuxObservation {
    let mut observation = LinuxObservation {
        os: Fact::Known(std::env::consts::OS.into()),
        architecture: Fact::Known(std::env::consts::ARCH.into()),
        resources: ResourceObservation::default(),
        observed_at: at,
        failures: Vec::new(),
    };
    #[cfg(target_os = "linux")]
    let (memory, cpu) = (
        read_bounded("/proc/meminfo", MAX_MEMINFO_BYTES).and_then(|s| parse_meminfo(&s)),
        read_bounded("/proc/stat", MAX_STAT_BYTES).and_then(|s| parse_cpu_stat(&s)),
    );
    #[cfg(not(target_os = "linux"))]
    let (memory, cpu): (Result<PhysicalMemory, ProbeError>, Result<u32, ProbeError>) = (
        Err(ProbeError::UnsupportedPlatform),
        Err(ProbeError::UnsupportedPlatform),
    );
    match memory {
        Ok(memory) => {
            observation.resources.physical_memory_total_bytes = Fact::Known(memory.total_bytes);
            observation.resources.physical_memory_available_bytes =
                Fact::Known(memory.available_bytes);
        }
        Err(reason) => observation.failures.push(ProbeFailure {
            resource: ProbeResource::PhysicalMemory,
            reason,
        }),
    }
    match cpu {
        Ok(count) => observation.resources.logical_cpu_count = Fact::Known(count),
        Err(reason) => observation.failures.push(ProbeFailure {
            resource: ProbeResource::LogicalCpuCount,
            reason,
        }),
    }
    observation
}
