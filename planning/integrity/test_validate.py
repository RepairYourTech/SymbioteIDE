"""Run: python3 planning/integrity/test_validate.py [--snapshot FILE]."""

import copy
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

from validate import IntegrityError, render_index, validate_snapshot


def issue(number, key, body, labels=None):
    return dict(number=number, title=key, state="open", state_reason=None,
                updated_at="2026-09-07T12:00:00Z", labels=labels or [{"name": "planning:canonical"}],
                body=f"<!-- symbiote-plan-key: {key} -->\n<!-- symbiote-plan-revision: approved-v1 -->\n" + body)


def fixture():
    return [issue(1, "ROADMAP", "2 canonical atomic issues across 1 epics\n## Canonical epics\n- #2 (2 children)\n"),
            issue(2, "E00", "## Child issues\n- [ ] #3\n- [ ] #4\n"),
            issue(3, "A01", "**Parent epic:** #2 · **Wave:** 0\n## Dependencies\n- None\n\n## Related work\n#4 #999\n"),
            issue(4, "A02", "Parent epic: #2 · Wave: 0\n## Dependencies\n- #3\n")]


class IntegrityTests(unittest.TestCase):
    def setUp(self):
        self.snapshot = fixture()

    def fails(self, text):
        with self.assertRaisesRegex(IntegrityError, text):
            validate_snapshot(self.snapshot)

    def test_pinned_real_audit_structure(self):
        root = Path(__file__).parent
        fixture = json.loads((root / "fixtures/audit-2026-09-08.json").read_text())
        expected = json.loads((root / "generated/registry.json").read_text())
        registry = validate_snapshot(fixture["issues"])
        self.assertEqual(fixture["source_snapshot_sha256"], expected["snapshot_sha256"])
        for field in ("topological_order", "waves"):
            self.assertEqual(registry[field], expected[field])
        # The reduced fixture keeps the structure but not every body marker: the four
        # issues that carry a key and no planning label (#15, #19, #26, #443) lose the
        # marker that separates `unclassified` from `unkeyed`, so the fixture counts
        # them as unkeyed while the committed registry holds them as entries. Every
        # class the fixture can attest is compared, the two it cannot are held to
        # accounting for the same number of issues, and the committed registry is held
        # to its own entries in test_coverage_ledger.py.
        attested = {k: v for k, v in registry["counts"].items() if k not in ("unclassified", "unkeyed")}
        self.assertEqual({k: expected["counts"][k] for k in attested}, attested)
        self.assertEqual(registry["counts"]["unclassified"] + registry["counts"]["unkeyed"],
                         expected["counts"]["unclassified"] + expected["counts"]["unkeyed"])
        self.assertEqual(expected["counts"]["unclassified"],
                         sum(1 for entry in expected["entries"] if entry["kind"] == "unclassified"),
                         "the committed registry declares the unclassified entries it holds")
        by_number = {r["number"]: r for r in registry["entries"]}
        originals = {r["number"]: r for r in fixture["issues"]}
        for entry in expected["entries"]:
            self.assertEqual(originals[entry["number"]]["source_body_sha256"], entry["body_sha256"])
            if entry["kind"] == "unclassified":
                continue
            for field in ("kind", "key", "revision", "epic", "wave", "dependencies", "children", "canonical_issue"):
                self.assertEqual(by_number[entry["number"]].get(field), entry.get(field))

    def test_registry_records_acceptance_items_as_a_count_and_never_as_text(self):
        """An entry pins the body it was generated from and how many acceptance items
        that body states — never the item text. Registrations that live in an issue's
        own thread are therefore invisible to this artifact and to every reader of it,
        which is what the architecture contract's bar registration relies on: #173's
        entry holds the count and the body hash, so the two items registered on that
        issue cannot be found here, and a later pass has to read the issue."""
        expected = json.loads((Path(__file__).parent / "generated/registry.json").read_text())
        # Fields whose value is a collection by design rather than item text.
        structural = {"labels", "dependencies", "children", "program_entry"}
        for entry in expected["entries"]:
            self.assertIsInstance(entry["body_sha256"], str)
            self.assertEqual(len(entry["body_sha256"]), 64, f"#{entry['number']} pins its body")
            self.assertTrue(entry["revision"] is None or isinstance(entry["revision"], str))
            if "acceptance_items" in entry:
                self.assertIsInstance(entry["acceptance_items"], int)
            for field, value in entry.items():
                if field in structural:
                    continue
                self.assertNotIsInstance(
                    value, (list, dict),
                    f"#{entry['number']} carries item text in {field}")
        bar = next(entry for entry in expected["entries"] if entry["number"] == 173)
        self.assertEqual(bar["key"], "A04")
        self.assertIsInstance(bar["acceptance_items"], int, "the bar's entry records a count")
        self.assertIsInstance(bar["body_sha256"], str)
        self.assertIsInstance(bar["revision"], str)

    def test_every_snapshot_issue_belongs_to_one_counted_class(self):
        """No issue may disappear into a class the counts do not name."""
        self.snapshot.append(
            dict(number=5, title="history", state="closed", state_reason="completed",
                 updated_at="2026-09-07T12:00:00Z", labels=[], body="an issue no roadmap key names\n")
        )
        counts = validate_snapshot(self.snapshot)["counts"]
        classes = ("masters", "tasks", "epics", "retired", "unclassified",
                   "references", "program_entries", "unkeyed")
        self.assertEqual(set(counts), set(classes) | {"snapshot_issues"}, "the counts name every class")
        self.assertEqual(sum(counts[field] for field in classes), counts["snapshot_issues"])
        self.assertEqual(counts["unkeyed"], 1, "the issue no key names is counted, not dropped")

    def test_topological_order_is_deterministic_and_not_readiness(self):
        registry = validate_snapshot(self.snapshot)
        self.assertEqual(registry, validate_snapshot(list(reversed(self.snapshot))))
        self.assertEqual(registry["topological_order"], [3, 4])
        self.assertIn("does not establish readiness", render_index(registry))
        self.assertEqual(registry["counts"]["tasks"], 2)

    def test_exact_body_hash_and_rest_provenance(self):
        registry = validate_snapshot(self.snapshot)
        record = next(r for r in registry["entries"] if r["number"] == 3)
        self.assertEqual(record["body_sha256"], hashlib.sha256(self.snapshot[2]["body"].encode()).hexdigest())
        self.assertEqual(record["updated_at"], self.snapshot[2]["updated_at"])
        self.assertEqual(record["state"], "open")

    def test_duplicate_key(self):
        self.snapshot[3]["body"] = self.snapshot[3]["body"].replace("A02", "A01")
        self.fails("duplicate active key")

    def test_dangling_dependency(self):
        self.snapshot[3]["body"] = self.snapshot[3]["body"].replace("- #3", "- #999")
        self.fails("dangling dependency")

    def test_missing_or_renamed_dependency_heading(self):
        self.snapshot[3]["body"] = self.snapshot[3]["body"].replace("## Dependencies", "## Prerequisites")
        self.fails("missing explicit Dependencies")

    def test_empty_dependency_section(self):
        self.snapshot[2]["body"] = self.snapshot[2]["body"].replace("- None", "")
        self.fails("empty Dependencies")

    def test_dependency_url(self):
        self.snapshot[3]["body"] = self.snapshot[3]["body"].replace("- #3", "- https://github.com/RepairYourTech/SymbioteIDE/issues/3")
        self.assertEqual(validate_snapshot(self.snapshot)["topological_order"], [3, 4])
        self.snapshot[3]["body"] = self.snapshot[3]["body"].replace("/3", "/999")
        self.fails("dangling dependency")

    def test_unparsed_dependency_syntax(self):
        self.snapshot[3]["body"] = self.snapshot[3]["body"].replace("- #3", "Depends on task three")
        self.fails("unparsed dependency syntax")

    def test_canonical_details_do_not_hide_dependencies(self):
        self.snapshot[3]["body"] = self.snapshot[3]["body"].replace("## Dependencies", "<details>Explanation</details>\n## Dependencies")
        self.assertEqual(validate_snapshot(self.snapshot)["topological_order"], [3, 4])

    def test_cycle(self):
        self.snapshot[2]["body"] = self.snapshot[2]["body"].replace("- None", "- #4")
        self.fails("dependency cycle")

    def test_later_wave(self):
        self.snapshot[2]["body"] = self.snapshot[2]["body"].replace("Wave:** 0", "Wave:** 1")
        self.fails("later wave")

    def test_reference_mapping_and_historical_dependencies(self):
        reference = issue(5, "A01", "# Reference\nCanonical owner: #3\n<details>\n<!-- symbiote-plan-key: A02 -->\n## Dependencies\n- #999\n</details>", [{"name": "planning:reference"}])
        reference["body"] = reference["body"].replace("symbiote-plan-key: A01", "symbiote-reference-key: A01", 1)
        self.snapshot.append(reference)
        registry = validate_snapshot(self.snapshot)
        self.assertEqual(registry["counts"]["references"], 1)
        self.assertEqual(registry["entries"][-1]["canonical_issue"], 3)
        reference["body"] = reference["body"].replace("Canonical owner: #3", "Canonical owner: #5")
        self.fails("alias must resolve directly")

    def test_missing_alias_label(self):
        self.snapshot[3]["body"] = self.snapshot[3]["body"].replace("symbiote-plan-key", "symbiote-reference-key")
        self.fails("invalid alias mapping")

    def test_not_planned_prerequisite(self):
        self.snapshot[2].update(state="closed", state_reason="not_planned")
        self.fails("epic membership|not canonical atomic")

    def test_archived_prerequisite(self):
        self.snapshot[2]["labels"].append({"name": "planning:archived"})
        self.fails("archived or not planned")

    def test_epic_membership(self):
        self.snapshot[1]["body"] = self.snapshot[1]["body"].replace("- [ ] #4", "- [ ] #999")
        self.fails("epic membership")

    def test_master_count(self):
        self.snapshot[0]["body"] = self.snapshot[0]["body"].replace("2 canonical", "3 canonical")
        self.fails("declared counts")

    def test_master_epic_membership(self):
        self.snapshot[0]["body"] = self.snapshot[0]["body"].replace("#2", "#999")
        self.fails("master epic membership")

    def test_duplicate_dependency_sections(self):
        self.snapshot[3]["body"] += "\n## Dependencies\n- #3\n"
        self.fails("duplicate Dependencies")

    def test_missing_revision(self):
        self.snapshot[3]["body"] = self.snapshot[3]["body"].replace("<!-- symbiote-plan-revision: approved-v1 -->", "")
        self.fails("missing revision authority")

    def test_body_label_wave_disagreement(self):
        self.snapshot[3]["labels"].append({"name": "wave:2"})
        self.fails("wave disagree")

    def test_cli_invalid_snapshot_writes_nothing(self):
        with tempfile.TemporaryDirectory() as directory:
            source = Path(directory) / "source.json"
            output = Path(directory) / "output"
            self.snapshot[3]["body"] = self.snapshot[3]["body"].replace("A02", "A01")
            source.write_text(json.dumps(self.snapshot))
            result = subprocess.run([sys.executable, str(Path(__file__).with_name("validate.py")), str(source), "--output", str(output)], capture_output=True, text=True)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("duplicate active key", result.stderr)
            self.assertFalse(output.exists())


