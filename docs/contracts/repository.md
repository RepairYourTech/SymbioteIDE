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
null, an empty environment (repo-configured `GIT_DIR`/`GIT_INDEX_FILE`
cannot redirect a ref-mutating invocation), stderr discarded, output
captured through a 256 KiB bound, and a watchdog thread that SIGKILLs the
child's **process group** at the deadline — hooks and filters are children
of git in the same group, so no orphaned descendant can hold the stdout
pipe forever, and the flag is re-checked after the loop to close the
PID-reuse window. Nonzero exit is `GitRefused`; over-bound output is
`OutputTooLarge`; spawn/IO failures are `Execution`; request-side problems
(empty/overlong branch) are `InvalidRequest`. Parsed facts that violate
structural bounds (commit length/charset, branch name bounds and control
characters, status path bounds, truncated rename records) are
`MalformedOutput` — malformed output is rejected, never guessed into a
plausible shape.

## Observed facts

- `observe_head` classifies HEAD as `Branch` (with name), `Detached`, or
  `Unborn` (symbolic-ref succeeds while rev-parse fails quietly). The
  commit is empty only for unborn. Detection order: symbolic-ref first,
  then rev-parse — matching git's actual failure modes.
- `observe_status` parses `status --porcelain -z` — NUL-terminated
  records defeat the C-quoting porcelain v1 applies to special-character
  paths (a quoted or arrow-containing filename can never be mistaken for
  structure) — into uncommitted and untracked path lists. Rename/copy
  records carry their origin as the next NUL field; the destination is
  recorded and a missing origin is a truncated frame, not a skip. Paths
  only, never content: content belongs to the diff commands, which remain
  pending.

All observations are advertisements — observed provenance, never
authorization or proof of remote identity.

## Materialization

`materialize` first verifies the branch resolves as a LOCAL ref
(`rev-parse --verify refs/heads/<branch>`), refusing remote-tracking-only
names — without this, `worktree add`'s DWIM would silently create a new
local branch from a matching remote-tracking ref, an undocumented ref
mutation of the user's repository. It then runs `git worktree add
--no-checkout` rooted at the source repository followed by `checkout`
rooted at the new worktree, then `observe_head` to confirm the result.
The scripted test pins the full invocation sequence including the
pre-verification and both rooting points; the live test materializes a
real branch's content into the reserved directory.

## Tests

Seven tests cover the real binary against temporary repositories (branch,
detached, unborn, changed/untracked separation, clean tree, real
materialization with content verification, non-empty and missing-branch
refusals) and scripted executors (exact invocation rooting and ordering,
malformed output rejection per parser, refusal surfacing, output-bound
constant contract).

## Honest non-claims

Remotes, fetch/push, credential delegation, submodules/sparse checkout,
status/diff/history normalization with exact provenance, safe multi-step
transactions with recovery, and base-commit validation before dispatch
all remain pending on #190. Network claim, scoped honestly: the crate
sends no fetch/push/pull/clone command, but `checkout` executes
repo-configured post-checkout hooks and smudge filters — a repo with
git-lfs configured could reach its remote through the filter — so callers
must compose this crate inside the Host sandbox (no network). The output
bound is exercised end to end by a real git invocation producing >256 KiB
of stdout. #211's Git materialization acceptance
additionally requires store integration with recorded stream identity and
evidence-confirmed cleanup — the reservation layer (already merged) plus
this crate provide the location and materialization halves, not full
acceptance of either issue.
