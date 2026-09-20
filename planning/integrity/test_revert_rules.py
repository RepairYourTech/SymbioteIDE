"""Run: python3 planning/integrity/test_revert_rules.py.

The driver's table is itself a claim: that every row's anchor is text this tree
holds exactly once, and that every row's case is one a suite actually runs. This
holds the table to the tree without running cargo, so a moved anchor or a renamed
case fails in a second rather than becoming a row the driver skips. The driver's
`HOLDS` table — the rules a reversion cannot prove, because the rule is the
refusal being reverted or the link the refusal is about — is held the same way:
each row's subject is in the tree, states the refusal exactly once, names a case
the subject runs, and lays every way it claims in a file this tree holds — every
kind of way the driver names, `weaker` included, whose two texts must be text this
tree carries once. The driver's own run re-reads the live tree, so the case below
drives that comparison rather than reading the code that makes it.

What it does not ask is whether the table is complete: no relation between the
rows and the crate's refusal sites exists, so a rule added without a row is not
noticed here, and the driver's report is about the rows it has.

"Is this name a case?" is implemented twice — in the crate's own check (Rust,
`defines_a_case`) and in `is_case` below — because that check runs inside a cargo
test while this suite is Python-only and must not need a toolchain. The Rust one
decides what the crate does; a change to the rule is a change to both, and this
one governs the table's rows.
"""

import contextlib
import hashlib
import importlib
import io
import pathlib
import shutil
import tempfile
import unittest

import revert_rules
from revert_rules import DECLARED, HOLDS, HOLDER, ROOT, RULES, WAYS, case_target, scanned, unheld_cases


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


@contextlib.contextmanager
def driver_state(**swapped):
    """The driver's own names swapped for a run, and put back whatever the run does: a case that
    drives the run needs the state the run reads, and one place to swap it is one place to get it
    back.
    """
    real = {name: getattr(revert_rules, name) for name in swapped}
    for name, value in swapped.items():
        setattr(revert_rules, name, value)
    try:
        yield
    finally:
        for name, value in real.items():
            setattr(revert_rules, name, value)


