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
const RAW: &str = "docs/proofs/evidence/desktop-shell/run-1.log";
const SHELL_CONTRACT: &str = "#38/desktop-shell-representative-workload";
const OBLIGATION: &str = "#38: four concurrent agent streams";
/// The fixture record's four clauses, word for word. An answer carries the
/// clause it answers rather than a number, so what it claims is the record's own
/// text and a clause cannot be renumbered out from under it.
const CLAUSE_WORKLOAD: &str = "the workload the choice was accepted with";
const CLAUSE_STANDARD: &str = "the standard its runs are held to";
const CLAUSE_METHOD: &str = "the method a run follows.";
const CLAUSE_WIDER: &str = "the wider acceptance this contract is not.";

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

/// Whether a refusal says this, with every refusal printed when none does.
#[track_caller]
fn refused(found: &[Problem], needle: &str) {
    assert!(
        found.iter().any(|problem| problem.detail.contains(needle)),
        "no refusal mentions {needle:?}: {found:?}"
    );
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
    for recorded in [
        "the commit it was built from",
        "the platform and version it exercised",
        "the hardware it ran on",
        "the contract fingerprint",
        "the obligations it exercised",
        "its raw artifacts",
        "the failures it saw",
    ] {
        assert!(
            contract.method.contains(recorded),
            "the method a run is measured by must name {recorded:?}, which the result has to carry: {}",
            contract.method
        );
    }
    assert!(contract.cleanup.contains("no user-display use"));
}

#[test]
fn the_bar_cannot_be_shrunk_by_dropping_an_obligation_from_the_contract() {
    // The audit's own probe: three workload obligations removed from the
    // contract, their answers left behind. Every clause of the accepted bar is
    // read from the record, so the answers point at obligations the contract no
    // longer declares instead of the bar quietly getting smaller.
    let fixture = Fixture::new(fixture_ledger(), vec![fixture_contract()], None);
    let mut value = json!({
        "schema_version": 1,
        "contracts": [fixture_contract()]
    });
    value["contracts"][0]["workload"] = json!(["#38: a different obligation entirely"]);
    fixture.write(CONTRACTS_PATH, &value);
    let error = Contracts::read(&fixture.path(CONTRACTS_PATH))
        .expect_err("a dropped obligation leaves a clause answered by nothing");
    assert!(
        error.0.contains("which this contract does not declare"),
        "{error}"
    );
    assert!(error.0.contains(OBLIGATION), "{error}");
}

#[test]
fn an_obligation_that_answers_no_clause_of_the_accepted_bar_is_refused() {
    let fixture = Fixture::new(fixture_ledger(), vec![fixture_contract()], None);
    let mut value = json!({
        "schema_version": 1,
        "contracts": [fixture_contract()]
    });
    value["contracts"][0]["workload"] = json!([OBLIGATION, "#38: a bar nobody accepted"]);
    fixture.write(CONTRACTS_PATH, &value);
    let error = Contracts::read(&fixture.path(CONTRACTS_PATH))
        .expect_err("a contract cannot declare more bar than its decision was accepted with");
    assert!(
        error.0.contains("answers no clause of the accepted bar"),
        "{error}"
    );
}

#[test]
fn an_answer_that_names_two_carriers_or_none_is_refused() {
    let fixture = Fixture::new(fixture_ledger(), vec![fixture_contract()], None);
    let mut value = json!({
        "schema_version": 1,
        "contracts": [fixture_contract()]
    });
    value["contracts"][0]["answers"][2]["obligation"] = json!(OBLIGATION);
    fixture.write(CONTRACTS_PATH, &value);
    let error = Contracts::read(&fixture.path(CONTRACTS_PATH))
        .expect_err("a clause is answered once, by one of the three");
    assert!(error.0.contains("not 2 of them"), "{error}");

    let mut value = json!({
        "schema_version": 1,
        "contracts": [fixture_contract()]
    });
    value["contracts"][0]["answers"][2] = json!({ "clause": CLAUSE_METHOD });
    fixture.write(CONTRACTS_PATH, &value);
    let error = Contracts::read(&fixture.path(CONTRACTS_PATH))
        .expect_err("a clause is answered by something");
    assert!(error.0.contains("not 0 of them"), "{error}");
}

