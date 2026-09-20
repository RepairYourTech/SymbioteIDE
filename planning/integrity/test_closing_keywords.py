"""Run: python3 planning/integrity/test_closing_keywords.py."""

import io
import re
import subprocess
import tempfile
import unittest
from contextlib import redirect_stdout
from pathlib import Path

from closing_keywords import SNAPSHOT, TITLE_CHANNEL, closings, main, report

HERE = Path(__file__).resolve().parent


def run(body, *arguments):
    """`main` over `body`, returning its exit code and printed lines."""
    with tempfile.TemporaryDirectory() as directory:
        path = Path(directory) / "body.md"
        path.write_text(body, encoding="utf-8")
        output = io.StringIO()
        with redirect_stdout(output):
            code = main(["--body-file", str(path), *arguments])
    return code, output.getvalue()


class ClosingKeywordTests(unittest.TestCase):
    def test_real_530_description_is_reported_as_closing_54(self):
        # PR #530's description, verbatim: GitHub's parser ignored the negation
        # and closed #54 one second after the merge.
        body = "Refs #54 — a slice, not its completion. This does not close #54.\n\n## Why\n"
        code, output = run(body)
        self.assertEqual([closing.reference for closing in closings(body)], ["#54"])
        self.assertEqual(code, 1)
        self.assertIn('Merging closes #54 (keyword "close").', output)
        self.assertIn('"This does not close #54"', output)

    def test_real_507_description_closes_only_the_slice_it_names(self):
        # PR #507's description, verbatim: an intended closure, and the one
        # form that is honest about what stays open.
        body = (
            "Closes #218 (slice: the sandbox launch executor composition behind "
            "`WorkerTransports`; #466 approval flows and #465 remain open).\n"
        )
        code, output = run(body)
        self.assertEqual([closing.reference for closing in closings(body)], ["#218"])
        self.assertEqual(code, 0)
        self.assertNotIn("FAIL", output)

    def test_a_reference_without_a_keyword_closes_nothing(self):
        code, output = run("Refs #549 — a slice, not its completion.\n")
        self.assertEqual(code, 0)
        self.assertEqual(closings("Refs #549 — a slice, not its completion."), [])
        self.assertIn("closes nothing", output)

    def test_documented_forms_including_colons_and_case(self):
        body = "CLOSES: #10, resolves #123, Resolves octo-org/octo-repo#100\n"
        self.assertEqual(
            [closing.reference for closing in closings(body)],
            ["#10", "#123", "octo-org/octo-repo#100"],
        )
        self.assertEqual(run(body)[0], 0)

    def test_a_keyword_inside_a_word_is_not_a_keyword(self):
        code, output = run("The disclosed #5 wording stayed closed-source #5.\n")
        self.assertEqual(code, 0)
        self.assertIn("closes nothing", output)

    def test_negation_outside_the_keywords_clause_does_not_flag_it(self):
        body = "This does not complete the work; the slice is done.\nCloses #12.\n"
        self.assertEqual([closing.reference for closing in closings(body)], ["#12"])
        self.assertEqual(run(body)[0], 0)

    def test_each_sentence_is_judged_on_its_own(self):
        body = "We do NOT fix #7. Fixes #8.\n"
        code, output = run(body)
        self.assertEqual([closing.reference for closing in closings(body)], ["#7", "#8"])
        self.assertEqual(code, 1)
        self.assertIn('"We do NOT fix #7"', output)
        self.assertNotIn("Fixes #8", output)

    def test_a_negation_split_across_commas_is_still_read(self):
        code, output = run("This does not, however, close #54.\n")
        self.assertEqual(code, 1)
        self.assertIn('"This does not, however, close #54"', output)

    def test_a_closure_buried_in_a_sentence_is_refused(self):
        # The description form of the accident: the keyword and the reference
        # are adjacent inside prose that reads as a remark, and GitHub closes
        # the issue all the same.
        code, output = run("This lands the slice, and the defect that closed #54 is fixed.\n")
        self.assertEqual(code, 1)
        self.assertIn("buries a closing keyword", output)
        self.assertIn('"Closes #54"', output)

    def test_the_real_551_commit_message_is_refused_as_a_commit_message(self):
        # The squash-merge commit of PR #551, verbatim: its last sentence
        # closed #54 at 2026-09-12T21:00:18Z, the issue that PR was written to
        # protect.
        body = (
            "chore(planning): fail a pull request whose prose negates a closing keyword (#551)\n"
            "\n"
            "GitHub's issue parser does not read negation, so a description that denies a\n"
            "closure still performs it: #54 was closed one second after PR #530 merged,\n"
            "whose own first line disclaimed the closure. Add planning/integrity/\n"
            "closing_keywords.py, which reports the issues a description will close and\n"
            "fails when a keyword it honours sits in a clause that negates it, plus a\n"
            "roadmap-integrity job that runs it on every pull request. The tests pin the\n"
            "verbatim descriptions of PR #530 and PR #507, so the defect that closed #54 is\n"
            "reproduced rather than described.\n"
        )
        description, said = run(body)
        self.assertEqual(description, 1)
        self.assertIn("buries a closing keyword", said)
        code, output = run(body, "--commits", "--label", "commit 5bf7437f")
        self.assertEqual(code, 1)
        self.assertIn("the commit 5bf7437f carries a closing keyword", output)
        self.assertIn("so the defect that closed #54", output)

    def test_the_real_546_title_is_read_as_a_title(self):
        # PR #546's title, verbatim: this repository's style of citing issue
        # numbers in parentheses, which the title rule must not refuse. It
        # became that merge's subject, because the branch carried more than
        # one commit.
        title = (
            "fix(runtime): the Host owns the external harness lane — its launch, "
            "ceiling, worktree and wall time (#229, #464)\n"
        )
        code, output = run(title, "--title")
        self.assertEqual(code, 0)
        self.assertIn("closes nothing", output)

    def test_a_landed_title_citing_an_issue_number_passes(self):
        # PR #554's title, verbatim.
        code, output = run(
            "chore(planning): refuse a closing keyword in a commit message (#54)\n",
            "--title",
        )
        self.assertEqual(code, 0)
        self.assertIn("closes nothing", output)

    def test_a_title_that_states_a_closure_is_still_refused(self):
        # `Closes #218` is the form a description is told to use, and the
        # reason the title carries its own rule: the merged subject is read by
        # GitHub, not by a reviewer.
        self.assertEqual(run("Closes #218.\n")[0], 0)
        code, output = run("Closes #218.\n", "--title")
        self.assertEqual(code, 1)
        self.assertIn("the pull-request title carries a closing keyword", output)
        self.assertIn("becomes the merged commit's subject", output)

    def test_a_hazardous_title_is_refused(self):
        # A title with a keyword buried in it, of the kind that closed #54 when
        # it arrived through a commit message.
        code, output = run("fix(host): the preflight that closed #54 now refuses\n", "--title")
        self.assertEqual(code, 1)
        self.assertIn('"fix(host): the preflight that closed #54"', output)

    def test_a_commit_message_may_not_close_anything_even_in_a_stated_clause(self):
        code, output = run("Closes #218.\n", "--commits")
        self.assertEqual(code, 1)
        self.assertIn("Nothing in review reads a commit message", output)

    def test_keywords_on_a_non_default_base_close_nothing(self):
        body = "This does not close #54.\n"
        code, output = run(body, "--base", "release")
        self.assertEqual(code, 0)
        self.assertIn("not main, the default branch", output)
        self.assertNotIn("FAIL", output)

    def test_report_names_every_closure_and_every_hazard(self):
        lines, failed = report(closings("Closes #1. Does not fix #2.\n"))
        self.assertTrue(failed)
        self.assertEqual(
            lines[0:2],
            [
                'Merging closes #1 (keyword "Closes").',
                'Merging closes #2 (keyword "fix").',
            ],
        )
        self.assertIn('"Does not fix #2"', lines[2])


