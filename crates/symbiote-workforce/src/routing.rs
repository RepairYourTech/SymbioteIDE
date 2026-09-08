//! Deterministic work routing to a Team Role. A resolved Role is a Team policy
//! statement, never a qualified runtime binding, and no-route never silently
//! substitutes a different Role.
use crate::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fmt};

pub const ROUTE_VERSION: u32 = 1;
pub const MAX_ROUTE_DOMAINS: usize = 16;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RouteConfidence {
    /// Derived entirely from recorded Team structure by an exact rule.
    Deterministic,
    /// Closest partial coverage; recorded so heuristic routing stays auditable.
    Heuristic,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RouteReason {
    /// An explicit Lead/user assignment, honored before any classification.
    ExplicitRequest,
    /// Every requested domain is covered by the resolved Role's task domains.
    ExactDomainMatch,
    /// First partial domain coverage in deterministic Role order, not maximal coverage.
    PartialDomainMatch,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum RouteDiagnosis {
    /// The explicitly requested Role is not a current Team member.
    RequestedRoleNotMember { role: RoleId },
    /// No executable non-lead Team member covers these domains.
    NoExecutableRole { domains: BTreeSet<String> },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RouteDecision {
    pub schema_version: u32,
    pub project_id: ProjectId,
    pub work_id: WorkId,
    /// Explicit assignment, preserved separately from the resolution.
    pub requested: Option<RoleId>,
    /// None records a no-route diagnosis instead of a silent substitution.
    pub resolved: Option<RoleId>,
    pub reason: Option<RouteReason>,
    pub confidence: Option<RouteConfidence>,
    pub domains: BTreeSet<String>,
    pub diagnosis: Option<RouteDiagnosis>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RouteError {
    InvalidTeam,
    InvalidRequest,
    ProjectMismatch,
}
impl fmt::Display for RouteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for RouteError {}

/// Domain labels mirror Team task domain references so routing inputs stay
/// comparable with configured Team policy.
fn domain_reference(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.as_bytes()[0].is_ascii_alphanumeric()
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
}

impl RouteDecision {
    pub fn validate(&self) -> Result<(), RouteError> {
        if self.schema_version != ROUTE_VERSION
            || self.domains.len() > MAX_ROUTE_DOMAINS
            || self.domains.iter().any(|d| !domain_reference(d))
        {
            return Err(RouteError::InvalidRequest);
        }
        match (&self.resolved, &self.diagnosis) {
            (Some(_), None) => {
                if self.reason.is_none() || self.confidence.is_none() {
                    return Err(RouteError::InvalidRequest);
                }
            }
            (None, Some(_)) => {
                if self.reason.is_some() || self.confidence.is_some() {
                    return Err(RouteError::InvalidRequest);
                }
            }
            _ => return Err(RouteError::InvalidRequest),
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RouteRequest {
    pub project_id: ProjectId,
    pub work_id: WorkId,
    /// Explicit Lead/user assignment; honored before classification.
    pub requested: Option<RoleId>,
    pub domains: BTreeSet<String>,
}

impl RouteRequest {
    pub fn validate(&self) -> Result<(), RouteError> {
        if self.domains.len() > MAX_ROUTE_DOMAINS
            || self.domains.iter().any(|d| !domain_reference(d))
        {
            return Err(RouteError::InvalidRequest);
        }
        if self.requested.is_none() && self.domains.is_empty() {
            return Err(RouteError::InvalidRequest);
        }
        Ok(())
    }
}

/// Same recorded Team structure and inputs always yield the same decision.
/// Eligibility is Team-structural: runtime qualification, binding readiness and
/// permission enforcement remain separate downstream gates.
pub fn resolve_route(
    team: &TeamConfiguration,
    request: &RouteRequest,
) -> Result<RouteDecision, RouteError> {
    team.validate().map_err(|_| RouteError::InvalidTeam)?;
    request.validate()?;
    if request.project_id != team.project_id {
        return Err(RouteError::ProjectMismatch);
    }
    let base = RouteDecision {
        schema_version: ROUTE_VERSION,
        project_id: request.project_id.clone(),
        work_id: request.work_id.clone(),
        requested: request.requested.clone(),
        resolved: None,
        reason: None,
        confidence: None,
        domains: request.domains.clone(),
        diagnosis: None,
    };
    if let Some(role) = &request.requested {
        return if team.members.iter().any(|m| &m.role_id == role) {
            Ok(RouteDecision {
                resolved: Some(role.clone()),
                reason: Some(RouteReason::ExplicitRequest),
                confidence: Some(RouteConfidence::Deterministic),
                ..base
            })
        } else {
            Ok(RouteDecision {
                diagnosis: Some(RouteDiagnosis::RequestedRoleNotMember { role: role.clone() }),
                ..base
            })
        };
    }
    // Classification never routes to the Lead. The Lead orchestrates; when Lead
    // execution is intended it is requested explicitly and recorded as such.
    let mut executable = BTreeMap::new();
    for member in &team.members {
        if member.function != RoleFunction::LeadOrchestrator
            && member.access.grants.contains(&Permission::ExecuteProcess)
        {
            executable.insert(&member.role_id, member);
        }
    }
    let resolved = executable
        .values()
        .find(|m| m.task_domains.is_superset(&request.domains))
        .map(|m| {
            (
                m.role_id.clone(),
                RouteReason::ExactDomainMatch,
                RouteConfidence::Deterministic,
            )
        })
        .or_else(|| {
            executable
                .values()
                .find(|m| !m.task_domains.is_disjoint(&request.domains))
                .map(|m| {
                    (
                        m.role_id.clone(),
                        RouteReason::PartialDomainMatch,
                        RouteConfidence::Heuristic,
                    )
                })
        });
    match resolved {
        Some((role, reason, confidence)) => Ok(RouteDecision {
            resolved: Some(role),
            reason: Some(reason),
            confidence: Some(confidence),
            ..base
        }),
        None => Ok(RouteDecision {
            diagnosis: Some(RouteDiagnosis::NoExecutableRole {
                domains: request.domains.clone(),
            }),
            ..base
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use symbiote_domain::{AccessSnapshot, Permission, RoleId, RolePolicy};

    fn access(project: &str) -> AccessSnapshot {
        AccessSnapshot {
            project_id: ProjectId::new(project).unwrap(),
            roots: BTreeSet::from([RootId::new("root").unwrap()]),
            grants: BTreeSet::from([Permission::ReadRoot, Permission::ExecuteProcess]),
            policy_revision: Revision(1),
        }
    }

    fn member(role: &str, function: RoleFunction, domains: &[&str]) -> RolePolicy {
        let mut policy = RolePolicy {
            role_id: RoleId::new(role).unwrap(),
            function,
            responsibilities: vec!["test responsibility".into()],
            task_domains: BTreeSet::new(),
            access: access("demo"),
            context_policy_ref: "context".into(),
            tool_policy_ref: "tools".into(),
            skill_policy_ref: "skills".into(),
            execution_policy_ref: "execution".into(),
            independent_reviewers: BTreeSet::new(),
            fallbacks: Vec::new(),
        };
        for domain in domains {
            policy.task_domains.insert((*domain).into());
        }
        policy
    }

    fn team(members: Vec<RolePolicy>) -> TeamConfiguration {
        let lead = members
            .iter()
            .find(|m| m.function == RoleFunction::LeadOrchestrator)
            .map(|m| m.role_id.clone())
            .unwrap();
        let mut configured = TeamConfiguration {
            schema_version: 1,
            project_id: ProjectId::new("demo").unwrap(),
            revision: Revision(0),
            lead_role_id: lead,
            access_ceiling: access("demo"),
            members,
        };
        // Reviewer links only need root coverage; reciprocal review is valid.
        let ids: Vec<RoleId> = configured
            .members
            .iter()
            .map(|m| m.role_id.clone())
            .collect();
        for member in &mut configured.members {
            member.independent_reviewers = ids
                .iter()
                .filter(|id| *id != &member.role_id)
                .cloned()
                .collect();
        }
        configured
    }

    fn request(
        project: &str,
        work: &str,
        requested: Option<&str>,
        domains: &[&str],
    ) -> RouteRequest {
        RouteRequest {
            project_id: ProjectId::new(project).unwrap(),
            work_id: WorkId::Request(RequestId::new(work).unwrap()),
            requested: requested.map(|r| RoleId::new(r).unwrap()),
            domains: domains.iter().map(|d| (*d).into()).collect(),
        }
    }

    fn standard() -> TeamConfiguration {
        team(vec![
            member("lead", RoleFunction::LeadOrchestrator, &["coordination"]),
            member(
                "engineer",
                RoleFunction::GeneralExecution,
                &["backend", "coding"],
            ),
        ])
    }

    #[test]
    fn explicit_request_resolves_without_classification() {
        let decision = resolve_route(
            &standard(),
            &request("demo", "w1", Some("lead"), &["coordination"]),
        )
        .unwrap();
        assert_eq!(decision.resolved.as_ref().map(|r| r.as_str()), Some("lead"));
        assert_eq!(decision.reason, Some(RouteReason::ExplicitRequest));
        assert_eq!(decision.confidence, Some(RouteConfidence::Deterministic));
        assert_eq!(
            decision.requested.as_ref().map(|r| r.as_str()),
            Some("lead")
        );
        decision.validate().unwrap();
    }

    #[test]
    fn explicit_request_survives_domain_mismatch() {
        let decision = resolve_route(
            &standard(),
            &request("demo", "w1", Some("engineer"), &["frontend"]),
        )
        .unwrap();
        assert_eq!(
            decision.resolved.as_ref().map(|r| r.as_str()),
            Some("engineer")
        );
        assert_eq!(decision.reason, Some(RouteReason::ExplicitRequest));
    }

    #[test]
    fn unknown_explicit_role_diagnoses_without_substitution() {
        let decision = resolve_route(
            &standard(),
            &request("demo", "w1", Some("ghost"), &["coding"]),
        )
        .unwrap();
        assert_eq!(decision.resolved, None);
        match &decision.diagnosis {
            Some(RouteDiagnosis::RequestedRoleNotMember { role }) => {
                assert_eq!(role.as_str(), "ghost");
            }
            other => panic!("unexpected diagnosis: {other:?}"),
        }
        // The requested Role stays recorded even when routing failed.
        assert_eq!(
            decision.requested.as_ref().map(|r| r.as_str()),
            Some("ghost")
        );
        decision.validate().unwrap();
    }

    #[test]
    fn classification_prefers_full_coverage_over_the_lead() {
        let decision =
            resolve_route(&standard(), &request("demo", "w1", None, &["coding"])).unwrap();
        assert_eq!(
            decision.resolved.as_ref().map(|r| r.as_str()),
            Some("engineer")
        );
        assert_eq!(decision.reason, Some(RouteReason::ExactDomainMatch));
        assert_eq!(decision.confidence, Some(RouteConfidence::Deterministic));
    }

    #[test]
    fn partial_coverage_is_recorded_as_heuristic() {
        let decision = resolve_route(
            &standard(),
            &request("demo", "w1", None, &["coding", "frontend"]),
        )
        .unwrap();
        assert_eq!(
            decision.resolved.as_ref().map(|r| r.as_str()),
            Some("engineer")
        );
        assert_eq!(decision.reason, Some(RouteReason::PartialDomainMatch));
        assert_eq!(decision.confidence, Some(RouteConfidence::Heuristic));
    }

    #[test]
    fn ties_break_on_lexicographic_role_order() {
        let configured = team(vec![
            member("lead", RoleFunction::LeadOrchestrator, &["coordination"]),
            member("zeta", RoleFunction::GeneralExecution, &["coding"]),
            member("alpha", RoleFunction::GeneralExecution, &["coding"]),
        ]);
        let decision =
            resolve_route(&configured, &request("demo", "w1", None, &["coding"])).unwrap();
        assert_eq!(
            decision.resolved.as_ref().map(|r| r.as_str()),
            Some("alpha")
        );
    }

    #[test]
    fn lead_only_domain_coverage_is_not_a_route() {
        let configured = team(vec![
            member("lead", RoleFunction::LeadOrchestrator, &["coding"]),
            member("engineer", RoleFunction::GeneralExecution, &["backend"]),
        ]);
        let decision =
            resolve_route(&configured, &request("demo", "w1", None, &["coding"])).unwrap();
        match &decision.diagnosis {
            Some(RouteDiagnosis::NoExecutableRole { domains }) => {
                assert!(domains.contains("coding"));
            }
            other => panic!("unexpected diagnosis: {other:?}"),
        }
        decision.validate().unwrap();
    }

    #[test]
    fn members_without_execution_capability_are_not_routes() {
        let mut specialist = member("designer", RoleFunction::Specialist, &["coding"]);
        specialist.access.grants.remove(&Permission::ExecuteProcess);
        let configured = team(vec![
            member("lead", RoleFunction::LeadOrchestrator, &["coordination"]),
            member("engineer", RoleFunction::GeneralExecution, &["docs"]),
            specialist,
        ]);
        let decision =
            resolve_route(&configured, &request("demo", "w1", None, &["coding"])).unwrap();
        assert_eq!(decision.resolved, None);
    }

    #[test]
    fn repeated_calls_are_deterministic() {
        let configured = standard();
        let first = resolve_route(&configured, &request("demo", "w1", None, &["coding"])).unwrap();
        let second = resolve_route(&configured, &request("demo", "w1", None, &["coding"])).unwrap();
        assert_eq!(first, second);
        let serialized = serde_json::to_string(&first).unwrap();
        let deserialized: RouteDecision = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, first);
    }

    #[test]
    fn invalid_requests_are_rejected_before_routing() {
        let configured = standard();
        assert_eq!(
            resolve_route(&configured, &request("other", "w1", None, &["coding"])),
            Err(RouteError::ProjectMismatch)
        );
        assert_eq!(
            resolve_route(&configured, &request("demo", "w1", None, &[])).unwrap_err(),
            RouteError::InvalidRequest
        );
        assert_eq!(
            resolve_route(&configured, &request("demo", "w1", None, &["bad domain!"])).unwrap_err(),
            RouteError::InvalidRequest
        );
        let overflow: Vec<String> = (0..MAX_ROUTE_DOMAINS + 1)
            .map(|n| format!("d{n}"))
            .collect();
        let overflow: BTreeSet<String> = overflow.into_iter().collect();
        assert_eq!(
            resolve_route(
                &configured,
                &RouteRequest {
                    project_id: ProjectId::new("demo").unwrap(),
                    work_id: WorkId::Request(RequestId::new("w1").unwrap()),
                    requested: None,
                    domains: overflow,
                },
            )
            .unwrap_err(),
            RouteError::InvalidRequest
        );
    }
}
