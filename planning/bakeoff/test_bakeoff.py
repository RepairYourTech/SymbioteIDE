"""The bake-off record's own rules, driven in both directions.

Every case except the first mutates a copy of the committed record — and, where the rule is
about a fact another owner holds, a copy of that owner's artifact — and asks the reader for
its refusals, so a rule that stopped refusing shows up as a case that no longer finds what it
must, and a record a maintainer honestly reordered or extended stays green.
"""
import copy
import json
import pathlib
import re
import shutil
import tempfile
import unittest

import bakeoff

RECORD = json.loads(bakeoff.RECORD.read_text())
BLOCK = RECORD["contract"]
# The files the record routes through, copied into a tree of its own so a case can break one
# without touching the live tree. The program is copied whole and then cut down to the entries
# these rules read, so the copy is the artifact's own shape rather than a fixture's.
ROUTED = (BLOCK["document"], BLOCK["ledger"], BLOCK["result"], BLOCK["record"],
          "planning/integrity/generated/registry.json")


def mutate(**changes):
    """The record with dotted paths replaced, so a case states what it changed."""
    row = copy.deepcopy(RECORD)
    for path, value in changes.items():
        parts = path.split(".")
        target = row
        for part in parts[:-1]:
            target = target[int(part)] if part.isdigit() else target[part]
        target[parts[-1]] = value
    return row


def refused(problems, needle):
    return any(needle in one for one in problems), problems


class Tree:
    """A copy of the routed artifacts in a directory of its own, and the record over it."""

    def __init__(self):
        self.where = tempfile.TemporaryDirectory()
        self.root = pathlib.Path(self.where.name)
        for name in ROUTED:
            target = self.root / name
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(bakeoff.ROOT / name, target)
        kept = {38, 426, 218, 434, 417}
        program = self.read("planning/integrity/generated/registry.json")
        program["entries"] = [one for one in program["entries"] if one.get("number") in kept]
        self.write("planning/integrity/generated/registry.json", program)

    def __enter__(self):
        return self

    def __exit__(self, *_thrown):
        self.where.cleanup()

    def sources(self) -> bakeoff.Sources:
        return bakeoff.Sources(self.root)

    def read(self, name: str) -> dict:
        return json.loads((self.root / name).read_text())

    def write(self, name: str, document: dict) -> None:
        (self.root / name).write_text(json.dumps(document))

    def write_text(self, name: str, text: str) -> None:
        (self.root / name).write_text(text)


class TheCommittedRecord(unittest.TestCase):
    def test_every_criterion_the_issue_states_is_routed_and_every_carrier_resolves(self):
        self.assertEqual(bakeoff.problems(RECORD, bakeoff.Sources()), [])
        self.assertEqual([row["n"] for row in RECORD["criteria"]], list(range(1, 9)))
        for row in RECORD["criteria"]:
            for carrier in row["carried_by"]:
                self.assertEqual(len(bakeoff.kind_of(carrier)), 1)

    def test_the_count_is_the_programs_own_and_not_this_records(self):
        entry = bakeoff.Sources().entry(38)
        self.assertIsInstance(entry.get("acceptance_items"), int)
        self.assertEqual(entry["acceptance_items"], len(RECORD["criteria"]),
                         "the program's own acceptance-item count and this record's rows "
                         "disagree, so one of them states a criterion the other does not")

    def test_a_row_carries_the_issues_number_and_what_carries_it_and_nothing_else(self):
        """The words are the issue's and this tree cannot hold them: the program pins the body's
        hash and records how many acceptance items it stated, and the only committed body capture
        reduces bodies — its own scope says it is not an exact-body provenance claim. Measured,
        #38's captured body carries no checkbox line. So a row states the number and
        the carriers, a field beside them is refused, and the words are read at the issue.
        """
        entry = bakeoff.Sources().entry(38)
        self.assertRegex(entry["body_sha256"], r"^[0-9a-f]{64}$")
        capture = bakeoff.Sources().read("planning/integrity/fixtures/audit-2026-09-08.json")
        self.assertIn("not an exact-body provenance claim", capture["scope"])
        body = next(one["body"] for one in capture["issues"] if one["number"] == 38)
        self.assertNotRegex(body, r"(?m)^\s*- \[[ xX]\]",
                            "the committed capture now carries #38's acceptance criteria, so the "
                            "words can be held against it rather than read at the issue")
        for row in RECORD["criteria"]:
            self.assertEqual(sorted(row), ["carried_by", "n"])