class Copy:
    """`shutil` as a run of the driver here uses it: a copy holding only the hold's own file, so a
    run reads what its rows need rather than the whole repository, and a real removal.
    """

    @staticmethod
    def copytree(source, destination, **_):
        made = pathlib.Path(destination) / "planning" / "integrity"
        made.mkdir(parents=True, exist_ok=True)
        shutil.copy2(pathlib.Path(source) / "planning/integrity/test_revert_rules.py",
                     made / "test_revert_rules.py")

    rmtree = staticmethod(shutil.rmtree)


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

    def test_the_hold_table_names_a_rule_a_case_a_subject_and_its_ways(self):
        self.assertTrue(HOLDS, "the driver holds no refusal at all")
        for rule, case, stated_in, states, ways in HOLDS:
            self.assertTrue(rule.strip() and case.strip() and stated_in.strip(), (rule, case))
            self.assertTrue(stated_in.startswith("planning/integrity/test_"), stated_in)
            self.assertTrue(ways, f"{rule} names no way to show its refusal")
        rules = [rule for rule, *_ in HOLDS]
        self.assertEqual(len(rules), len(set(rules)), "two rows state one rule")

    def test_every_hold_row_states_its_refusal_once_and_lays_its_ways_in_the_tree(self):
        # A row whose refusal or way cannot be placed is a row the driver cannot show failing, so it
        # fails here in a second instead of going quiet.
        for rule, case, stated_in, states, ways in HOLDS:
            subject = ROOT / stated_in
            self.assertTrue(subject.is_file(), f"{rule} names {stated_in}, not in the tree")
            stated = subject.read_text().count(states)
            self.assertEqual(stated, 1, f"{rule} states its refusal {stated} times in {stated_in}, "
                                        f"so the driver cannot place it")
            self.assertIn(case, collected(importlib.import_module(subject.stem)),
                          f"{rule} names {case}, which {stated_in} does not run")
            for where, how, argument in ways:
                self.assertIn(how, WAYS, f"{rule} names {how!r}")
                path = ROOT / where
                self.assertTrue(path.is_file(), f"{rule} names {where}, not in the tree")
                if how == "larger":
                    self.assertIsNotNone(revert_rules.declared(path.read_text(), argument),
                                         f"{rule} names {argument!r}, which {where} does not "
                                         f"declare a number after")
                elif how in ("gone", "weaker"):
                    text = argument if how == "gone" else argument[0]
                    found = path.read_text().count(text)
                    self.assertEqual(found, 1, f"{rule} names a {how} way {where} carries {found} "
                                               f"times, so the driver cannot place it")
    def test_a_case_no_row_names_and_no_declaration_accounts_for_is_refused(self):
        """Driven over texts written here: a case the file collects that neither a row names nor the
        declaration accounts for is a case whose loss nothing would show, so it is refused — and so
        is a declared case the file no longer collects, which is a case renamed out of the suite.
        Which names to write over is read from the file rather than copied into this one, so a rename
        cannot leave the text below unchanged while the case passes on its own stale input.
        """
        self.assertIsNone(unheld_cases(ROOT), "a case here is unaccounted for")
        here = ROOT / "planning/integrity/test_revert_rules.py"
        body = here.read_text()
        declared = sorted(DECLARED[HOLDER] & set(collected(importlib.import_module(here.stem))))
        self.assertEqual(declared, sorted(DECLARED[HOLDER]),
                         "the declaration names a case this file no longer collects")

        def written_over(name, replacement):
            """The file's text with `name`'s definition replaced, or a failure when it has none."""
            written = body.replace(f"    def {name}(self):", replacement, 1)
            self.assertNotEqual(written, body, f"{name} is not written as a case in {here.name}, "
                                               f"so this case would be driving nothing")
            return written

        with tempfile.TemporaryDirectory() as where:
            scratch = pathlib.Path(where) / "planning/integrity"
            scratch.mkdir(parents=True)
            (scratch / here.name).write_text(written_over(
                declared[0], "    def test_nothing_accounts_for_this(self):\n        pass\n\n"
                             f"    def {declared[0]}(self):"))
            self.assertIn("no declaration accounts for", unheld_cases(scratch.parent.parent) or "")
            (scratch / here.name).write_text(written_over(
                declared[-1], "    def renamed_out_of_the_suite(self):"))
            self.assertIn("collects no such case", unheld_cases(scratch.parent.parent) or "")

    def test_a_live_file_moved_while_the_driver_runs_is_refused(self):
        """The driver's own guarantee, driven rather than read: it digests the files it watches
        before and after the run and refuses when one moved, and nothing outside that comparison
        observes it. So `digest` is given two different answers and the run must say so and fail.
        """
        here = ROOT / "planning/integrity/test_revert_rules.py"
        self.assertEqual(revert_rules.digest([here]),
                         {str(here.relative_to(ROOT)): hashlib.sha256(here.read_bytes()).hexdigest()},
                         "a file's digest must be its own bytes, or the re-read cannot see a move")
        reads = []

        def two_answers(_files):
            reads.append(1)
            return {"moved.md": "before" if len(reads) == 1 else "after"}

        said = io.StringIO()
        with driver_state(digest=two_answers, shutil=Copy, RULES=[], HOLDS=[], NEGATIVE=()):
            with contextlib.redirect_stdout(said):
                code = revert_rules.main()
        self.assertIn("1 files of the live tree moved (moved.md)", said.getvalue(),
                      "the driver did not refuse a file moved while it ran")
        self.assertNotEqual(code, 0, "the run must fail when the live tree moved")

    def test_the_checker_and_its_own_proof_refuse_a_way_that_does_not_bite(self):
        # The checker cannot hold its own presence by watching the rows it judges, so this drives it
        # and its built-in negative inputs directly: a way that leaves its case green must be
        # reported, a checker patched to hold everything must be refused by the proof, and a run
        # handed that same checker must refuse it by name — the run reading its own checker, driven
        # rather than read off the line that calls it.
        with tempfile.TemporaryDirectory() as where:
            scratch = pathlib.Path(where)
            tree, subject = revert_rules.negative_inputs(scratch)
            for way in revert_rules.NEGATIVE:
                why = revert_rules.shown(subject, way, "test_ok", tree, scratch)
                self.assertIsNotNone(why, f"a way that does not bite reads as held: {way}")
            self.assertIsNone(revert_rules.self_proof(scratch), "the proof must refuse each input")
            said = io.StringIO()
            with driver_state(shown=lambda *args, **kwargs: None, shutil=Copy, RULES=[]):
                with contextlib.redirect_stdout(said):
                    code = revert_rules.main()
            self.assertIn("NOT HELD: this driver's own checker", said.getvalue(),
                          "a run handed a checker that holds everything must refuse it by name")
            self.assertNotEqual(code, 0, "the run must fail when its own checker is vacated")


if __name__ == "__main__":
    unittest.main(verbosity=2)
