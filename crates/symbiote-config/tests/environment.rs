use std::collections::BTreeSet;
use symbiote_config::environment::*;
use symbiote_domain::*;

fn reference(s: &str) -> ResourceRef {
    ResourceRef::new(s).unwrap()
}
fn resource() -> EnvironmentResource {
    EnvironmentResource {
        id: reference("tool-a"),
        kind: ResourceKind::Tool,
        payload_ref: reference("catalog-a"),
        version_ref: reference("v1"),
        compatibility_ref: reference("compat-v1"),
        requirement: Requirement::Required,
        ownership: Ownership::SymbioteManaged,
        trust: Trust::Approved,
        lifecycle: Lifecycle::Desired,
        hook_intent: None,
        required_grants: BTreeSet::from([reference("read")]),
    }
}
fn fixture() -> (EnvironmentDocument, ResolutionTarget, ResolutionFacts) {
    let target = ResolutionTarget {
        project_id: ProjectId::new("project-a").unwrap(),
        user_id: UserId::new("user").unwrap(),
        role_id: Some(RoleId::new("lead").unwrap()),
        binding_id: Some(BindingId::new("binding-a").unwrap()),
        task_id: Some(TaskId::new("task-a").unwrap()),
        dispatch_id: Some(DispatchId::new("dispatch-a").unwrap()),
        profile_id: RuntimeProfileId::new("native").unwrap(),
        host_id: HostId::new("host").unwrap(),
        runtime_kind: RuntimeKind::NativeSymbiote,
    };
    let doc = EnvironmentDocument {
        schema_version: ENVIRONMENT_SCHEMA_VERSION,
        project_id: target.project_id.clone(),
        mode: EnvironmentMode::ManagedIsolated,
        core_policies: BTreeSet::from([reference("constitution")]),
        layers: vec![EnvironmentLayer {
            target: TargetSelector::Project(target.project_id.clone()),
            source: reference("project"),
            revision: reference("v1"),
            resources: vec![resource()],
        }],
        extensions: Default::default(),
    };
    let facts = ResolutionFacts {
        target: target.clone(),
        mode: doc.mode,
        mode_supported: true,
        resources: vec![ResourceFact {
            resource: resource(),
            supported: true,
            feasible: true,
        }],
        approved_core_policies: doc.core_policies.clone(),
        approved_extensions: Default::default(),
        grants: BTreeSet::from([reference("read")]),
        parent_grants: None,
    };
    (doc, target, facts)
}

#[test]
fn native_external_share_resolution_with_explicit_facts() {
    let (doc, mut target, mut facts) = fixture();
    assert!(doc.resolve(&target, &facts).unwrap().blockers.is_empty());
    target.runtime_kind = RuntimeKind::ExternalHarness;
    assert_eq!(
        doc.resolve(&target, &facts),
        Err(EnvironmentError::InvalidSelector)
    );
    facts.target = target.clone();
    assert!(doc.resolve(&target, &facts).unwrap().blockers.is_empty());
}

#[test]
fn precedence_and_provenance_do_not_depend_on_input_order() {
    let (mut doc, target, mut facts) = fixture();
    doc.layers[0].resources[0].requirement = Requirement::Optional;
    let mut role = doc.layers[0].clone();
    role.target = TargetSelector::Role(target.role_id.clone().unwrap());
    role.source = reference("role");
    role.resources[0] = resource();
    doc.layers.push(role);
    facts.resources[0].resource = resource();
    let first = doc.resolve(&target, &facts).unwrap();
    doc.layers.reverse();
    assert_eq!(first, doc.resolve(&target, &facts).unwrap());
    assert_eq!(first.resources[&reference("tool-a")].provenance.len(), 2);
    assert!(first.blockers.is_empty());
}

