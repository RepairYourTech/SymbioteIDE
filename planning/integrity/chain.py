"""The chain that runs this directory's proof, read as the commands its steps run.

A hold nothing runs is not a hold. This directory's suite states the reader-size guard's cap;
`revert_rules.py` is what refuses a guard grown a line past it, and its refusal is shown against an
earlier guard, so its checkout must hold one. So the job that runs this directory's suite must run
the driver too — two halves of one proof over one checkout — and must fetch the full history both
read. `test_python_floor.py`'s chain case refuses every link of that, and the driver's `HOLDS` rows
hold that case.

This module is the reading both go through, so which step runs these checks has **one** owner: a job
is named for the commands its steps run, so `-s planning/integrity`, `-s ./planning/integrity`, the
directory positionally, a `cd` into it and a preceding `cd` line are one check. One predicate says
what a check step is (`runs_a_check`), one locator says where a step's own keys are written
(`step_ranges`, two columns past its dash however the dash is spelled), and the states the driver's
ways show a refusal by are written *through* both (`mutated`) — so a way cannot demand a spelling the
case accepts, and a step that runs none of these checks is never written into. A job may indent its
`jobs:` children any way YAML allows, and the column is read from the workflow rather than assumed,
so a job written otherwise is still the job that runs these checks.
"""
from __future__ import annotations

import pathlib
import posixpath
import re

HERE = pathlib.Path(__file__).resolve().parent
ROOT = HERE.parents[1]
WORKFLOWS = ROOT / ".github" / "workflows"

# The proof this directory's chain is made of: the job whose commands run this directory's own
# unittest discovery — the suite that declares the reader-size guard — the driver that refuses a
# guard grown past it, and the fetch that gives both the history they read.
SUITE_DIRECTORY = "planning/integrity"
DRIVER_SCRIPT = "revert_rules.py"
FULL_HISTORY = "fetch-depth: 0"
# The links a job that runs these checks owes, named once here so the clauses that find them, the
# assertions that refuse a job missing one, and the mutations below cannot drift apart.
DRIVER_LINK = "the rule driver beside the suite"
HISTORY_LINK = "the history both read"
FATAL_LINK = "a failure that reaches the job"
# How a workflow is written into the state of one link being gone, named once for the same reason:
# the driver's ways carry these names, and a name this file does not write is a way that cannot bite.
NO_DRIVER = "no driver"
NO_HISTORY = "no history"
NON_FATAL = "non-fatal"
CONDITIONAL = "conditional"
SWALLOWED = "swallowed"
MUTATIONS: tuple[str, ...] = (NO_DRIVER, NO_HISTORY, NON_FATAL, CONDITIONAL, SWALLOWED)

SEPARATOR = re.compile(r"&&|\|\||[;&|]")
# What a step's line starts with before its command does: `run:`, and the list dash a job may put in
# front of it. A step written `run: cd here && python -m unittest discover` is one line holding two
# commands, so the key has to come off before the first of them can be read as a command.
RUN_KEY = re.compile(r"^(?:-\s*)?run:\s*")
DISCOVER = re.compile(r"\bunittest\b.*\bdiscover\b")
CHDIR = re.compile(r"^cd\s+(.+)$")
# A directory can hold spaces (`${{ github.workspace }}/…`), so it runs to the next option or the
# end of the command rather than to the next blank.
START_DIRECTORY = re.compile(r"(?:-s|--start-directory)(?:\s+|=)([^-].*?)(?=\s+-|\s*$)")
# Options whose value is the token after them, so `-p 'test_*.py'` is not read as a directory.
VALUED = {"-p", "--pattern", "-k", "-t", "--top-level-directory"}
RUNS_DRIVER = re.compile(rf"(?:^|\s)(?:python3?\s+)?(?:\S*/)?{re.escape(DRIVER_SCRIPT)}(?:\s|$)")
JOBS_KEY = re.compile(r"^jobs:\s*$")
# The dash that begins a step, keeping its column: a step's own keys are written two columns past
# it, whether the dash carries the first key (`- run: …`) or stands alone (`-` then `run: …`), so
# one locator finds where a state written into a step belongs in every spelling of a step.
STEP = re.compile(r"^(?P<indent> *)-(?:\s|$)")
KEY_PAST_DASH = 2
# A step that runs a check can stop its failure reaching the job two ways, both read from a step's
# own keys: made non-fatal, or behind a condition that cannot hold. `false` is the one value of
# `continue-on-error` that keeps the step fatal, and a condition is read for the value no reading of
# the workflow can call a day the checks ran — `if: false` and its spellings. A condition that *can*
# hold is not refused: measured, `if: ${{ github.event_name == 'push' }}` is one a job can satisfy,
# and deciding a non-constant expression means evaluating it rather than reading it. What that leaves
# — a condition false on an event no reading can name — is stated in the chain case's own statement.
NONFATAL = re.compile(r"^(?:-\s*)?continue-on-error\s*:\s*(?!false\s*$)\S")
NEVER_RUNS = re.compile(r"^(?:-\s*)?if\s*:\s*(?:\$\{\{\s*)?(?:false|0)?\s*(?:\}\})?\s*$",
                        re.IGNORECASE)
