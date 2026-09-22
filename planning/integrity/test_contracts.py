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
import unittest

import contracts

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
        """`BOUND` proven on the spellings it must read and the shapes it must not, so the count in
        `FIGURES` cannot move because the reading narrowed: each spelling below is a statement the
        census reads, and each shape after it a line that is not one — a version, a mode, a plain
        numeral, a worded count.
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


if __name__ == "__main__":
    unittest.main()
