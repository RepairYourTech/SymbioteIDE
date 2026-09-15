//! This repository's decision ledger (#173): what it publishes, and every rule
//! that must fail when the ledger or the tree moves under it.
//!
//! The real-tree tests assert the committed ledger against the workspace cargo
//! reports. The fixture tests build a throwaway tree and break exactly one
//! thing at a time, so each refusal is proved by the rule it belongs to rather
//! than by the suite as a whole.

use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use symbiote_architecture::checks::{Problem, problems};
use symbiote_architecture::ledger::{LEDGER_PATH, Ledger};
use symbiote_architecture::repository::{Workspace, hash};
use symbiote_architecture::{DecisionState, GateStatus, workspace_root};

/// 2026-09-08 UTC, the day ADR-0001 and its ledger were recorded.
const NOW: u64 = 1_788_825_600;
const ADR: &str = "docs/architecture/adr-0001-technology-direction.md";
const ID: &str = "TEST/HOST-STACK";

fn root() -> PathBuf {
    workspace_root()
}

fn ledger() -> Ledger {
    Ledger::read(&root().join(LEDGER_PATH)).expect("the committed decision ledger")
}

fn workspace() -> Workspace {
    Workspace::read(&root()).expect("cargo reports this workspace")
}

fn real_problems() -> Vec<Problem> {
    problems(&ledger(), &workspace(), &root(), NOW)
}

#[test]
fn the_committed_ledger_passes_every_rule() {
    assert_eq!(real_problems(), Vec::new());
}

#[test]
fn every_workspace_member_cargo_reports_is_recorded_as_an_artifact() {
    let ledger = ledger();
    let workspace = workspace();
    let members: Vec<&str> = workspace.paths().collect();
    let artifacts: Vec<&str> = ledger
        .artifacts
        .iter()
        .map(|artifact| artifact.pins.artifact.as_str())
        .collect();
    assert!(
        members.len() > 20,
        "the workspace still has members to cover"
    );
    assert_eq!(artifacts, members);
    for artifact in &ledger.artifacts {
        assert!(!artifact.pins.decisions.is_empty());
    }
}

#[test]
fn the_desktop_shell_is_published_as_provisional_and_owned_by_38() {
    let ledger = ledger();
    let registry = ledger.registry().expect("the ledger replays");
    let record = ledger
        .decision("ADR-0001/DESKTOP-SHELL")
        .expect("the shell decision");
    assert_eq!(record.published.state, DecisionState::Investigating);
    assert_eq!(record.blocking_issue, Some(38));
    assert_eq!(record.proposed_dependencies, ["tauri", "tauri-build"]);
    let artifact = ledger
        .artifacts
        .iter()
        .find(|artifact| artifact.pins.artifact == "crates/symbiote-desktop")
        .expect("the desktop crate pins the shell choice");
    assert_eq!(artifact.status, GateStatus::Provisional);
    assert_eq!(artifact.blocking_issue, Some(38));
    let reached = registry.gate("crates/symbiote-desktop", NOW);
    assert_eq!(reached.status, GateStatus::Provisional);
    assert!(
        reached
            .reasons
            .iter()
            .any(|reason| reason.contains("ADR-0001/DESKTOP-SHELL")),
        "the reason names the choice that is not accepted: {:?}",
        reached.reasons
    );
    assert!(
        registry
            .require_ready("crates/symbiote-desktop", NOW)
            .is_err()
    );
}

#[test]
fn an_accepted_record_is_the_bytes_it_was_accepted_from() {
    let root = root();
    for record in &ledger().decisions {
        assert_eq!(
            hash(&root, &record.record).as_deref(),
            Some(record.record_sha256.as_str()),
            "{} pins its own text",
            record.draft.id
        );
        for evidence in &record.draft.evidence {
            assert_eq!(
                hash(&root, &evidence.artifact).as_deref(),
                Some(evidence.sha256.as_str()),
                "{} pins the evidence it cites",
                record.draft.id
            );
        }
    }
}

