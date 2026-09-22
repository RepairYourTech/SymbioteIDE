"""Every refusal the parity matrices claim, driven alone, and the readings #172 asks for.

The committed matrices are a document, so each state here mutates a copy in memory rather than a file
on disk: what is measured is the rule, and nothing in this repository is written to measure it. Both
directions wherever a rule can be read both ways — a state that breaks the rule is refused naming its
subject, and an honest edit of the same shape passes. The movement cases assert what a reviewer would
otherwise have to trust: that a registry change touches the rows resting on it, that a locked row is
reported locked rather than recomputed, and that a changed capability no `watch` row binds is
reported rather than silently dropped.
"""
from __future__ import annotations

import json
import pathlib
import re
import unittest

import parity as P


def document() -> dict:
    return json.loads(P.MATRICES.read_text())


def committed(change) -> P.Matrices:
    """The committed matrices with one change applied, read back through the reader."""
    value = document()
    change(value)
    return P.Matrices(value, "the committed matrices", P.research.registry())


def refusal(where: list[str], text: str) -> None:
    assert any(text in one for one in where), f"no refusal names {text!r}: {where}"


def row(value: dict, capability: str) -> dict:
    for one in value["rows"]:
        if one["capability"] == capability:
            return one
    raise AssertionError(f"{capability} is not classified by the committed matrices")


def pattern(value: dict, name: str) -> dict:
    for one in value["patterns"]:
        if one["pattern"] == name:
            return one
    raise AssertionError(f"{name!r} is not a pattern the committed matrices state")


def problems(value: P.Matrices) -> list[str]:
    return P.problems(value, P.program_entries(), P.ledger_ids())


class TheCommittedMatrices(unittest.TestCase):
    """The file as it is committed."""

    def test_the_committed_matrices_pass_every_rule(self):
        self.assertEqual(problems(P.matrices()), [])

    def test_every_capability_the_registry_carries_is_classified_once(self):
        carried = set(P.research.registry().capabilities)
        classified = {one["capability"] for one in P.matrices().rows}
        self.assertEqual(carried - classified, set(),
                         "a capability the registry carries and no row classifies is a market axis "
                         "nothing looked at")
        refusal(problems(committed(lambda v: v["rows"].pop(0))), "classified by no row")
        refusal(problems(committed(lambda v: v["rows"].append(dict(v["rows"][0])))),
                "is classified twice")

    def test_every_class_of_comparison_is_carried_by_a_row(self):
        carried = {one["class"] for one in P.matrices().rows}
        self.assertEqual(set(P.CLASSES) - carried, set(),
                         "a class nothing carries is a distinction this file cannot draw")
        refusal(problems(committed(
            lambda v: row(v, "orchestration.task-hierarchy").update({"class": "COPY_IT"}))),
            "which is not a class of comparison")

    def test_every_row_maps_to_a_requirement_an_issue_or_a_deferral(self):
        for one in P.matrices().rows:
            self.assertTrue(one.get("requirements") or one.get("issues")
                            or str(one.get("deferral", "")).strip(), one["capability"])
        refusal(problems(committed(
            lambda v: row(v, "runtime.structured-transport").update({"requirements": []}))),
            "maps to no requirement, no issue and no deferral")

    def test_every_requirement_and_decision_a_row_names_is_one_the_tree_carries(self):
        refusal(problems(committed(
            lambda v: row(v, "runtime.harness-support").update({"requirements": ["ZZZ-99"]}))),
            "is not a requirement key this program carries")
        refusal(problems(committed(
            lambda v: row(v, "artifacts.spec-artifacts").update({"decisions": ["ADR-9999/NONE"]}))),
            "which the architecture ledger does not hold")