#[test]
fn project_role_binding_profile_isolation() {
    let (mut doc, mut target, mut facts) = fixture();
    for selector in [
        TargetSelector::Role(RoleId::new("other").unwrap()),
        TargetSelector::Binding(BindingId::new("other").unwrap()),
        TargetSelector::RuntimeProfile(RuntimeProfileId::new("other").unwrap()),
    ] {
        let mut layer = doc.layers[0].clone();
        layer.target = selector;
        layer.resources[0].trust = Trust::Quarantined;
        doc.layers.push(layer);
    }
    assert!(doc.resolve(&target, &facts).unwrap().blockers.is_empty());
    target.project_id = ProjectId::new("other").unwrap();
    facts.target = target.clone();
    assert_eq!(
        doc.resolve(&target, &facts),
        Err(EnvironmentError::WrongProject)
    );
}

#[test]
fn duplicate_and_same_scope_conflicts_never_silently_win() {
    let (mut doc, target, facts) = fixture();
    doc.layers.push(doc.layers[0].clone());
    assert_eq!(doc.validate(), Err(EnvironmentError::DuplicateLayer));
    doc.layers[1].source = reference("another-source");
    doc.layers[1].resources[0].payload_ref = reference("other");
    let first = doc.resolve(&target, &facts).unwrap();
    assert!(
        first
            .blockers
            .iter()
            .any(|b| b.reason == BlockReason::Conflicted)
    );
    doc.layers.reverse();
    assert_eq!(first, doc.resolve(&target, &facts).unwrap());
}

#[test]
fn required_constraints_and_core_policy_cannot_be_overridden() {
    let (mut doc, target, facts) = fixture();
    let mut layer = doc.layers[0].clone();
    layer.target = TargetSelector::Role(target.role_id.clone().unwrap());
    layer.resources[0].requirement = Requirement::Disabled;
    doc.layers.push(layer);
    let plan = doc.resolve(&target, &facts).unwrap();
    assert!(
        plan.blockers
            .iter()
            .any(|b| b.reason == BlockReason::RequiredConstraint)
    );
    assert_eq!(
        plan.resources[&reference("tool-a")].resource.requirement,
        Requirement::Required
    );
    doc.layers[0].resources[0].id = reference("constitution");
    assert_eq!(doc.validate(), Err(EnvironmentError::InvalidDocument));
}

#[test]
fn required_resource_health_and_exact_compatibility_block() {
    let (base, target, facts) = fixture();
    for (trust, lifecycle, reason) in [
        (
            Trust::Quarantined,
            Lifecycle::Desired,
            BlockReason::Quarantined,
        ),
        (Trust::Approved, Lifecycle::Drifted, BlockReason::Drifted),
        (
            Trust::Approved,
            Lifecycle::Conflicted,
            BlockReason::Conflicted,
        ),
    ] {
        let mut doc = base.clone();
        doc.layers[0].resources[0].trust = trust;
        doc.layers[0].resources[0].lifecycle = lifecycle;
        assert!(
            doc.resolve(&target, &facts)
                .unwrap()
                .blockers
                .iter()
                .any(|b| b.reason == reason)
        );
    }
    let mut unsupported = facts.clone();
    unsupported.resources[0].supported = false;
    assert!(
        base.resolve(&target, &unsupported)
            .unwrap()
            .blockers
            .iter()
            .any(|b| b.reason == BlockReason::Unsupported)
    );
    let mut changed = base;
    changed.layers[0].resources[0].version_ref = reference("v2");
    assert!(
        changed
            .resolve(&target, &facts)
            .unwrap()
            .blockers
            .iter()
            .any(|b| b.reason == BlockReason::MissingFacts)
    );
}

#[test]
fn child_grants_must_be_subset_and_resources_do_not_grant_authority() {
    let (doc, target, mut facts) = fixture();
    facts.parent_grants = Some(BTreeSet::new());
    assert!(
        doc.resolve(&target, &facts)
            .unwrap()
            .blockers
            .iter()
            .any(|b| b.reason == BlockReason::ChildAuthorityExpansion)
    );
    facts.parent_grants = Some(facts.grants.clone());
    assert!(doc.resolve(&target, &facts).unwrap().blockers.is_empty());
    facts.grants.clear();
    assert!(
        doc.resolve(&target, &facts)
            .unwrap()
            .blockers
            .iter()
            .any(|b| b.reason == BlockReason::MissingGrant)
    );
}