#[test]
fn an_elsewhere_answer_with_no_reason_is_refused() {
    let fixture = Fixture::new(fixture_ledger(), vec![fixture_contract()], None);
    let mut value = json!({
        "schema_version": 1,
        "contracts": [fixture_contract()]
    });
    value["contracts"][0]["answers"][3]["elsewhere"]["why"] = json!("");
    fixture.write(CONTRACTS_PATH, &value);
    let error = Contracts::read(&fixture.path(CONTRACTS_PATH))
        .expect_err("naming the owner is not saying why the clause is not here");
    assert!(error.0.contains("does not say why"), "{error}");
}
#[test]
fn a_contract_that_answers_no_clause_of_the_bar_is_refused() {
    let fixture = Fixture::new(fixture_ledger(), vec![fixture_contract()], None);
    let mut value = json!({
        "schema_version": 1,
        "contracts": [fixture_contract()]
    });
    value["contracts"][0]["answers"] = json!([]);
    fixture.write(CONTRACTS_PATH, &value);
    let error = Contracts::read(&fixture.path(CONTRACTS_PATH))
        .expect_err("a contract that answers no clause of the bar is not a contract");
    assert!(error.0.contains("answers no clause of the bar"), "{error}");
}

#[test]
fn a_choice_that_names_a_proof_and_no_bar_section_is_refused() {
    let mut value = fixture_ledger();
    value["decisions"][0]["proof_section"] = Value::Null;
    let found = Fixture::new(value, vec![fixture_contract()], None).problems();
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].subject, format!("{ID}: proof section"));
    refused(&found, "the contract would state its own bar");
}

#[test]
fn a_choice_that_names_a_bar_section_and_no_contract_is_refused() {
    let mut value = fixture_ledger();
    value["decisions"][0]["proof_contract"] = Value::Null;
    let found = Fixture::new(value, Vec::new(), None).problems();
    assert_eq!(found.len(), 1, "{found:?}");
    refused(&found, "and no contract answers it");
}

#[test]
fn a_clause_of_the_accepted_bar_that_nothing_answers_is_refused() {
    let mut contract = fixture_contract();
    contract["answers"]
        .as_array_mut()
        .expect("the fixture's answers")
        .retain(|answer| answer["clause"] != json!(CLAUSE_METHOD));
    let found = Fixture::new(fixture_ledger(), vec![contract], None).problems();
    assert_eq!(found.len(), 1, "{found:?}");
    refused(&found, "states a clause this contract answers with nothing");
    refused(&found, "the method a run follows");
}

#[test]
fn an_answer_naming_an_issue_the_choice_does_not_own_is_refused() {
    let mut contract = fixture_contract();
    contract["answers"][3]["elsewhere"]["issue"] = json!(233);
    let found = Fixture::new(fixture_ledger(), vec![contract], None).problems();
    assert_eq!(found.len(), 1, "{found:?}");
    refused(
        &found,
        "answered by #233, which this choice does not name as its own",
    );
}

#[test]
fn an_answer_whose_words_the_section_does_not_state_is_refused() {
    // A clause's identity is the accepted text, so an answer carries the words
    // it answers and the record is asked whether it states them: a paraphrase,
    // a trivially edited clause, and a clause from somewhere else in the record
    // are all words the bar does not state.
    let mut contract = fixture_contract();
    contract["answers"][2]["clause"] = json!("the method a run never follows");
    let found = Fixture::new(fixture_ledger(), vec![contract], None).problems();
    // Two refusals, one per direction: the words are not the bar's, and the
    // clause they displaced is now answered by nothing.
    assert_eq!(found.len(), 2, "{found:?}");
    refused(&found, "does not state as a clause of the accepted bar");
    refused(&found, "the method a run never follows");

    let mut contract = fixture_contract();
    contract["answers"][2]["clause"] = json!("the method a run follows");
    let found = Fixture::new(fixture_ledger(), vec![contract], None).problems();
    refused(&found, "does not state as a clause of the accepted bar");

    let mut contract = fixture_contract();
    contract["answers"][2]["clause"] = json!("nothing yet.");
    let found = Fixture::new(fixture_ledger(), vec![contract], None).problems();
    refused(&found, "does not state as a clause of the accepted bar");
}

#[test]
fn an_answer_whose_words_are_stated_twice_by_the_section_is_refused() {
    // Identity by words needs the words to name one clause: a section that
    // states the same clause twice would otherwise let one answer stand for
    // both twins, which is a bar answered by nothing wearing an answer's face.
    let mut contract = fixture_contract();
    contract["answers"][0]["clause"] = json!("the same words");
    let found =
        Fixture::new_with_record(TWICE_RECORD, fixture_ledger(), vec![contract], None).problems();
    refused(&found, "are 2 clauses of");
}

