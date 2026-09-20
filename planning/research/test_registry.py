"""Every refusal the competitor registry claims, driven alone, and the readings the issue asks for.

The committed registry is a document, so each state here mutates a copy in memory rather than a
file on disk: what is measured is the rule, and nothing in this repository is written to measure it.
Snapshots arrive through the file system, so the two snapshot states are driven in a temporary copy
of `snapshots/`, which is removed when the case ends.

Both directions everywhere a rule can be read both ways: a claim that breaks the rule is refused
naming its subject, and an honest edit of the same shape passes. The delta cases assert what a
reviewer would have to trust otherwise — that a rename is one change rather than two, that a change
to a capability a `watch` row binds appears against the row, and that a locked decision is reported
locked rather than reported changed.
"""
from __future__ import annotations

import copy
import json
import pathlib
import shutil
import tempfile
import unittest

import registry as R


def document() -> dict:
    return json.loads(R.REGISTRY.read_text())


def commit(change) -> R.Registry:
    """The committed registry with one change applied, read back through the reader."""
    value = document()
    change(value)
    return R.Registry(value, "the committed registry")


def history() -> set[str]:
    found = set(document()["products"][index]["id"] for index in range(len(document()["products"])))
    for name in R.snapshots():
        found.update(R.load_snapshot(R.ROOT, name).products)
    return found


def claim(value: dict, product: str, capability: str) -> dict:
    for entry in value["products"]:
        if entry["id"] == product:
            for one in entry["claims"]:
                if one["capability"] == capability:
                    return one
    raise AssertionError(f"{product}/{capability} is not in the committed registry")


def refusal(where: list[str], text: str) -> None:
    assert any(text in one for one in where), f"no refusal names {text!r}: {where}"


class TheCommittedRegistry(unittest.TestCase):
    """The registry and its snapshots as they are committed."""

    def test_the_committed_registry_and_every_snapshot_pass_every_rule(self):
        self.assertEqual(R.problems(R.registry(), history()), [])
        for name in R.snapshots():
            R.load_snapshot(R.ROOT, name)

    def test_every_claim_states_a_state_and_an_observation(self):
        for product, one in R.registry().claims():
            self.assertIn(one["state"], R.STATES, f"{product}/{one['capability']}")
            self.assertTrue(one.get("observation", "").strip(),
                            f"{product}/{one['capability']} states no observation")

    def test_the_universe_and_the_dossiers_agree_in_both_directions(self):
        committed = R.registry()
        missing = [one for one in committed.universe if one not in committed.products]
        extra = [one for one in committed.products if one not in committed.universe]
        self.assertEqual((missing, extra), ([], []))
        refusal(R.problems(commit(lambda v: v["products"].pop(0)), history()),
                "is in the universe and carries no dossier")
        refusal(R.problems(commit(lambda v: v["universe"].pop(0)), history()),
                "carries a dossier and is not in the universe")

    def test_every_taxonomy_capability_states_its_staleness_window(self):
        committed = R.registry()
        for capability, (_, entry) in committed.capabilities.items():
            self.assertIsInstance(entry.get("window_days"), int, capability)
        refusal(R.problems(commit(lambda v: v["taxonomy"]["dimensions"][0]["capabilities"][0]
                                  .pop("window_days")), history()),
                "states no staleness window")


