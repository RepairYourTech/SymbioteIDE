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
it reaches it, and a reordering costs nothing. The rest of the module is deliberately outside —
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


if __name__ == "__main__":
    unittest.main(verbosity=2)
