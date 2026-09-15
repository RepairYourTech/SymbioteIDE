//! The constitution's non-negotiable invariants, checked rather than described.
//!
//! Every test here either evaluates the whole ledger against the real
//! constitution and tree, or proves that one of its channels refuses something
//! it must refuse. A channel that cannot fail is not evidence, so each check has
//! a companion that hands it a weakened input.

use std::collections::BTreeSet;
use symbiote_constitution::*;

#[test]
fn the_coverage_map_names_every_invariant_and_owner() {
    let mut identifiers = BTreeSet::new();
    for invariant in INVARIANTS {
        assert!(
            identifiers.insert(invariant.id),
            "duplicate invariant id {}",
            invariant.id
        );
        assert!(
            !invariant.requirement.trim().is_empty() && !invariant.statement.trim().is_empty(),
            "{} has no requirement or statement",
            invariant.id
        );
        assert!(
            !invariant.document.is_empty()
                || !invariant.tests.is_empty()
                || !invariant.facts.is_empty(),
            "{} declares no evidence channel at all",
            invariant.id
        );
    }
    assert_eq!(
        identifiers.len(),
        INVARIANTS.len(),
        "two invariants share an id"
    );
}

/// The whole point: every invariant passes, and `evaluate` did not pass by
/// finding nothing to check.
#[test]
fn the_constitution_conforms_and_every_binding_resolves() {
    let report = evaluate(&workspace_root());
    assert_eq!(report.invariants.len(), INVARIANTS.len());
    assert!(report.checked > 0, "the report checked nothing");
    for coverage in &report.invariants {
        let failed: Vec<&Outcome> = coverage.outcomes.iter().filter(|o| !o.ok).collect();
        assert!(
            coverage.is_proven(),
            "{} is not proven: {failed:?}",
            coverage.id
        );
        let invariant = INVARIANTS.iter().find(|i| i.id == coverage.id).unwrap();
        let expected = invariant.document.len()
            + invariant.forbidden.len()
            + invariant.tests.len()
            + invariant.facts.len();
        assert_eq!(
            coverage.outcomes.len(),
            expected,
            "{} dropped one of its channels",
            coverage.id
        );
    }
    assert!(
        report.failures.is_empty(),
        "the constitution has failures:\n{}",
        report.failures.join("\n")
    );
}

#[test]
fn the_repository_keeps_the_locked_technology_choices() {
    let root = workspace_root();
    for fact in [
        Fact::NoElectron,
        Fact::NoGoCore,
        Fact::ForbidUnsafeRust,
        Fact::StrictTypeScript,
    ] {
        let outcome = repository::fact_outcome(fact, &root);
        assert!(outcome.ok, "{}: {}", outcome.subject, outcome.detail);
    }
}

/// The document channel is not a formality: removing one normative sentence
/// fails the invariant that sentence carries.
#[test]
fn a_weakened_constitution_is_refused() {
    let weakened = CONSTITUTION.replace("The process is code; expertise is pluggable.", "");
    assert_ne!(weakened, CONSTITUTION, "the clause was not there to remove");
    let process = INVARIANTS.iter().find(|i| i.id == "CN-06").unwrap();
    assert!(
        document::outcomes(process, &weakened).iter().any(|o| !o.ok),
        "removing the clause did not fail CN-06"
    );
    assert!(
        document::outcomes(process, CONSTITUTION)
            .iter()
            .all(|o| o.ok)
    );
}

/// A permission written back into prose is an authorization the build can see,
/// which is how the non-goals stop being only an absence of lines.
#[test]
fn a_forbidden_authorization_is_refused() {
    let permissive = format!("{CONSTITUTION}\nElectron is eligible for the desktop.\n");
    let non_goals = INVARIANTS.iter().find(|i| i.id == "CN-10").unwrap();
    assert!(
        document::outcomes(non_goals, &permissive)
            .iter()
            .any(|o| !o.ok),
        "permitting Electron did not fail the non-goals"
    );
    assert!(
        document::outcomes(non_goals, CONSTITUTION)
            .iter()
            .all(|o| o.ok)
    );
}

