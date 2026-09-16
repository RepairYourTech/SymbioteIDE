//! This repository's own spike contract (#173): #38's proof obligations as
//! predeclared data, and every rule that must fail when a choice is settled by a
//! run that does not meet them.
//!
//! The real-tree tests assert the committed contract against the committed
//! ledger. The fixture tests build a throwaway tree and break exactly one thing
//! at a time — including the whole path through to a settled choice, so the
//! rules are proved to be walkable and not only restrictive.

use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use symbiote_architecture::checks::{Problem, problems};
use symbiote_architecture::ledger::{LEDGER_PATH, Ledger};
use symbiote_architecture::repository::{Workspace, hash};
use symbiote_architecture::spike::{CONTRACTS_PATH, Contracts, Results, fingerprint};
use symbiote_architecture::{SpikeContract, workspace_root};

/// 2026-09-08 UTC, the day ADR-0001 and its ledger were recorded.
const NOW: u64 = 1_788_825_600;
const ADR: &str = "docs/architecture/adr-0001-technology-direction.md";
const ID: &str = "TEST/SHELL";
const CONTRACT: &str = "TEST/SHELL-CONTRACT";
const RESULTS: &str = "docs/proofs/results/desktop-shell.json";
const SHELL_CONTRACT: &str = "#38/desktop-shell-representative-workload";

fn root() -> PathBuf {
    workspace_root()
}

fn ledger() -> Ledger {
    Ledger::read(&root().join(LEDGER_PATH)).expect("the committed decision ledger")
}

fn contracts() -> Contracts {
    Contracts::read(&root().join(CONTRACTS_PATH)).expect("the committed spike contracts")
}

fn real_problems() -> Vec<Problem> {
    let contracts = contracts();
    problems(
        &ledger(),
        &contracts,
        &Workspace::read(&root()).expect("cargo reports this workspace"),
        &root(),
        NOW,
    )
}

#[test]
fn the_committed_contract_passes_every_rule() {
    assert_eq!(real_problems(), Vec::new());
}

#[test]
fn the_shell_choice_names_the_contract_that_would_settle_it() {
    let ledger = ledger();
    let contracts = contracts();
    let record = ledger
        .decision("ADR-0001/DESKTOP-SHELL")
        .expect("the shell decision");
    assert_eq!(record.proof_contract.as_deref(), Some(SHELL_CONTRACT));
    let contract = contracts
        .contract(SHELL_CONTRACT)
        .expect("the shell contract");
    assert_eq!(contract.decision, record.draft.id);
    assert_eq!(record.blocking_issue, Some(38));
    assert!(
        !root().join(&contract.result_artifact).exists(),
        "nothing has been measured yet, so the run the contract points at is absent"
    );
}

#[test]
fn the_contract_predeclares_finite_thresholds_and_the_fingerprint_pins_them() {
    let contract = contracts()
        .contract(SHELL_CONTRACT)
        .expect("the shell contract")
        .clone();
    assert!(
        contract.measurements.len() >= 8,
        "a representative workload is measured on more than a couple of quantities"
    );
    let mut seen = std::collections::BTreeSet::new();
    for measurement in &contract.measurements {
        assert!(
            measurement.maximum.is_finite() && measurement.maximum >= 0.0,
            "{}: {:?}",
            measurement.name,
            measurement.maximum
        );
        assert!(!measurement.name.trim().is_empty() && !measurement.unit.trim().is_empty());
        assert!(seen.insert(&measurement.name), "{} twice", measurement.name);
    }
    let mut moved = contract.clone();
    moved.measurements[0].maximum += 1.0;
    assert_ne!(
        fingerprint(&contract),
        fingerprint(&moved),
        "a moved threshold must not keep a result valid"
    );
}

#[test]
fn every_obligation_belongs_to_an_issue_the_choice_owns() {
    let contract = contracts()
        .contract(SHELL_CONTRACT)
        .expect("the shell contract")
        .clone();
    let mut cited = std::collections::BTreeSet::new();
    for entry in contract.workload.iter().chain(&contract.stop_conditions) {
        let tag = entry
            .split_once(':')
            .map(|(tag, _)| tag.trim())
            .unwrap_or_default();
        assert_eq!(tag, "#38", "{entry:?} names no owned clause");
        cited.insert(tag.to_string());
    }
    assert!(cited.contains("#38"));
    assert!(contract.cleanup.contains("no user-display use"));
}

