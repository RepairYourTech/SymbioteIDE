# Dispatch preparation: composing scheduling, routing, leases, worktrees, providers

Canonical owner: [#205](https://github.com/RepairYourTech/SymbioteIDE/issues/205) and [#189](https://github.com/RepairYourTech/SymbioteIDE/issues/189) (Dispatch aggregate), depending on the merged slices: Role routing (#204), dependency edges (#94), task leases (#95), reserved worktree locations (#211), and the provider registry (#464). This slice adds the durable **composition** record — the point where all five primitives meet. The live `Dispatch` runtime contract and `Start` transition existed since #184; what was missing is the journaled, explainable preparation that decides *what* may be started and *why it was refused*. The worker loops (#465 native, #464/#465 external) consume this record next.

## The preparation record

`prepare_dispatch(task_id)` runs entirely against authoritative storage — no client-supplied composition — and journals one `dispatch_prepared` event per task (latest wins, table replays exactly, both directions audited):

- **Scheduling** — the task must be `Ready`; the #95 projection's dependency/stream detail is recorded there and the same conditions gate a `Ready` verdict here.
- **Routing** — the latest route decision (#204 slice) for the task's origin work identity supplies the resolved Role; unresolved routes are recorded as such.
- **Lease** — a `held`, unexpired lease (#95 slice) contributes its dispatch binding and fencing token; the token is what a later `Start` transition must carry forward.
- **Worktree** — the stream's reserved worktree identity and branch from the Change Stream record (#211 slice owns filesystem reservation; this records the binding).
- **Provider** — the routed Role's workforce binding (#203/#486) carries the runtime profile; its provider connection must exist in the #464 registry for the provider step to resolve.

The outcome is `ready` only when scheduling, routing, lease, and provider all resolve. **A refused composition is recorded like a ready one**: the response body carries `outcome: "refused"` with the refusing step — refusal is a successful read of recorded state, not an error. Failures *outside* the enumerated steps (unknown task, unreadable origin, malformed stored binding) surface as errors and leave no preparation record; the journal's surrounding events remain the evidence for those.

## Authority

`prepare_dispatch` and `get_dispatch_preparation` are local-owner operations, consistent with the scheduler endpoints; restricted worker identities arrive with #269. The preparation record carries no secret values: provider identity is a connection reference, and credential materialization stays with #217.

## Evidence and remaining acceptance

Covered by a store test (composition steps, refusal recording, replay, readback, unknown-task refusal) and a daemon test (projection → prepare → refusal recorded → replay identical → restart replay → journal lineage).

Pending #205 acceptance, tracked in the issue: consuming the preparation in a `Start` transition with dispatch-contract compilation against Host enforcement claims; worker-loop activation (#465 native, #464/#465 external); mixed-runtime staffing round-trips; completion/verification wiring; and the #269 least-privilege effective-access manifest at dispatch time.

Applicability: security/privacy — owner authority only, no secrets, refusals recorded as evidence. Accessibility not applicable to a headless composition layer. Performance bounded by per-task record count. Linux-first, matching the Host.
