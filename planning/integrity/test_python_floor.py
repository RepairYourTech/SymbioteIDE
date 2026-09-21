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

The same derivation holds the chain that runs this directory's proof: the job that runs the suite
must run the rule driver beside it and fetch the history both read — the reader-size guard holds
nothing if the driver that refuses a grown guard never runs, and its refusal is shown against an
earlier guard only when the checkout holds one. It owes every suite this repository's checks
consist of as well, the competitor registry's `planning/research` among them, because a suite the
job does not run is a refusal CI does not enforce. A further link is that those steps' failures
reach the job at all: a step made non-fatal, given a condition that cannot hold, or written so
another command owns its exit status would leave CI green with the cap exceeded. The reading is
`chain.py`, which the case below and the driver's ways both use, so which step runs these checks has
one owner; the `HOLDS` rows that hold this case in turn, since a case cannot hold its own presence,
are `revert_rules.py`'s.

Stated with their figures, what this cannot see — each a derivation reading *statements* where the
answer would take running the effect, which is why no case here closes them:

* the vocabulary below is a statement, not a derivation: a gated name it does not carry is
  invisible;
* `feature_version` gates syntax, and not all of it — measured, it rejects `match` (3.10),
  `except*` (3.11) and `type X = …` (3.12), and **accepts** `f"{"a"}"` at 11, so a PEP 701
  nested-quote f-string (3.12) is invisible;
* a name reached as a string is found where a call states it (`import_module("tomllib")`,
  `getattr(module, "batched")`); bound to a variable first and passed on, it is not — measured,
  `m = "tomllib"` then `import_module(m)` reports nothing, since following a value means running it;
* a module **outside** this directory reached through `sys.path` that needs a newer Python —
  measured, a sibling needing 3.12 left this suite at 232 OK, since `closure()` follows the files
  beside this one and following a run-time path means running it;
* a step replaced by a **composite action** or reusable workflow — measured, nothing reds, because
  the marking reads the steps a job states; and a crate path assembled at run time, though an
  absolute path or one held in a variable is still found;
* jobs are read as the indentation shape of a `jobs:` block rather than as YAML, with the block's
  own column read from the workflow and a job key allowed a comment or an anchor — what that leaves
  is a whole `jobs:` block written as a **flow mapping** (`jobs: {checks: {…}}`), which is not read at
  all — measured, a workflow written that way whose step runs this directory's suite with no driver
  left this suite at 232 OK. A job whose `steps:` is a flow list is read and refused by the
  interpreter rule above, and tab indentation is a text no YAML parser accepts rather than a spelling
  that hides a job.
"""
from __future__ import annotations

import ast
import difflib
import pathlib
import posixpath
import re
import tempfile
import unittest

import chain
import python_floor
from chain import (BAKEOFF_LINK, DRIVER_LINK, FATAL_LINK, HISTORY_LINK, PARITY_LINK, POLICY_LINK,
                   REGISTRY_LINK, RELEASE_LINK, WORKFLOWS, jobs, links_missing, runs_the_suite)

HERE = pathlib.Path(__file__).resolve().parent
ROOT = HERE.parents[1]
CRATES = ROOT / "crates"
# The workflow the chain is read in, mutated below to drive the reading on states the live tree does
# not hold: this repository's own job, named once.
CHAIN_WORKFLOW = WORKFLOWS / "roadmap-integrity.yml"
# Which link each state the driver writes must leave the reading reporting, so a reading vacated into
# finding nothing fails here rather than leaving the assertions above satisfied by a whole tree.
STATE_LINKS = ((chain.NO_DRIVER, DRIVER_LINK), (chain.NO_HISTORY, HISTORY_LINK),
               (chain.NO_SUITE, REGISTRY_LINK), (chain.NO_SUITE, PARITY_LINK),
               (chain.NO_SUITE, POLICY_LINK),
               (chain.NO_SUITE, RELEASE_LINK),
               (chain.NO_SUITE, BAKEOFF_LINK),
               (chain.NON_FATAL, FATAL_LINK), (chain.CONDITIONAL, FATAL_LINK),
               (chain.SWALLOWED, FATAL_LINK), (chain.JOB_CONDITIONAL, FATAL_LINK))

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
# The bare names of the gated attributes above, which is how `getattr(module, "name")` spells one:
# a string exactly equal to one of these reaches the feature its qualified name is listed under.
MEMBERS = {feature.split(".")[-1]: feature for feature in GATED if "." in feature}

VERSION = re.compile(r"""^\s*python-version:\s*['"]?([0-9]+)\.([0-9]+)['"]?\s*$""")
COMMENT = re.compile(r"^\s*#")
# A crate whose tests spawn a tool here: a Rust source naming this directory and starting python.
SPAWN = re.compile(r"planning/integrity/")
PYTHON = re.compile(r"""Command::new\(\s*"python""")


