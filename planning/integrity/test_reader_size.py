"""Run: python3 planning/integrity/test_reader_size.py.

`toolchains.py` is the one reader the two toolchain rules compare, and nothing measured the
reading itself. This holds the reader's own size: `entries`, where a block's lines become what
each key states, and every module-level definition it reaches — a function or class wherever a
module-level statement writes one, a guard included, and the names an assignment or an import
binds — counted as the lines each spans, from a definition's first decorator onwards. It counts
the names it reads, so what is reached without one — a string, or the logic behind an imported
name — is not counted: that is where the membership stops, and the cases below hold its kinds and
limits over small modules of their own.

The rest of the module, and the module's own size, are deliberately outside this. The declaration
below equals the reader's size rather than leaving slack, and the measure is lines, so a line
grown longer costs nothing; this file's own size is declared the same way, and the case below
holds it.
"""
from __future__ import annotations

import ast
import pathlib
import sys
import unittest

TOOLCHAINS = pathlib.Path(__file__).resolve().parent / "toolchains.py"
GUARD = pathlib.Path(__file__).resolve()
# The reader's size: the three functions and seven names it was read down to, plus the `import re`
# the widened membership brings in. It was 91 from `dd3dae80` until `entries` learned to read a
# shell continuation, 98 until the marker stopped being part of what an entry states, 105 until it
# read a plain scalar's folded continuation, and 109 until it read a command the way a shell reads
# it and every other value the way a parser states it — which made an entry say whether it is a
# collection's items or one value and brought `COMMAND` in — and 136 since `unfolded` read a list
# of mappings, bringing `SEQUENCE`. Only what `entries` reaches moves this — a consumer outside its
# reach, `directory` among them, is not a change here.
READER_LINES = 136
SEED = "entries"
# This guard's own size: its lines, and the cases a loader finds in it.
GUARD_LINES = 181
GUARD_CASES = 7


def definitions(tree: ast.Module) -> dict[str, ast.AST]:
    """Every name a module-level statement binds, whichever statement writes it. A definition's own
    body is not descended into: what the definition holds is its span.
    """
    found: dict[str, ast.AST] = {}

    def collect(node: ast.AST) -> None:
        for child in ast.iter_child_nodes(node):
            if isinstance(child, (ast.FunctionDef, ast.AsyncFunctionDef, ast.ClassDef)):
                found[child.name] = child
                continue
            targets = (child.targets if isinstance(child, ast.Assign)
                       else [child.target] if isinstance(child, ast.AnnAssign) else [])
            for target in targets:
                if isinstance(target, ast.Name):
                    found[target.id] = child
            if isinstance(child, (ast.Import, ast.ImportFrom)):
                for alias in child.names:
                    found[(alias.asname or alias.name).split(".")[0]] = child
            collect(child)

    collect(tree)
    return found


def reader(source: str) -> dict[str, int]:
    """The lines each name the reader reaches occupies, followed through the module's own call and
    name graph."""
    defined = definitions(ast.parse(source))
    spans: dict[str, int] = {}
    pending = [SEED]
    while pending:
        name = pending.pop()
        node = defined.get(name)
        if name in spans or node is None:
            continue
        start = min([d.lineno for d in getattr(node, "decorator_list", [])] or [node.lineno])
        spans[name] = node.end_lineno - start + 1
        pending += [call.func.id for call in ast.walk(node)
                    if isinstance(call, ast.Call) and isinstance(call.func, ast.Name)]
        pending += [read.id for read in ast.walk(node)
                    if isinstance(read, ast.Name) and isinstance(read.ctx, ast.Load)]
    return spans


def small_module(definition: str, use: str) -> str:
    """A module whose reader is the seed: the reader uses what `definition` states."""
    return (f"def {SEED}(block: str) -> str:\n"
            f'    """The one reader of a small module."""\n'
            f"    return {use}\n\n\n{definition}")