#[test]
fn a_contract_document_that_names_a_clause_by_its_number_is_refused_naming_what_moved() {
    // The shape this loader read before #607 was accepted: the answer named its
    // clause by a number. Version 1 absorbs the move, so the refusal says what
    // moved and what to write instead rather than reporting a type error.
    let fixture = Fixture::new(fixture_ledger(), vec![fixture_contract()], None);
    let mut value = json!({
        "schema_version": 1,
        "contracts": [fixture_contract()]
    });
    value["contracts"][0]["answers"][0]["clause"] = json!(1);
    fixture.write(CONTRACTS_PATH, &value);
    let error = Contracts::read(&fixture.path(CONTRACTS_PATH))
        .expect_err("a clause named by its number is the shape before the move");
    assert!(error.0.contains("by its own words"), "{error}");
    assert!(error.0.contains("`clause` number"), "{error}");
}

#[test]
fn a_bar_whose_accepted_record_is_not_in_the_tree_is_refused() {
    let fixture = Fixture::new(fixture_ledger(), vec![fixture_contract()], None);
    std::fs::remove_file(fixture.path(ADR)).expect("the fixture record is removable");
    let found = fixture.problems();
    refused(&found, "which cannot be read");
}

#[test]
fn a_bar_named_from_a_section_the_record_does_not_state_is_refused() {
    let mut value = fixture_ledger();
    value["decisions"][0]["proof_section"] = json!("A Section Nobody Wrote");
    let found = Fixture::new(value, vec![fixture_contract()], None).problems();
    assert_eq!(found.len(), 1, "{found:?}");
    refused(&found, "that record states no such section");
}

#[test]
fn the_committed_choice_names_the_section_its_bar_comes_from() {
    // The bar's location is the choice's, not the contract's: if this heading
    // moved, a contract could be answered against a smaller part of the record
    // while every rule still passed. The heading is the record's own, so this
    // pin fails loudly rather than letting the bar be chosen after the fact.
    let record = ledger()
        .decision("ADR-0001/DESKTOP-SHELL")
        .expect("the shell decision")
        .clone();
    assert_eq!(
        record.proof_section.as_deref(),
        Some("Proof contract and stop conditions"),
        "the choice names where its bar lives"
    );
    let accepted = std::fs::read_to_string(root().join(ADR)).expect("the accepted record");
    let body = symbiote_architecture::spike::section(
        &accepted,
        record.proof_section.as_deref().expect("a named section"),
    )
    .expect("the record states the section the choice names");
    let clauses = symbiote_architecture::spike::clauses(&body);
    let contract = contracts()
        .contract(SHELL_CONTRACT)
        .expect("the shell contract")
        .clone();
    let answered: std::collections::BTreeSet<&str> = contract
        .answers
        .iter()
        .map(|answer| answer.clause.as_str())
        .collect();
    assert_eq!(
        answered.len(),
        clauses.len(),
        "{answered:?} against {clauses:?}"
    );
    assert!(
        clauses
            .iter()
            .all(|clause| answered.contains(clause.as_str())),
        "every clause of the accepted bar has an answer, in the record's own words: {clauses:?}"
    );
    assert!(
        answered
            .iter()
            .all(|words| clauses.iter().any(|clause| clause == words)),
        "every answer carries a clause the accepted section states: {answered:?}"
    );
    assert!(
        contract
            .answers
            .iter()
            .any(|answer| answer.obligation.is_some())
            && contract
                .answers
                .iter()
                .any(|answer| answer.elsewhere.is_some()),
        "the bar is answered by this contract's obligations and, where a clause belongs to #38's wider acceptance, by naming that"
    );
}

#[test]
fn a_contract_that_settles_a_decision_the_ledger_does_not_hold_is_refused() {
    let mut contract = fixture_contract();
    contract["decision"] = json!("TEST/UNKNOWN");
    let fixture = Fixture::new(fixture_ledger(), vec![contract], None);
    let found = fixture.problems();
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].subject, format!("contract {CONTRACT}"));
    refused(&found, "TEST/UNKNOWN");
}

#[test]
fn a_choice_whose_contract_does_not_name_it_back_is_refused() {
    let mut value = fixture_ledger();
    value["decisions"][0]["proof_contract"] = Value::Null;
    value["decisions"][0]["proof_section"] = Value::Null;
    let fixture = Fixture::new(value, vec![fixture_contract()], None);
    let found = fixture.problems();
    assert_eq!(found.len(), 1, "{found:?}");
    refused(&found, "does not name this contract as its proof");
}

