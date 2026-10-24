"""Run: python3 planning/integrity/test_mutation_probe.py.

The probe's table is a claim about this tree, and it is held here without running cargo: every
row's claim is text its document holds exactly once, every row names a case the census records as
holding that document and a selector that reaches a target the crate has, the self-proof's prose is
text a witness really does not read, and every case the census records as holding a document has a
row at all. A moved anchor or a renamed case fails in a second here rather than becoming a row
`mutation_probe.py` can no longer run — and a row it cannot run is a row that would otherwise sit in
the table looking proved.

One thing this deliberately does not do: it does not run cargo, so it cannot tell whether a claim
is *held*. That is the driver's work and the two are separate on purpose, because the driver needs
a toolchain and a build and this suite is the one that runs in a second in the integrity job
before the toolchain is spent. Completeness is the other half and it needs no build: whether a case
the census records as holding a document has a row is a question about two tables, so it is answered
here.

Nor is the crate a row's to name: a row names the case, the census says which crate holds it, and
the comparison the first version of this file made — that the row's crate and case equal the
census's — is a check that no longer has a state to be written in. What replaced it is the census
holding every case that reads each document, so `work-hierarchy.md`'s two holders are both stated
there, and this file asking the other direction: every one of them has a row.
"""
from __future__ import annotations

import pathlib
import unittest

import contracts
import mutation_probe

ROOT = pathlib.Path(__file__).resolve().parents[2]
CRATES = ROOT / "crates"


def reaches(crate: str, selector: str) -> bool:
    """Whether a selector names something the crate has. A selector is written the way it is given
    to cargo, so this is the reading of the three: a test target is a file under the crate's
    `tests/`, a binary is under its `src/bin/`, and `--lib` is the library's own unit tests — which
    every crate here has, so the name is never one to check."""
    words = selector.split()
    folder = CRATES / crate
    if not folder.is_dir():
        return False
    if words == ["--lib"]:
        return True
    if len(words) != 2 or words[0] not in ("--test", "--bin"):
        return False
    name = words[1]
    if words[0] == "--test":
        return any((folder / "tests").glob(f"{name}.rs"))
    return (folder / "src" / "bin" / f"{name}.rs").is_file() or (
        folder / "src" / "bin" / name / "main.rs").is_file()


