use std::{
    collections::BTreeMap,
    ffi::OsString,
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};
use symbiote_domain::*;
use symbiote_runtime_discovery::{
    ConfigRootSource, ObservationSource, PROFILE_VERSION, ProbeProvenance, ProfileError,
    ProfileObservation, ProfileSpec, resolve_profiles,
};

static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);

fn home() -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "symbiote-profiles-{}-{}",
        std::process::id(),
        NEXT_ROOT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&path).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
    path
}

fn spec(
    profile: &str,
    identity: &str,
    variable: &str,
    default: &str,
    account: Option<&str>,
) -> ProfileSpec {
    ProfileSpec {
        profile_id: RuntimeProfileId::new(profile).unwrap(),
        instance_name: format!("{profile} instance"),
        config_identity: identity.into(),
        environment_variable: variable.into(),
        default_relative_path: default.into(),
        account_ref: account.map(str::to_owned),
    }
}

fn provenance() -> ProbeProvenance {
    ProbeProvenance {
        probe_id: CommandId::new("profile-probe").unwrap(),
        source: ObservationSource::TrustedHostProbe,
        adapter_revision: "v1".into(),
    }
}

fn environment(entries: &[(&str, &Path)]) -> BTreeMap<String, OsString> {
    entries
        .iter()
        .map(|(name, path)| ((*name).to_owned(), path.as_os_str().to_owned()))
        .collect()
}