class TheEvidenceDiscipline(unittest.TestCase):
    """What may stand behind a claim, and what a claim must say when nothing does."""

    def test_community_evidence_alone_cannot_carry_a_verified_state(self):
        def mutate(value):
            one = claim(value, "orca", "workbench.terminal-floor")
            one["evidence"] = [{"kind": "community", "source": "https://example.invalid/",
                                "retrieved_at": "2026-09-20", "confidence": "low",
                                "excerpt": "a forum said so"}]
        found = R.problems(commit(mutate), history())
        refusal(found, "orca/workbench.terminal-floor reads VERIFIED_CURRENT")
        refusal(found, "community stands behind it")

    def test_a_claim_without_authoritative_evidence_states_a_research_gap(self):
        def mutate(value):
            claim(value, "orca", "workbench.mobile-remote").pop("gap")
        refusal(R.problems(commit(mutate), history()),
                "orca/workbench.mobile-remote has no authoritative evidence and states no research gap")

    def test_evidence_must_carry_a_source_a_date_and_an_excerpt(self):
        def mutate(value):
            claim(value, "orca", "workbench.terminal-floor")["evidence"][0].pop("excerpt")
        refusal(R.problems(commit(mutate), history()),
                "orca/workbench.terminal-floor cites evidence with no excerpt")

    def test_a_stale_claim_may_not_read_as_current(self):
        def mutate(value):
            value["observed_at"] = "2027-09-20"
        found = R.problems(commit(mutate), history())
        refusal(found, "reads VERIFIED_CURRENT on evidence 365 days old")
        refusal(found, "past its 180-day window; a stale claim reads STALE")

    def test_a_guess_with_authoritative_evidence_behind_it_is_refused(self):
        def mutate(value):
            one = claim(value, "t3code", "operations.hosting-model")
            one["evidence"].append({"kind": "official_site", "source": "https://example.invalid/",
                                    "retrieved_at": "2026-09-20", "confidence": "high",
                                    "excerpt": "official words"})
        refusal(R.problems(commit(mutate), history()),
                "t3code/operations.hosting-model reads INFERRED with an authoritative source")

    def test_evidence_retrieved_after_the_registry_was_observed_is_refused(self):
        def mutate(value):
            claim(value, "orca", "workbench.terminal-floor")["evidence"][0]["retrieved_at"] = "2026-10-01"
        refusal(R.problems(commit(mutate), history()),
                "cites evidence retrieved after the registry was observed")


class TheHarnessGrade(unittest.TestCase):
    """A terminal is not a structured integration, and the ladder is the taxonomy's."""

    def test_a_terminal_observation_cannot_be_graded_structured(self):
        def mutate(value):
            one = claim(value, "orca", "runtime.harness-support")
            one["grade"] = "structured"
        found = R.problems(commit(mutate), history())
        refusal(found, "is graded structured over 'pty'")
        refusal(found, "is not evidence of structured integration")

    def test_a_deep_grade_needs_an_authoritative_interface_behind_it(self):
        def mutate(value):
            one = claim(value, "t3code", "runtime.harness-support")
            one["grade"] = "orchestrator_grade"
            one["transport"] = "rpc"
        refusal(R.problems(commit(mutate), history()),
                "is graded orchestrator_grade with no authoritative evidence of the interface")

    def test_a_terminal_grade_over_a_structured_transport_is_refused(self):
        def mutate(value):
            claim(value, "orca", "runtime.harness-support")["transport"] = "rpc"
        refusal(R.problems(commit(mutate), history()),
                "is graded terminal_compatible over 'rpc'")

    def test_a_grade_for_a_capability_the_taxonomy_does_not_grade_is_refused(self):
        def mutate(value):
            claim(value, "orca", "workbench.terminal-floor")["grade"] = "managed"
        refusal(R.problems(commit(mutate), history()),
                "states a support grade for a capability the taxonomy does not grade")

    def test_an_unknown_integration_states_no_grade_and_its_gap_instead(self):
        # The honest shape for a claim that read nothing: no grade, and the gap says what to read.
        committed = R.registry()
        self.assertIsNone(claim(document(), "traycer", "runtime.harness-support").get("grade"))
        self.assertIn("traycer", committed.products)


class TheTaxonomyAndIdentities(unittest.TestCase):
    """Capabilities are the taxonomy's, one claim each, and a rename replaces what it names."""

    def test_a_capability_outside_the_taxonomy_is_refused(self):
        def mutate(value):
            claim(value, "orca", "workbench.terminal-floor")["capability"] = "workbench.unknown-row"
        refusal(R.problems(commit(mutate), history()),
                "claims 'workbench.unknown-row', which the taxonomy does not carry")

    def test_a_product_claiming_one_capability_twice_is_refused(self):
        def mutate(value):
            for entry in value["products"]:
                if entry["id"] == "orca":
                    entry["claims"].append(copy.deepcopy(entry["claims"][0]))
        refusal(R.problems(commit(mutate), history()),
                "orca claims orchestration.parallel-worktrees twice")

    def test_a_rename_that_replaces_something_present_is_refused(self):
        def mutate(value):
            for entry in value["products"]:
                if entry["id"] == "orca":
                    entry["renamed_from"] = "traycer"
        refusal(R.problems(commit(mutate), history()),
                "declares itself the rename of traycer, and traycer is still in the registry")

    def test_a_rename_must_name_something_the_history_carries(self):
        def mutate(value):
            for entry in value["products"]:
                if entry["id"] == "orca":
                    entry["renamed_from"] = "no-such-product"
        refusal(R.problems(commit(mutate), history()),
                "declares itself the rename of no-such-product, which no snapshot carries")
        # And the honest shape it is measured against: the committed rename of a name a snapshot used.
        self.assertEqual(R.problems(R.registry(), history()), [])

    def test_a_capability_rename_replaces_a_id_the_taxonomy_no_longer_carries(self):
        def mutate(value):
            claim(value, "traycer", "artifacts.spec-artifacts")["renamed_from"] = "artifacts.artifact-lifecycle"
        refusal(R.problems(commit(mutate), history()),
                "and the taxonomy still carries artifacts.artifact-lifecycle")


