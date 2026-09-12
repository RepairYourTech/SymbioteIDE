use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use symbiote_domain::*;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    StructuredMessages,
    Streaming,
    Tools,
    Approvals,
    Questions,
    Resume,
    Fork,
    Models,
    Reasoning,
    ContextLimits,
    Usage,
    Quotas,
    Hooks,
    Mcp,
    Skills,
    NativeTasks,
    Images,
    WorktreeIsolation,
    Instructions,
    Extensions,
    ProfileIsolation,
    Authentication,
    Cancellation,
    GoalPersistence,
    ChildAgents,
    ChildModelSelection,
    ChildEvents,
    ChildUsage,
    AggregateBudgets,
    DepthLimits,
    ConcurrencyLimits,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum IntegrationTier {
    TerminalCompatible,
    Detected,
    Managed,
    Structured,
    OrchestratorGrade,
    Certified,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum RuntimeOwner {
    SymbioteNative {},
    External { driver: HarnessDriverId },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Transport {
    NativeLoop,
    StructuredRpc,
    Pty,
}

/// Metadata references to Host-verified proof, never a self-authenticating claim.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProbeEvidence {
    pub artifact: EvidenceId,
    pub adapter_id: AgentRuntimeAdapterId,
    pub installation: Option<InstallationId>,
    pub profile_id: RuntimeProfileId,
    pub profile_revision: Revision,
    pub model_id: ModelId,
    pub host_id: HostId,
    pub adapter_version: String,
    pub upstream_version: String,
    pub platform: String,
    pub observed_at: Timestamp,
    pub expires_at: Timestamp,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum Support {
    Supported { evidence: Box<ProbeEvidence> },
    Unsupported,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ControlSupport {
    pub strength: EnforcementStrength,
    pub mechanism: String,
    pub evidence: ProbeEvidence,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RuntimeContextLimits {
    pub context_window_tokens: u32,
    pub max_output_tokens: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RuntimeDescriptor {
    pub sdk_version: u32,
    pub adapter_id: AgentRuntimeAdapterId,
    pub installation: Option<InstallationId>,
    pub profile_id: RuntimeProfileId,
    pub profile_revision: Revision,
    pub model_id: ModelId,
    pub adapter_version: String,
    pub upstream_version: String,
    pub runtime: RuntimeKind,
    pub owner: RuntimeOwner,
    pub transport: Transport,
    /// Integration tier does not imply any individual capability or certification.
    pub tier: IntegrationTier,
    pub host_id: HostId,
    pub platform: String,
    pub capabilities: BTreeMap<Capability, Support>,
    pub controls: BTreeMap<Control, ControlSupport>,
    pub tools: BTreeSet<String>,
    pub skills: BTreeSet<String>,
    pub context_limits: Option<RuntimeContextLimits>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QualificationError {
    UnsupportedVersion,
    InvalidDescriptor,
    RuntimeOwnerMismatch,
    AdapterMismatch,
    IneligibleHost,
    MissingCapability(Capability),
    UnknownCapability(Capability),
    MissingControl(Control),
    InsufficientControl(Control),
    StaleEvidence,
    InvalidContract,
    MissingResource,
    ContextLimitExceeded,
}
impl std::fmt::Display for QualificationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for QualificationError {}

impl RuntimeDescriptor {
    pub fn validate(&self) -> Result<(), QualificationError> {
        if self.sdk_version != crate::SDK_VERSION {
            return Err(QualificationError::UnsupportedVersion);
        }
        for text in [
            &self.adapter_version,
            &self.upstream_version,
            &self.platform,
        ] {
            if text.is_empty() || text.len() > 128 || text.chars().any(char::is_control) {
                return Err(QualificationError::InvalidDescriptor);
            }
        }
        if self.tools.len() > 128
            || self.skills.len() > 128
            || self.tools.iter().chain(&self.skills).any(|text| {
                text.trim().is_empty() || text.len() > 128 || text.chars().any(char::is_control)
            })
        {
            return Err(QualificationError::InvalidDescriptor);
        }
        if self.context_limits.as_ref().is_some_and(|limits| {
            limits.max_output_tokens == 0 || limits.context_window_tokens < limits.max_output_tokens
        }) {
            return Err(QualificationError::InvalidDescriptor);
        }
        if !matches!(
            (&self.runtime, &self.owner, &self.transport),
            (
                RuntimeKind::NativeSymbiote,
                RuntimeOwner::SymbioteNative {},
                Transport::NativeLoop
            ) | (
                RuntimeKind::ExternalHarness,
                RuntimeOwner::External { .. },
                Transport::StructuredRpc | Transport::Pty
            )
        ) {
            return Err(QualificationError::RuntimeOwnerMismatch);
        }
        Ok(())
    }
    fn evidence(&self, proof: &ProbeEvidence, now: Timestamp) -> Result<(), QualificationError> {
        if proof.adapter_id != self.adapter_id
            || proof.installation != self.installation
            || proof.profile_id != self.profile_id
            || proof.profile_revision != self.profile_revision
            || proof.model_id != self.model_id
            || proof.host_id != self.host_id
            || proof.adapter_version != self.adapter_version
            || proof.upstream_version != self.upstream_version
            || proof.platform != self.platform
            || proof.observed_at > now
            || proof.expires_at <= now
            || proof.observed_at >= proof.expires_at
        {
            return Err(QualificationError::StaleEvidence);
        }
        Ok(())
    }
}

/// A Host-provisioned declaration of a runtime's identity and facts, before any
/// observation exists. The evidence a [`RuntimeDescriptor`] carries is
/// Host-owned: a declaration names WHAT an operator asserts about a runtime
/// (its pinned profile identity, capabilities, controls and resources), and the
/// Host turns it into a descriptor stamped with its own identity and a bounded
/// evidence window — the same posture as Host enforcement claims, never a
/// substitute for sandbox-observed evidence (#269).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DeclaredRuntime {
    pub adapter_id: AgentRuntimeAdapterId,
    pub installation: Option<InstallationId>,
    pub profile_id: RuntimeProfileId,
    pub profile_revision: Revision,
    pub model_id: ModelId,
    pub adapter_version: String,
    pub upstream_version: String,
    pub runtime: RuntimeKind,
    pub owner: RuntimeOwner,
    pub transport: Transport,
    pub tier: IntegrationTier,
    pub platform: String,
    /// The capabilities the declared runtime supports. A capability absent here
    /// is unknown to the Host, never assumed.
    pub capabilities: BTreeSet<Capability>,
    pub controls: BTreeMap<Control, DeclaredControl>,
    pub tools: BTreeSet<String>,
    pub skills: BTreeSet<String>,
    pub context_limits: Option<RuntimeContextLimits>,
}

/// One declared enforcement control with the mechanism that realizes it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DeclaredControl {
    pub strength: EnforcementStrength,
    pub mechanism: String,
}

impl DeclaredRuntime {
    /// How long a Host-stamped observation stays valid. One hour, matching the
    /// Host's own enforcement claims until sandbox-observed evidence lands.
    pub const EVIDENCE_WINDOW_MS: u64 = 3_600_000;

    /// The descriptor the Host observes from this declaration: identity is the
    /// declaration's, while the observation identity, artifact and window are
    /// the Host's own and are never operator-supplied. Every declared capability
    /// and control carries the same fresh evidence; a capability a profile
    /// requires but the declaration omits stays absent, so it refuses rather
    /// than passing on an assumption.
    pub fn observe(
        &self,
        host_id: &HostId,
        at: Timestamp,
    ) -> Result<RuntimeDescriptor, QualificationError> {
        let artifact = EvidenceId::new(format!("runtime-declaration-{}", host_id.as_str()))
            .map_err(|_| QualificationError::InvalidDescriptor)?;
        let evidence = ProbeEvidence {
            artifact,
            adapter_id: self.adapter_id.clone(),
            installation: self.installation.clone(),
            profile_id: self.profile_id.clone(),
            profile_revision: self.profile_revision,
            model_id: self.model_id.clone(),
            host_id: host_id.clone(),
            adapter_version: self.adapter_version.clone(),
            upstream_version: self.upstream_version.clone(),
            platform: self.platform.clone(),
            observed_at: at,
            expires_at: Timestamp(at.0.saturating_add(Self::EVIDENCE_WINDOW_MS)),
        };
        Ok(RuntimeDescriptor {
            sdk_version: crate::SDK_VERSION,
            adapter_id: self.adapter_id.clone(),
            installation: self.installation.clone(),
            profile_id: self.profile_id.clone(),
            profile_revision: self.profile_revision,
            model_id: self.model_id.clone(),
            adapter_version: self.adapter_version.clone(),
            upstream_version: self.upstream_version.clone(),
            runtime: self.runtime,
            owner: self.owner.clone(),
            transport: self.transport.clone(),
            tier: self.tier.clone(),
            host_id: host_id.clone(),
            platform: self.platform.clone(),
            capabilities: self
                .capabilities
                .iter()
                .cloned()
                .map(|capability| {
                    (
                        capability,
                        Support::Supported {
                            evidence: Box::new(evidence.clone()),
                        },
                    )
                })
                .collect(),
            controls: self
                .controls
                .iter()
                .map(|(control, declared)| {
                    (
                        control.clone(),
                        ControlSupport {
                            strength: declared.strength,
                            mechanism: declared.mechanism.clone(),
                            evidence: evidence.clone(),
                        },
                    )
                })
                .collect(),
            tools: self.tools.clone(),
            skills: self.skills.clone(),
            context_limits: self.context_limits.clone(),
        })
    }

    /// Validates the declaration on its own terms, through the descriptor
    /// contract it can produce: the Host identity is irrelevant to the checks
    /// this performs, so it is a fixed local placeholder.
    pub fn validate(&self) -> Result<(), QualificationError> {
        let host =
            HostId::new("host_declared").map_err(|_| QualificationError::InvalidDescriptor)?;
        self.observe(&host, Timestamp(1))?.validate()
    }
}

/// Checks current facts, not adapter names or marketing tiers. The caller must
/// authenticate referenced proof artifacts before allowing this metadata to qualify.
pub fn qualify_profile(
    profile: &RuntimeProfile,
    descriptor: &RuntimeDescriptor,
    required_capabilities: &BTreeSet<Capability>,
    required_controls: &BTreeSet<Control>,
    now: Timestamp,
) -> Result<(), QualificationError> {
    let minimums = required_controls
        .iter()
        .cloned()
        .map(|control| (control, EnforcementStrength::HostEnforced))
        .collect();
    qualify_profile_with_minimums(profile, descriptor, required_capabilities, &minimums, now)
}

/// Matches explicit minimum controls while retaining exact, fresh evidence.
/// Observed and emulated controls are incomparable; neither prevents effects.
/// The caller authenticates evidence and decides which minimum is permissible.
pub fn qualify_profile_with_minimums(
    profile: &RuntimeProfile,
    descriptor: &RuntimeDescriptor,
    required_capabilities: &BTreeSet<Capability>,
    minimums: &BTreeMap<Control, EnforcementStrength>,
    now: Timestamp,
) -> Result<(), QualificationError> {
    descriptor.validate()?;
    if profile.adapter != descriptor.adapter_id
        || profile.runtime != descriptor.runtime
        || profile.installation != descriptor.installation
        || profile.id != descriptor.profile_id
        || profile.revision != descriptor.profile_revision
        || profile.model != descriptor.model_id
    {
        return Err(QualificationError::AdapterMismatch);
    }
    if !profile.eligible_hosts.contains(&descriptor.host_id) {
        return Err(QualificationError::IneligibleHost);
    }
    if profile.runtime == RuntimeKind::ExternalHarness && profile.installation.is_none() {
        return Err(QualificationError::InvalidDescriptor);
    }
    for capability in required_capabilities {
        match descriptor.capabilities.get(capability) {
            Some(Support::Supported { evidence }) => descriptor.evidence(evidence, now)?,
            Some(Support::Unknown) => {
                return Err(QualificationError::UnknownCapability(capability.clone()));
            }
            _ => return Err(QualificationError::MissingCapability(capability.clone())),
        }
    }
    for (control, minimum) in minimums {
        let support = descriptor
            .controls
            .get(control)
            .ok_or_else(|| QualificationError::MissingControl(control.clone()))?;
        let meets = match minimum {
            EnforcementStrength::Native => support.strength == EnforcementStrength::Native,
            EnforcementStrength::HostEnforced => matches!(
                support.strength,
                EnforcementStrength::Native | EnforcementStrength::HostEnforced
            ),
            EnforcementStrength::ExternallyObserved => matches!(
                support.strength,
                EnforcementStrength::Native
                    | EnforcementStrength::HostEnforced
                    | EnforcementStrength::ExternallyObserved
            ),
            EnforcementStrength::Emulated => matches!(
                support.strength,
                EnforcementStrength::Native
                    | EnforcementStrength::HostEnforced
                    | EnforcementStrength::Emulated
            ),
            EnforcementStrength::Unsupported => false,
        };
        if !meets || support.mechanism.trim().is_empty() || support.mechanism.len() > 256 {
            return Err(QualificationError::InsufficientControl(control.clone()));
        }
        descriptor.evidence(&support.evidence, now)?;
    }
    Ok(())
}

pub fn qualify_dispatch(
    dispatch: &Dispatch,
    descriptor: &RuntimeDescriptor,
    required_capabilities: &BTreeSet<Capability>,
    now: Timestamp,
) -> Result<(), QualificationError> {
    let contract = dispatch.contract();
    contract
        .validate_at(now)
        .map_err(|_| QualificationError::InvalidContract)?;
    if contract.host_id() != &descriptor.host_id {
        return Err(QualificationError::IneligibleHost);
    }
    let controls = contract.enforcement().keys().cloned().collect();
    let mut capabilities = required_capabilities.clone();
    capabilities.insert(Capability::ContextLimits);
    let limits = descriptor
        .context_limits
        .as_ref()
        .ok_or(QualificationError::ContextLimitExceeded)?;
    let context = &contract.binding().context;
    if context.reserved_output_tokens > limits.max_output_tokens
        || context
            .max_input_tokens
            .checked_add(context.reserved_output_tokens)
            .is_none_or(|total| total > limits.context_window_tokens)
    {
        return Err(QualificationError::ContextLimitExceeded);
    }
    if !contract.binding().required_tools.is_empty() {
        capabilities.insert(Capability::Tools);
    }
    if !contract.binding().required_skills.is_empty() {
        capabilities.insert(Capability::Skills);
    }
    if !contract
        .binding()
        .required_tools
        .is_subset(&descriptor.tools)
        || !contract
            .binding()
            .required_skills
            .is_subset(&descriptor.skills)
    {
        return Err(QualificationError::MissingResource);
    }
    qualify_profile(
        contract.profile(),
        descriptor,
        &capabilities,
        &controls,
        now,
    )
}
