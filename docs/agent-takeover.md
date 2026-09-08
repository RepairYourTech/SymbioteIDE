# Symbiote end-to-end build takeover prompt

Copy this document into the next agent's task, or instruct it to read and execute
this document from the repository. This is a continuation of the user's build
request, not a request for another roadmap or a release-readiness assertion.

## Mission and authority

Take over RepairYourTech/SymbioteIDE as principal implementation engineer. Keep
building Symbiote end to end. Write and integrate working code through verified
milestones. The user wants the application, not only contracts and documentation.
Preserve the complete objective across turns; do not redefine completion around
the implemented foundation. Symbiote is **not ready to ship**.

The first usable release must let a user open a repository, describe a coding task
to the Lead, execute it, inspect its diff and independent verification evidence,
restart the desktop and resume without losing canonical state. BOTH native Rust
execution using the OpenAI Responses API and Codex as an external App Server
harness must work, including mixed staffing. Switch between two Projects without
leaking work, configuration or credentials. Worker completion requests never
bypass Host verification and independent review.

User defaults: Linux first; Rust core; strict TypeScript/React workbench; Tauri 2
preferred but subject to architecture proof. No Electron or second Go core. No
new spending, credential changes or paid fallback authorization is implied.
Implement the integrations and tests without spending; surface the exact live
verification requiring authorization if it remains necessary. Do not silently
use native API billing for external execution or Codex credentials for native.

## Start from current authoritative state

Main checkout: `/home/birdman/Projects/Symbiote`, also exposed as
`/mnt/data/projects/Symbiote`. GitHub repo: `RepairYourTech/SymbioteIDE`.
All product implementation through PR #486 is merged on main at
`b257c8fba02c1087163772227ac482681829eac8`. This handoff is a subsequent documentation
change. Recheck remote/main and current issues rather than assuming this snapshot
is still current. The last build turn was progress, not blocked: #486 merged with
all required checks and published issue evidence.

First inspect git status, remote/main, worktrees and any repository instructions.
Preserve existing work. Read `docs/engineering-handoff.md`,
`docs/architecture/product-constitution.md`, relevant `docs/contracts` and
`docs/security`, and the live canonical issue bodies/comments. The historical
handoff has dated sections; its oldest “no application” text describes the
initial checkout, not the current source.

Roadmap entry points: #154, `planning/integrity/generated/registry.json` and
`planning/integrity/generated/execution-index.md`. Live issue dependencies and
acceptance govern. Do not run legacy roadmap importers/synchronizers. Their
refusal safeguards are intentional; product development does not depend on
running a roadmap writer. Do not close broad foundation issues based on a slice.

## What is actually implemented

The repository now contains a Rust workspace and working headless Host/CLI, not
just a plan. Read source and tests for exact APIs; this list is orientation.

- Domain contracts: canonical Project, Root, Role, Binding, Task, Dispatch,
  Change Stream, permissions, evidence and verified lifecycle. Runtime adapter
  and inference provider are separate. Existing Dispatch contracts retain
  historical Role/profile/binding/Host snapshots.
- Configuration: scoped portable Project manifests, environment/resource plans,
  projection intent and trust/consent boundaries. Projection is not activation.
- Durable Host: `symbioted` plus `symbiote` CLI, local same-UID authenticated Unix
  socket, typed protocol **v1.5**, 64 KiB requests and 1 MiB responses. The local
  owner policy is not a restricted worker identity and must not be exposed to
  untrusted Preview. No worker activation endpoint exists yet.
- Storage: SQLite **schema v5**, transactional event journal, exact idempotency,
  revision/CAS checks, semantic replay audits, migration/recovery and SIGKILL
  tests. `rusqlite = 0.40.2` bundles SQLite 3.53.2; keep the >=3.51.3 WAL fix gate.
  Do not downgrade it to fix an unrelated build.
- Runtime SDK: typed runtime/provider contracts, events, capability/evidence
  qualification, bounded requests. `qualify_profile` retains strict preventive
  controls; `qualify_profile_with_minimums` honors explicit lower requirements
  with the same exact fresh evidence. Observed and emulated are incomparable;
  neither satisfies a preventive requirement.
