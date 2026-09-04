# Symbiote Roadmap v2.2 — Change Streams and Collision Intelligence

**Status:** Accepted roadmap amendment  
**Date:** 2026-09-04  
**Applies to:** Symbiote v2 program, Git/worktree architecture, Diagnostics Bus, System Graph applications, scheduler, multi-Host execution, desktop/mobile health surfaces, GitHub delivery, and production-readiness gates.

## Why this amendment exists

The earlier roadmap correctly required isolated worktrees for parallel tasks, but that model was too narrow. A user can open a new mutating chat in the same Project while an existing long-running objective is still working. These independent bodies of work must not share a dirty checkout merely because they belong to the same Project, and Git textual conflicts are not sufficient to detect semantic incompatibility between them.

This amendment makes **Change Stream** the durable unit of mutable Git isolation and integration, and makes **Collision Intelligence** a continuous producer of the same real-time Diagnostics Bus used for lint, type, compiler, test, browser/runtime, CI, security, and graph-policy diagnostics.

## Canonical Change Stream model

A `ChangeStream` is distinct from Project, Chat, Session, Objective, Capability, Task, Branch, and Worktree.

A Change Stream:

- owns the mutable branch/worktree integration boundary;
- records target branch, base commit, current head, last synchronized target SHA, and last validated target SHA;
- may contain one task or multiple dependency-coherent tasks intentionally delivered together;
- can originate from a separate mutating user chat, an orchestrated objective, a human edit workflow, or other approved mutation source;
- survives harness/model/session/client replacement;
- records predicted and observed mutation subgraphs;
- records upstream/downstream Change Stream dependencies;
- can be independent or explicitly stacked on another unmerged Change Stream;
- owns integration/revalidation/collision state, while its member Tasks retain canonical work semantics.

Read-only investigation does not require a Change Stream unless project policy says otherwise.

## Workspace isolation invariant

The protected/canonical checkout is a reference checkout, not the normal mutation workspace.

When a new independent mutation begins, Symbiote must establish an isolated Change Stream before edits are allowed. A new chat in the same Project therefore shares authorized Project knowledge, System Graph, artifacts, decisions, and task state, but does **not** inherit another stream's dirty working tree.

## Collision Intelligence

Collision Intelligence continuously evaluates active Change Streams using both planned and observed evidence.

Inputs include:

- Git refs, branches, indexes, worktrees, renames, deletes, and target advancement;
- live file/symbol ownership and edits;
- predicted mutation sets from task planning;
- actual diff-to-System-Graph mapping;
- symbol/type/reference/call/data-flow relationships;
- API/schema/event/configuration/feature-flag/infrastructure contracts;
- database migrations and ordering;
- generated-code inputs and outputs;
- package manifests and lockfiles;
- tests/shared fixtures;
- human edits and external Git clients;
- CI/compiler/lint/runtime evidence;
- speculative merge/rebase/restack results.

A clean textual merge is never treated as proof of semantic compatibility.

## Diagnostics Bus integration

Collision Intelligence is **not** a separate warning system. It publishes typed canonical diagnostics into the common Diagnostics Bus.

Required collision diagnostic classes include at least:

- `textual_conflict`
- `semantic_overlap`
- `contract_conflict`
- `migration_conflict`
- `generated_artifact_conflict`
- `stale_base`
- `stack_dependency_changed`
- `ownership_violation`
- `human_agent_collision`
- `integration_regression`
- `merge_order_hazard`

Each diagnostic carries severity, confidence, evidence sources, affected streams/tasks/roles/files/symbols/contracts, base/head/target SHAs, timestamps, owner set, remediation guidance, and resolution state.

The Diagnostics Bus correlates duplicate symptoms. One underlying contract collision may be supported simultaneously by System Graph overlap, type errors, failed tests, browser/runtime failures, and speculative integration failure while remaining one canonical diagnostic with multiple evidence sources.

## Speculative Integration Lab

Symbiote must discover integration problems in disposable integration workspaces/refs, never by experimenting on the protected checkout or an active agent worktree.

Progressive checks should include:

1. cheap deterministic Git merge-tree/equivalent analysis;
2. graph overlap and contract/schema analysis;
3. disposable merge/rebase/restack candidate construction;
4. compile/type/lint as applicable;
5. affected tests;
6. broader integration checks when risk justifies them.