# A command after the check that still fails the step, so `… || exit 1` swallows nothing.
STILL_FAILS = re.compile(r"^exit\s+[1-9][0-9]*$")
# A step's other keys, which are not commands and may follow its `run:` line.
KEY_LINE = re.compile(r"^[a-z][a-z0-9_-]*:\s")
COMMENT = re.compile(r"^\s*#")


def commands(lines: list[str]) -> list[str]:
    """Every command a job's step lines run, in order: the shell's separators read as what separates
    one command from the next, so a step that chains two is read as both.
    """
    found = []
    for line in lines:
        for command in SEPARATOR.split(line):
            command = RUN_KEY.sub("", command.strip()).strip()
            if command:
                found.append(command)
    return found


def named_directory(command: str) -> str | None:
    """The directory a `discover` command says to search: `-s`/`--start-directory`, or the positional
    argument unittest reads the same way. Options and their values are skipped, so `-p 'test_*.py'`
    is not read as a directory.
    """
    said = START_DIRECTORY.search(command)
    if said:
        return said.group(1)
    skip = False
    for token in command[command.index("discover") + len("discover"):].split():
        if token.startswith("#"):
            return None
        if skip:
            skip = False
        elif token.startswith("-"):
            skip = token in VALUED
        else:
            return token
    return None


def names_this_directory(where: str | None) -> bool:
    """Whether a path names this directory: the tracked path itself, or one that ends in it — a path
    built from the workspace variable is the same directory as the relative one.
    """
    if where is None:
        return False
    joined = posixpath.normpath(where)
    return joined == SUITE_DIRECTORY or joined.endswith(f"/{SUITE_DIRECTORY}")


def runs_the_suite(lines: list[str]) -> bool:
    """Whether these steps run unittest's discovery over *this* directory, in any spelling of it.

    `python -m unittest discover -s planning/integrity`, the same with `./` or a trailing slash, the
    directory given positionally, `cd planning/integrity && python -m unittest discover` and a `cd`
    on a line of its own before either are one check written five ways — discovery starts in the
    directory it runs in when nothing names one — so all five are read, and the hold is not bound to
    the spelling one workflow happens to write. A relative directory is resolved where the command
    runs, so `-s .` after a `cd` into this directory is this directory. A directory named to
    `-t`/`--top-level-directory` is not read: discovery would start at the top level and search wider
    than this directory.
    """
    directory = None
    for command in commands(lines):
        went = CHDIR.match(command)
        if went:
            directory = posixpath.normpath(went.group(1).split("#")[0].strip().strip("'\""))
            continue
        if not DISCOVER.search(command):
            continue
        named = named_directory(command)
        # `-s` names a directory relative to where the command runs, so `.` is the tracked one.
        where = posixpath.join(directory or ".", named) if named else directory
        if names_this_directory(where):
            return True
    return False


def runs_the_driver(lines: list[str]) -> bool:
    """Whether these steps run the rule driver, however the interpreter and path are spelled."""
    return any(RUNS_DRIVER.search(command) for command in commands(lines))


def command_lines(step: list[str]) -> list[tuple[int, str]]:
    """`commands`'s own reading of a step, keeping the line each command was written on."""
    return [(at, one) for at, line in enumerate(step) for one in commands([line])]


