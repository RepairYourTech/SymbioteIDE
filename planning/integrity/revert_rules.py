#!/usr/bin/env python3
"""Revert each refusal rule of the architecture governance crate alone, in a
throwaway copy of this working tree, and watch the case that holds it fail.

`RULES` holds one row for each refusal this repository has proved by reversion:
the text that holds the rule, the text that removes it, and the case that must
fail. What it does **not** claim is completeness — nothing here relates the rows
to the crate's refusal sites, so a rule added without a row runs unproved and the
driver still reports every row it has as biting. The rows are also the only place
a rule's proof is recorded: `test_revert_rules.py` holds each row's anchor to the
tree and each row's case to a case the suite runs, so a moved anchor or a renamed
case fails there rather than becoming a row this driver skips.

`HOLDS` holds the rules a reversion cannot prove because the rule *is* the refusal
being reverted, or the link the refusal is about: removing the text removes the
refusal, so a mutation would watch nothing fail. Each row names the refusal, the
case that states it, the subject stating it, the text it states exactly once, and
every way the driver shows it biting — the subject a line past the number it
declares, the case run against a tip whose guard was smaller, a link taken out of
the file it is written in, a text in that file replaced by a weaker one, or a
workflow written into one of the states `chain.py` names, which reaches the
workflow through the reading the case itself makes and so freezes no spelling the
case accepts. What a row cannot see, how the checker proves itself every run, and
how the declarations and the rows together account for every case in the classes
the rows are proved in are stated where they are done: in `holds` and `shown`, in
`self_proof` and `NEGATIVE`, and in `unheld_cases`.

The live tree is never touched: the driver copies this working tree into a
temporary directory, mutates the copy, runs each case there, removes it and fails
if any byte of the live tree moved, so it can be run beside another build.
"""
from __future__ import annotations

import ast
import hashlib
import json
import os
import pathlib
import re
import shutil
import subprocess
import sys
import tempfile

import chain

ROOT = pathlib.Path(__file__).resolve().parents[2]
SOURCES = ("crates/symbiote-architecture/src", "crates/symbiote-architecture/tests")
# The subjects a row's anchor may live in: the contract document the crate's own
# citations are read from, the proof record its document target compares the bar
# against, and the committed contract data a case reads its platforms, thresholds
# and clauses out of. Each is read by a case, so a rule about any of them is
# proved by moving the text it holds — and a rule comparing two of those facts
# needs the document moved rather than the comparison reverted, since a
# comparison two honestly agreeing sides satisfy stays green when it is removed.
DOCUMENTS = (
    "docs/contracts/architecture.md",
    "docs/proofs/linux-shell.md",
    "docs/architecture/spike-contracts.json",
)
IGNORED = shutil.ignore_patterns("target", ".freebuff", "__pycache__", "node_modules")

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
        """    assert!(
        message.starts_with("unknown field `obligations`, expected one of `"),""",
        """    assert!(
        message.starts_with("unknown field `answers`, expected one of `"),""",
        'the_shape_before_the_bar_moved_is_the_derive_refusing_the_stale_field',
    ),
    (
        'the message names the section\u2019s replacement only inside the list of fields',
        '    assert!(message.contains("`answers` at line "), "{message}");',
        '    assert!(message.contains("`obligations` at line "), "{message}");',
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
    (
        'a platform family is a word the reading names rather than nothing',
        '.then(|| word.to_lowercase())',
        '.then(String::new)',
        'the_committed_contract_names_the_platforms_its_own_clause_names_and_its_prose_defers_to_them',
    ),
    (
        'a contract applies to no platform the clause that sets applicability does not name',
        '"macOS 14 arm64"',
        '"FreeBSD 14 amd64"',
        'the_committed_contract_names_the_platforms_its_own_clause_names_and_its_prose_defers_to_them',
    ),
    (
        "a clause that opens on a word naming nothing is not read as naming a platform",
        '        .skip(usize::from(opens_a_sentence))',
        '        .skip(0)',
        'the_committed_contract_names_the_platforms_its_own_clause_names_and_its_prose_defers_to_them',
    ),
    (
        'the clause that sets applicability is the one naming a platform the contract applies to',
        '        .find(|answer| !named_platforms(&answer.clause, true).is_disjoint(&declared))',
        '        .find(|_| true)',
        'the_committed_contract_names_the_platforms_its_own_clause_names_and_its_prose_defers_to_them',
    ),
]


