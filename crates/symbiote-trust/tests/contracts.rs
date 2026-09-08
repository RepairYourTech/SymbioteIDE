use std::collections::BTreeSet;
use symbiote_domain::*;
use symbiote_trust::*;

fn fixture() -> ResourceConsent {
    let project = ProjectId::new("project").unwrap();
    ResourceConsent {
        id: CommandId::new("consent").unwrap(),
        user_id: UserId::new("user").unwrap(),
        issued_at: Timestamp(10),
        expires_at: Timestamp(20),
        revoked_at: None,
        snapshot: ResourceSnapshot {
            project_id: project.clone(),
            role_id: RoleId::new("lead").unwrap(),
            profile_id: RuntimeProfileId::new("native").unwrap(),
            host_id: HostId::new("host").unwrap(),
            resource_ref: "skill.ref".into(),
            fingerprint: Fingerprint::of(b"private content"),
            access: AccessSnapshot {
                project_id: project,
                roots: BTreeSet::from([RootId::new("root").unwrap()]),
                grants: BTreeSet::from([Permission::ReadRoot]),
                policy_revision: Revision(3),
            },
        },
    }
}
fn check(consent: &ResourceConsent) -> Result<(), TrustError> {
    authorize_load(
        consent,
        &consent.snapshot,
        &consent.snapshot.access,
        &consent.snapshot.host_id,
        Timestamp(15),
    )
}

#[test]
fn exact_snapshot_within_time_and_current_policy_passes() {
    let consent = fixture();
    assert_eq!(check(&consent), Ok(()));
    let mut policy = consent.snapshot.access.clone();
    policy.grants.insert(Permission::Network);
    assert_eq!(
        authorize_load(
            &consent,
            &consent.snapshot,
            &policy,
            &consent.snapshot.host_id,
            Timestamp(10)
        ),
        Ok(())
    );
}

#[test]
fn every_bound_snapshot_dimension_is_checked() {
    let consent = fixture();
    type SnapshotMutation = Box<dyn Fn(&mut ResourceSnapshot)>;
    let changes: Vec<SnapshotMutation> = vec![
        Box::new(|s| s.fingerprint = Fingerprint::of(b"changed")),
        Box::new(|s| s.resource_ref = "another".into()),
        Box::new(|s| s.role_id = RoleId::new("worker").unwrap()),
        Box::new(|s| s.host_id = HostId::new("other").unwrap()),
        Box::new(|s| s.profile_id = RuntimeProfileId::new("external").unwrap()),
        Box::new(|s| {
            s.access.roots.insert(RootId::new("other").unwrap());
        }),
        Box::new(|s| {
            s.access.grants.insert(Permission::ExecuteProcess);
        }),
        Box::new(|s| s.access.policy_revision = Revision(4)),
    ];
    for change in changes {
        let mut snapshot = consent.snapshot.clone();
        change(&mut snapshot);
        assert_eq!(
            authorize_load(
                &consent,
                &snapshot,
                &consent.snapshot.access,
                &consent.snapshot.host_id,
                Timestamp(15)
            ),
            Err(TrustError::SnapshotChanged)
        );
    }
    assert_eq!(
        authorize_load(
            &consent,
            &consent.snapshot,
            &consent.snapshot.access,
            &HostId::new("other").unwrap(),
            Timestamp(15)
        ),
        Err(TrustError::WrongHost)
    );
}

#[test]
fn expiry_revocation_and_invalid_time_are_denied() {
    let mut consent = fixture();
    for (time, expected) in [
        (9, TrustError::NotYetValid),
        (20, TrustError::Expired),
        (u64::MAX, TrustError::Expired),
    ] {
        assert_eq!(
            authorize_load(
                &consent,
                &consent.snapshot,
                &consent.snapshot.access,
                &consent.snapshot.host_id,
                Timestamp(time)
            ),
            Err(expected)
        );
    }
    consent.revoked_at = Some(Timestamp(15));
    assert_eq!(check(&consent), Err(TrustError::Revoked));
    consent.revoked_at = Some(Timestamp(9));
    assert_eq!(consent.validate(), Err(TrustError::InvalidTime));
    consent.revoked_at = None;
    consent.expires_at = consent.issued_at;
    assert_eq!(consent.validate(), Err(TrustError::InvalidTime));
    consent.expires_at = Timestamp(u64::MAX);
    assert_eq!(consent.validate(), Err(TrustError::InvalidTime));
}

