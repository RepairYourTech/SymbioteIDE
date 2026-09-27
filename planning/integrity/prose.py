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
A list marker is a container too, so what a document writes after one is that container's own
line: `- ``` ` opens a fence inside the item and `- >` alone says nothing at all. An indented
code block is four columns past the content of the block the line is in, so a list item that
indents its own content by four is writing a paragraph, and one that puts five spaces after its
marker is quoting a block — a fifth space is not spacing.

**Each question has one owner, because a rule split across two places is two rules that will
disagree.** Is this line an indented code block: `_indented_code`, from the measurement
`_opening` took, and nothing else. Does this line open a fence: `_fence`, and how wide it is.
Is this line an HTML comment: `_opens_comment` and `_closes_comment`. The walk in `_read` keeps
the state and asks nothing, so a change to one shape lands in the one function that owns it. A
fence opened with four or more markers closes here at the first three, so a block the author
believes still open is read as ending early. A line of any of these shapes is left uncounted and
the document is refused by the line the shape begins on, because that is the line an author
completes, deletes or unfences. **Every reason is returned, not the first**, so one run names
every line to mend.

**One walk answers all of it, because it is all asked of the same line.** `statements`, `unclosed`
and `unseen` were separate answers and became separate copies of the same state machine, and a copy
is changed on its own. `_read` walks once and answers all three; the names below read it.
"""
from __future__ import annotations

import pathlib
import re
from typing import NamedTuple

FENCE = ("```", "~~~")
# `> ` or `> ` repeated, at up to three columns of indent: a blockquote marker, which opens a
# container rather than saying anything about what the line inside it is.
QUOTE = re.compile(r" {0,3}>[ \t]?")
# A bullet or an ordered marker, which only opens an item when a space or the line's end follows
# it: a rule of `---` is a rule, not a list.
ITEM = re.compile(r"(?:[-+*]|[0-9]{1,9}[.)])(?=[ \t]|\Z)")
# An HTML comment, which runs to the line that closes it and is not Markdown at all.
COMMENT = "<!--"
CLOSES = "-->"
CODE = 4


class _Opening(NamedTuple):
    """How one line sits in the block it is in, measured once and read by name.

    `past` is how many columns this line's own text sits past the content of the block it is
    in, and `item` is the content column of the list item it opens, or `None` when it opens
    none. A bare tuple is read three ways by three callers, and a caller that reads it by
    position is counting.
    """
    past: int
    item: int | None


def _unquote(line: str) -> tuple[str, int]:
    """`line` with every blockquote marker off it, and how many markers there were.

    One loop, because a line's quote markers come off once and every reading of that line
    starts from what is left: the width of the rest, and the text the rest says.
    """
    depth = 0
    while (marker := QUOTE.match(line)):
        line, depth = line[marker.end():], depth + 1
    return line, depth


def _indented(line: str) -> tuple[str, int, int]:
    """The line as its container sees it: every blockquote marker off it, how far in what is
    left begins, and how many markers there were.

    The count is the quote's depth, and it is what tells a line that continues the paragraph
    a quote already holds from one that opens a quote beside it — a marker at the depth the
    paragraph was written at continues it, and a deeper one is a new container, which is
    also what tells the walk to forget the list items outside it.
    """
    line, depth = _unquote(line)
    width = 0
    for character in line:
        if character == " ":
            width += 1
        elif character == "\t":
            width += 4 - width % 4
        else:
            break
    return line, width, depth


def _opening(line: str, width: int) -> _Opening:
    """How many columns this line's text sits past the content of the block it is in, and the
    content column of the list item it opens if it opens one.

    A line that is not a list item is indented by its own columns, so its text sits `width`
    columns past the container's content. A line that opens an item is measured against the
    content of the item it opens, which its own marker width decides: one to four spaces
    after a marker are spacing, so the text is inside the item and nothing is past its
    content, while a fifth space is not spacing — the content starts one column in and the
    text then sits four past it, which is the code block inside the item.

    An item with nothing after its marker is its own case: the marker and the one space that
    would have followed it are the item's indent, so the lines below it are measured one
    column further right than a reader would guess. One function, so `_indented_code` and the
    walk cannot disagree about where a line's content begins.
    """
    marker = ITEM.match(line, width)
    if marker is None:
        return _Opening(width, None)
    rest = line[marker.end():]
    gap = len(rest) - len(rest.lstrip(" "))
    if gap > CODE:
        # A fifth space is not spacing: the content starts one column in and the text that
        # follows sits four past it, which is the code block inside the item.
        return _Opening(gap - 1, marker.end() + 1)
    if not rest.strip():
        # Nothing after the marker at all: the lines below are read against the marker and
        # the one space that would have followed it, whatever spacing the marker line itself
        # carries — which is one column further right than a reader counting the spaces would
        # put it, and the difference is whether four columns below is the item's own line.
        return _Opening(0, marker.end() + 1)
    # One to four spaces are spacing and the content starts after them. Either way the text
    # is inside the item, and nothing is past its content — which is what the fifth space
    # above is for.
    return _Opening(0, marker.end() + gap)


