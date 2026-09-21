"""The release record's own rules, driven in both directions.

Each case mutates the committed record in memory (or a small tree for the register and
concern readings) and asks the reader for its refusals, so a rule that stopped refusing
shows up as a case that no longer finds what it must — and a record a maintainer
honestly reworded, reordered or extended stays green.
"""
import copy
import json
import pathlib
import re
import tempfile
import unittest

import release

RECORD = json.loads(release.RECORD.read_text())
TREE = release.Tree(release.ROOT)
PROGRAM = release.program_issues()


def refused(problems, needle):
    return any(needle in one for one in problems), problems


def mutate(**changes):
    row = copy.deepcopy(RECORD)
    for path, value in changes.items():
        parts = path.split(".")
        target = row
        for part in parts[:-1]:
            target = target[int(part)] if part.isdigit() else target[part]
        target[parts[-1]] = value
    return row


class MetricRules(unittest.TestCase):
    def test_every_metric_the_issue_lists_is_defined_once_and_sourced(self):
        problems = release.metric_problems(RECORD, TREE, PROGRAM)
        self.assertEqual(problems, [])
        self.assertEqual(sorted(row["id"] for row in RECORD["metrics"]), sorted(release.METRICS))
        for row in RECORD["metrics"]:
            self.assertIn(row["source"], release.SOURCES)

    def test_a_metric_dropped_from_the_record_is_refused(self):
        row = mutate()
        del row["metrics"][0]
        found, _ = refused(release.metric_problems(row, TREE, PROGRAM),
                           "a metric dropped or added here")
        self.assertTrue(found)

    def test_a_metric_added_to_the_record_is_refused(self):
        row = mutate()
        row["metrics"].append({"id": "velocity", "definition": "x", "unit": "y",
                               "source": "delegated", "owner": 456, "reason": "z"})
        found, _ = refused(release.metric_problems(row, TREE, PROGRAM), "a metric dropped or added")
        self.assertTrue(found)

    def test_a_metric_with_no_definition_or_unit_is_refused(self):
        found, _ = refused(release.metric_problems(mutate(**{"metrics.0.definition": ""}), TREE,
                                                   PROGRAM), "states no definition")
        self.assertTrue(found)
        found, _ = refused(release.metric_problems(mutate(**{"metrics.0.unit": ""}), TREE, PROGRAM),
                           "states no definition")
        self.assertTrue(found)

    def test_a_metric_with_no_source_is_refused(self):
        found, _ = refused(release.metric_problems(mutate(**{"metrics.0.source": "assumed"}),
                                                   TREE, PROGRAM), "where this reader states")
        self.assertTrue(found)

    def test_a_measured_metric_with_no_evidence_path_is_refused(self):
        found, _ = refused(release.metric_problems(
            mutate(**{"metrics.4.evidence.path": "planning/release/nowhere.py"}), TREE, PROGRAM),
            "this tree does not carry it")
        self.assertTrue(found)

    def test_a_measured_metric_whose_case_no_file_declares_is_refused(self):
        found, _ = refused(release.metric_problems(
            mutate(**{"metrics.4.evidence.case": "nothing_declares_this"}), TREE, PROGRAM),
            "which no file in this tree declares")
        self.assertTrue(found)

    def test_a_measured_metric_whose_case_is_declared_elsewhere_is_refused(self):
        found, _ = refused(release.metric_problems(
            mutate(**{"metrics.7.evidence.case": "the_committed_history_recovers_what_the_ledger_"
                                                 "publishes"}), TREE, PROGRAM),
            "and the files declaring it are")
        self.assertTrue(found)

    def test_a_measured_metric_whose_artifact_is_missing_is_refused(self):
        found, _ = refused(release.metric_problems(
            mutate(**{"metrics.7.artifact": ".github/workflows/gone.yml"}), TREE, PROGRAM),
            "as what it measures")
        self.assertTrue(found)

    def test_a_delegated_metric_whose_owner_the_program_lacks_is_refused(self):
        found, _ = refused(release.metric_problems(mutate(**{"metrics.0.owner": 999999}), TREE,
                                                   PROGRAM), "which this program does not carry")
        self.assertTrue(found)

    def test_a_delegated_metric_with_no_reason_is_refused(self):
        found, _ = refused(release.metric_problems(mutate(**{"metrics.0.reason": ""}), TREE,
                                                   PROGRAM), "delegated with no reason")
        self.assertTrue(found)

    def test_a_narrowing_with_no_scope_or_a_missing_owner_is_refused(self):
        found, _ = refused(release.metric_problems(
            mutate(**{"metrics.4.narrowing.scope": ""}), TREE, PROGRAM), "narrowed with no scope")
        self.assertTrue(found)
        found, _ = refused(release.metric_problems(
            mutate(**{"metrics.4.narrowing.owners": [999999]}), TREE, PROGRAM),
            "is narrowed by #999999")
        self.assertTrue(found)