#[test]
fn policy_project_revision_roots_and_grants_must_still_allow_access() {
    let consent = fixture();
    let mut policy = consent.snapshot.access.clone();
    policy.project_id = ProjectId::new("other").unwrap();
    assert_eq!(
        authorize_load(
            &consent,
            &consent.snapshot,
            &policy,
            &consent.snapshot.host_id,
            Timestamp(15)
        ),
        Err(TrustError::PolicyChanged)
    );
    policy = consent.snapshot.access.clone();
    policy.policy_revision = Revision(4);
    assert_eq!(
        authorize_load(
            &consent,
            &consent.snapshot,
            &policy,
            &consent.snapshot.host_id,
            Timestamp(15)
        ),
        Err(TrustError::PolicyChanged)
    );
    policy = consent.snapshot.access.clone();
    policy.grants.clear();
    assert_eq!(
        authorize_load(
            &consent,
            &consent.snapshot,
            &policy,
            &consent.snapshot.host_id,
            Timestamp(15)
        ),
        Err(TrustError::AccessDenied)
    );
    policy = consent.snapshot.access.clone();
    policy.roots.clear();
    assert_eq!(
        authorize_load(
            &consent,
            &consent.snapshot,
            &policy,
            &consent.snapshot.host_id,
            Timestamp(15)
        ),
        Err(TrustError::AccessDenied)
    );
    let mut invalid = consent;
    invalid.snapshot.access.project_id = ProjectId::new("other").unwrap();
    assert_eq!(invalid.validate(), Err(TrustError::InvalidAccess));
}

#[test]
fn validated_serde_strict_parsing_and_metadata_only_schema() {
    let consent = fixture();
    let json = serde_json::to_string(&consent).unwrap();
    assert_eq!(ResourceConsent::parse(&json).unwrap(), consent);
    assert!(!json.contains("private content"));
    assert!(!format!("{consent:?}").contains("private content"));
    let schema = serde_json::to_string(&schemars::schema_for!(ResourceConsent)).unwrap();
    assert!(!schema.contains("private content"));
    assert!(!schema.contains("secret_value"));
    assert!(ResourceConsent::parse(&json.replacen("{", "{\"issued_at\":10,", 1)).is_err());
    assert!(ResourceConsent::parse(&json.replacen("{", "{\"authority\":\"host\",", 1)).is_err());
    assert!(ResourceConsent::parse(&format!("{}{}", " ".repeat(65_536), json)).is_err());
    let mut invalid = consent.clone();
    invalid.expires_at = Timestamp(1);
    assert!(
        serde_json::from_str::<ResourceConsent>(&serde_json::to_string(&invalid).unwrap()).is_err()
    );
    invalid = consent;
    invalid.snapshot.resource_ref = "/tmp/path".into();
    assert!(
        serde_json::from_str::<ResourceConsent>(&serde_json::to_string(&invalid).unwrap()).is_err()
    );
}

#[test]
fn fingerprints_hash_exact_bytes_and_reject_noncanonical_hex() {
    assert_eq!(
        Fingerprint::of(b"").as_str(),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    assert_ne!(Fingerprint::of(b"x"), Fingerprint::of(b"x\n"));
    for invalid in ["a".repeat(63), "A".repeat(64), "g".repeat(64)] {
        assert!(Fingerprint::new(invalid.clone()).is_err());
        assert!(serde_json::from_str::<Fingerprint>(&format!("\"{invalid}\"")).is_err());
    }
}
