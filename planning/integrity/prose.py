"""The lines a document states, as against the ones it quotes.

`contracts.BOUND` reads a figure statement out of a line and `contracts.figures` counts the lines
carrying one. It counted every line, fenced code blocks included, so an envelope `cli.md` publishes
for a request the CLI refuses moved the count with it: the block states a byte count, the count
moved 2 → 3, and the only way to put the example back was to leave it out. A fenced block is a
document quoting a program, a transcript or a message. The byte counts in a refusal are the
binary's, not the document's claim about itself, and a bound written into a shell example is that
example's. What the census owes a decision about is a claim the document makes, so this is the one
reading that tells a statement from a quotation and `figures` counts through it.

**A fence is a line beginning ` ``` ` or `~~~`, and the block it opens closes at the first line
beginning with that same marker written at least as many times as the opening line did.** A block
that opens and never closes runs to the end of the
document, because a document that opens a fence and stops has still published a block. A shorter
run of the same characters inside a longer fence is that block quoting a fence, not closing one, so
the census does not start counting a fenced example's own examples as the document's claims. An
indented code block is a shape this reading does not see, and says so here rather than leaving it
to a reader of the diff to discover: the documents in this tree fence what they quote.

**One walk answers both questions, because they are asked of the same fence.** `statements` and
`unclosed` were two copies of the same six lines of state machine, and a copy is changed on its
own: the one that never refused still reported a document's statements correctly while a fence it
left open swallowed every claim below it. `_read` owns the walk; the two names below read it.
"""
from __future__ import annotations

import pathlib

FENCE = ("```", "~~~")


def _fence(stripped: str) -> tuple[str, int] | None:
    """The fence a line writes — which three characters, and how many of them in a row — or None
    when the line writes none. A closing fence writes the same characters at least as many times
    as the opening one, so a fenced example that quotes a shorter fence states that line like any
    other instead of closing its own fence and turning the lines after it into claims."""
    for mark in FENCE:
        width = len(stripped) - len(stripped.lstrip(mark[:1]))
        if width >= len(mark):
            return mark, width
    return None


def _read(document: pathlib.Path) -> tuple[list[str], str | None]:
    """The lines `document` states, and why it is not well formed, or None when it is.

    The lines are returned whole and unaltered, so whatever reads figures out of a line reads the
    document's own text and not a rewriting of it: a statement is counted because the document
    states it, and a quotation is left out because the document is showing rather than claiming.

    The refusal names the line the fence opened on, because that is the line an author completes or
    deletes, and it is returned rather than raised so one run can name every document that is wrong
    instead of the first. An unclosed fence is the one shape this reading cannot absorb: it makes
    every line after it a quotation, so a bound the document states below it stops being counted,
    the figure a census records is smaller than what the document publishes, and a document in that
    state passes every check that only compares counts.
    """
    stated: list[str] = []
    fence: tuple[str, int] | None = None
    opened = 0
    for number, line in enumerate(document.read_text().splitlines(), 1):
        stripped = line.lstrip()
        if fence is None:
            mark = _fence(stripped)
            if mark is None:
                stated.append(line)
            else:
                fence, opened = mark, number
        elif (mark := _fence(stripped)) is not None and mark[0] == fence[0] and mark[1] >= fence[1]:
            fence = None
    if fence is None:
        return stated, None
    return stated, f"a `{fence[0]}` fence opened on line {opened} is never closed"


def statements(document: pathlib.Path) -> list[str]:
    """Every line of `document` outside a fenced code block, in the order written."""
    return _read(document)[0]


def unclosed(document: pathlib.Path) -> str | None:
    """Why this document is not well formed, or None when every fence it opens it also closes.

    Read from the same walk `statements` counts through, so the two cannot disagree about a fence.
    """
    return _read(document)[1]