class GateRules(unittest.TestCase):
    def test_every_release_the_issue_names_is_a_gate(self):
        self.assertEqual([row["id"] for row in RECORD["gates"]], list(release.GATES))
        self.assertEqual(release.gate_problems(RECORD), [])

    def test_a_gate_requiring_an_unknown_metric_is_refused(self):
        found, _ = refused(release.gate_problems(mutate(**{"gates.0.requires": ["delivery",
                                                                               "recovery", "velocity",
                                                                               "golden-path"]})),
                           "which is not a metric this record defines")
        self.assertTrue(found)

    def test_a_gate_requiring_nothing_is_refused(self):
        found, _ = refused(release.gate_problems(mutate(**{"gates.0.requires": []})),
                           "requires nothing")
        self.assertTrue(found)

    def test_a_gate_without_the_golden_path_is_refused(self):
        found, _ = refused(release.gate_problems(mutate(**{"gates.0.requires": ["delivery",
                                                                               "recovery"]})),
                           "does not require 'golden-path'")
        self.assertTrue(found)

    def test_a_gate_naming_an_unstated_non_goal_is_refused(self):
        found, _ = refused(release.gate_problems(mutate(**{"gates.0.non_goals": ["preview-no-mobile",
                                                                                 "preview-no-cloud"]})),
                           "which the record does not state")
        self.assertTrue(found)

    def test_a_gate_tracking_an_unknown_register_is_refused(self):
        found, _ = refused(release.gate_problems(mutate(**{"gates.0.tracks": ["constitution",
                                                                              "parity", "safety"]})),
                           "which is not a register this record states")
        self.assertTrue(found)

    def test_a_gate_tracking_no_register_is_refused(self):
        found, _ = refused(release.gate_problems(mutate(**{"gates.0.tracks": []})),
                           "tracks no register")
        self.assertTrue(found)

    def test_a_gate_with_no_non_goal_is_refused(self):
        found, _ = refused(release.gate_problems(mutate(**{"gates.0.non_goals": []})),
                           "states no non-goal")
        self.assertTrue(found)

    def test_a_metric_required_by_no_gate_is_refused(self):
        row = mutate()
        for gate in row["gates"]:
            gate["requires"] = [one for one in gate["requires"] if one != "mobile-control"]
        found, _ = refused(release.gate_problems(row), "required by no gate")
        self.assertTrue(found)

    def test_a_register_tracked_by_no_gate_is_refused(self):
        row = mutate()
        for gate in row["gates"]:
            gate["tracks"] = ["constitution"]
        found, _ = refused(release.gate_problems(row), "tracked by no gate")
        self.assertTrue(found)


class NonGoalRules(unittest.TestCase):
    def test_every_non_goal_names_its_release_what_it_does_not_support_and_why(self):
        self.assertEqual(release.non_goal_problems(RECORD, PROGRAM), [])

    def test_a_non_goal_with_an_unknown_release_or_no_reason_is_refused(self):
        found, _ = refused(release.non_goal_problems(mutate(**{"non_goals.0.release": "preview"}),
                                                     PROGRAM), "is not one of")
        self.assertTrue(found)
        found, _ = refused(release.non_goal_problems(mutate(**{"non_goals.0.why": ""}), PROGRAM),
                           "states no why")
        self.assertTrue(found)

    def test_a_non_goal_with_an_owner_outside_the_program_is_refused(self):
        found, _ = refused(release.non_goal_problems(mutate(**{"non_goals.0.owner": 999999}),
                                                     PROGRAM), "which this program does not carry")
        self.assertTrue(found)

    def test_a_non_goal_stated_twice_is_refused(self):
        row = mutate()
        row["non_goals"].append(copy.deepcopy(row["non_goals"][0]))
        found, _ = refused(release.non_goal_problems(row, PROGRAM), "is stated twice")
        self.assertTrue(found)


