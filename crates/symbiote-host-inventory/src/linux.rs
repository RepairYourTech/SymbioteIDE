//! Passive, bounded Linux observations. Physical memory and online CPU counts
//! do not establish cgroup, affinity, reservation or schedulable capacity; the
//! process's own cgroup establishes what the kernel actually enforces, and
//! nothing here reads accounts, configuration or the network.
use crate::{Fact, ResourceObservation};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use symbiote_domain::Timestamp;

pub const MAX_MEMINFO_BYTES: usize = 65_536;
pub const MAX_STAT_BYTES: usize = 1_048_576;
pub const MAX_CGROUP_BYTES: usize = 4_096;
/// The unified (cgroup v2) mount point. Only the unified hierarchy is observed:
/// a hybrid or v1-only host reports unknown effective facts with a static
/// reason rather than translating a hierarchy whose semantics differ.
pub const CGROUP_ROOT: &str = "/sys/fs/cgroup";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProbeError {
    UnsupportedPlatform,
    UnsupportedHierarchy,
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
    EffectiveMemory,
    EffectiveCpu,
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

/// The unified path of the process's own cgroup, from `/proc/self/cgroup`'s
/// `0::<path>` line. Kernel-owned input is still validated: the path must be
/// absolute, bounded, free of control characters and free of `..` segments, so
/// it can never address a file outside the cgroup mount.
pub fn parse_cgroup_path(input: &str) -> Result<String, ProbeError> {
    if input.len() > MAX_CGROUP_BYTES {
        return Err(ProbeError::TooLarge);
    }
    let mut path = None;
    for line in input.lines() {
        let Some(rest) = line.strip_prefix("0::") else {
            continue;
        };
        if path.is_some() {
            return Err(ProbeError::DuplicateField);
        }
        path = Some(rest.trim());
    }
    let path = path.ok_or(ProbeError::UnsupportedHierarchy)?;
    if !path.starts_with('/')
        || path.len() > 1_024
        || path.chars().any(char::is_control)
        || path.split('/').any(|segment| segment == "..")
    {
        return Err(ProbeError::Malformed);
    }
    Ok(path.to_owned())
}

/// `memory.max`: a byte ceiling, or `max` when the kernel enforces none.
pub fn parse_memory_max(input: &str) -> Result<Option<u64>, ProbeError> {
    let value = input.trim();
    if value == "max" {
        return Ok(None);
    }
    number(value).map(Some)
}

/// `memory.current`: the cgroup's current charge, in bytes.
pub fn parse_memory_current(input: &str) -> Result<u64, ProbeError> {
    number(input.trim())
}

/// `cpu.max`: `<quota> <period>` in microseconds, or `max <period>` when no
/// quota is enforced. A quota is reported in millicores, rounded down, so a
/// fractional quota never rounds up to capacity it does not have.
pub fn parse_cpu_max(input: &str) -> Result<Option<u64>, ProbeError> {
    let mut fields = input.split_ascii_whitespace();
    let quota = fields.next().ok_or(ProbeError::Malformed)?;
    let period = fields.next().ok_or(ProbeError::Malformed)?;
    if fields.next().is_some() {
        return Err(ProbeError::Malformed);
    }
    let period = number(period)?;
    if period == 0 {
        return Err(ProbeError::Inconsistent);
    }
    if quota == "max" {
        return Ok(None);
    }
    number(quota)?
        .checked_mul(1_000)
        .map(|millicores| Some(millicores / period))
        .ok_or(ProbeError::Overflow)
}

/// What the process's cgroup establishes about memory: the enforced ceiling (if
/// any), the availability under it, and the static reason availability is
/// unknown. A ceiling reported as `max` means the cgroup enforces none, so the
/// machine's own observed availability is the bound; a finite ceiling never
/// claims more headroom than the machine reports available.
pub fn derive_memory(
    limit: Result<Option<u64>, ProbeError>,
    current: Result<u64, ProbeError>,
    physical_available: &Fact<u64>,
) -> (Fact<u64>, Fact<u64>, Option<ProbeError>) {
    match limit {
        Ok(None) => (Fact::Unknown, physical_available.clone(), None),
        Err(reason) => (Fact::Unknown, Fact::Unknown, Some(reason)),
        Ok(Some(limit)) => match current {
            Ok(current) => {
                let headroom = limit.saturating_sub(current);
                let available = match physical_available {
                    Fact::Known(physical) => Fact::Known(headroom.min(*physical)),
                    Fact::Unknown => Fact::Known(headroom),
                };
                (Fact::Known(limit), available, None)
            }
            Err(reason) => (Fact::Known(limit), Fact::Unknown, Some(reason)),
        },
    }
}

/// What the process's cgroup establishes about CPU. A quota is a real
/// measurement; without one the effective CPU stays unknown — online CPU
/// counts establish neither affinity nor a reservation — which is an absence of
/// evidence, never a claim of capacity.
pub fn derive_cpu(quota: Result<Option<u64>, ProbeError>) -> (Fact<u64>, Option<ProbeError>) {
    match quota {
        Ok(Some(millicores)) => (Fact::Known(millicores), None),
        Ok(None) => (Fact::Unknown, None),
        Err(reason) => (Fact::Unknown, Some(reason)),
    }
}

/// The process's own cgroup, read file by file so one unreadable file reports
/// its own reason instead of discarding facts another file establishes.
#[cfg(target_os = "linux")]
struct CgroupFiles {
    memory_limit: Result<Option<u64>, ProbeError>,
    memory_current: Result<u64, ProbeError>,
    cpu_millicores: Result<Option<u64>, ProbeError>,
}

#[cfg(target_os = "linux")]
impl CgroupFiles {
    fn read(path: &str) -> Self {
        let file =
            |name: &str| read_bounded(&format!("{CGROUP_ROOT}{path}/{name}"), MAX_CGROUP_BYTES);
        Self {
            memory_limit: file("memory.max").and_then(|text| parse_memory_max(&text)),
            memory_current: file("memory.current").and_then(|text| parse_memory_current(&text)),
            cpu_millicores: file("cpu.max").and_then(|text| parse_cpu_max(&text)),
        }
    }
    fn failed(reason: ProbeError) -> Self {
        Self {
            memory_limit: Err(reason),
            memory_current: Err(reason),
            cpu_millicores: Err(reason),
        }
    }
}

#[cfg(target_os = "linux")]
fn cgroup_path() -> Result<String, ProbeError> {
    read_bounded("/proc/self/cgroup", MAX_CGROUP_BYTES).and_then(|text| parse_cgroup_path(&text))
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

/// Read-only synchronous sample of fixed procfs and cgroup files. It never
/// launches tools, reads accounts/configuration, or probes networks: what the
/// process's own cgroup enforces is read, and what it does not enforce stays
/// unknown rather than being guessed from the machine's totals.
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
    #[cfg(target_os = "linux")]
    let CgroupFiles {
        memory_limit,
        memory_current,
        cpu_millicores: cpu_quota,
    } = cgroup_path()
        .map(|path| CgroupFiles::read(&path))
        .unwrap_or_else(CgroupFiles::failed);
    #[cfg(not(target_os = "linux"))]
    let (memory_limit, memory_current, cpu_quota): (
        Result<Option<u64>, ProbeError>,
        Result<u64, ProbeError>,
        Result<Option<u64>, ProbeError>,
    ) = (
        Err(ProbeError::UnsupportedPlatform),
        Err(ProbeError::UnsupportedPlatform),
        Err(ProbeError::UnsupportedPlatform),
    );
    let (limit, available, memory_failure) = derive_memory(
        memory_limit,
        memory_current,
        &observation.resources.physical_memory_available_bytes,
    );
    observation.resources.effective_memory_limit_bytes = limit;
    observation.resources.effective_memory_available_bytes = available;
    if let Some(reason) = memory_failure {
        observation.failures.push(ProbeFailure {
            resource: ProbeResource::EffectiveMemory,
            reason,
        });
    }
    let (millicores, cpu_failure) = derive_cpu(cpu_quota);
    observation.resources.effective_cpu_millicores = millicores;
    if let Some(reason) = cpu_failure {
        observation.failures.push(ProbeFailure {
            resource: ProbeResource::EffectiveCpu,
            reason,
        });
    }
    observation
}