#[test]
fn a_choice_naming_a_contract_the_repository_does_not_hold_is_refused() {
    let fixture = Fixture::new(fixture_ledger(), Vec::new(), None);
    let found = fixture.problems();
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].subject, format!("{ID}: proof contract"));
    refused(&found, "does not hold");
}

#[test]
fn an_obligation_naming_no_owned_clause_is_refused() {
    let mut contract = fixture_contract();
    contract["workload"] = json!([OBLIGATION, "no clause at all"]);
    let fixture = Fixture::new(fixture_ledger(), vec![contract], None);
    let found = fixture.problems();
    assert_eq!(found.len(), 1, "{found:?}");
    refused(&found, "no clause at all");
    refused(&found, "names no clause of TEST/SHELL");
    refused(&found, "so nothing owns it");
}

#[test]
fn an_issue_the_choice_names_that_no_obligation_addresses_is_refused() {
    let mut value = fixture_ledger();
    value["decisions"][0]["draft"]["issue_refs"] = json!([38, 233]);
    let fixture = Fixture::new(value, vec![fixture_contract()], None);
    let found = fixture.problems();
    assert_eq!(found.len(), 1, "{found:?}");
    refused(&found, "addresses no obligation to #233");
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
    refused(&found, "no run settles it");
    refused(&found, "does not cite it as its own evidence");
}

#[test]
fn a_decided_choice_that_does_not_cite_its_own_run_is_refused() {
    let found = Fixture::new(
        accepted_ledger(),
        vec![fixture_contract()],
        Some(results_value(json!([run_value(observations())]))),
    )
    .problems();
    assert_eq!(found.len(), 1, "{found:?}");
    refused(&found, "does not cite it as its own evidence");
}

#[test]
fn a_partial_run_is_recorded_without_settling_the_choice() {
    let mut run = run_value(json!([]));
    run["exercised"] = json!([]);
    run["artifacts"] = json!([]);
    let mut results = results_value(json!([run]));
    results["untested_platforms"] = json!(["windows-11"]);
    let fixture = Fixture::new(fixture_ledger(), vec![fixture_contract()], Some(results));
    assert_eq!(
        fixture.problems(),
        Vec::new(),
        "an open choice may record an incomplete run; it may not settle on one"
    );
}

#[test]
fn a_result_that_records_no_run_does_not_settle_the_choice() {
    let found = settled(json!([])).problems();
    assert_eq!(found.len(), 1, "{found:?}");
    refused(&found, "records no run, so nothing has been measured");
}

#[test]
fn a_run_that_names_no_revision_does_not_settle_the_choice() {
    let found = settled_run(|run| run["commit"] = json!("unknown"));
    assert_eq!(found.len(), 1, "{found:?}");
    refused(&found, "names no revision");
    refused(&found, "\"unknown\" is not a commit hash");
}

#[test]
fn a_run_that_names_no_platform_does_not_settle_the_choice() {
    let found = settled_run(|run| run["platform"] = json!(""));
    // Two facts, said once each: the run does not say where it was measured, and
    // the platform the contract applies to is therefore unaccounted for.
    assert_eq!(found.len(), 2, "{found:?}");
    refused(
        &found,
        "a run names no platform, so nothing says where it was measured",
    );
    refused(
        &found,
        "leaves \"linux-wayland\" neither measured nor declared untested",
    );
}

#[test]
fn a_run_that_names_no_platform_version_does_not_settle_the_choice() {
    let found = settled_run(|run| run["version"] = json!("  "));
    assert_eq!(found.len(), 1, "{found:?}");
    refused(
        &found,
        "records no platform version, so its platform is unpinned",
    );
}

#[test]
fn a_run_that_names_no_hardware_does_not_settle_the_choice() {
    let found = settled_run(|run| run["hardware"] = json!(""));
    assert_eq!(found.len(), 1, "{found:?}");
    refused(
        &found,
        "records no hardware, so its figures carry no reference configuration",
    );
}

#[test]
fn a_run_that_exercises_part_of_the_workload_does_not_settle_the_choice() {
    let found = settled_run(|run| run["exercised"] = json!([]));
    assert_eq!(found.len(), 1, "{found:?}");
    refused(&found, "does not attest");
    refused(&found, "which the contract's workload declares");
}

#[test]
fn a_run_that_attests_an_obligation_the_contract_never_declared_is_refused() {
    let found = settled_run(|run| {
        run["exercised"] = json!([OBLIGATION, "#38: a workload nobody declared"])
    });
    assert_eq!(found.len(), 1, "{found:?}");
    refused(
        &found,
        "attests \"#38: a workload nobody declared\", which the contract's workload never declares",
    );
}