class CriterionRules(unittest.TestCase):
    def test_a_criterion_dropped_from_the_record_is_refused(self):
        row = mutate()
        del row["criteria"][0]
        found, _ = refused(bakeoff.criteria_problems(row, self.contract(), self.entry(),
                                                     self.sources()),
                           "is a claim the issue makes that no row answers")
        self.assertTrue(found)

    def test_a_criterion_added_to_the_record_is_refused(self):
        row = mutate()
        row["criteria"].append({"n": 9, "carried_by": [
            {"measurement": "installer_size_mib"}]})
        found, _ = refused(bakeoff.criteria_problems(row, self.contract(), self.entry(),
                                                     self.sources()),
                           "is a claim the issue makes that no row answers")
        self.assertTrue(found)

    def test_a_criterion_renumbered_or_stated_twice_is_refused(self):
        row = mutate(**{"criteria.3.n": 9})
        found, _ = refused(bakeoff.criteria_problems(row, self.contract(), self.entry(),
                                                     self.sources()),
                           "is a claim the issue makes that no row answers")
        self.assertTrue(found)
        row = mutate()
        row["criteria"].append(copy.deepcopy(row["criteria"][0]))
        found, _ = refused(bakeoff.criteria_problems(row, self.contract(), self.entry(),
                                                     self.sources()),
                           "is a claim the issue makes that no row answers")
        self.assertTrue(found)

    def test_a_field_beside_the_number_and_carriers_is_refused(self):
        row = mutate()
        row["criteria"][0]["criterion"] = "a claim restated here, which no rule reads"
        found, _ = refused(bakeoff.criteria_problems(row, self.contract(), self.entry(),
                                                     self.sources()),
                           "which no rule here reads")
        self.assertTrue(found)

    def test_a_criterion_carried_by_nothing_is_refused(self):
        found, _ = refused(bakeoff.criteria_problems(mutate(**{"criteria.0.carried_by": []}),
                                                     self.contract(), self.entry(),
                                                     self.sources()),
                           "is carried by nothing")
        self.assertTrue(found)

    def test_a_carrier_naming_two_kinds_at_once_is_refused(self):
        row = mutate(**{"criteria.5.carried_by": [
            {"obligation": "#38: a second desktop client controlling a headless Linux Host, "
                           "including controller handoff",
             "measurement": "installer_size_mib"}]})
        found, _ = refused(bakeoff.criteria_problems(row, self.contract(), self.entry(),
                                                     self.sources()),
                           "at once: a carrier is one obligation, one measurement or one issue")
        self.assertTrue(found)

    def test_one_carrier_named_twice_in_a_row_is_refused(self):
        row = mutate()
        row["criteria"][2]["carried_by"].append(
            copy.deepcopy(row["criteria"][2]["carried_by"][0]))
        found, _ = refused(bakeoff.criteria_problems(row, self.contract(), self.entry(),
                                                     self.sources()),
                           "twice, so one carrier stands where two facts are owed")
        self.assertTrue(found)

    def test_an_obligation_the_contract_does_not_declare_is_refused(self):
        found, _ = refused(bakeoff.criteria_problems(
            mutate(**{"criteria.0.carried_by": [{"obligation": "#38: something nobody declared"}]}),
            self.contract(), self.entry(), self.sources()),
            "which the contract does not declare")
        self.assertTrue(found)

    def test_a_measurement_the_contract_does_not_predeclare_is_refused(self):
        found, _ = refused(bakeoff.criteria_problems(
            mutate(**{"criteria.0.carried_by": [{"measurement": "wall_clock_seconds"}]}),
            self.contract(), self.entry(), self.sources()),
            "which the contract does not predeclare")
        self.assertTrue(found)

    def test_an_owner_the_program_does_not_carry_is_refused(self):
        found, _ = refused(bakeoff.criteria_problems(
            mutate(**{"criteria.5.carried_by": [{"elsewhere": {"issue": 999999, "why": "x"}}]}),
            self.contract(), self.entry(), self.sources()),
            "as an owner, and this program does not carry it")
        self.assertTrue(found)

    def test_a_routing_with_no_reason_is_refused(self):
        found, _ = refused(bakeoff.criteria_problems(
            mutate(**{"criteria.5.carried_by": [{"elsewhere": {"issue": 434}}]}),
            self.contract(), self.entry(), self.sources()),
            "routes part of itself away with no reason")
        self.assertTrue(found)

    def test_a_row_the_programs_count_cannot_place_is_refused(self):
        found, _ = refused(bakeoff.criteria_problems(mutate(**{"criteria.3.n": "4"}),
                                                     self.contract(), self.entry(),
                                                     self.sources()),
                           "one of those is not a number")
        self.assertTrue(found)

    def test_a_carrier_that_is_not_one_is_refused(self):
        found, _ = refused(bakeoff.criteria_problems(
            mutate(**{"criteria.0.carried_by": ["#38: an obligation as a bare string"]}),
            self.contract(), self.entry(), self.sources()),
            "which names no obligation, measurement or owner")
        self.assertTrue(found)

    def test_a_record_stating_no_criterion_at_all_is_refused(self):
        found, _ = refused(bakeoff.criteria_problems(mutate(**{"criteria": []}), self.contract(),
                                                     self.entry(), self.sources()),
                           "states no criterion")
        self.assertTrue(found)

    def contract(self):
        return bakeoff.contract_block(RECORD, self.sources())[0]

    def entry(self):
        return self.sources().entry(38)

    def sources(self):
        return bakeoff.Sources()