# The kinds `definitions` counts, each as its own small module: the seed uses the definition and
# its name must join the spans — written here rather than found in `toolchains.py`.
COUNTED = (
    ("helper", "def helper(value: str) -> str:\n    return value\n", "helper(block)"),
    ("Spell", 'class Spell:\n    """One spelling."""\n\n    @staticmethod\n'
              "    def tail(value: str) -> str:\n        return value\n", "Spell.tail(block)"),
    ("guarded", "if True:  # a guard, such as a platform check\n"
                "    def guarded(value: str) -> str:\n        return value\n", "guarded(block)"),
    ("FORMS", 'FORMS = {"plain": str}\n', 'FORMS["plain"](block)'),
    ("spelling", "import re as spelling\n", 'spelling.sub("", block)'),
)
# Bindings the mechanism does not count: the ways a module-level statement binds a name without an
# assignment or an import — the boundary the case below holds, rather than one assumed here.
BOUND_ELSEWHERE = (
    ("BOUND", "for BOUND in (str,):\n    pass\n", "BOUND(block)"),
    ("handle", 'with open("a") as handle:\n    pass\n', "handle"),
    ("error", "try:\n    pass\nexcept Exception as error:\n    pass\n", "error"),
)


class ReaderSize(unittest.TestCase):
    def test_the_reader_is_the_size_declared_for_it(self):
        spans = reader(TOOLCHAINS.read_text())
        self.assertIn(SEED, spans, f"toolchains.py defines no {SEED}, so this case no longer "
                                   f"knows where the reader starts")
        size = sum(spans.values())
        self.assertEqual(size, READER_LINES,
                         f"the reader is {size} lines — {', '.join(sorted(spans))} — where "
                         f"{READER_LINES} is declared: the declaration moves with any change to "
                         f"the reader's size, and a spelling it must now read either replaces "
                         f"one it reads or is the reason this number moved")


class GuardSize(unittest.TestCase):
    """The guard's own size, declared the way the reader's is: growing this file without moving
    the declaration reds here, and no edit outside this file can.
    """

    def test_the_guard_is_the_size_declared_for_it(self):
        lines = len(GUARD.read_text().splitlines())
        cases = unittest.defaultTestLoader.loadTestsFromModule(
            sys.modules[__name__]).countTestCases()
        self.assertEqual((lines, cases), (GUARD_LINES, GUARD_CASES),
                         f"the guard is {lines} lines and {cases} cases where {GUARD_LINES} and "
                         f"{GUARD_CASES} are declared: a change that grows it moves them in the "
                         f"same change")


class CountedKinds(unittest.TestCase):
    """The kinds the derivation counts, held by small modules written here: a kind deleted from the
    mechanism reds here alone."""

    def test_every_definition_kind_the_reader_reaches_is_counted(self):
        for name, definition, use in COUNTED:
            with self.subTest(kind=name):
                spans = reader(small_module(definition, use))
                self.assertIn(name, spans, f"{name} is a kind the reader reaches: {sorted(spans)}")

    def test_a_name_bound_by_a_loop_a_with_or_a_handler_is_not_counted(self):
        for name, definition, use in BOUND_ELSEWHERE:
            with self.subTest(binding=name):
                spans = reader(small_module(definition, use))
                self.assertNotIn(name, spans,
                                 f"{name} is bound by a module-level statement that binds no "
                                 f"assignment or import: {sorted(spans)}")

    def test_a_helper_reached_only_as_a_string_is_not_counted(self):
        reached = "def helper(value: str) -> str:\n    return value\n"
        spans = reader(small_module(reached, 'globals()["helper"](block)'))
        self.assertNotIn("helper", spans,
                         f"a string is not a binding the reader reads: {sorted(spans)}")

    def test_a_helper_reached_through_an_import_is_not_counted(self):
        behind = "def tail(value: str) -> str:\n    return value\n"
        spans = reader(small_module("import spelling\n" + behind, "spelling.tail(block)"))
        self.assertEqual(spans.get("spelling"), 1,
                         f"`import spelling` is one line: {spans}")
        self.assertNotIn("tail", spans,
                         f"not the logic an imported name reaches: {sorted(spans)}")

    def test_a_definition_costs_its_decorator_too(self):
        decorated = "@staticmethod\ndef helper(value: str) -> str:\n    return value\n"
        spans = reader(small_module(decorated, "helper(block)"))
        self.assertEqual(spans.get("helper"), 3,
                         f"the decorator line is part of what helper costs: {spans}")


if __name__ == "__main__":
    unittest.main(verbosity=2)