class TheEvidenceARowRestsOn(unittest.TestCase):
    """A claim about us needs current evidence about them, from the registry, about that axis."""

    def test_a_differentiation_claim_rests_on_current_evidence(self):
        refusal(problems(committed(
            lambda v: row(v, "orchestration.agent-identity").update({"evidence": []}))),
            "with no competitor evidence")
        refusal(problems(committed(
            lambda v: row(v, "orchestration.agent-identity").update(
                {"evidence": [{"product": "orca", "capability": "runtime.harness-support"}]}))),
            "which is not the axis this row classifies")
        refusal(problems(committed(
            lambda v: row(v, "orchestration.agent-identity").update(
                {"evidence": [{"product": "nobody", "capability": "orchestration.agent-identity"}]}))),
            "which the registry does not hold")
        # A claim the registry holds about the axis but reads UNKNOWN cannot carry a claim of ours:
        # driven on runtime.harness-support, whose only other claimant is a gap.
        refusal(problems(committed(
            lambda v: row(v, "runtime.harness-support").update(
                {"class": "DIFFERENTIATOR",
                 "evidence": [{"product": "traycer", "capability": "runtime.harness-support"}]}))),
            "which cannot carry a DIFFERENTIATOR")
        # An honest edit of the same shape: a table-stakes row may point at any claim about its axis,
        # and a differentiation row resting on an inferred-but-real claim passes.
        self.assertEqual(problems(committed(
            lambda v: row(v, "runtime.harness-support").update(
                {"evidence": [{"product": "t3code", "capability": "runtime.harness-support"}]}))), [])

    def test_an_unknown_states_a_research_question(self):
        refusal(problems(committed(
            lambda v: row(v, "workbench.mobile-remote").update({"research": ""}))),
            "states no research question")
        self.assertEqual(problems(committed(
            lambda v: row(v, "workbench.mobile-remote").update({"research": "asks the same question "
                                                                           "in another way"}))), [])

    def test_an_adjacent_opportunity_comes_from_more_than_one_product(self):
        refusal(problems(committed(
            lambda v: row(v, "orchestration.verification-behavior").update(
                {"evidence": [{"product": "traycer",
                               "capability": "orchestration.verification-behavior"}]}))),
            "an opportunity read off one product")
        # The committed row cites two, which is what makes it an opportunity rather than a copy.
        self.assertEqual(len(row(document(), "orchestration.verification-behavior")["evidence"]), 2)


class TheDecisions(unittest.TestCase):
    """Borrow, adapt, reject or research — on a real axis, with a rationale and a risk."""

    def test_a_rejected_pattern_keeps_its_durable_decision(self):
        refusal(problems(committed(
            lambda v: v["patterns"].remove(pattern(v, "a proprietary parallel runtime environment")))),
            "with no REJECT decision on it")
        refusal(problems(committed(
            lambda v: pattern(v, "a proprietary parallel runtime environment").update(
                {"decision": "BORROW"}))), "with no REJECT decision on it")
        refusal(problems(committed(
            lambda v: pattern(v, "a proprietary parallel runtime environment").update(
                {"capability": "runtime.harness-support"}))), "with no REJECT decision on it")

    def test_a_pattern_that_would_move_a_locked_decision_says_so(self):
        refusal(problems(committed(
            lambda v: pattern(v, "artifact provenance carries the decision it serves").update(
                {"changes_locked": False}))),
            "and claims to move nothing")
        refusal(problems(committed(
            lambda v: pattern(v, "stable teammate identity across workspaces").update(
                {"changes_locked": "perhaps"}))),
            "does not say whether it would move a locked decision")
        # The committed file records exactly one such pattern, and the reading prints it for review.
        self.assertEqual([one["pattern"] for one in P.matrices().patterns
                          if one["changes_locked"] is True],
                         ["artifact provenance carries the decision it serves"])

    def test_a_pattern_rests_on_a_claim_the_registry_holds(self):
        refusal(problems(committed(
            lambda v: pattern(v, "provider drivers and instances behind one contract").update(
                {"source": {"product": "nobody", "capability": "runtime.harness-support"}}))),
            "which the registry does not hold as a claim")
        refusal(problems(committed(
            lambda v: pattern(v, "provider drivers and instances behind one contract").update(
                {"capability": "runtime.not-a-thing"}))),
            "which the registry's taxonomy does not carry")
        refusal(problems(committed(
            lambda v: pattern(v, "provider drivers and instances behind one contract").update(
                {"risk": ""}))), "states no risk")
        refusal(problems(committed(
            lambda v: pattern(v, "provider drivers and instances behind one contract").update(
                {"decision": "COPY"}))), "which is not one of BORROW, ADAPT, REJECT, RESEARCH")

    def test_every_pattern_maps_to_an_issue_the_program_carries(self):
        refusal(problems(committed(
            lambda v: pattern(v, "a terminal floor that runs any agent CLI").update(
                {"issues": [999999]}))), "which this program does not carry")


