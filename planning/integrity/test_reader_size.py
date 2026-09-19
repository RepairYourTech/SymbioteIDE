"""Run: python3 planning/integrity/test_reader_size.py.

`toolchains.py` is the one reader the two toolchain rules compare, and nothing measured the
reading itself. This holds the reading path: every module-level definition that reads text — a
pattern read by name, an `re`/`tomllib` operation, `read_text`, `splitlines` — and everything such
a definition reaches. Reading is *acquiring* text, so a definition that only rearranges a value
the path states is not a reader, which is why `legs` trades in strings and still sits outside. The
names the rules read directly are theirs, derived from the rule files rather than listed here, and
what they reach without the path in between must read no text, so a spelling cannot be read by the
rules instead of the reader.

No classifier anticipates every spelling, so the operation list is not trusted either: a case reds
when a definition outside the path calls an operation the classifier does not name, so a reading
cannot hide outside the path and outside the fence.

The reading path is not the module: the names the rules read that reach nothing in the path —
measured, `legs`, `workspace_jobs`, `floor`, `declared`, `version`, `workspace_of` — are outside
it, free to grow. The declaration below equals the path's size rather than leaving slack, and the
measure is lines, so a line grown longer costs nothing; this file's own size is declared the same
way.
"""
from __future__ import annotations

import ast
import pathlib
import sys
import unittest

HERE = pathlib.Path(__file__).resolve().parent
TOOLCHAINS = HERE / "toolchains.py"
GUARD = pathlib.Path(__file__).resolve()
RULES = ("test_toolchain_floors.py", "test_handoff.py")
# The reading path's size, and this guard's own: its lines, and the cases a loader finds in it.
READER_LINES = 450
GUARD_LINES = 375
GUARD_CASES = 12
# Local modules whose reading is part of the subject. The repo's own files only, named here
# rather than followed quietly: one a reader reaches through an import must be declared, and its
# reading then counts like the reader's own. `python_floor` reads no text, so 450 stands.
LOCAL_IMPORTS: tuple[str, ...] = ("python_floor",)
# What reading text is, as a shape: these module names, whose operations read it, and the calls
# that turn a file or a block into lines. A definition holding one of these reads text.
TEXT_MODULES = frozenset({"re", "tomllib"})
TEXT_CALLS = frozenset({"read_text", "readlines", "read", "splitlines"})
# Every operation this module calls in a definition the path does not hold, named so a new one
# reds the case below rather than hiding: `split`/`join`/`strip`/`count` rearrange a value the
# path stated, the next six touch no text, and the builtins stand for the same claim as a method.
NOT_READING = frozenset({"split", "join", "strip", "count", "get", "values", "fromkeys",
                         "append", "is_file", "resolve", "any()", "isinstance()", "list()",
                         "next()", "range()", "AssertionError()"})


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


def patterns(defined: dict[str, ast.AST]) -> set[str]:
    """The names this module binds a compiled pattern to."""
    return {name for name, node in defined.items()
            if isinstance(node, ast.Assign) and isinstance(node.value, ast.Call)
            and isinstance(node.value.func, ast.Attribute)
            and node.value.func.attr == "compile"
            and isinstance(node.value.func.value, ast.Name) and node.value.func.value.id == "re"}


def reads(node: ast.AST, named: set[str]) -> list[str]:
    """What this definition reads text through, named — empty when it reads none."""
    found = []
    for child in ast.walk(node):
        if isinstance(child, ast.Name) and isinstance(child.ctx, ast.Load) and child.id in named:
            found.append(child.id)
        if isinstance(child, ast.Call) and isinstance(child.func, ast.Attribute):
            if isinstance(child.func.value, ast.Name) and child.func.value.id in TEXT_MODULES:
                found.append(f"{child.func.value.id}.{child.func.attr}")
            elif child.func.attr in TEXT_CALLS:
                found.append(f".{child.func.attr}")
    return found


def reachable(defined: dict[str, ast.AST], seeds: set[str]) -> set[str]:
    """The names reached from the seeds by call or read, the seeds included."""
    touched, frontier = set(seeds), list(seeds)
    while frontier:
        name = frontier.pop()
        node = defined.get(name)
        if node is None:
            continue
        for child in ast.walk(node):
            callee = (child.func.id if isinstance(child, ast.Call)
                      and isinstance(child.func, ast.Name)
                      else child.id if isinstance(child, ast.Name)
                      and isinstance(child.ctx, ast.Load) else None)
            if callee in defined and callee not in touched:
                touched.add(callee)
                frontier.append(callee)
    return touched


def reading(source: str) -> dict[str, int]:
    """The reading path: every definition that reads text, the lines each occupies, and everything
    those definitions reach.
    """
    defined = definitions(ast.parse(source))
    named = patterns(defined)
    seeds = {name for name, node in defined.items() if reads(node, named)}
    found: dict[str, int] = {}
    for name in reachable(defined, seeds):
        node = defined.get(name)
        if node is None:
            continue
        start = min([d.lineno for d in getattr(node, "decorator_list", [])] or [node.lineno])
        found[name] = node.end_lineno - start + 1
    return found


