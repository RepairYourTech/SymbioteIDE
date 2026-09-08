use std::collections::{BTreeMap, BTreeSet};
use symbiote_domain::*;
fn role_id(name: &str) -> RoleId {
    RoleId::new(name).unwrap()
}
fn ceiling() -> AccessSnapshot {
    AccessSnapshot {
        project_id: ProjectId::new("project").unwrap(),
        roots: BTreeSet::from([RootId::new("root").unwrap()]),
        grants: BTreeSet::from([
            Permission::ReadRoot,
            Permission::ExecuteProcess,
            Permission::MutateStream,
        ]),
        policy_revision: Revision(1),
    }
}
fn policy(template: RoleTemplate, name: &str, reviewer: &str) -> RolePolicy {
    let mut member = template.policy(role_id(name), ceiling());
    member.context_policy_ref = "context-v1".into();
    member.tool_policy_ref = "tools-v1".into();
    member.skill_policy_ref = "skills-v1".into();
    member.execution_policy_ref = "execution-v1".into();
    member.independent_reviewers.insert(role_id(reviewer));
    member
}
fn team() -> TeamConfiguration {
    TeamConfiguration {
        schema_version: 1,
        project_id: ProjectId::new("project").unwrap(),
        revision: Revision(0),
        lead_role_id: role_id("lead"),
        members: vec![
            policy(RoleTemplate::LeadOrchestrator, "lead", "engineer"),
            policy(RoleTemplate::GeneralExecution, "engineer", "lead"),
        ],
        access_ceiling: ceiling(),
    }
}
fn roles() -> Vec<Role> {
    ["lead", "engineer"]
        .into_iter()
        .map(|name| Role {
            id: role_id(name),
            project_id: ProjectId::new("project").unwrap(),
            revision: Revision(0),
            name: format!("Custom {name}"),
            operating_contract: VersionedRoleContract {
                id: RoleContractId::new(format!("{name}-contract")).unwrap(),
                revision: Revision(1),
            },
        })
        .collect()
}

