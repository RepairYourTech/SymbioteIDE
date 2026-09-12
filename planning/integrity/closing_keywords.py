#!/usr/bin/env python3
"""What a pull-request description will close, and where its prose denies it.

GitHub honours a closing keyword — `close[sd]`, `fix(e[sd])`, `resolve[sd]`,
optionally followed by a colon, in any case — before `#N` (an issue in this
repository) or `OWNER/REPOSITORY#N` (an issue in that one), and it does not
read negation. A description that says "This does not close #54." still closes
#54 when the pull request merges into the default branch. That is how #54 was
closed one second after PR #530 merged, whose own first line says the slice
does not complete it.

This tool reports the references GitHub will close and fails when one of them
sits in a clause that negates the keyword, so the wording can be fixed before
the merge instead of after it. It reads the description from stdin (or
--body-file), writes nothing, and implements only the documented syntax table:
https://docs.github.com/en/issues/tracking-your-work-with-issues/using-issues/linking-a-pull-request-to-an-issue
Keywords are honoured only when the pull request targets the default branch,
so `--base` reports them as ignored rather than failing a description on a
branch where merging cannot close anything.

    printf '%s' "$PR_BODY" | python3 planning/integrity/closing_keywords.py
"""

import argparse
import re
import sys
from dataclasses import dataclass
from pathlib import Path

KEYWORD = r"close[sd]?|fix(?:e[sd])?|resolve[sd]?"
CLOSING_REFERENCE = re.compile(
    rf"(?<![\w-])(?P<keyword>{KEYWORD})\b\s*:?\s*"
    r"(?P<reference>(?P<repository>[\w.-]+/[\w.-]+)?#(?P<issue>\d+))",
    re.IGNORECASE,
)
NEGATION = re.compile(
    r"\b(?:not|no|never|without|nor|cannot|can't|won't|don't|doesn't|isn't|aren't|"
    r"hasn't|haven't|didn't|wouldn't|shouldn't)\b",
    re.IGNORECASE,
)
CLAUSE_END = re.compile(r"[.!?;\n]+")


@dataclass(frozen=True)
class Closing:
    """One reference GitHub would close, with the clause that states it."""

    keyword: str
    reference: str
    clause: str

    def negated(self):
        return NEGATION.search(self.clause) is not None


def closings(body):
    """The references GitHub will close, in the order the description states
    them. Only the keyword's own clause is kept, because a negation anywhere
    else in the description does not stop GitHub from reading the keyword."""
    found = []
    for match in CLOSING_REFERENCE.finditer(body):
        start = 0
        for end in CLAUSE_END.finditer(body[: match.start()]):
            start = end.end()
        found.append(
            Closing(
                keyword=match.group("keyword"),
                reference=match.group("reference"),
                clause=body[start : match.end()],
            )
        )
    return found


def report(closings):
    """The lines to print for `closings`, and whether any of them is negated."""
    hazards = [closing for closing in closings if closing.negated()]
    lines = []
    if not closings:
        lines.append("No closing keyword: merging this description closes nothing.")
    else:
        for closing in closings:
            lines.append(
                f'Merging closes {closing.reference} (keyword "{closing.keyword}").'
            )
    for closing in hazards:
        lines.append(
            "FAIL: this clause negates a closing keyword, and GitHub's parser "
            f'does not read negation:\n  "{closing.clause.strip()}"\n'
            f"It will still close {closing.reference} when this pull request "
            "merges. Rewrite the clause so the keyword and the number are not "
            f'adjacent — for example, "leaves {closing.reference} open" — or '
            "remove the negation if the closure is intended."
        )
    return lines, bool(hazards)


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument(
        "--body-file",
        type=Path,
        help="pull-request description to read (default: standard input)",
    )
    parser.add_argument(
        "--base",
        help="the pull request's base branch; keywords count only on the default branch",
    )
    parser.add_argument("--default-branch", default="main")
    arguments = parser.parse_args(argv)

    body = (
        arguments.body_file.read_text(encoding="utf-8")
        if arguments.body_file
        else sys.stdin.read()
    )
    if arguments.base and arguments.base != arguments.default_branch:
        print(
            f"Base {arguments.base} is not {arguments.default_branch}, the default "
            "branch, so GitHub ignores these keywords and merging closes nothing."
        )
        return 0

    lines, failed = report(closings(body))
    for line in lines:
        print(line)
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
