"""Run: python3 planning/integrity/test_revert_rules.py.

The revert driver's table is itself a claim: that every rule it says it removes is
still held by text this tree contains once, and that the case it names is one the
suite runs. Nothing here runs cargo — this holds the table to the tree, so a
renamed case or a moved anchor fails in a second rather than becoming a row the
driver skips when somebody finally runs it.
"""

import unittest
from pathlib import Path

import revert_rules
from revert_rules import CRATE, DOCUMENT, ROOT, RULES


def tree_files():
    return [
        *sorted((CRATE / "src").rglob("*.rs")),
        *sorted((CRATE / "tests").rglob("*.rs")),
        DOCUMENT,
    ]


def is_case(text, name):
    """Whether `text` defines `name` as a case it runs, not merely as a function."""
    lines = text.splitlines()
    needle = f"fn {name}("
    for at, line in enumerate(lines):
        if not line.lstrip().startswith(needle):
            continue
        above = at
        while above > 0:
            previous = lines[above - 1].strip()
            if previous == "#[test]":
                return True
            if previous.startswith("#[") or not previous:
                above -= 1
                continue
            break
    return False


class RevertRulesTable(unittest.TestCase):
    def test_the_driver_proves_rules_of_the_crate_it_names(self):
        self.assertTrue((ROOT / "Cargo.toml").is_file(), ROOT)
        self.assertEqual(ROOT / "crates/symbiote-architecture", CRATE)
        self.assertTrue(DOCUMENT.is_file(), DOCUMENT)

    def test_every_row_names_a_rule_an_anchor_a_removal_and_a_case(self):
        for row in RULES:
            self.assertEqual(len(row), 4, row)
            rule, anchor, removal, case = row
            self.assertTrue(rule.strip(), row)
            self.assertTrue(anchor.strip(), row)
            self.assertTrue(case.strip(), row)
            self.assertNotEqual(anchor, removal, rule)

    def test_every_row_changes_the_tree(self):
        for rule, anchor, removal, _case in RULES:
            self.assertFalse(
                removal in anchor and anchor == removal,
                f"{rule} removes nothing",
            )
            self.assertTrue(
                anchor not in removal or removal.startswith(anchor),
                f"{rule} removes the anchor without replacing it",
            )

    def test_every_anchor_is_held_once_in_the_tree(self):
        for rule, anchor, _removal, _case in RULES:
            holders = [path for path in tree_files() if path.read_text().count(anchor) == 1]
            self.assertEqual(
                len(holders),
                1,
                f"{rule} is held in {len(holders)} files, so the driver cannot place it",
            )

    def test_no_two_rows_are_the_same_proof(self):
        # Two rows may hold one anchor and remove it differently — the document's
        # citation is renamed in one row and turned into a helper in the other —
        # but no two rows are the same rule or the same mutation of one text.
        rules = [rule for rule, _anchor, _removal, _case in RULES]
        self.assertEqual(len(rules), len(set(rules)), "two rows state one rule")
        proofs = [(anchor, removal) for _rule, anchor, removal, _case in RULES]
        self.assertEqual(len(proofs), len(set(proofs)), "two rows make one mutation")

    def test_every_named_case_is_one_the_suite_runs(self):
        for rule, _anchor, _removal, case in RULES:
            holders = [
                path for path in sorted((ROOT / "crates").glob("*/tests/*.rs"))
                if f"fn {case}(" in path.read_text()
            ]
            self.assertEqual(
                len(holders),
                1,
                f"{rule} names {case}, which {len(holders)} test targets define",
            )
            self.assertTrue(
                is_case(holders[0].read_text(), case),
                f"{rule} names {case}, which is not a case the suite runs",
            )

    def test_the_table_is_not_empty(self):
        self.assertGreater(len(RULES), 40, len(RULES))


if __name__ == "__main__":
    unittest.main(verbosity=2)
