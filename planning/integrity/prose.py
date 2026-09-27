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
beginning with that same marker.** A block that opens and never closes runs to the end of the
document, because a document that opens a fence and stops has still published a block.

**Two shapes this reading does not see, refused by the line they begin on rather than parsed.**
A line indented four spaces or a tab is an indented code block to Markdown, and this walk cannot
tell such a block from a line the document states, so a bound inside one is either a claim the
document makes or one it is quoting and no count comparison says which. A fence opened with four
or more markers closes here at the first three, so a bound the block still holds is read as a
statement the moment a three-marker line appears inside it. Both would fail the same way — a
figure moves and every count-only check stays green — and guessing at them would guess differently
on the next document, so neither is classified: a line of either shape is left uncounted, and
`unseen` refuses the document by the line the shape begins on. Measured over every document this
census records: one held the first, quoting a refusal the Host writes, and was fenced; its count
did not move, because no line of that shape held a figure.

**One walk answers all of it, because it is all asked of the same fence.** `statements`,
`unclosed` and `unseen` were written as separate answers and became separate copies of the same
state machine, and a copy is changed on its own: the one that counted was right about every line it
saw while the one that refused had nothing to refuse over, so a fence left open swallowed every
claim below it. `_read` walks once and answers all three; the names below read it, so there is no
second copy to be right on its own.
"""
from __future__ import annotations

import pathlib

FENCE = ("```", "~~~")


def _read(document: pathlib.Path) -> tuple[list[str], str | None, str | None]:
    """The lines `document` states, why it is not well formed, and why it is not read whole.

    The lines are returned whole and unaltered, so whatever reads figures out of a line reads the
    document's own text and not a rewriting of it: a statement is counted because the document
    states it, and a quotation is left out because the document is showing rather than claiming.

    Both refusals name the line they begin on, because that is the line an author completes,
    deletes or unfences, and both are returned rather than raised so one run can name every
    document that is wrong instead of the first. A document may be refused twice over — an
    indented block inside a fence it never closes, say — because the two are asked separately and a
    caller that wants both reads both.

    An unclosed fence is the one shape this reading cannot absorb: it makes every line after it a
    quotation, so a bound the document states below it stops being counted, the figure a census
    records is smaller than what the document publishes, and a document in that state passes every
    check that only compares counts. The other two are shapes it cannot see rather than cannot
    absorb, and they would fail in the opposite direction — a quoted bound counted as a claim —
    which no count comparison notices when the number it moves happens to be the number recorded.
    So neither is counted and both are refused, by the line each begins on.
    """
    stated: list[str] = []
    fence: str | None = None
    opened = 0
    unseen: str | None = None
    for number, line in enumerate(document.read_text().splitlines(), 1):
        stripped = line.lstrip()
        if fence is None:
            marker = stripped[:3]
            if marker in FENCE:
                width = len(stripped) - len(stripped.lstrip(marker[:1]))
                if width > 3 and unseen is None:
                    unseen = (f"a fence opened on line {number} uses {width} markers, and a block "
                              f"closes here at the first three, so a bound it still holds would be "
                              f"counted as a statement the moment a three-marker line appears "
                              f"inside it — open the block with three")
                fence, opened = marker, number
            elif line.strip() and line.startswith(("    ", "\t")):
                if unseen is None:
                    unseen = (f"a line indented four spaces or a tab begins on line {number}, "
                              f"which is an indented code block to a Markdown reader, and this "
                              f"walk cannot tell such a block from a line the document states, "
                              f"so a bound inside one is left uncounted rather than guessed at "
                              f"— fence it, or state it in prose")
            else:
                stated.append(line)
        elif stripped.startswith(fence):
            fence = None
    if fence is None:
        return stated, None, unseen
    return stated, f"a `{fence}` fence opened on line {opened} is never closed", unseen


def statements(document: pathlib.Path) -> list[str]:
    """Every line of `document` outside a fenced code block, in the order written."""
    return _read(document)[0]


def unclosed(document: pathlib.Path) -> str | None:
    """Why this document is not well formed, or None when every fence it opens it also closes.

    Read from the same walk `statements` counts through, so the two cannot disagree about a fence.
    """
    return _read(document)[1]


def unseen(document: pathlib.Path) -> str | None:
    """Why this reading cannot vouch for this document whole, or None when it can.

    Two shapes are offered to this walk and it cannot see either: a line indented four spaces or a
    tab, which is an indented code block to a Markdown reader, and a fence opened with four or more
    markers, which closes here at the first three. Counted, the first reads a quotation as a claim
    and the second ends a block its author believes still open, so a bound the document only quotes
    becomes its own and a figure moves where the number recorded does not move with it. So a line
    of either shape is left uncounted — this walk will not classify what it cannot see, and reports
    that rather than guessing once and not the next time — and the document is refused by the line
    the shape begins on, which is the line an author completes, deletes or unfences, and by which of
    the two it is, because the repair is the author's to make: a block indented four spaces is
    fenced, and a block opened with four markers is opened with three.
    """
    return _read(document)[2]
