"""Run: python3 planning/integrity/test_reader_size.py.

`toolchains.py` is the one reader of the workflow spellings two rules compare, and nothing
measured the reading itself: the module grew twice (258 → 332 lines) and was read back down by
audits and passes rather than by anything this tree runs. This holds the reader's own size —
`entries`, where a block's lines become what each key states, and every module-level definition
it calls or reads: a function or class wherever a module-level statement writes one, a guard
included, and the names an assignment or an import binds — so a spelling it must read next
either replaces one it reads or moves the declaration below in the change itself.

Membership is derived from the module's own call and name graph, never listed here: a helper, a
class, a function written under a guard or an import the reader reads joins the count the moment
it reaches it, and a reordering costs nothing. The kinds it counts are held by cases over small
modules of their own, so a narrowing cannot hide behind this tree holding no instance of one.
The rest of the module is deliberately outside —
reading which lines belong to a job, resolving a stated path, answering about what was read, and
the manifest side's TOML may each grow without this case firing, and nothing here holds the
module's own size.

The declaration below equals the reader's size, so a reader read further down moves the number
too rather than leaving slack — and the measure is lines, so a line grown longer is not what
this case weighs.
"""
from __future__ import annotations

import ast
import pathlib
import unittest

TOOLCHAINS = pathlib.Path(__file__).resolve().parent / "toolchains.py"
# The reader's size at the merge that added this case (`580e41d2`): the three functions and four
# names it was read down to there, plus the `import re` the widened membership brings in. Growth
# is this number moving in a change that says which spelling the reader must now read — or the
# spelling that goes.
READER_LINES = 91
SEED = "entries"


def definitions(tree: ast.Module) -> dict[str, ast.AST]:
    """Every name a module-level statement binds: a function or class wherever it is written, a
    guard included, and the names an assignment or import binds. A definition's own body is not
    descended into — what it holds is the span of the definition itself.
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
    """The lines each name the reader is occupies: `entries`, everything it calls or reads, and
    what those state, followed through the module's own call and name graph.
    """
    defined = definitions(ast.parse(source))
    spans: dict[str, int] = {}
    pending = [SEED]
    while pending:
        name = pending.pop()
        node = defined.get(name)
        if name in spans or node is None:
            continue
        spans[name] = node.end_lineno - node.lineno + 1
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


# The kinds `definitions` counts, each as its own small module: the seed uses the definition, and
# the definition's name must join the spans. Written here rather than found in `toolchains.py`,
# so a kind is held while the real module holds no instance of it.
COUNTED = (
    ("helper", "def helper(value: str) -> str:\n    return value\n", "helper(block)"),
    ("Spell", 'class Spell:\n    """One spelling."""\n\n    @staticmethod\n'
              "    def tail(value: str) -> str:\n        return value\n", "Spell.tail(block)"),
    ("guarded", "if True:  # a guard, such as a platform check\n"
                "    def guarded(value: str) -> str:\n        return value\n", "guarded(block)"),
    ("FORMS", 'FORMS = {"plain": str}\n', 'FORMS["plain"](block)'),
    ("spelling", "import re as spelling\n", 'spelling.sub("", block)'),
)
# A binding the mechanism does not count: the docstring above states only assignment and import
# names, and these three are the boundary that sentence means.
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


class CountedKinds(unittest.TestCase):
    """The derivation's kinds, held by small modules written here rather than by whatever
    `toolchains.py` happens to contain. A kind deleted from the mechanism reds here alone.
    """

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


if __name__ == "__main__":
    unittest.main(verbosity=2)