class GoldenPathRules(unittest.TestCase):
    def test_the_golden_path_names_a_committed_artifact_and_the_case_that_reads_it(self):
        self.assertEqual(release.golden_problems(RECORD, TREE, PROGRAM), [])

    def test_a_golden_path_with_a_missing_artifact_or_case_is_refused(self):
        found, _ = refused(release.golden_problems(
            mutate(**{"golden_path.artifact": "docs/proofs/results/gone.json"}), TREE, PROGRAM),
            "this tree does not carry it")
        self.assertTrue(found)
        found, _ = refused(release.golden_problems(
            mutate(**{"golden_path.evidence.case": "nothing_declares_this"}), TREE, PROGRAM),
            "which no file in this tree declares")
        self.assertTrue(found)

    def test_a_golden_path_with_no_owner_in_the_program_is_refused(self):
        found, _ = refused(release.golden_problems(mutate(**{"golden_path.owner": 999999}), TREE,
                                                   PROGRAM), "which this program does not carry")
        self.assertTrue(found)


class RegisterRules(unittest.TestCase):
    def test_each_register_reads_its_own_committed_artifact(self):
        self.assertEqual(release.register_problems(RECORD, TREE), [])
        for row in RECORD["registers"]:
            document = json.loads((release.ROOT / row["source"]).read_text())
            self.assertTrue(document[row["carries"]])

    def test_a_register_whose_source_is_missing_is_refused(self):
        found, _ = refused(release.register_problems(
            mutate(**{"registers.0.source": "docs/contracts/gone.json"}), TREE),
            "this tree does not carry it")
        self.assertTrue(found)

    def test_a_register_that_carries_nothing_is_refused(self):
        row = mutate(**{"registers.1.source": "planning/release/release.json",
                        "registers.1.carries": "rows"})
        found, _ = refused(release.register_problems(row, TREE), "which holds")
        self.assertTrue(found)

    def test_a_register_whose_holding_case_no_file_declares_is_refused(self):
        found, _ = refused(release.register_problems(
            mutate(**{"registers.1.holds.case": "nothing_declares_this"}), TREE),
            "which no file in this tree declares")
        self.assertTrue(found)


class ConcernRules(unittest.TestCase):
    def test_the_concerns_reviewed_are_the_policy_records_marks(self):
        self.assertEqual(release.concern_problems(RECORD, PROGRAM), [])
        marks = {row["concern"] for row in json.loads(release.POLICY.read_text())["applicability"]}
        self.assertEqual(marks, set(release.CONCERNS))

    def test_a_concern_reviewed_but_not_marked_is_refused(self):
        found, _ = refused(release.concern_problems(
            mutate(**{"concerns.reviewed": ["security", "privacy", "accessibility",
                                            "performance", "cross_platform", "cost"]}), PROGRAM),
            "is applicability assumed rather than checked")
        self.assertTrue(found)

    def test_a_concern_marked_that_is_not_reviewed_is_refused(self):
        found, _ = refused(release.concern_problems(
            mutate(**{"concerns.reviewed": ["security", "privacy"]}), PROGRAM),
            "is applicability assumed rather than checked")
        self.assertTrue(found)

    def test_a_mark_outside_the_three_is_refused(self):
        with tempfile.TemporaryDirectory() as where:
            root = pathlib.Path(where)
            (root / "policy.json").write_text(json.dumps(
                {"applicability": [{"concern": "security", "mark": "assumed"}]}))
            original, release.ROOT = release.ROOT, root
            try:
                found, _ = refused(release.concern_problems(
                    {"concerns": {"source": "policy.json", "reviewed": ["security"]}}, PROGRAM),
                    "where the issue states")
            finally:
                release.ROOT = original
            self.assertTrue(found)

    def test_a_tracked_concern_owned_by_an_issue_the_program_lacks_is_refused(self):
        marks = json.loads(release.POLICY.read_text())["applicability"]
        marks = [dict(one, owners=[999999]) if one["concern"] == "performance" else one
                 for one in marks]
        with tempfile.TemporaryDirectory() as where:
            root = pathlib.Path(where)
            (root / "policy.json").write_text(json.dumps({"applicability": marks}))
            original, release.ROOT = release.ROOT, root
            try:
                found, _ = refused(release.concern_problems(
                    {"concerns": {"source": "policy.json",
                                  "reviewed": [one["concern"] for one in marks]}}, PROGRAM),
                    "which this program does not carry")
            finally:
                release.ROOT = original
            self.assertTrue(found)


