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

`materialize` CREATES the work branch explicitly at the
caller-validated start point: `git worktree add --no-checkout -b
<branch> -- <path> <start-point>` rooted at the source repository,
followed by `checkout` rooted at the new worktree, then `observe_head` to
confirm the result. Explicit `-b` with an explicit start point resolves
the DWIM hazard structurally — there is no remote-tracking ambiguity for
git to resolve, and `-b` on an existing branch fails (refuse, don't
clobber). The caller supplies the start point from validated state:
`provision` (below) passes the Change Stream's recorded base commit after
validating the source repository's actual HEAD against it. The scripted
test pins the full invocation sequence including both rooting points and
the explicit `-b`; the live test materializes a real branch and asserts
its HEAD equals the start point.

## Dispatch-time provisioning (#211)

`provision` is the composition a started dispatch runs before any work:
(1) the Change Stream's reserved location is claimed and verified — the
identity is re-derived from the canonical Project/Root/Stream identities
and the policy seed, and a mismatch with the stream's recorded worktree
id or branch refuses BEFORE any git call (pinned by a scripted-executor
test with an empty call log); (2) the source repository's actual HEAD is
observed and validated — an unborn HEAD is `BaseUnresolved` and a HEAD
that moved off the stream's recorded base is `BaseMoved`, refused before
materialization (pinned: the reserved location is claimed but holds no
git content after the refusal); (3) the worktree is materialized at the
stream's derived branch, created at the validated base. Errors are
stage-honest (`Reservation` / `BaseUnresolved` / `BaseMoved` / `Git`) so
retry policy can distinguish a compromised location from a stale premise
from a git refusal. Provisioning is not idempotent across a crash between
reserve and materialize: the reservation layer's O_EXCL marker refuses
re-claim until an explicit `release()` — operationally, a failed run's
task cannot re-activate without that release, and the refusal reports the
Reservation stage. The Host wiring (`symbiote-host::runner::
provision_worktree`) resolves the stream, Root placement (per-Host
host_paths), and policy seed from trusted store state; the policy seed is
a hex digest of the stream id, never the raw id (domain ids allow 128
bytes, the seed bound is 64).

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