class ProgramRules(unittest.TestCase):
    def test_an_issue_the_program_does_not_carry_is_refused(self):
        with Tree() as tree:
            program = tree.read("planning/integrity/generated/registry.json")
            program["entries"] = [one for one in program["entries"] if one["number"] != 38]
            tree.write("planning/integrity/generated/registry.json", program)
            found, _ = refused(bakeoff.problems(RECORD, tree.sources()),
                               "the program carries no issue #38")
            self.assertTrue(found)

    def test_an_entry_with_no_acceptance_count_is_refused(self):
        with Tree() as tree:
            program = tree.read("planning/integrity/generated/registry.json")
            for one in program["entries"]:
                one.pop("acceptance_items", None)
            tree.write("planning/integrity/generated/registry.json", program)
            found, _ = refused(bakeoff.problems(RECORD, tree.sources()),
                               "records no acceptance-item count")
            self.assertTrue(found)

    def test_an_owner_the_program_drops_is_refused(self):
        with Tree() as tree:
            program = tree.read("planning/integrity/generated/registry.json")
            program["entries"] = [one for one in program["entries"] if one["number"] != 426]
            tree.write("planning/integrity/generated/registry.json", program)
            found, _ = refused(bakeoff.problems(RECORD, tree.sources()),
                               "names #426 as an owner")
            self.assertTrue(found)