#[test]
fn a_contract_that_settles_a_decision_the_ledger_does_not_hold_is_refused() {
    let mut contract = fixture_contract();
    contract["decision"] = json!("TEST/UNKNOWN");
    let fixture = Fixture::new(fixture_ledger(), vec![contract], None);
    let found = fixture.problems();
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].subject, format!("contract {CONTRACT}"));
    assert!(found[0].detail.contains("TEST/UNKNOWN"));
}

#[test]
fn a_choice_whose_contract_does_not_name_it_back_is_refused() {
    let mut value = fixture_ledger();
    value["decisions"][0]["proof_contract"] = Value::Null;
    let fixture = Fixture::new(value, vec![fixture_contract()], None);
    let found = fixture.problems();
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(
        found[0]
            .detail
            .contains("does not name this contract as its proof")
    );
}

#[test]
fn a_choice_naming_a_contract_the_repository_does_not_hold_is_refused() {
    let fixture = Fixture::new(fixture_ledger(), Vec::new(), None);
    let found = fixture.problems();
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].subject, format!("{ID}: proof contract"));
    assert!(found[0].detail.contains("does not hold"));
}

#[test]
fn an_obligation_naming_no_owned_clause_is_refused() {
    let mut contract = fixture_contract();
    contract["workload"] = json!(["#38: four concurrent agent streams", "no clause at all"]);
    let fixture = Fixture::new(fixture_ledger(), vec![contract], None);
    let found = fixture.problems();
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].detail.contains("no clause at all"));
    assert!(found[0].detail.contains("names no clause of TEST/SHELL"));
    assert!(found[0].detail.contains("so nothing owns it"));
}

#[test]
fn an_issue_the_choice_names_that_no_obligation_addresses_is_refused() {
    let mut value = fixture_ledger();
    value["decisions"][0]["draft"]["issue_refs"] = json!([38, 233]);
    let fixture = Fixture::new(value, vec![fixture_contract()], None);
    let found = fixture.problems();
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].detail.contains("addresses no obligation to #233"));
}

#[test]
fn a_decided_choice_with_no_run_is_refused() {
    let fixture = Fixture::new(accepted_ledger(), vec![fixture_contract()], None);
    let found = fixture.problems();
    assert_eq!(found.len(), 2, "{found:?}");
    assert!(
        found
            .iter()
            .all(|problem| problem.subject.starts_with("contract"))
    );
    assert!(
        found
            .iter()
            .any(|problem| problem.detail.contains("no run settles it"))
    );
    assert!(found.iter().any(|problem| {
        problem
            .detail
            .contains("does not cite it as its own evidence")
    }));
}

#[test]
fn a_partial_run_is_recorded_without_settling_the_choice() {
    let mut results = results_value(json!([]));
    results["untested_platforms"] = json!(["windows-11"]);
    let fixture = Fixture::new(fixture_ledger(), vec![fixture_contract()], Some(results));
    assert_eq!(
        fixture.problems(),
        Vec::new(),
        "an open choice may record an incomplete run; it may not settle on one"
    );
}

#[test]
fn a_run_that_ends_on_a_stop_condition_does_not_settle_the_choice() {
    let mut results =
        results_value(json!([{ "measurement": "cold_start_seconds", "observed": 2.0 }]));
    results["outcome"] = json!("stop_condition_triggered");
    results["stop_condition"] = json!("a native command was allowed from Preview");
    let fixture = Fixture::new(accepted_ledger(), vec![fixture_contract()], Some(results));
    let found = fixture.problems();
    assert!(found.iter().any(|problem| {
        problem
            .detail
            .contains("ended on a stop condition: a native command was allowed from Preview")
    }));
    assert!(
        found
            .iter()
            .any(|problem| problem.detail.contains("does not settle the choice"))
    );
}

#[test]
fn a_run_that_leaves_a_platform_untested_does_not_settle_the_choice() {
    let mut results =
        results_value(json!([{ "measurement": "cold_start_seconds", "observed": 2.0 }]));
    results["untested_platforms"] = json!(["windows-11", "macos-14"]);
    let fixture = Fixture::new(accepted_ledger(), vec![fixture_contract()], Some(results));
    let found = fixture.problems();
    assert!(found.iter().any(|problem| {
        problem.detail.contains("2 platform(s) were not exercised")
            && problem.detail.contains("windows-11, macos-14")
    }));
}