def local_chain(folder: pathlib.Path) -> list[str]:
    """Every repo module the reader reaches through local imports, `toolchains` excluded: a file
    beside it, never a package path or a stdlib name, so this cannot wander outside the tree.
    """
    seen, worklist = set(), ["toolchains"]
    while worklist:
        source = (folder / f"{worklist.pop()}.py").read_text()
        for node in ast.walk(ast.parse(source)):
            names = ([node.module] if isinstance(node, ast.ImportFrom) and node.module
                     else [alias.name for alias in node.names]
                     if isinstance(node, ast.Import) else [])
            for module in names:
                if module not in seen and module != "toolchains" \
                        and (folder / f"{module.replace('.', '/')}.py").is_file():
                    seen.add(module)
                    worklist.append(module)
    return sorted(seen)


def reading_path(folder: pathlib.Path) -> dict[str, int]:
    """The reader's subject: its own reading path, and the reading of every declared local module
    it imports, keyed `module.name` — a helper the reader calls counts where it lives, so reading
    cannot leave the subject by changing file.
    """
    held = reading((folder / "toolchains.py").read_text())
    for module in LOCAL_IMPORTS:
        source = (folder / f"{module}.py").read_text()
        held |= {f"{module}.{name}": span for name, span in reading(source).items()}
    return held


def rules_read(folder: pathlib.Path) -> set[str]:
    """The names the two rules take from `toolchains`, read from the rule files themselves."""
    names: set[str] = set()
    for rule in RULES:
        for node in ast.walk(ast.parse((folder / rule).read_text())):
            if isinstance(node, ast.ImportFrom) and node.module == "toolchains":
                names |= {alias.name for alias in node.names}
            if isinstance(node, ast.Attribute) and isinstance(node.value, ast.Name) \
                    and node.value.id == "toolchains":
                names.add(node.attr)
    return names


def outside(source: str, used: set[str]) -> dict[str, list[str]]:
    """Definitions that are neither the reading path nor the rules' own — what the rules read and
    what that reaches before the path takes over — each with what it reads or reaches in the path.
    """
    defined = definitions(ast.parse(source))
    held = set(reading(source))
    own = reachable(defined, set(used) & set(defined) - held) - held
    named = patterns(defined)
    flagged = {}
    for name, node in defined.items():
        if name in held or name in own or isinstance(node, (ast.Import, ast.ImportFrom)):
            continue
        reach = {child.func.id for child in ast.walk(node)
                 if isinstance(child, ast.Call) and isinstance(child.func, ast.Name)
                 and child.func.id in held}
        flagged[name] = sorted(set(reads(node, named)) | reach) or ["nothing reads it"]
    return flagged


def operations(source: str) -> dict[str, list[str]]:
    """For each definition the reading path does not hold, the operations it calls that the
    classifier does not name. A call to a name this module binds is not an operation: that
    definition is held by the path or by the case that calls this.
    """
    defined = definitions(ast.parse(source))
    held, known = set(reading(source)), TEXT_CALLS | {"compile"}

    def unnamed(node: ast.AST):
        for call in ast.walk(node):
            if not isinstance(call, ast.Call):
                continue
            func = call.func
            if isinstance(func, ast.Attribute):
                if func.attr not in known and not (isinstance(func.value, ast.Name)
                                                   and func.value.id in TEXT_MODULES):
                    yield func.attr
            elif isinstance(func, ast.Name) and func.id not in defined and func.id not in known:
                yield f"{func.id}()"

    return {name: sorted(set(unnamed(node))) for name, node in defined.items()
            if name not in held and not isinstance(node, (ast.Import, ast.ImportFrom))}


def small_module(definition: str, use: str) -> str:
    """A module whose reading path starts at `spell`: it reads the block a line at a time."""
    return (f"def spell(block: str) -> str:\n"
            f'    """The one definition here that reads text."""\n'
            f"    return {use}.splitlines()[0]\n\n\n{definition}")


# The kinds `definitions` counts, each as its own small module: the reading path reaches the
# definition and its name must join the spans — written here rather than found in `toolchains.py`.
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
    def test_the_reading_path_is_the_size_declared_for_it(self):
        held = reading_path(HERE)
        self.assertIn("entries", held, "toolchains.py's reader is no longer reached, so this case "
                                       "no longer knows where the path starts")
        size = sum(held.values())
        self.assertEqual(size, READER_LINES,
                         f"the reading path is {size} lines — {', '.join(sorted(held))} — where "
                         f"{READER_LINES} is declared: the declaration moves with any change to "
                         f"what reads text, and a definition that reads none of it belongs to the "
                         f"rules or does not belong here")

    def test_no_local_module_the_reader_imports_supplies_reading_uncounted(self):
        found = local_chain(HERE)
        self.assertEqual(found, sorted(LOCAL_IMPORTS),
                         f"the reader reaches these local modules through imports: {found} — "
                         f"reading behind an import is where the path would otherwise stop, so "
                         f"each is named in LOCAL_IMPORTS and its reading then counts in "
                         f"READER_LINES like the reader's own")