#[test]
fn a_settlement_on_a_run_that_published_nothing_is_refused() {
    let found = settled_run(|run| run["artifacts"] = json!([]));
    assert_eq!(found.len(), 1, "{found:?}");
    refused(
        &found,
        "publishes no raw artifact, so nothing stands behind its figures",
    );
}

#[test]
fn a_run_that_lists_the_platform_it_exercised_as_untested_is_refused() {
    let mut results = results_value(json!([run_value(observations())]));
    results["untested_platforms"] = json!(["linux-wayland"]);
    let found = Fixture::new(
        citing_its_own_run(accepted_ledger()),
        vec![fixture_contract()],
        Some(results),
    )
    .problems();
    refused(
        &found,
        "lists linux-wayland, the platform that run exercised, as untested",
    );
}

#[test]
fn a_run_that_records_a_failure_with_nothing_said_about_it_is_refused() {
    let found = settled_run(|run| run["failures"] = json!(["  "]));
    assert_eq!(found.len(), 1, "{found:?}");
    refused(&found, "lists a failure with nothing said about it");
}

#[test]
fn a_run_that_stops_without_recording_the_failure_it_stopped_on_is_refused() {
    let found = settled_run(|run| {
        run["outcome"] = json!("stop_condition_triggered");
        run["stop_condition"] = json!(STOP_CONDITION);
    });
    refused(&found, "does not settle the choice");
    refused(&found, "does not record it among its failures");
}

#[test]
fn a_run_that_stops_on_a_condition_the_contract_never_declared_is_refused() {
    let found = settled_run(|run| {
        run["outcome"] = json!("stop_condition_triggered");
        run["stop_condition"] = json!("#38: nobody declared this stop");
        run["failures"] = json!(["#38: nobody declared this stop"]);
    });
    assert_eq!(found.len(), 2, "{found:?}");
    refused(&found, "does not settle the choice");
    refused(&found, "which the contract's stop conditions never declare");
}

#[test]
fn a_run_that_ends_on_a_stop_condition_does_not_settle_the_choice() {
    let found = settled_run(|run| {
        run["outcome"] = json!("stop_condition_triggered");
        run["stop_condition"] = json!(STOP_CONDITION);
        run["failures"] = json!([STOP_CONDITION]);
    });
    assert_eq!(found.len(), 1, "{found:?}");
    refused(&found, "does not settle the choice");
}

#[test]
fn a_run_that_leaves_a_platform_untested_does_not_settle_the_choice() {
    let mut results = results_value(json!([run_value(observations())]));
    results["untested_platforms"] = json!(["windows-11"]);
    let found = Fixture::new(
        citing_its_own_run(accepted_ledger()),
        vec![spanning_contract()],
        Some(results),
    )
    .problems();
    assert_eq!(found.len(), 1, "{found:?}");
    refused(&found, "1 platform(s) were not exercised");
    refused(&found, "windows-11");
}

#[test]
fn a_result_that_leaves_an_applicable_platform_unaccounted_for_is_refused() {
    // The settlement the audit built: one platform measured, nothing declared
    // untested, and a contract that applies to two.
    let found = Fixture::new(
        citing_its_own_run(accepted_ledger()),
        vec![spanning_contract()],
        Some(results_value(json!([run_value(observations())]))),
    )
    .problems();
    assert_eq!(found.len(), 1, "{found:?}");
    refused(
        &found,
        "leaves \"windows-11\" neither measured nor declared untested",
    );
}

#[test]
fn a_result_that_declares_a_platform_the_contract_does_not_apply_to_is_refused() {
    let mut results = results_value(json!([run_value(observations())]));
    results["untested_platforms"] = json!(["windows-11"]);
    let found = Fixture::new(
        citing_its_own_run(accepted_ledger()),
        vec![fixture_contract()],
        Some(results),
    )
    .problems();
    refused(
        &found,
        "declares \"windows-11\" untested, which the contract does not apply to",
    );
}

#[test]
fn a_contract_that_declares_no_applicable_platform_is_refused() {
    let fixture = Fixture::new(fixture_ledger(), vec![fixture_contract()], None);
    let mut value = json!({ "schema_version": 1, "contracts": [fixture_contract()] });
    value["contracts"][0]["applicable_platforms"] = json!([]);
    fixture.write(CONTRACTS_PATH, &value);
    let error = Contracts::read(&fixture.path(CONTRACTS_PATH))
        .expect_err("a contract applying to nothing accounts for nothing");
    assert!(error.0.contains("must be named once each"), "{error}");
}