# The tree text a hold names and weakens: the comparison of the live tree before and after the run,
# which must stay able to see a file move. No row anchors the workflow: which step runs these checks
# is owned by `chain.py`, read from the commands a step runs, and the rows that hold the chain mutate
# a workflow *through* that reading (`mutates` below) — a row freezing the workflow's own text would
# refuse a maintainer who re-spells the step the way the case reads it.
MOVED = ("    after = digest(watched)\n"
         "    moved = [name for name in before if after.get(name) != before[name]]\n")
# The line the case accounting derives the files it walks from, as text: a row replaces it with the
# hold's own file alone, so an edit that narrows that walk is a walk the case below stops stating
# rather than a silent narrowing — and a spelling of the line this constant does not hold is a row
# that no longer places its refusal.
SUBJECTS = "    for where in sorted({row[2] for row in HOLDS if len(row) == 5}):\n"
CHAIN = "planning/integrity/test_python_floor.py"
CONTRACTS = "planning/integrity/test_contracts.py"
WORKFLOW = ".github/workflows/roadmap-integrity.yml"

# The ways a hold may show its refusal, named once here so the rows, the inputs the checker is
# proved on and the rule holding both cannot drift apart: `larger` adds lines to `file` until it is
# past the number it declares after `argument`, `earlier` runs the case against the smallest
# revision of `file` this checkout holds, `gone` takes `argument` out of `file`, `weaker` replaces
# `argument` — a pair of texts — with the second, and `mutates` writes a workflow into the state
# `argument` names through `chain.mutated`, which is how a link gone or a step made non-fatal is
# driven without either way matching the workflow's own text.
WAYS: tuple[str, ...] = ("larger", "earlier", "gone", "weaker", "mutates")

