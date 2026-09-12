"""Run: python3 planning/housekeeping/test_prune_merged_checkouts.py."""

import io
import os
import subprocess
import tempfile
import unittest
from contextlib import redirect_stdout
from pathlib import Path
from unittest import mock

from prune_merged_checkouts import Branch, local_branches, main, prune_plan


def branch(name, **evidence):
    """A branch whose only evidence is what the test states."""
    return Branch(
        name=name,
        current=evidence.pop("current", False),
        contained=evidence.pop("contained", False),
        upstream_gone=evidence.pop("upstream_gone", False),
        **evidence,
    )


def decisions(branches, merged=()):
    return {decision.name: decision for decision in prune_plan(branches, merged)}


class PlanTests(unittest.TestCase):
    def test_merged_pull_request_with_a_gone_remote_is_removed(self):
        decision = decisions([branch("landed", upstream_gone=True)], ["landed"])
        self.assertTrue(decision["landed"].remove)
        self.assertIn("remote branch is gone", decision["landed"].reason)

    def test_merged_pull_request_in_an_unpushed_branch_is_removed(self):
        self.assertTrue(decisions([branch("landed")], ["landed"])["landed"].remove)

    def test_contained_branch_is_removed_even_without_a_merged_pull_request(self):
        # The one branch the earlier cleanup kept, whose tip is an ancestor of
        # main although no merge names it.
        decision = decisions([branch("contained", contained=True)])
        self.assertTrue(decision["contained"].remove)
        self.assertIn("content is preserved", decision["contained"].reason)

    def test_current_branch_is_kept(self):
        decision = decisions(
            [branch("main", current=True, contained=True)], ["main"]
        )
        self.assertFalse(decision["main"].remove)
        self.assertIn("current branch", decision["main"].reason)

    def test_uncommitted_worktree_changes_are_kept(self):
        decision = decisions(
            [branch("landed", worktree="/tmp/landed", worktree_dirty=True)], ["landed"]
        )
        self.assertFalse(decision["landed"].remove)
        self.assertIn("/tmp/landed", decision["landed"].reason)

    def test_a_clean_worktree_does_not_block_removal(self):
        decision = decisions(
            [branch("landed", worktree="/tmp/landed")], ["landed"]
        )
        self.assertTrue(decision["landed"].remove)
        self.assertEqual(decision["landed"].worktree, "/tmp/landed")

    def test_branch_without_landing_evidence_is_kept(self):
        decision = decisions([branch("open-pr")], ["some-other-branch"])
        self.assertFalse(decision["open-pr"].remove)
        self.assertIn("no merged pull request", decision["open-pr"].reason)


class RepositoryTests(unittest.TestCase):
    """`local_branches` and `--apply` against a real throwaway repository."""

    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.previous = os.getcwd()
        os.chdir(self.directory.name)
        self.addCleanup(os.chdir, self.previous)
        self.git("init", "--initial-branch=main", "--quiet")
        self.git("config", "user.email", "proof@example.invalid")
        self.git("config", "user.name", "Proof")
        (Path(self.directory.name) / "file").write_text("one\n")
        self.git("add", "file")
        self.git("commit", "--quiet", "-m", "one")

    def git(self, *arguments):
        subprocess.run(["git", *arguments], check=True, capture_output=True)

    def run_tool(self, *arguments):
        """`main`, without the plan it prints to standard output."""
        with mock.patch("prune_merged_checkouts.merged_heads", return_value=[]):
            with redirect_stdout(io.StringIO()):
                return main([*arguments, "--default-branch", "main"])

    def test_local_branches_reads_containment_and_the_checkout(self):
        self.git("branch", "landed")
        branches = {entry.name: entry for entry in local_branches("main")}
        self.assertTrue(branches["landed"].contained)
        self.assertFalse(branches["landed"].current)
        self.assertNotIn("main", branches)

    def test_the_default_branch_is_not_a_candidate_when_it_is_a_remote_ref(self):
        self.git("branch", "landed")
        names = [entry.name for entry in local_branches("origin/main")]
        self.assertNotIn("main", names)
        self.assertIn("landed", names)

    def test_apply_removes_a_contained_branch_and_keeps_the_working_tree(self):
        self.git("branch", "landed")
        self.assertEqual(self.run_tool("--apply"), 0)
        listed = subprocess.run(
            ["git", "branch", "--format=%(refname:short)"],
            capture_output=True,
            text=True,
            check=True,
        ).stdout.split()
        self.assertEqual(listed, ["main"])
        self.assertTrue(Path(self.directory.name, "file").exists())

    def test_dry_run_removes_nothing(self):
        self.git("branch", "landed")
        self.run_tool()
        listed = subprocess.run(
            ["git", "branch", "--format=%(refname:short)"],
            capture_output=True,
            text=True,
            check=True,
        ).stdout.split()
        self.assertCountEqual(listed, ["main", "landed"])


if __name__ == "__main__":
    unittest.main()
