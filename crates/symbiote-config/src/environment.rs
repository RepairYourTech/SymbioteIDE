//! Portable desired state and a pure, fail-closed environment resolver.
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use symbiote_domain::{
    BindingId, DispatchId, HostId, ProjectId, RoleId, RuntimeKind, RuntimeProfileId, TaskId, UserId,
};

pub const ENVIRONMENT_SCHEMA_VERSION: u32 = 1;

/// Opaque catalog reference, never a path, executable, credential value or URL.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[serde(try_from = "String", into = "String")]
pub struct ResourceRef(String);
impl ResourceRef {
    pub fn new(value: impl Into<String>) -> Result<Self, EnvironmentError> {
        let value = value.into();
        if value.is_empty()
            || value.len() > 128
            || !value
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.'))
        {
            return Err(EnvironmentError::InvalidReference);
        }
        Ok(Self(value))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl TryFrom<String> for ResourceRef {
    type Error = EnvironmentError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}
impl From<ResourceRef> for String {
    fn from(value: ResourceRef) -> Self {
        value.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EnvironmentError {
    InvalidReference,
    InvalidVersion,
    LimitExceeded,
    WrongProject,
    InvalidSelector,
    DuplicateLayer,
    DuplicateResource,
    InvalidDocument,
}
impl std::fmt::Display for EnvironmentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("invalid environment contract")
    }
}
impl std::error::Error for EnvironmentError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EnvironmentMode {
    AdoptExisting,
    ManagedIsolated,
    Ephemeral,
}

/// Declaration order is the documented specificity order. Runtime profiles override
/// dispatch overlays; Host layers may constrain resources but never confer authority.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "scope",
    content = "id",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum TargetSelector {
    UserGlobal(UserId),
    Project(ProjectId),
    Role(RoleId),
    Binding(BindingId),
    Task(TaskId),
    Dispatch(DispatchId),
    RuntimeProfile(RuntimeProfileId),
    Host(HostId),
}
impl TargetSelector {
    fn specificity(&self) -> u8 {
        match self {
            Self::UserGlobal(_) => 0,
            Self::Project(_) => 1,
            Self::Role(_) => 2,
            Self::Binding(_) => 3,
            Self::Task(_) => 4,
            Self::Dispatch(_) => 5,
            Self::RuntimeProfile(_) => 6,
            Self::Host(_) => 7,
        }
    }
    fn matches(&self, t: &ResolutionTarget) -> bool {
        match self {
            Self::UserGlobal(id) => id == &t.user_id,
            Self::Project(id) => id == &t.project_id,
            Self::Role(id) => t.role_id.as_ref() == Some(id),
            Self::Binding(id) => t.binding_id.as_ref() == Some(id),
            Self::Task(id) => t.task_id.as_ref() == Some(id),
            Self::Dispatch(id) => t.dispatch_id.as_ref() == Some(id),
            Self::RuntimeProfile(id) => id == &t.profile_id,
            Self::Host(id) => id == &t.host_id,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ResourceKind {
    Mcp,
    Tool,
    Skill,
    Instruction,
    Rule,
    Command,
    Prompt,
    Hook,
    Plugin,
    Package,
    Environment,
    SecretReference,
    Permission,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Requirement {
    Required,
    Optional,
    Disabled,
    Inherited,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Ownership {
    UnmanagedUser,
    Adopted,
    SymbioteManaged,
    Generated,
    Ephemeral,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Trust {
    Untrusted,
    Approved,
    Quarantined,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Lifecycle {
    Desired,
    Active,
    Drifted,
    Conflicted,
    Retired,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HookIntent {
    PreExecution,
    ToolGuard,
    ToolObservation,
    PostEdit,
    CompletionRequest,
    Teardown,
    Diagnostic,
    ApprovalEscalation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EnvironmentResource {
    pub id: ResourceRef,
    pub kind: ResourceKind,
    pub payload_ref: ResourceRef,
    pub version_ref: ResourceRef,
    pub compatibility_ref: ResourceRef,
    pub requirement: Requirement,
    pub ownership: Ownership,
    pub trust: Trust,
    pub lifecycle: Lifecycle,
    pub hook_intent: Option<HookIntent>,
    /// Symbolic authority requirements, not permission grants.
    pub required_grants: BTreeSet<ResourceRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EnvironmentLayer {
    pub target: TargetSelector,
    pub source: ResourceRef,
    pub revision: ResourceRef,
    pub resources: Vec<EnvironmentResource>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EnvironmentDocument {
    pub schema_version: u32,
    pub project_id: ProjectId,
    pub mode: EnvironmentMode,
    /// Immutable policy catalog references; expertise overlays cannot replace these.
    pub core_policies: BTreeSet<ResourceRef>,
    pub layers: Vec<EnvironmentLayer>,
    /// Namespace -> symbolic catalog reference. Unknown namespaces round trip.
    pub extensions: BTreeMap<ResourceRef, ResourceRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ResolutionTarget {
    pub project_id: ProjectId,
    pub user_id: UserId,
    pub role_id: Option<RoleId>,
    pub binding_id: Option<BindingId>,
    pub task_id: Option<TaskId>,
    pub dispatch_id: Option<DispatchId>,
    pub profile_id: RuntimeProfileId,
    pub host_id: HostId,
    pub runtime_kind: RuntimeKind,
}
impl ResolutionTarget {
    pub fn parse(input: &str) -> Result<Self, EnvironmentError> {
        crate::json::parse(input, 16_384).map_err(|_| EnvironmentError::InvalidDocument)
    }
}

/// Trusted Host input, not portable desired state. Approval and runtime support
/// must be established outside this resolver. Exact resource equality binds facts
/// to versions, payloads, grants and lifecycle rather than just a resource name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceFact {
    pub resource: EnvironmentResource,
    pub supported: bool,
    pub feasible: bool,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolutionFacts {
    pub target: ResolutionTarget,
    pub mode: EnvironmentMode,
    /// Explicit support evidence for this target/profile/mode; false also covers unknown.
    pub mode_supported: bool,
    pub resources: Vec<ResourceFact>,
    pub approved_core_policies: BTreeSet<ResourceRef>,
    pub approved_extensions: BTreeSet<ResourceRef>,
    pub grants: BTreeSet<ResourceRef>,
    /// For delegated work, independently authenticated parent authority ceiling.
    pub parent_grants: Option<BTreeSet<ResourceRef>>,
}
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum BlockReason {
    MissingFacts,
    Unsupported,
    Infeasible,
    Untrusted,
    Quarantined,
    Drifted,
    Conflicted,
    Retired,
    RequiredConstraint,
    MissingGrant,
    ChildAuthorityExpansion,
    UnsupportedMode,
    CorePolicyUnapproved,
    ExtensionUnapproved,
    MissingInheritance,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EnvironmentBlocker {
    pub resource: Option<ResourceRef>,
    pub reason: BlockReason,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ResourceProvenance {
    pub target: TargetSelector,
    pub source: ResourceRef,
    pub revision: ResourceRef,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EffectiveResource {
    pub resource: EnvironmentResource,
    pub provenance: Vec<ResourceProvenance>,
    pub issues: BTreeSet<BlockReason>,
}
impl EffectiveResource {
    /// Eligible for a later Host preparation step, never execution authorization.
    /// Optional resources with issues are omitted even when the overall plan is unblocked.
    pub fn is_eligible(&self) -> bool {
        matches!(
            self.resource.requirement,
            Requirement::Required | Requirement::Optional
        ) && self.issues.is_empty()
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EffectiveEnvironment {
    pub target: ResolutionTarget,
    pub mode: EnvironmentMode,
    pub core_policies: BTreeSet<ResourceRef>,
    pub resources: BTreeMap<ResourceRef, EffectiveResource>,
    pub blockers: Vec<EnvironmentBlocker>,
}

impl EnvironmentDocument {
    pub fn parse(input: &str) -> Result<Self, EnvironmentError> {
        let document: Self =
            crate::json::parse(input, 1_048_576).map_err(|_| EnvironmentError::InvalidDocument)?;
        document.validate()?;
        Ok(document)
    }
    pub fn validate(&self) -> Result<(), EnvironmentError> {
        if self.schema_version != ENVIRONMENT_SCHEMA_VERSION {
            return Err(EnvironmentError::InvalidVersion);
        }
        if self.layers.len() > 128
            || self.core_policies.len() > 128
            || self.extensions.len() > 128
            || self.layers.iter().map(|l| l.resources.len()).sum::<usize>() > 4096
        {
            return Err(EnvironmentError::LimitExceeded);
        }
        let mut layers = BTreeSet::new();
        for layer in &self.layers {
            if !layers.insert((&layer.target, &layer.source)) {
                return Err(EnvironmentError::DuplicateLayer);
            }
            if matches!(&layer.target, TargetSelector::Project(id) if id != &self.project_id) {
                return Err(EnvironmentError::WrongProject);
            }
            let mut resources = BTreeSet::new();
            for resource in &layer.resources {
                if !resources.insert(&resource.id) {
                    return Err(EnvironmentError::DuplicateResource);
                }
                if resource.required_grants.len() > 128 {
                    return Err(EnvironmentError::LimitExceeded);
                }
                if (resource.kind == ResourceKind::Hook) != resource.hook_intent.is_some()
                    || self.core_policies.contains(&resource.id)
                {
                    return Err(EnvironmentError::InvalidDocument);
                }
            }
        }
        Ok(())
    }

    pub fn resolve(
        &self,
        target: &ResolutionTarget,
        facts: &ResolutionFacts,
    ) -> Result<EffectiveEnvironment, EnvironmentError> {
        self.validate()?;
        if target.project_id != self.project_id {
            return Err(EnvironmentError::WrongProject);
        }
        if facts.target != *target || facts.mode != self.mode {
            return Err(EnvironmentError::InvalidSelector);
        }
        if facts.resources.len() > 4096
            || facts.grants.len() > 4096
            || facts.parent_grants.as_ref().is_some_and(|g| g.len() > 4096)
        {
            return Err(EnvironmentError::LimitExceeded);
        }
        let mut fact_ids = BTreeSet::new();
        for fact in &facts.resources {
            if !fact_ids.insert(&fact.resource.id) {
                return Err(EnvironmentError::DuplicateResource);
            }
        }
        let mut plan = EffectiveEnvironment {
            target: target.clone(),
            mode: self.mode,
            core_policies: self.core_policies.clone(),
            resources: BTreeMap::new(),
            blockers: Vec::new(),
        };
        if !facts.mode_supported {
            plan.blockers.push(EnvironmentBlocker {
                resource: None,
                reason: BlockReason::UnsupportedMode,
            });
        }
        if facts
            .parent_grants
            .as_ref()
            .is_some_and(|parent| !facts.grants.is_subset(parent))
        {
            plan.blockers.push(EnvironmentBlocker {
                resource: None,
                reason: BlockReason::ChildAuthorityExpansion,
            });
        }
        for policy in self.core_policies.difference(&facts.approved_core_policies) {
            plan.blockers.push(EnvironmentBlocker {
                resource: Some(policy.clone()),
                reason: BlockReason::CorePolicyUnapproved,
            });
        }
        for reference in self.extensions.values() {
            if !facts.approved_extensions.contains(reference) {
                plan.blockers.push(EnvironmentBlocker {
                    resource: Some(reference.clone()),
                    reason: BlockReason::ExtensionUnapproved,
                });
            }
        }
        let mut layers: Vec<_> = self
            .layers
            .iter()
            .filter(|l| l.target.matches(target))
            .collect();
        layers.sort_by(|a, b| (&a.target, &a.source).cmp(&(&b.target, &b.source)));
        let mut seen: BTreeMap<(u8, ResourceRef), EnvironmentResource> = BTreeMap::new();
        for layer in layers {
            for resource in &layer.resources {
                let provenance = ResourceProvenance {
                    target: layer.target.clone(),
                    source: layer.source.clone(),
                    revision: layer.revision.clone(),
                };
                let prior = plan.resources.get(&resource.id);
                let required =
                    prior.is_some_and(|p| p.resource.requirement == Requirement::Required);
                let same_scope_conflict = seen
                    .get(&(layer.target.specificity(), resource.id.clone()))
                    .is_some_and(|r| r != resource);
                seen.insert(
                    (layer.target.specificity(), resource.id.clone()),
                    resource.clone(),
                );
                let entry = plan
                    .resources
                    .entry(resource.id.clone())
                    .or_insert_with(|| EffectiveResource {
                        resource: resource.clone(),
                        provenance: Vec::new(),
                        issues: BTreeSet::new(),
                    });
                if same_scope_conflict {
                    entry.issues.insert(BlockReason::Conflicted);
                }
                if resource.requirement == Requirement::Inherited {
                    if entry.provenance.is_empty() {
                        entry.issues.insert(BlockReason::MissingInheritance);
                    }
                } else if required && entry.resource != *resource {
                    entry.issues.insert(BlockReason::RequiredConstraint);
                } else {
                    entry.resource = resource.clone();
                }
                entry.provenance.push(provenance);
            }
        }
        for (id, entry) in &mut plan.resources {
            let resource = &entry.resource;
            if resource.requirement != Requirement::Disabled {
                match resource.trust {
                    Trust::Untrusted => {
                        entry.issues.insert(BlockReason::Untrusted);
                    }
                    Trust::Quarantined => {
                        entry.issues.insert(BlockReason::Quarantined);
                    }
                    Trust::Approved => {}
                }
                match resource.lifecycle {
                    Lifecycle::Drifted => {
                        entry.issues.insert(BlockReason::Drifted);
                    }
                    Lifecycle::Conflicted => {
                        entry.issues.insert(BlockReason::Conflicted);
                    }
                    Lifecycle::Retired => {
                        entry.issues.insert(BlockReason::Retired);
                    }
                    _ => {}
                }
                if !resource.required_grants.is_subset(&facts.grants) {
                    entry.issues.insert(BlockReason::MissingGrant);
                }
                match facts.resources.iter().find(|f| f.resource == *resource) {
                    None => {
                        entry.issues.insert(BlockReason::MissingFacts);
                    }
                    Some(fact) => {
                        if !fact.supported {
                            entry.issues.insert(BlockReason::Unsupported);
                        }
                        if !fact.feasible {
                            entry.issues.insert(BlockReason::Infeasible);
                        }
                    }
                }
            }
            for reason in &entry.issues {
                if resource.requirement == Requirement::Required
                    || matches!(
                        reason,
                        BlockReason::RequiredConstraint
                            | BlockReason::Conflicted
                            | BlockReason::MissingInheritance
                    )
                {
                    plan.blockers.push(EnvironmentBlocker {
                        resource: Some(id.clone()),
                        reason: *reason,
                    });
                }
            }
        }
        Ok(plan)
    }
}