# Rules held by a case's presence and its refusals rather than by a reversion: the case cannot
# hold its own presence — renaming it out of collection, deleting the class it sits in or emptying
# the comparison it makes leaves the suite green and this driver reporting every row it has as
# biting — so each row names the refusal, the case stating it, the subject stating it, the text the
# subject states exactly once, and the ways that must make the case fail. A way is `(file, how,
# argument)`, and `how` is one of `WAYS` above. A row with no way, and this table with no row, are
# refused rather than passed.
HOLDS: list[tuple[str, str, str, str, tuple[tuple[str, str, str], ...]]] = [
    (
        "a contract entry under docs/contracts/ that no census entry classifies is refused",
        "test_every_entry_under_the_contracts_directory_is_classified_once",
        "planning/integrity/test_contracts.py",
        "sits under docs/contracts/ and no census entry classifies it",
        # Two directions the census owner must refuse: an entry taken out of the classification —
        # the state a document added to the directory is in — and a reading narrowed to entries
        # that do not exist, which leaves every real document classified by nothing. A third, the
        # figures a document states moving without the record, is the count case's own and is left
        # to it: this row holds the refusal it is stated against.
        (("planning/integrity/contracts.py", "gone",
          '    "domain-ontology.v1.schema.json": "generated by the ontology schema example and '
          'compared with `--check`",\n'),
         ("planning/integrity/contracts.py", "weaker",
          ("    return sorted(one.name for one in root.iterdir())\n",
           "    return sorted(one.name for one in root.iterdir() if one.name.startswith('z'))\n")),
         ("planning/integrity/contracts.py", "gone",
          '    "worktrees.md": ("symbiote-worktrees", '
          '"the_contract_states_the_identities_this_module_derives"),\n')),
    ),
    (
        "the reader-size guard's cap refuses a guard larger than the tip it lands on",
        "test_the_guard_is_within_the_cap_it_may_not_raise",
        "planning/integrity/test_reader_size.py",
        "        self.assertLessEqual(GUARD_CAP, parent,",
        (("planning/integrity/test_reader_size.py", "larger", "GUARD_CAP = "),
         ("planning/integrity/test_reader_size.py", "earlier", "")),
    ),
    (
        "the job that runs this directory's checks also runs the rule driver",
        "test_the_job_that_runs_these_checks_also_runs_the_driver_over_full_history",
        CHAIN,
        "these jobs run the suite but not the rule driver",
        ((WORKFLOW, "mutates", chain.NO_DRIVER),),
    ),
    (
        "the hold's own checker refuses a way that does not bite",
        "test_the_checker_and_its_own_proof_refuse_a_way_that_does_not_bite",
        "planning/integrity/test_revert_rules.py",
        "a way that does not bite reads as held",
        (("planning/integrity/revert_rules.py", "gone",
          '    return None if why is None else f"{why} {said}"\n'),),
    ),
    (
        "the job that runs those checks fetches the history they read",
        "test_the_job_that_runs_these_checks_also_runs_the_driver_over_full_history",
        CHAIN,
        "these jobs run those checks without the history they read",
        ((WORKFLOW, "mutates", chain.NO_HISTORY),),
    ),
    (
        "the job that runs this directory's checks runs every suite this repository's checks "
        "consist of",
        "test_the_job_that_runs_these_checks_also_runs_the_driver_over_full_history",
        CHAIN,
        "these jobs run this directory's suite without",
        # Each suite's own command taken out of its step, located by the reading the case itself
        # makes rather than matched against the text: a job that runs this directory's suite and not
        # the registry's, the matrices' or the policy's leaves their refusals run by nothing, which
        # is the state a step deleted or moved to another job is in. And the case's own expectation
        # table is held in turn: a pair taken out of it is a suite, or a state, the reading is never
        # asked about, so the two `gone` ways must make the case fail on accounts it would otherwise
        # have stopped stating.
        ((WORKFLOW, "mutates", chain.NO_SUITE),
         (CHAIN, "gone", "(chain.NO_SUITE, POLICY_LINK),\n"),
         (CHAIN, "gone", "(chain.NO_DRIVER, DRIVER_LINK), ")),
    ),
    (
        "the job that runs these checks fails when they fail",
        "test_the_job_that_runs_these_checks_also_runs_the_driver_over_full_history",
        CHAIN,
        "these steps run the checks without their failure reaching the job",
        ((WORKFLOW, "mutates", chain.NON_FATAL),
         (WORKFLOW, "mutates", chain.CONDITIONAL),
         (WORKFLOW, "mutates", chain.JOB_CONDITIONAL),
         (WORKFLOW, "mutates", chain.SWALLOWED)),
    ),
    (
        "the states the mutation writes land where a YAML parser reads a key",
        "test_each_state_is_written_inside_the_definition_it_means",
        CHAIN,
        "                self.assertGreater(column, dash,",
        (("planning/integrity/chain.py", "weaker",
          ('            return found.start("key")', '            return found.start("indent")')),),
    ),
    (
        "the driver refuses a live file moved while it runs",
        "test_a_live_file_moved_while_the_driver_runs_is_refused",
        "planning/integrity/test_revert_rules.py",
        "the driver did not refuse a file moved while it ran",
        (("planning/integrity/revert_rules.py", "weaker",
          (MOVED, "    after = digest(watched)\n    moved = []\n")),),
    ),
    (
        "the case accounting walks every file a row states its refusal in",
        "test_a_case_no_row_names_and_no_declaration_accounts_for_is_refused",
        "planning/integrity/test_revert_rules.py",
        "no declaration accounts for",
        # The walk's own subject list, replaced by the hold's file alone: the case drives every
        # file the rows state their refusals in, so a walk narrowed to one of them fails there.
        (("planning/integrity/revert_rules.py", "weaker",
          (SUBJECTS, "    for where in [HOLDER]:\n")),),
    ),
    (
        "a step that runs one suite is that suite's step and no other's",
        "test_a_step_is_read_as_the_suite_it_names_and_as_no_other",
        CHAIN,
        '        """Which suite a step runs discovery in, driven from the rows themselves:',
        # Three of the directions the case above drives, each one edit to `names_directory`: it
        # normalizes before it compares, which is what reads a `./` path and a trailing slash; it
        # compares a whole path rather than a prefix; and it compares the whole path rather than the
        # last name, which is what refuses a directory of the same name elsewhere. A reader that
        # stopped normalizing leaves a spelling the case drives unread, one widened to a prefix reads
        # a directory that only starts with this one, and one widened to the last name reads a
        # directory the row it is asked about does not name — in each case the case fails there
        # rather than the reading drifting unproved. Not every edit the case catches is carried as a
        # way: a reader that reads any path naming the directory (`directory in joined`), one that
        # reads every path or none, and one that answers with this directory whatever was asked all
        # red the case and are left to it. A family emptied of the rows it is stated over stops
        # failing here, which the run of this table refuses in turn.
        (("planning/integrity/chain.py", "weaker",
          ("    joined = posixpath.normpath(where)\n", "    joined = where\n")),
         ("planning/integrity/chain.py", "weaker",
          ('    return joined == directory or joined.endswith(f"/{directory}")\n',
           "    return joined.startswith(directory)\n")),
         ("planning/integrity/chain.py", "weaker",
          ('    return joined == directory or joined.endswith(f"/{directory}")\n',
           "    return posixpath.basename(joined) == posixpath.basename(directory)\n"))),
    ),
    (
        "the counts the README states for the data suites are the ones those suites collect",
        "test_the_counts_the_readme_states_are_the_cases_those_suites_collect",
        CHAIN,
        "the README states counts these suites do not collect",
        # Both ways weaken the derivation the case makes rather than the README's sentence: a
        # count that no longer answers for its suite is refused by the case's own equality, so the
        # ways are that answer written wrong (the count collected rather than one fewer) and the
        # walk read over one suite instead of all five. The sentence is the anchor, not a way — a
        # way that edited it would prove the case fails on text the derivation does not read. The
        # case reads this row back in turn, so taking the row out is that case failing rather than
        # a hold gone: the pair cannot be separated in one edit.
        ((CHAIN, "weaker",
          ('    return f"{label} {count}→{count - 1}"\n',
           '    return f"{label} {count}→{count}"\n')),
         (CHAIN, "weaker",
          ("        expected = [one_fewer(label, collected(suite)) for label, suite in PRESENCE]\n",
           "        expected = [one_fewer(label, collected(suite)) for label, suite in PRESENCE[:1]]\n"))),
    ),
]