class JoinRules(unittest.TestCase):
    """The contract, the ledger record that binds it to this issue and the dossier must agree."""

    def test_a_routed_path_the_tree_does_not_carry_is_refused(self):
        with Tree() as tree:
            (tree.root / "docs/proofs/linux-shell.md").unlink()
            found, _ = refused(bakeoff.problems(RECORD, tree.sources()),
                               "as the record and this tree does not carry it")
            self.assertTrue(found)

    def test_a_shell_record_naming_no_measurement_the_contract_predeclares_is_refused(self):
        with Tree() as tree:
            where = tree.root / BLOCK["record"]
            where.write_text(where.read_text().replace("workload_process_tree_pss_mib", "a figure"))
            found, _ = refused(bakeoff.problems(RECORD, tree.sources()),
                               "names no measurement 'workload_process_tree_pss_mib'")
            self.assertTrue(found)

    def test_a_shell_record_naming_every_figure_the_contract_predeclares_stays_green(self):
        with Tree() as tree:
            where = tree.root / BLOCK["record"]
            where.write_text(where.read_text() + "\n\nBoth figures above are named once each.\n")
            self.assertEqual(bakeoff.problems(RECORD, tree.sources()), [])

    def test_a_contract_document_that_holds_no_such_contract_is_refused(self):
        with Tree() as tree:
            document = tree.read(BLOCK["document"])
            document["contracts"][0]["id"] = "#38/something-else"
            tree.write(BLOCK["document"], document)
            found, _ = refused(bakeoff.problems(RECORD, tree.sources()),
                               "holds no contract '#38/desktop-shell-representative-workload'")
            self.assertTrue(found)

    def test_a_contract_settling_another_decision_is_refused(self):
        with Tree() as tree:
            document = tree.read(BLOCK["document"])
            document["contracts"][0]["decision"] = "ADR-0001/GRAPH-STORAGE"
            tree.write(BLOCK["document"], document)
            found, _ = refused(bakeoff.problems(RECORD, tree.sources()),
                               "and the record names 'ADR-0001/DESKTOP-SHELL'")
            self.assertTrue(found)

    def test_a_ledger_without_this_decision_is_refused(self):
        with Tree() as tree:
            ledger = tree.read(BLOCK["ledger"])
            ledger["decisions"] = [one for one in ledger["decisions"]
                                   if one["draft"]["id"] != BLOCK["decision"]]
            tree.write(BLOCK["ledger"], ledger)
            found, _ = refused(bakeoff.problems(RECORD, tree.sources()),
                               "the ledger holds no decision 'ADR-0001/DESKTOP-SHELL'")
            self.assertTrue(found)

    def test_a_choice_blocked_by_another_issue_is_refused(self):
        with Tree() as tree:
            ledger = tree.read(BLOCK["ledger"])
            for one in ledger["decisions"]:
                if one["draft"]["id"] == BLOCK["decision"]:
                    one["blocking_issue"] = 233
            tree.write(BLOCK["ledger"], ledger)
            found, _ = refused(bakeoff.problems(RECORD, tree.sources()),
                               "is blocked by #233, not by the issue this record states")
            self.assertTrue(found)

    def test_a_choice_proved_by_another_contract_is_refused(self):
        with Tree() as tree:
            ledger = tree.read(BLOCK["ledger"])
            for one in ledger["decisions"]:
                if one["draft"]["id"] == BLOCK["decision"]:
                    one["proof_contract"] = "#38/another-workload"
            tree.write(BLOCK["ledger"], ledger)
            found, _ = refused(bakeoff.problems(RECORD, tree.sources()),
                               "is proved by '#38/another-workload'")
            self.assertTrue(found)

    def test_a_dossier_measured_against_another_contract_is_refused(self):
        with Tree() as tree:
            dossier = tree.read(BLOCK["result"])
            dossier["contract"] = "#38/another-workload"
            tree.write(BLOCK["result"], dossier)
            found, _ = refused(bakeoff.problems(RECORD, tree.sources()),
                               "and this record names '#38/desktop-shell-representative-workload'")
            self.assertTrue(found)

    def test_a_dossier_recording_no_run_is_refused(self):
        with Tree() as tree:
            dossier = tree.read(BLOCK["result"])
            dossier["runs"] = []
            tree.write(BLOCK["result"], dossier)
            found, _ = refused(bakeoff.problems(RECORD, tree.sources()),
                               "records no run")
            self.assertTrue(found)