def gated(source: str) -> dict[str, tuple[int, int]]:
    """Every feature in `GATED` this text reaches, and how it reaches it.

    A statement that names the feature is the ordinary way; a **string** that spells it is the
    other, and it needs the same interpreter. Leaving the second out is how a cause goes unseen:
    measured, `tomllib = importlib.import_module("tomllib")` dropped the requirement to the syntax
    floor, which made the declaration look too high and let a lowered floor silence it.
    """
    found: dict[str, tuple[tuple[int, int], str]] = {}
    for node in ast.walk(ast.parse(source)):
        keys: list[tuple[str, str]] = []
        if isinstance(node, ast.Import):
            keys = [(alias.name, "import") for alias in node.names]
        elif isinstance(node, ast.ImportFrom) and node.module:
            keys = [(node.module, "import"),
                    *((f"{node.module}.{alias.name}", "import") for alias in node.names)]
        elif isinstance(node, ast.Attribute) and isinstance(node.value, ast.Name):
            keys = [(f"{node.value.id}.{node.attr}", "attribute")]
        elif isinstance(node, ast.Call):
            # A string the code reaches *with*: `importlib.import_module("tomllib")`, or the same
            # through `getattr`. Not a string held as data — this rule's own vocabulary is written
            # as string keys, and reading those as uses made it report itself.
            given = [*node.args, *(keyword.value for keyword in node.keywords)]
            keys = [(value.value.strip(), "name") for value in given
                    if isinstance(value, ast.Constant) and isinstance(value.value, str)]
        for key, how in keys:
            feature = key if key in GATED else MEMBERS.get(key)
            if feature is not None:
                found[feature] = (GATED[feature], how)
    return found


def features(path: pathlib.Path) -> list[str]:
    """What this module's requirement is made of, its imports included, each with the version it
    needs, how it was reached and where: the sentence a failure says instead of a bare number.
    """
    found = {}
    for module in closure(path):
        for feature, (version, how) in gated(module.read_text()).items():
            found[feature] = (version, how, module.stem)
    return sorted(f"{feature} needs {version[0]}.{version[1]}, reached by {how} in "
                  f"{where}.py" for feature, (version, how, where) in found.items())


def own(path: pathlib.Path) -> tuple[int, int]:
    """The oldest interpreter `path` itself can run on: its newest gated feature, raised to the
    oldest version its syntax parses at where that is newer.
    """
    text = path.read_text()
    version = max((needed for needed, _ in gated(text).values()), default=(3, 0))
    for minor in CANDIDATES:
        try:
            ast.parse(text, feature_version=minor)
        except SyntaxError:
            continue
        return max(version, (3, minor))
    return (3, CANDIDATES[-1] + 1)


def parses_at(text: str) -> int | None:
    """The oldest syntax version `text` parses at, or None where it parses at none of CANDIDATES."""
    for minor in CANDIDATES:
        try:
            ast.parse(text, feature_version=minor)
        except SyntaxError:
            continue
        return minor
    return None


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
        over = {path.name: features(path) for path in modules()
                if needs(path) > python_floor.FLOOR}
        self.assertEqual(over, {},
                         f"these modules need more than the declared "
                         f"{python_floor.FLOOR[0]}.{python_floor.FLOOR[1]}, and it is what they "
                         f"reach that says so: {over}")

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

    def test_a_gated_feature_reached_without_a_statement_is_named_as_such(self):
        """Driven over module texts written here: a feature reached by name is the same
        requirement as one imported, so it is found and reported with its version and how it was
        reached — the sentence that makes a lowered declaration fail instead of going quiet.
        """
        for text, feature in (('importlib.import_module("tomllib")\n', "tomllib"),
                              ('getattr(itertools, "batched")\n', "itertools.batched")):
            with self.subTest(reached=text.strip()):
                self.assertEqual(gated(text).get(feature), (GATED[feature], "name"),
                                 "a feature reached as a name is reached")

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