#[test]
fn the_recorded_choices_name_the_issues_that_own_them() {
    let ledger = ledger();
    let owners: Vec<(String, Option<u64>)> = ledger
        .decisions
        .iter()
        .map(|record| (record.draft.id.clone(), record.blocking_issue))
        .collect();
    assert_eq!(
        owners,
        [
            ("ADR-0001/HOST-STACK".to_string(), None),
            ("ADR-0001/DESKTOP-SHELL".to_string(), Some(38)),
            ("ADR-0001/GRAPH-STORAGE".to_string(), Some(233)),
        ]
    );
    assert!(ledger.decisions[1].acceptance.is_none());
    assert!(ledger.decisions[1].draft.evidence.is_empty());
}

#[test]
fn an_edited_accepted_record_is_refused_by_its_recorded_hash() {
    let mut value = fixture_ledger();
    value["decisions"][0]["record_sha256"] = json!("0".repeat(64));
    let fixture = Fixture::new(value);
    let found = fixture.problems();
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(
        found[0]
            .detail
            .contains("has changed since this decision cited it")
    );
}

#[test]
fn a_missing_accepted_record_is_refused() {
    let mut value = fixture_ledger();
    value["decisions"][0]["record"] = json!("docs/architecture/absent.md");
    let fixture = Fixture::new(value);
    let found = fixture.problems();
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].detail.contains("is not in the tree"));
}

#[test]
fn evidence_a_decision_cites_is_refused_when_its_bytes_change() {
    let mut value = fixture_ledger();
    value["decisions"][0]["draft"]["evidence"] = json!([{
        "artifact": ADR,
        "sha256": "0".repeat(64),
        "method": "a fixture observation",
        "observed_at": NOW - 10,
        "reviewer": "fixture"
    }]);
    let fixture = Fixture::new(value);
    let found = fixture.problems();
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].subject, format!("{ID}: {ADR}"));
    assert!(
        found[0]
            .detail
            .contains("the evidence has changed since this decision cited it")
    );
}

#[test]
fn evidence_a_decision_cites_is_refused_when_it_is_gone() {
    let mut value = fixture_ledger();
    value["decisions"][0]["draft"]["evidence"] = json!([{
        "artifact": "docs/architecture/absent.md",
        "sha256": "0".repeat(64),
        "method": "a fixture observation",
        "observed_at": NOW - 10,
        "reviewer": "fixture"
    }]);
    let fixture = Fixture::new(value);
    let found = fixture.problems();
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(
        found[0]
            .detail
            .contains("the evidence this decision cites is not in the tree")
    );
}

#[test]
fn a_worker_authority_acceptance_is_refused_by_the_replay() {
    let mut value = fixture_ledger();
    value["decisions"][0]["acceptance"]["authority"] = json!("worker");
    let fixture = Fixture::new(value);
    let found = fixture.problems();
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(
        found[0]
            .detail
            .contains("does not replay through the policy registry")
    );
    assert!(
        found[0]
            .detail
            .contains("worker cannot authorize architecture")
    );
}

#[test]
fn a_record_that_publishes_a_revision_the_registry_never_reaches_is_refused() {
    let mut value = fixture_ledger();
    value["decisions"][0]["published"]["revision"] = json!(3);
    let fixture = Fixture::new(value);
    let found = fixture.problems();
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(
        found[0].detail.contains("publishes Accepted at revision 3"),
        "{found:?}"
    );
    assert!(found[0].detail.contains("reaches Accepted at revision 2"));
}

#[test]
fn an_accepted_record_that_still_names_a_blocking_issue_is_refused() {
    let mut value = fixture_ledger();
    value["decisions"][0]["blocking_issue"] = json!(38);
    let fixture = Fixture::new(value);
    let found = fixture.problems();
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].detail.contains("cannot still be blocked by #38"));
}