Every result is tied to exact base/head/target SHAs and tool versions. Disposable workspace cleanup is crash-safe and journaled.

## Intelligent remediation

Collision diagnostics should produce ranked, evidence-backed remediation choices such as:

- continue independently;
- coordinate ownership/boundaries;
- pause one stream at a safe checkpoint;
- serialize streams;
- create a shared prerequisite/contract stream;
- convert work to an explicit stacked stream;
- rebase/restack after an upstream merge;
- split a stream;
- create an integration/remediation Task;
- escalate to Lead/Architect/client decision;
- require additional affected tests/review;
- defer only with explicit authorized risk.

Deterministic safe actions may be automated only under explicit project policy. Product/architecture conflicts requiring reasoning are escalated rather than guessed.

## Rebase and restack policy

Symbiote does not continuously rewrite active branches just because target HEAD changes.

Preferred synchronization points:

- before mutation begins;
- after an upstream Change Stream lands;
- before Ready for Review;
- before final integration;
- after a materially overlapping competing stream lands.

Before history rewrite, Symbiote verifies:

- worktree is clean or safely checkpointed;
- no untracked/unrecoverable edits exist;
- dependent stacked streams are known;
- external branch modifications are reconciled;
- PR/review implications are known;
- remote rewrite policy permits the operation.

Published-branch rewrites use lease-protected update semantics equivalent to `--force-with-lease`; blind force pushes are prohibited.

Rebase/restack invalidates stale CI, review, Impact Closure, affected-test, graph-impact, and integration evidence tied to prior commits.

## Integration Queue

Parallel development is encouraged; landing is controlled.

Streams targeting the same branch/release line enter a provider-neutral Integration Queue. GitHub Merge Queue may execute part of this policy when connected, but Symbiote retains canonical integration semantics for local/private Git as well.

When target HEAD advances, only evidence-proven unaffected streams may retain readiness. Relevant overlapping streams transition to `revalidation_required` and must validate the exact current-target candidate before merge.

## Real-time UX

Collisions appear alongside lint/type/test/runtime problems in the same project-health surfaces.

Examples:

- Agent Floor stream status and blocker badges;
- Problems/Diagnostics view;
- Project Control health summary;
- Lead activity/events;
- mobile alerts and approvals;
- task and Change Stream detail views.

A multi-owner collision routes to every responsible owner while retaining one canonical diagnostic identity.

## Multi-Host invariant

Change Stream identity and collision safety are Host-independent. Two streams on different Symbiote Hosts are not considered isolated merely because their worktrees are physically separated. The same graph/Git/integration model applies across the Fabric.

## Required golden scenarios

The implementation program must verify at minimum:

1. Chat A runs a long mutation; Chat B starts unrelated mutation in the same Project; both receive separate Change Streams/worktrees and merge safely.
2. Two streams edit different files but the same API/schema/event and receive a semantic collision before merge.
3. Independent same-directory work remains parallel without false blocking.
4. A textual conflict is discovered in a disposable integration workspace before target mutation.
5. A stacked Stream B depends on unmerged Stream A; after A lands, B is restacked and stale evidence is invalidated/recomputed.
6. Target HEAD advances after a stream was green; relevant streams become revalidation-required before merge.
7. Dirty/uncheckpointed work prevents unsafe automatic rebase.
8. Human edits and external Git changes produce the same canonical collision model as agent edits.
9. Multi-Host streams receive identical collision/integration guarantees.
10. Crash/restart preserves streams, diagnostics, integration evidence, and cleanup state while proving canonical checkout integrity.

## Roadmap issue propagation

This amendment is represented in the live issue program by:

- `GIT-05` / #442 — Continuous Collision Intelligence, speculative integration, Integration Queue, and remediation.
- `GIT-01` / #129 — Change Stream-owned branch/worktree isolation.
- `GIT-02` / #130 — graph-aware overlap prediction and mutation ownership.
- `DIAG-01` / #138 — common real-time Diagnostics Bus and multi-owner routing.
- `FND-03` / #36 — canonical Change Stream domain model.
- `WORK-02` / #94 — Task ↔ Change Stream separation and collision/revalidation gating.

The master roadmap, scheduler, distributed execution, Git delivery, verification, and UI contracts must continue to reference this accepted architecture.
