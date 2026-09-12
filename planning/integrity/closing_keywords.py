#!/usr/bin/env python3
"""What a pull-request description will close, and where its prose denies it.

GitHub honours a closing keyword — `close[sd]`, `fix(e[sd])`, `resolve[sd]`,
optionally followed by a colon, in any case — before `#N` (an issue in this
repository) or `OWNER/REPOSITORY#N` (an issue in that one), and it does not
read negation. A description that says "This does not close #54." still closes
#54 when the pull request merges into the default branch. That is how #54 was
closed one second after PR #530 merged, whose own first line says the slice
does not complete it.

This tool reports the references GitHub will close and fails under two rules:

* a description fails when one of them sits in a clause that negates the
  keyword, so the wording can be fixed before the merge instead of after it;
* a commit message (`--commits`) fails on any closing keyword at all. GitHub
  scans the commit messages of the merged commits, nothing in review reads
  them, and the description is where an intended closure belongs. PR #551 was
  landed to prevent accidental closures and was itself closed through this
  channel: its final sentence, "so the defect that closed #54 is reproduced
  rather than described", closed #54 on merge because the guard read only
  descriptions. That is also why a description now has to *state* its
  closures: `Closes #54` opens its own clause, while a keyword buried in a
  sentence is prose about a closure, and GitHub cannot tell the difference.

It reads the text from stdin (or --body-file), writes nothing, and implements
only the documented syntax table:
https://docs.github.com/en/issues/tracking-your-work-with-issues/using-issues/linking-a-pull-request-to-an-issue
Keywords are honoured only when the pull request targets the default branch,
so `--base` reports them as ignored rather than failing text on a branch where
merging cannot close anything.

    printf '%s' "$PR_BODY" | python3 planning/integrity/closing_keywords.py
    git log -1 --format=%B "$SHA" | python3 planning/integrity/closing_keywords.py --commits
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
CLAUSE_END = re.compile(r"[.!?;\n,]+")
SENTENCE_END = re.compile(r"[.!?;\n]+")
# What may precede a keyword that still opens its clause: list markers,
# quotes, emphasis, an ordered-list number.
CLAUSE_OPENING = re.compile(r"^[\s>*+`\"'|\-]*(?:\d+\.)?[\s>*+`\"'|\-]*$")


@dataclass(frozen=True)
class Closing:
    """One reference GitHub would close, with the text that carries it."""

    keyword: str
    reference: str
    clause: str
    lead: str
    sentence: str

    def negated(self):
        """Whether the sentence denies the closure GitHub will perform."""
        return NEGATION.search(self.sentence) is not None

    def denied(self):
        """Whether the description rule refuses this reference: its sentence
        denies the closure, or the keyword is prose rather than a statement."""
        return self.negated() or not self.stated()

    def stated(self):
        """Whether the keyword opens its clause, as a stated closure does
        (`Closes #54`) rather than mentioning one (`the defect that closed
        #54`). GitHub reads both the same way; a reader does not."""
        return bool(CLAUSE_OPENING.match(self.lead))


def closings(body):
    """The references GitHub will close, in the order the text states them.
    Each carries the clause that opens before its keyword and the sentence
    around it, so a rule can ask whether the closure was stated (clause) or
    denied (sentence)."""
    found = []
    for match in CLOSING_REFERENCE.finditer(body):
        before = body[: match.start()]
        clause_start = max((end.end() for end in CLAUSE_END.finditer(before)), default=0)
        sentence_start = max(
            (end.end() for end in SENTENCE_END.finditer(before)), default=0
        )
        found.append(
            Closing(
                keyword=match.group("keyword"),
                reference=match.group("reference"),
                clause=body[clause_start : match.end()],
                lead=body[clause_start : match.start()],
                sentence=body[sentence_start : match.end()],
            )
        )
    return found


def report(closings, label="description", refuse_all=False):
    """The lines to print for `closings`, and whether any of them is refused.
    `refuse_all` is the commit-message rule: a closing keyword there is
    honoured on merge and read by nobody, so it is refused whatever the
    sentence around it means. A description is refused when its sentence
    denies the closure, or when the keyword is buried in prose rather than
    stated as its own clause."""
    hazards = closings if refuse_all else [c for c in closings if c.denied()]
    lines = []
    if not closings:
        lines.append("No closing keyword: merging this text closes nothing.")
    else:
        for closing in closings:
            lines.append(
                f'Merging closes {closing.reference} (keyword "{closing.keyword}").'
            )
    for closing in hazards:
        if refuse_all:
            lines.append(
                f"FAIL: the {label} carries a closing keyword, and GitHub closes "
                "the issue when the pull request merges into the default branch:\n"
                f'  "{closing.clause.strip()}"\n'
                "Nothing in review reads a commit message, so state an intended "
                f"closure in the description instead (\"Closes {closing.reference}\"), "
                "or rephrase this one so the keyword and the number are not "
                f'adjacent — for example, "{closing.reference} was closed by …".'
            )
        elif closing.negated():
            lines.append(
                f"FAIL: the {label} negates a closing keyword in its own sentence, "
                f"and GitHub's parser does not read negation:\n  "
                f'"{closing.sentence.strip()}"\n'
                f"It will still close {closing.reference} when this pull request "
                "merges. Rewrite the clause so the keyword and the number are not "
                f'adjacent — for example, "leaves {closing.reference} open" — or '
                "remove the negation if the closure is intended."
            )
        else:
            lines.append(
                f"FAIL: the {label} buries a closing keyword in a sentence, and "
                f"GitHub closes the issue whatever the sentence means:\n  "
                f'"{closing.clause.strip()}"\n'
                f"State it as its own clause if the closure is intended (\"Closes "
                f'{closing.reference}\"), or rephrase the prose so the keyword and '
                f'the number are not adjacent ("{closing.reference} was closed by…").'
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
        "--commits",
        action="store_true",
        help="the text is a commit message: refuse any closing keyword",
    )
    parser.add_argument(
        "--label",
        help="what the text is, named in a refusal",
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

    label = arguments.label or ("commit message" if arguments.commits else "description")
    lines, failed = report(closings(body), label, refuse_all=arguments.commits)
    for line in lines:
        print(line)
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
