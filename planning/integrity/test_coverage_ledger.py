"""Run: python3 planning/integrity/test_coverage_ledger.py."""

import hashlib
import json
from pathlib import Path
import subprocess
import sys
import unittest

import coverage_ledger


def task(number, dependencies=(), acceptance_items=1):
    return {"number": number, "kind": "task", "dependencies": list(dependencies),
            "acceptance_items": acceptance_items}


def epic(number, children=()):
    return {"number": number, "kind": "epic", "dependencies": [], "children": list(children),
            "acceptance_items": 1}


def reference(number, canonical_issue):
    return {"number": number, "kind": "reference", "dependencies": [], "canonical_issue": canonical_issue}


def registry_of(*entries, **declared):
    """A registry whose declared counts are counts of the entries it holds, unless asked."""
    counts = {
        "tasks": sum(1 for entry in entries if entry["kind"] == "task"),
        "epics": sum(1 for entry in entries if entry["kind"] == "epic"),
        "references": sum(1 for entry in entries if entry["kind"] == "reference"),
        "program_entries": 0,
    }
    counts.update(declared)
    return {"entries": list(entries), "counts": counts}


MATRIX = "# Accepted decision coverage\n\n## Firm identity\n\n#1\n\n## Delivery\n\n#3\n"


class CoverageLedgerTests(unittest.TestCase):
    def setUp(self):
        self.snapshot = (task(1), task(2), task(3, dependencies=(1,)), epic(9), reference(400, 1))
        self.registry = registry_of(*self.snapshot)

    def found(self, matrix=MATRIX, registry=None, audited=None):
        """Every refusal for a ledger of this matrix and registry, named."""
        registry = self.registry if registry is None else registry
        provenance = dict(coverage_ledger.PROVENANCE,
                          audited_totals=coverage_ledger.facts(registry) if audited is None else audited)
        built = coverage_ledger.ledger(matrix, registry, provenance)
        return coverage_ledger.problems(built, registry, matrix)

    def fails(self, text, matrix=MATRIX, registry=None, audited=None):
        self.assertRegex("\n".join(self.found(matrix, registry, audited)), text)

    def test_a_sound_matrix_over_a_consistent_registry_is_refused_for_nothing(self):
        self.assertEqual(self.found(), [])

    def test_a_family_with_no_name_is_refused(self):
        self.fails("a coverage family has no name", "# Coverage\n\n## \n\n#1\n")

    def test_a_family_named_twice_is_refused(self):
        self.fails("the family 'Firm identity' is named twice",
                   "# Coverage\n\n## Firm identity\n\n#1\n\n## Firm identity\n\n#2\n")

    def test_a_family_with_no_owner_is_refused(self):
        self.fails("the family 'Empty' names no owner, which is not coverage",
                   "# Coverage\n\n## Empty\n\nno owners named here\n")

    def test_an_owner_named_twice_by_one_family_is_refused(self):
        self.fails("the family 'Firm identity' names #1 more than once",
                   "# Coverage\n\n## Firm identity\n\n#1, #1, #2\n")

    def test_an_owner_the_registry_does_not_hold_is_refused(self):
        self.fails("the family 'Firm identity' names #404, which the registry does not hold",
                   "# Coverage\n\n## Firm identity\n\n#404\n")

    def test_an_owner_the_registry_holds_as_a_reference_is_refused(self):
        self.fails("the family 'Firm identity' names #400, a reference entry rather than the work it resolves to",
                   "# Coverage\n\n## Firm identity\n\n#1, #400\n")

    def test_a_registry_whose_declared_counts_disagree_with_its_entries_is_refused(self):
        self.fails("the registry declares 99 tasks and holds 3",
                   registry=registry_of(*self.snapshot, tasks=99))

    def test_totals_that_are_not_the_registrys_are_refused(self):
        built = coverage_ledger.ledger(MATRIX, self.registry,
                                       dict(coverage_ledger.PROVENANCE, audited_totals=coverage_ledger.facts(self.registry)))
        built["totals"]["edges"] = 900
        self.assertIn("the ledger's edges is 900 where the registry holds 1",
                      coverage_ledger.problems(built, self.registry, MATRIX))

    def test_a_registry_that_moved_past_the_audited_revision_is_refused(self):
        """The reconciliation is frozen to the inventory the audit read; a later one needs re-deriving."""
        self.fails("the audit reconciled 241 atoms at 2026-09-07-v2.4 and the registry holds 3: "
                   "the coverage needs re-deriving against the current inventory",
                   audited=coverage_ledger.PROVENANCE["audited_totals"])

    def test_canonical_work_that_declares_no_acceptance_item_is_refused(self):
        registry = registry_of(task(1), task(2, acceptance_items=0), epic(9))
        self.fails("#2 is canonical work that declares no acceptance item", registry=registry)

    def test_unclaimed_atoms_that_are_not_what_the_families_leave_unclaimed_are_refused(self):
        built = coverage_ledger.ledger(MATRIX, self.registry,
                                       dict(coverage_ledger.PROVENANCE, audited_totals=coverage_ledger.facts(self.registry)))
        built["unclaimed_atoms"] = []
        self.assertIn("the ledger's unclaimed atoms are not the atoms these families leave unclaimed",
                      coverage_ledger.problems(built, self.registry, MATRIX))

    def test_a_ledger_of_another_matrix_is_refused(self):
        other = "# Coverage\n\n## Firm identity\n\n#1\n"
        built = coverage_ledger.ledger(other, self.registry,
                                       dict(coverage_ledger.PROVENANCE, audited_totals=coverage_ledger.facts(self.registry)))
        self.assertRegex("\n".join(coverage_ledger.problems(built, self.registry, MATRIX)),
                         "it is not this matrix's ledger")

    def test_the_ledger_records_the_atoms_the_families_leave_unclaimed(self):
        built = coverage_ledger.ledger(MATRIX, self.registry)
        self.assertEqual(built["unclaimed_atoms"], [2])
        self.assertEqual(built["families_count"], 2)

    def test_the_ledger_records_the_matrixs_bytes(self):
        built = coverage_ledger.ledger(MATRIX, self.registry)
        self.assertEqual(built["provenance"]["matrix_sha256"], hashlib.sha256(MATRIX.encode()).hexdigest())