class HonestEdits(unittest.TestCase):
    def test_reordered_metrics_gates_and_requirements_stay_green(self):
        row = mutate()
        row["metrics"] = list(reversed(row["metrics"]))
        for gate in row["gates"]:
            gate["requires"] = list(reversed(gate["requires"]))
            gate["tracks"] = list(reversed(gate["tracks"]))
        self.assertEqual(release.problems(row, TREE, PROGRAM), [])

    def test_a_reworded_definition_or_reason_stays_green(self):
        row = mutate(**{"metrics.0.definition": "the same metric, said another way",
                        "metrics.1.reason": "the conformance laboratory owns this evidence path"})
        self.assertEqual(release.problems(row, TREE, PROGRAM), [])

    def test_a_new_non_goal_and_a_new_requirement_stay_green(self):
        row = mutate()
        row["non_goals"].append({"id": "beta-no-cloud-sync", "release": "beta",
                                 "does_not_support": "cloud sync of projects",
                                 "why": "the sync transport is not built", "owner": 414})
        row["gates"][2]["non_goals"].append("beta-no-cloud-sync")
        self.assertEqual(release.problems(row, TREE, PROGRAM), [])

    def test_a_second_measured_metric_stays_green(self):
        row = mutate()
        row["metrics"][0] = {"id": "quality", "definition": "verified output quality",
                             "unit": "per-release rates",
                             "source": "measured",
                             "evidence": {"path": "crates/symbiote-architecture/tests/audit.rs",
                                          "case": "the_committed_history_recovers_what_the_ledger_"
                                                  "publishes"}}
        self.assertEqual(release.problems(row, TREE, PROGRAM), [])


class ReadmeFigures(unittest.TestCase):
    def test_the_readme_states_the_numbers_this_record_carries(self):
        readme = re.sub(r"\s+", " ", (release.HERE / "README.md").read_text())
        stated = re.search(r"As committed: \*\*(\d+)\*\* metrics \((\d+) measured, (\d+) delegated\), "
                           r"\*\*(\d+)\*\* gates, \*\*(\d+)\*\* non-goals, \*\*(\d+)\*\* registers "
                           r"\((\d+) invariants, (\d+) parity rows\), \*\*(\d+)\*\* concerns", readme)
        self.assertIsNotNone(stated, "the README no longer states the record's figures")
        count, sources, goals, registers = release.readings(RECORD)
        self.assertEqual([int(one) for one in stated.groups()],
                         [count, sources.get("measured", 0), sources.get("delegated", 0),
                          len(RECORD["gates"]), goals, len(RECORD["registers"]),
                          registers.get("constitution", 0), registers.get("parity", 0),
                          len(RECORD["concerns"]["reviewed"])])
        # And every other place this file writes one of those figures in the `N <noun>` form is the
        # same figure: a number written twice is a number one of the copies can leave stale while
        # this case stays green, which is how the registers paragraph carried an unread 18 until a
        # pass removed it. The limit, stated: a figure written another way — a bare `(18)` beside a
        # path — is not read by this, so the descriptive sentences name the files rather than
        # repeat a count.
        held = {
            "metrics": count,
            "measured": sources.get("measured", 0),
            "delegated": sources.get("delegated", 0),
            "gates": len(RECORD["gates"]),
            "non-goals": goals,
            "registers": len(RECORD["registers"]),
            "invariants": registers.get("constitution", 0),
            "parity rows": registers.get("parity", 0),
            "concerns": len(RECORD["concerns"]["reviewed"]),
        }
        for noun, expected in held.items():
            written = re.findall(rf"(\d+)\s+{noun}\b", readme.replace("**", ""))
            self.assertTrue(written, f"the README no longer states how many {noun} the record carries")
            self.assertEqual([int(one) for one in written], [expected] * len(written),
                             f"the README states {written} {noun} where the record carries {expected}")


if __name__ == "__main__":
    unittest.main()
