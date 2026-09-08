//! Versioned staffing intent around the canonical WorkforceBinding identity.
//! Structural validation grants neither credential access nor execution authority.
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Write,
};
use symbiote_domain::*;
pub const BINDING_VERSION: u32 = 1;
pub const MAX_BINDING_BYTES: usize = 32 * 1024;
mod readiness;
pub use readiness::*;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PolicyReference {
    pub id: String,
    pub revision: Revision,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RootEffort {
    Low,
    Medium,
    High,
    Max,
    Ultra,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum LocalChildPolicy {
    Disabled {},
}
impl Default for LocalChildPolicy {
    fn default() -> Self {
        Self::Disabled {}
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ResourceLimits {
    pub max_total_tokens: u64,
    pub max_wall_time_ms: u64,
    pub max_concurrency: u16,
    pub max_memory_bytes: u64,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StaffingCandidate {
    pub profile: RuntimeProfile,
    pub config_identity: String,
    pub access: AccessSnapshot,
    pub tools: BTreeSet<String>,
    pub skills: BTreeSet<String>,
    pub secret_scopes: BTreeSet<String>,
    pub context: ContextPolicy,
    pub limits: ResourceLimits,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkforcePolicies {
    pub environment: PolicyReference,
    pub worktree: PolicyReference,
    pub verification: PolicyReference,
    pub documentation: PolicyReference,
    pub artifacts: PolicyReference,
    pub mcp: PolicyReference,
    pub escalation: PolicyReference,
    pub root_effort: RootEffort,
    #[serde(default)]
    pub local_children: LocalChildPolicy,
    pub fallback_consent: FallbackConsent,
    #[serde(deserialize_with = "unique_controls")]
    pub minimum_enforcement: BTreeMap<Control, EnforcementStrength>,
    pub required_capabilities: BTreeSet<symbiote_runtime_sdk::Capability>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(try_from = "BindingWire")]
pub struct BindingConfiguration {
    pub schema_version: u32,
    pub binding: WorkforceBinding,
    pub team_revision: Revision,
    pub primary: StaffingCandidate,
    pub fallbacks: Vec<StaffingCandidate>,
    pub policies: WorkforcePolicies,
}
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct BindingWire {
    schema_version: u32,
    binding: WorkforceBinding,
    team_revision: Revision,
    primary: StaffingCandidate,
    fallbacks: Vec<StaffingCandidate>,
    policies: WorkforcePolicies,
}
impl TryFrom<BindingWire> for BindingConfiguration {
    type Error = BindingError;
    fn try_from(w: BindingWire) -> Result<Self, Self::Error> {
        let result = Self {
            schema_version: w.schema_version,
            binding: w.binding,
            team_revision: w.team_revision,
            primary: w.primary,
            fallbacks: w.fallbacks,
            policies: w.policies,
        };
        result.validate()?;
        Ok(result)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum BindingError {
    UnsupportedVersion,
    ResourceLimit,
    InvalidReference,
    InvalidProfile,
    InvalidPolicy,
    ProjectMismatch,
    ProfileMismatch,
    DuplicateProfile,
    TeamMismatch,
    RoleMissing,
    PrivilegeEscalation,
}
impl std::fmt::Display for BindingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for BindingError {}
fn symbolic(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 96
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}
fn names(values: &BTreeSet<String>) -> bool {
    values.len() <= 32 && values.iter().all(|v| symbolic(v))
}
fn access_within(a: &AccessSnapshot, b: &AccessSnapshot) -> bool {
    a.project_id == b.project_id
        && a.policy_revision == b.policy_revision
        && a.roots.is_subset(&b.roots)
        && a.grants.is_subset(&b.grants)
}
impl StaffingCandidate {
    pub fn validate(&self) -> Result<(), BindingError> {
        if !symbolic(&self.config_identity)
            || !names(&self.tools)
            || !names(&self.skills)
            || !names(&self.secret_scopes)
        {
            return Err(BindingError::InvalidReference);
        }
        if self.profile.eligible_hosts.is_empty()
            || self.profile.eligible_hosts.len() > 16
            || self.access.roots.is_empty()
            || self.access.roots.len() > 32
        {
            return Err(BindingError::ResourceLimit);
        }
        if self.profile.runtime == RuntimeKind::ExternalHarness
            && self.profile.installation.is_none()
        {
            return Err(BindingError::InvalidProfile);
        }
        if !self.access.grants.contains(&Permission::ReadRoot)
            || (!self.secret_scopes.is_empty()
                && !self.access.grants.contains(&Permission::UseCredential))
        {
            return Err(BindingError::InvalidPolicy);
        }
        let l = &self.limits;
        if l.max_total_tokens == 0
            || l.max_total_tokens > 1_000_000_000
            || l.max_wall_time_ms == 0
            || l.max_wall_time_ms > 604_800_000
            || l.max_concurrency == 0
            || l.max_concurrency > 64
            || l.max_memory_bytes == 0
            || l.max_memory_bytes > (1u64 << 50)
        {
            return Err(BindingError::ResourceLimit);
        }
        let context = u64::from(self.context.max_input_tokens)
            + u64::from(self.context.reserved_output_tokens);
        if self.context.max_input_tokens == 0
            || self.context.reserved_output_tokens == 0
            || context > l.max_total_tokens
        {
            return Err(BindingError::InvalidPolicy);
        }
        Ok(())
    }
}
fn unique_controls<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<BTreeMap<Control, EnforcementStrength>, D::Error> {
    struct Visitor;
    impl<'de> serde::de::Visitor<'de> for Visitor {
        type Value = BTreeMap<Control, EnforcementStrength>;
        fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("unique control requirements")
        }
        fn visit_map<M: serde::de::MapAccess<'de>>(
            self,
            mut map: M,
        ) -> Result<Self::Value, M::Error> {
            let mut result = BTreeMap::new();
            while let Some((key, value)) = map.next_entry()? {
                if result.insert(key, value).is_some() {
                    return Err(serde::de::Error::custom("duplicate control"));
                }
            }
            Ok(result)
        }
    }
    deserializer.deserialize_map(Visitor)
}
struct SizeLimit(usize);
impl Write for SizeLimit {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0 = self
            .0
            .checked_add(bytes.len())
            .filter(|n| *n <= MAX_BINDING_BYTES)
            .ok_or_else(|| std::io::Error::other("binding size exceeded"))?;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
impl BindingConfiguration {
    pub fn validate(&self) -> Result<(), BindingError> {
        if self.schema_version != BINDING_VERSION {
            return Err(BindingError::UnsupportedVersion);
        }
        if self.fallbacks.len() > 4 {
            return Err(BindingError::ResourceLimit);
        }
        if self.primary.profile.id != self.binding.profile_id
            || self.primary.profile.revision != self.binding.profile_revision
        {
            return Err(BindingError::ProfileMismatch);
        }
        if self.primary.access != self.binding.access
            || self.primary.context != self.binding.context
            || self.primary.tools != self.binding.required_tools
            || self.primary.skills != self.binding.required_skills
        {
            return Err(BindingError::InvalidPolicy);
        }
        let mut profiles = BTreeSet::new();
        for c in std::iter::once(&self.primary).chain(&self.fallbacks) {
            c.validate()?;
            if c.access.project_id != self.binding.project_id {
                return Err(BindingError::ProjectMismatch);
            }
            if !profiles.insert(&c.profile.id) {
                return Err(BindingError::DuplicateProfile);
            }
        }
        let p = &self.policies;
        for reference in [
            &p.environment,
            &p.worktree,
            &p.verification,
            &p.documentation,
            &p.artifacts,
            &p.mcp,
            &p.escalation,
        ] {
            if !symbolic(&reference.id) {
                return Err(BindingError::InvalidReference);
            }
        }
        if self.binding.required_controls.is_empty()
            || !self
                .binding
                .required_controls
                .iter()
                .all(|control| p.minimum_enforcement.contains_key(control))
            || p.minimum_enforcement
                .values()
                .any(|v| *v == EnforcementStrength::Unsupported)
        {
            return Err(BindingError::InvalidPolicy);
        }
        serde_json::to_writer(SizeLimit(0), self).map_err(|_| BindingError::ResourceLimit)?;
        Ok(())
    }
    pub fn validate_team(&self, team: &TeamConfiguration) -> Result<(), BindingError> {
        self.validate()?;
        team.validate().map_err(|_| BindingError::TeamMismatch)?;
        if self.binding.project_id != team.project_id || self.team_revision != team.revision {
            return Err(BindingError::TeamMismatch);
        }
        let member = team
            .members
            .iter()
            .find(|m| m.role_id == self.binding.role_id)
            .ok_or(BindingError::RoleMissing)?;
        for c in std::iter::once(&self.primary).chain(&self.fallbacks) {
            if !access_within(&c.access, &member.access)
                || !access_within(&c.access, &team.access_ceiling)
            {
                return Err(BindingError::PrivilegeEscalation);
            }
        }
        Ok(())
    }
    pub fn candidate(&self, profile_id: &RuntimeProfileId) -> Option<&StaffingCandidate> {
        std::iter::once(&self.primary)
            .chain(&self.fallbacks)
            .find(|c| &c.profile.id == profile_id)
    }
    pub fn parse(input: &str) -> Result<Self, BindingError> {
        if input.len() > MAX_BINDING_BYTES {
            return Err(BindingError::ResourceLimit);
        }
        serde_json::from_str(input).map_err(|_| BindingError::InvalidPolicy)
    }
}