class TheWorkflows(unittest.TestCase):
    """End-to-end chains with owners and handoffs, not feature lists."""

    def test_a_workflow_states_steps_owners_and_a_handoff(self):
        refusal(problems(committed(
            lambda v: v["workflows"][0].update({"handoffs": []}))),
            "is a feature list rather than a workflow")
        refusal(problems(committed(
            lambda v: v["workflows"][0].update(
                {"steps": [{"owner": "symbiote", "action": "one"}, {"owner": "symbiote", "action": "two"}]}))),
            "workflow shorter than an end-to-end pass")
        refusal(problems(committed(
            lambda v: v["workflows"][0]["steps"][1].update({"owner": "somebody"}))),
            "which is neither a product in the universe nor this repository")
        refusal(problems(committed(
            lambda v: v["workflows"][0]["handoffs"][0].update({"at": 99}))),
            "which the workflow does not have")
        refusal(problems(committed(
            lambda v: v["workflows"][0]["handoffs"][0].update({"consequence": ""}))),
            "states a gap with no consequence")

    def test_at_least_one_handoff_leaves_the_product(self):
        def internal_only(value):
            for workflow in value["workflows"]:
                for handoff in workflow["handoffs"]:
                    handoff["kind"] = "internal"

        refusal(problems(committed(internal_only)), "no workflow names a cross-product handoff")
        refusal(problems(committed(
            lambda v: v["workflows"][0]["handoffs"][0].update({"kind": "wherever"}))),
            "which this reader does not know")
        # The committed file leaves the product in five of its six workflows.
        crossing = [workflow["id"] for workflow in P.matrices().workflows
                    if any(one["kind"] == "cross_product" for one in workflow["handoffs"])]
        self.assertEqual(len(crossing), 5, crossing)


class TheRevisions(unittest.TestCase):
    """A claim of ours a competitor turned out to match, revised rather than kept."""

    def test_a_revision_withdraws_a_claim_and_names_what_matched_it(self):
        refusal(problems(committed(lambda v: v.update({"revisions": []}))),
                "no claim of ours was ever tested")
        refusal(problems(committed(
            lambda v: v["revisions"][0].update({"matched_by": {"product": "traycer",
                                                               "capability": "runtime.harness-support"}}))),
                "which is not evidence that a competitor matches anything")
        refusal(problems(committed(
            lambda v: v["revisions"][0].update({"was": ""}))), "does not state what was says")
        refusal(problems(committed(
            lambda v: v["revisions"][0].update({"capability": "runtime.nothing"}))),
            "an axis the registry does not carry")
        # Both committed revisions are against current claims, which is what makes them revisions.
        for revision in P.matrices().revisions:
            claim = P.matrices().claim(revision["matched_by"]["product"],
                                       revision["matched_by"]["capability"])
            self.assertNotIn(claim["state"], ("UNKNOWN", "STALE"), revision["capability"])


