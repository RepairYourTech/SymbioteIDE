"""Run: python3 planning/integrity/test_revert_rules.py.

The driver's table is itself a claim: that every row's anchor is text this tree
holds exactly once, and that every row's case is one a suite actually runs. This
holds the table to the tree without running cargo, so a moved anchor or a renamed
case fails in a second rather than becoming a row the driver skips. The driver's
`HELD` table — the rules a reversion cannot prove, because the rule is the refusal
being reverted — is held the same way: each row's subject is in the tree, states
the refusal exactly once, and runs the case the row names.

What it does not ask is whether the table is complete: no relation between the
rows and the crate's refusal sites exists, so a rule added without a row is not
noticed here, and the driver's report is about the rows it has.

"Is this name a case?" is implemented twice — in the crate's own check (Rust,
`defines_a_case`) and in `is_case` below — because that check runs inside a cargo
test while this suite is Python-only and must not need a toolchain. The Rust one
decides what the crate does; a change to the rule is a change to both, and this
one governs the table's rows.
"""

import importlib
import unittest

import revert_rules
from revert_rules import HELD, ROOT, RULES, case_target, scanned


def collected(module):
    """The case names a loader collects from `module`."""
    names, stack = [], [unittest.defaultTestLoader.loadTestsFromModule(module)]
    while stack:
        item = stack.pop()
        if isinstance(item, unittest.TestSuite):
            stack.extend(item)
        else:
            names.append(item.id().rsplit(".", 1)[-1])
    return names


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
    def test_the_driver_roots_itself_at_this_repository(self):
        # The copy the driver mutates is made from this root, so it must be the
        # repository that holds the workspace and the driver itself.
        self.assertTrue((ROOT / "Cargo.toml").is_file(), ROOT)
        self.assertTrue((ROOT / "planning/integrity/revert_rules.py").is_file(), ROOT)

    def test_the_driver_scans_the_tree_it_is_held_to(self):
        files = scanned(ROOT)
        self.assertTrue(files, "the driver scans no file")
        for path in files:
            self.assertTrue(path.is_file(), path)

    def test_every_row_names_a_rule_an_anchor_a_removal_and_a_case(self):
        for row in RULES:
            self.assertEqual(len(row), 4, row)
            rule, anchor, removal, case = row
            self.assertTrue(rule.strip(), row)
            self.assertTrue(anchor.strip(), row)
            self.assertTrue(case.strip(), row)
            self.assertNotEqual(anchor, removal, rule)

    def test_every_anchor_is_held_once_in_the_tree(self):
        for rule, anchor, _removal, _case in RULES:
            self.assertIsNotNone(
                revert_rules.anchor_in(anchor, ROOT),
                f"{rule} is not held in exactly one file, so the driver cannot place it",
            )

    def test_no_two_rows_are_the_same_proof(self):
        # Two rows may hold one anchor and remove it differently — the document's
        # citation is renamed in one row and turned into a helper in another — but
        # no two rows are the same rule or the same mutation of one text.
        rules = [rule for rule, _anchor, _removal, _case in RULES]
        self.assertEqual(len(rules), len(set(rules)), "two rows state one rule")
        proofs = [(anchor, removal) for _rule, anchor, removal, _case in RULES]
        self.assertEqual(len(proofs), len(set(proofs)), "two rows make one mutation")

    def test_every_named_case_is_one_the_suite_runs(self):
        for rule, _anchor, _removal, case in RULES:
            target = case_target(case, ROOT)
            self.assertIsNotNone(target, f"{rule} names {case}, which no test target defines")
            package, name = target
            holder = ROOT / "crates" / package / "tests" / f"{name}.rs"
            self.assertTrue(
                is_case(holder.read_text(), case),
                f"{rule} names {case}, which is not a case the suite runs",
            )

    def test_the_table_names_rules_at_all(self):
        self.assertTrue(RULES, "the driver's table names no rule")

    def test_every_held_row_states_its_refusal_once_and_runs_the_case_it_names(self):
        for rule, subject, anchor, declares, case in HELD:
            path = ROOT / subject
            self.assertTrue(path.is_file(), f"{rule} names {subject}, which is not in the tree")
            held = path.read_text().count(anchor)
            self.assertEqual(held, 1, f"{rule} states its refusal {held} times in {subject}, so the "
                                       f"driver cannot place it")
            self.assertIsNotNone(revert_rules.declared(path.read_text(), declares),
                                 f"{rule} names {declares!r}, which {subject} does not declare a "
                                 f"number after")
            self.assertIn(case, collected(importlib.import_module(path.stem)),
                          f"{rule} names {case}, which {subject} does not run")

    def test_the_held_table_names_a_rule_a_subject_an_anchor_a_declaration_and_a_case(self):
        self.assertTrue(HELD, "the driver holds no rule by its case's presence")
        for row in HELD:
            self.assertEqual(len(row), 5, row)
            rule, subject, anchor, declares, case = row
            self.assertTrue(rule.strip() and anchor.strip() and declares.strip() and case.strip(), row)
            self.assertTrue(subject.startswith("planning/integrity/test_"), subject)
        rules = [rule for rule, _subject, _anchor, _declares, _case in HELD]
        self.assertEqual(len(rules), len(set(rules)), "two rows state one rule")


if __name__ == "__main__":
    unittest.main(verbosity=2)