HOLDER = "planning/integrity/test_revert_rules.py"

# The cases in a file a row states its refusal in that no row names: each checks the shape of these
# tables, of this driver, or of the reading a row proves rather than a refusal the repository
# makes, so losing one is not losing a refusal — declared once per file the way the reader-size
# guard declares what is not reading. The run requires these declarations *and* the rows to account
# for every case in the classes the rows prove their refusals in, across every file the rows name,
# both directions: a case named here that the file no longer collects means one was renamed out of
# it, and a case collected that neither names is one whose loss would be silent. The walk itself is
# no declaration: it reads the files the rows name, and the case below drives all of them.
DECLARED: dict[str, frozenset[str]] = {
    HOLDER: frozenset({
        "test_the_driver_roots_itself_at_this_repository",
        "test_the_driver_scans_the_tree_it_is_held_to",
        "test_every_row_names_a_rule_an_anchor_a_removal_and_a_case",
        "test_every_anchor_is_held_once_in_the_tree",
        "test_no_two_rows_are_the_same_proof",
        "test_every_named_case_is_one_the_suite_runs",
        "test_the_table_names_rules_at_all",
        "test_the_hold_table_names_a_rule_a_case_a_subject_and_its_ways",
        "test_every_hold_row_states_its_refusal_once_and_lays_its_ways_in_the_tree",
    }),
    # The proof file the chain reading lives in: the case below states a shape of that reading
    # rather than a refusal of its own, and the reading's other case is named by the rows above.
    CHAIN: frozenset({
        "test_a_job_is_read_at_any_indentation_and_with_a_comment_or_anchor",
    }),
    # The census owner's own cases: one is named by the row above, and these state the other halves
    # of the census — the figures a document states, the case that holds it, the reason an entry no
    # case reads, and the reading's own spellings — so a census that stopped stating one of them is
    # refused here rather than reading as a census still holding.
    CONTRACTS: frozenset({
        "test_every_classified_document_states_the_figures_the_census_records",
        "test_every_entry_no_case_reads_states_why",
        "test_every_held_document_is_read_by_the_case_that_holds_it",
        "test_the_figure_reading_is_the_one_this_census_states",
    }),
}


