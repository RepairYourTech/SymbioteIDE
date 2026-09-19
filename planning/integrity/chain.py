"""The chain that runs this directory's proof, read as the commands its steps run.

A hold nothing runs is not a hold. This directory's suite states the reader-size guard's cap;
`revert_rules.py` is what refuses a guard grown a line past it, and its refusal is shown against an
earlier guard, so its checkout must hold one. So the job that runs this directory's suite must run
the driver too — two halves of one proof over one checkout — and must fetch the full history both
read. `test_python_floor.py`'s chain case refuses every link of that, and the driver's `HOLDS` rows
hold that case.

This module is the reading both go through, so which step runs these checks has **one** owner: a job
is named for the commands its steps run, so `-s planning/integrity`, `-s ./planning/integrity`, the
directory positionally, a `cd` into it and a preceding `cd` line are one check. The states the
driver's ways show a refusal by are written by this reading too (`mutated`), so a step re-spelled in
a way `runs_the_suite` accepts is the step a way mutates: a way cannot demand a spelling the case
accepts, which is the false red an anchored workflow text produced before this module existed.
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
JOB = re.compile(r"^  ([A-Za-z0-9_.\-]+):\s*$")
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


def unfatal(lines: list[str]) -> list[str]:
    """The steps of a job that run these checks without a failure reaching the job.

    The cap the suite declares is enforced by that step failing, so a step made non-fatal, given a
    condition that cannot hold, or written so another command owns its exit status (`… || true`,
    `… ; echo done`) leaves the job green with the cap exceeded — the whole hold undone by one edit
    that no reading of the check itself can see. Read as the commands a step runs, the way the chain
    case reads the discovery, so `cd … && python -m unittest discover` is the check with nothing
    after it and stays green. A step's own keys and the job's are read and no other step's: a checkout
    behind `if: always()` says nothing about these checks.
    """
    groups: list[list[str]] = []
    for line in lines:
        if line.startswith("-") or not groups:
            groups.append([])
        groups[-1].append(line)
    steps = [group for group in groups if group]
    checks = [step for step in steps if runs_the_suite(step) or runs_the_driver(step)]
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
        reached = next((at for at in range(len(ran))
                        if runs_the_suite(ran[:at + 1]) or runs_the_driver(ran[:at + 1])), None)
        rest = ran[(reached + 1 if reached is not None else 0):]
        after = [one for one in rest if not KEY_LINE.match(one)]
        if reached is None or any(not STILL_FAILS.match(one) for one in after):
            found.append(f"{name} runs the check before `{ran[-1]}`, so that command owns its "
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
    """A workflow's text in the state of one link being gone: `NO_DRIVER` and `NO_HISTORY` take the
    lines that hold them out, and the other three write that way of throwing a check's failure away
    into the step that runs it. A name this file does not write leaves the text alone, which the
    driver refuses as a way that cannot bite. The step is found by the reading above and never by
    matching a spelling, which is why the ways go through here at all.
    """
    written = []
    for line in text.splitlines(keepends=True):
        stripped = line.rstrip("\n").removesuffix("\\").strip()
        command = RUN_KEY.sub("", stripped).strip()
        runs_a_check = bool(DISCOVER.search(command) or RUNS_DRIVER.search(command))
        if (what == NO_DRIVER and RUNS_DRIVER.search(command)) \
                or (what == NO_HISTORY and stripped == FULL_HISTORY):
            continue
        pad = line[:len(line) - len(line.lstrip())]
        if runs_a_check and what == NON_FATAL:
            written.append(f"{pad}continue-on-error: true\n")
        if runs_a_check and what == CONDITIONAL:
            written.append(f"{pad}if: false\n")
        if runs_a_check and what == SWALLOWED:
            line = line.rstrip("\n") + " || true\n"
        written.append(line)
    return "".join(written)