#[test]
fn a_run_that_omits_a_predeclared_measurement_does_not_settle_the_choice() {
    let fixture = Fixture::new(
        accepted_ledger(),
        vec![fixture_contract()],
        Some(results_value(json!([]))),
    );
    let found = fixture.problems();
    assert!(found.iter().any(|problem| {
        problem
            .detail
            .contains("predeclares cold_start_seconds and the run recorded no observation")
    }));
}

#[test]
fn an_observation_above_the_predeclared_maximum_does_not_settle_the_choice() {
    let fixture = Fixture::new(
        citing_its_own_run(accepted_ledger()),
        vec![fixture_contract()],
        Some(results_value(
            json!([{ "measurement": "cold_start_seconds", "observed": 4.5 }]),
        )),
    );
    let found = fixture.problems();
    assert!(found.iter().any(|problem| {
        problem
            .detail
            .contains("cold_start_seconds observed 4.5 s, above the predeclared maximum of 3.0")
    }));
}

#[test]
fn a_run_measured_against_a_moved_threshold_does_not_settle_the_choice() {
    let mut results =
        results_value(json!([{ "measurement": "cold_start_seconds", "observed": 2.0 }]));
    results["contract_sha256"] = json!("0".repeat(64));
    let fixture = Fixture::new(accepted_ledger(), vec![fixture_contract()], Some(results));
    let found = fixture.problems();
    assert!(found.iter().any(|problem| {
        problem
            .detail
            .contains("its thresholds have moved since the run")
    }));
}

#[test]
fn a_run_that_observes_something_the_contract_does_not_predeclare_is_refused() {
    let fixture = Fixture::new(
        citing_its_own_run(accepted_ledger()),
        vec![fixture_contract()],
        Some(results_value(json!([
            { "measurement": "cold_start_seconds", "observed": 2.0 },
            { "measurement": "a_quantity_nobody_declared", "observed": 1.0 }
        ]))),
    );
    let found = fixture.problems();
    assert!(found.iter().any(|problem| {
        problem.detail.contains(
            "the run observes a_quantity_nobody_declared, which the contract does not predeclare",
        )
    }));
}

#[test]
fn a_run_citing_a_raw_artifact_the_tree_does_not_hold_is_refused() {
    let mut results =
        results_value(json!([{ "measurement": "cold_start_seconds", "observed": 2.0 }]));
    results["artifacts"] = json!([
        { "artifact": "docs/proofs/results/raw/run-1.json", "sha256": "a".repeat(64) }
    ]);
    let fixture = Fixture::new(fixture_ledger(), vec![fixture_contract()], Some(results));
    let found = fixture.problems();
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(
        found[0]
            .detail
            .contains("docs/proofs/results/raw/run-1.json, which is not in the tree")
    );
}

#[test]
fn a_run_citing_a_raw_artifact_that_has_changed_is_refused() {
    let mut results =
        results_value(json!([{ "measurement": "cold_start_seconds", "observed": 2.0 }]));
    results["artifacts"] = json!([{ "artifact": ADR, "sha256": "b".repeat(64) }]);
    let fixture = Fixture::new(fixture_ledger(), vec![fixture_contract()], Some(results));
    let found = fixture.problems();
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(
        found[0]
            .detail
            .contains("has changed since the run cited it")
    );
}

#[test]
fn the_settled_path_passes_when_the_run_is_complete_and_in_threshold() {
    let fixture = Fixture::new(
        citing_its_own_run(accepted_ledger()),
        vec![fixture_contract()],
        Some(results_value(
            json!([{ "measurement": "cold_start_seconds", "observed": 2.5 }]),
        )),
    );
    assert_eq!(fixture.problems(), Vec::new());
}

#[test]
fn a_decided_choice_that_does_not_cite_its_own_run_is_refused() {
    let fixture = Fixture::new(
        accepted_ledger(),
        vec![fixture_contract()],
        Some(results_value(
            json!([{ "measurement": "cold_start_seconds", "observed": 2.5 }]),
        )),
    );
    let found = fixture.problems();
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(
        found[0]
            .detail
            .contains("does not cite it as its own evidence")
    );
}