#[test]
fn strict_team_resolves_existing_roles_and_roundtrips_without_runtime_binding() {
    let team = team();
    team.validate_roles(&roles()).unwrap();
    team.autonomy_structure().unwrap();
    let json = serde_json::to_string(&team).unwrap();
    assert_eq!(TeamConfiguration::parse(&json).unwrap(), team);
    assert!(!json.contains("runtime"));
    assert!(!json.contains("Custom engineer"));
    assert_eq!(
        team.referenced_roles(),
        BTreeSet::from([role_id("lead"), role_id("engineer")])
    );
    let mut renamed = roles();
    renamed[1].name = "Custom implementation specialist".into();
    team.validate_roles(&renamed).unwrap();
}
#[test]
fn missing_or_impossible_staffing_and_unreviewed_roles_are_rejected() {
    let mut configured = team();
    configured.members.pop();
    assert_eq!(configured.validate(), Err(TeamError::MissingExecutionRole));
    let mut configured = team();
    configured.members[1].function = RoleFunction::Specialist;
    assert_eq!(configured.validate(), Err(TeamError::MissingExecutionRole));
    let mut configured = team();
    configured.lead_role_id = role_id("missing");
    assert_eq!(configured.validate(), Err(TeamError::MissingLead));
    let mut configured = team();
    configured.members[1].independent_reviewers.clear();
    assert_eq!(configured.validate(), Err(TeamError::UnreviewedRole));
    let mut configured = team();
    configured.members[1].independent_reviewers = BTreeSet::from([role_id("engineer")]);
    assert_eq!(configured.validate(), Err(TeamError::SelfReview));
    let mut configured = team();
    configured.members[1].independent_reviewers = BTreeSet::from([role_id("missing")]);
    assert_eq!(configured.validate(), Err(TeamError::DanglingRole));
    let mut configured = team();
    configured.members.push(configured.members[0].clone());
    assert_eq!(configured.validate(), Err(TeamError::DuplicateRole));
}
#[test]
fn permissions_and_reviewer_root_coverage_cannot_escalate() {
    let mut configured = team();
    configured.members[1]
        .access
        .grants
        .insert(Permission::UseCredential);
    assert_eq!(configured.validate(), Err(TeamError::PrivilegeEscalation));
    let mut configured = team();
    configured.members[1].access.policy_revision = Revision(2);
    assert_eq!(configured.validate(), Err(TeamError::PrivilegeEscalation));
    let mut configured = team();
    configured.members[1].access.project_id = ProjectId::new("other").unwrap();
    assert_eq!(configured.validate(), Err(TeamError::ProjectMismatch));
    let mut configured = team();
    configured
        .access_ceiling
        .roots
        .insert(RootId::new("second-root").unwrap());
    configured.members[1].access.roots = configured.access_ceiling.roots.clone();
    assert_eq!(configured.validate(), Err(TeamError::UnreviewedRole));
    let mut configured = team();
    configured.members[0].access.grants = BTreeSet::from([Permission::ReadRoot]);
    configured.validate().unwrap();
    // A fallback does not inherit or acquire a more privileged replacement's policy.
    configured.members[0].fallbacks.push(RoleFallback {
        role_id: role_id("engineer"),
        consent: FallbackConsent::ExplicitRequired,
    });
    assert_eq!(configured.validate(), Err(TeamError::PrivilegeEscalation));
}
#[test]
fn fallback_chains_are_ordered_explicit_bounded_and_acyclic() {
    let mut configured = team();
    configured.members[1].fallbacks.push(RoleFallback {
        role_id: role_id("lead"),
        consent: FallbackConsent::ExplicitRequired,
    });
    configured.validate().unwrap();
    let mut cyclic = configured.clone();
    cyclic.members[0].fallbacks.push(RoleFallback {
        role_id: role_id("engineer"),
        consent: FallbackConsent::ExplicitRequired,
    });
    assert_eq!(cyclic.validate(), Err(TeamError::CyclicFallback));
    let mut duplicate = configured.clone();
    let repeated = duplicate.members[1].fallbacks[0].clone();
    duplicate.members[1].fallbacks.push(repeated);
    assert_eq!(duplicate.validate(), Err(TeamError::InvalidFallback));
    let mut dangling = team();
    dangling.members[1].fallbacks.push(RoleFallback {
        role_id: role_id("absent"),
        consent: FallbackConsent::ExplicitRequired,
    });
    assert_eq!(dangling.validate(), Err(TeamError::DanglingRole));
    let mut self_fallback = team();
    self_fallback.members[1].fallbacks.push(RoleFallback {
        role_id: role_id("engineer"),
        consent: FallbackConsent::ExplicitRequired,
    });
    assert_eq!(self_fallback.validate(), Err(TeamError::InvalidFallback));
    let mut json = serde_json::to_value(configured).unwrap();
    json["members"][1]["fallbacks"][0]["consent"] = serde_json::json!("automatic");
    assert!(serde_json::from_value::<TeamConfiguration>(json).is_err());
}
#[test]
fn canonical_role_references_reject_dangling_cross_project_and_bad_names() {
    let configured = team();
    assert_eq!(
        configured.validate_roles(&roles()[..1]),
        Err(TeamError::DanglingRole)
    );
    let mut other = roles();
    other[1].project_id = ProjectId::new("other").unwrap();
    assert_eq!(
        configured.validate_roles(&other),
        Err(TeamError::ProjectMismatch)
    );
    let mut invalid = roles();
    invalid[1].name = "\n".into();
    assert_eq!(
        configured.validate_roles(&invalid),
        Err(TeamError::InvalidPolicy)
    );
}
#[test]
fn all_templates_remain_customizable_and_require_explicit_policy_configuration() {
    let templates = [
        RoleTemplate::LeadOrchestrator,
        RoleTemplate::GeneralExecution,
        RoleTemplate::Architect,
        RoleTemplate::Backend,
        RoleTemplate::Frontend,
        RoleTemplate::Qa,
        RoleTemplate::Reviewer,
        RoleTemplate::Security,
        RoleTemplate::Documentation,
        RoleTemplate::Infrastructure,
    ];
    let mut domains = BTreeMap::new();
    for template in templates {
        let p = template.policy(role_id("custom"), ceiling());
        assert_eq!(p.role_id, role_id("custom"));
        assert!(!p.responsibilities.is_empty());
        assert!(p.context_policy_ref.is_empty());
        assert!(p.independent_reviewers.is_empty());
        domains.insert(format!("{template:?}"), p.task_domains);
    }
    assert_eq!(domains.len(), 10);
}
#[test]
fn schema_unknown_fields_limits_and_invalid_references_fail_closed() {
    let configured = team();
    let mut json = serde_json::to_value(&configured).unwrap();
    json["members"][0]["runtime_profile"] = serde_json::json!("forged");
    assert!(serde_json::from_value::<TeamConfiguration>(json).is_err());
    let mut configured = team();
    configured.schema_version = 2;
    assert_eq!(configured.validate(), Err(TeamError::UnsupportedVersion));
    let mut configured = team();
    configured.members[1].tool_policy_ref = "/home/user/private".into();
    assert_eq!(configured.validate(), Err(TeamError::InvalidPolicy));
    let mut configured = team();
    configured.members = vec![configured.members[0].clone(); 33];
    assert_eq!(configured.validate(), Err(TeamError::ResourceLimit));
    assert!(TeamConfiguration::parse(&"x".repeat(65_537)).is_err());
}