class CommittedLedgerTests(unittest.TestCase):
    """The matrix, the ledger and the registry this tree ships, checked as they are."""

    def setUp(self):
        self.root = Path(__file__).parent
        self.matrix = (self.root / "fixtures/audit-2026-09-07-coverage.md").read_text()
        self.registry = json.loads((self.root / "generated/registry.json").read_text())
        self.ledger = json.loads((self.root / "generated/coverage.json").read_text())

    def test_the_committed_matrix_is_the_artifact_bytes(self):
        self.assertEqual(hashlib.sha256(self.matrix.encode()).hexdigest(),
                         "1e86040cf4d0142c10eb10aadaa66b3daffcc370c451075e27fc2d9699b7b116")

    def test_the_audited_totals_are_the_artifacts_numbers_and_the_registrys(self):
        """The audit's revision counts are held as data, so a moved inventory is refused."""
        self.assertEqual(coverage_ledger.PROVENANCE["audited_totals"],
                         {"atoms": 241, "epics": 19, "references": 198, "edges": 702})
        self.assertEqual(coverage_ledger.facts(self.registry), coverage_ledger.PROVENANCE["audited_totals"])
        self.assertEqual(self.ledger["totals"], coverage_ledger.PROVENANCE["audited_totals"])

    def test_the_committed_ledger_is_true_of_the_committed_registry(self):
        self.assertEqual(coverage_ledger.problems(self.ledger, self.registry, self.matrix), [])

    def test_the_committed_ledger_names_the_coverage_and_what_no_family_claims(self):
        self.assertEqual(self.ledger["families_count"], 28)
        self.assertEqual(self.ledger["owners_named"], 196)
        self.assertEqual(len(self.ledger["unclaimed_atoms"]), 47)
        self.assertEqual(self.ledger["atoms_without_acceptance_items"], [])

    def test_check_matches_the_committed_ledger_and_write_reproduces_it(self):
        for mode in ("--check", "--write"):
            result = subprocess.run([sys.executable, "coverage_ledger.py", mode], cwd=self.root,
                                    capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertIn("28 families, 196 owners, 241 atoms, 47 unclaimed", result.stdout)
        self.assertEqual((self.root / "generated/coverage.json").read_text(),
                         coverage_ledger.render(self.matrix, self.registry))

    def check(self):
        return subprocess.run([sys.executable, "coverage_ledger.py", "--check"], cwd=self.root,
                              capture_output=True, text=True)

    def test_check_fails_closed_on_a_drifted_ledger_and_on_a_missing_one(self):
        drift = self.root / "generated/coverage.json"
        original = drift.read_text()
        try:
            drift.write_text(original.replace('"owners_named": 196', '"owners_named": 195'))
            result = self.check()
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("is not what this tree emits", result.stderr)
            drift.unlink()
            result = self.check()
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("does not exist", result.stderr)
        finally:
            drift.write_text(original)

    def test_check_names_the_untrue_claim_inside_a_committed_ledger(self):
        """Drift is reported generically; the claim that is untrue is named too."""
        drift = self.root / "generated/coverage.json"
        original = drift.read_text()
        try:
            drift.write_text(original.replace('"unclaimed_atoms": [', '"unclaimed_atoms": [999, '))
            result = self.check()
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("the ledger's unclaimed atoms are not the atoms these families leave unclaimed", result.stderr)
            self.assertIn("is not what this tree emits", result.stderr)
        finally:
            drift.write_text(original)

    def test_write_refuses_a_ledger_that_is_not_true_of_the_registry_it_was_given(self):
        """A refused ledger is not written, so a broken inventory cannot be blessed."""
        import tempfile
        with tempfile.TemporaryDirectory() as scratch:
            inconsistent = json.loads(json.dumps(self.registry))
            inconsistent["counts"]["tasks"] = 99
            (Path(scratch) / "registry.json").write_text(json.dumps(inconsistent))
            result = subprocess.run([sys.executable, "coverage_ledger.py", "--write",
                                     "--registry", f"{scratch}/registry.json",
                                     "--output", f"{scratch}/coverage.json"],
                                    cwd=self.root, capture_output=True, text=True)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("the registry declares 99 tasks and holds 241", result.stderr)
            self.assertIn("refusing to write a ledger that is not true of this registry", result.stderr)
            self.assertFalse((Path(scratch) / "coverage.json").exists())


if __name__ == "__main__":
    unittest.main()