class TheReadings(unittest.TestCase):
    """The movement and matrix readings, driven rather than described."""

    def test_a_movement_touches_the_rows_resting_on_a_change(self):
        report = P.movement(P.matrices(), "2026-08-15", "registry")
        affected = {one["capability"] for one in report["affected"]}
        self.assertIn("runtime.harness-support", affected,
                      "the committed movement changes that axis and the row rests on it")
        self.assertTrue(report["changes"], "the two committed states differ")

    def test_a_movement_reports_locked_rows_locked_rather_than_recomputed(self):
        report = P.movement(P.matrices(), "2026-08-15", "registry")
        locked = {one["capability"] for one in report["locked"]}
        self.assertEqual(locked, {"artifacts.spec-artifacts", "operations.hosting-model"})
        self.assertFalse(locked & {one["capability"] for one in report["affected"]},
                         "a locked row must not appear among the rows to recompute")

    def test_a_movement_reports_a_change_nothing_watches(self):
        report = P.movement(P.matrices(), "2026-08-15", "registry")
        self.assertTrue(report["unwatched"], "most capabilities carry no watch row and the reading "
                                             "must say so rather than dropping them")
        self.assertNotIn("runtime.harness-support", report["unwatched"],
                         "that capability is bound by a watch row and is therefore watched")

    def test_a_movement_refuses_a_state_the_registry_does_not_carry(self):
        with self.assertRaises(P.Refused):
            P.movement(P.matrices(), "1999-01-01", "registry")

    def test_a_matrix_reading_prints_the_rows_of_one_matrix(self):
        report = P.matrix_report(P.matrices(), "runtime")
        expected = [one for one in P.matrices().rows if one["matrix"] == "runtime"]
        self.assertTrue(expected, "the committed matrices carry rows for the matrix below")
        # The rows themselves, not only their number: a reading held by a count the same reader
        # produced cannot tell an empty list, or another matrix's rows, from the ones it read.
        self.assertEqual([one["capability"] for one in report["rows"]],
                         [one["capability"] for one in expected],
                         "the reading prints the rows of the matrix it names, in the file's order")
        self.assertEqual(report["count"], len(expected))
        self.assertTrue(all(one["class"] in P.CLASSES for one in report["rows"]))
        with self.assertRaises(P.Refused):
            P.matrix_report(P.matrices(), "no-such-matrix")


class HonestEdits(unittest.TestCase):
    """What the rules must not refuse."""

    def test_a_reworded_stance_a_renamed_workflow_and_a_reordered_file_pass(self):
        self.assertEqual(problems(committed(
            lambda v: row(v, "workbench.terminal-floor").update(
                {"stance": "A terminal floor is the price of entry."}))), [])
        self.assertEqual(problems(committed(
            lambda v: v["workflows"][0].update({"id": "idea-to-specification"}))), [])
        self.assertEqual(problems(committed(lambda v: v["rows"].reverse())), [])
        self.assertEqual(problems(committed(
            lambda v: row(v, "workbench.preview-browser").update(
                {"deferral": "still an experiment, said in another sentence"}))), [])


class TheReadme(unittest.TestCase):
    """The figures the README states are the ones this file measures, each written beside the noun
    it counts, so neither a count nor the thing it counts can drift the way prose does."""

    def test_the_readme_states_the_counts_this_file_measures(self):
        readme = (P.ROOT / "planning/parity/README.md").read_text()
        measured = P.matrices()
        counts = {"rows": len(measured.rows), "matrices": len(measured.matrices),
                  "workflows": len(measured.workflows), "patterns": len(measured.patterns),
                  "revisions": len(measured.revisions)}
        # The figure read beside its own noun rather than the figure found anywhere: measured, a
        # case that asked only whether the five numerals were present stayed green when two of them
        # were swapped in place, so `**7** rows over **18** matrices` passed as the committed counts
        # and a figure moved to the noun beside it went unread too. A count written twice is held as
        # well — every copy read here must state the same figure — which is how the policy, release
        # and bake-off records' own README cases read theirs. The limit, stated: a figure written
        # without the emphasis this reads, or counting something this record does not carry, is not
        # read by this.
        for noun, number in counts.items():
            written = re.findall(rf"\*\*(\d+)\*\* +{noun}\b", readme)
            self.assertTrue(written, f"the README states no {noun} figure, which this file measures "
                                     f"as {number}")
            self.assertEqual([int(one) for one in written], [number] * len(written),
                             f"the README states {written} {noun} where this file measures "
                             f"{number}: a figure belongs beside the noun it counts")