def cases_in(tree: pathlib.Path, where: str = HOLDER,
             only: set[str] | None = None) -> set[str]:
    """Every case a loader collects from `where`: the `test…` methods of each class there that
    extends `unittest.TestCase`, or of the classes in `only` when it names them, read from its text
    rather than by importing it.
    """
    source = tree / where
    if not source.is_file():
        # A deleted file holds no case: the accounting below reports every name it should have
        # collected rather than raising where a refusal by name belongs.
        return set()
    found: set[str] = set()
    for node in ast.walk(ast.parse(source.read_text())):
        if not isinstance(node, ast.ClassDef):
            continue
        if "TestCase" not in {ast.unparse(base).split(".")[-1] for base in node.bases}:
            continue
        if only is not None and node.name not in only:
            continue
        found |= {child.name for child in node.body
                  if isinstance(child, ast.FunctionDef) and child.name.startswith("test")}
    return found


def proving_classes(tree: pathlib.Path, where: str) -> set[str]:
    """The classes in `where` that hold a case a row names: a row names the case that states its
    refusal, and the class that case sits in is the class the refusal is proved in — so every case
    beside it is a proof of the same reading, and losing one is losing a proof nothing else states.
    Read from the file's text, so a renamed class is a class this names none of.
    """
    source = tree / where
    if not source.is_file():
        return set()
    named = {row[1] for row in HOLDS if len(row) == 5 and row[2] == where}
    if not named:
        return set()
    found: set[str] = set()
    for node in ast.walk(ast.parse(source.read_text())):
        if not isinstance(node, ast.ClassDef):
            continue
        if named & {child.name for child in node.body if isinstance(child, ast.FunctionDef)}:
            found.add(node.name)
    return found


def unheld_cases(tree: pathlib.Path) -> str | None:
    """Why the cases of the classes the rows prove their refusals in are not all accounted for, or
    None when they are. Every file a row states its refusal in is asked — a row over a new file
    makes that file's cases accountable without a declaration of its own: the classes holding a
    case a row names, and every case in them either named by a row or declared — because a case
    that cannot hold its own presence is the case a row is shown against, and the ones beside it
    are the proofs of the same reading, which nothing else states. A subject this tree does not
    hold is left to the rows that name it, which refuse a file that is not there by name.
    """
    for where in sorted({row[2] for row in HOLDS if len(row) == 5}):
        if not (tree / where).is_file():
            continue
        cases = cases_in(tree, where, proving_classes(tree, where))
        accounted = {row[1] for row in HOLDS if len(row) == 5 and row[2] == where}
        accounted |= set(DECLARED.get(where, frozenset()))
        missing = sorted(accounted - cases)
        extra = sorted(cases - accounted)
        if missing:
            return (f"{', '.join(missing)} is named by a row or declared for {where}, and it "
                    f"collects no such case, so it was renamed out of the suite")
        if extra:
            return (f"{', '.join(extra)} is a case in {where} that no row names and no declaration "
                    f"accounts for, so losing it would be silent")
    return None


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