#[test]
fn an_unaccepted_record_without_an_owning_issue_is_refused() {
    let mut value = fixture_ledger();
    value["decisions"][0]["published"] = json!({ "state": "investigating", "revision": 2 });
    value["decisions"][0]["draft"]["state"] = json!("investigating");
    value["decisions"][0]
        .as_object_mut()
        .unwrap()
        .remove("acceptance");
    value["artifacts"][0]["status"] = json!("provisional");
    value["artifacts"][0]["blocking_issue"] = json!(38);
    let fixture = Fixture::new(value);
    let found = fixture.problems();
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(
        found[0]
            .detail
            .contains("names no issue that owns what decides it")
    );
}

#[test]
fn a_rejected_record_replays_and_needs_no_owning_issue() {
    let mut value = fixture_ledger();
    value["decisions"][0]["published"] = json!({ "state": "rejected", "revision": 2 });
    value["decisions"][0]["draft"]["state"] = json!("rejected");
    value["decisions"][0]
        .as_object_mut()
        .unwrap()
        .remove("acceptance");
    value["artifacts"][0]["status"] = json!("blocked");
    value["artifacts"][0]["blocking_issue"] = json!(38);
    let fixture = Fixture::new(value);
    assert_eq!(fixture.problems(), Vec::new());
}

#[test]
fn a_member_cargo_reports_but_the_ledger_does_not_is_refused() {
    let fixture = Fixture::new(fixture_ledger());
    let found = problems(
        &fixture.ledger(),
        &fixture.workspace_with(&[("crates/member", &[]), ("crates/newcomer", &[])]),
        &fixture.root,
        NOW,
    );
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].subject, "crates/newcomer");
    assert!(found[0].detail.contains("pins no architecture decision"));
}

#[test]
fn a_record_for_an_artifact_cargo_does_not_report_is_refused() {
    let mut value = fixture_ledger();
    value["artifacts"][0]["pins"]["artifact"] = json!("crates/absent");
    let fixture = Fixture::new(value);
    let found = fixture.problems();
    assert_eq!(found.len(), 2, "{found:?}");
    assert!(found.iter().any(|problem| {
        problem
            .detail
            .contains("does not report as a workspace member")
    }));
    assert!(
        found
            .iter()
            .any(|problem| problem.subject == "crates/member")
    );
}

#[test]
fn an_artifact_that_publishes_a_status_the_registry_does_not_reach_is_refused() {
    let mut value = fixture_ledger();
    value["decisions"][0]["published"] = json!({ "state": "investigating", "revision": 2 });
    value["decisions"][0]["draft"]["state"] = json!("investigating");
    value["decisions"][0]
        .as_object_mut()
        .unwrap()
        .remove("acceptance");
    value["decisions"][0]["blocking_issue"] = json!(38);
    let fixture = Fixture::new(value);
    let found = fixture.problems();
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].subject, "crates/member");
    assert!(found[0].detail.contains("the ledger publishes settled"));
    assert!(found[0].detail.contains("the registry reaches provisional"));
    assert!(found[0].detail.contains("is not accepted"));
}

#[test]
fn a_settled_artifact_that_still_names_a_blocking_issue_is_refused() {
    let mut value = fixture_ledger();
    value["artifacts"][0]["blocking_issue"] = json!(38);
    let fixture = Fixture::new(value);
    let found = fixture.problems();
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(
        found[0]
            .detail
            .contains("a settled artifact cannot still name")
    );
}

#[test]
fn an_artifact_short_of_settled_without_an_owning_issue_is_refused() {
    let mut value = fixture_ledger();
    value["decisions"][0]["published"] = json!({ "state": "investigating", "revision": 2 });
    value["decisions"][0]["draft"]["state"] = json!("investigating");
    value["decisions"][0]
        .as_object_mut()
        .unwrap()
        .remove("acceptance");
    value["decisions"][0]["blocking_issue"] = json!(38);
    value["artifacts"][0]["status"] = json!("provisional");
    let fixture = Fixture::new(value);
    let found = fixture.problems();
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(
        found[0]
            .detail
            .contains("names no issue that owns what stops it")
    );
}

