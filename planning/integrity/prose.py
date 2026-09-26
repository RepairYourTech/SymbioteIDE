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
document, because a document that opens a fence and stops has still published a block. An indented
code block is a shape this reading does not see, and says so here rather than leaving it to a
reader of the diff to discover: the documents in this tree fence what they quote.
"""
from __future__ import annotations

import pathlib

FENCE = ("```", "~~~")


def statements(document: pathlib.Path) -> list[str]:
    """Every line of `document` outside a fenced code block, in the order written.

    The lines kept are returned whole and unaltered, so whatever reads figures out of a line reads
    the document's own text and not a rewriting of it: a statement is counted because the document
    states it, and a quotation is left out because the document is showing rather than claiming.
    """
    kept: list[str] = []
    fence: str | None = None
    for line in document.read_text().splitlines():
        stripped = line.lstrip()
        if fence is None:
            if stripped.startswith(FENCE):
                fence = stripped[:3]
            else:
                kept.append(line)
        elif stripped.startswith(fence):
            fence = None
    return kept