/// A ledger entry that points at a check the harness does not run is drift, not
/// evidence. The fixture supplies every way that happens, and the load-bearing
/// regression is the first case: a plain `#[test]` in a file cargo never
/// compiles. Before the harness was consulted it passed, because nothing but
/// `#[ignore]` stood between a test-shaped file and an invariant.
#[test]
fn a_binding_the_harness_does_not_run_is_refused() {
    let root = workspace_root();
    let harness = Harness::discover(&root).expect("the workspace harness");
    let fixture = "crates/symbiote-constitution/tests/fixtures/not_evidence.rs";
    for binding in [
        format!("{fixture}::a_test_the_harness_never_compiles"),
        format!("{fixture}::a_skipped_check"),
        format!("{fixture}::a_helper_mistaken_for_a_test"),
        "crates/symbiote-domain/tests/contracts.rs::no_such_test".to_string(),
        "crates/symbiote-constitution/src/nowhere.rs::a_test".to_string(),
        "planning/integrity/validate.py::main".to_string(),
        "not_a_binding".to_string(),
    ] {
        assert!(
            !harness.outcome(&binding).ok,
            "{binding} should have been refused"
        );
    }
    // A test target of a member, a unit test reached through a `mod`, and a
    // test the roadmap-integrity job discovers are all real checks.
    for binding in [
        "crates/symbiote-constitution/tests/conformance.rs::the_coverage_map_names_every_invariant_and_owner",
        "crates/symbiote-domain/src/lease.rs::lease_expiry_bounds_are_enforced",
        "planning/integrity/test_closing_keywords.py::test_a_landed_title_citing_an_issue_number_passes",
    ] {
        assert!(
            harness.outcome(binding).ok,
            "{binding} should have been accepted"
        );
    }
}

/// #170 routes the goal/delegation, learning and roadmap work to named owners.
/// The check this layer can make is that the record routes to them; whether
/// their integrations exist is theirs to prove, not this record's to claim.
#[test]
fn the_routed_owners_are_named_by_the_record() {
    const ROUTED: [u64; 6] = [464, 447, 460, 449, 454, 470];
    let routed: BTreeSet<u64> = INVARIANTS
        .iter()
        .flat_map(|invariant| invariant.owners.iter().copied())
        .collect();
    for issue in ROUTED {
        assert!(
            routed.contains(&issue),
            "#{issue} is named by the criterion but routed by no invariant"
        );
    }
    let owners = INVARIANTS
        .iter()
        .find(|invariant| invariant.id == "CN-23")
        .expect("the routing entry");
    assert_eq!(owners.owners, ROUTED.to_vec());
}

/// Every invariant is machine-checked by something that runs, and an invariant
/// with no executable check has to say why rather than simply having none. The
/// two directions are checked together, so an explanation cannot become a crutch
/// for an entry that does have a check, and a check cannot quietly disappear
/// behind a silence.
#[test]
fn an_invariant_with_no_executable_check_states_why() {
    let executable: BTreeSet<&str> = INVARIANTS
        .iter()
        .filter(|invariant| !invariant.tests.is_empty() || !invariant.facts.is_empty())
        .map(|invariant| invariant.id)
        .collect();
    let explained: BTreeSet<&str> = EXPLANATIONS.iter().map(|(id, _)| *id).collect();
    for invariant in INVARIANTS {
        assert!(
            !invariant.document.is_empty()
                || !invariant.forbidden.is_empty()
                || executable.contains(invariant.id),
            "{} is checked by nothing at all",
            invariant.id
        );
        if executable.contains(invariant.id) {
            assert!(
                !explained.contains(invariant.id),
                "{} has an executable check and an explanation for having none",
                invariant.id
            );
        } else {
            assert!(
                explained.contains(invariant.id),
                "{} has no executable check and no stated reason",
                invariant.id
            );
        }
    }
    for (id, why) in EXPLANATIONS {
        assert!(
            INVARIANTS.iter().any(|invariant| invariant.id == *id),
            "EXPLANATIONS names an unknown invariant: {id}"
        );
        assert!(!why.trim().is_empty(), "{id} explains nothing");
    }
}

/// The checks a coverage row may cite, as this tree answers.
fn checks() -> repository::Checks {
    repository::Checks::read(&workspace_root()).expect("the checks this tree provides")
}