- Runtime transport: bounded subprocess JSONL, duplicate-key rejection,
  deadlines/cancellation and static errors. Descendant cleanup remains Unknown;
  do not upgrade that claim based only on a successful leader exit.
- Linux sandbox: trusted helper closes inherited descriptors, bubblewrap
  isolation, read-only/worktree-write profiles, no network, private HOME,
  permission/consent checks and path alias rejection. Trusted `/usr` and stable
  reserved worktree are explicit premises. Read sandbox docs before composing
  agent execution; shell setup success is not target success.
- Canonical work hierarchy: Request, Objective, Capability, Plan, Milestone,
  Checkpoint, Follow-up; durable history, graph checks and explicit Task origins.
  Legacy unclassified Tasks cannot start. No automatic capability closure.
- Runtime discovery (#483): actual installed **Codex 0.118.0** App Server offline
  handshake/account-read/model-list inside the sandbox. Fresh disposable HOME,
  no credential access, no model turn. Current installed `/usr/bin/codex` version
  was verified, not inferred from older NVM notes. Versionless JSONL RPC, observed
  `symbiote/0.118.0` user-agent product. Current docs may describe newer APIs;
  inspect pinned generated schemas. Model listing is not model usability.
- Project Team (#484): durable Team policies, required Lead/general executor,
  independent review and permission ceilings. Stable Role references. Separate
  ManageTeam permission; schema v4 history carried into v5. Team structure does
  not qualify runtime staffing.
- Host inventory (#485): owner-only passive CPU/memory pulse, private stable
  random `host-id`, bounded procfs reads, <=1Hz cache with unchanged five-second
  expiry, `--no-telemetry`, explicit failures/unknowns. Physical memory/CPU are
  not effective capacity. Cgroups, reservations and broader capability probes
  remain pending.
- Workforce binding (#486): bounded `symbiote-workforce::BindingConfiguration`
  around canonical binding identity; explicit independent primary/fallback
  scopes, Team revision, policies/limits; durable replace/get/readiness. Separate
  ManageBindings permission. Replay validates binding against Team at that
  journal position; a later Team update makes desired binding stale, not corrupt.
  Binding/Role/Project identities cannot be replaced silently. Readiness uses
  current Team/Host/optional SDK evidence; actual Host reports missing runtime
  observations. Even `ReadyForPreflight` leaves six activation gates pending.

There is an earlier Linux shell/workload proof with TypeScript/React, editor,
streams/PTYs, Preview and floating controls. It is a proof, not the finished
workbench. Tauri acceptance still needs real Wayland/X11, focus/composition,
accessibility, Preview isolation and whole-process measurements; Windows/macOS
evidence remains explicitly pending. Inspect `docs/proofs` and locate the spike
source before deciding what to reuse. Control-plane storage choice is separate
from System Graph/SurrealDB evaluation.

## Immediate continuation and critical path

Do not spend the next task restating this document. Inspect current prerequisites,
choose the next dependency-ready implementation, and make reviewable code. Move
toward real dispatch and both worker loops rather than indefinitely expanding
contracts. The latest foundation makes #204 Role resolution dependency-ready for
its first real integration with #189 and #203. Other execution prerequisites can
advance independently where their live dependencies permit.

The live #205 dispatch loop dependencies at handoff are:
`#185 #187 #189 #203 #204 #211 #214 #269 #94 #95 #460`.
Recheck them. Earlier inspected paths include:

- #211 Change Stream Git worktrees depends on #189/#190.
- #94 Task DAG depends on #189/#43; #95 scheduler/leases depends on #94/#181/#210.
- #269 least-privilege assignment depends on #217/#203/#218/#184/#214.
- #460 goal/delegation contracts depend on #184/#203/#241/#464.
- #465 owns the real native Responses API model/tool loop; #464 owns shared
  native/external provider contracts. Preserve AgentRuntimeAdapter versus
  InferenceProviderAdapter separation.

Complete the remaining foundational portions of those canonical owners needed
for mutation: authenticated assignment, context/environment resolution and
credential isolation, enforceable permissions and aggregate limits, reserved
worktrees, durable leases/fencing, cancellation/recovery and verification.
Runtime discovery, provider/model entitlement health and effective Host capacity
are not yet a complete qualification pipeline. Existing read-only readiness
reports intentionally leave these missing. Replace pending gates with actual
integrations and evidence, not always-true flags or mocks labeled complete.

Then implement both real loops against the same canonical dispatch/task/control
plane, followed by the usable desktop workflow. Native operation must work with
Codex absent. External operation must not silently fall back to native billing.
Extend through System Graph/Context Broker, Deep Guidance, Living Documentation,
Capability Closure, voice, delegation, delivery and platforms following the
canonical queue; bring prerequisite portions forward when required. Do not drop
these from the full end-to-end objective.

## Working rules and verification

Use isolated issue-owned branches/worktrees from refreshed main, focused PRs and
explicit independent reviewer ownership. Two prior implementation agents used
cross-review: each reviewed the other author's module and root integration,
never approved their own code. A separate `domain_review` agent repeatedly hit
an automatic security filter; it was not counted as review. CodeRabbit has
recently been rate limited; its green status is not completed review either.
Create fresh reviewers or reuse successful agents with clear scopes.

Run relevant tests, `cargo fmt --all --check`,
`cargo clippy --workspace --all-targets --locked -- -D warnings` and required CI.
Local stable is newer; locally registered Rust 1.85 was broken, so fresh CI is
the MSRV proof. CI uses Ubuntu22.04 and pinned bubblewrap0.12 because Ubuntu24's
default setup failed namespace loopback initialization. Do not relax AppArmor,
namespace flags, permissions or tests to make it pass. The pinned offline Codex
binary test installs only on disposable CI and verifies SHA-512.

Test denied permissions, interrupted/duplicate tool delivery, concurrent edits,
stale revisions/evidence, cancelled/recovered work, unknown side effects and
Preview attempts to invoke privileged operations. Keep worker completion
separate from verified completion. Evidence must cover actual integrations;
mocks only prove contracts. Preserve canonical state on client disconnect and
restart. Do not declare the desktop shell or application complete from headless
unit tests.

Before merge, recheck exact PR head, target main, checks and review findings.
Integrate, fast-forward the clean main checkout, and publish exact issue evidence.
Do not delete preserved branches/worktrees without separate cleanup authority.
No running demo daemon was left behind by the handing-off agent.

## Latest evidence and saved state

PR #486: merged `b257c8fba02c1087163772227ac482681829eac8`, reviewed head
`c05b537007bdec0d439cfbb53833357ae308a685`.
Rust checks: https://github.com/RepairYourTech/SymbioteIDE/actions/runs/34201362020
Host/shell: https://github.com/RepairYourTech/SymbioteIDE/actions/runs/34201362036
Roadmap: https://github.com/RepairYourTech/SymbioteIDE/actions/runs/34201362048
Review: https://github.com/RepairYourTech/SymbioteIDE/pull/486#issuecomment-5581317417
Issue evidence: https://github.com/RepairYourTech/SymbioteIDE/issues/203#issuecomment-5581361419

Focused results at that head: 34 Store, 18 Host, 16 protocol, 32 SDK and 7 workforce
tests passed, plus workspace CI and strict Clippy. CLI demo created binding
journal sequence3, reported not_ready with pending gates, restarted and recovered
sequence3 replayed=true. Its private fixture directory is
`/tmp/symbiote-binding-demo.RTDxeODo`; daemon stopped. Prefer checked-in
`fixtures/workforce-bindings`, `fixtures/project-team`, `fixtures/host-inventory`
and `fixtures/work-hierarchy` over temporary artifacts.

All implementation worktrees through `issue-203-workforce-bindings` are retained
under `/mnt/data/projects/Symbiote-worktrees`. The last implementation worktree
and main were clean after merge. No unstaged implementation is required from
those worktrees. Earlier merged PRs #472–#485 and their issue comments document
the preceding foundations. Revalidate live state before continuing.

## Definition of done

Finish the user's full requested application, not this handoff or a foundation
PR. Verify the first-release end-to-end demonstration with BOTH runtimes and
mixed staffing, real source changes, independent review, Host verification,
restart/resume, Project isolation and secure Preview. Track later canonical
scope honestly. If live credentials/spending authorization or platform access
is needed, identify the exact remaining verification and continue other useful
work. Never claim unverified integrations have passed.