def history(tree: pathlib.Path, subject: str) -> list[str]:
    """Every revision of `subject` this checkout holds, newest first — one alone in a shallow
    clone, none in a source export."""
    log = subprocess.run(["git", "log", "--format=%H", "--", subject], cwd=tree,
                         capture_output=True, text=True)
    return log.stdout.split()


def smaller_revision(tree: pathlib.Path, subject: str) -> str | None:
    """The revision of `subject` that held the fewest lines, when it held fewer than the tree does
    — the state a rule about the subject's own size must refuse, and the tip a push would name."""
    now = len((tree / subject).read_text().splitlines())
    sizes: dict[str, int] = {}
    for revision in history(tree, subject):
        shown = subprocess.run(["git", "show", f"{revision}:{subject}"], cwd=tree,
                               capture_output=True, text=True)
        if shown.returncode == 0:
            sizes[revision] = len(shown.stdout.splitlines())
    if not sizes:
        return None
    smallest = min(sizes, key=lambda revision: sizes[revision])
    return smallest if sizes[smallest] < now else None


def shown(subject: pathlib.Path, way: tuple[str, str, str | tuple[str, str]], case: str,
          tree: pathlib.Path, scratch: pathlib.Path, states: str | None = None) -> str | None:
    """Why this way does not make the case fail, or None when it does: the file is mutated in the
    copy for the length, removal and weakening ways, the case is run against a named tip for the
    history way, and every way names text this tree holds so the driver cannot be told to mutate
    nothing.
    """
    where, how, argument = way
    target = tree / where
    if not target.is_file():
        return f"{where} is not in the tree"
    text = target.read_text()
    if how == "earlier":
        smaller = smaller_revision(tree, where)
        if smaller is None:
            held = history(tree, where)
            if len(held) <= 1:
                return (f"this checkout holds {len(held)} revision of {where}, so the smaller "
                        f"guard the refusal is shown against is not in it: a shallow clone or a "
                        f"source export cannot hold this rule — fetch the history "
                        f"(`fetch-depth: 0`, or `git fetch --unshallow`)")
            return f"no revision of {where} holds a smaller guard, so the refusal refuses nothing"
        payload = scratch / f"{target.stem}-before.json"
        payload.write_text(json.dumps({"before": smaller}))
        why = case_fails(subject, case, {"GITHUB_EVENT_NAME": "push",
                                         "GITHUB_EVENT_PATH": str(payload)})
        said = f"against a push naming {where}'s smaller guard"
    elif how == "larger":
        declared_number = declared(text, argument)
        if declared_number is None:
            return f"{where} declares no number after {argument!r}"
        target.write_text(text.rstrip("\n")
                          + "\n# one line past this cap\n" * max(0, declared_number + 1
                                                                  - len(text.splitlines())))
        why, said = case_fails(subject, case, {}), f"with {where} a line past its declared cap"
    elif how == "gone":
        if text.count(argument) != 1:
            return f"{where} does not carry what the refusal is about exactly once"
        target.write_text(text.replace(argument, ""))
        why, said = case_fails(subject, case, {}), f"with the link taken out of {where}"
    elif how == "weaker":
        before, after = argument
        if text.count(before) != 1:
            return f"{where} does not carry what the refusal is about exactly once"
        target.write_text(text.replace(before, after))
        why, said = case_fails(subject, case, {}), f"with {where} weakened"
    elif how == "mutates":
        # The state is written by the reading the case itself makes (`chain`), never by matching the
        # workflow's text, so a spelling the case accepts is a state this way can reach and a way
        # cannot demand one the case calls satisfied.
        changed = chain.mutated(text, argument)
        if changed == text or argument not in chain.MUTATIONS:
            return (f"{where} is unchanged by {argument!r}: the states this driver writes are "
                    f"{', '.join(chain.MUTATIONS)}")
        target.write_text(changed)
        why = case_fails(subject, case, {}, states)
        said = f"with the workflow in the state {argument!r}"
    else:
        return (f"{how!r} is not a way this driver shows a refusal: name "
                f"{', '.join(WAYS)}")
    target.write_text(text)
    return None if why is None else f"{why} {said}"