class Snapshots(unittest.TestCase):
    """Historical snapshots read under the same rules, and name their own date."""

    def setUp(self):
        self.root = pathlib.Path(tempfile.mkdtemp())
        shutil.copytree(R.SNAPSHOTS, self.root / "planning/research/snapshots")

    def tearDown(self):
        shutil.rmtree(self.root)

    def test_a_snapshot_whose_name_is_not_its_date_is_refused(self):
        path = self.root / "planning/research/snapshots/2026-08-15.json"
        value = json.loads(path.read_text())
        value["observed_at"] = "2026-08-16"
        path.write_text(json.dumps(value))
        with self.assertRaises(R.Refused) as raised:
            R.load_snapshot(self.root, "2026-08-15")
        self.assertIn("its own name says 2026-08-15", str(raised.exception))

    def test_a_snapshot_that_breaks_a_rule_is_refused_rather_than_carried(self):
        path = self.root / "planning/research/snapshots/2026-08-15.json"
        value = json.loads(path.read_text())
        for entry in value["products"]:
            if entry["id"] == "orca":
                entry["claims"][0]["evidence"] = []
        path.write_text(json.dumps(value))
        with self.assertRaises(R.Refused) as raised:
            R.load_snapshot(self.root, "2026-08-15")
        self.assertIn("orca/orchestration.parallel-worktrees", str(raised.exception))

    def test_a_snapshot_reconstructs_the_claims_it_recorded(self):
        before = R.load_snapshot(R.ROOT, "2026-08-15")
        recorded = {(product, one["capability"]): one["state"] for product, one in before.claims()}
        self.assertEqual(recorded[("orca", "orchestration.parallel-worktrees")], "VERIFIED_CURRENT")
        self.assertEqual(recorded[("traycer", "artifacts.spec-documents")], "UNKNOWN")
        self.assertEqual(recorded[("bridgemind-one", "orchestration.agent-identity")], "UNKNOWN")

    def test_a_snapshot_that_is_not_there_is_refused_by_name(self):
        with self.assertRaises(R.Refused) as raised:
            R.load_snapshot(self.root, "2026-07-01")
        self.assertIn("no registry at", str(raised.exception))
        self.assertIn("2026-07-01.json", str(raised.exception))


class Deltas(unittest.TestCase):
    """What changed, which rows it touches, and which of those are locked."""

    @classmethod
    def setUpClass(cls):
        cls.before = R.load_snapshot(R.ROOT, "2026-08-15")
        cls.after = R.registry()
        cls.report = R.delta(cls.before, cls.after)
        cls.changes = {(one["product"], one["capability"]): one for one in cls.report["changes"]}

    def test_a_rename_is_one_change_rather_than_a_loss_beside_a_gain(self):
        renamed = [one for one in self.report["changes"] if one["change"] == "renamed"]
        self.assertEqual({(one["product"], one["from"], one["to"]) for one in renamed},
                         {("bridgemind", "bridgemind-one", "bridgemind"),
                          ("traycer", "artifacts.spec-documents", "artifacts.spec-artifacts")})
        self.assertEqual([one for one in self.report["changes"] if one["change"] == "removed"], [])

    def test_a_state_that_moved_is_reported_with_both_ends(self):
        one = self.changes[("traycer", "artifacts.artifact-lifecycle")]
        self.assertEqual((one["change"], one["from"], one["to"]), ("changed", "UNKNOWN", "VERIFIED_CURRENT"))

    def test_a_reworded_observation_is_a_revision_not_a_capability_change(self):
        one = self.changes[("orca", "orchestration.parallel-worktrees")]
        self.assertEqual(one["change"], "revised")

    def test_a_change_a_watch_row_binds_appears_against_the_row(self):
        rows = {one["capability"]: one for one in self.report["affected"]}
        self.assertIn("runtime.harness-support", rows)
        self.assertEqual(rows["runtime.harness-support"]["requirements"], ["FND-05"])
        self.assertEqual(rows["runtime.harness-support"]["decisions"], ["ADR-0001/DESKTOP-SHELL"])

    def test_a_locked_row_is_reported_locked_rather_than_reported_changed(self):
        locked = {one["capability"]: one for one in self.report["locked"]}
        self.assertIn("artifacts.spec-artifacts", locked)
        self.assertTrue(locked["artifacts.spec-artifacts"]["locked"])
        self.assertNotIn("artifacts.spec-artifacts",
                         {one["capability"] for one in self.report["affected"]})

    def test_a_change_no_watch_row_binds_is_visible_rather_than_dropped(self):
        self.assertIn("workbench.voice-dictation", self.report["unwatched"])
        self.assertNotIn("runtime.harness-support", self.report["unwatched"])

    def test_a_delta_with_nothing_between_it_is_empty(self):
        self.assertEqual(R.delta(self.after, self.after)["changes"], [])
        self.assertEqual(R.delta(self.after, self.after)["affected"], [])