#[test]
fn strict_bounded_metadata_roundtrip_and_unknown_extensions() {
    let (mut doc, _, _) = fixture();
    doc.extensions
        .insert(reference("vendor.future"), reference("catalog-extension"));
    let serialized = serde_json::to_string(&doc).unwrap();
    assert_eq!(EnvironmentDocument::parse(&serialized).unwrap(), doc);
    assert!(
        EnvironmentDocument::parse(&serialized.replacen("{", "{\"secret\":\"hidden\",", 1))
            .is_err()
    );
    assert!(
        EnvironmentDocument::parse(&serialized.replacen("{", "{\"schema_version\":1,", 1)).is_err()
    );
    for invalid in [
        "/tmp/file",
        "https://example.test",
        "run --command",
        "x\nsecret",
        "",
    ] {
        assert!(ResourceRef::new(invalid).is_err());
    }
    doc.schema_version = 2;
    assert_eq!(doc.validate(), Err(EnvironmentError::InvalidVersion));
}

#[test]
fn target_parser_rejects_duplicates_unknown_fields_and_oversize() {
    let (_, target, _) = fixture();
    let encoded = serde_json::to_string(&target).unwrap();
    assert_eq!(ResolutionTarget::parse(&encoded).unwrap(), target);
    assert!(
        ResolutionTarget::parse(&encoded.replacen("{", "{\"project_id\":\"project-a\",", 1))
            .is_err()
    );
    assert!(
        ResolutionTarget::parse(&encoded.replacen("{", "{\"principal\":\"host\",", 1)).is_err()
    );
    assert!(ResolutionTarget::parse(&format!("{}{}", " ".repeat(16_384), encoded)).is_err());
}

#[test]
fn optional_disabled_and_inherited_have_explicit_eligibility() {
    let (mut doc, target, mut facts) = fixture();
    doc.layers[0].resources[0].requirement = Requirement::Optional;
    facts.resources.clear();
    let plan = doc.resolve(&target, &facts).unwrap();
    assert!(plan.blockers.is_empty());
    assert!(!plan.resources[&reference("tool-a")].is_eligible());
    doc.layers[0].resources[0].requirement = Requirement::Disabled;
    let plan = doc.resolve(&target, &facts).unwrap();
    assert!(plan.blockers.is_empty());
    assert!(!plan.resources[&reference("tool-a")].is_eligible());
    doc.layers[0].resources[0].requirement = Requirement::Inherited;
    assert!(
        doc.resolve(&target, &facts)
            .unwrap()
            .blockers
            .iter()
            .any(|b| b.reason == BlockReason::MissingInheritance)
    );
}

#[test]
fn mode_and_host_feasibility_require_explicit_proof() {
    let (mut doc, target, mut facts) = fixture();
    facts.mode_supported = false;
    assert!(
        doc.resolve(&target, &facts)
            .unwrap()
            .blockers
            .iter()
            .any(|b| b.reason == BlockReason::UnsupportedMode)
    );
    facts.mode_supported = true;
    doc.mode = EnvironmentMode::Ephemeral;
    assert_eq!(
        doc.resolve(&target, &facts),
        Err(EnvironmentError::InvalidSelector)
    );
    facts.mode = doc.mode;
    facts.resources[0].feasible = false;
    assert!(
        doc.resolve(&target, &facts)
            .unwrap()
            .blockers
            .iter()
            .any(|b| b.reason == BlockReason::Infeasible)
    );
    facts.resources[0].feasible = true;
    assert!(doc.resolve(&target, &facts).unwrap().resources[&reference("tool-a")].is_eligible());
}