# The inputs this driver's own checker must refuse: one crafted way per kind, each built so its case
# stays green when the way is applied, so a checker reading any of them as held no longer refuses.
# `main` reads them every run, which is how the hold proves its checker rather than trusting it.
NEGATIVE: tuple[tuple[str, str, str | tuple[str, str]], ...] = (
    ("link.txt", "gone", "a link\n"),
    ("cap.txt", "larger", "CAP = "),
    ("cap.txt", "earlier", ""),
    ("cap.txt", "weaker", ("CAP = 1", "CAP = 0")),
    ("workflow.yml", "mutates", chain.NO_DRIVER),
)


def negative_inputs(scratch: pathlib.Path) -> tuple[pathlib.Path, pathlib.Path]:
    """The tree those ways are read in and the module they act on: a case that passes whatever they
    do, the link `gone` would take out, and the declaration `larger` counts to."""
    tree = scratch / "negative"
    tree.mkdir(parents=True, exist_ok=True)
    (tree / "test_negative.py").write_text(
        "import unittest\n\n\nclass T(unittest.TestCase):\n    def test_ok(self):\n        pass\n")
    (tree / "link.txt").write_text("a link\n")
    (tree / "cap.txt").write_text("CAP = 1\n")
    (tree / "workflow.yml").write_text(
        "jobs:\n  check:\n    steps:\n      - name: the checks\n"
        "        run: python -m unittest discover -s planning/integrity\n")
    return tree, tree / "test_negative.py"


def self_proof(scratch: pathlib.Path) -> str | None:
    """Why this driver cannot show what it must refuse, or None when it can: one row per kind of way,
    each stating the refusal a hold must refuse in this very tree, plus the rows a hold must refuse —
    one naming no way, one whose subject states no such refusal. They are read through `holds`, so a
    checker that stopped refusing shows up as one of them reading as held, and vacating the checker is
    caught by this run rather than by a reading of its text.
    """
    tree, _subject = negative_inputs(scratch)
    stated = "class T(unittest.TestCase):"
    rows = [(f"a {way[1]} way that does not bite", stated, (way,)) for way in NEGATIVE]
    rows += [("a row naming no way", stated, ()),
             ("a subject that states no such refusal", "nothing here says this", NEGATIVE[:1])]
    kinds = {way[1] for row in HOLDS if len(row) == 5 for way in row[4]}
    refuses = {way[1] for way in NEGATIVE}
    if kinds != set(WAYS) or refuses != set(WAYS):
        return (f"the rows name {sorted(kinds)} and the inputs above refuse {sorted(refuses)}, "
                f"where this file states the ways as {sorted(WAYS)} — a kind no input is built to "
                f"refuse, or one no row shows, can be added or dropped without this run seeing it")
    for what, states, ways in rows:
        why = holds(("a rule this driver must not hold", "test_ok", "test_negative.py", states, ways),
                    tree, scratch)
        if why is None:
            return f"{what} reads as held"
    return None