#[test]
fn a_member_that_declares_a_provisional_choice_without_recording_it_is_refused() {
    let mut value = fixture_ledger();
    value["decisions"][0]["proposed_dependencies"] = json!(["tauri"]);
    let fixture = Fixture::new(value);
    let found = problems(
        &fixture.ledger(),
        &fixture.workspace_with(&[
            ("crates/member", &["tauri"]),
            ("crates/desktop", &["tauri"]),
        ]),
        &fixture.root,
        NOW,
    );
    assert_eq!(found.len(), 2, "{found:?}");
    assert!(found.iter().any(|problem| {
        problem.subject == "crates/desktop"
            && problem.detail.contains("it declares tauri")
            && problem.detail.contains(ID)
    }));
    assert!(
        found
            .iter()
            .any(|problem| problem.subject == "crates/desktop"
                && problem.detail.contains("pins no architecture decision"))
    );
}

#[test]
fn a_pin_to_a_choice_the_member_does_not_declare_is_refused() {
    let mut value = fixture_ledger();
    value["decisions"][0]["proposed_dependencies"] = json!(["tauri"]);
    value["artifacts"][0]["pins"]["decisions"]
        .as_array_mut()
        .unwrap()
        .push(json!({ "id": ID, "revision": 2 }));
    let fixture = Fixture::new(value);
    let found = fixture.problems();
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].subject, "crates/member");
    assert!(found[0].detail.contains("whose dependency names (tauri)"));
}

#[test]
fn an_expired_compatibility_fact_is_refused() {
    let mut value = fixture_ledger();
    value["facts"] = json!([{
        "id": "webkit-2",
        "revision": 1,
        "tested_version": "2.0",
        "sources": ["https://example.invalid/notes"],
        "observed_at": NOW - 100,
        "expires_at": NOW,
        "status": "verified",
        "native_path": "webkitgtk",
        "precedence": "version-pinned",
        "trust_behavior": "Host isolated",
        "unknowns": []
    }]);
    let fixture = Fixture::new(value);
    let found = fixture.problems();
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].subject, "compatibility fact webkit-2");
    assert!(found[0].detail.contains("its validity lapsed"));
}

#[test]
fn a_fact_a_refreshed_ledger_holds_at_a_later_revision_still_replays() {
    let mut value = fixture_ledger();
    value["facts"] = json!([
        {
            "id": "webkit-2",
            "revision": 1,
            "tested_version": "2.0",
            "sources": ["https://example.invalid/notes"],
            "observed_at": NOW - 100,
            "expires_at": NOW + 1_000,
            "status": "verified",
            "native_path": "webkitgtk",
            "precedence": "version-pinned",
            "trust_behavior": "Host isolated",
            "unknowns": []
        },
        {
            "id": "webkit-2",
            "revision": 2,
            "tested_version": "2.1",
            "sources": ["https://example.invalid/notes"],
            "observed_at": NOW - 50,
            "expires_at": NOW + 2_000,
            "status": "verified",
            "native_path": "webkitgtk",
            "precedence": "version-pinned",
            "trust_behavior": "Host isolated",
            "unknowns": []
        }
    ]);
    let fixture = Fixture::new(value);
    assert_eq!(fixture.problems(), Vec::new());
    let registry = fixture
        .ledger()
        .registry()
        .expect("the refreshed fact replays");
    assert!(registry.decision(ID).is_some());
}

#[test]
fn a_ledger_of_an_unknown_schema_version_is_refused() {
    let mut value = fixture_ledger();
    value["schema_version"] = json!(2);
    let fixture = Fixture::new(value);
    let error = Ledger::read(&fixture.ledger_path()).expect_err("a future schema is refused");
    assert!(error.0.contains("unsupported decision ledger schema"));
}

#[test]
fn a_ledger_with_an_unknown_field_is_refused() {
    let mut value = fixture_ledger();
    value["approvals"] = json!([]);
    let fixture = Fixture::new(value);
    let error = Ledger::read(&fixture.ledger_path()).expect_err("an unknown field is refused");
    assert!(error.0.contains("unknown field"), "{error}");
}