def _content(body: str, item: int | None) -> str:
    """The line as the block it is in reads it, with every container marker off it.

    A list marker and a blockquote marker open containers rather than saying anything, so
    what a document writes after one is that container's own line: `- ``` ` opens a fence
    inside the item, `- > a quote` says a quote, and `- >` alone says nothing at all. The
    walk tests the text this returns rather than the raw line, which is what lets one fence
    rule and one comment rule serve a marker and a bare line alike.
    """
    return _unquote((body if item is None else body[item:]).lstrip())[0]


def _indented_code(width: int, content: int, paragraph: bool, opening: _Opening,
                   opens_quote: bool) -> bool:
    """Whether this line is an indented code block: the one place that question is asked.

    Four columns past the content of the block the line is in is a block, and that is one
    number whichever way the columns got there — the line's own indent, or the spaces after
    a list marker — which is why the measurement arrives here already taken rather than
    taken again. Three things keep it from being a block. An indented code block cannot
    interrupt a paragraph, and only a blank line closes one: `- a step` and `    and the
    next thing it does` with no blank between them are one paragraph, which is how every
    document wraps a list item and a sentence. An HTML block is not Markdown at all, so
    nothing under a comment is a code block. And a line that opens a container of its own is
    measured as a block whatever the paragraph above it was doing, which is what
    `_quoted_block` answers.
    """
    if paragraph and not _quoted_block(width, opening, opens_quote):
        return False
    if width >= content + CODE:
        # Four past the container is a block whatever the line says, a list marker included:
        # a marker that far in is quoted text, not an item nested four columns in.
        return True
    # Below that, a line that opens no item is indented by its own columns and has to clear
    # the container's content column. A line that opens an item is measured against the
    # content of the item it opens, which already carries the marker's own width and the
    # five-space rule, so the two never add up twice.
    return (opening.past if opening.item is not None else width - content) >= CODE


def _quoted_block(width: int, opening: _Opening, opens_quote: bool) -> bool:
    """Whether this line is quoted text by its own prefix: a five-space list marker, or a
    blockquote marker with four columns behind it.

    A line with no marker at all is never this: four columns of a paragraph's own wrapping
    are prose however deep they are, and only a blank line ends that. A marker followed by
    one to four spaces is spacing and the text after it is the item's own, so a paragraph
    continues under it; five spaces is not spacing — the content starts one column in and
    the text sits four past it — so the line is a block inside the item and it interrupts
    whatever paragraph it lands in. A blockquote marker is the same: four columns behind it
    is quoted, none is a paragraph continuing.
    """
    if opening.item is not None:
        # A list marker whose own spacing puts the text four columns past the item's content.
        return opening.past >= CODE
    # A blockquote marker is already counted, so `width` is the columns behind it. Four of
    # them are quoted text when this line is what opens the quote; when the quote was already
    # open the columns are the item it holds, and the paragraph continues inside it.
    return opens_quote and width >= CODE


def _ends_paragraph(line: str) -> bool:
    """Whether this line closes an open paragraph rather than continuing one.

    A line with nothing on it ends one, which is what carries the whole weight here: a marker
    that opens an item and writes nothing in it says nothing, so four columns below it are a
    block and not that item's own second line. A heading ends one too, and so does a rule —
    which doubles as the underline that turns the paragraph above it into a setext heading.
    Every other line with text on it is paragraph text, a list item's own text included, so a
    paragraph a marker opened is still open on the line after it.
    """
    stripped = line.strip()
    if not stripped or stripped[0] in "#":
        return True
    return set(stripped) <= {"-", "=", "*", "_"}


def _fence(text: str) -> tuple[str, int] | None:
    """The fence this line opens and how many markers it is opened with, or `None` when it
    opens none.

    `text` is the line as the block it is in reads it, so this is the one place that answers
    for a fence written after a list marker and one written on a line of its own alike — a
    fence is a shape, and where it was written is not part of what it is.
    """
    if text[:3] not in FENCE:
        return None
    marker = text[:3]
    return marker, len(text) - len(text.lstrip(marker[0]))


