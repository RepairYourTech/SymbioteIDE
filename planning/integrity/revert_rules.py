#!/usr/bin/env python3
"""Revert each refusal rule of the architecture governance crate alone, in a
throwaway copy of this working tree, and watch the case that holds it fail.

The table below holds one row for each refusal this repository has proved by
reversion: the text that holds the rule, the text that removes it, and the case
that must fail. What it does **not** claim is completeness — nothing here relates
the rows to the crate's refusal sites, so a rule added without a row runs unproved
and the driver still reports every row it has as biting. The rows are also the
only place a rule's proof is recorded: `test_revert_rules.py` holds each row's
anchor to the tree and each row's case to a case the suite runs, so a moved anchor
or a renamed case fails that test rather than becoming a row this driver skips.

The live tree is never touched. The driver copies this working tree (everything
cargo needs, without `.git/`, `target/` or caches) into a temporary directory,
mutates the copy, runs each case there with the copy's own target directory, and
removes it — then re-reads the live tree's files and fails if any byte of them
moved. That is why it can be run beside another build: the version before this one
mutated the live tree in place, and a concurrent `cargo test` reported failures
that were only its mutations.
"""
from __future__ import annotations

import hashlib
import pathlib
import shutil
import subprocess
import sys
import tempfile

ROOT = pathlib.Path(__file__).resolve().parents[2]
SOURCES = ("crates/symbiote-architecture/src", "crates/symbiote-architecture/tests")
# The subjects a row's anchor may live in: the contract document the crate's own
# citations are read from, and the proof record its document target compares the
# bar against. Both are read by a case, so a rule about either is proved by moving
# the text one of them holds.
DOCUMENTS = ("docs/contracts/architecture.md", "docs/proofs/linux-shell.md")
IGNORED = shutil.ignore_patterns(".git", "target", ".freebuff", "__pycache__", "node_modules")

