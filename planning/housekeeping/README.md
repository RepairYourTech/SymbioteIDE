# Merged-checkout housekeeping

This Python standard-library maintenance tool prunes local branches and worktrees whose work has already landed, from evidence rather than from a guess. Its only path outside the local repository is reading merged pull-request head names through `gh`; when `gh` cannot answer it says so and falls back to containment in the default branch. Python is used for existing planning maintenance; this does not select the product runtime.

```sh
python3 planning/housekeeping/prune_merged_checkouts.py            # print the plan
python3 planning/housekeeping/prune_merged_checkouts.py --apply    # remove what it lists
python3 planning/housekeeping/test_prune_merged_checkouts.py
```

Merges in this repository are squashes, so a landed branch tip is never an ancestor of `origin/main`, GitHub deletes the remote branch after the merge, and the branch name is the only remaining link between a local checkout and the merge that landed it. A branch is removed when its tip is contained in the default branch, or when a merged pull request names it as its head. It is kept when it is the current branch, when a worktree holds uncommitted changes on it, or when neither piece of evidence holds. `--apply` removes the worktree first — `git worktree remove` refuses a dirty worktree on its own — and then the branch.

The cleanup that motivated this tool removed 57 clean worktrees and 70 local branches whose content was in `main`; the worktrees alone held about 140 GiB of build caches, and branches pointing at superseded heads produced diffs that read like uncommitted work. This tool is the repeatable form of that cleanup: it never pushes, never rewrites history, and mutates nothing outside the local repository.

## What it does not decide

It does not verify that a merged pull request's diff is the whole of a branch's content, so it should be run when the worktree is clean; a branch whose tip is an ancestor of the default branch is removed on that containment alone. It also does not remove remote branches, which GitHub deletes on merge.