class Readings(unittest.TestCase):
    """What stands behind the routings is derived from the dossier, not stated in the record."""

    def artifact(self) -> dict:
        return bakeoff.Sources().read(BLOCK["result"])

    def test_the_readings_are_counted_from_the_record_its_contract_and_its_runs(self):
        contract, found = bakeoff.contract_block(RECORD, bakeoff.Sources())
        self.assertEqual(found, [])
        counts = bakeoff.readings(RECORD, contract, self.artifact())
        # Counted from the sources rather than declared here: a carrier added to the record is a
        # figure the README has to state (the case below), not a second list this one keeps.
        self.assertEqual(counts["criteria"], len(RECORD["criteria"]))
        self.assertEqual(counts["carriers"],
                         sum(len(row["carried_by"]) for row in RECORD["criteria"]))
        self.assertEqual(counts["kinds"]["obligation"] + counts["kinds"]["measurement"]
                         + counts["kinds"]["elsewhere"], counts["carriers"])
        self.assertEqual(counts["obligations"], len(contract["workload"]))
        self.assertEqual(counts["measurements"], len(contract["measurements"]))
        self.assertEqual(counts["runs"], len(self.artifact()["runs"]))
        self.assertEqual(counts["platforms"], len(contract["applicable_platforms"]))
        self.assertEqual(counts["obligations_named"],
                         len({one["obligation"] for row in RECORD["criteria"]
                              for one in row["carried_by"] if "obligation" in one}))
        self.assertLessEqual(counts["obligations_named"], counts["obligations"])
        self.assertLessEqual(counts["measurements_named"], counts["measurements"])

    def test_a_carrier_no_run_shows_is_reported_as_such_rather_than_read_as_met(self):
        artifact = self.artifact()
        rows = {row["n"]: row for row in RECORD["criteria"]}
        stated = " ".join(bakeoff.measured_for(rows[2], artifact))
        self.assertIn("#38: three active terminal tabs with bounded scrollback in real PTYs': "
                      "attested by a committed run", stated)
        self.assertIn("#38: four concurrent agent streams, dispatched and supervised at once': "
                      "attested by no committed run", stated)
        fourth = " ".join(bakeoff.measured_for(rows[4], artifact))
        self.assertIn("observed by no committed run", fourth)
        self.assertIn("owner #434", " ".join(bakeoff.measured_for(rows[6], artifact)))

    def test_the_dossier_and_the_contract_still_agree_on_every_platform(self):
        contract, _found = bakeoff.contract_block(RECORD, bakeoff.Sources())
        artifact = self.artifact()
        measured = {run["platform"] for run in artifact["runs"]}
        untested = set(artifact["untested_platforms"])
        self.assertEqual(measured | untested, set(contract["applicable_platforms"]),
                         "the dossier no longer accounts for every platform the contract "
                         "applies to, so the readings above would count a row that is neither")


class HonestEdits(unittest.TestCase):
    def test_reordered_criteria_rows_and_carriers_stay_green(self):
        row = mutate()
        row["criteria"] = list(reversed(row["criteria"]))
        for one in row["criteria"]:
            one["carried_by"] = list(reversed(one["carried_by"]))
        self.assertEqual(bakeoff.problems(row, bakeoff.Sources()), [])

    def test_a_reworded_reason_or_an_added_carrier_stays_green(self):
        row = mutate(**{"criteria.5.carried_by.0.elsewhere.why": "the same routing, said "
                                                                "another way"})
        self.assertEqual(bakeoff.problems(row, bakeoff.Sources()), [])
        row = mutate()
        row["criteria"][0]["carried_by"].append({"measurement": "clean_locked_build_seconds"})
        self.assertEqual(bakeoff.problems(row, bakeoff.Sources()), [])

    def test_a_record_routed_to_a_second_owner_stays_green(self):
        row = mutate()
        row["criteria"][3]["carried_by"].append({"elsewhere": {
            "issue": 372, "why": "crash-safe canonical recovery is Q06's beyond this fixture"}})
        self.assertEqual(bakeoff.problems(row, bakeoff.Sources()), [])


