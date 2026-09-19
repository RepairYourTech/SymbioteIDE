"""The interpreter this directory's tools need, derived both ways and owed by the jobs that run it.

Measured, not assumed: every module here parses at 3.8 and reaches exactly one version-gated
thing, `tomllib`, which is 3.11 — so the floor is 3.11 and that is its whole cause. The number is
declared once, in `python_floor.py`; this rule fails by name both for a module needing more than
it and for a declaration above what the directory needs, since a floor drifting upward without a
cause is the defect that is too low with the sign reversed.

A job owes it when its steps name this directory, or compile a crate whose own sources spawn one
of its tools — the crates read from `crates/` and the jobs from the workflows, neither listed. A
job that runs this directory and configures no interpreter is what failed in CI: it inherits the
runner image, which was 3.10 on `ubuntu-22.04`, and the checker died as `No module named
'tomllib'` where a refusal naming the floor belongs.

Stated, not implied, what this does not see: the vocabulary below is a statement rather than a
derivation, so a gated module or name it does not carry is invisible; `feature_version` gates
syntax only, so an API gate written in older syntax is caught only where the vocabulary names the
API; and jobs are read as this repository's indentation, not as YAML.
"""
from __future__ import annotations

import ast
import pathlib
import re
import unittest

import python_floor

HERE = pathlib.Path(__file__).resolve().parent
ROOT = HERE.parents[1]
WORKFLOWS = ROOT / ".github" / "workflows"
CRATES = ROOT / "crates"

# What a version-gated feature is, as far as this rule can tell: the global names the standard
# library added, and the version that first had them. `tomllib` is the one this directory uses.
GATED = {
    "tomllib": (3, 11),
    "graphlib": (3, 9),
    "zoneinfo": (3, 9),
    "importlib.metadata": (3, 8),
    "typing.Self": (3, 11),
    "typing.Never": (3, 11),
    "typing.TypeAliasType": (3, 12),
    "enum.StrEnum": (3, 11),
    "datetime.UTC": (3, 11),
    "itertools.batched": (3, 12),
}
# The versions `ast.parse` can be asked about, oldest first: the syntax oracle.
CANDIDATES = (8, 9, 10, 11, 12, 13)

JOB = re.compile(r"^  ([A-Za-z0-9_.\-]+):\s*$")
VERSION = re.compile(r"""^\s*python-version:\s*['"]?([0-9]+)\.([0-9]+)['"]?\s*$""")
COMMENT = re.compile(r"^\s*#")
# A crate whose tests spawn a tool here: a Rust source naming this directory and starting python.
SPAWN = re.compile(r"planning/integrity/")
PYTHON = re.compile(r"""Command::new\(\s*"python""")


def gated(source: str) -> dict[str, tuple[int, int]]:
    """Every feature in `GATED` this text uses, by the name it is written with."""
    found: dict[str, tuple[int, int]] = {}
    for node in ast.walk(ast.parse(source)):
        keys: list[str] = []
        if isinstance(node, ast.Import):
            keys = [alias.name for alias in node.names]
        elif isinstance(node, ast.ImportFrom) and node.module:
            keys = [node.module, *(f"{node.module}.{alias.name}" for alias in node.names)]
        elif isinstance(node, ast.Attribute) and isinstance(node.value, ast.Name):
            keys = [f"{node.value.id}.{node.attr}"]
        for key in keys:
            if key in GATED:
                found[key] = GATED[key]
    return found


def own(path: pathlib.Path) -> tuple[int, int]:
    """The oldest interpreter `path` itself can run on: its newest gated feature, raised to the
    oldest version its syntax parses at where that is newer.
    """
    text = path.read_text()
    version = max(gated(text).values(), default=(3, 0))
    for minor in CANDIDATES:
        try:
            ast.parse(text, feature_version=minor)
        except SyntaxError:
            continue
        return max(version, (3, minor))
    return (3, CANDIDATES[-1] + 1)


def imported(source: str) -> list[str]:
    """Every module this text imports, each name's first part."""
    found: list[str] = []
    for node in ast.walk(ast.parse(source)):
        found += ([alias.name for alias in node.names] if isinstance(node, ast.Import)
                  else [node.module] if isinstance(node, ast.ImportFrom) and node.module else [])
    return [name.split(".")[0] for name in found]


def closure(path: pathlib.Path, seen: set[str] | None = None) -> list[pathlib.Path]:
    """`path` and every module beside it it imports, transitively."""
    seen = {path.stem} if seen is None else seen
    found = [path]
    for name in imported(path.read_text()):
        beside = HERE / f"{name}.py"
        if beside.is_file() and name not in seen:
            seen.add(name)
            found += closure(beside, seen)
    return found


def needs(path: pathlib.Path) -> tuple[int, int]:
    """The oldest interpreter that can run `path`, the modules it imports included: a script is no
    older than what it reaches, which is how `source_record.py` owes `tomllib` without naming it.
    """
    return max(own(module) for module in closure(path))


def modules() -> list[pathlib.Path]:
    """Every module here — the rule files too, since they run on the same interpreter."""
    return sorted(HERE.glob("*.py"))


def spawning() -> list[str]:
    """The crates whose own sources spawn a tool in this directory, read from them rather than
    listed: a crate that runs one owes the interpreter, and no workflow has to say so.
    """
    found = []
    for crate in sorted(crate for crate in CRATES.iterdir() if crate.is_dir()):
        for source in sorted(crate.rglob("*.rs")):
            text = source.read_text(errors="replace")
            if SPAWN.search(text) and PYTHON.search(text):
                found.append(crate.name)
                break
    return found