/// The coverage map is the record's own account of what it covers, so a row
/// that names neither the check that runs it nor an issue that owns it is an
/// unaccounted claim — the exact defect the map exists to prevent. The map must
/// be there, and every row must account for itself, against checks this
/// repository actually provides.
#[test]
fn every_coverage_row_names_a_check_or_an_owner() {
    let checks = checks();
    let rows = document::coverage_rows(CONSTITUTION).expect("the coverage map");
    assert!(!rows.is_empty(), "the coverage map has no rows");
    for row in &rows {
        assert!(
            document::accounts_for(row.accounting, |name| checks.knows(name)),
            "the coverage row {:?} names neither a check this repository runs nor an owner: {:?}",
            row.item,
            row.accounting
        );
    }
}

/// The resolution is the point: a row may only cite a check this repository
/// actually runs. A name shaped like a check is not a check, and naming one real
/// job beside an invented one is still not accounting for the row.
#[test]
fn a_row_citing_a_check_this_repository_does_not_run_is_refused() {
    let checks = checks();
    // The names the map does cite resolve, so this is not refusing everything.
    for known in [
        "closing-keywords",
        "CN-22",
        "crates/symbiote-constitution/tests/conformance.rs",
        "every_coverage_row_names_a_check_or_an_owner",
    ] {
        assert!(checks.knows(known), "{known} should resolve");
    }
    for invented in [
        "contracts-typo",
        "not-a-check-this-repository-runs",
        "review_it",
        "CN-99",
    ] {
        assert!(!checks.knows(invented), "{invented} should not resolve");
        assert!(
            !document::accounts_for(&format!("`{invented}`"), |name| checks.knows(name)),
            "{invented} should not have accounted for a row"
        );
    }
    assert!(
        !document::accounts_for(
            "`closing-keywords`, `not-a-check-this-repository-runs`",
            |name| checks.knows(name)
        ),
        "one real check beside an invented one is not accounting for the row"
    );
    assert!(document::accounts_for(
        "`closing-keywords`, `contracts`",
        |name| checks.knows(name)
    ));
    // The row as the map really spells it, doctored to cite a job that does not run.
    let doctored = CONSTITUTION.replace("`closing-keywords`", "`closing-keywords-typo`");
    assert_ne!(doctored, CONSTITUTION, "the job was not there to rename");
    let rows = document::coverage_rows(&doctored).expect("the coverage map");
    assert!(
        rows.iter()
            .any(|row| !document::accounts_for(row.accounting, |name| checks.knows(name))),
        "a row citing a job this repository does not run must be refused"
    );
}

/// The rule above is evidence only if it refuses something, so the ways a row
/// fails to account for itself are handed to it: prose, an empty cell, prose in
/// backticks, unpaired backticks, and a reference to the issue this record is
/// accounting for.
#[test]
fn a_coverage_row_that_names_neither_is_refused() {
    let checks = checks();
    for unaccounted in [
        "separate reviewer evidence recorded against #170",
        "it is reviewed",
        "",
        "`closing-keywords` and `unpaired",
    ] {
        assert!(
            !document::accounts_for(unaccounted, |name| checks.knows(name)),
            "{unaccounted:?} should not have accounted for a row"
        );
    }
    for accounted in ["`closing-keywords`", "#387 and #394", "\n`CN-22`; #38\n"] {
        assert!(
            document::accounts_for(accounted, |name| checks.knows(name)),
            "{accounted:?} should have accounted for a row"
        );
    }
    // A map that is missing is not a map whose rows all pass.
    assert!(document::coverage_rows("# no coverage map here\n").is_none());
}

/// The evidence recorded against the issue is a generated artifact, so it is
/// guarded the way this repository guards its other generated artifacts: the
/// committed encoding must be exactly what the tree emits.
#[test]
fn the_committed_report_is_what_the_tree_emits() {
    let root = workspace_root();
    let committed = std::fs::read_to_string(root.join(REPORT_PATH))
        .unwrap_or_else(|error| panic!("the committed report at {REPORT_PATH}: {error}"));
    let current = report_json(&root);
    if committed != current {
        let drift = committed
            .lines()
            .zip(current.lines())
            .enumerate()
            .find(|(_, (committed, current))| committed != current)
            .map(|(line, (committed, current))| {
                format!(
                    "\n  line {}:\n  committed: {committed}\n  current:   {current}",
                    line + 1
                )
            })
            .unwrap_or_default();
        panic!(
            "the committed report drifted from the tree{drift}\n  regenerate with `cargo run -p symbiote-constitution --example constitution_report -- --write`"
        );
    }
    // A record that records nothing is not a record.
    assert!(
        current.len() > 1024,
        "the report is too small to be the ledger"
    );
}
