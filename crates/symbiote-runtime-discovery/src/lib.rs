//! Host-local runtime inventory. Discovery never grants activation, credential,
//! installation or billing authority. Caller-supplied observations require Host provenance.
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};
use symbiote_domain::*;
pub mod codex;
pub const DISCOVERY_VERSION: u32 = 1;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "status",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum Fact<T> {
    Unknown,
    Known(T),
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RuntimeInstanceIntent {
    pub profile_id: RuntimeProfileId,
    pub installation_id: InstallationId,
    pub host_id: HostId,
    pub runtime_kind: RuntimeKind,
    pub adapter_id: AgentRuntimeAdapterId,
    /// Display label only; never used for eligibility or adapter dispatch.
    pub instance_name: String,
    /// Symbolic identity, never an absolute HOME, credential or native configuration path.
    pub config_identity: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct InterfaceIdentity {
    pub name: String,
    pub version: String,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Health {
    Reachable,
    Unavailable,
    RateLimited,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AuthMode {
    None,
    ApiKey,
    ChatGpt,
    Other,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AuthState {
    NotRequired,
    Required,
    Authenticated,
    Expired,
    RateLimited,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Authentication {
    pub mode: AuthMode,
    pub state: AuthState,
    /// Host-generated opaque reference only. Never copy an email, token or account payload here.
    pub account_ref: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ModelListing {
    /// External model identifiers remain separate from canonical Model identity.
    pub upstream_model: String,
    pub canonical_model_id: Option<ModelId>,
    pub context_tokens: Fact<u64>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MethodAvailability {
    Unknown,
    Available,
    Unsupported,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct IsolationEvidence {
    pub verified_by: HostId,
    pub evidence_id: EvidenceId,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DiscoveryFacts {
    pub health: Fact<Health>,
    pub authentication: Fact<Authentication>,
    pub models: Fact<Vec<ModelListing>>,
    #[serde(deserialize_with = "methods_decode")]
    pub methods: BTreeMap<String, MethodAvailability>,
    pub isolation: Fact<IsolationEvidence>,
}
fn methods_decode<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<BTreeMap<String, MethodAvailability>, D::Error> {
    struct Methods;
    impl<'de> serde::de::Visitor<'de> for Methods {
        type Value = BTreeMap<String, MethodAvailability>;
        fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("at most 64 unique method names")
        }
        fn visit_map<A: serde::de::MapAccess<'de>>(
            self,
            mut map: A,
        ) -> Result<Self::Value, A::Error> {
            let mut methods = BTreeMap::new();
            while let Some((name, state)) = map.next_entry()? {
                if methods.len() == 64 || methods.insert(name, state).is_some() {
                    return Err(serde::de::Error::custom("invalid methods"));
                }
            }
            Ok(methods)
        }
    }
    deserializer.deserialize_map(Methods)
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ObservationSource {
    SandboxedProbe,
    TrustedHostProbe,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProbeProvenance {
    pub probe_id: CommandId,
    pub source: ObservationSource,
    pub adapter_revision: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct InstallationObservation {
    pub config_identity: String,
    pub profile_id: RuntimeProfileId,
    pub installation_id: InstallationId,
    pub host_id: HostId,
    pub runtime_kind: RuntimeKind,
    pub adapter_id: AgentRuntimeAdapterId,
    pub version: Fact<String>,
    pub interface: Fact<InterfaceIdentity>,
    pub facts: DiscoveryFacts,
    pub observed_at: Timestamp,
    pub expires_at: Timestamp,
    pub provenance: ProbeProvenance,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(try_from = "RecordWire")]
pub struct DiscoveryRecord {
    pub schema_version: u32,
    pub intent: RuntimeInstanceIntent,
    pub observation: InstallationObservation,
}
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct RecordWire {
    schema_version: u32,
    intent: RuntimeInstanceIntent,
    observation: InstallationObservation,
}
impl TryFrom<RecordWire> for DiscoveryRecord {
    type Error = DiscoveryError;
    fn try_from(w: RecordWire) -> Result<Self, Self::Error> {
        let value = Self {
            schema_version: w.schema_version,
            intent: w.intent,
            observation: w.observation,
        };
        value.validate()?;
        Ok(value)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DiscoveryError {
    InvalidRecord,
    ResourceLimit,
    UnsupportedVersion,
    NotFound,
    IdentityMismatch,
    VersionMismatch,
    Stale,
    UnknownMandatoryFact,
    Unhealthy,
    AuthenticationRequired,
    AuthenticationExpired,
    RateLimited,
    UnsupportedMethod,
    ModelNotListed,
    InsufficientContext,
    IsolationUnverified,
}
impl fmt::Display for DiscoveryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for DiscoveryError {}
fn symbol(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 128
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
}
fn version(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 128
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-+".contains(&b))
}
fn method(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 128
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"/_-.".contains(&b))
}
fn model(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 128
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"/_-.:".contains(&b))
}
struct Size(usize, usize);
impl std::io::Write for Size {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if bytes.len() > self.1 - self.0 {
            return Err(std::io::Error::other("discovery limit"));
        }
        self.0 += bytes.len();
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
impl DiscoveryRecord {
    pub fn validate(&self) -> Result<(), DiscoveryError> {
        if self.schema_version != DISCOVERY_VERSION {
            return Err(DiscoveryError::UnsupportedVersion);
        }
        let i = &self.intent;
        let o = &self.observation;
        if i.instance_name.trim().is_empty()
            || i.instance_name.len() > 128
            || i.instance_name.chars().any(char::is_control)
            || !symbol(&i.config_identity)
            || !symbol(&o.config_identity)
            || !version(&o.provenance.adapter_revision)
        {
            return Err(DiscoveryError::InvalidRecord);
        }
        if o.expires_at.0 <= o.observed_at.0 || o.expires_at.0 - o.observed_at.0 > 86_400_000 {
            return Err(DiscoveryError::InvalidRecord);
        }
        if let Fact::Known(v) = &o.version {
            if !version(v) {
                return Err(DiscoveryError::InvalidRecord);
            }
        }
        if let Fact::Known(interface) = &o.interface {
            if !symbol(&interface.name) || !version(&interface.version) {
                return Err(DiscoveryError::InvalidRecord);
            }
        }
        if o.facts.methods.len() > 64 {
            return Err(DiscoveryError::ResourceLimit);
        }
        if o.facts.methods.keys().any(|s| !method(s)) {
            return Err(DiscoveryError::InvalidRecord);
        }
        if let Fact::Known(auth) = &o.facts.authentication {
            if auth
                .account_ref
                .as_ref()
                .is_some_and(|s| !symbol(s) || !s.starts_with("account_"))
                || ((auth.state == AuthState::Authenticated) && (auth.mode == AuthMode::None))
                || ((auth.state == AuthState::Required || auth.state == AuthState::NotRequired)
                    && auth.account_ref.is_some())
            {
                return Err(DiscoveryError::InvalidRecord);
            }
        }
        if let Fact::Known(models) = &o.facts.models {
            if models.len() > 256 {
                return Err(DiscoveryError::ResourceLimit);
            }
            let mut names = BTreeSet::new();
            let mut mapped = BTreeSet::new();
            for m in models {
                if !model(&m.upstream_model)
                    || !names.insert(&m.upstream_model)
                    || m.canonical_model_id
                        .as_ref()
                        .is_some_and(|id| !mapped.insert(id))
                    || matches!(m.context_tokens, Fact::Known(0))
                {
                    return Err(DiscoveryError::InvalidRecord);
                }
            }
        }
        if let Fact::Known(isolation) = &o.facts.isolation {
            if isolation.verified_by != o.host_id {
                return Err(DiscoveryError::IdentityMismatch);
            }
        }
        serde_json::to_writer(Size(0, 65_536), self).map_err(|_| DiscoveryError::ResourceLimit)?;
        Ok(())
    }
    /// Qualification is an observation match, never permission to launch or bill.
    pub fn qualify(&self, q: &EligibilityQuery, at: Timestamp) -> Result<(), DiscoveryError> {
        self.validate()?;
        q.validate()?;
        let i = &self.intent;
        let o = &self.observation;
        if i.config_identity != q.config_identity
            || o.config_identity != q.config_identity
            || i.host_id != q.host_id
            || o.host_id != q.host_id
            || i.profile_id != q.profile_id
            || o.profile_id != q.profile_id
            || i.installation_id != q.installation_id
            || o.installation_id != q.installation_id
            || i.runtime_kind != q.runtime_kind
            || o.runtime_kind != q.runtime_kind
            || i.adapter_id != q.adapter_id
            || o.adapter_id != q.adapter_id
        {
            return Err(DiscoveryError::IdentityMismatch);
        }
        if at.0 < o.observed_at.0 || at.0 >= o.expires_at.0 {
            return Err(DiscoveryError::Stale);
        }
        match (&o.version, &o.interface) {
            (Fact::Known(v), Fact::Known(interface))
                if v == &q.version && interface == &q.interface => {}
            (Fact::Unknown, _) | (_, Fact::Unknown) => {
                return Err(DiscoveryError::UnknownMandatoryFact);
            }
            _ => return Err(DiscoveryError::VersionMismatch),
        }
        match o.facts.health {
            Fact::Known(Health::Reachable) => {}
            Fact::Known(Health::RateLimited) => return Err(DiscoveryError::RateLimited),
            Fact::Known(Health::Unavailable) => return Err(DiscoveryError::Unhealthy),
            Fact::Unknown => return Err(DiscoveryError::UnknownMandatoryFact),
        }
        for name in &q.required_methods {
            match o.facts.methods.get(name) {
                Some(MethodAvailability::Available) => {}
                Some(MethodAvailability::Unsupported) => {
                    return Err(DiscoveryError::UnsupportedMethod);
                }
                _ => return Err(DiscoveryError::UnknownMandatoryFact),
            }
        }
        if q.requires_authenticated {
            match &o.facts.authentication {
                Fact::Known(Authentication {
                    state: AuthState::Authenticated,
                    ..
                }) => {}
                Fact::Known(Authentication {
                    state: AuthState::Expired,
                    ..
                }) => return Err(DiscoveryError::AuthenticationExpired),
                Fact::Known(Authentication {
                    state: AuthState::RateLimited,
                    ..
                }) => return Err(DiscoveryError::RateLimited),
                Fact::Unknown => return Err(DiscoveryError::UnknownMandatoryFact),
                _ => return Err(DiscoveryError::AuthenticationRequired),
            }
        }
        if let Some(name) = &q.upstream_model {
            let Fact::Known(models) = &o.facts.models else {
                return Err(DiscoveryError::UnknownMandatoryFact);
            };
            let listed = models
                .iter()
                .find(|m| &m.upstream_model == name)
                .ok_or(DiscoveryError::ModelNotListed)?;
            if let Some(min) = q.minimum_context_tokens {
                match listed.context_tokens {
                    Fact::Known(tokens) if tokens >= min => {}
                    Fact::Known(_) => return Err(DiscoveryError::InsufficientContext),
                    Fact::Unknown => return Err(DiscoveryError::UnknownMandatoryFact),
                }
            }
        }
        if q.requires_isolation && matches!(o.facts.isolation, Fact::Unknown) {
            return Err(DiscoveryError::IsolationUnverified);
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EligibilityQuery {
    pub config_identity: String,
    pub profile_id: RuntimeProfileId,
    pub installation_id: InstallationId,
    pub host_id: HostId,
    pub runtime_kind: RuntimeKind,
    pub adapter_id: AgentRuntimeAdapterId,
    pub version: String,
    pub interface: InterfaceIdentity,
    pub required_methods: BTreeSet<String>,
    pub requires_authenticated: bool,
    pub upstream_model: Option<String>,
    pub minimum_context_tokens: Option<u64>,
    pub requires_isolation: bool,
}
impl EligibilityQuery {
    pub fn validate(&self) -> Result<(), DiscoveryError> {
        if !symbol(&self.config_identity)
            || !version(&self.version)
            || !symbol(&self.interface.name)
            || !version(&self.interface.version)
            || self.required_methods.len() > 64
            || self.required_methods.iter().any(|m| !method(m))
            || self.upstream_model.as_ref().is_some_and(|s| !model(s))
            || self
                .minimum_context_tokens
                .is_some_and(|n| n == 0 || self.upstream_model.is_none())
        {
            return Err(DiscoveryError::InvalidRecord);
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(try_from = "InventoryWire")]
pub struct Inventory {
    schema_version: u32,
    records: Vec<DiscoveryRecord>,
}
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct InventoryWire {
    schema_version: u32,
    records: Vec<DiscoveryRecord>,
}
impl TryFrom<InventoryWire> for Inventory {
    type Error = DiscoveryError;
    fn try_from(w: InventoryWire) -> Result<Self, Self::Error> {
        if w.schema_version != DISCOVERY_VERSION {
            return Err(DiscoveryError::UnsupportedVersion);
        }
        Self::new(w.records)
    }
}
impl Inventory {
    pub fn new(records: Vec<DiscoveryRecord>) -> Result<Self, DiscoveryError> {
        if records.len() > 256 {
            return Err(DiscoveryError::ResourceLimit);
        }
        let mut identities = BTreeSet::new();
        for record in &records {
            record.validate()?;
            if !identities.insert((&record.intent.host_id, &record.intent.profile_id)) {
                return Err(DiscoveryError::IdentityMismatch);
            }
        }
        let inventory = Self {
            schema_version: DISCOVERY_VERSION,
            records,
        };
        serde_json::to_writer(Size(0, 4_194_304), &inventory)
            .map_err(|_| DiscoveryError::ResourceLimit)?;
        Ok(inventory)
    }
    pub fn parse(input: &str) -> Result<Self, DiscoveryError> {
        if input.len() > 4_194_304 {
            return Err(DiscoveryError::ResourceLimit);
        }
        serde_json::from_str(input).map_err(|_| DiscoveryError::InvalidRecord)
    }
    pub fn records(&self) -> &[DiscoveryRecord] {
        &self.records
    }
    pub fn qualify(&self, q: &EligibilityQuery, at: Timestamp) -> Result<(), DiscoveryError> {
        self.records
            .iter()
            .find(|r| r.intent.host_id == q.host_id && r.intent.profile_id == q.profile_id)
            .ok_or(DiscoveryError::NotFound)?
            .qualify(q, at)
    }
}