def case_fails(path: pathlib.Path, case: str, env: dict[str, str],
               states: str | None = None) -> str | None:
    """Why the case does not fail when it must, or None when it does.

    `states` is the refusal a row says the case states, and is asked for by the way that writes a
    workflow into a state: then the failure has to be that refusal rather than any earlier one, so a
    reading vacated into failing somewhere above the assertion reads as nothing here.
    """
    run = subprocess.run(
        [sys.executable, "-m", "unittest", "-v", path.stem],
        cwd=path.parent, capture_output=True, text=True,
        env={**os.environ, "PYTHONDONTWRITEBYTECODE": "1", **env},
    )
    output = run.stdout + run.stderr
    if f"{case} (" not in output:
        return f"{case} is not a case the suite runs"
    # The named case has to be the one that failed: a module where something else broke holds this
    # row for the wrong reason, which is how a way that places nothing would read as biting.
    if run.returncode == 0 or not any(f"{kind}: {case} (" in output for kind in ("FAIL", "ERROR")):
        return f"{case} passes where the rule forbids that state"
    if states is not None and states not in output:
        return (f"{case} fails for another reason than the refusal the row states, so what made it "
                f"fail need not be the state this way wrote")
    return None


def declared(text: str, prefix: str) -> int | None:
    """The number `text` declares after `prefix`, or None when it declares none."""
    found = re.search(rf"^{re.escape(prefix)}(\d+)$", text, re.M)
    return int(found.group(1)) if found else None


def holds(row: tuple, tree: pathlib.Path, scratch: pathlib.Path) -> str | None:
    """Why this row does not hold its refusal, or None when it does: the row names the five parts a
    hold states, the subject is in the tree and states the refusal once, and every way the row names
    makes the case fail. A row naming no way holds nothing, which is refused here rather than read
    as a row with nothing to check, and so is a row naming fewer parts than a hold states.
    """
    if len(row) != 5:
        return f"names {len(row)} parts, not the five a hold states"
    _rule, case, stated_in, states, ways = row
    subject = tree / stated_in
    if not subject.is_file():
        return f"{stated_in} is not in the tree"
    if subject.read_text().count(states) != 1:
        return f"{stated_in} does not state the refusal exactly once"
    if not ways:
        return "names no way to show its refusal, so it holds nothing"
    for way in ways:
        why = shown(subject, way, case, tree, scratch, states)
        if why is not None:
            return why
    return None


def main() -> int:
    rows = [row for row in HOLDS if len(row) == 5]
    watched = [path for path in (scanned(ROOT) + [ROOT / row[2] for row in rows]
                                 + [ROOT / way[0] for row in rows for way in row[4]])
               if path.is_file()]
    before = digest(watched)
    bit, missing, stale, silent, unheld, kept = [], [], [], [], [], 0
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
        for name, table in (("the rule table", RULES), ("the hold table", HOLDS),
                            ("the inputs the hold refuses", NEGATIVE)):
            if not table:
                missing.append((name, "names no row, so no refusal is held"))
                print(f"MISSING: {name} names no row, so no refusal is held")
        proof = self_proof(pathlib.Path(scratch))
        if proof is not None:
            unheld.append(("this driver's own checker", proof))
            print(f"NOT HELD: this driver's own checker ({proof})")
        unaccounted = unheld_cases(tree)
        if unaccounted is not None:
            unheld.append(("the cases the rows are proved in", unaccounted))
            print(f"NOT HELD: the cases the rows are proved in ({unaccounted})")
        for row in HOLDS:
            name = row[0] if row else "a hold row with no name"
            why = holds(row, tree, pathlib.Path(scratch))
            if why is None:
                kept += 1
                print(f"HELD: {name}")
            else:
                unheld.append((name, why))
                print(f"NOT HELD: {name} ({why})")
    finally:
        shutil.rmtree(scratch, ignore_errors=True)

    after = digest(watched)
    moved = [name for name in before if after.get(name) != before[name]]
    print(
        f"\n{len(bit)}/{len(RULES)} rules bit; {len(missing)} missing tests; "
        f"{len(stale)} stale anchors; {len(silent)} silent; "
        f"{kept}/{len(HOLDS)} cases held by their refusals; "
        f"{len(moved)} files of the live tree moved ({', '.join(moved) or 'none'})"
    )
    return 1 if missing or stale or silent or unheld or moved else 0


if __name__ == "__main__":
    sys.exit(main())