class TheChainThatRunsTheseChecks(unittest.TestCase):
    """The links between the reader-size guard's hold and the workflow that runs it.

    A hold nothing runs is not a hold: the job that runs this directory's suite must run the driver
    too — two halves of one proof over one checkout — and must fetch the history both read, because
    the cap case reads the tip a push names and the driver an earlier guard.    It owes every suite
    this repository's checks consist of as well, each row of `chain.SUITES` beside this one — because
    a suite the job does not run is a refusal CI does not enforce, and each of those suites is the
    only thing refusing what it refuses. That list is read from the table rather than repeated here,
    so a suite added beside this one is owed without a sentence to keep in step. Read as the commands a job's steps run, the way the interpreter rule above reads them,
    not as YAML and not as one spelling: whichever way a job writes the discovery, it is the job that
    runs the check, and this case names it by failing rather than by matching its command. Two spellings are deliberately
    green, and the refusal is no wider than they are:

      - a condition that *can* hold (`if: ${{ github.event_name == 'push' }}`), because a condition
        this job satisfies is not one the request named and deciding a non-constant one means
        evaluating it — what that leaves is answered by reading the workflow, not by this case; and
      - a condition or a `continue-on-error` on a step that runs none of these checks (a checkout
        behind `if: always()`), which says nothing about whether the check's failure reaches the job.
    """

    def test_the_job_that_runs_these_checks_also_runs_the_driver_over_full_history(self):
        ran, lacking = [], {link: [] for _directory, link in chain.SUITES}
        lacking.update({link: [] for link in (DRIVER_LINK, HISTORY_LINK, FATAL_LINK)})
        for workflow in sorted(WORKFLOWS.glob("*.yml")):
            for job, lines in jobs(workflow).items():
                if not runs_the_suite(lines):
                    continue
                named = f"{workflow.name}:{job}"
                ran.append(named)
                for link in links_missing(lines):
                    lacking[link].append(named)
        self.assertTrue(ran, f"no job runs this directory's unittest discovery, so the "
                             f"reader-size guard's hold is run by nothing CI runs")
        for _directory, link in chain.SUITES:
            self.assertEqual(lacking[link], [],
                             f"these jobs run this directory's suite without {link}, so a suite "
                             f"this repository's checks consist of can be dropped from CI by one "
                             f"edit nothing sees: {lacking[link]}")
        self.assertEqual(lacking[DRIVER_LINK], [],
                         "these jobs run the suite but not the rule driver, whose `HOLDS` row is "
                         f"the only check that refuses a guard grown past its cap in both "
                         f"directions: {lacking[DRIVER_LINK]}")
        self.assertEqual(lacking[HISTORY_LINK], [],
                         "these jobs run those checks without the history they read — the cap case "
                         f"reads the tip a push names and the driver an earlier guard: "
                         f"{lacking[HISTORY_LINK]}")
        self.assertEqual(lacking[FATAL_LINK], [],
                         "these steps run the checks without their failure reaching the job, so a "
                         f"guard grown past the cap it declares leaves CI green: "
                         f"{lacking[FATAL_LINK]}")
        # And the table those expectations come from is held to what a row has to be: a directory
        # this tree carries, named once, because two rows on one directory let one suite's step
        # answer another's row — measured, re-pointing a row at a directory already named left this
        # case green with that suite's step deleted.
        directories = [directory for directory, _link in chain.SUITES]
        self.assertEqual(sorted(directories), sorted(set(directories)),
                         f"two rows of the table name one directory, so one suite's step can "
                         f"answer another suite's row: {directories}")
        for directory in directories:
            self.assertTrue((ROOT / directory).is_dir(),
                            f"the table names {directory!r}, which this tree does not carry, so "
                            f"the row is owed by a job that cannot run it")
        # And the expectation table the drive below reads is held in both directions, from sources
        # it does not state: the suites it expects under the state that takes a suite's step away
        # are the ones this repository's checks consist of, and the states it drives are the ones
        # the mutation names — so a pair taken out of it is a link the reading is never asked
        # about, or a state never shown, refused rather than narrowed away.
        suites = [(chain.NO_SUITE, link) for directory, link in chain.SUITES
                  if directory != chain.SUITE_DIRECTORY]
        expected = [pair for pair in STATE_LINKS if pair[0] == chain.NO_SUITE]
        self.assertCountEqual(expected, suites,
                              f"the suites the state {chain.NO_SUITE!r} is expected under, "
                              f"{expected}, are not the ones this repository's checks consist of, "
                              f"{suites}: a pair dropped or added here is a link the reading is "
                              f"never asked about")
        self.assertEqual(sorted({what for what, _link in STATE_LINKS}), sorted(chain.MUTATIONS),
                         f"the states this case drives are not the ones the mutation names, "
                         f"{list(chain.MUTATIONS)}: a state dropped or added here is a way the "
                         f"driver writes that the reading is never shown")
        # And the reading is driven on the states the driver's ways write: a reading that stopped
        # finding a link would leave the assertions above satisfied by a tree that happens to be
        # whole, which is a net going quiet rather than a rule holding.
        with tempfile.TemporaryDirectory() as where:
            written = pathlib.Path(where) / CHAIN_WORKFLOW.name
            for what, link in STATE_LINKS:
                written.write_text(chain.mutated(CHAIN_WORKFLOW.read_text(), what))
                found = [one for _job, lines in jobs(written).items() for one in links_missing(lines)]
                self.assertIn(link, found, f"a workflow in the state {what!r} is not read as lacking "
                                           f"{link}, so this reading cannot find it")

    def test_each_state_is_written_inside_the_definition_it_means(self):
        """A key written beside a step rather than among the step's own keys is a workflow no YAML
        parser reads, so the drive above would be showing a refusal on a text no workflow can be; and
        a key written into a step that runs none of these checks — or into none of them — would move
        a link the driver's ways never meant. Read from the mutated text itself, in the shapes a step
        and a job are written: one key per definition that runs these checks, each inside its
        definition and past that definition's own `-` or job key, which is where a parser reads a key
        rather than beside one.
        """
        live = CHAIN_WORKFLOW.read_text()
        lines = live.splitlines()
        check_jobs = [(begin, end) for _name, begin, end in chain.job_ranges(live)
                      if chain.runs_a_check([lines[each].strip()
                                             for each in range(begin + 1, end)])]
        check_steps = [one for one in chain.step_ranges(live)
                       if one[2] is not None
                       and chain.runs_a_check([lines[each].strip()
                                               for each in range(one[0], one[1])])]

        def inserted_positions(written):
            """The lines the mutation added, by position: a workflow that already writes the same
            text elsewhere would hide one from a membership test by text."""
            return [at for tag, _a, _b, first, last in
                    difflib.SequenceMatcher(a=lines, b=written, autojunk=False).get_opcodes()
                    if tag == "insert" for at in range(first, last)]

        for what in (chain.NON_FATAL, chain.CONDITIONAL, chain.JOB_CONDITIONAL):
            written = chain.mutated(live, what)
            mutated = written.splitlines()
            meant = check_jobs if what == chain.JOB_CONDITIONAL else check_steps
            self.assertTrue(meant, f"the live workflow runs these checks somewhere, so the state "
                                   f"{what!r} has a definition it must be written into")
            written_at = inserted_positions(mutated)
            self.assertEqual(len(written_at), len(meant),
                             f"the state {what!r} writes one key into each of the {len(meant)} "
                             f"definitions that run these checks, and it wrote {len(written_at)} "
                             f"into {[mutated[at].strip() for at in written_at]}")
            for at in written_at:
                key = mutated[at]
                column = len(key) - len(key.lstrip())
                if what == chain.JOB_CONDITIONAL:
                    in_job = [(begin, end) for _name, begin, end in chain.job_ranges(written)
                              if begin < at < end]
                    self.assertTrue(in_job, f"the state {what!r} wrote `{key.strip()}` outside every "
                                            f"job, where no parser reads a key")
                    begin, end = in_job[0]
                    self.assertTrue(chain.runs_a_check([mutated[each].strip()
                                                        for each in range(begin + 1, end)]),
                                    f"the state {what!r} wrote into a job that runs none of these "
                                    f"checks, so it moves a link the driver's ways never mean")
                    self.assertGreater(
                        column, len(mutated[begin]) - len(mutated[begin].lstrip()),
                        f"the state {what!r} wrote `{key.strip()}` at its job key's own column, "
                        f"where no parser reads it as one of the job's keys")
                    continue
                in_step = [(start, end) for start, end, _column in chain.step_ranges(written)
                           if start <= at < end]
                self.assertTrue(in_step, f"the state {what!r} wrote `{key.strip()}` outside every "
                                          f"step, where no parser reads a key")
                start, end = in_step[0]
                self.assertTrue(chain.runs_a_check([mutated[each].strip()
                                                    for each in range(start, end)]),
                                f"the state {what!r} wrote into a step that runs none of these "
                                f"checks: {key.strip()!r}")
                dash = len(mutated[start]) - len(mutated[start].lstrip())
                self.assertGreater(column, dash,
                                   f"the state {what!r} wrote `{key.strip()}` at its step's own "
                                   f"dash column, which is a text no YAML parser reads")

    def test_a_step_is_read_as_the_suite_it_names_and_as_no_other(self):
        """Which suite a step runs discovery in, driven from the rows themselves: for every row of
        `chain.SUITES`, a step that names that directory — to `-s`, with `./`, with a trailing slash,
        as a workspace path, positionally, or reached by a `cd` in the same command or on a line of
        its own up to a `-s .` inside it — is that row's step and no other row's, and a step that
        names something else is no row's step at all.

        Both families are written from the row's own directory and from the tree beside it rather
        than listed here: the other rows come from that one table, and a directory that only shares
        the last name comes from this repository's own directories. So a reader narrowed to the
        spelling one workflow happens to write, or widened to something that is merely not that
        directory — a prefix, a deeper path, a parent, or a directory of the same name elsewhere —
        fails here by name, rather than leaving a job invisible to every rule above it or read as a
        suite it does not run. What holds this case is the two families stated over the rows: empty
        either and it stops failing when the reading is weakened, which the row that names it refuses
        in turn. The same-last-name directories beside them are coverage the every-other-row family
        also carries — asserted here, and measured not to be what holds the row.
        """
        flat = "jobs:\n  checks:\n    runs-on: ubuntu-latest\n    steps:\n      %s\n"
        suites = [directory for directory, _link in chain.SUITES]
        twins = sorted(one.name for one in chain.ROOT.iterdir() if one.is_dir())

        with tempfile.TemporaryDirectory() as where:
            drafted = pathlib.Path(where) / "spelled.yml"

            def read_as(step):
                """The rows a step is read as, from the text written here."""
                drafted.write_text(flat % step)
                return sorted({directory for lines in jobs(drafted).values()
                               for directory in suites if chain.runs_suite(lines, directory)})

            def names(directory):
                """Every step that runs discovery in `directory`, written from the directory."""
                return (f"- run: python -m unittest discover -s {directory}",
                        f"- run: python -m unittest discover -s ./{directory}",
                        f"- run: python -m unittest discover -s {directory}/",
                        f"- run: python -m unittest discover -s "
                        f"${{{{ github.workspace }}}}/{directory}",
                        f"- run: python -m unittest discover {directory}",
                        f"- run: cd {directory} && python -m unittest discover",
                        f"- run: |\n          cd {directory}\n          python -m unittest discover",
                        f"- run: |\n          cd {directory}\n          python -m unittest "
                        f"discover -s .")

            def names_elsewhere(directory):
                """Steps that run discovery somewhere that is not `directory`, each with the row it
                is the step of where it is one: a parent, a prefix, a deeper path, a directory of
                the same name below it or beside it, another row of the same table, or `-t`.
                """
                parent, base = posixpath.dirname(directory), posixpath.basename(directory)
                elsewhere = [(f"- run: python -m unittest discover -s {parent}", None),
                             (f"- run: python -m unittest discover -s {directory}-extra", None),
                             (f"- run: python -m unittest discover -s {directory}/{base}", None),
                             ("- run: python -m unittest discover -s .", None),
                             (f"- run: python -m unittest discover -t {directory}", None)]
                elsewhere += [(f"- run: python -m unittest discover -s {other}", other)
                              for other in suites if other != directory]
                elsewhere += [(f"- run: python -m unittest discover -s {twin}/{base}",
                               f"{twin}/{base}" if f"{twin}/{base}" in suites else None)
                              for twin in twins if f"{twin}/{base}" != directory]
                return elsewhere

            for directory in suites:
                for step in names(directory):
                    read = read_as(step)
                    self.assertEqual(
                        read, [directory],
                        f"a step that runs one suite is that suite's step and no other's: a step "
                        f"naming {directory} was read as {read}, so a job that runs one of these "
                        f"checks can be read as running another — or as running none")
                for step, of_row in names_elsewhere(directory):
                    read = read_as(step)
                    want = [] if of_row is None else [of_row]
                    self.assertEqual(
                        read, want,
                        f"a step that names no row's directory is no row's step: {step!r} names "
                        f"something that is not {directory} and was read as {read} where {want} is "
                        f"what it names, so a reading that stopped comparing whole paths, or that "
                        f"narrowed to the last name alone, passes a job that runs elsewhere")

    def test_a_job_is_read_at_any_indentation_and_with_a_comment_or_anchor(self):
        """The `jobs:` block's own column is read from the workflow rather than assumed — two
        columns is what this repository writes, not what a job must use — and a job key may carry a
        comment or the anchor an alias elsewhere resolves to. Read from texts written here, so a
        derivation replaced by a constant reds here instead of leaving a job written that way
        invisible to every rule above it.
        """
        four = ("jobs:\n"
                "    checks:\n"
                "        runs-on: ubuntu-latest\n"
                "        steps:\n"
                "            - run: python -m unittest discover -s planning/integrity\n")
        marked = ("jobs:\n"
                  "  anchored: &a\n"
                  "    runs-on: ubuntu-latest\n"
                  "    steps:\n"
                  "      - run: python -m unittest discover -s planning/integrity\n"
                  "  commented: # the second\n"
                  "    runs-on: ubuntu-latest\n"
                  "    steps:\n"
                  "      - run: python -m unittest discover -s planning/integrity\n")
        # Every link a job that runs these checks owes, read off the one table that holds them
        # rather than restated: a suite added there is owed here without this case being edited.
        owed = [link for directory, link in chain.SUITES if directory != chain.SUITE_DIRECTORY]
        owed += [DRIVER_LINK, HISTORY_LINK]
        with tempfile.TemporaryDirectory() as where:
            for name, text, called in (("four.yml", four, ["checks"]),
                                       ("marked.yml", marked, ["anchored", "commented"])):
                path = pathlib.Path(where) / name
                path.write_text(text)
                read = jobs(path)
                self.assertEqual(sorted(read), called,
                                 f"{name} writes {called} and this reading found {sorted(read)}: "
                                 f"a job written that way must be read like any other")
                for job, job_lines in read.items():
                    self.assertEqual(sorted(links_missing(job_lines)), sorted(owed),
                                     f"{name}:{job} runs this directory's suite with no driver, no "
                                     f"history and no suite beside it, so it must be read as lacking "
                                     f"every link a job that runs these checks owes rather than "
                                     f"invisible: {sorted(links_missing(job_lines))}")


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

