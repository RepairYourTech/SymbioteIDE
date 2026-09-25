use std::{
    fs::File,
    io::Read,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use symbiote_domain::{CommandId, HostId, Timestamp};
use symbiote_host_inventory::{HostPulse, linux};
use symbiote_protocol::{ErrorCode, ProtocolError};

pub(crate) struct InventoryService {
    host_id: HostId,
    enabled: bool,
    nonce: String,
    sequence: u64,
    cached: Option<(Instant, HostPulse)>,
    /// The [runtime inventory](../../../docs/contracts/runtime-discovery.md) this Host serves.
    /// It is held here rather than threaded through every dispatch signature because it is
    /// this Host's own read of its own state directory, exactly as the pulse is: the daemon
    /// resolves both from the state directory once, at startup.
    published: crate::runtime_inventory::Published,
}
impl InventoryService {
    pub(crate) fn new(
        host_id: HostId,
        enabled: bool,
        published: crate::runtime_inventory::Published,
    ) -> std::io::Result<Self> {
        let mut bytes = [0u8; 16];
        File::open("/dev/urandom")?.read_exact(&mut bytes)?;
        Ok(Self {
            host_id,
            enabled,
            nonce: bytes.iter().map(|b| format!("{b:02x}")).collect(),
            sequence: 0,
            cached: None,
            published,
        })
    }
    pub(crate) fn host_id(&self) -> &HostId {
        &self.host_id
    }
    /// The re-validated inventory, or the named reason this Host will not serve one. The
    /// pulse is cached for a second because it is a live measurement; this is not cached at
    /// all, because the file is the durable record and a read after a republish is the new
    /// document.
    pub(crate) fn runtime_inventory(
        &self,
    ) -> Result<symbiote_runtime_discovery::Inventory, crate::runtime_inventory::Refusal> {
        self.published.read()
    }
    pub(crate) fn pulse(&mut self) -> Result<HostPulse, ProtocolError> {
        let started = Instant::now();
        if let Some((at, pulse)) = &self.cached {
            if started.duration_since(*at) < Duration::from_secs(1) {
                return Ok(pulse.clone());
            }
        }
        let unavailable = || ProtocolError::new(ErrorCode::Unavailable);
        let at = Timestamp(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|_| unavailable())?
                .as_millis()
                .try_into()
                .map_err(|_| unavailable())?,
        );
        self.sequence = self.sequence.checked_add(1).ok_or_else(unavailable)?;
        let id = CommandId::new(format!("pulse_{}_{}", self.nonce, self.sequence))
            .map_err(|_| unavailable())?;
        let pulse = if self.enabled {
            HostPulse::from_observation(self.host_id.clone(), id, linux::probe(at))
        } else {
            HostPulse::telemetry_disabled(self.host_id.clone(), id, at)
        }
        .map_err(|_| unavailable())?;
        pulse.validate().map_err(|_| unavailable())?;
        self.cached = Some((started, pulse.clone()));
        Ok(pulse)
    }
}
