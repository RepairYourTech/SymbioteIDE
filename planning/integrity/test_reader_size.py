"""Run: python3 planning/integrity/test_reader_size.py.

`toolchains.py` is the reader the two toolchain rules share: this holds the reading path — every
module-level definition that reads text and all it reaches — against an equal declaration, because
the path grows as it learns a spelling, a reviewed act. This file's size is the opposite: `GUARD_CAP`
cannot rise above the guard `origin/main` holds, so it comes down. Reading is *acquiring* text, not
rearranging what the path states, which is why `legs` and the names the rules read that reach nothing
in it sit outside, free to grow (measured: `workspace_jobs`, `floor`, `declared`, `version`,
`workspace_of`). The measure is lines, so a guarded definition costs its own lines, not the one above it.
"""
from __future__ import annotations

import ast
import pathlib
import re
import subprocess
import sys
import unittest

HERE = pathlib.Path(__file__).resolve().parent
TOOLCHAINS = HERE / "toolchains.py"
RULES = ("test_toolchain_floors.py", "test_handoff.py")
# The reading path's size; this guard's own cap, which may not rise above the guard `main` already
# holds; and the cases a loader finds here.
READER_LINES = 450
GUARD_CAP = 365
GUARD_CASES = 13
# The repo's own modules whose reading counts with the reader's, named rather than followed quietly:
# `python_floor` reads no text, so the 450 stands.
LOCAL_IMPORTS: tuple[str, ...] = ("python_floor",)
# What reading text is, as a shape: these module names, whose operations read it, and the calls
# that turn a file or a block into lines. A definition holding one of these reads text.
TEXT_MODULES = frozenset({"re", "tomllib"})
TEXT_CALLS = frozenset({"read_text", "readlines", "read", "splitlines"})
# Every operation this module calls in a definition the path does not hold, named so a new one reds
# the case below rather than hiding: `split`/`join`/`strip`/`count` rearrange a value the path
# stated, the next six touch no text, and the builtins stand for the same claim as a method.
NOT_READING = frozenset({"split", "join", "strip", "count", "get", "values", "fromkeys",
                         "append", "is_file", "resolve", "any()", "isinstance()", "list()",
                         "next()", "range()", "AssertionError()"})


def definitions(tree: ast.Module) -> dict[str, ast.AST]:
    """Every name a module-level statement binds, whichever statement writes it — a definition spans
    its own lines, and no name inside a body counts."""
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
        for child in ast.walk(defined[name]):
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
    those definitions reach."""
    defined = definitions(ast.parse(source))
    named = patterns(defined)
    seeds = {name for name, node in defined.items() if reads(node, named)}
    found: dict[str, int] = {}
    for name in reachable(defined, seeds):
        node = defined[name]
        start = min([d.lineno for d in getattr(node, "decorator_list", [])] or [node.lineno])
        found[name] = node.end_lineno - start + 1
    return found


def local_chain(folder: pathlib.Path) -> list[str]:
    """Every repo module the reader reaches through local imports, `toolchains` excluded — a file
    beside it, never a package path or a stdlib name, so this cannot wander outside the tree."""
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
    """The reader's subject: its own reading path, and the reading of every declared local module it
    imports, keyed `module.name` — a helper the reader calls counts where it lives."""
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
    """Definitions that are neither the reading path nor the rules' own — what the rules read, and
    what that reaches before the path takes over — each with what it reads or reaches in the path."""
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
    classifier does not name; a call to a name this module binds is not an operation."""
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
# assignment or an import.
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

    def test_the_readme_states_the_two_numbers_this_rule_declares(self):
        stated = re.search(r"(\d+) lines of the module's (\d+)", (HERE / "README.md").read_text())
        self.assertIsNotNone(stated, "the README no longer states the reading path's size and the "
                                     "module's own, so this case holds nothing")
        path_lines, module_lines = (int(number) for number in stated.groups())
        self.assertEqual(path_lines, READER_LINES,
                         f"the README states {path_lines} lines of reading path where "
                         f"{READER_LINES} is declared, and this rule is what declares it")
        actual = len(TOOLCHAINS.read_text().splitlines())
        self.assertEqual(module_lines, actual,
                         f"the README states the module is {module_lines} lines and it is "
                         f"{actual}: a line added to the reader moves this figure in the change "
                         f"that adds it, the way READER_LINES moves")

    def test_no_local_module_the_reader_imports_supplies_reading_uncounted(self):
        found = local_chain(HERE)
        self.assertEqual(found, sorted(LOCAL_IMPORTS),
                         f"the reader reaches these local modules through imports: {found} — "
                         f"reading behind an import is where the path would otherwise stop, so "
                         f"each is named in LOCAL_IMPORTS and its reading then counts in "
                         f"READER_LINES like the reader's own")


class OutsideThePath(unittest.TestCase):
    """What the path does not hold and no case would otherwise name."""

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
    def test_the_guard_is_within_the_cap_it_may_not_raise(self):
        lines = len(pathlib.Path(__file__).read_text().splitlines())
        cases = unittest.defaultTestLoader.loadTestsFromModule(sys.modules[__name__]).countTestCases()
        done = subprocess.run(["git", "show", f"origin/main:./{pathlib.Path(__file__).name}"],
                              cwd=HERE, capture_output=True, text=True)
        self.assertTrue(done.returncode == 0, f"the merged guard is unreadable ({done.stderr.strip()}), "
                                              f"so the cap cannot be held: fetch the base branch here")
        merged = len(done.stdout.splitlines())
        self.assertEqual(lines, GUARD_CAP,
                         f"this guard is {lines} lines where {GUARD_CAP} is declared: the cap is "
                         f"this file's own size, and it moves nowhere but down")
        self.assertLessEqual(GUARD_CAP, merged,
                             f"the cap is {GUARD_CAP} where the guard `origin/main` holds is "
                             f"{merged}: a size larger than the merged guard is refused here")
        self.assertEqual(cases, GUARD_CASES, f"a loader finds {cases} cases, {GUARD_CASES} declared")


class CountedKinds(unittest.TestCase):
    """The kinds the derivation counts, and the bindings it does not, driven over small modules."""

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