class ReadmeFigures(unittest.TestCase):
    """Every figure this record's README writes is held to the record, its contract and the
    dossier, so a count that nothing carries cannot go stale silently."""

    def test_the_readme_states_the_figures_this_record_carries(self):
        readme = re.sub(r"\s+", " ", (bakeoff.HERE / "README.md").read_text())
        contract, _found = bakeoff.contract_block(RECORD, bakeoff.Sources())
        counts = bakeoff.readings(RECORD, contract,
                                  bakeoff.Sources().read(BLOCK["result"]))
        stated = re.search(r"As committed: \*\*(\d+)\*\* criteria over \*\*(\d+)\*\* carriers: "
                           r"(\d+) obligation carriers, (\d+) measurement carriers and "
                           r"(\d+) routings to (\d+) owner issues; (\d+) of the contract's "
                           r"(\d+) obligations and (\d+) of its (\d+) measurements named; "
                           r"(\d+) of (\d+) platforms measured, (\d+) declared untested, over "
                           r"(\d+) runs",
                           readme)
        self.assertIsNotNone(stated, "the README no longer states the record's figures")
        self.assertEqual([int(one) for one in stated.groups()], [
            counts["criteria"], counts["carriers"], counts["kinds"]["obligation"],
            counts["kinds"]["measurement"], counts["kinds"]["elsewhere"], counts["routed"],
            counts["obligations_named"], counts["obligations"], counts["measurements_named"],
            counts["measurements"], counts["platforms_measured"], counts["platforms"],
            counts["platforms_untested"], counts["runs"]])
        # And every figure written again as `N <noun>` elsewhere in the file is the same one.
        held = {"criteria": counts["criteria"], "carriers": counts["carriers"],
                "obligations": counts["obligations"], "measurements": counts["measurements"],
                "runs": counts["runs"], "platforms": counts["platforms"]}
        for noun, expected in held.items():
            written = re.findall(rf"(\d+)\s+{noun}\b", readme.replace("**", ""))
            self.assertTrue(written, f"the README no longer states how many {noun} there are")
            self.assertEqual([int(one) for one in written], [expected] * len(written),
                             f"the README states {written} {noun} where the record carries "
                             f"{expected}")
        # And the limits sentence's dossier figures, which are read from the runs the same way.
        partial = re.search(r"Of the carriers this record names, the committed runs attest "
                            r"(\d+) of the (\d+) obligations and observe (\d+) of the (\d+) "
                            r"measurements the contract predeclares", readme)
        self.assertIsNotNone(partial, "the README no longer states what the runs back")
        self.assertEqual([int(one) for one in partial.groups()], [
            counts["obligations_attested"], counts["obligations"],
            counts["measurements_observed"], counts["measurements"]],
            "the README states what the runs back where the dossier reads another figure")
        # And the captured body's size, which the limits sentence states about the one artifact this
        # record reads rather than about the issue: the capture owns the body, so the capture's own
        # measurement is what the sentence's figure has to equal.
        capture = bakeoff.Sources().read("planning/integrity/fixtures/audit-2026-09-08.json")
        body = next(one["body"] for one in capture["issues"] if one["number"] == 38)
        size = re.search(r"#38's captured body is (\d+) bytes", readme)
        self.assertIsNotNone(size, "the README no longer states the captured body's size")
        self.assertEqual(int(size.group(1)), len(body.encode()),
                         "the README states the captured body's size in bytes where the capture "
                         "measures another")


if __name__ == "__main__":
    unittest.main()
