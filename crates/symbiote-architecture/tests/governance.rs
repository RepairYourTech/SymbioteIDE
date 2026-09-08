use symbiote_architecture::*;

fn decision(id: &str) -> Decision {
    Decision {
        schema_version: 1,
        id: id.into(),
        revision: 1,
        state: DecisionState::Proposed,
        title: "Shell candidate".into(),
        constraints: vec!["No Electron".into()],
        alternatives: vec!["Non-Electron native shell".into()],
        consequences: vec!["Platform evidence required".into()],
        security: "Isolated Preview".into(),
        reversal: "Replace client without canonical data migration".into(),
        high_reversal_cost: false,
        issue_refs: vec![38],
        requirement_refs: vec!["preview-isolation".into()],
        graph_refs: vec![],
        deployment_refs: vec![],
        compatibility: vec![],
        evidence: vec![],
        supersedes: None,
    }
}
fn approval() -> Acceptance {
    Acceptance {
        actor: "architect".into(),
        authority: Authority::DelegatedArchitect,
        now: 10,
        client_exception: None,
    }
}
fn fact(id: &str) -> CompatibilityFact {
    CompatibilityFact {
        id: id.into(),
        revision: 1,
        tested_version: "test-fixture-1".into(),
        sources: vec!["https://example.com/official".into()],
        observed_at: 5,
        expires_at: 20,
        status: FactStatus::Verified,
        native_path: "stdio".into(),
        precedence: "version-pinned".into(),
        trust_behavior: "Host isolated".into(),
        unknowns: vec!["real platform test pending".into()],
    }
}
fn pins(artifact: &str, id: &str, revision: u64) -> ImplementationPins {
    ImplementationPins {
        artifact: artifact.into(),
        decisions: vec![VersionPin {
            id: id.into(),
            revision,
        }],
        compatibility: vec![],
    }
}

#[test]
fn unresolved_and_worker_authorization_are_denied_without_mutation() {
    let mut r = DecisionRegistry::default();
    r.propose(decision("shell")).unwrap();
    r.register_implementation(pins("workbench@1", "shell", 1))
        .unwrap();
    assert_eq!(r.gate("workbench@1", 10).status, GateStatus::Provisional);
    assert!(r.require_ready("workbench@1", 10).is_err());
    let before = r.decision("shell").unwrap().clone();
    let mut worker = approval();
    worker.authority = Authority::Worker;
    assert!(r.accept("shell", 1, worker).is_err());
    assert_eq!(r.decision("shell"), Some(&before));
    assert!(r.approval("shell").is_none());
}

#[test]
fn acceptance_requires_new_pin_and_freezes_content() {
    let mut r = DecisionRegistry::default();
    r.propose(decision("shell")).unwrap();
    r.register_implementation(pins("draft@1", "shell", 1))
        .unwrap();
    assert_eq!(r.accept("shell", 1, approval()).unwrap(), ["draft@1"]);
    assert_eq!(r.gate("draft@1", 10).status, GateStatus::Stale);
    r.register_implementation(pins("workbench@1", "shell", 2))
        .unwrap();
    r.require_ready("workbench@1", 10).unwrap();
    let mut changed = r.decision("shell").unwrap().clone();
    changed.title = "Silent change".into();
    assert!(r.revise_draft(2, changed).is_err());
    assert_eq!(r.decision("shell").unwrap().title, "Shell candidate");
    assert!(
        r.register_implementation(pins("workbench@1", "other", 1))
            .is_err()
    );
}

#[test]
fn draft_revision_reports_invalidated_pins() {
    let mut r = DecisionRegistry::default();
    r.propose(decision("shell")).unwrap();
    r.register_implementation(pins("draft@1", "shell", 1))
        .unwrap();
    assert_eq!(r.revise_draft(1, decision("shell")).unwrap(), ["draft@1"]);
    assert_eq!(r.gate("draft@1", 10).status, GateStatus::Stale);
}

#[test]
fn optimistic_conflict_and_retry_do_not_change_state() {
    let mut r = DecisionRegistry::default();
    r.propose(decision("a")).unwrap();
    let mut draft = decision("a");
    draft.state = DecisionState::Investigating;
    r.revise_draft(1, draft.clone()).unwrap();
    assert!(r.revise_draft(1, draft).is_err());
    assert!(r.accept("a", 1, approval()).is_err());
    assert_eq!(r.decision("a").unwrap().revision, 2);
    r.accept("a", 2, approval()).unwrap();
    let accepted = r.decision("a").unwrap().clone();
    assert!(r.accept("a", 2, approval()).is_err());
    assert_eq!(r.decision("a"), Some(&accepted));
}

#[test]
fn high_cost_requires_evidence_or_actual_client_exception() {
    let mut r = DecisionRegistry::default();
    let mut d = decision("a");
    d.high_reversal_cost = true;
    r.propose(d).unwrap();
    assert!(r.accept("a", 1, approval()).is_err());
    let mut waived = approval();
    waived.client_exception = Some("Client accepts reversal cost".into());
    assert!(r.accept("a", 1, waived.clone()).is_err());
    waived.authority = Authority::Client;
    r.accept("a", 1, waived.clone()).unwrap();
    assert_eq!(r.approval("a"), Some(&waived));
}