#[test]
fn named_profiles_resolve_distinct_roots_without_mutating_environment_or_home() {
    let root = home();
    let default_path = root.join(".codex");
    let override_path = root.join("work-codex");
    let env = environment(&[("CODEX_WORK_HOME", &override_path)]);
    let before = env.clone();
    let resolved = resolve_profiles(
        &[
            spec("personal", "config_personal", "CODEX_HOME", ".codex", None),
            spec(
                "work",
                "config_work",
                "CODEX_WORK_HOME",
                "work/codex",
                Some("account_work"),
            ),
        ],
        &env,
        &root,
        Timestamp(10),
        Timestamp(20),
        provenance(),
    )
    .unwrap();
    assert_eq!(resolved.len(), 2);
    assert_eq!(resolved[0].path, default_path.to_str().unwrap());
    assert_eq!(resolved[0].source, ConfigRootSource::ProfileDefault);
    assert_eq!(resolved[1].path, override_path.to_str().unwrap());
    assert_eq!(resolved[1].source, ConfigRootSource::EnvironmentOverride);
    assert_eq!(resolved[1].account_ref.as_deref(), Some("account_work"));
    assert_eq!(env, before);
    assert!(!default_path.exists());
    assert!(!override_path.exists());
    let debug = format!("{resolved:?}");
    assert!(!debug.contains(default_path.to_str().unwrap()));
    let encoded = serde_json::to_string(&resolved).unwrap();
    assert_eq!(
        serde_json::from_str::<Vec<ProfileObservation>>(&encoded).unwrap(),
        resolved
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn invalid_profile_specs_duplicates_and_shared_roots_are_named_refusals() {
    let root = home();
    let valid = spec("one", "config_one", "ONE_HOME", ".one", None);
    let mut invalid = valid.clone();
    invalid.instance_name = " \n".into();
    assert_eq!(
        resolve_profiles(
            &[invalid],
            &BTreeMap::new(),
            &root,
            Timestamp(10),
            Timestamp(20),
            provenance(),
        ),
        Err(ProfileError::InvalidSpec)
    );
    let mut invalid = valid.clone();
    invalid.config_identity = "config/unsafe".into();
    assert_eq!(
        resolve_profiles(
            &[invalid],
            &BTreeMap::new(),
            &root,
            Timestamp(10),
            Timestamp(20),
            provenance(),
        ),
        Err(ProfileError::InvalidSpec)
    );
    let mut invalid = valid.clone();
    invalid.environment_variable = "one_home".into();
    assert_eq!(
        resolve_profiles(
            &[invalid],
            &BTreeMap::new(),
            &root,
            Timestamp(10),
            Timestamp(20),
            provenance(),
        ),
        Err(ProfileError::InvalidSpec)
    );
    let mut invalid = valid.clone();
    invalid.default_relative_path = "/absolute".into();
    assert_eq!(
        resolve_profiles(
            &[invalid],
            &BTreeMap::new(),
            &root,
            Timestamp(10),
            Timestamp(20),
            provenance(),
        ),
        Err(ProfileError::InvalidSpec)
    );
    let mut invalid = valid.clone();
    invalid.default_relative_path = "../escape".into();
    assert_eq!(
        resolve_profiles(
            &[invalid],
            &BTreeMap::new(),
            &root,
            Timestamp(10),
            Timestamp(20),
            provenance(),
        ),
        Err(ProfileError::InvalidSpec)
    );
    let mut invalid = valid.clone();
    invalid.account_ref = Some("someone@example.test".into());
    assert_eq!(
        resolve_profiles(
            &[invalid],
            &BTreeMap::new(),
            &root,
            Timestamp(10),
            Timestamp(20),
            provenance(),
        ),
        Err(ProfileError::InvalidSpec)
    );
    assert_eq!(
        resolve_profiles(
            &[valid.clone(), valid.clone()],
            &BTreeMap::new(),
            &root,
            Timestamp(10),
            Timestamp(20),
            provenance(),
        ),
        Err(ProfileError::DuplicateProfile)
    );
    assert_eq!(
        resolve_profiles(
            &[
                valid.clone(),
                spec("two", "config_one", "TWO_HOME", ".two", None),
            ],
            &BTreeMap::new(),
            &root,
            Timestamp(10),
            Timestamp(20),
            provenance(),
        ),
        Err(ProfileError::DuplicateConfig)
    );
    assert_eq!(
        resolve_profiles(
            &[
                valid.clone(),
                spec("two", "config_two", "ONE_HOME", ".two", None),
            ],
            &BTreeMap::new(),
            &root,
            Timestamp(10),
            Timestamp(20),
            provenance(),
        ),
        Err(ProfileError::DuplicateEnvironment)
    );
    assert_eq!(
        resolve_profiles(
            &[
                valid.clone(),
                spec("two", "config_two", "TWO_HOME", ".one", None),
            ],
            &environment(&[("TWO_HOME", &root.join("other-default"))]),
            &root,
            Timestamp(10),
            Timestamp(20),
            provenance(),
        ),
        Err(ProfileError::DuplicatePath)
    );
    let shared = root.join("shared");
    assert_eq!(
        resolve_profiles(
            &[
                spec("one", "config_one", "ONE_HOME", "one", None),
                spec("two", "config_two", "TWO_HOME", "two", None),
            ],
            &environment(&[("ONE_HOME", &shared), ("TWO_HOME", &shared)]),
            &root,
            Timestamp(10),
            Timestamp(20),
            provenance(),
        ),
        Err(ProfileError::DuplicatePath)
    );
    let trailing = PathBuf::from(format!("{}/", shared.display()));
    assert_eq!(
        resolve_profiles(
            &[
                spec("one", "config_one", "ONE_HOME", "one", None),
                spec("two", "config_two", "TWO_HOME", "two", None),
            ],
            &environment(&[("ONE_HOME", &shared), ("TWO_HOME", &trailing)]),
            &root,
            Timestamp(10),
            Timestamp(20),
            provenance(),
        ),
        Err(ProfileError::DuplicatePath)
    );
    assert_eq!(
        resolve_profiles(
            &[
                spec("one", "config_one", "ONE_HOME", "a/b", None),
                spec("two", "config_two", "TWO_HOME", "a//b", None),
            ],
            &environment(&[
                ("ONE_HOME", &root.join("first-override")),
                ("TWO_HOME", &root.join("second-override")),
            ]),
            &root,
            Timestamp(10),
            Timestamp(20),
            provenance(),
        ),
        Err(ProfileError::DuplicatePath)
    );
    assert_eq!(
        resolve_profiles(
            std::slice::from_ref(&valid),
            &environment(&[("ONE_HOME", Path::new("relative"))]),
            &root,
            Timestamp(10),
            Timestamp(20),
            provenance(),
        ),
        Err(ProfileError::InvalidPath)
    );
    assert_eq!(
        resolve_profiles(
            &[valid],
            &BTreeMap::new(),
            Path::new("relative-home"),
            Timestamp(10),
            Timestamp(20),
            provenance(),
        ),
        Err(ProfileError::InvalidPath)
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn profile_wire_validation_and_resource_limits_refuse_unproven_state() {
    let root = home();
    let valid = spec("one", "config_one", "ONE_HOME", ".one", None);
    let resolved = resolve_profiles(
        std::slice::from_ref(&valid),
        &BTreeMap::new(),
        &root,
        Timestamp(10),
        Timestamp(20),
        provenance(),
    )
    .unwrap();
    assert_eq!(resolved[0].schema_version, PROFILE_VERSION);
    let mut wire = serde_json::to_value(&resolved[0]).unwrap();
    wire["schema_version"] = serde_json::json!(2);
    assert!(serde_json::from_value::<ProfileObservation>(wire).is_err());
    let mut wire = serde_json::to_value(&resolved[0]).unwrap();
    wire["path"] = serde_json::json!("relative");
    assert!(serde_json::from_value::<ProfileObservation>(wire).is_err());
    assert_eq!(
        resolve_profiles(
            std::slice::from_ref(&valid),
            &BTreeMap::new(),
            &root,
            Timestamp(20),
            Timestamp(20),
            provenance(),
        ),
        Err(ProfileError::Stale)
    );
    let too_many = (0..=64)
        .map(|index| {
            spec(
                &format!("profile-{index}"),
                &format!("config-{index}"),
                &format!("PROFILE_{index}_HOME"),
                &format!("profile-{index}"),
                None,
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        resolve_profiles(
            &too_many,
            &BTreeMap::new(),
            &root,
            Timestamp(10),
            Timestamp(20),
            provenance(),
        ),
        Err(ProfileError::ResourceLimit)
    );
    fs::remove_dir_all(root).unwrap();
}
