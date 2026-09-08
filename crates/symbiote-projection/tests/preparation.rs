use std::collections::BTreeMap;
use symbiote_config::{ProjectManifest, environment::*};
use symbiote_projection::{toml::*, *};

fn environment() -> EffectiveEnvironment {
    let manifest = ProjectManifest::parse(include_str!(
        "../../symbiote-config/fixtures/environment/project.json"
    ))
    .unwrap();
    let mut document = manifest.agent_environments["development"].clone();
    let target = ResolutionTarget::parse(include_str!(
        "../../symbiote-config/fixtures/environment/target.json"
    ))
    .unwrap();
    document.layers[0].resources[0].trust = Trust::Approved;
    let resource = document.layers[0].resources[0].clone();
    document
        .resolve(
            &target,
            &ResolutionFacts {
                target: target.clone(),
                mode: document.mode,
                mode_supported: true,
                resources: vec![ResourceFact {
                    resource: resource.clone(),
                    supported: true,
                    feasible: true,
                }],
                approved_core_policies: document.core_policies.clone(),
                approved_extensions: Default::default(),
                grants: resource.required_grants,
                parent_grants: None,
            },
        )
        .unwrap()
}
fn declaration(environment: &EffectiveEnvironment) -> TomlDeclaration {
    TomlDeclaration {
        resource: environment
            .resources
            .values()
            .next()
            .unwrap()
            .resource
            .clone(),
        filename: "config.toml".into(),
        changes: vec![ManagedChange {
            path: vec!["enabled".into()],
            before: Some(PrimitiveValue::Boolean(false)),
            desired: Some(PrimitiveValue::Boolean(true)),
            ownership: ManagedOwnership::SymbioteManaged,
        }],
    }
}
fn input(text: &str) -> BTreeMap<String, String> {
    BTreeMap::from([("config.toml".into(), text.into())])
}

#[test]
fn resolved_environment_prepares_comments_and_unmanaged_current_edits() {
    let environment = environment();
    let prepared = prepare_toml(
        &environment,
        &input("enabled = false\nuser = 1\n"),
        &input("# human header\nenabled = false # keep\nuser = 2\n"),
        &[declaration(&environment)],
    )
    .unwrap();
    let output = std::str::from_utf8(&prepared.files()["config.toml"]).unwrap();
    assert!(output.contains("# human header"));
    assert!(output.contains("enabled = true # keep"));
    assert!(output.contains("user = 2"));
}

#[test]
fn missing_or_stale_resource_mapping_never_silently_passes() {
    let environment = environment();
    let inputs = input("enabled = false\n");
    assert!(matches!(
        prepare_toml(&environment, &inputs, &inputs, &[]),
        Err(PreparationError::MissingMapping)
    ));
    let mut stale = declaration(&environment);
    stale.resource.version_ref = ResourceRef::new("stale-version").unwrap();
    assert!(matches!(
        prepare_toml(&environment, &inputs, &inputs, &[stale]),
        Err(PreparationError::IneligibleResource)
    ));
    let mut empty = declaration(&environment);
    empty.changes.clear();
    assert!(matches!(
        prepare_toml(&environment, &inputs, &inputs, &[empty]),
        Err(PreparationError::MissingMapping)
    ));
}

#[test]
fn global_blockers_and_concurrent_managed_changes_refuse_preparation() {
    let mut environment = environment();
    let mapping = declaration(&environment);
    let inputs = input("enabled = false\n");
    environment.blockers.push(EnvironmentBlocker {
        resource: None,
        reason: BlockReason::UnsupportedMode,
    });
    assert!(matches!(
        prepare_toml(&environment, &inputs, &inputs, &[mapping]),
        Err(PreparationError::BlockedEnvironment)
    ));
    environment.blockers.clear();
    assert!(matches!(
        prepare_toml(
            &environment,
            &inputs,
            &input("enabled = 3\n"),
            &[declaration(&environment)]
        ),
        Err(PreparationError::Reconciliation(
            ProjectionError::ConcurrentEdit
        ))
    ));
}

#[test]
fn generation_publication_is_inactive_and_never_overwrites_an_existing_profile() {
    use std::{
        fs,
        os::unix::fs::PermissionsExt,
        time::{SystemTime, UNIX_EPOCH},
    };
    let root = std::env::temp_dir().join(format!(
        "symbiote-preparation-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&root).unwrap();
    fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
    let environment = environment();
    let inputs = input("enabled = false # retain\n");
    let prepared =
        prepare_toml(&environment, &inputs, &inputs, &[declaration(&environment)]).unwrap();
    let receipt = generation::publish(&root, "generation-one", prepared.files()).unwrap();
    assert_eq!(
        generation::verify(&root, "generation-one").unwrap(),
        receipt
    );
    let file = root.join("generation-one/config.toml");
    fs::write(&file, "# concurrent human edit\n").unwrap();
    assert!(generation::publish(&root, "generation-one", prepared.files()).is_err());
    assert_eq!(
        fs::read_to_string(&file).unwrap(),
        "# concurrent human edit\n"
    );
    assert!(generation::verify(&root, "generation-one").is_err());
    fs::remove_dir_all(root).unwrap();
}
