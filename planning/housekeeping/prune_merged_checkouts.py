#!/usr/bin/env python3
"""Prune local branches and worktrees whose work has already landed.

Merges here are squashes, so a landed branch tip is never an ancestor of
`origin/main`: after the merge GitHub deletes the remote branch and, with it,
the only cheap signal that the checkout is finished with. 59 registered
worktrees and 70 local branches survived their merges at once, holding ~330 GB
of build caches and producing phantom diffs against superseded heads.

This decides what is safe to remove from evidence — the branch tip is
contained in the default branch, or its pull request is merged — and never
touches the current branch, a branch checked out in a worktree with
uncommitted changes, or a branch with no landing evidence. Dry run by default;
`--apply` removes worktrees first (`git worktree remove`, which refuses a dirty
worktree on its own) and then the branches. It runs no `git push` and mutates
nothing outside the local repository.

    python3 planning/housekeeping/prune_merged_checkouts.py [--apply]
"""

import argparse
import re
import subprocess
import sys
from dataclasses import dataclass, replace


@dataclass(frozen=True)
class Decision:
    """One local branch and the verdict about it."""

    name: str
    worktree: str = ""
    remove: bool = False
    reason: str = ""


@dataclass(frozen=True)
class Branch:
    """One local branch, and the evidence about whether its work landed."""

    name: str
    current: bool
    contained: bool
    upstream_gone: bool
    worktree: str = ""
    worktree_dirty: bool = False
    pull_request_merged: bool = False

    def plan(self):
        """Whether to remove this branch, and the reason either way."""
        if self.current:
            return Decision(self.name, reason="the current branch")
        if self.worktree and self.worktree_dirty:
            return Decision(
                self.name,
                self.worktree,
                reason=f"its worktree at {self.worktree} has uncommitted changes",
            )
        if self.contained:
            return Decision(
                self.name,
                self.worktree,
                remove=True,
                reason="contained in the default branch, so its content is preserved",
            )
        if self.pull_request_merged and self.upstream_gone:
            return Decision(
                self.name,
                self.worktree,
                remove=True,
                reason="its pull request is merged and its remote branch is gone",
            )
        if self.pull_request_merged:
            return Decision(
                self.name,
                self.worktree,
                remove=True,
                reason="its pull request is merged",
            )
        return Decision(
            self.name,
            self.worktree,
            reason="no merged pull request and not contained in the default branch",
        )


def prune_plan(branches, merged_heads):
    """The plan for `branches`, given the head branch names of merged pull
    requests. A merged head is matched by name: the merge is a squash, so the
    name is the only evidence that a local branch corresponds to the merge."""
    merged = set(merged_heads)
    return [
        replace(branch, pull_request_merged=branch.name in merged).plan()
        for branch in branches
    ]


def render(plan):
    """One line per branch: the verdict, the name, and the reason."""
    return [
        f"{'remove' if decision.remove else 'keep  '} {decision.name:<40} "
        f"{decision.reason}"
        for decision in plan
    ]


def git(*arguments, cwd=None):
    return subprocess.run(
        ["git", *arguments], cwd=cwd, capture_output=True, text=True, check=True
    ).stdout


def local_branches(default_branch):
    """Every local branch with its containment and checkout evidence. The
    default branch itself is not a candidate: its tip is trivially contained
    in the default branch, and the only reason a plan would not remove it is
    that it is checked out here."""
    default = default_branch.rsplit("/", 1)[-1]
    worktrees = {}
    for block in git("worktree", "list", "--porcelain").split("\n\n"):
        branch = re.search(r"(?m)^branch refs/heads/(.+)$", block)
        if branch:
            path = re.search(r"(?m)^worktree (.+)$", block)
            worktrees[branch.group(1)] = path.group(1) if path else ""
    branches = []
    for line in git(
        "for-each-ref",
        "--format=%(refname:short)%09%(upstream:track)%09%(HEAD)",
        "refs/heads",
    ).splitlines():
        name, track, head = line.split("\t")
        if name == default:
            continue
        worktree = worktrees.get(name, "")
        branches.append(
            Branch(
                name=name,
                current=head == "*",
                contained=subprocess.run(
                    ["git", "merge-base", "--is-ancestor", name, default_branch]
                ).returncode
                == 0,
                upstream_gone="gone" in track,
                worktree=worktree,
                worktree_dirty=bool(worktree)
                and bool(git("status", "--porcelain", cwd=worktree).strip()),
            )
        )
    return branches


def merged_heads(limit):
    """The head branch names of merged pull requests, or none when `gh` cannot
    answer: pruning then rests on containment in the default branch alone."""
    try:
        output = subprocess.run(
            [
                "gh",
                "pr",
                "list",
                "--state",
                "merged",
                "--limit",
                str(limit),
                "--json",
                "headRefName",
                "-q",
                ".[].headRefName",
            ],
            capture_output=True,
            text=True,
            check=True,
        ).stdout
    except (OSError, subprocess.CalledProcessError) as error:
        print(f"gh cannot list merged pull requests ({error}); containment only")
        return []
    return output.split()


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument(
        "--apply",
        action="store_true",
        help="remove what the plan lists (default: print the plan)",
    )
    parser.add_argument(
        "--default-branch",
        default="origin/main",
        help="the ref whose content counts as preserved",
    )
    parser.add_argument("--merged-limit", type=int, default=500)
    arguments = parser.parse_args(argv)

    plan = prune_plan(
        local_branches(arguments.default_branch), merged_heads(arguments.merged_limit)
    )
    for line in render(plan):
        print(line)
    removals = [decision for decision in plan if decision.remove]
    print(
        f"{len(removals)} of {len(plan)} local branches are landed; "
        f"{'apply with --apply' if removals else 'nothing to do'}"
    )
    if arguments.apply:
        for decision in removals:
            if decision.worktree:
                git("worktree", "remove", decision.worktree)
            git("branch", "-D", decision.name)
            print(f"removed {decision.name}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
