use serde_json::json;
use symbiote_config::*;

fn manifest() -> ProjectManifest {
    ProjectManifest::parse(r#"{"schema_version":1,"project_id":"project-a"}"#).unwrap()
}
fn context(project: &str, controller: &str) -> Context {
    Context {
        project: project.into(),
        root: None,
        repository: None,
        role: None,
        controller: controller.into(),
    }
}
fn layer(scope: Scope, c: Option<Context>, learning: bool) -> Layer {
    Layer {
        schema_version: 1,
        scope,
        source: format!("{scope:?}"),
        revision: "1".into(),
        context: c,
        settings: Settings {
            learning: Some(learning),
            ..Settings::default()
        },
    }
}

#[test]
fn partial_manifest_opens_without_harness_or_authentication() {
    let m = manifest();
    assert!(m.roles.is_empty());
    assert_eq!(
        ProjectManifest::parse(&m.canonical_json().unwrap()).unwrap(),
        m
    );
}

#[test]
fn versions_never_silently_coerce_in_either_direction() {
    for (from, to) in [(0, 1), (2, 1), (1, 0), (1, 2)] {
        let original = json!({"schema_version":from,"project_id":"project-a","future":42});
        let preview = preview_migration(original.clone(), to).unwrap();
        assert!(!preview.supported);
        assert_eq!(preview.original, original);
    }
    let original = serde_json::to_value(manifest()).unwrap();
    assert!(preview_migration(original, 1).unwrap().changes.is_empty());
    assert!(ProjectManifest::parse(r#"{"schema_version":2,"project_id":"a"}"#).is_err());
}

#[test]
fn portable_core_rejects_machine_auth_and_unknown_keys() {
    for key in ["api_key", "credentials", "machine", "sessions", "unknown"] {
        let mut v = serde_json::to_value(manifest()).unwrap();
        v[key] = json!("sensitive-placeholder");
        assert!(ProjectManifest::parse(&v.to_string()).is_err());
    }
    for path in ["/home/user", "C:\\repo", "../repo", "a/../b", "a//b"] {
        let mut m = manifest();
        m.roots.insert("main".into(), path.into());
        assert!(m.validate().is_err());
    }
}

#[test]
fn portable_roots_reject_cross_platform_ambiguous_components() {
    for path in [
        "repo\0hidden",
        "repo\nhidden",
        "repo\u{7f}hidden",
        "CON",
        "con.txt",
        "src/NuL.json",
        "PRN",
        "AUX",
        "COM1",
        "LPT9.log",
        "COM¹.txt",
        "LPT²",
        "CONIN$",
        "CONOUT$",
        "a.",
        "a ",
        "a./child",
        "a /child",
        "con .txt",
        "a<b",
        "a>b",
        "a:b",
        "a\"b",
        "a|b",
        "a?b",
        "a*b",
        "a\\b",
        "a/./b",
        "a/../b",
        "",
        "/absolute",
        "a/",
    ] {
        let mut m = manifest();
        m.roots.insert("main".into(), path.into());
        assert!(m.validate().is_err(), "accepted {path:?}");
        assert!(ProjectManifest::parse(&serde_json::to_string(&m).unwrap()).is_err());
    }
    for path in [
        ".",
        "repo",
        "src/project",
        ".hidden",
        "my project",
        "café",
        "COM10",
        "auxiliary",
    ] {
        let mut m = manifest();
        m.roots.insert("main".into(), path.into());
        assert!(m.validate().is_ok(), "rejected {path:?}");
    }
}

#[test]
fn extension_roundtrip_is_deterministic_and_managed_fields_stay_separate() {
    let mut m = manifest();
    m.extensions
        .insert("org.example".into(), json!({"unknown":[1,{"future":true}]}));
    m.managed.insert(
        "/graph_profile_ref".into(),
        ManagedField {
            source: "importer".into(),
            revision: "1".into(),
        },
    );
    let first = m.canonical_json().unwrap();
    assert_eq!(
        ProjectManifest::parse(&first)
            .unwrap()
            .canonical_json()
            .unwrap(),
        first
    );
    assert!(m.name.is_none());
}

#[test]
fn projects_and_controllers_are_isolated_and_restore_is_presentation_only() {
    let a = context("a", "controller-1");
    let b = context("b", "controller-2");
    let layers = vec![
        layer(Scope::Application, None, false),
        layer(Scope::Project, Some(a.clone()), true),
        layer(Scope::Project, Some(b.clone()), false),
    ];
    assert_eq!(resolve(&layers, &a).unwrap().settings.learning, Some(true));
    assert_eq!(resolve(&layers, &b).unwrap().settings.learning, Some(false));
    let mut controller_a = ControllerState::default();
    let mut controller_b = ControllerState::default();
    controller_a.switch_project("a".into());
    controller_b.switch_project("b".into());
    let saved = serde_json::to_string(&controller_a).unwrap();
    controller_a.switch_project("b".into());
    let restored: ControllerState = serde_json::from_str(&saved).unwrap();
    assert_eq!(restored.visible_project.as_deref(), Some("a"));
    assert_eq!(controller_b.visible_project.as_deref(), Some("b"));
    assert_eq!(resolve(&layers, &a).unwrap().settings.learning, Some(true));
}

#[test]
fn local_override_is_explicit_and_specific_to_controller() {
    let a = context("a", "controller-1");
    let b = context("a", "controller-2");
    let layers = [
        layer(Scope::Project, Some(a.clone()), true),
        layer(Scope::LocalOverride, Some(a.clone()), false),
    ];
    let resolved = resolve(&layers, &a).unwrap();
    assert_eq!(resolved.settings.learning, Some(false));
    assert_eq!(resolved.provenance["learning"].len(), 2);
    assert_eq!(resolve(&layers, &b).unwrap().settings.learning, Some(true));
}

#[test]
fn equal_scope_conflicts_fail_independent_of_input_order() {
    let a = context("a", "one");
    let one = layer(Scope::Project, Some(a.clone()), true);
    let mut two = layer(Scope::Project, Some(a.clone()), false);
    two.source = "other".into();
    assert_eq!(
        resolve(&[one.clone(), two.clone()], &a),
        resolve(&[two, one], &a)
    );
    assert!(resolve(&[layer(Scope::Project, None, true)], &a).is_err());
}

#[test]
fn roles_sharing_runtime_keep_independent_resources() {
    let mut m = manifest();
    for (role, skill, kind) in [
        ("lead", "review", RuntimeKind::ExternalHarness),
        ("worker", "implement", RuntimeKind::ExternalHarness),
    ] {
        m.roles.insert(
            role.into(),
            RoleAssignment {
                runtime_kind: kind,
                runtime_profile_ref: "shared-codex".into(),
                provider_connection_ref: None,
                entitlement_ref: Some("subscription".into()),
                resources: Resources {
                    skills: [skill.into()].into(),
                    ..Resources::default()
                },
            },
        );
    }
    let original = m.roles["lead"].clone();
    m.roles.get_mut("worker").unwrap().runtime_kind = RuntimeKind::NativeSymbiote;
    assert_eq!(m.roles["lead"], original);
    assert_ne!(m.roles["lead"].resources, m.roles["worker"].resources);
    assert!(ProjectManifest::parse(&m.canonical_json().unwrap()).is_ok());
}

#[test]
fn merge_disjoint_changes_and_refuse_conflicts_and_delete_edit() {
    let base = manifest();
    let mut left = base.clone();
    let mut right = base.clone();
    left.name = Some("rename".into());
    right.graph_profile_ref = Some("graph".into());
    let result = merge(&base, &left, &right).unwrap().finish().unwrap();
    assert_eq!(result.name, left.name);
    assert_eq!(result.graph_profile_ref, right.graph_profile_ref);
    right.name = Some("different".into());
    let preview = merge(&base, &left, &right).unwrap();
    assert_eq!(preview.conflicts[0].path, "/name");
    assert!(preview.finish().is_err());
    let mut base = base;
    base.extensions.insert("org.example".into(), json!({"x":1}));
    let mut left = base.clone();
    left.extensions.clear();
    let mut right = base.clone();
    right
        .extensions
        .insert("org.example".into(), json!({"x":2}));
    assert!(merge(&base, &left, &right).unwrap().finish().is_err());
}

#[test]
fn scope_storage_and_secrets_are_explicit() {
    assert_eq!(
        scope_storage(Scope::Project),
        StorageClass::VersionControlled
    );
    assert_eq!(scope_storage(Scope::LocalOverride), StorageClass::LocalOnly);
    assert_eq!(scope_storage(Scope::User), StorageClass::UserSynchronized);
    assert_eq!(SecretValue::STORAGE, StorageClass::ProhibitedFromDisk);
}

#[test]
fn conflicting_project_runtime_and_telemetry_intent_survives_rename() {
    let mut a = manifest();
    let mut b = manifest();
    b.project_id = "project-b".into();
    for (m, runtime, enabled) in [
        (&mut a, RuntimeKind::NativeSymbiote, false),
        (&mut b, RuntimeKind::ExternalHarness, true),
    ] {
        m.lead_role_ref = Some("lead".into());
        m.roles.insert(
            "lead".into(),
            RoleAssignment {
                runtime_kind: runtime,
                runtime_profile_ref: "profile".into(),
                provider_connection_ref: None,
                entitlement_ref: None,
                resources: Resources::default(),
            },
        );
        m.policies.preferences.learning = Some(enabled);
        m.policies.preferences.telemetry = Some(enabled);
    }
    let before_b = b.canonical_json().unwrap();
    a.name = Some("Renamed".into());
    let restored = ProjectManifest::parse(&a.canonical_json().unwrap()).unwrap();
    assert_eq!(restored.project_id, "project-a");
    assert_eq!(
        restored.roles["lead"].runtime_kind,
        RuntimeKind::NativeSymbiote
    );
    assert_eq!(restored.policies.preferences.telemetry, Some(false));
    assert_eq!(b.canonical_json().unwrap(), before_b);
}
