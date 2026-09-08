//! Project Team operating policy, separate from immutable runtime Dispatch and
//! WorkforceBinding contracts. Structural validity is not runtime qualification.
use crate::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RoleFunction {
    LeadOrchestrator,
    GeneralExecution,
    Specialist,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FallbackConsent {
    ExplicitRequired,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RoleFallback {
    pub role_id: RoleId,
    pub consent: FallbackConsent,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RolePolicy {
    pub role_id: RoleId,
    pub function: RoleFunction,
    pub responsibilities: Vec<String>,
    pub task_domains: BTreeSet<String>,
    pub access: AccessSnapshot,
    pub context_policy_ref: String,
    pub tool_policy_ref: String,
    pub skill_policy_ref: String,
    pub execution_policy_ref: String,
    pub independent_reviewers: BTreeSet<RoleId>,
    pub fallbacks: Vec<RoleFallback>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(try_from = "TeamWire")]
pub struct TeamConfiguration {
    pub schema_version: u32,
    pub project_id: ProjectId,
    pub revision: Revision,
    pub lead_role_id: RoleId,
    pub members: Vec<RolePolicy>,
    pub access_ceiling: AccessSnapshot,
}
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct TeamWire {
    schema_version: u32,
    project_id: ProjectId,
    revision: Revision,
    lead_role_id: RoleId,
    members: Vec<RolePolicy>,
    access_ceiling: AccessSnapshot,
}
impl TryFrom<TeamWire> for TeamConfiguration {
    type Error = TeamError;
    fn try_from(w: TeamWire) -> Result<Self, TeamError> {
        let team = Self {
            schema_version: w.schema_version,
            project_id: w.project_id,
            revision: w.revision,
            lead_role_id: w.lead_role_id,
            members: w.members,
            access_ceiling: w.access_ceiling,
        };
        team.validate()?;
        Ok(team)
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TeamError {
    UnsupportedVersion,
    ResourceLimit,
    InvalidPolicy,
    ProjectMismatch,
    DuplicateRole,
    MissingLead,
    MissingExecutionRole,
    DanglingRole,
    SelfReview,
    UnreviewedRole,
    PrivilegeEscalation,
    CyclicFallback,
    InvalidFallback,
}
impl fmt::Display for TeamError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for TeamError {}
fn reference(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.as_bytes()[0].is_ascii_alphanumeric()
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
}
fn access_within(access: &AccessSnapshot, ceiling: &AccessSnapshot) -> bool {
    access.project_id == ceiling.project_id
        && access.policy_revision == ceiling.policy_revision
        && access.roots.is_subset(&ceiling.roots)
        && access.grants.is_subset(&ceiling.grants)
}
struct Size(usize);
impl std::io::Write for Size {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if bytes.len() > 65_536 - self.0 {
            return Err(std::io::Error::other("team limit"));
        }
        self.0 += bytes.len();
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
impl TeamConfiguration {
    /// Valid configured Team, not a draft or evidence of qualified workforce bindings.
    pub fn validate(&self) -> Result<(), TeamError> {
        if self.schema_version != 1 {
            return Err(TeamError::UnsupportedVersion);
        }
        if self.members.len() > 32 || self.access_ceiling.roots.len() > 64 {
            return Err(TeamError::ResourceLimit);
        }
        if self.access_ceiling.project_id != self.project_id {
            return Err(TeamError::ProjectMismatch);
        }
        if self.access_ceiling.roots.is_empty() {
            return Err(TeamError::InvalidPolicy);
        }
        let mut members = BTreeMap::new();
        for member in &self.members {
            if members.insert(&member.role_id, member).is_some() {
                return Err(TeamError::DuplicateRole);
            }
            if member.responsibilities.is_empty()
                || member.responsibilities.len() > 16
                || member.task_domains.is_empty()
                || member.task_domains.len() > 16
                || member.independent_reviewers.len() > 32
                || member.fallbacks.len() > 8
            {
                return Err(TeamError::InvalidPolicy);
            }
            if member
                .responsibilities
                .iter()
                .any(|s| s.trim().is_empty() || s.len() > 1024 || s.contains('\0'))
                || member.task_domains.iter().any(|s| !reference(s))
                || [
                    &member.context_policy_ref,
                    &member.tool_policy_ref,
                    &member.skill_policy_ref,
                    &member.execution_policy_ref,
                ]
                .into_iter()
                .any(|s| !reference(s))
            {
                return Err(TeamError::InvalidPolicy);
            }
            if member.access.project_id != self.project_id {
                return Err(TeamError::ProjectMismatch);
            }
            if !access_within(&member.access, &self.access_ceiling) {
                return Err(TeamError::PrivilegeEscalation);
            }
            if member.access.roots.is_empty()
                || !member.access.grants.contains(&Permission::ReadRoot)
            {
                return Err(TeamError::InvalidPolicy);
            }
        }
        let lead = members
            .get(&self.lead_role_id)
            .ok_or(TeamError::MissingLead)?;
        if lead.function != RoleFunction::LeadOrchestrator
            || members.values().any(|m| {
                m.function == RoleFunction::LeadOrchestrator && m.role_id != self.lead_role_id
            })
        {
            return Err(TeamError::MissingLead);
        }
        if !members.values().any(|m| {
            m.role_id != self.lead_role_id
                && m.function == RoleFunction::GeneralExecution
                && m.access.grants.contains(&Permission::ExecuteProcess)
        }) {
            return Err(TeamError::MissingExecutionRole);
        }
        for member in members.values() {
            if member.independent_reviewers.is_empty() {
                return Err(TeamError::UnreviewedRole);
            }
            for reviewer in &member.independent_reviewers {
                if reviewer == &member.role_id {
                    return Err(TeamError::SelfReview);
                }
                let reviewer = members.get(reviewer).ok_or(TeamError::DanglingRole)?;
                if !member.access.roots.is_subset(&reviewer.access.roots) {
                    return Err(TeamError::UnreviewedRole);
                }
            }
            let mut fallbacks = BTreeSet::new();
            for fallback in &member.fallbacks {
                if fallback.role_id == member.role_id || !fallbacks.insert(&fallback.role_id) {
                    return Err(TeamError::InvalidFallback);
                }
                let target = members
                    .get(&fallback.role_id)
                    .ok_or(TeamError::DanglingRole)?;
                if !access_within(&target.access, &member.access) {
                    return Err(TeamError::PrivilegeEscalation);
                }
            }
        }
        // Traverse only fallback edges. Reciprocal independent review is valid.
        fn visit<'a>(
            id: &'a RoleId,
            members: &BTreeMap<&'a RoleId, &'a RolePolicy>,
            active: &mut BTreeSet<&'a RoleId>,
            done: &mut BTreeSet<&'a RoleId>,
        ) -> Result<(), TeamError> {
            if done.contains(id) {
                return Ok(());
            }
            if !active.insert(id) {
                return Err(TeamError::CyclicFallback);
            }
            for next in &members[id].fallbacks {
                visit(&next.role_id, members, active, done)?;
            }
            active.remove(id);
            done.insert(id);
            Ok(())
        }
        let mut active = BTreeSet::new();
        let mut done = BTreeSet::new();
        for id in members.keys() {
            visit(id, &members, &mut active, &mut done)?;
        }
        serde_json::to_writer(Size(0), self).map_err(|_| TeamError::ResourceLimit)?;
        Ok(())
    }
    /// No runtime, model, billing or enforcement qualification is implied.
    pub fn autonomy_structure(&self) -> Result<(), TeamError> {
        self.validate()
    }
    pub fn referenced_roles(&self) -> BTreeSet<RoleId> {
        let mut roles = BTreeSet::from([self.lead_role_id.clone()]);
        for member in &self.members {
            roles.insert(member.role_id.clone());
            roles.extend(member.independent_reviewers.iter().cloned());
            roles.extend(member.fallbacks.iter().map(|f| f.role_id.clone()));
        }
        roles
    }
    /// Resolves names and identities through existing canonical Roles, not copies in Team policy.
    pub fn validate_roles(&self, roles: &[Role]) -> Result<(), TeamError> {
        self.validate()?;
        if roles.len() > 4096 {
            return Err(TeamError::ResourceLimit);
        }
        let mut index = BTreeMap::new();
        for role in roles {
            if index.insert(&role.id, role).is_some() {
                return Err(TeamError::DuplicateRole);
            }
        }
        for id in self.referenced_roles() {
            let role = index.get(&id).ok_or(TeamError::DanglingRole)?;
            if role.project_id != self.project_id {
                return Err(TeamError::ProjectMismatch);
            }
            if role.name.trim().is_empty()
                || role.name.len() > 256
                || role.name.chars().any(char::is_control)
            {
                return Err(TeamError::InvalidPolicy);
            }
        }
        Ok(())
    }
    pub fn parse(input: &str) -> Result<Self, TeamError> {
        if input.len() > 65_536 {
            return Err(TeamError::ResourceLimit);
        }
        serde_json::from_str(input).map_err(|_| TeamError::InvalidPolicy)
    }
}

/// Optional starting policies, never fixed Role identities or runtime assignments.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RoleTemplate {
    LeadOrchestrator,
    GeneralExecution,
    Architect,
    Backend,
    Frontend,
    Qa,
    Reviewer,
    Security,
    Documentation,
    Infrastructure,
}
impl RoleTemplate {
    /// Callers explicitly supply access, policy references and reviewer identities afterward.
    /// The resulting draft is intentionally not independently autonomy-ready.
    pub fn policy(self, role_id: RoleId, access: AccessSnapshot) -> RolePolicy {
        let (domain, responsibility) = match self {
            Self::LeadOrchestrator => {
                ("coordination", "Plan, partition and integrate Project work")
            }
            Self::GeneralExecution => (
                "implementation",
                "Implement and verify general Project tasks",
            ),
            Self::Architect => ("architecture", "Evaluate architecture and record decisions"),
            Self::Backend => ("backend", "Implement and verify backend changes"),
            Self::Frontend => ("frontend", "Implement accessible frontend changes"),
            Self::Qa => ("quality", "Exercise acceptance and failure scenarios"),
            Self::Reviewer => ("review", "Independently review changes and evidence"),
            Self::Security => ("security", "Review security boundaries and threat evidence"),
            Self::Documentation => (
                "documentation",
                "Maintain documentation with verified changes",
            ),
            Self::Infrastructure => ("infrastructure", "Plan and verify infrastructure work"),
        };
        RolePolicy {
            role_id,
            function: match self {
                Self::LeadOrchestrator => RoleFunction::LeadOrchestrator,
                Self::GeneralExecution | Self::Backend | Self::Frontend => {
                    RoleFunction::GeneralExecution
                }
                _ => RoleFunction::Specialist,
            },
            responsibilities: vec![responsibility.into()],
            task_domains: BTreeSet::from([domain.into()]),
            access,
            context_policy_ref: String::new(),
            tool_policy_ref: String::new(),
            skill_policy_ref: String::new(),
            execution_policy_ref: String::new(),
            independent_reviewers: BTreeSet::new(),
            fallbacks: Vec::new(),
        }
    }
}
