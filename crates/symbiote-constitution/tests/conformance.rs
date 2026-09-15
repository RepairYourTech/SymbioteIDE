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
        let outcome = fact_outcome(fact, &root);
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
        document_outcomes(process, &weakened).iter().any(|o| !o.ok),
        "removing the clause did not fail CN-06"
    );
    assert!(
        document_outcomes(process, CONSTITUTION)
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
        document_outcomes(non_goals, &permissive)
            .iter()
            .any(|o| !o.ok),
        "permitting Electron did not fail the non-goals"
    );
    assert!(
        document_outcomes(non_goals, CONSTITUTION)
            .iter()
            .all(|o| o.ok)
    );
}

/// A ledger entry that no longer points at a real, running test is drift, not
/// evidence. The fixture supplies the two ways that happens.
#[test]
fn a_missing_skipped_or_non_test_binding_is_refused() {
    let root = workspace_root();
    let fixture = "crates/symbiote-constitution/tests/fixtures/not_evidence.rs";
    for binding in [
        format!("{fixture}::a_skipped_check"),
        format!("{fixture}::a_helper_mistaken_for_a_test"),
        "crates/symbiote-domain/tests/contracts.rs::no_such_test".to_string(),
        "crates/symbiote-constitution/src/nowhere.rs::a_test".to_string(),
        "not_a_binding".to_string(),
    ] {
        assert!(
            !binding_outcome(&root, &binding).ok,
            "{binding} should have been refused"
        );
    }
    assert!(
        binding_outcome(
            &root,
            "crates/symbiote-domain/tests/contracts.rs::compilation_is_deterministic_and_role_survives_restaffing"
        )
        .ok
    );
}

/// Every invariant's requirement is accounted for by something that runs: a
/// document clause checked by this suite, a repository fact, or a named test.
#[test]
fn every_invariant_is_machine_checked() {
    let report = evaluate(&workspace_root());
    for coverage in &report.invariants {
        assert!(
            !coverage.outcomes.is_empty(),
            "{} is checked by nothing",
            coverage.id
        );
    }
    let behavioural: usize = INVARIANTS
        .iter()
        .filter(|invariant| !invariant.tests.is_empty() || !invariant.facts.is_empty())
        .count();
    assert!(
        behavioural * 2 >= INVARIANTS.len(),
        "fewer than half the invariants have an executable check: {behavioural}"
    );
}