#[test]
fn a_contract_document_that_names_one_identity_twice_is_refused() {
    let fixture = Fixture::new(
        fixture_ledger(),
        vec![fixture_contract(), fixture_contract()],
        None,
    );
    fixture.write(
        CONTRACTS_PATH,
        &json!({
            "schema_version": 1,
            "contracts": [fixture_contract(), fixture_contract()]
        }),
    );
    let error = Contracts::read(&fixture.path(CONTRACTS_PATH)).expect_err("one identity, twice");
    assert!(
        error.0.contains("two contracts share this identity"),
        "{error}"
    );
}

#[test]
fn a_contract_document_of_an_unknown_schema_version_is_refused() {
    let fixture = Fixture::new(fixture_ledger(), vec![fixture_contract()], None);
    let mut value = json!({ "schema_version": 2, "contracts": [fixture_contract()] });
    value["contracts"][0]["schema_version"] = json!(2);
    fixture.write(CONTRACTS_PATH, &value);
    let error = Contracts::read(&fixture.path(CONTRACTS_PATH))
        .expect_err("a future contract schema is refused");
    assert!(error.0.contains("unsupported spike contracts schema"));
}

#[test]
fn a_contract_document_with_an_unknown_field_or_no_contract_is_refused() {
    let fixture = Fixture::new(fixture_ledger(), vec![fixture_contract()], None);
    fixture.write(
        CONTRACTS_PATH,
        &json!({ "schema_version": 1, "contracts": [fixture_contract()], "runs": [] }),
    );
    let error =
        Contracts::read(&fixture.path(CONTRACTS_PATH)).expect_err("an unknown field is refused");
    assert!(error.0.contains("unknown field"), "{error}");

    fixture.write(
        CONTRACTS_PATH,
        &json!({ "schema_version": 1, "contracts": [] }),
    );
    let error =
        Contracts::read(&fixture.path(CONTRACTS_PATH)).expect_err("an empty document is refused");
    assert!(error.0.contains("holds no contract"), "{error}");
}

#[test]
fn a_result_artifact_of_an_unknown_schema_version_is_refused() {
    let fixture = Fixture::new(fixture_ledger(), vec![fixture_contract()], None);
    let mut results =
        results_value(json!([{ "measurement": "cold_start_seconds", "observed": 2.0 }]));
    results["schema_version"] = json!(2);
    fixture.write(RESULTS, &results);
    let error = Results::read(&fixture.path(RESULTS)).expect_err("a future results schema");
    assert!(
        error.0.contains("unsupported spike results schema"),
        "{error}"
    );
}

/// A ledger for one choice that is still investigating, valid as written.
fn fixture_ledger() -> Value {
    json!({
        "schema_version": 1,
        "decisions": [
            {
                "published": { "state": "investigating", "revision": 2 },
                "draft": {
                    "schema_version": 1,
                    "id": ID,
                    "revision": 1,
                    "state": "investigating",
                    "title": "A shell candidate",
                    "constraints": ["no shell is final before the contract is met"],
                    "alternatives": ["a non-Electron shell that passes the workload"],
                    "consequences": ["the choice stays unproven"],
                    "security": "Preview is untrusted and inherits no authority",
                    "reversal": "a shell is not chosen by preference",
                    "high_reversal_cost": true,
                    "issue_refs": [38],
                    "requirement_refs": ["FND-05"],
                    "graph_refs": [],
                    "deployment_refs": [],
                    "compatibility": [],
                    "evidence": [],
                    "supersedes": null
                },
                "record": ADR,
                "record_sha256": "",
                "blocking_issue": 38,
                "proof_contract": CONTRACT
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
                "status": "provisional",
                "blocking_issue": 38
            }
        ]
    })
}

/// The same choice, settled on the run the contract points at.
fn accepted_ledger() -> Value {
    let mut value = fixture_ledger();
    value["decisions"][0]["published"] = json!({ "state": "accepted", "revision": 2 });
    value["decisions"][0]["acceptance"] = json!({
        "actor": "client",
        "authority": "client",
        "now": NOW,
        "client_exception": null
    });
    value["decisions"][0]["blocking_issue"] = Value::Null;
    value["decisions"][0]["draft"]["evidence"] = json!([{
        "artifact": ADR,
        "sha256": "",
        "method": "the recorded text the choice was accepted from",
        "observed_at": NOW,
        "reviewer": "the client instruction this record interprets"
    }]);
    value["artifacts"][0]["status"] = json!("settled");
    value["artifacts"][0]["blocking_issue"] = Value::Null;
    value
}

