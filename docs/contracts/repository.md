# Repository observation and worktree materialization (#190 foundation)

`symbiote-repo` implements the observation/materialization layer of the
Git/repository abstraction: reading a repository's HEAD state and worktree
status, and materializing a git worktree onto a reserved, verified-empty
location for Change Stream mutation isolation. It is the observation layer
only — nothing here authorizes access, pushes, pulls, fetches, contacts a
remote, or commits on the user's behalf. Repository identity stays with
the canonical Root records; this crate never second-guesses them.

## Invocation discipline

Every git invocation runs through the injected `GitExecutor` trait. The
production `SystemGit` runs the trusted `git` binary with no shell, stdin
null, stderr discarded, output captured through a 256 KiB bound, and a
watchdog thread that SIGKILLs the child at the deadline (a hung git cannot
hold the caller forever). Nonzero exit is `GitRefused`; over-bound output
is `OutputTooLarge`; spawn/IO failures are `Execution`. Parsed facts that
violate structural bounds (commit length/charset, branch name bounds and
control characters, status path bounds) are `MalformedOutput` — malformed
output is rejected, never guessed into a plausible shape.

## Observed facts

- `observe_head` classifies HEAD as `Branch` (with name), `Detached`, or
  `Unborn` (symbolic-ref succeeds while rev-parse fails quietly). The
  commit is empty only for unborn. Detection order: symbolic-ref first,
  then rev-parse — matching git's actual failure modes.
- `observe_status` parses `status --porcelain` v1 into uncommitted and
  untracked path lists (rename entries record the destination path). Paths
  only, never content: content belongs to the diff commands, which remain
  pending.

All observations are advertisements — observed provenance, never
authorization or proof of remote identity.

## Materialization

`materialize` runs `git worktree add --no-checkout` rooted at the source
repository followed by `checkout` rooted at the new worktree, then
`observe_head` to confirm the result. Preconditions it relies on and does
not re-derive: the worktree directory exists and is empty (the reservation
layer's verified premise) and the branch exists in the source repository.
Both violations fail through git's own refusals. The scripted test pins
the exact invocation shape and rooting; the live test materializes a real
branch's content into the reserved directory.

## Tests

Seven tests cover the real binary against temporary repositories (branch,
detached, unborn, changed/untracked separation, clean tree, real
materialization with content verification, non-empty and missing-branch
refusals) and scripted executors (exact invocation rooting and ordering,
malformed output rejection per parser, refusal surfacing, output-bound
constant contract).

## Honest non-claims

Remotes, fetch/push, credential delegation, submodules/LFS/sparse
checkout, status/diff/history normalization with exact provenance, safe
multi-step transactions with recovery, and base-commit validation before
dispatch all remain pending on #190. #211's Git materialization acceptance
additionally requires store integration with recorded stream identity and
evidence-confirmed cleanup — the reservation layer (already merged) plus
this crate provide the location and materialization halves, not full
acceptance of either issue.