#[test]
fn evidence_metadata_is_checked_before_acceptance() {
    let mut r = DecisionRegistry::default();
    let mut d = decision("a");
    d.high_reversal_cost = true;
    d.evidence.push(Evidence {
        artifact: "raw.json".into(),
        sha256: "a".repeat(64),
        method: "fixture-only".into(),
        observed_at: 11,
        reviewer: "reviewer".into(),
    });
    r.propose(d.clone()).unwrap();
    assert!(r.accept("a", 1, approval()).is_err());
    d.evidence[0].observed_at = 9;
    r.revise_draft(1, d).unwrap();
    r.accept("a", 2, approval()).unwrap();
}

#[test]
fn supersession_invalidates_only_dependent_artifacts() {
    let mut r = DecisionRegistry::default();
    for id in ["shell", "protocol"] {
        r.propose(decision(id)).unwrap();
        r.accept(id, 1, approval()).unwrap();
    }
    r.register_implementation(pins("ui@1", "shell", 2)).unwrap();
    r.register_implementation(pins("host@1", "protocol", 2))
        .unwrap();
    let mut replacement = decision("shell-next");
    replacement.supersedes = Some("shell".into());
    r.propose(replacement).unwrap();
    let mut both = pins("draft@2", "shell-next", 1);
    both.decisions.push(VersionPin {
        id: "shell".into(),
        revision: 2,
    });
    r.register_implementation(both).unwrap();
    assert_eq!(
        r.accept("shell-next", 1, approval()).unwrap(),
        ["draft@2", "ui@1"]
    );
    assert_eq!(
        r.decision("shell").unwrap().state,
        DecisionState::Superseded
    );
    assert_eq!(r.gate("ui@1", 10).status, GateStatus::Stale);
    r.require_ready("host@1", 10).unwrap();
}

#[test]
fn competing_supersession_fails_atomically() {
    let mut r = DecisionRegistry::default();
    r.propose(decision("old")).unwrap();
    r.accept("old", 1, approval()).unwrap();
    for id in ["new-a", "new-b"] {
        let mut d = decision(id);
        d.supersedes = Some("old".into());
        r.propose(d).unwrap();
    }
    r.accept("new-a", 1, approval()).unwrap();
    assert!(r.accept("new-b", 1, approval()).is_err());
    assert_eq!(r.decision("new-b").unwrap().state, DecisionState::Proposed);
    assert!(r.approval("new-b").is_none());
}

#[test]
fn fact_expiry_revocation_and_changed_versions_invalidate_selectively() {
    let mut r = DecisionRegistry::default();
    r.put_fact(None, fact("webkit")).unwrap();
    let mut ui = decision("ui");
    ui.compatibility.push(VersionPin {
        id: "webkit".into(),
        revision: 1,
    });
    r.propose(ui).unwrap();
    r.accept("ui", 1, approval()).unwrap();
    r.propose(decision("store")).unwrap();
    r.accept("store", 1, approval()).unwrap();
    r.register_implementation(pins("ui@1", "ui", 2)).unwrap();
    r.register_implementation(pins("store@1", "store", 2))
        .unwrap();
    r.require_ready("ui@1", 19).unwrap();
    assert_eq!(r.gate("ui@1", 20).status, GateStatus::Stale);
    let mut changed = fact("webkit");
    changed.revision = 2;
    changed.status = FactStatus::Revoked;
    assert!(r.put_fact(Some(0), changed.clone()).is_err());
    assert_eq!(r.put_fact(Some(1), changed).unwrap(), ["ui@1"]);
    assert!(r.require_ready("ui@1", 10).is_err());
    r.require_ready("store@1", 10).unwrap();
}

#[test]
fn unknown_compatibility_never_accepts() {
    let mut r = DecisionRegistry::default();
    let mut d = decision("ui");
    d.compatibility.push(VersionPin {
        id: "webkit".into(),
        revision: 1,
    });
    r.propose(d).unwrap();
    assert!(r.accept("ui", 1, approval()).is_err());
    let mut unknown = fact("webkit");
    unknown.status = FactStatus::Unknown;
    r.put_fact(None, unknown).unwrap();
    assert!(r.accept("ui", 1, approval()).is_err());
}

#[test]
fn wire_roundtrip_and_future_schema_rejection() {
    let d = decision("a");
    let json = serde_json::to_string(&d).unwrap();
    assert_eq!(serde_json::from_str::<Decision>(&json).unwrap(), d);
    let mut future = d;
    future.schema_version = 2;
    assert!(DecisionRegistry::default().propose(future).is_err());
    assert!(
        serde_json::from_str::<Decision>(&json.replace(
            "\"schema_version\":1",
            "\"extra\":true,\"schema_version\":1"
        ))
        .is_err()
    );
    let schema = schemars::schema_for!(Decision);
    assert!(
        serde_json::to_string(&schema)
            .unwrap()
            .contains("schema_version")
    );
}

#[test]
fn spike_requires_reproducibility_and_finite_predeclared_thresholds() {
    let mut spike = SpikeContract {
        schema_version: 1,
        id: "test".into(),
        decision: "shell".into(),
        hypothesis: "bounded queues".into(),
        workload: vec!["four streams".into()],
        platform: "test only".into(),
        hardware: "fixture".into(),
        method: "measure tree".into(),
        measurements: vec![Measurement {
            name: "queue".into(),
            unit: "items".into(),
            maximum: 100.0,
        }],
        stop_conditions: vec!["overflow".into()],
        result_artifact: "result.json".into(),
        cleanup: "terminate fixture processes".into(),
    };
    spike.validate().unwrap();
    spike.measurements[0].maximum = f64::NAN;
    assert!(spike.validate().is_err());
    spike.measurements[0].maximum = 100.0;
    spike.cleanup.clear();
    assert!(spike.validate().is_err());
}