class OutsideThePath(unittest.TestCase):
    """What the reading path does not hold: a definition that reads text, or reaches the path,
    and that neither the rules read nor their own reach — no case, and no number, would say so.
    """

    def test_nothing_outside_the_path_reads_text_or_reaches_it(self):
        used = rules_read(HERE)
        flagged = outside(TOOLCHAINS.read_text(), used)
        self.assertEqual(flagged, {},
                         f"these definitions are neither the reading path nor the rules' own, and "
                         f"each reads text or reaches the path: {flagged}")

    def test_every_operation_outside_the_path_is_a_named_one(self):
        found = operations(TOOLCHAINS.read_text())
        called = {op for ops in found.values() for op in ops}
        undeclared = sorted(called ^ NOT_READING)
        callers = {op: sorted(n for n, ops in found.items() if op in ops) for op in undeclared}
        self.assertEqual(undeclared, [],
                         f"these operations are called by definitions the reading path does not "
                         f"hold and the classifier does not name: {callers} — each belongs in "
                         f"the classifier as reading or here as named, and a name nothing calls "
                         f"comes out")

    def test_a_definition_that_reads_text_joins_the_path(self):
        source = ('import re\n\n\nPATTERN = re.compile("")\n\n\n'
                  "def spell(block: str) -> str:\n"
                  "    return PATTERN.sub('', block)\n\n\n"
                  "def helper(value: str) -> str:\n    return PATTERN.sub(value)\n")
        held = reading(source)
        self.assertIn("spell", held, "the definition that reads text is the path's")
        self.assertIn("PATTERN", held, "and so is the pattern it reads")
        self.assertIn("helper", held, "and a definition that reads its own text joins it")

    def test_a_definition_that_reaches_the_path_is_named_unless_the_rules_read_it(self):
        spell = ("def spell(block: str) -> str:\n"
                 "    return block.splitlines()[0]\n\n\n")
        reaching = spell + "def helper(value: str) -> str:\n    return spell(value)\n"
        self.assertEqual(reading(reaching), {"spell": 2},
                         "`helper` reads no text and is not reached from the path")
        self.assertEqual(outside(reaching, used=set()), {"helper": ["spell"]},
                         "so a definition that reaches the path and is not the rules' is named")
        self.assertEqual(outside(reaching, used={"helper"}), {},
                         "unless the rules read it")
        under_rules = (reaching + "\n\ndef chosen(value: str) -> str:\n"
                                  "    return helper(value)\n")
        self.assertEqual(outside(under_rules, used={"chosen"}), {},
                         "and a definition the rules' own reach is the rules' to hold")


class GuardSize(unittest.TestCase):
    """The guard's own size, declared the way the path's is: growing this file without moving the
    declaration reds here, and no edit outside this file can.
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
    mechanism reds here alone.
    """

    def test_every_definition_kind_the_path_reaches_is_counted(self):
        for name, definition, use in COUNTED:
            with self.subTest(kind=name):
                held = reading(small_module(definition, use))
                self.assertIn(name, held, f"{name} is a kind the path reaches: {sorted(held)}")

    def test_a_helper_reached_only_as_a_string_is_not_counted(self):
        reached = "def helper(value: str) -> str:\n    return value\n"
        held = reading(small_module(reached, 'globals()["helper"](block)'))
        self.assertNotIn("helper", held, f"a string is not a binding the path reads: {sorted(held)}")

    def test_a_helper_reached_through_an_import_is_not_counted(self):
        behind = "def tail(value: str) -> str:\n    return value\n"
        held = reading(small_module("import spelling\n" + behind, "spelling.tail(block)"))
        self.assertEqual(held.get("spelling"), 1, f"`import spelling` is one line: {held}")
        self.assertNotIn("tail", held, f"not the logic an imported name reaches: {sorted(held)}")

    def test_a_definition_costs_its_decorator_too(self):
        decorated = "@staticmethod\ndef helper(value: str) -> str:\n    return value\n"
        held = reading(small_module(decorated, "helper(block)"))
        self.assertEqual(held.get("helper"), 3, f"the decorator line is part of what helper "
                                               f"costs: {held}")

    def test_a_name_bound_without_an_assignment_is_not_counted(self):
        for name, definition, use in BOUND_ELSEWHERE:
            with self.subTest(binding=name):
                held = reading(small_module(definition, use))
                self.assertNotIn(name, held,
                                 f"{name} is bound by a module-level statement that binds no "
                                 f"assignment or import, so the path does not hold it: "
                                 f"{sorted(held)}")


if __name__ == "__main__":
    unittest.main(verbosity=2)