def check_reached(step: list[str]) -> int | None:
    """Where in a step's commands the check runs: the first one after which the step has run this
    directory's suite or the rule driver. This is the one owner of "the step that runs these
    checks" — `unfatal` refuses a failure this step throws away and `mutated` writes a state inside a
    step this names, so neither can mean a different step than the case reads.
    """
    ran = commands(step)
    return next((at for at in range(len(ran))
                 if runs_the_suite(ran[:at + 1]) or runs_the_driver(ran[:at + 1])), None)


def runs_a_check(step: list[str]) -> bool:
    """Whether these step lines run this directory's suite or the rule driver."""
    return check_reached(step) is not None


def job_ranges(text: str) -> list[tuple[str, int, int]]:
    """Each job's name and the line range of its own lines. One scan: `jobs()` strips these lines for
    the case and `step_ranges` locates the steps in them for the mutation, so both see the same jobs.

    The column the jobs are written at is read from the first line under `jobs:` rather than assumed
    two, so a workflow indented any way YAML allows is read: a job key is whatever sits at that
    column, and a key carrying a comment or the anchor an alias elsewhere resolves to is still the
    job (a key with a scalar after it, `name: value`, is not a job).
    """
    lines = text.splitlines()
    begin = next((at for at, line in enumerate(lines) if JOBS_KEY.match(line)), None)
    if begin is None:
        return []
    written = [child for child in lines[begin + 1:] if child.strip() and not COMMENT.match(child)]
    # A child of `jobs:` is indented past it; a block whose first line sits at column zero has no
    # children, and reading its own key as one would invent a job nothing wrote.
    if not written or not written[0].startswith(" "):
        return []
    column = len(written[0]) - len(written[0].lstrip())
    job = re.compile(rf"^ {{{column}}}(?P<name>[A-Za-z0-9_.\-]+):\s*(?:#.*|[&*][^\s]*)?$")
    found: list[tuple[str, int, int]] = []
    current, start = None, 0
    for at in range(begin + 1, len(lines)):
        line = lines[at]
        if line.strip() and (len(line) - len(line.lstrip())) < column:
            break
        key = job.match(line)
        if key:
            if current is not None:
                found.append((current, start, at))
            current, start = key.group("name"), at
    if current is not None:
        found.append((current, start, len(lines)))
    return found


def step_ranges(text: str) -> list[tuple[int, int, int]]:
    """Every step of a workflow as the lines it spans and the column its own keys are written at: a
    step begins where a line's own text begins with `-` at or above the previous step's dash, and
    ends where the next such dash begins, so a nested list inside a step is part of it. Read from
    the same job ranges the case reads, so a step the mutation writes into is a step the case sees.
    """
    lines = text.splitlines()
    found: list[tuple[int, int, int]] = []
    for _name, begin, end in job_ranges(text):
        opened, dash = None, None
        for at in range(begin + 1, end):
            began = STEP.match(lines[at])
            if began and (dash is None or len(began.group("indent")) <= dash):
                if opened is not None:
                    found.append((opened, at, dash + KEY_PAST_DASH))
                opened, dash = at, len(began.group("indent"))
        if opened is not None:
            found.append((opened, end, dash + KEY_PAST_DASH))
    return found


def jobs(workflow: pathlib.Path) -> dict[str, list[str]]:
    """Each job of a workflow as the lines of its steps, comments dropped so a note about this
    directory is not read as a command that runs it, and continuations joined so a command written
    over several lines is one string.
    """
    text = workflow.read_text(errors="replace")
    lines = text.splitlines()
    return {name: [lines[at].rstrip().removesuffix("\\").strip()
                   for at in range(begin + 1, end) if not COMMENT.match(lines[at])]
            for name, begin, end in job_ranges(text)}