#[test]
fn a_ledger_that_records_no_decision_is_refused() {
    let mut value = fixture_ledger();
    value["decisions"] = json!([]);
    let fixture = Fixture::new(value);
    let error = Ledger::read(&fixture.ledger_path()).expect_err("an empty ledger is refused");
    assert!(error.0.contains("records no decision"));
}

/// A ledger for one member pinning one accepted decision, valid as written.
fn fixture_ledger() -> Value {
    json!({
        "schema_version": 1,
        "decisions": [
            {
                "published": { "state": "accepted", "revision": 2 },
                "draft": {
                    "schema_version": 1,
                    "id": ID,
                    "revision": 1,
                    "state": "proposed",
                    "title": "A stack constraint",
                    "constraints": ["Rust"],
                    "alternatives": ["another language"],
                    "consequences": ["the Host is Rust"],
                    "security": "the client holds no canonical state",
                    "reversal": "replacing it would rewrite the Host",
                    "high_reversal_cost": true,
                    "issue_refs": [173],
                    "requirement_refs": ["A04"],
                    "graph_refs": [],
                    "deployment_refs": [],
                    "compatibility": [],
                    "evidence": [],
                    "supersedes": null
                },
                "record": ADR,
                "record_sha256": "",
                "acceptance": {
                    "actor": "client",
                    "authority": "client",
                    "now": NOW,
                    "client_exception": "the client accepted the reversal cost"
                }
            }
        ],
        "facts": [],
        "artifacts": [
            {
                "pins": {
                    "artifact": "crates/member",
                    "decisions": [{ "id": ID, "revision": 2 }],
                    "compatibility": []
                },
                "status": "settled"
            }
        ]
    })
}

static NEXT: AtomicUsize = AtomicUsize::new(0);

/// A throwaway tree holding a record and one ledger, removed when it drops.
struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn new(mut value: Value) -> Fixture {
        let root = std::env::temp_dir().join(format!(
            "symbiote-decisions-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::SeqCst)
        ));
        std::fs::create_dir_all(root.join("docs/architecture")).expect("a fixture tree");
        let root = root.canonicalize().expect("a canonical fixture root");
        std::fs::write(
            root.join(ADR),
            "the accepted text these records interpret\n",
        )
        .expect("the fixture record");
        let unrecorded = value["decisions"]
            .as_array()
            .and_then(|decisions| decisions.first())
            .is_some_and(|record| record["record_sha256"] == json!(""));
        if unrecorded {
            value["decisions"][0]["record_sha256"] =
                json!(hash(&root, ADR).expect("the fixture record hashes"));
        }
        let fixture = Fixture { root };
        Fixture::write(&fixture.ledger_path(), &value);
        fixture
    }

    fn write(path: &Path, value: &Value) {
        std::fs::write(
            path,
            serde_json::to_vec_pretty(value).expect("readable JSON"),
        )
        .expect("the fixture ledger");
    }

    fn ledger_path(&self) -> PathBuf {
        self.root.join(LEDGER_PATH)
    }

    fn ledger(&self) -> Ledger {
        Ledger::read(&self.ledger_path()).expect("the fixture ledger")
    }

    fn workspace_with(&self, members: &[(&str, &[&str])]) -> Workspace {
        let packages: Vec<Value> = members
            .iter()
            .map(|(member, dependencies)| {
                json!({
                    "manifest_path": self.root.join(member).join("Cargo.toml"),
                    "dependencies": dependencies
                        .iter()
                        .map(|name| json!({ "name": name }))
                        .collect::<Vec<_>>(),
                })
            })
            .collect();
        Workspace::from_metadata(&json!({ "packages": packages }), &self.root)
            .expect("the fixture workspace")
    }

    fn problems(&self) -> Vec<Problem> {
        problems(
            &self.ledger(),
            &self.workspace_with(&[("crates/member", &[])]),
            &self.root,
            NOW,
        )
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}
