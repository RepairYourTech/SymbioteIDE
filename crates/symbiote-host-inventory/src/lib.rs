//! Bounded Host observations, not schedulable reservations or execution authority.
//! Callers authenticate observation provenance; deserialization cannot authenticate a Host.
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use symbiote_domain::{CommandId, HostId, Timestamp};
pub use symbiote_runtime_discovery::Fact;
pub mod linux;
pub const PULSE_VERSION: u32 = 1;
pub const MAX_TTL_MS: u64 = 30_000;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ResourceObservation {
    pub logical_cpu_count: Fact<u32>,
    pub physical_memory_total_bytes: Fact<u64>,
    pub physical_memory_available_bytes: Fact<u64>,
    pub effective_cpu_millicores: Fact<u64>,
    pub effective_memory_limit_bytes: Fact<u64>,
    pub effective_memory_available_bytes: Fact<u64>,
}
impl Default for ResourceObservation {
    fn default() -> Self {
        Self {
            logical_cpu_count: Fact::Unknown,
            physical_memory_total_bytes: Fact::Unknown,
            physical_memory_available_bytes: Fact::Unknown,
            effective_cpu_millicores: Fact::Unknown,
            effective_memory_limit_bytes: Fact::Unknown,
            effective_memory_available_bytes: Fact::Unknown,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TelemetryMode {
    Enabled,
    Disabled,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PulseSource {
    OperatingSystem,
    UnsupportedPlatform,
    TelemetryDisabled,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PulseProvenance {
    pub source: PulseSource,
    pub probe_version: String,
}
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityKind {
    Gpu,
    Disk,
    Container,
    Toolchain,
    Signing,
    Browser,
    RuntimeInventory,
    LocalModels,
    InferenceServer,
    ProjectService,
    NetworkClass,
    Git,
    Pty,
    Database,
    Tunnel,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityHealth {
    Healthy,
    Degraded,
    Busy,
    RateLimited,
    Unavailable,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityAuthentication {
    NotRequired,
    Authenticated,
    Required,
    Expired,
    RateLimited,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CapabilitySource {
    PassiveObservation,
    RuntimeDiscovery,
    HealthCheck,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CapabilityObservation {
    pub kind: CapabilityKind,
    pub installed: Fact<bool>,
    pub authentication: Fact<CapabilityAuthentication>,
    pub health: Fact<CapabilityHealth>,
    pub version: Fact<String>,
    pub observed_at: Timestamp,
    pub expires_at: Timestamp,
    pub source: CapabilitySource,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(try_from = "HostPulseWire")]
pub struct HostPulse {
    pub schema_version: u32,
    pub host_id: HostId,
    pub observation_id: CommandId,
    pub observed_at: Timestamp,
    pub expires_at: Timestamp,
    pub telemetry: TelemetryMode,
    pub os: Fact<String>,
    pub architecture: Fact<String>,
    pub resources: ResourceObservation,
    pub capabilities: Vec<CapabilityObservation>,
    pub provenance: PulseProvenance,
    pub probe_failures: Vec<linux::ProbeFailure>,
}
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct HostPulseWire {
    schema_version: u32,
    host_id: HostId,
    observation_id: CommandId,
    observed_at: Timestamp,
    expires_at: Timestamp,
    telemetry: TelemetryMode,
    os: Fact<String>,
    architecture: Fact<String>,
    resources: ResourceObservation,
    capabilities: Vec<CapabilityObservation>,
    provenance: PulseProvenance,
    probe_failures: Vec<linux::ProbeFailure>,
}
impl TryFrom<HostPulseWire> for HostPulse {
    type Error = PulseError;
    fn try_from(w: HostPulseWire) -> Result<Self, Self::Error> {
        let pulse = Self {
            schema_version: w.schema_version,
            host_id: w.host_id,
            observation_id: w.observation_id,
            observed_at: w.observed_at,
            expires_at: w.expires_at,
            telemetry: w.telemetry,
            os: w.os,
            architecture: w.architecture,
            resources: w.resources,
            capabilities: w.capabilities,
            provenance: w.provenance,
            probe_failures: w.probe_failures,
        };
        pulse.validate()?;
        Ok(pulse)
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CapabilityRequirement {
    pub kind: CapabilityKind,
    pub version: Option<String>,
    pub authentication_required: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PulseRequirements {
    pub host_id: HostId,
    pub minimum_cpu_millicores: Option<u64>,
    pub minimum_available_memory_bytes: Option<u64>,
    pub capabilities: Vec<CapabilityRequirement>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PulseError {
    Invalid,
    UnsupportedVersion,
    Capacity,
    HostMismatch,
    Stale,
    Disabled,
    Unknown,
    Insufficient,
    CapabilityUnavailable,
}
impl std::fmt::Display for PulseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Host Pulse rejected: {self:?}")
    }
}
impl std::error::Error for PulseError {}
fn text_valid(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 128
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-/ ".contains(&b))
}
fn time_valid(start: Timestamp, end: Timestamp) -> bool {
    end.0 > start.0 && end.0 - start.0 <= MAX_TTL_MS
}
fn pair_valid(available: &Fact<u64>, total: &Fact<u64>) -> bool {
    !matches!((available,total),(Fact::Known(a),Fact::Known(t)) if a>t)
}
impl HostPulse {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        host_id: HostId,
        observation_id: CommandId,
        observed_at: Timestamp,
        expires_at: Timestamp,
        telemetry: TelemetryMode,
        os: Fact<String>,
        architecture: Fact<String>,
        resources: ResourceObservation,
        provenance: PulseProvenance,
    ) -> Result<Self, PulseError> {
        let value = Self {
            schema_version: PULSE_VERSION,
            host_id,
            observation_id,
            observed_at,
            expires_at,
            telemetry,
            os,
            architecture,
            resources,
            capabilities: Vec::new(),
            provenance,
            probe_failures: Vec::new(),
        };
        value.validate()?;
        Ok(value)
    }
    pub fn from_observation(
        host_id: HostId,
        observation_id: CommandId,
        mut observation: linux::LinuxObservation,
    ) -> Result<Self, PulseError> {
        let expires_at = Timestamp(
            observation
                .observed_at
                .0
                .checked_add(5_000)
                .ok_or(PulseError::Invalid)?,
        );
        let unsupported = observation
            .failures
            .iter()
            .any(|f| f.reason == linux::ProbeError::UnsupportedPlatform);
        let source = if unsupported || observation.os == Fact::Unknown {
            observation.os = Fact::Unknown;
            observation.architecture = Fact::Unknown;
            PulseSource::UnsupportedPlatform
        } else {
            PulseSource::OperatingSystem
        };
        let mut pulse = Self::new(
            host_id,
            observation_id,
            observation.observed_at,
            expires_at,
            TelemetryMode::Enabled,
            observation.os,
            observation.architecture,
            observation.resources,
            PulseProvenance {
                source,
                probe_version: "passive-linux-v1".into(),
            },
        )?;
        pulse.probe_failures = observation.failures;
        pulse.validate()?;
        Ok(pulse)
    }
    pub fn telemetry_disabled(
        host_id: HostId,
        observation_id: CommandId,
        at: Timestamp,
    ) -> Result<Self, PulseError> {
        Self::disabled(
            host_id,
            observation_id,
            at,
            Timestamp(at.0.checked_add(5_000).ok_or(PulseError::Invalid)?),
        )
    }
    pub fn disabled(
        host_id: HostId,
        observation_id: CommandId,
        observed_at: Timestamp,
        expires_at: Timestamp,
    ) -> Result<Self, PulseError> {
        Self::new(
            host_id,
            observation_id,
            observed_at,
            expires_at,
            TelemetryMode::Disabled,
            Fact::Unknown,
            Fact::Unknown,
            ResourceObservation::default(),
            PulseProvenance {
                source: PulseSource::TelemetryDisabled,
                probe_version: "host-pulse-v1".into(),
            },
        )
    }
    pub fn validate(&self) -> Result<(), PulseError> {
        if self.schema_version != PULSE_VERSION {
            return Err(PulseError::UnsupportedVersion);
        }
        if !time_valid(self.observed_at, self.expires_at)
            || !text_valid(&self.provenance.probe_version)
        {
            return Err(PulseError::Invalid);
        }
        if self.capabilities.len() > 32 {
            return Err(PulseError::Capacity);
        }
        if self.probe_failures.len() > 2 {
            return Err(PulseError::Capacity);
        }
        for (i, failure) in self.probe_failures.iter().enumerate() {
            if self.probe_failures[..i]
                .iter()
                .any(|previous| previous.resource == failure.resource)
            {
                return Err(PulseError::Invalid);
            }
            match failure.resource {
                linux::ProbeResource::PhysicalMemory
                    if self.resources.physical_memory_total_bytes != Fact::Unknown
                        || self.resources.physical_memory_available_bytes != Fact::Unknown =>
                {
                    return Err(PulseError::Invalid);
                }
                linux::ProbeResource::LogicalCpuCount
                    if self.resources.logical_cpu_count != Fact::Unknown =>
                {
                    return Err(PulseError::Invalid);
                }
                _ => {}
            }
        }
        for value in [&self.os, &self.architecture] {
            if matches!(value,Fact::Known(s) if !text_valid(s)) {
                return Err(PulseError::Invalid);
            }
        }
        let r = &self.resources;
        if matches!(r.logical_cpu_count, Fact::Known(0))
            || matches!(r.physical_memory_total_bytes, Fact::Known(0))
            || !pair_valid(
                &r.physical_memory_available_bytes,
                &r.physical_memory_total_bytes,
            )
            || !pair_valid(
                &r.effective_memory_available_bytes,
                &r.effective_memory_limit_bytes,
            )
        {
            return Err(PulseError::Invalid);
        }
        if (self.telemetry == TelemetryMode::Disabled
            || self.provenance.source != PulseSource::OperatingSystem)
            && (self.os != Fact::Unknown
                || self.architecture != Fact::Unknown
                || *r != ResourceObservation::default()
                || !self.capabilities.is_empty())
        {
            return Err(PulseError::Invalid);
        }
        if (self.telemetry == TelemetryMode::Disabled)
            != (self.provenance.source == PulseSource::TelemetryDisabled)
        {
            return Err(PulseError::Invalid);
        }
        if self.telemetry == TelemetryMode::Disabled && !self.probe_failures.is_empty() {
            return Err(PulseError::Invalid);
        }
        let mut kinds = BTreeSet::new();
        for c in &self.capabilities {
            if !kinds.insert(c.kind)
                || !time_valid(c.observed_at, c.expires_at)
                || c.observed_at.0 > self.observed_at.0
                || c.expires_at.0 > self.expires_at.0
                || matches!(&c.version,Fact::Known(v) if !text_valid(v))
            {
                return Err(PulseError::Invalid);
            }
        }
        Ok(())
    }
    /// Observation matching only: does not reserve resources or prove task execution readiness.
    pub fn qualify(
        &self,
        requirements: &PulseRequirements,
        now: Timestamp,
    ) -> Result<(), PulseError> {
        self.validate()?;
        if self.host_id != requirements.host_id {
            return Err(PulseError::HostMismatch);
        }
        if now.0 < self.observed_at.0 || now.0 >= self.expires_at.0 {
            return Err(PulseError::Stale);
        }
        if self.telemetry == TelemetryMode::Disabled {
            return Err(PulseError::Disabled);
        }
        if self.provenance.source != PulseSource::OperatingSystem
            || self.os == Fact::Unknown
            || self.architecture == Fact::Unknown
        {
            return Err(PulseError::Unknown);
        }
        for (wanted, observed) in [
            (
                requirements.minimum_cpu_millicores,
                &self.resources.effective_cpu_millicores,
            ),
            (
                requirements.minimum_available_memory_bytes,
                &self.resources.effective_memory_available_bytes,
            ),
        ] {
            if let Some(minimum) = wanted {
                match observed {
                    Fact::Unknown => return Err(PulseError::Unknown),
                    Fact::Known(actual) if *actual < minimum => {
                        return Err(PulseError::Insufficient);
                    }
                    _ => {}
                }
            }
        }
        if requirements.capabilities.len() > 32 {
            return Err(PulseError::Capacity);
        }
        for requirement in &requirements.capabilities {
            let c = self
                .capabilities
                .iter()
                .find(|c| c.kind == requirement.kind)
                .ok_or(PulseError::Unknown)?;
            if now.0 < c.observed_at.0 || now.0 >= c.expires_at.0 {
                return Err(PulseError::Stale);
            }
            if c.installed == Fact::Unknown || c.health == Fact::Unknown {
                return Err(PulseError::Unknown);
            }
            if c.installed != Fact::Known(true)
                || c.health != Fact::Known(CapabilityHealth::Healthy)
            {
                return Err(PulseError::CapabilityUnavailable);
            }
            if let Some(version) = &requirement.version {
                if c.version == Fact::Unknown {
                    return Err(PulseError::Unknown);
                }
                if c.version != Fact::Known(version.clone()) {
                    return Err(PulseError::CapabilityUnavailable);
                }
            }
            if requirement.authentication_required {
                match c.authentication {
                    Fact::Unknown => return Err(PulseError::Unknown),
                    Fact::Known(
                        CapabilityAuthentication::Authenticated
                        | CapabilityAuthentication::NotRequired,
                    ) => {}
                    _ => return Err(PulseError::CapabilityUnavailable),
                }
            }
        }
        Ok(())
    }
    pub fn parse(input: &str) -> Result<Self, PulseError> {
        if input.len() > 64 * 1024 {
            return Err(PulseError::Capacity);
        }
        let value: Self = serde_json::from_str(input).map_err(|_| PulseError::Invalid)?;
        value.validate()?;
        Ok(value)
    }
}