#[test]
fn a_contract_that_names_one_applicable_platform_twice_is_refused() {
    let fixture = Fixture::new(fixture_ledger(), vec![fixture_contract()], None);
    let mut value = json!({ "schema_version": 1, "contracts": [fixture_contract()] });
    value["contracts"][0]["applicable_platforms"] = json!(["linux-wayland", "Linux-Wayland"]);
    fixture.write(CONTRACTS_PATH, &value);
    let error = Contracts::read(&fixture.path(CONTRACTS_PATH))
        .expect_err("one platform named twice is one platform");
    assert!(error.0.contains("must be named once each"), "{error}");
}

#[test]
fn the_committed_contract_names_the_platforms_it_applies_to_and_its_prose_defers_to_them() {
    let contract = contracts()
        .contract(SHELL_CONTRACT)
        .expect("the shell contract")
        .clone();
    assert_eq!(
        contract.applicable_platforms.len(),
        4,
        "the four platforms the contract applies to belong in data: {:?}",
        contract.applicable_platforms
    );
    assert!(
        contract.platform.contains("applicable_platforms"),
        "the prose has to point at the data rather than restate it: {}",
        contract.platform
    );
}

#[test]
fn a_run_that_omits_a_predeclared_measurement_does_not_settle_the_choice() {
    let found = settled_run(|run| run["observations"] = json!([]));
    assert_eq!(found.len(), 1, "{found:?}");
    refused(
        &found,
        "records no observation for cold_start_seconds, which the contract predeclares",
    );
}

#[test]
fn an_observation_above_the_predeclared_maximum_does_not_settle_the_choice() {
    let found = settled_run(|run| {
        run["observations"] = json!([{ "measurement": "cold_start_seconds", "observed": 4.5 }])
    });
    assert_eq!(found.len(), 1, "{found:?}");
    refused(
        &found,
        "observed cold_start_seconds at 4.5 s, above the predeclared maximum of 3.0",
    );
}

#[test]
fn a_run_that_records_one_measurement_twice_does_not_settle_the_choice() {
    let found = settled_run(|run| {
        run["observations"] = json!([
            { "measurement": "cold_start_seconds", "observed": 2.0 },
            { "measurement": "cold_start_seconds", "observed": 2.5 }
        ])
    });
    assert_eq!(found.len(), 1, "{found:?}");
    refused(
        &found,
        "records the predeclared measurement cold_start_seconds 2 times",
    );
}

#[test]
fn a_run_measured_against_a_moved_threshold_does_not_settle_the_choice() {
    let mut results = results_value(json!([run_value(observations())]));
    results["contract_sha256"] = json!("0".repeat(64));
    let found = Fixture::new(
        citing_its_own_run(accepted_ledger()),
        vec![fixture_contract()],
        Some(results),
    )
    .problems();
    refused(&found, "its thresholds have moved since the run");
}

#[test]
fn a_run_that_observes_something_the_contract_does_not_predeclare_is_refused() {
    let found = settled_run(|run| {
        run["observations"] = json!([
            { "measurement": "cold_start_seconds", "observed": 2.0 },
            { "measurement": "a_quantity_nobody_declared", "observed": 1.0 }
        ])
    });
    assert_eq!(found.len(), 1, "{found:?}");
    refused(
        &found,
        "observes a_quantity_nobody_declared, which the contract does not predeclare",
    );
}

#[test]
fn a_settled_run_citing_a_raw_artifact_the_tree_does_not_hold_is_refused() {
    let found = settled_run(|run| {
        run["artifacts"] = json!([
            { "artifact": "docs/proofs/evidence/desktop-shell/absent.log", "sha256": "a".repeat(64) }
        ])
    });
    assert_eq!(found.len(), 1, "{found:?}");
    refused(
        &found,
        "docs/proofs/evidence/desktop-shell/absent.log, which is not in the tree",
    );
}

#[test]
fn a_settled_run_citing_a_raw_artifact_that_has_changed_is_refused() {
    let found = settled_run(|run| {
        run["artifacts"] = json!([{ "artifact": RAW, "sha256": "b".repeat(64) }])
    });
    assert_eq!(found.len(), 1, "{found:?}");
    refused(&found, "has changed since the run cited it");
}