if __name__ == "__main__":
    if "--snapshot" in sys.argv:
        at = sys.argv.index("--snapshot")
        snapshot_path = Path(sys.argv[at + 1])
        del sys.argv[at:at + 2]

        def test_live_snapshot_regression(self):
            snapshot = json.loads(snapshot_path.read_text())
            registry = validate_snapshot(snapshot)
            # Four issues carry a key and no planning label; two carry neither, so the
            # capture's own counts are unclassified 4 / unkeyed 2 and never 0 / 6.
            self.assertEqual(registry["counts"], {"tasks": 241, "epics": 19, "references": 198, "program_entries": 1,
                                                        "masters": 1, "retired": 0, "unclassified": 4, "unkeyed": 2,
                                                        "snapshot_issues": 466})
            positions = {n: i for i, n in enumerate(registry["topological_order"])}
            for entry in registry["entries"]:
                if entry["kind"] == "task":
                    for dependency in entry["dependencies"]:
                        self.assertLess(positions[dependency], positions[entry["number"]])
            changed = copy.deepcopy(snapshot)
            task = next(i for i in changed if i["number"] == 470)
            task["body"] = task["body"].replace("symbiote-plan-key: PLAN-01", "symbiote-plan-key: A01")
            with self.assertRaisesRegex(IntegrityError, "duplicate active key"):
                validate_snapshot(changed)

        IntegrityTests.test_live_snapshot_regression = test_live_snapshot_regression
    unittest.main()