def merged(revision):
    """The pull requests merged into `revision`: a first-parent merge commit, or a squash whose
    subject names the pull request it landed. None where this checkout holds no such history.
    """
    log = subprocess.run(["git", "log", "--first-parent", revision, "--format=%s%x00%P"],
                         cwd=HERE, capture_output=True, text=True)
    if log.returncode != 0:
        return None
    found = []
    for line in log.stdout.splitlines():
        subject, parents = line.split("\x00")
        if len(parents.split()) > 1 or re.search(r"\(#\d+\)$", subject):
            found.append(subject)
    return found


class ReadmeSnapshot(unittest.TestCase):
    """The pull-request section states a snapshot of `main` at a revision, and this is what holds
    it: two of its figures are re-measured from that revision's own merged history, the rest are
    the service's record of each pull request, which this tree does not hold and which
    `closing_keywords.SNAPSHOT` therefore declares once.
    """

    def test_the_readme_states_the_snapshot_this_rule_declares(self):
        readme = (HERE / "README.md").read_text()
        claimed = {
            "descriptions": (r"of the (\d+) descriptions merged up to", SNAPSHOT["merges"]),
            "refused_descriptions": (r"exactly (one|\d+) trips it", SNAPSHOT["refused_descriptions"]),
            "merged": (r"of the (\d+) pull requests merged up to", SNAPSHOT["merges"]),
            "multi_commit": (r"(\d+) carried more than one commit", SNAPSHOT["multi_commit"]),
            "title_as_subject": (r"and (\d+) have a subject that is exactly",
                                 SNAPSHOT["title_as_subject"]),
            "refused_titles": (r"refuses none of the (\d+) merged titles", SNAPSHOT["merges"]),
        }
        words = {"one": 1, "none": 0}
        for label, (pattern, expected) in claimed.items():
            found = re.search(pattern, readme)
            self.assertIsNotNone(found, f"the README no longer states the {label} of its "
                                        f"pull-request snapshot")
            stated = words.get(found.group(1), found.group(1))
            self.assertEqual(int(stated), expected,
                             f"the README states {found.group(0)!r} where the snapshot declares "
                             f"{expected}: it is a snapshot of one revision, so the two move "
                             f"together or the figure has drifted")
        self.assertIn(f"PR {SNAPSHOT['refused_description']}", readme,
                      f"the README no longer names the one description that trips the rule "
                      f"({SNAPSHOT['refused_description']})")
        named = set(re.findall(r"(?:merged up to|snapshot of `main` at) `([0-9a-f]{7,40})`", readme))
        self.assertEqual(named, {SNAPSHOT["revision"]},
                         f"the README pins these counts to {sorted(named)} where the snapshot "
                         f"declares {SNAPSHOT['revision']}")

    def test_the_snapshot_is_of_a_revision_this_tree_holds_and_its_title_channel_reads_clean(self):
        revision = SNAPSHOT["revision"]
        landed = merged(revision)
        self.assertIsNotNone(landed, f"this checkout holds no history for {revision}: the "
                                      f"snapshot's counts are of that revision, so re-measuring "
                                      f"them needs it (a shallow clone or a source export is refused "
                                      f"here rather than passed)")
        self.assertEqual(len(landed), SNAPSHOT["merges"],
                         f"{revision} has {len(landed)} merges and the snapshot declares "
                         f"{SNAPSHOT['merges']}")
        refused = [subject for subject in landed
                   if report(closings(subject), "pull-request title", TITLE_CHANNEL)[1]]
        self.assertEqual(len(refused), SNAPSHOT["refused_titles"],
                         f"the title rule refuses {refused}, where the snapshot declares "
                         f"{SNAPSHOT['refused_titles']} of {revision}'s merged subjects")


if __name__ == "__main__":
    unittest.main()
