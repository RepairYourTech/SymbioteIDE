use serde_json::{Value, json};
use symbiote_config::{ProjectManifest, merge, preview_migration};

fn document(project: &str) -> Value {
    json!({
        "schema_version":1,
        "project_id":project,
        "mode":"managed_isolated",
        "core_policies":[],
        "layers":[],
        "extensions":{}
    })
}

fn manifest() -> ProjectManifest {
    ProjectManifest::parse(
        &json!({
            "schema_version":1,"project_id":"project-a",
            "agent_environments":{"default":document("project-a")}
        })
        .to_string(),
    )
    .unwrap()
}

#[test]
fn environment_roundtrips_with_portable_project_and_existing_manifests() {
    let manifest = manifest();
    let encoded = manifest.canonical_json().unwrap();
    assert_eq!(ProjectManifest::parse(&encoded).unwrap(), manifest);
    assert!(
        preview_migration(serde_json::to_value(&manifest).unwrap(), 1)
            .unwrap()
            .supported
    );
    let legacy =
        ProjectManifest::parse(r#"{"schema_version":1,"project_id":"project-a"}"#).unwrap();
    assert!(legacy.agent_environments.is_empty());
    assert!(
        !legacy
            .canonical_json()
            .unwrap()
            .contains("agent_environments")
    );
}

#[test]
fn environments_cannot_cross_project_or_hide_credentials_in_core_fields() {
    for environment in [
        document("project-b"),
        {
            let mut value = document("project-a");
            value["credentials"] = json!({"api_key":"fixture-only"});
            value
        },
        {
            let mut value = document("project-a");
            value["schema_version"] = json!(2);
            value
        },
    ] {
        let value = json!({"schema_version":1,"project_id":"project-a","agent_environments":{"default":environment}});
        let error = ProjectManifest::parse(&value.to_string()).unwrap_err();
        assert!(!format!("{error:?}").contains("fixture-only"));
    }
}

#[test]
fn concurrent_environment_edits_require_conflict_resolution() {
    let base = manifest();
    let mut left = serde_json::to_value(&base).unwrap();
    let mut right = left.clone();
    left["agent_environments"]["default"]["mode"] = json!("adopt_existing");
    right["agent_environments"]["default"]["mode"] = json!("ephemeral");
    let left = ProjectManifest::parse(&left.to_string()).unwrap();
    let right = ProjectManifest::parse(&right.to_string()).unwrap();
    let preview = merge(&base, &left, &right).unwrap();
    assert_eq!(preview.conflicts.len(), 1);
    assert_eq!(
        preview.conflicts[0].path,
        "/agent_environments/default/mode"
    );
    assert!(preview.finish().is_err());
}

#[test]
fn duplicate_environment_names_are_rejected_before_map_collapse() {
    let env = document("project-a");
    let input = format!(
        r#"{{"schema_version":1,"project_id":"project-a","agent_environments":{{"default":{env},"default":{env}}}}}"#
    );
    assert!(ProjectManifest::parse(&input).is_err());
}

#[test]
fn canonical_serialization_refuses_output_that_its_parser_cannot_read() {
    let compact = json!({
        "schema_version":1,"project_id":"project-a",
        "extensions":{"org.example":vec![0;200_000]}
    })
    .to_string();
    assert!(compact.len() < symbiote_config::MAX_MANIFEST_BYTES);
    let manifest = ProjectManifest::parse(&compact).unwrap();
    assert!(manifest.canonical_json().is_err());
}

#[test]
fn published_inspector_inputs_are_valid_and_carry_no_observed_authority() {
    use symbiote_config::environment::*;
    let manifest =
        ProjectManifest::parse(include_str!("../fixtures/environment/project.json")).unwrap();
    let target =
        ResolutionTarget::parse(include_str!("../fixtures/environment/target.json")).unwrap();
    let document = &manifest.agent_environments["development"];
    let plan = document
        .resolve(
            &target,
            &ResolutionFacts {
                target: target.clone(),
                mode: document.mode,
                mode_supported: false,
                resources: vec![],
                approved_core_policies: Default::default(),
                approved_extensions: Default::default(),
                grants: Default::default(),
                parent_grants: None,
            },
        )
        .unwrap();
    assert!(
        plan.blockers
            .iter()
            .any(|issue| issue.reason == BlockReason::UnsupportedMode)
    );
    assert!(
        plan.blockers
            .iter()
            .any(|issue| issue.reason == BlockReason::MissingGrant)
    );
    assert!(
        plan.resources
            .values()
            .all(|resource| !resource.is_eligible())
    );
}