class MutationProbeTable(unittest.TestCase):
    def test_the_driver_roots_itself_at_this_repository(self):
        """The driver resolves its own root from its own path, so a copy of this file run from
        elsewhere still probes the tree it belongs to rather than whatever it was pointed at."""
        self.assertEqual(mutation_probe.ROOT, ROOT)

    def test_every_row_names_a_label_a_document_a_case_and_a_different_claim(self):
        """Every row's shape: a label, a document this tree holds, a case the census records as
        holding that document, a replacement that differs from the claim it replaces, and a claim the
        replacement does not contain. A row that changed a claim into itself would pass every run
        while proving nothing, which is the one failure a table of proofs must not be able to have;
        a replacement still containing the claim is read green by a case that finds the text with
        `contains` anyway, which is how the `runtime-sdk.md` row first passed over a hold that was
        not there."""
        self.assertTrue(mutation_probe.CLAIMS, "the table states no claim to revert")
        for claim in mutation_probe.CLAIMS:
            with self.subTest(row=claim.label):
                self.assertTrue(claim.label.strip(), "a row states no label for a failure to name it by")
                self.assertNotEqual(claim.claim, claim.replacement,
                                    f"{claim.label} changes {claim.claim!r} into itself, so it "
                                    f"proves nothing")
                self.assertNotIn(
                    claim.claim, claim.replacement,
                    f"{claim.label} replaces {claim.claim!r} with text that still contains it, so a "
                    f"case reading the document with `contains` finds the claim anyway and the row "
                                    f"goes green on a hold that is not there")
                self.assertTrue((ROOT / claim.document).is_file(),
                                f"{claim.label} names {claim.document}, which this tree does not hold")
                self.assertIsNotNone(
                    contracts.holder(claim.document, claim.case),
                    f"{claim.label} names {claim.case}, which the census does not record as a case "
                    f"holding {claim.document}: a row proves the census's claim, so a case the "
                    f"census does not name for that document is a claim of its own")

    def test_every_claim_is_written_once_in_the_document_it_names(self):
        """Each claim and its replacement written exactly once in that document: `swap` needs one
        occurrence to change and leaves the rest alone, so a claim that appears twice is a row that
        would change a sentence the witness does not read and report the wrong result."""
        for claim in (*mutation_probe.CLAIMS, mutation_probe.SELF_PROOF):
            with self.subTest(row=claim.label):
                text = (ROOT / claim.document).read_text()
                self.assertEqual(text.count(claim.claim), 1,
                                 f"{claim.label}: {claim.document} writes {claim.claim!r} "
                                 f"{text.count(claim.claim)} times, where the row needs once")
                self.assertEqual(text.count(claim.replacement), 0,
                                 f"{claim.label}: {claim.document} already writes the replacement "
                                 f"{claim.replacement!r}, so the mutation would change nothing")

    def test_every_selector_reaches_a_target_the_crate_carries(self):
        """`SELECTORS` against the census and the tree: one entry per case the census records as
        holding a document — a holder nothing names is a claim this tree cannot prove, and an entry
        for a case no document records is a target this driver would run for nobody — and each
        selector naming something the crate actually has. A case the crate does not carry is the
        census's own case to refuse; what a selector adds is the cargo half of reaching it."""
        recorded = {(crate, case)
                    for holders in contracts.HELD.values() for crate, case in holders}
        self.assertEqual(
            sorted(set(mutation_probe.SELECTORS) - recorded), [],
            f"SELECTORS names {len(set(mutation_probe.SELECTORS) - recorded)} case the census "
            f"records no document as being held by, so a target would be run for a claim nobody "
            f"makes")
        self.assertEqual(
            sorted(recorded - set(mutation_probe.SELECTORS)), [],
            f"the census records {len(recorded - set(mutation_probe.SELECTORS))} case no selector "
            f"states how to reach, so a claim the census names is one this driver cannot prove")
        for (crate, case), selector in sorted(mutation_probe.SELECTORS.items()):
            with self.subTest(case=case):
                self.assertTrue(
                    reaches(crate, selector),
                    f"{case} in {crate} is reached with {selector!r}, which selects nothing "
                    f"there: a test target is a file under the crate's tests/ or a binary under "
                    f"its src/bin/")

    def test_the_self_proof_names_prose_no_row_changes(self):
        """The self-proof exists to tell a row's red from a suite that was already broken, and it
        only does that if it changes something no row changes. A self-proof that edits a claim
        would go red for the same reason a row does, which is the one state it is meant to rule
        out."""
        proof = mutation_probe.SELF_PROOF
        self.assertTrue(proof.claim.strip(), "the self-proof names no prose to change")
        self.assertNotEqual(proof.claim, proof.replacement,
                            f"the self-proof changes {proof.claim!r} into itself, so it proves "
                            f"nothing")
        claims = {claim.claim for claim in mutation_probe.CLAIMS}
        self.assertNotIn(proof.claim, claims,
                         "the self-proof changes a claim a row also changes, so it cannot rule "
                         "out the failure it exists to rule out")
        self.assertNotIn(proof.replacement, claims,
                         "the self-proof writes a claim a row also changes")

    def test_every_case_the_census_records_as_holding_a_document_has_a_row(self):
        """The direction that makes the table worth running, stated over the census's holders rather
        than its documents: a case the census records as holding a document is a claim that *that
        case* compares it, and nothing but a row measures that. A document held with no row is a
        claim nothing measures, which is what a new entry in `HELD` is until a row names it; and
        because the census now records every holder, `work-hierarchy.md`'s two — the domain's bound
        check and the host's measured answer — each need a row of their own rather than one row
        standing for the document.

        What made this worth asking in the first place: the census's own check asks whether a *file*
        in the crate reads the document, which a case that reads a different document out of the
        same file satisfies — that is how `cli.md` was held by a case that never looked at it, with
        every case in this directory green; and a row that merely probed the document with some
        other case would pass whatever the census said, which is the state the first version of this
        case allowed, the row naming the case that really holds `cli.md` and the census naming one
        that does not. A row cannot name a case the census does not record, so the mismatch has no
        spelling left.
        """
        probed = {(claim.document, claim.case) for claim in mutation_probe.CLAIMS}
        unprobed = sorted(
            f"{document} is held by {crate}'s {case}"
            for document, holders in contracts.HELD.items()
            for crate, case in holders
            if (document, case) not in probed)
        self.assertEqual(
            unprobed, [],
            f"the census records a case that holds a document and the probe has no row naming it: "
            f"{'; '.join(unprobed)} — add a row naming the document, the text to change and the "
            f"case that must go red")

    def test_a_leftover_backup_is_refused_rather_than_overwritten(self):
        """The guard that protects a run killed between its write and its restore: the driver
        refuses to start when a backup is already on disk, so a document a previous run left
        mutated is recovered rather than overwritten by a second mutation."""
        leftover = mutation_probe.leftover()
        self.assertEqual(
            leftover, [],
            f"a previous probe left {', '.join(leftover)} behind, so it was killed between its "
            f"write and its restore; restore the document from the backup before probing again")


if __name__ == "__main__":
    unittest.main()
