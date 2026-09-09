# Dispatch preparation: composing scheduling, routing, leases, worktrees, providers

Canonical owner: [#205](https://github.com/RepairYourTech/SymbioteIDE/issues/205) and [#189](https://github.com/RepairYourTech/SymbioteIDE/issues/189) (Dispatch aggregate), depending on the merged slices: Role routing (#204), dependency edges (#94), task leases (#95), reserved worktree locations (#211), and the provider registry (#464). This slice adds the durable **composition** record — the point where all five primitives meet. The live `Dispatch` runtime contract and `Start` transition existed since #184; what was missing is the journaled, explainable preparation that decides *what* may be started and *why it was refused*. The worker loops (#465 native, #464/#465 external) consume this record next.

## The preparation record

`prepare_dispatch(task_id)` runs entirely against authoritative storage — no client-supplied composition — and journals one `dispatch_prepared` event per task (latest wins, table replays exactly, both directions audited):

- **Scheduling** — the task must be `Ready`; the #95 projection's dependency and stream-safety conditions are enforced again at Start time (not in the preparation itself).
- **Routing** — the latest route decision (#204 slice) for the task's origin work identity supplies the resolved Role; unresolved routes are recorded as such.
- **Lease** — informational: whether a lease is already held (renewal scenario). First-time starts acquire the lease transactionally during Start; the fencing token is minted at start.
- **Worktree** — the stream's reserved worktree identity and branch from the Change Stream record (#211 slice owns filesystem reservation; this records the binding).
- **Provider** — the routed Role's workforce binding (#203/#486) carries the runtime profile; its provider connection must exist in the #464 registry for the provider step to resolve.

The outcome is `ready` only when scheduling, routing, and provider all resolve. **A refused composition is recorded like a ready one**: the response body carries `outcome: "refused"` with the refusing step — refusal is a successful read of recorded state, not an error. Failures *outside* the enumerated steps (unknown task, unreadable origin, malformed stored binding) surface as errors and leave no preparation record; the journal's surrounding events remain the evidence for those.

## Authority

`prepare_dispatch` and `get_dispatch_preparation` are local-owner operations, consistent with the scheduler endpoints; restricted worker identities arrive with #269. The preparation record carries no secret values: provider identity is a connection reference, and credential materialization stays with #217.

## Starting from a preparation

`start_prepared_task` (protocol v1.11 `start_prepared_task`) consumes a `Ready` preparation in one transaction: it re-verifies the routed Role still exists, refuses when a live lease from a previous generation is held, compiles the live `WorkforceRuntimeContract`/`Dispatch` via the domain `Dispatch::compile` (Role record, workforce binding, profile, and the Host record with its enforcement claims), applies the domain `Start` transition, and then **acquires the governing lease in the same transaction** — the fencing token is minted at start, journaling a `task_leased` event under a `<command>-lease` id. First-time starts can never pre-hold a lease (leases exist to fence Running dispatches), so the preparation's lease step is informational for renewals. The response carries the started dispatch identity, stream, and fencing token.

The Host record's enforcement claims are operator-provisioned for the Host's own inventory identity (the service refuses a host identity that is not its own); claim windows are bounded, and the evidence ids are Host-derived, not client-supplied. A refused preparation (`failed_precondition`), a superseded preparation, or a second start all fail closed — the task stays Ready and no lease exists.

## Evidence and remaining acceptance

Covered by a store test (composition steps, refusal recording, replay, readback, unknown-task refusal) and a daemon test (projection → prepare → refusal recorded → replay identical → restart replay → journal lineage).

Pending #205 acceptance, tracked in the issue: worker-loop activation (#465 native, #464/#465 external) — the started dispatch is a control-plane record only, no runtime process is launched; mixed-runtime staffing round-trips; completion/verification wiring; and the #269 least-privilege effective-access manifest at dispatch time.

Applicability: security/privacy — owner authority only, no secrets, refusals recorded as evidence. Accessibility not applicable to a headless composition layer. Performance bounded by per-task record count. Linux-first, matching the Host.
