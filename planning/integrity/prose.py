"""The lines a document states, as against the ones it quotes.

`contracts.BOUND` reads a figure statement out of a line and `contracts.figures` counts the lines
carrying one. It counted every line, fenced code blocks included, so an envelope `cli.md` publishes
for a request the CLI refuses moved the count with it: the block states a byte count, the count
moved 2 → 3, and the only way to put the example back was to leave it out. A fenced block is a
document quoting a program, a transcript or a message. The byte counts in a refusal are the
binary's, not the document's claim about itself, and a bound written into a shell example is that
example's. What the census owes a decision about is a claim the document makes, so this is the one
reading that tells a statement from a quotation and `figures` counts through it.

**A line is a quotation by what it is inside, so this reads the line's own prefix.** Every
blockquote marker comes off first, because a blockquote is a container and a shape inside one is
the shape it would be outside: `>     text` is an indented code block and `> ``` ` opens a fence.
An indented code block is four columns past the content column of the block the line is in, which is
what tells a wrapped list item — one paragraph written over two lines — from a block quoted inside
it, and a list item that indents its own content by four is a paragraph too. A fence opens on three
markers and closes at the first line beginning with those three, so a block opened with four or more
closes here early and is refused rather than half-read. A line of any of these shapes is left
uncounted and the document is refused by the line the shape begins on, because that is the line an
author completes, deletes or unfences. **Every reason is returned, not the first**, so one run names
every line to mend; a document may be refused several times over, and each reason is asked and
answered on its own.

**One walk answers all of it, because it is all asked of the same line.** `statements`, `unclosed`
and `unseen` were separate answers and became separate copies of the same state machine, and a copy
is changed on its own: the one that counted was right about every line it saw while the one that
refused had nothing to refuse over. `_read` walks once and answers all three; the names below read
it, so there is no second copy to be right on its own.
"""
from __future__ import annotations

import pathlib
import re

FENCE = ("```", "~~~")
# `> ` or `> ` repeated, at up to three columns of indent: a blockquote marker, which opens a
# container rather than saying anything about what the line inside it is.
QUOTE = re.compile(r" {0,3}>[ \t]?")
# A bullet or an ordered marker, which only opens an item when a space or the line's end follows
# it: a rule of `---` is a rule, not a list.
ITEM = re.compile(r"(?:[-+*]|[0-9]{1,9}[.)])(?=[ \t]|\Z)")
CODE = 4


def _indented(line: str) -> tuple[str, int]:
    """The line as its container sees it: every blockquote marker off it, and how far in what
    is left begins. A marker means a new container, which is the walk's signal to forget the
    list items it was inside — the line after a quote is not a continuation of the list above.
    """
    quoted = False
    while (marker := QUOTE.match(line)):
        line, quoted = line[marker.end():], True
    width = 0
    for character in line:
        if character == " ":
            width += 1
        elif character == "\t":
            width += 4 - width % 4
        else:
            break
    return line, width, quoted


def _item(line: str, width: int) -> int | None:
    """The content column of the list item `line` opens, or None when it opens no item.

    One to four spaces follow the marker and the content starts after them. A fifth space is
    not spacing, it is a block of its own, so the content starts one column in and the rest is
    an indented code block inside the item — which a bare four-column test would call prose.
    """
    marker = ITEM.match(line, width)
    if marker is None:
        return None
    rest = line[marker.end():]
    gap = len(rest) - len(rest.lstrip(" "))
    return width + marker.end() - width + (1 if gap > CODE else gap)


def _indented_code(number: int) -> str:
    """Why the line `number` is a quotation this walk will not count."""
    return (f"a line indented four columns past the block it is in begins on line {number}, "
            f"which is an indented code block to a Markdown reader, and this walk cannot tell "
            f"such a block from a line the document states, so a bound inside one is left "
            f"uncounted rather than guessed at — fence it, or state it in prose")


def _wide_fence(number: int, width: int) -> str:
    """Why the fence on line `number` is one this walk will not read as a block."""
    return (f"a fence opened on line {number} uses {width} markers, and a block closes here at "
            f"the first three, so a bound it still holds would be counted as a statement the "
            f"moment a three-marker line appears inside it — open the block with three")


def _read(document: pathlib.Path) -> tuple[list[str], list[str], list[str]]:
    """The lines `document` states, why it is not well formed, and why it is not read whole.

    The lines are returned whole and unaltered, so whatever reads figures out of a line reads
    the document's own text and not a rewriting of it: a statement is counted because the
    document states it, and a quotation is left out because the document is showing rather
    than claiming.

    Both refusals are lists rather than one reason because a document can hold more than one
    shape and an author mends a file in one pass, not one shape per run. An unclosed fence is
    the shape this reading cannot absorb: it makes every line after it a quotation, so a bound
    the document states below it stops being counted and every check that compares counts
    alone passes. The other two are shapes it cannot see rather than cannot absorb, and they
    would fail in the opposite direction — a quoted bound counted as a claim — which no count
    comparison notices when the number it moves is the number already recorded.
    """
    stated: list[str] = []
    malformed: list[str] = []
    unread: list[str] = []
    # The content column of each list item we are inside, outermost first. A line indented to
    # a deeper column than the innermost one plus four is a code block inside it; a line
    # indented less closes the items it is no longer in. A blank line closes none, because a
    # list item continues across one.
    items: list[int] = []
    fence: str | None = None
    opened = 0
    for number, line in enumerate(document.read_text().splitlines(), 1):
        body, width, quoted = _indented(line)
        if quoted:
            items.clear()
        if fence is not None:
            if body.lstrip().startswith(fence):
                fence, opened = None, 0
            continue
        if body.strip():
            while items and width < items[-1]:
                items.pop()
        content = items[-1] if items else 0
        if body.strip() and width >= content + CODE:
            unread.append(_indented_code(number))
            continue
        stripped = body.lstrip()
        if stripped[:3] in FENCE:
            marker = stripped[:3]
            markers = len(stripped) - len(stripped.lstrip(marker[0]))
            if markers > 3:
                unread.append(_wide_fence(number, markers))
            fence, opened = marker, number
            continue
        item = _item(body, content if not items else items[-1])
        if item is not None:
            while items and item <= items[-1]:
                items.pop()
            items.append(item)
        stated.append(line)
    if fence is not None:
        malformed.append(f"a `{fence}` fence opened on line {opened} is never closed")
    return stated, malformed, unread


def statements(document: pathlib.Path) -> list[str]:
    """Every line of `document` outside a quotation, in the order written."""
    return _read(document)[0]


def unclosed(document: pathlib.Path) -> list[str]:
    """Why this document is not well formed, one reason per fence it opens and never closes."""
    return _read(document)[1]


def unseen(document: pathlib.Path) -> list[str]:
    """Why this reading cannot vouch for this document whole, one reason per shape it cannot see.

    A line of any such shape is left uncounted rather than classified: this walk will not
    guess at a block it cannot see, and a guess is made once and not the next time. The
    reasons name the line the shape begins on and which of the shapes it is, because the
    repair is the author's to make — a block indented four columns past its own is fenced,
    and a block opened with four markers is opened with three.
    """
    return _read(document)[2]