# Each row: the rule, the text that holds it, the text that removes it, and the
# case that fails when it is removed.
RULES: list[tuple[str, str, str, str]] = [
    (
        'a contract that answers no clause of the bar is refused',
        """        require(
            !self.answers.is_empty(),""",
        """        require(
            true,""",
        'a_contract_that_answers_no_clause_of_the_bar_is_refused',
    ),
    (
        'a clause cannot be answered by an obligation the contract does not declare',
        """                require(
                    declared.contains(&obligation),""",
        """                require(
                    true,""",
        'the_bar_cannot_be_shrunk_by_dropping_an_obligation_from_the_contract',
    ),
    (
        'every obligation the contract declares answers a clause',
        """        require(
            declared
                .iter()
                .all(|obligation| answered.contains(obligation.as_str())),""",
        """        require(
            true,""",
        'an_obligation_that_answers_no_clause_of_the_accepted_bar_is_refused',
    ),
    (
        'a clause is answered by exactly one carrier',
        '                carried == 1,',
        '                carried >= 1,',
        'an_answer_that_names_two_carriers_or_none_is_refused',
    ),
    (
        'an elsewhere answer says why this contract does not carry the clause',
        '                    text(&elsewhere.why),',
        '                    true,',
        'an_elsewhere_answer_with_no_reason_is_refused',
    ),
    (
        'a clause cannot be answered by an issue the choice does not own',
        '            if !record.draft.issue_refs.contains(&elsewhere.issue) {',
        '            if false {',
        'an_answer_naming_an_issue_the_choice_does_not_own_is_refused',
    ),
    (
        'every clause of the accepted bar is answered',
        """    for clause in &clauses {
        if !claimed.contains(clause.as_str()) {""",
        """    for clause in &clauses {
        if false && !claimed.contains(clause.as_str()) {""",
        'a_clause_of_the_accepted_bar_that_nothing_answers_is_refused',
    ),
    (
        'a bar named from a section the record states',
        """    let Some(text) = section(&accepted, heading) else {
        problems.push(Problem::new(
            subject,
            format!(
                "its choice names the {heading:?} section of {}, and that record states no such section",
                record.record
            ),
        ));
        return;
    };""",
        '    let text = section(&accepted, heading).unwrap_or_default();',
        'a_bar_named_from_a_section_the_record_does_not_state_is_refused',
    ),
    (
        'a choice that names a proof contract and no bar section is refused',
        """        (Some(_), None) => problems.push(Problem::new(
            format!("{id}: proof section"),
            "it names a proof contract and no section of its accepted text that says what that proof must do, so the contract would state its own bar",
        )),""",
        '        (Some(_), None) => {}',
        'a_choice_that_names_a_proof_and_no_bar_section_is_refused',
    ),
    (
        "the pre-#602 refusal is the derive's own message for the stale field",
        '    assert!(message.contains("unknown field `obligations`"), "{message}");',
        '    assert!(message.contains("unknown field `answers`"), "{message}");',
        'the_shape_before_the_bar_moved_is_the_derive_refusing_the_stale_field',
    ),
    (
        'a contract document of the shape before the bar moved is refused naming what moved',
        """    let before_the_bar_moved = contracts
        .iter()
        .any(|contract| contract.get("obligations").is_some());""",
        """    let before_the_bar_moved = false
        && contracts
            .iter()
            .any(|contract| contract.get("obligations").is_some());""",
        'a_contract_document_of_the_shape_before_the_bar_moved_is_refused_naming_what_moved',
    ),
    (
        'a choice that names a bar section and no contract is refused',
        """        (None, Some(section)) => problems.push(Problem::new(
            format!("{id}: proof section"),
            format!(
                "it names the {section:?} section of its accepted text as the bar its proof must answer, and no contract answers it"
            ),
        )),""",
        '        (None, Some(_)) => {}',
        'a_choice_that_names_a_bar_section_and_no_contract_is_refused',
    ),
    (
        'a contract settles a decision the ledger does not hold',
        """            problems.push(Problem::new(
                subject,
                format!(
                    "it settles {}, which this ledger does not hold, so nothing owns the choice it is written for",
                    contract.decision
                ),
            ));
            continue;""",
        '            continue;',
        'a_contract_that_settles_a_decision_the_ledger_does_not_hold_is_refused',
    ),
    (
        'a contract and its decision name each other',
        """        if record.proof_contract.as_deref() != Some(contract.id.as_str()) {
            problems.push(Problem::new(
                subject.clone(),
                format!(
                    "{} does not name this contract as its proof, so nothing would settle it by this run",
                    record.draft.id
                ),
            ));
        }
""",
        '',
        'a_choice_whose_contract_does_not_name_it_back_is_refused',
    ),
    (
        'a choice may not name a contract the repository does not hold',
        """        if !contracts.holds(id) {
            problems.push(Problem::new(
                format!("{}: proof contract", record.draft.id),
                format!("it is settled by {id}, which this repository does not hold"),
            ));
        }
""",
        '',
        'a_choice_naming_a_contract_the_repository_does_not_hold_is_refused',
    ),
    (
        'every obligation names a clause the decision owns',
        """            _ => problems.push(Problem::new(
                subject,
                format!(
                    "the {kind} {entry:?} names no clause of {}, so nothing owns it (it belongs to one of {})",
                    record.draft.id,
                    owned.iter().cloned().collect::<Vec<_>>().join(", ")
                ),
            )),""",
        '            _ => {}',
        'an_obligation_naming_no_owned_clause_is_refused',
    ),
    (
        'every issue the decision names is addressed',
        """    for issue in &owned {
        if !cited.contains(issue.as_str()) {
            problems.push(Problem::new(
                subject,
                format!(
                    "the contract addresses no obligation to {issue}, which {} names as its own",
                    record.draft.id
                ),
            ));
        }
    }
""",
        '',
        'an_issue_the_choice_names_that_no_obligation_addresses_is_refused',
    ),
    (
        'a decided choice needs the run it points at',
        """            if decided {
                problems.push(Problem::new(
                    subject,
                    format!(
                        "the choice is {} and no run settles it: {error}",""",
        """            if false {
                problems.push(Problem::new(
                    subject,
                    format!(
                        "the choice is {} and no run settles it: {error}",""",
        'a_decided_choice_with_no_run_is_refused',
    ),
    (
        'a decided choice cites its own run as evidence',
        """    if decided
        && !record
            .draft
            .evidence
            .iter()
            .any(|evidence| evidence.artifact == contract.result_artifact)
    {
        problems.push(Problem::new(
            subject,
            format!(
                "the record settles the choice on {} and does not cite it as its own evidence",
                contract.result_artifact
            ),
        ));
    }
""",
        '',
        'a_decided_choice_that_does_not_cite_its_own_run_is_refused',
    ),
    (
        "an undecided choice is held only to the result's identity",
        '        results.identity_unmet(contract, &fingerprint)',
        '        results.unmet(contract, &fingerprint)',
        'a_partial_run_is_recorded_without_settling_the_choice',
    ),
    (
        'a raw artifact a settled run cites must be in the tree',
        """            None => problems.push(Problem::new(
                subject,
                format!(
                    "the run cites raw artifact {}, which is not in the tree",
                    cited.artifact
                ),
            )),""",
        '            None => {}',
        'a_settled_run_citing_a_raw_artifact_the_tree_does_not_hold_is_refused',
    ),
    (
        'a raw artifact a settled run cites must not have changed',
        '            Some(actual) if actual != cited.sha256 => problems.push(Problem::new(',
        '            Some(actual) if false => problems.push(Problem::new(',
        'a_settled_run_citing_a_raw_artifact_that_has_changed_is_refused',
    ),
    (
        'a result must record a run',
        """        if self.runs.is_empty() {
            unmet.push("the result records no run, so nothing has been measured".into());
        }
""",
        '',
        'a_result_that_records_no_run_does_not_settle_the_choice',
    ),
    (
        'a run names the platform it measured',
        """        if self.platform.trim().is_empty() {
            unmet.push("a run names no platform, so nothing says where it was measured".into());
        }
""",
        '',
        'a_run_that_names_no_platform_does_not_settle_the_choice',
    ),
    (
        'a run names the platform version',
        '        if !text(&self.version) {',
        '        if false {',
        'a_run_that_names_no_platform_version_does_not_settle_the_choice',
    ),
    (
        'a run names the hardware it measured on',
        '        if !text(&self.hardware) {',
        '        if false {',
        'a_run_that_names_no_hardware_does_not_settle_the_choice',
    ),
    (
        'a run names the revision it was built from',
        '        if !revision(&self.commit) {',
        '        if false {',
        'a_run_that_names_no_revision_does_not_settle_the_choice',
    ),
    (
        'a run may not attest an obligation the contract never declared',
        '        for entry in &exercised {',
        '        for entry in exercised.iter().take(0) {',
        'a_run_that_attests_an_obligation_the_contract_never_declared_is_refused',
    ),
    (
        'a run must attest every obligation the contract declares',
        '        for obligation in &contract.workload {',
        '        for obligation in contract.workload.iter().take(0) {',
        'a_run_that_exercises_part_of_the_workload_does_not_settle_the_choice',
    ),
    (
        'a run that settles must have published something',
        '        if self.artifacts.is_empty() {',
        '        if false {',
        'a_settlement_on_a_run_that_published_nothing_is_refused',
    ),
    (
        'a run may not call the platform it exercised untested',
        """            if !platform.is_empty()
                && self
                    .untested_platforms""",
        """            if false
                && self
                    .untested_platforms""",
        'a_run_that_lists_the_platform_it_exercised_as_untested_is_refused',
    ),
    (
        'a run names the failures it saw',
        '        for failure in &self.failures {',
        '        for failure in self.failures.iter().take(0) {',
        'a_run_that_records_a_failure_with_nothing_said_about_it_is_refused',
    ),
    (
        'a stopped run does not settle the choice',
        """                    Some(condition) => {
                        unmet.push(format!(
                            "{label} stopped on {condition:?}, so the contract does not settle the choice"
                        ));
""",
        """                    Some(condition) => {
""",
        'a_run_that_ends_on_a_stop_condition_does_not_settle_the_choice',
    ),
    (
        'a run stops on a condition the contract declares',
        """                        if !contract
                            .stop_conditions
                            .iter()
                            .any(|declared| declared == condition)
                        {""",
        '                        if false {',
        'a_run_that_stops_on_a_condition_the_contract_never_declared_is_refused',
    ),
    (
        'a stopped run records the failure it stopped on',
        '                        if !self.failures.iter().any(|failure| failure == condition) {',
        '                        if false {',
        'a_run_that_stops_without_recording_the_failure_it_stopped_on_is_refused',
    ),
    (
        'a run records each predeclared measurement once',
        """                Some(values) => unmet.push(format!(
                    "{label} records the predeclared measurement {} {} times",
                    measurement.name,
                    values.len()
                )),""",
        '                Some(values) => {}',
        'a_run_that_records_one_measurement_twice_does_not_settle_the_choice',
    ),
    (
        'a run records every predeclared measurement',
        """                None => unmet.push(format!(
                    "{label} records no observation for {}, which the contract predeclares",
                    measurement.name
                )),""",
        '                None => {}',
        'a_run_that_omits_a_predeclared_measurement_does_not_settle_the_choice',
    ),
    (
        'an observation above its ceiling does not settle',
        '                    } else if *value > measurement.maximum {',
        '                    } else if false {',
        'an_observation_above_the_predeclared_maximum_does_not_settle_the_choice',
    ),
    (
        'a run may not observe what the contract never declared',
        '        for name in observed.keys() {',
        '        for name in observed.keys().take(0) {',
        'a_run_that_observes_something_the_contract_does_not_predeclare_is_refused',
    ),
    (
        'a threshold may not move under its result',
        '        if self.contract_sha256 != fingerprint {',
        '        if false {',
        'a_run_measured_against_a_moved_threshold_does_not_settle_the_choice',
    ),
    (
        'an untested platform keeps the choice pending',
        '        if !self.untested_platforms.is_empty() {',
        '        if false {',
        'a_run_that_leaves_a_platform_untested_does_not_settle_the_choice',
    ),
    (
        'the results schema version is enforced',
        '            results.schema_version == RESULTS_SCHEMA_VERSION,',
        '            true || results.schema_version == RESULTS_SCHEMA_VERSION,',
        'a_result_artifact_of_an_unknown_schema_version_is_refused',
    ),
    (
        'the contract document schema version is enforced',
        '            contracts.schema_version == CONTRACTS_SCHEMA_VERSION,',
        '            true || contracts.schema_version == CONTRACTS_SCHEMA_VERSION,',
        'a_contract_document_of_an_unknown_schema_version_is_refused',
    ),
    (
        'a document that holds no contract is refused',
        '            !contracts.contracts.is_empty(),',
        '            true || !contracts.contracts.is_empty(),',
        'a_contract_document_with_an_unknown_field_or_no_contract_is_refused',
    ),
    (
        'two contracts may not share an identity',
        """        let mut ids = BTreeSet::new();
        for contract in &contracts.contracts {
            contract.validate()?;
            require(
                ids.insert(contract.id.clone()),
                format!("{}: two contracts share this identity", contract.id),
            )?;
        }
""",
        """        for contract in &contracts.contracts {
            contract.validate()?;
        }
""",
        'a_contract_document_that_names_one_identity_twice_is_refused',
    ),
    (
        "the document cites a case this crate's suite runs",
        '`the_committed_choice_names_the_section_its_bar_comes_from`',
        '`nonexistent_case_2`',
        'the_contract_document_names_only_cases_this_crate_holds',
    ),
    (
        "the document's citation is a case, not a helper",
        '`the_committed_choice_names_the_section_its_bar_comes_from`',
        '`pre_bar_move_contract`',
        'the_contract_document_names_only_cases_this_crate_holds',
    ),
    (
        'a listed non-case must still be cited by the document',
        '    "require_ready",',
        '    "phantom_field_here",',
        'the_contract_document_names_only_cases_this_crate_holds',
    ),
    (
        'a name the document cites as not a case must be listed',
        '`require_ready` refuses provisional',
        '`require_ready_moved` refuses provisional',
        'the_contract_document_names_only_cases_this_crate_holds',
    ),
    (
        "a listed non-case must be a name the crate's source carries",
        '    pub fn require_ready(&self, artifact: &str, now: u64) -> Result<()> {',
        '    pub fn require_ready_moved(&self, artifact: &str, now: u64) -> Result<()> {',
        'the_contract_document_names_only_cases_this_crate_holds',
    ),
    (
        'an answer carries a clause the accepted section states',
        """            0 => problems.push(Problem::new(
                subject,
                format!(
                    "it answers {carried:?}, which {heading:?} of {} does not state as a clause of the accepted bar",
                    record.record
                ),
            )),""",
        """            0 => {}
            #[allow(unreachable_patterns)]""",
        'an_answer_whose_words_the_section_does_not_state_is_refused',
    ),
    (
        "an answer's words name one clause of the accepted section",
        """            many => problems.push(Problem::new(
                subject,
                format!(
                    "the words {carried:?} are {many} clauses of {heading:?}, so they name no one clause of the accepted bar"
                ),
            )),""",
        """            many => {
                let _ = many;
            }""",
        'an_answer_whose_words_are_stated_twice_by_the_section_is_refused',
    ),
    (
        'a clause named by its number is the shape before the move',
        '    let before_a_clause_had_its_words = contracts.iter().any(|contract| {',
        """    let before_a_clause_had_its_words = false
        && contracts.iter().any(|contract| {""",
        'a_contract_document_that_names_a_clause_by_its_number_is_refused_naming_what_moved',
    ),
    (
        "the record's stated ceiling is the contract's own maximum",
        '0.353** of a 3.0 ceiling',
        '0.353** of a 3.1 ceiling',
        'the_proof_record_states_the_contracts_own_bar',
    ),
    (
        "a ceiling is stated for a measurement the committed run observed",
        '**`cold_start_to_first_frame_seconds` 0.353**',
        '**`cold_start_to_first_frame_second` 0.353**',
        'the_proof_record_states_the_contracts_own_bar',
    ),
    (
        'the stale field is a move this loader names beside the answers beside it',
        """    let before_the_bar_moved = contracts
        .iter()
        .any(|contract| contract.get("obligations").is_some());""",
        """    let before_the_bar_moved = contracts
        .iter()
        .any(|contract| {
            contract.get("obligations").is_some() && contract.get("answers").is_none()
        });""",
        'a_contract_document_holding_the_old_field_beside_its_answers_names_the_move',
    ),
    (
        'a document carrying both moves is told about both',
        """    if before_a_clause_had_its_words {
        moved.push(("before a clause was named by its own words", CLAUSE_MOVED));
    }""",
        """    if before_a_clause_had_its_words && false {
        moved.push(("before a clause was named by its own words", CLAUSE_MOVED));
    }""",
        'a_contract_document_carrying_both_old_shapes_at_once_names_both',
    ),
]