def unfatal(lines: list[str]) -> list[str]:
    """The steps of a job that run these checks without a failure reaching the job.

    The cap the suite declares is enforced by that step failing, so a step made non-fatal, given a
    condition that cannot hold, or written so another command owns its exit status (`… || true`,
    `… ; echo done`) leaves the job green with the cap exceeded — the whole hold undone by one edit
    that no reading of the check itself can see. Read as the commands a step runs through the same
    predicate the mutation writes its states through, so `cd … && python -m unittest discover` is
    the check with nothing after it and stays green. A step's own keys and the job's are read and no
    other step's: a checkout behind `if: always()` says nothing about these checks.
    """
    groups: list[list[str]] = []
    for line in lines:
        if line.startswith("-") or not groups:
            groups.append([])
        groups[-1].append(line)
    steps = [group for group in groups if group]
    checks = [step for step in steps if runs_a_check(step)]
    if not checks:
        return []
    found = []
    # Read over the job's own lines — the ones before its first step, which are a group of their own
    # above — and over the steps that run the checks: a condition on the job and the same on the
    # check are one clause rather than two that can be weakened apart, and a condition on another
    # step is not this job's failure to reach the cap.
    head = steps[0] if not steps[0][0].startswith("-") else []
    read = head + [line for step in checks for line in step]
    if any(NONFATAL.match(line) for line in read):
        found.append("the check is made non-fatal, so its failure would leave this job green")
    if any(NEVER_RUNS.match(line) for line in read):
        found.append("the check runs only when a condition holds, so it can be skipped")
    for step in checks:
        name = next((line.removeprefix("- ").split(":", 1)[1].strip()
                     for line in step if line.startswith("- name:")), step[0])
        ran = commands(step)
        after = [one for one in ran[check_reached(step) + 1:]
                 if not KEY_LINE.match(one)]
        if any(not STILL_FAILS.match(one) for one in after):
            found.append(f"{name} runs the check before `{after[-1]}`, so that command owns its "
                         f"exit status")
    return found


def links_missing(lines: list[str]) -> list[str]:
    """Which of the links a job that runs these checks owes that job lacks: the rule driver beside
    the suite, the full history both read, and a failure that reaches the job. Read as commands, so
    a job is named for what it runs rather than for a spelling, and the ways a step throws its own
    failure away are `unfatal`'s above.
    """
    if not runs_the_suite(lines):
        return []
    found = []
    if not runs_the_driver(lines):
        found.append(DRIVER_LINK)
    if not any(line.strip() == FULL_HISTORY for line in lines):
        found.append(HISTORY_LINK)
    if unfatal(lines):
        found.append(FATAL_LINK)
    return found


def mutated(text: str, what: str) -> str:
    """A workflow's text in the state of one link being gone, or the text unchanged when `what` is
    not a state this file names.

    `NO_DRIVER` and `NO_HISTORY` take the lines that hold them out — the one a command runs on and
    the one the fetch is written as, so neither depends on how a step is spelled. The other three
    write the way a check's failure is thrown away *inside the step that runs the check*, located by
    the reading the case itself makes (`runs_a_check`) and at the column that step's own keys are
    written at — two past its dash — so the state is one link gone in any spelling the case accepts,
    and a step that runs none of these checks is never written into.
    """
    if what not in MUTATIONS:
        return text
    lines = text.splitlines(keepends=True)
    if what in (NO_DRIVER, NO_HISTORY):
        kept = []
        for line in lines:
            stripped = line.strip()
            command = RUN_KEY.sub("", stripped.removesuffix("\\").strip()).strip()
            if (what == NO_DRIVER and RUNS_DRIVER.search(command)) \
                    or (what == NO_HISTORY and stripped == FULL_HISTORY):
                continue
            kept.append(line)
        return "".join(kept)
    written: dict[int, list[str]] = {}
    swallowed: dict[int, str] = {}
    for start, end, indent in step_ranges(text):
        step = [lines[at].rstrip("\n").removesuffix("\\").strip() for at in range(start, end)]
        if not runs_a_check(step):
            continue
        if what == SWALLOWED:
            at = start + command_lines(step)[check_reached(step)][0]
            swallowed[at] = swallowed.get(at, lines[at].rstrip("\n")) + " || true"
            continue
        written.setdefault(end, []).append(
            f"{' ' * indent}{'continue-on-error: true' if what == NON_FATAL else 'if: false'}\n")
    out = []
    for at, line in enumerate(lines):
        out += written.get(at, [])
        if at in swallowed:
            line = swallowed[at] + ("\n" if line.endswith("\n") else "")
        out.append(line)
    return "".join(out) + "".join(written.get(len(lines), []))
