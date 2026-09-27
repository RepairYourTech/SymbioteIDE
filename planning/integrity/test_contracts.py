"""Hold the census of `docs/contracts/` to the tree it census.

The record of the passes that measured those documents said the census was complete, and nothing read
that sentence. This file reads it: every entry under the directory is classified exactly once, every
classification that names a holding case is a case the crate really carries and one that really reads
the document, every Markdown document states the figures the census recorded for it, and the reading
that counts them is the one the census states — proven on the spellings it must read and the shapes it
must not.

What nothing here decides: whether a held figure is the right one to hold, and whether a sentence's
meaning is true. The reading counts statements in the shapes `contracts.BOUND` names; a bound written
in another shape is a figure this census does not see, and a line it does see keeps its count when its
*value* changes, which the case that holds the document is what refuses.
"""
from __future__ import annotations

import pathlib
import re
import tempfile
import unittest

import contracts
import prose

ROOT = pathlib.Path(__file__).resolve().parents[2]
CRATES = ROOT / "crates"


class TheCensus(unittest.TestCase):
    def test_every_entry_under_the_contracts_directory_is_classified_once(self):
        """An entry nothing classifies, a classification the directory no longer holds, and an entry
        in both tables, each named: a document added to the directory is a surface this census has
        not decided about, and one removed is a line here that decides about nothing.
        """
        found = contracts.entries()
        held, unheld = set(contracts.HELD), set(contracts.NOT_HELD)
        both = sorted(held & unheld)
        self.assertEqual(both, [], f"these entries are both held and not held: {both}")
        classified = held | unheld
        missing = sorted(set(found) - classified)
        self.assertEqual(
            missing, [],
            f"{', '.join(missing)} sits under docs/contracts/ and no census entry classifies it: "
            f"a document a case reads is held by naming its crate and case, and one no case reads is "
            f"stated with why — a contract added without either is a surface nothing holds")
        stale = sorted(classified - set(found))
        self.assertEqual(
            stale, [],
            f"the census classifies {', '.join(stale)}, which docs/contracts/ does not hold: a "
            f"classification of a document that is gone decides about nothing")

    def test_every_classified_document_states_the_figures_the_census_records(self):
        """Every Markdown document against the count `contracts.FIGURES` records: a document that
        gains a statement in one of the shapes `BOUND` reads fails here naming the document and both
        numbers, so a new bound cannot enter a contract without the census moving in that change.
        """
        documents = sorted(one.name for one in contracts.CONTRACTS.glob("*.md"))
        self.assertEqual(
            sorted(contracts.FIGURES), documents,
            "the figure table names " + ", ".join(sorted(set(contracts.FIGURES) - set(documents)))
            or "the figure table names no document the directory does not hold")
        moved = []
        for name in documents:
            derived = contracts.figures(name)
            if derived != contracts.FIGURES[name]:
                moved.append(f"{name} states {derived} figure statements where the census records "
                             f"{contracts.FIGURES[name]}")
        self.assertEqual(
            moved, [],
            "the census and the documents disagree about the figures they state: " + "; ".join(moved))

    def test_every_held_document_is_read_by_the_case_that_holds_it(self):
        """Every held document against the crate and case the census names: the case must exist in
        that crate, and that crate must be one that reads the document. A holding case renamed or
        deleted fails here naming both, so the census cannot keep claiming a hold nothing makes.

        The tie is at the crate rather than the case's own file because a crate may reach its
        contract through a path constant instead of a literal — `symbiote-constitution` reads
        `constitution.md` through `claims::CONTRACT_PATH`, which a reading of literals cannot
        follow. What is held here is therefore that the case exists and the crate reads the
        document; that the case is what reads it is the case's own text, which no reading decides.
        """
        broken = []
        for name, (crate, case) in sorted(contracts.HELD.items()):
            folder = CRATES / crate
            if not folder.is_dir():
                broken.append(f"{name} is held by {crate}, which is not a crate in this tree")
                continue
            defining = [source for source in sorted(folder.rglob("*.rs"))
                        if re.search(rf"\bfn {re.escape(case)}\b", source.read_text(errors="replace"))]
            if not defining:
                broken.append(f"{name} is held by {case} in {crate}, and no case in that crate "
                              f"is named that")
            reading = [one for one in contracts.readers(name)
                       if one.startswith(f"crates/{crate}/")]
            if not reading:
                broken.append(
                    f"{name} is held by {case} in {crate}, and nothing in that crate reads the "
                    f"document: the files that read it are "
                    f"{', '.join(contracts.readers(name)) or 'none'}")
        self.assertEqual(
            broken, [],
            "the census claims holds the tree does not make: " + "; ".join(broken))

    def test_every_entry_no_case_reads_states_why(self):
        """The other half of the classification: an entry with no reason, and one whose reason is
        the empty string, each refused — an unexplained entry reads as a decision nobody made.
        """
        unexplained = sorted(name for name, why in contracts.NOT_HELD.items() if not why.strip())
        self.assertEqual(
            unexplained, [],
            f"{', '.join(unexplained)} is classified as read by no case and states no reason, so "
            f"the census decides about it without saying why")

    def test_the_figure_reading_is_the_one_this_census_states(self):
        """`BOUND` proven on the spellings it must read and the shapes it must not, and the lines it
        is offered proven on both sides, so the count in `FIGURES` cannot move because the reading
        narrowed or widened: each spelling below is a statement the census reads, each shape after it
        a line that is not one — a version, a mode, a plain numeral, a worded count — and a fenced
        block is a quotation rather than a claim, so the figures inside one are the quoted program's.
        Every name the table records resolves to a document on its own as well, so the count cannot
        move because the census changed where it reads its own keys from.

        The three parts of the second half are here together because each alone is a reading that
        would pass. A reading that skipped prose to quieten a quoted example would leave every count
        here satisfied and let a contract publish a bound nothing in this directory ever saw; a
        reading that counted fences would put `cli.md` one figure higher over an envelope the CLI
        writes, which is how the `request_refused` example was lost in the first place; and a census
        that read its names from the wrong root would fail on a path no document has — every count
        here satisfied, and every count in `FIGURES` a number nobody took.
        """
        reads = (
            "This limits input to 1 MiB.",
            "Configuration is bounded to 32 KiB and validates identity.",
            "It allows at most 16 files, 1 MiB per file and 4 MiB total.",
            "plus up to 32 untracked file content heads",
            "`st-` plus 16 hex characters carrying a 64-bit digest truncation",
            "branch length is bounded to 512 bytes",
            "defaults to 128 and a maximum of 1,024 frames",
            "the writer waits 5,000 ms and no more than 10 seconds",
        )
        not_read = (
            "files use 0600 and generation directories 0700",
            "Protocol v1.5 adds `replace_binding` and `get_binding`.",
            "SQLite schema v4 adds `team_configurations`.",
            "the suites the job runs are named in one table",
            "one authorized root, so the lane is the only remaining question",
        )
        unread = [line for line in reads if not contracts.BOUND.search(line)]
        self.assertEqual(
            unread, [],
            "the census states it reads these shapes and its reading does not: "
            + "; ".join(unread))
        wrongly = [line for line in not_read if contracts.BOUND.search(line)]
        self.assertEqual(
            wrongly, [],
            "the census reads these as figures and states that it does not: " + "; ".join(wrongly))

        stated = "Configuration is bounded to 32 KiB and validates identity."
        quoted = '{"message": "' + stated + '"}'
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            (root / "stated.md").write_text(
                "# A contract\n\n" + stated + "\n\nIt also allows at most 16 files.\n")
            (root / "quoted.md").write_text(
                "# A contract\n\nIt states nothing numeric.\n\n```json\n" + quoted + "\n```\n")
            (root / "both.md").write_text(
                stated + "\n\n```json\n" + quoted + "\n```\n")
            (root / "unclosed.md").write_text(stated + "\n\n```json\n" + quoted + "\n")
            self.assertEqual(
                contracts.figures("stated.md", root=root), 2,
                "a bound this document states is no longer a figure, so a contract can publish one "
                "the census never sees and the count moves when the count is the only thing wrong")
            self.assertEqual(
                contracts.figures("quoted.md", root=root), 0,
                "a bound inside a fenced block is counted, so every transcript, shell example and "
                "message a document quotes moves the figure the census records")
            self.assertEqual(
                contracts.figures("both.md", root=root), 1,
                "a document stating a bound and quoting the same bound counts it twice, so quoting "
                "a figure is indistinguishable from claiming it")
            self.assertEqual(
                contracts.figures("unclosed.md", root=root), 1,
                "a fence that never closes does not run to the end of the document, so a long "
                "example's remaining lines are counted as claims the document makes")

            # The one shape this reading cannot absorb: a fence left open makes every line after it
            # a quotation, so a bound the document states below one stops being counted, the figure
            # recorded for it is smaller than what it publishes, and every check that compares
            # counts alone passes. So the documents this census records are asked whether they are
            # well formed, and the refusal names each one and the fence it left open.
            self.assertEqual(
                [], prose.unclosed(root / "both.md"),
                "a document whose fence it closes is called malformed: the reading closes a fence "
                "itself, so refusing one that is well formed is the rule refusing the document")
            self.assertIn(
                "line 3", "\n".join(prose.unclosed(root / "unclosed.md")),
                "the refusal does not say which line the fence opened on, so an author is left "
                "looking for it")

            # The shapes this reading cannot see rather than cannot absorb, and they fail in the
            # opposite direction: a fence it cannot absorb makes a bound below it stop counting,
            # while an indented block and a wide fence are quotations to a Markdown reader and
            # claims to this one, so a figure moves into either and every count-only check stays
            # green over a bound the document is only quoting. Nothing is parsed here either — a
            # line of either shape is left uncounted rather than guessed at — so the shape is
            # refused by the line it begins on, the way the unclosed fence is.
            (root / "indented.md").write_text(
                stated + "\n\n    A bound of 64 MiB, indented into a block\n\n"
                "It also allows at most 16 files.\n")
            (root / "wide.md").write_text(
                stated + "\n\n````text\nA bound of 64 MiB, held by the block\n"
                "A bound of 128 MiB, still inside it\n```\n")
            self.assertEqual(
                contracts.figures("indented.md", root=root), 2,
                "a bound written into an indented block is counted, so a document can move a "
                "figure into a shape this reading cannot see and keep the count it records")
            self.assertEqual(
                contracts.figures("wide.md", root=root), 1,
                "a fence opened with four markers closes here at the first three, so a block the "
                "author believes still holds its bounds is read as ending early and counted as a "
                "claim the moment a three-marker line appears inside it")
            self.assertEqual(
                [], prose.unseen(root / "quoted.md"),
                "a document whose fences it opens and closes is refused as unreadable: every shape "
                "asked about is in the reading, so the rule is refusing the document and not the "
                "shape, which would cost a well-formed contract a figure nothing took from it")
            self.assertIn(
                "line 3", "\n".join(prose.unseen(root / "indented.md")),
                "the refusal does not say which line the block begins on, so an author is left "
                "looking for it")
            self.assertIn(
                "4 markers", "\n".join(prose.unseen(root / "wide.md")),
                "the refusal does not say the block is opened wide, so an author is left to count "
                "the markers by hand to find the shape that cost the count")

            # The shape of a quotation is a class, and the two refusals above are two spellings
            # of it. A blockquote is a container, so the same shapes hide inside one: read
            # without the prefix, `> ` + four columns is an indented code block and `> ` + three
            # markers is a fence, and a reading that classified either by the bare line would
            # count a quoted bound as a claim with the count unchanged and nothing refused.
            (root / "quoted-indent.md").write_text(
                stated + "\n\n>     A bound of 64 MiB, indented inside a quote\n")
            (root / "quoted-wide.md").write_text(
                stated + "\n\n> ````text\n> A bound of 64 MiB, held by a quoted block\n> ```\n")
            (root / "quoted-fence.md").write_text(
                stated + "\n\n> ```text\n> A bound of 64 MiB, quoted inside a quote\n> ```\n")
            self.assertEqual(
                contracts.figures("quoted-indent.md", root=root), 1,
                "a block written into a blockquote's code block is counted, so a figure moves into "
                "a quotation the reading cannot see and the count it carries does not move")
            self.assertEqual(
                contracts.figures("quoted-fence.md", root=root), 1,
                "a fence inside a blockquote is not a fence to this reading, so every bound a "
                "document quotes inside one is counted as its own claim")
            self.assertEqual(
                [], prose.unseen(root / "quoted-fence.md"),
                "a fence this reading opens and closes is refused as unreadable: it read the block "
                "inside the quote, so refusing it would be refusing a document it can see whole")
            self.assertIn(
                "line 3", "\n".join(prose.unseen(root / "quoted-indent.md")),
                "a block inside a blockquote is not refused, or an author with a quoted code block "
                "is told to fence prose and cannot tell from this which line the walk meant")
            self.assertIn(
                "4 markers", "\n".join(prose.unseen(root / "quoted-wide.md")),
                "a wide fence inside a blockquote is not refused, so a block opened with four "
                "markers is refused or not depending on what quotes it, which is the same rule "
                "answering two different things about one shape")

            # Four columns are one number and the quote beside them is what decides what they
            # are: four columns behind the line that opens a quote is the block above, and
            # four columns behind a line that goes on inside a quote already open is that
            # paragraph's own wrapping, which is the same rule every wrapped line in this
            # file is held by. So the question is not what the columns are but whether the
            # quote was already open when the line arrived, and a walk that measures the
            # columns without asking turns a bound a document states into one it is shown —
            # dropping it out of the count, refusing a sentence nobody quoted, and sending
            # its author to fence prose the document never held in a block.
            (root / "lazy-quote.md").write_text(
                "> a step\n"
                ">     A bound of 512 KiB, four columns into a quote already open\n")
            self.assertEqual(
                contracts.figures("lazy-quote.md", root=root), 1,
                "a line four columns into a paragraph a quote already holds is that "
                "paragraph's own wrapping rather than a block, so reading it as a quotation "
                "drops a bound the document states out of the count and tells its author to "
                "fence a sentence it never quoted")
            self.assertEqual(
                [], prose.unseen(root / "lazy-quote.md"),
                "a line continuing a paragraph inside a quote is refused as a shape this "
                "reading cannot see, so an author with an ordinary wrapped blockquote is "
                "sent away from the census over the way blockquotes are written")

            # The other direction, and the one that costs: a line indented four columns is a code
            # block only when it is four columns past the content of the block it is in. A list
            # item that indents its own content by four is writing a paragraph, and a bound on
            # such a line is a claim the document makes. Refusing it drops a real figure out of
            # the count and tells the author to fence a line that was never a quotation.
            (root / "wrapped.md").write_text(
                stated + "\n\n1.  Read the plan, then check the registry holds every key,\n"
                "    allowing at most 16 files.\n")
            (root / "nested.md").write_text(
                "- a step\n  - a sub-step that is bounded to 8 MiB,\n    and wraps here\n")
            (root / "in-list.md").write_text(
                "- a step\n\n      A bound of 64 MiB, a block inside the item\n")
            self.assertEqual(
                contracts.figures("wrapped.md", root=root), 2,
                "a list item that indents its own content by four is writing a paragraph, and a "
                "bound on that line is a claim: refusing it drops a real figure out of the count")
            self.assertEqual(
                contracts.figures("nested.md", root=root), 1,
                "a nested bullet that indents its content by four is a paragraph too, so a bound "
                "written on its continuation line is a claim and not a quoted block")
            self.assertEqual(
                contracts.figures("in-list.md", root=root), 0,
                "a block six columns into a list item is four past the item's own content column, "
                "so it is a code block and a bound inside it is the block's, not the document's")
            self.assertEqual(
                [], prose.unseen(root / "nested.md"),
                "an ordinary nested bullet is refused as unreadable, so a document is turned away "
                "from the census over a list it wrote the ordinary way")
            self.assertIn(
                "line 3", "\n".join(prose.unseen(root / "in-list.md")),
                "a code block inside a list item is measured from the item's content column and "
                "not from column zero, so the refusal has to name the line that column reaches")

            # A heading is a block of its own, so it ends the paragraph above it however that
            # paragraph was written, and until it does only a blank line was ending one. That
            # is the whole weight of the arm: with the paragraph still open, four columns
            # below a heading are that item's own wrapping and a bound on them is a claim,
            # and with it closed the same four columns are a block and the bound is the
            # block's. A walk that reads a heading as one more line of the paragraph above it
            # cannot tell which of the two it is looking at, so it counts a bound the author
            # quoted as one the document states — the direction that loses a hold rather than
            # dropping one, and the direction nothing above here would notice.
            (root / "heading-in-item.md").write_text(
                "- a step\n  # heading\n"
                "      A bound of 512 KiB, four past the item's content\n")
            self.assertEqual(
                contracts.figures("heading-in-item.md", root=root), 0,
                "a heading closes the paragraph above it, so the four columns below one are a "
                "block rather than that item's own wrapping, and reading them as prose counts a "
                "bound the document only quotes")
            self.assertIn(
                "line 3", "\n".join(prose.unseen(root / "heading-in-item.md")),
                "a block below a heading is refused without naming the line it begins on, so an "
                "author is told a document cannot be read whole and cannot tell which of the "
                "three lines to mend")

            # Every reason, not the first: an author mends a file in one pass, and two shapes in
            # one document reported one at a time is a second run to find the second.
            (root / "two.md").write_text(
                "    A bound of 64 MiB, indented\n\n````text\nA bound of 128 MiB, held\n```\n")
            self.assertEqual(
                2, len(prose.unseen(root / "two.md")),
                "a document holding two shapes is told about one, so the author fixes what was "
                "named, runs the census again, and is sent back for a shape the first run saw")
            self.assertIn(
                "line 1", "\n".join(prose.unseen(root / "two.md")),
                "the reasons are returned but do not name their lines, so one run cannot tell the "
                "author which two lines to mend")

            # The columns a list marker's own spacing leaves are one question asked of the same
            # line twice, and a question answered in two places is answered two ways. One to four
            # spaces after a marker are spacing, so the item is writing a paragraph and the bound
            # on it is the item's own claim; a fifth space is not spacing, so the content starts
            # one column in and the text is four past it, which is a block inside the item. A
            # fence written after a marker is the item's fence, and an HTML comment is not
            # Markdown at all, so nothing under one is a block and a bound written there is a
            # remark about the document rather than a claim it makes. An item or a quote with
            # nothing after its marker holds nothing, so the lines below it are read one column
            # further right than a reader would guess: that is what keeps an empty item's own
            # spacing from reading as a block, and what makes four columns below it a block.
            (root / "five-spaced.md").write_text(
                stated + "\n\n-     A bound of 64 MiB, five columns into an item\n")
            (root / "four-spaced.md").write_text(
                stated + "\n\n-    A bound of 128 MiB, four columns after a marker\n")
            (root / "fenced-in-item.md").write_text(
                "- a step\n\n  ```json\n  A bound of 64 MiB, in a quoted envelope\n  ```\n")
            (root / "commented.md").write_text(
                stated + "\n\n<!--\nA bound of 256 MiB, inside a comment\n-->\n")
            (root / "empty-item.md").write_text(
                "-\n     A bound of 512 MiB, under an item that holds nothing\n")
            (root / "empty-quote.md").write_text(
                "- >\n      A bound of 64 MiB, under a quote that holds nothing\n")
            self.assertEqual(
                contracts.figures("five-spaced.md", root=root), 1,
                "five spaces after a marker are not spacing, so the bound is four columns past "
                "the item's own content, and a document can move a figure into a block this "
                "reading cannot see and keep the count it records")
            self.assertEqual(
                contracts.figures("four-spaced.md", root=root), 2,
                "four spaces after a marker are spacing, so refusing that line drops a real "
                "figure out of the count and tells an author to fence a paragraph")
            self.assertEqual(
                contracts.figures("fenced-in-item.md", root=root), 0,
                "a fence written after a list marker is a fence, so a bound a document quotes "
                "inside one is counted as its own claim and the item's column is then applied to "
                "the block that follows it")
            self.assertEqual(
                contracts.figures("commented.md", root=root), 1,
                "a bound written inside an HTML comment is a remark about the document rather "
                "than a claim it makes, so it is not a figure the census records")
            self.assertEqual(
                contracts.figures("empty-item.md", root=root), 1,
                "an item with nothing after its marker is read one column further right than its "
                "marker alone, so five columns below it is still the item's own prose, and "
                "refusing it drops a real figure out of the count")
            self.assertEqual(
                contracts.figures("empty-quote.md", root=root), 0,
                "a quote with nothing after it holds nothing, so the columns below it are read "
                "against the item the quote is in, and four past that is a block")
            refused = [name for name in ("commented.md", "empty-item.md", "fenced-in-item.md",
                                         "four-spaced.md")
                       if prose.unseen(root / name)]
            self.assertEqual(
                refused, [],
                "a document written the ordinary way is refused as unreadable, so a well-formed "
                "contract is turned away from the census over spacing, a comment or a quote")
            self.assertIn(
                "line 3", "\n".join(prose.unseen(root / "five-spaced.md")),
                "a block five columns into a list item is not refused, or an author is told to "
                "fence a line that is a quotation and cannot tell which line the walk meant")
            self.assertIn(
                "line 2", "\n".join(prose.unseen(root / "empty-quote.md")),
                "a block under a quote that holds nothing is not refused, so one rule answers two "
                "different things about one shape depending on what quotes it")

            # Three rules follow from the same measurement, and they are held here for the same
            # reason the six above are: a container that holds nothing states nothing, and a line
            # with no marker goes on inside a quote only while the quote holds a paragraph to go
            # on inside. So four columns below an item whose marker is bare are a block rather
            # than that item's own second line, a quote holding such an item ends with it instead
            # of running on into the columns below, and the content of a marker with five columns
            # of spacing is the marker and one space — five of them are not spacing, so the
            # item's content is not six columns to the right of it, and eight columns below that
            # are still a block.
            (root / "empty-then-block.md").write_text(
                "1.\n        A bound of 64 MiB, a block under an item that holds nothing\n")
            (root / "empty-item-in-quote.md").write_text(
                "> -\n    A bound of 64 MiB, a block under a quote holding an empty item\n")
            (root / "wide-marker.md").write_text(
                "-     \n        A bound of 64 MiB, a block under a marker with wide spacing\n")
            self.assertEqual(
                contracts.figures("empty-then-block.md", root=root), 0,
                "an item whose marker is bare states nothing, so a line four columns past the "
                "item's content is a block and not the item's own second line, and reading it as "
                "a claim counts a quoted bound that no count-only check notices")
            self.assertEqual(
                contracts.figures("empty-item-in-quote.md", root=root), 0,
                "a quote holding an item that holds nothing holds no paragraph, so a markerless "
                "line does not go on inside it and those columns are read from the document's "
                "own margin rather than from the item inside the quote")
            self.assertEqual(
                contracts.figures("wide-marker.md", root=root), 0,
                "five columns of spacing after a marker are not spacing, so the content of that "
                "item is the marker and one space, and eight columns below it are still a block: "
                "reading the spacing as the content turns a quoted bound into a claim")
            unnamed = [name for name in ("empty-item-in-quote.md", "empty-then-block.md",
                                         "wide-marker.md")
                       if "line 2" not in "\n".join(prose.unseen(root / name))]
            self.assertEqual(
                unnamed, [],
                "a block under a container that holds nothing is refused without naming the line "
                "it begins on, so an author is told a document cannot be read whole and cannot "
                "tell which of the three lines to mend")

            # The four-column rule says a block whatever the line says, and a list marker is
            # something: a marker four columns in is quoted text, not an item nested four
            # columns in. The count on the document above says the other half of the same rule,
            # that a line with nothing on it is not a quotation however its spacing reads, so
            # only the line that has text on it is named.
            (root / "marked-block.md").write_text(
                "    - A bound of 64 MiB, a block that opens an item\n")
            self.assertEqual(
                contracts.figures("marked-block.md", root=root), 0,
                "a list marker four columns into the document's own indent is quoted text rather "
                "than an item nested four columns in, so a bound on that line is the block's "
                "and not a claim, and reading it as a claim counts it")
            self.assertIn(
                "line 1", "\n".join(prose.unseen(root / "marked-block.md")),
                "a block that opens a list item is refused without naming its line, so an author "
                "with quoted text that looks like a list cannot tell which line the walk meant")
            self.assertEqual(
                len(prose.unseen(root / "wide-marker.md")), 1,
                "a line with nothing on it is refused as a quotation, so a marker line holding "
                "no text is named as a shape to mend and an author is sent to a line with "
                "nothing on it")

            def path_of(name: str) -> pathlib.Path:
                """Where a name this census records lives, stated here and not read from
                `contracts.figures`, which reads it the same way on purpose: an assertion that
                called the function it is checking would be the rule agreeing with itself. The
                two are written out separately so a census whose count disagrees with this is
                something the reading case can catch, and `FIGURES` has been spelled two ways in
                this tree's life — a bare filename under `docs/contracts/` and a repo-relative
                path — so the name is what says which.
                """
                return contracts.CONTRACTS / name if "/" not in name else contracts.ROOT / name

            # The key space the tables are written in and the root they are read from are one
            # decision, and this holds `figures` to it: every recorded name is counted by naming
            # it and nothing else, so a resolver that reads one spelling from the other's root is
            # named here rather than raised as `docs/contracts/docs/contracts/cli.md` — which is
            # how one branch's spelling and the other branch's default met in a merged tree and
            # every count in `FIGURES` became a number nobody took. It asks before the sweep below
            # reads the same names, so the answer is the thirty-seven names and not the first path.
            uncounted = []
            for name in sorted(contracts.FIGURES):
                try:
                    contracts.figures(name)
                except FileNotFoundError:
                    uncounted.append(name)
            self.assertEqual(
                uncounted, [],
                "a name the figure table records cannot be counted without naming a root, so what "
                "this census writes its names in and where it reads them from are two decisions "
                "rather than one, and every count below would have to name a root to be taken at "
                "all: " + ", ".join(uncounted))

            malformed = []
            for name in sorted(contracts.FIGURES):
                for why in prose.unclosed(path_of(name)):
                    malformed.append(f"{name} is not well formed: {why}")
            self.assertEqual(
                malformed, [],
                "a document the census records leaves a fence open, so the figures below it read as "
                "quotations and the count it carries stops describing what the document states — a "
                "bound it publishes would go unheld and every count-only check would stay green: "
                + "; ".join(malformed))

            unread = []
            for name in sorted(contracts.FIGURES):
                for why in prose.unseen(path_of(name)):
                    unread.append(f"{name} is not read whole: {why}")
            self.assertEqual(
                unread, [],
                "a document the census records holds a shape this reading cannot see, so a figure "
                "can be moved into it — indented, behind a wide fence, or behind either inside a "
                "blockquote — and the count it carries stays the number recorded, which is the one "
                "hole the unclosed-fence rule above does not close: fence the block, or open it "
                "with three markers: "
                + "; ".join(unread))

            # The case that was actually broken, against the document it happened in: `cli.md` as
            # this tree publishes it, and the same document with every figure it quotes quoted
            # again. Where `cli.md` quotes the frame bound — the envelope the CLI writes for a
            # request too large to send — this is the end-to-end half; where it does not, the three
            # fixtures above are what hold the rule and this compares a document to itself.
            published_name = next(one for one in sorted(contracts.FIGURES)
                                  if one.endswith("cli.md"))
            published = path_of(published_name).read_text()
            example = [line for line in published.splitlines() if contracts.BOUND.search(line)]
            (root / "quoted-twice.md").write_text(
                published + "\n```json\n" + "\n".join(example) + "\n```\n")
            # The document is named, not its spelling: `path_of` reads either, so this half
            # compares a count with the count the census records for the document itself.
            self.assertEqual(
                contracts.figures("quoted-twice.md", root=root),
                contracts.figures(published_name),
                "quoting cli.md's own figures a second time moves its figure count, so a document "
                "cannot publish the envelope its refusals are written in without the census moving "
                "with it — which is the collision that cost the request_refused example")


if __name__ == "__main__":
    unittest.main()