#[test]
fn the_settled_path_passes_when_every_run_is_complete_and_in_threshold() {
    let fixture = Fixture::new(
        citing_its_own_run(accepted_ledger()),
        vec![spanning_contract()],
        Some(results_value(json!([
            run_value(observations()),
            run_on("windows-11")
        ]))),
    );
    assert_eq!(
        fixture.problems(),
        Vec::new(),
        "a run for every platform the contract applies to settles it"
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
fn the_shape_before_the_bar_moved_is_the_derive_refusing_the_stale_field() {
    // The message #601 printed was the derive's, not a rule's: `deny_unknown_fields`
    // rejects `obligations` and the section's replacement appears only in the list of
    // expected fields. The handoff quotes that message as the reason the read path was
    // taught to name the shape, and the #602 wrapper that formatted it no longer
    // exists, so this holds the quotation against the deserializer that produced it.
    // (Nothing here can run the pre-#602 loader itself: the line that formatted the
    // message is gone from the tree.)
    let error = serde_json::from_str::<Contracts>(&pre_bar_move_contract().to_string())
        .expect_err("the shape #601 read is refused by the type, not by a rule");
    let message = error.to_string();
    assert!(message.contains("unknown field `obligations`"), "{message}");
    assert!(message.contains("expected one of"), "{message}");
    assert!(message.contains("`answers`"), "{message}");
}

#[test]
fn a_contract_document_of_the_shape_before_the_bar_moved_is_refused_naming_what_moved() {
    // The shape changed incompatibly while the version stayed 1, so the refusal
    // has to say what moved rather than only which field is missing: a caller
    // holding the old document is told where the section went and that there is
    // no second version to wait for.
    let fixture = Fixture::new(fixture_ledger(), vec![fixture_contract()], None);
    fixture.write(CONTRACTS_PATH, &pre_bar_move_contract());
    let error = Contracts::read(&fixture.path(CONTRACTS_PATH))
        .expect_err("the shape this loader read before the bar moved is refused");
    assert!(
        error
            .0
            .contains("the section moved to the choice's record as `proof_section`"),
        "{error}"
    );
    assert!(error.0.contains("version 1 absorbs that move"), "{error}");
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
    let mut results = results_value(json!([run_value(observations())]));
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
                "proof_contract": CONTRACT,
                "proof_section": SECTION
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

/// A contract for that choice, predeclaring one measurement and one stop
/// condition and answering every clause of the fixture record's bar.
fn fixture_contract() -> Value {
    json!({
        "schema_version": 1,
        "id": CONTRACT,
        "decision": ID,
        "hypothesis": "the candidate meets every predeclared ceiling on the reference configuration",
        "workload": [OBLIGATION],
        "applicable_platforms": ["linux-wayland"],
        "platform": "Applicability is the data above, not this sentence",
        "hardware": "four cores, 16 GiB",
        "method": "sample the candidate's own process tree at 100 ms",
        "measurements": [
            { "name": "cold_start_seconds", "unit": "s", "maximum": 3.0 }
        ],
        "stop_conditions": [STOP_CONDITION],
        "result_artifact": RESULTS,
        "cleanup": "remove the disposable display and only the identities the driver observed",
        "answers": [
            { "clause": CLAUSE_WORKLOAD, "obligation": OBLIGATION },
            { "clause": CLAUSE_STANDARD, "obligation": STOP_CONDITION },
            { "clause": CLAUSE_METHOD, "part": "method" },
            { "clause": CLAUSE_WIDER, "elsewhere": { "issue": 38, "why": "the fixture's own wider acceptance" } }
        ]
    })
}

/// The section of the fixture record that states the bar, and the four clauses
/// the fixture contract answers: the workload, the stop condition, the method a
/// run follows, and one clause that belongs to a wider acceptance.
const SECTION: &str = "Proof contract and stop conditions";
const RECORD: &str = "# The accepted text this record interprets\n\n## Proof contract and stop conditions\n\nthe workload the choice was accepted with; the standard its runs are held to; the method a run follows.\nthe wider acceptance this contract is not.\n\n## Remaining acceptance\n\nnothing yet.\n";

/// The same record stating one clause twice, which is what identity by words
/// cannot resolve: a section has to name each of its clauses once.
const TWICE_RECORD: &str = "# The accepted text this record interprets\n\n## Proof contract and stop conditions\n\nthe same words; the same words; and the wider acceptance this contract is not.\n\n## Remaining acceptance\n\nnothing yet.\n";

const STOP_CONDITION: &str = "#38: a blank-window figure stops the run";

/// A contract document as #601 wrote it: the section it answered on the contract,
/// the answers under it, and no `answers` of its own. Two tests need exactly this
/// shape — one for the message the derive printed before the read path named it,
/// one for the refusal the read path produces now — so it is built once here.
fn pre_bar_move_contract() -> Value {
    let mut value = json!({ "schema_version": 1, "contracts": [fixture_contract()] });
    let contract = value["contracts"][0]
        .as_object_mut()
        .expect("the fixture's contract object");
    let answered = contract.remove("answers").expect("the fixture's answers");
    contract.insert(
        "obligations".into(),
        json!({ "section": SECTION, "answered": answered }),
    );
    value
}

/// The one measurement the fixture contract predeclares, inside its ceiling.
fn observations() -> Value {
    json!([{ "measurement": "cold_start_seconds", "observed": 2.5 }])
}

/// One run that answers everything the contract asks: the platform and version
/// it exercised, the hardware and revision it ran on, the obligation it ran, its
/// measurement, and the raw artifact it published (the fixture fills the hash).
fn run_value(observations: Value) -> Value {
    json!({
        "platform": "linux-wayland",
        "version": "kwin 5.27",
        "hardware": "four cores, 16 GiB",
        "commit": "cd204823d96de468520434e94054ff412b7cc497",
        "exercised": [OBLIGATION],
        "outcome": "within_thresholds",
        "failures": [],
        "observations": observations,
        "artifacts": [{ "artifact": RAW, "sha256": "" }]
    })
}

/// A contract applying to two platforms, so a settlement has to carry a run for
/// each of them rather than for whichever one it happened to measure.
fn spanning_contract() -> Value {
    let mut contract = fixture_contract();
    contract["applicable_platforms"] = json!(["linux-wayland", "windows-11"]);
    contract
}

/// The same run the fixture builds, on the contract's second platform.
fn run_on(platform: &str) -> Value {
    let mut run = run_value(observations());
    run["platform"] = json!(platform);
    run["version"] = json!("11 23H2");
    run
}

/// A dossier of runs, with the fingerprint and raw hashes filled by the fixture.
fn results_value(runs: Value) -> Value {
    json!({
        "schema_version": 1,
        "contract": CONTRACT,
        "contract_sha256": "",
        "untested_platforms": [],
        "runs": runs
    })
}

/// A choice settled on a dossier of runs, valid as written.
fn settled(runs: Value) -> Fixture {
    Fixture::new(
        citing_its_own_run(accepted_ledger()),
        vec![fixture_contract()],
        Some(results_value(runs)),
    )
}

/// A settled choice whose one run has exactly one thing broken in it.
fn settled_run(break_it: impl FnOnce(&mut Value)) -> Vec<Problem> {
    let mut run = run_value(observations());
    break_it(&mut run);
    settled(json!([run])).problems()
}

static NEXT: AtomicUsize = AtomicUsize::new(0);

/// A throwaway tree holding one ledger, one contract document, one raw artifact
/// and at most one results dossier, removed when it drops.
struct Fixture {
    root: PathBuf,
    contracts: Contracts,
}

impl Fixture {
    fn new(ledger: Value, contracts: Vec<Value>, results: Option<Value>) -> Fixture {
        Fixture::new_with_record(RECORD, ledger, contracts, results)
    }

    /// A fixture whose accepted record is the given text, so a rule about what
    /// the record states can be exercised on a record that states it.
    fn new_with_record(
        record: &str,
        ledger: Value,
        contracts: Vec<Value>,
        results: Option<Value>,
    ) -> Fixture {
        let root = std::env::temp_dir().join(format!(
            "symbiote-spikes-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::SeqCst)
        ));
        for directory in [
            "docs/proofs/results",
            "docs/proofs/evidence/desktop-shell",
            "docs/architecture",
        ] {
            std::fs::create_dir_all(root.join(directory)).expect("a fixture tree");
        }
        let root = root.canonicalize().expect("a canonical fixture root");
        std::fs::write(root.join(ADR), record).expect("the fixture record");
        std::fs::write(root.join(RAW), "the run's own log\n").expect("the fixture raw artifact");
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
            for run in results["runs"].as_array_mut().into_iter().flatten() {
                for cited in run["artifacts"].as_array_mut().into_iter().flatten() {
                    if cited["sha256"] == json!("") {
                        let artifact = cited["artifact"].as_str().expect("a cited artifact");
                        cited["sha256"] = json!(
                            hash(&fixture.root, artifact).expect("the cited raw artifact hashes")
                        );
                    }
                }
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