class FocusedRefresh(unittest.TestCase):
    """A pass touches the dimension it names, and no locked row."""

    def test_a_plan_inside_its_dimension_passes(self):
        plan = [{"capability": "runtime.usage-visibility", "product": "t3code"}]
        self.assertEqual(R.refresh(R.registry(), "runtime", plan), [])

    def test_a_plan_that_leaves_its_dimension_is_refused(self):
        plan = [{"capability": "artifacts.artifact-lifecycle", "product": "traycer"}]
        refusal(R.refresh(R.registry(), "runtime", plan),
                "a refresh of runtime names artifacts.artifact-lifecycle, which belongs to artifacts")

    def test_a_plan_that_re_observes_a_locked_row_is_refused(self):
        plan = [{"capability": "artifacts.spec-artifacts", "product": "traycer"}]
        refusal(R.refresh(R.registry(), "artifacts", plan),
                "which a locked row binds to ADR-0001/HOST-STACK")

    def test_a_plan_naming_a_capability_outside_the_taxonomy_is_refused(self):
        refusal(R.refresh(R.registry(), "runtime", [{"capability": "runtime.nonexistent"}]),
                "the plan names 'runtime.nonexistent'")

    def test_a_dimension_the_taxonomy_does_not_carry_is_refused(self):
        refusal(R.refresh(R.registry(), "pricing", []), "no taxonomy dimension is named 'pricing'")

    def test_a_refresh_leaves_every_other_dimension_and_identity_alone(self):
        committed = R.registry()
        before = {(product, one["capability"]): one["state"] for product, one in committed.claims()}
        plan = [{"capability": "runtime.usage-visibility", "product": "t3code"}]
        self.assertEqual(R.refresh(committed, "runtime", plan), [])
        after = {(product, one["capability"]): one["state"] for product, one in R.registry().claims()}
        self.assertEqual(before, after)


class HonestEdits(unittest.TestCase):
    """The shapes that must stay green, so the refusals above are read as narrow rather than noisy."""

    def test_a_new_claim_with_authoritative_evidence_passes(self):
        def mutate(value):
            for entry in value["products"]:
                if entry["id"] == "orca":
                    entry["claims"].append({
                        "capability": "operations.git-integration",
                        "state": "VERIFIED_CURRENT",
                        "observation": "Worktrees are git worktrees, tracked in one place",
                        "evidence": [{"kind": "official_repo", "source": "https://github.com/stablyai/orca",
                                      "retrieved_at": "2026-09-20", "confidence": "high",
                                      "excerpt": "tracked in one place"}]})
        self.assertEqual(R.problems(commit(mutate), history()), [])

    def test_a_new_product_with_a_research_gap_passes(self):
        def mutate(value):
            value["universe"].append({"id": "intent", "name": "Intent", "class": "orchestration_tool"})
            value["products"].append({"id": "intent", "name": "Intent", "class": "orchestration_tool",
                                      "claims": [],
                                      "gap": "Retrieve the product's own documentation; not retrieved"})
        self.assertEqual(R.problems(commit(mutate), history()), [])

    def test_reordering_the_registry_changes_nothing(self):
        def mutate(value):
            value["products"].reverse()
            value["universe"].reverse()
        self.assertEqual(R.problems(commit(mutate), history()), [])

    def test_a_claim_whose_observation_is_reworded_stays_green(self):
        def mutate(value):
            claim(value, "orca", "workbench.terminal-floor")["observation"] = "Terminals, diffs and files ship in the product"
        self.assertEqual(R.problems(commit(mutate), history()), [])


if __name__ == "__main__":
    unittest.main()