/// A contract for that choice, predeclaring one measurement.
fn fixture_contract() -> Value {
    json!({
        "schema_version": 1,
        "id": CONTRACT,
        "decision": ID,
        "hypothesis": "the candidate meets every predeclared ceiling on the reference configuration",
        "workload": ["#38: four concurrent agent streams"],
        "platform": "Linux Wayland",
        "hardware": "four cores, 16 GiB",
        "method": "sample the candidate's own process tree at 100 ms",
        "measurements": [
            { "name": "cold_start_seconds", "unit": "s", "maximum": 3.0 }
        ],
        "stop_conditions": ["#38: a blank-window figure stops the run"],
        "result_artifact": RESULTS,
        "cleanup": "remove the disposable display and only the identities the driver observed"
    })
}

/// The same choice, settling on the run the contract points at: the results
/// artifact is the evidence the acceptance rests on.
fn citing_its_own_run(mut value: Value) -> Value {
    value["decisions"][0]["draft"]["evidence"] = json!([{
        "artifact": RESULTS,
        "sha256": "",
        "method": "the spike published its raw data and failures",
        "observed_at": NOW,
        "reviewer": "the run's own record"
    }]);
    value
}

/// A run of that contract, with the fingerprint filled in by the fixture.
fn results_value(observations: Value) -> Value {
    json!({
        "schema_version": 1,
        "contract": CONTRACT,
        "contract_sha256": "",
        "outcome": "within_thresholds",
        "untested_platforms": [],
        "observations": observations,
        "artifacts": []
    })
}

static NEXT: AtomicUsize = AtomicUsize::new(0);

/// A throwaway tree holding one ledger, one contract document and at most one
/// run, removed when it drops.
struct Fixture {
    root: PathBuf,
    contracts: Contracts,
}

impl Fixture {
    fn new(ledger: Value, contracts: Vec<Value>, results: Option<Value>) -> Fixture {
        let root = std::env::temp_dir().join(format!(
            "symbiote-spikes-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::SeqCst)
        ));
        std::fs::create_dir_all(root.join("docs/proofs/results")).expect("a fixture tree");
        std::fs::create_dir_all(root.join("docs/architecture")).expect("a fixture tree");
        let root = root.canonicalize().expect("a canonical fixture root");
        std::fs::write(root.join(ADR), "the accepted text this record interprets\n")
            .expect("the fixture record");
        let contracts = Contracts {
            schema_version: 1,
            contracts: contracts
                .into_iter()
                .map(|value| {
                    serde_json::from_value::<SpikeContract>(value).expect("a fixture contract")
                })
                .collect(),
        };
        let fixture = Fixture { root, contracts };
        if let Some(mut results) = results {
            if results["contract_sha256"] == json!("") {
                results["contract_sha256"] = json!(fingerprint(&fixture.contracts.contracts[0]));
            }
            fixture.write(RESULTS, &results);
        }
        let mut ledger = ledger;
        if ledger["decisions"][0]["record_sha256"] == json!("") {
            ledger["decisions"][0]["record_sha256"] =
                json!(hash(&fixture.root, ADR).expect("the fixture record hashes"));
        }
        for evidence in ledger["decisions"][0]["draft"]["evidence"]
            .as_array_mut()
            .into_iter()
            .flatten()
        {
            if evidence["sha256"] == json!("") {
                let artifact = evidence["artifact"].as_str().expect("a cited artifact");
                evidence["sha256"] =
                    json!(hash(&fixture.root, artifact).expect("the cited artifact hashes"));
            }
        }
        fixture.write(LEDGER_PATH, &ledger);
        fixture
    }

    fn path(&self, relative: &str) -> PathBuf {
        self.root.join(relative)
    }

    fn write(&self, relative: &str, value: &Value) {
        std::fs::write(
            self.path(relative),
            serde_json::to_vec_pretty(value).expect("readable JSON"),
        )
        .expect("the fixture file");
    }

    fn problems(&self) -> Vec<Problem> {
        let ledger = Ledger::read(&self.path(LEDGER_PATH)).expect("the fixture ledger");
        let workspace = Workspace::from_metadata(
            &json!({
                "packages": [{
                    "manifest_path": self.root.join("crates/member").join("Cargo.toml"),
                    "dependencies": []
                }]
            }),
            &self.root,
        )
        .expect("the fixture workspace");
        problems(&ledger, &self.contracts, &workspace, &self.root, NOW)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(Path::new(&self.root));
    }
}