class ReadmeInterpreter(unittest.TestCase):
    """The README states the floor and what every module here parses at, so both are measured here
    rather than restated: the declared number is the one `python_floor` owns, and the syntax figure
    is the newest floor any module in this directory needs.
    """

    def test_the_readmes_interpreter_figures_are_this_directories(self):
        readme = (HERE / "README.md").read_text()
        declared = re.search(r"declared once in `python_floor\.py`: \*\*(\d+\.\d+)\*\*", readme)
        self.assertIsNotNone(declared, "the README no longer states the floor this directory declares")
        floor = f"{python_floor.FLOOR[0]}.{python_floor.FLOOR[1]}"
        self.assertEqual(declared.group(1), floor,
                         f"the README states {declared.group(1)} where `python_floor` declares {floor}")
        stated = re.search(r"every module here parses at (\d+)\.(\d+)", readme)
        self.assertIsNotNone(stated, "the README no longer states what every module here parses at")
        floors = {module.name: parses_at(module.read_text()) for module in modules()}
        self.assertNotIn(None, floors.values(),
                         f"these modules parse at none of {CANDIDATES}: "
                         f"{sorted(name for name, minor in floors.items() if minor is None)}")
        self.assertEqual(max(floors.values()), int(stated.group(2)),
                         f"the README states every module here parses at "
                         f"{stated.group(1)}.{stated.group(2)} and the newest syntax one needs is "
                         f"3.{max(floors.values())}: a module written in newer syntax moves this "
                         f"figure in the change that adds it")