def _opens_comment(text: str) -> bool:
    """Whether this line opens an HTML block, which is not Markdown at all: nothing under one
    is a claim and nothing under one is a code block."""
    return text.startswith(COMMENT)


def _closes_comment(line: str) -> bool:
    """Whether this line closes an HTML comment. A comment opened and closed on one line is
    still one, so this is asked of the line that opened it as well."""
    return CLOSES in line


def _code_reason(number: int) -> str:
    """Why the line `number` is a quotation this walk will not count."""
    return (f"a line indented four columns past the block it is in begins on line {number}, "
            f"which is an indented code block to a Markdown reader, and this walk cannot tell "
            f"such a block from a line the document states, so a bound inside one is left "
            f"uncounted rather than guessed at — fence it, or state it in prose")


def _fence_reason(number: int, markers: int) -> str:
    """Why the fence on line `number` is one this walk will not read as a block."""
    return (f"a fence opened on line {number} uses {markers} markers, and a block closes here at "
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
    # The content column of each list item we are inside, outermost first. A blank line
    # closes none of them, because a list item continues across one.
    items: list[int] = []
    fence: str | None = None
    opened = 0
    in_comment = False
    # Whether a paragraph is open, which is the one thing that stops an indented line from
    # being a block. A blank line closes it and every block of its own closes it.
    paragraph = False
    # How deep the blockquote the walk is inside, so a marker at that depth is read as
    # continuing it rather than opening a new one.
    quoted_depth = 0
    for number, line in enumerate(document.read_text().splitlines(), 1):
        body, width, depth = _indented(line)
        # Whether this line's quote continues the container the line above was in. A marker
        # at the depth already open is a lazy continuation of that paragraph, so the list
        # items and the paragraph stand; a deeper one opens a container beside it, and a
        # line with no marker at all is in whatever container the walk is already in.
        inside = depth > 0 and depth <= quoted_depth
        if depth > 0:
            quoted_depth = depth
            if not inside:
                # A new container: the list items outside it are closed, and a paragraph
                # inside a quote is not the one that was open outside it.
                items.clear()
                paragraph = False
        elif quoted_depth:
            # No marker on this line. It goes on inside the quote only if the quote holds a
            # paragraph to go on inside: an empty item states nothing, so there is nothing
            # to carry across and the containers the marker opened close here.
            quoted_depth = 0
            if not paragraph:
                items.clear()
        if fence is not None:
            if body.lstrip().startswith(fence):
                fence, opened, paragraph = None, 0, False
            continue

        if in_comment:
            # Everything under a comment is not Markdown: not a block this walk can refuse
            # over, and not a claim either, so it is left uncounted rather than stated.
            if _closes_comment(body):
                in_comment = False
            paragraph = False
            continue

        if not body.strip():
            # A blank line closes a paragraph, which is what lets the next line interrupt
            # one. It closes no list item, which continues across one, so `items` stands.
            paragraph = False
            stated.append(line)
            continue

        while items and width < items[-1]:
            items.pop()
        content = items[-1] if items else 0

        # What this line says once the containers it opens are off it, and the measurement
        # of it that goes with the answer. Taken once here, and read by every test below,
        # because two callers each measuring a line is two answers to the same question.
        opening = _opening(body, width)
        text = _content(body, opening.item)
        opens_quote = depth > 0 and not inside

        if _opens_comment(text):
            in_comment = not _closes_comment(text)
            paragraph = False
            continue

        # A line with no text on it is not a quotation to refuse, however its spacing reads.
        if text.strip() and _indented_code(width, content, paragraph, opening, opens_quote):
            unread.append(_code_reason(number))
            paragraph = False
            continue

        if (here := _fence(text)) is not None:
            marker, markers = here
            if markers > 3:
                unread.append(_fence_reason(number, markers))
            fence, opened, paragraph = marker, number, False
            continue

        if opening.item is not None:
            while items and opening.item <= items[-1]:
                items.pop()
            items.append(opening.item)
        # A line that states nothing — an item with no text of its own, a quote with none —
        # opens no paragraph and nothing continues it, which `_ends_paragraph` says for a line
        # with no text on it. Below an item with text in it four columns are the item's own
        # second line; below one holding nothing they are a block.
        paragraph = not _ends_paragraph(text)
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