def jobs(workflow: pathlib.Path) -> dict[str, list[str]]:
    """Each job of a workflow as the lines of its steps, comments dropped so a note about this
    directory is not read as a command that runs it, and continuations joined so a command written
    over several lines is one string.
    """
    found: dict[str, list[str]] = {}
    current, named = None, False
    for line in workflow.read_text(errors="replace").splitlines():
        if re.match(r"^jobs:\s*$", line):
            named = True
            continue
        if named and line[:1] not in (" ", ""):
            named = False
        if not named:
            continue
        job = JOB.match(line)
        if job:
            current = job.group(1)
            found[current] = []
        elif current is not None and not COMMENT.match(line):
            found[current].append(line.rstrip().removesuffix("\\").strip())
    return found


def builds(text: str, owed: list[str]) -> bool:
    """Whether a cargo invocation in this text would compile a crate that spawns a tool here: the
    whole workspace, or that crate named on its own.
    """
    if "cargo" not in text:
        return False
    # `--all` alone, not the `--all-targets` and `--all-features` a clippy run carries.
    if "--workspace" in text or re.search(r"--all(?![\w-])", text):
        return True
    return any(re.search(rf"(?:-p|--package)\s+{re.escape(crate)}\b", text)
               or re.search(rf"--manifest-path\s+(?:\./)?crates/{re.escape(crate)}/", text)
               for crate in owed)


class TheInterpreterTheCodeNeeds(unittest.TestCase):
    def test_no_module_here_needs_more_than_the_floor(self):
        """A module reaching a newer feature than `python_floor.FLOOR` states: the jobs are held to
        the declaration, so a requirement above it means a job could pass that cannot run the code.
        """
        over = {path.name: needs(path) for path in modules() if needs(path) > python_floor.FLOOR}
        self.assertEqual(over, {},
                         f"these modules need more than the declared "
                         f"{python_floor.FLOOR[0]}.{python_floor.FLOOR[1]}")

    def test_the_floor_is_not_above_what_this_directory_needs(self):
        """The other direction: a floor above the newest thing any module reaches is a number no
        cause holds, and every job is then constrained to an interpreter this code does not need.
        """
        required = max(needs(path) for path in modules())
        self.assertGreaterEqual(required, python_floor.FLOOR,
                                f"the declaration is {python_floor.FLOOR[0]}.{python_floor.FLOOR[1]}"
                                f" and the newest feature reached here is "
                                f"{required[0]}.{required[1]}")

    def test_every_script_that_reaches_a_gated_feature_states_the_floor(self):
        """So a lower interpreter is told the number and the cause rather than dying as `No module
        named 'tomllib'` — the failure CI reported. Derived from the import graph, and required of
        the entry points only: a rule file that imports the reader is refused by it in turn.
        """
        silent = []
        for path in modules():
            if path.name.startswith("test_") or "python_floor" in imported(path.read_text()):
                continue
            reaches = [module.name for module in closure(path) if gated(module.read_text())]
            if reaches:
                silent.append(f"{path.name} reaches {', '.join(sorted(reaches))}")
        self.assertEqual(silent, [],
                         "these scripts reach a gated feature without stating the floor, so a "
                         "lower interpreter is an ImportError instead of a refusal by name")

    def test_a_lower_interpreter_is_refused_with_the_floor_and_the_cause(self):
        """The refusal itself, driven rather than read: the floor, the cause, and the version it
        was given — and nothing said to an interpreter at or above it.
        """
        for minor in range(6, python_floor.FLOOR[1]):
            said = python_floor.refuse((python_floor.FLOOR[0], minor))
            self.assertIn(f"{python_floor.FLOOR[0]}.{python_floor.FLOOR[1]}", said)
            self.assertIn(python_floor.WHY, said)
            self.assertIn(f"{python_floor.FLOOR[0]}.{minor}", said)
        self.assertIsNone(python_floor.refuse(python_floor.FLOOR))
        self.assertIsNone(python_floor.refuse((python_floor.FLOOR[0], python_floor.FLOOR[1] + 1)))


class TheInterpreterTheJobsProvide(unittest.TestCase):
    def test_every_job_that_runs_this_directory_provides_the_interpreter_it_needs(self):
        """Derived from the workflows and from the crates that spawn these tools: a job naming this
        directory, or compiling one of those crates, must configure the floor or above. A job that
        configures nothing inherits the runner image, which is how the checker came to run on 3.10.
        """
        owed = spawning()
        offenders = []
        for workflow in sorted(WORKFLOWS.glob("*.yml")):
            for job, lines in jobs(workflow).items():
                text = " ".join(lines)
                if "planning/integrity" not in text and not builds(text, owed):
                    continue
                versions = [tuple(int(part) for part in version)
                            for line in lines for version in re.findall(VERSION, line)]
                if not versions:
                    offenders.append(f"{workflow.name}:{job} configures no interpreter, so it "
                                     f"inherits the runner's")
                offenders += [f"{workflow.name}:{job} configures {major}.{minor}"
                              for major, minor in versions if (major, minor) < python_floor.FLOOR]
        self.assertEqual(offenders, [],
                         "these jobs run this directory's tools on an interpreter below "
                         f"{python_floor.FLOOR[0]}.{python_floor.FLOOR[1]}")
