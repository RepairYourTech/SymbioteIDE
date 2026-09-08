# Durable task leases and the scheduling projection

Canonical owner: [#95](https://github.com/RepairYourTech/SymbioteIDE/issues/95), depending on #94 (dependency edges), #181 (protocol) and #210 (journal provenance). `symbiote-domain::lease` and `symbiote-store`'s `task_leases` implement the lease/fencing and explainable-scheduling slice. This is not full #95 acceptance: event-driven wake/notify, collision consumption, dead-letter delivery, budget/concurrency accounting, and fabric-distributed dispatch ownership remain pending.

## Durable leases with fencing tokens

A `TaskLease` binds one `Running` task's dispatch to a host for a bounded window (1 second to 1 hour). The fencing token is monotonic per task: every post-expiry acquisition increments it, and a superseded generation is fenced — an old owner cannot release or otherwise re-gate a task a newer generation owns. State transitions are `held → expired | released | fenced`; only `held` leases with an unexpired window hold.

- **Acquire/renew** (`acquire_lease`): validates the task is `Running`, that the dispatch and host match the stored task's current dispatch (a foreign dispatch is `invalid_request`, not a re-staff), extends the window for the same holder without burning a token, and takes the next generation after expiry. Exact idempotency: identical command IDs replay the original receipt.
- **Release** (`release_lease`): only the holding dispatch with the current token may release an unexpired lease; wrong token → `conflict` (fenced), wrong/expired state → `conflict` (not held) — with exact-idempotent replay of the original release attempt first, so a retry after a lost response returns the original receipt. Release and expiry events attribute the authenticated caller; system sweeps are separate.
- **Expiry** (`expire_stale_leases`): transitions every held-but-expired lease to `expired` under a deterministic command ID carrying the fencing generation (`symbiote-sweep-<task>-<token>`) from a reserved namespace that caller-initiated lease writes are refused, so sweeps are idempotent, replayable, and poison-proof. Expiry does **not** modify task state: the Task's own lifecycle (Interrupt/Recover) remains the Host's decision, keeping worker completion separate from verified completion.
- **Journal replay** (`task_leased` events, schema v8): replay verifies each lease's shape (including the declared 1s–1h window), project lineage, journal-request byte equality, and reconstructs per-task lease state as a state machine — first acquisition is token 1, same-generation transitions keep the token, and only a post-terminal acquisition takes the strictly next generation. The full Running/dispatch binding invariant applies to `held` events only: release and expiry legitimately journal after lifecycle recovery moved the task out of `Running`, so requiring it there would brick replay on the documented interrupt-then-sweep recovery path. The table must replay exactly in both directions (no journal-less owners, no unrecorded rows).

Leases authorize nothing by themselves — dispatch contracts, permission checks and consent stay upstream. The `Running`-task precondition and dispatch binding mean a lease is a *claim on an existing authorized dispatch*, not a substitute for one.

## Explainable scheduling projection

`scheduling_projection(now)` walks every `Ready` task and classifies it with a recorded reason:

- **Blocked: `stream_unsafe`** — the owning Change Stream is not `Active` (collided, integrated, cancelled). Stream state decides before the task graph, and stream-level blockers do not collapse into task state.
- **Blocked: `stream_leased`** — another task holds a live lease on the same stream: same-stream work is serialized by policy.
- **Blocked: `dependency_unresolved`** — an outgoing `requires`/`consumes_contract_from` target is not `Completed`. (Outgoing `blocks` deliberately does not gate starting: it gates the *target's* completion, matching the #94 direction semantics.)
- **Schedulable** — with `no_blocking_dependencies` or `dependencies_satisfied`.

Protocol v1.8 adds `acquire_task_lease` and `release_task_lease` (Host-local authority — the bootstrap local-owner policy, since only the Host process holds dispatch identity today; restricted worker identities arrive with #269), `expire_stale_leases` (owner sweep returning expired tokens plus the projection), `get_scheduling_projection` (owner), the `scheduler_sweep` response, the `task_leased` journal payload, and `TaskLeaseManagement`/`SchedulingProjection` capabilities.

## Evidence and remaining acceptance

Covered by unit tests (lease arithmetic bounds, held-only semantics), store tests (acquire/renew/expiry idempotence/token generations/fenced release/foreign dispatch rejection/not-Running rejection/restart replay) and a daemon test (projection explains a Ready task, sweep is empty and stable across restart).

Pending #95 acceptance, tracked in the issue: event-driven wake/notify (idle Lead consuming no inference), duplicate completion/collision event deduplication, collision-evidence consumption to pause/serialize streams, dead-letter visibility, budget/concurrency/Host-resource accounting, and fabric-distributed dispatch ownership. Host disconnect/stale-lease transition is partially covered: expiry + fencing + the Task lifecycle give safe recovery primitives, but automated reassignment by policy is future work.

Applicability: security/privacy — lease operations are owner-authority only and never appear in worker-visible surfaces; cross-project identifier disclosure rules from #94 apply unchanged. Accessibility is not applicable to a headless scheduler library. Performance is bounded by table caps, not benchmarked. Linux-first, matching the Host.
