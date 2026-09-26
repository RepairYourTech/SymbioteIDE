"""Run: python3 planning/integrity/test_contracts.py.

Hold the census of `docs/` to the tree it census.

The record of the passes that measured those documents said the census was complete, and nothing
read that sentence. This file reads it: every document the census covers is classified exactly once,
every classification that names a holding case is a case the crate really carries and one that
really reads the document, every Markdown document states the figures the census recorded for it,
and the reading that counts them is the one the census states — proven on the spellings it must read
and the shapes it must not.

The census covers two sets of documents under one table family, because a bound published outside
`docs/contracts/` is as unheld as one published inside it: every entry directly under the directory,
whatever it is, and every Markdown document outside it that states a figure. So each half of the
classification is stated once over the union of the two rather than twice over the two.

What nothing here decides: whether a held figure is the right one to hold, and whether a sentence's
meaning is true. The reading counts statements in the shapes `contracts.BOUND` names; a bound written
in another shape is a figure this census does not see, and a line it does see keeps its count when its
*value* changes, which is the case that holds the document refusing — and what that case compares is
`mutation_probe.py`'s row to measure rather than this file's business.
"""
from __future__ import annotations

import pathlib
import re
import unittest

import contracts

ROOT = pathlib.Path(__file__).resolve().parents[2]
CRATES = ROOT / "crates"


def defines_case(crate: str, case: str) -> bool:
    """Whether `crate` carries a test function named `case`. The harness prints `test <name> ... ok`,
    so the name is what the driver `mutation_probe.py` runs and this is the reading of it that needs
    no build: a case renamed or deleted is a claim the census states about a case that is gone."""
    folder = CRATES / crate
    return folder.is_dir() and any(
        re.search(rf"\bfn {re.escape(case)}\b", source.read_text(errors="replace"))
        for source in folder.rglob("*.rs"))


class TheCensus(unittest.TestCase):
    def test_every_document_the_census_covers_is_classified_once(self):
        """An entry nothing classifies, a classification no document holds, and a document in both
        tables, each named: a document added to either set is a surface this census has not decided
        about, and one removed is a line here that decides about nothing.
        """
        found = contracts.documents()
        held, unheld = set(contracts.HELD), set(contracts.NOT_HELD)
        both = sorted(held & unheld)
        self.assertEqual(both, [], f"these documents are both held and not held: {both}")
        classified = held | unheld
        inside = sorted(one for one in set(found) - classified if one.startswith("docs/contracts/"))
        self.assertEqual(
            inside, [],
            f"{', '.join(inside)} sits under docs/contracts/ and no census entry classifies it: a "
            f"document a case reads is held by naming its cases, and one no case reads is stated "
            f"with why — a contract added without either is a surface nothing holds")
        outside = sorted(one for one in set(found) - classified if not one.startswith("docs/contracts/"))
        self.assertEqual(
            outside, [],
            f"{', '.join(outside)} publishes a figure outside docs/contracts/ and no census entry "
            f"classifies it: a document a case reads is held by naming its cases, and one no case "
            f"reads is stated with why — a bound published without either is unheld")
        stale = sorted(classified - set(found))
        self.assertEqual(
            stale, [],
            f"the census classifies {', '.join(stale)}, which is not a document it covers: a "
            f"classification of a document that is gone decides about nothing")

    def test_every_markdown_document_states_the_figures_the_census_records(self):
        """Every Markdown document against the count `contracts.FIGURES` records: a document that
        gains a statement in one of the shapes `BOUND` reads fails here naming the document and both
        numbers, so a new bound cannot enter a document without the census moving in that change.
        """
        documents = contracts.markdown()
        self.assertEqual(
            sorted(contracts.FIGURES), documents,
            "the figure table names " + ", ".join(sorted(set(contracts.FIGURES) - set(documents)))
            or "the figure table names no document the census covers")
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
        """Every case every held document names: the crate must be one in this tree, the case must
        exist in it, and that crate must read the document. A holding case renamed or deleted fails
        here naming both, so the census cannot keep claiming a hold nothing makes.

        The tie is at the crate rather than the case's own file because a crate may reach its
        contract through a path constant instead of a literal — `symbiote-constitution` reads
        `constitution.md` through `claims::CONTRACT_PATH`, which a reading of literals cannot
        follow. What is held here is therefore that the case exists and the crate reads the
        document; that the case is what reads it is the case's own text, which no reading decides
        and which `mutation_probe.py`'s row is what measures.

        Two holders of one document are named by the crate, not by their case alone, because two
        crates declare a case of the same name — and two holders of one document sharing a case
        name would leave `contracts.holder` unable to say which crate a row is proved against, so
        that is refused here rather than resolved by table order.
        """
        broken = []
        for name, holders in sorted(contracts.HELD.items()):
            named = [case for _crate, case in holders]
            if len(set(named)) != len(named):
                broken.append(f"{name} is held by {', '.join(sorted(set(n for n in named if named.count(n) > 1)))}, "
                              f"which names one case twice, so a row naming it cannot say which "
                              f"crate to prove it against")
            for crate, case in holders:
                folder = CRATES / crate
                if not folder.is_dir():
                    broken.append(f"{name} is held by {crate}, which is not a crate in this tree")
                elif not defines_case(crate, case):
                    broken.append(f"{name} is held by {case} in {crate}, and no case in that "
                                  f"crate is named that")
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

    def test_every_document_no_case_reads_states_why(self):
        """The other half of the classification: a document with no reason, and one whose reason is
        the empty string, each refused — an unexplained entry reads as a decision nobody made, and
        in a security or proof document the shape of that decision is the whole point.
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