def scanned(tree: pathlib.Path) -> list[pathlib.Path]:
    """Every file a row can be held in, under the tree the driver mutates."""
    files = [tree / document for document in DOCUMENTS]
    for source in SOURCES:
        files.extend(sorted((tree / source).rglob("*.rs")))
    return files


def anchor_in(anchor: str, tree: pathlib.Path) -> pathlib.Path | None:
    """The one file holding the anchor, or None when it is moved or ambiguous."""
    found = [path for path in scanned(tree) if path.read_text().count(anchor) == 1]
    return found[0] if len(found) == 1 else None


def case_target(case: str, tree: pathlib.Path) -> tuple[str, str] | None:
    """The package and test target defining the case, or None when nothing does."""
    for path in sorted((tree / "crates").glob("*/tests/*.rs")):
        if f"fn {case}(" in path.read_text():
            return path.parent.parent.name, path.stem
    return None


def digest(files: list[pathlib.Path]) -> dict[str, str]:
    return {str(path.relative_to(ROOT)): hashlib.sha256(path.read_bytes()).hexdigest() for path in files}


def main() -> int:
    before = digest(scanned(ROOT))
    bit, missing, stale, silent = [], [], [], []
    scratch = tempfile.mkdtemp(prefix="revert-rules-")
    try:
        tree = pathlib.Path(scratch) / "tree"
        shutil.copytree(ROOT, tree, ignore=IGNORED)
        for rule, anchor, removal, case in RULES:
            target = case_target(case, tree)
            if target is None:
                missing.append((rule, case))
                print(f"TEST MISSING: {case}")
                continue
            path = anchor_in(anchor, tree)
            if path is None:
                stale.append((rule, "anchor moved or ambiguous"))
                print(f"ANCHOR MOVED OR AMBIGUOUS: {rule}")
                continue
            package, target_name = target
            original = path.read_bytes()
            path.write_text(path.read_text().replace(anchor, removal))
            run = subprocess.run(
                [
                    "cargo", "test", "--locked", "-q", "-p", package,
                    "--test", target_name, case, "--", "--exact",
                ],
                cwd=tree, capture_output=True, text=True,
            )
            path.write_bytes(original)
            output = run.stdout + run.stderr
            if "error[E" in output or "could not compile" in output:
                stale.append((rule, "no compile"))
                print(f"INVALID (no compile): {rule}")
            elif run.returncode != 0 and "FAILED" in output:
                bit.append(rule)
                print(f"BITES: {case}")
            else:
                silent.append((rule, case))
                print(f"SILENT: {rule} ({case})")
    finally:
        shutil.rmtree(scratch, ignore_errors=True)

    after = digest(scanned(ROOT))
    moved = [name for name in before if after.get(name) != before[name]]
    print(
        f"\n{len(bit)}/{len(RULES)} rules bit; {len(missing)} missing tests; "
        f"{len(stale)} stale anchors; {len(silent)} silent; "
        f"{len(moved)} files of the live tree moved ({', '.join(moved) or 'none'})"
    )
    return 1 if missing or stale or silent or moved else 0


if __name__ == "__main__":
    sys.exit(main())
