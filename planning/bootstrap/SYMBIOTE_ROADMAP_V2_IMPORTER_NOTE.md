# Symbiote Roadmap v2 Importer

This repository-native bootstrap is intentionally idempotent. The accompanying workflow creates or updates the dependency-ordered Symbiote v2 GitHub issue program by stable `symbiote-plan-key` markers. It never deletes or closes unrelated issues.

The generated roadmap treats Linux as the reference Host platform while preserving Windows, macOS, iOS, and Android as required targets. It incorporates the Host/Fabric architecture, per-harness native configuration contracts, mobile remote control, Deep Guidance, ambiguity remediation, System Graph, Context Broker, Delivery Architecture, Capability Closure, worktree isolation, and GitHub issue-to-PR delivery lifecycle.

## Accepted post-bootstrap roadmap amendments

The bootstrap source predates some architectural refinements that are now canonical in the live issue graph. **Do not treat a re-run of the older bootstrap payload as authority to remove or weaken later accepted amendments.** Any future importer regeneration must fold these amendments into its source data before being considered canonical.

### v2.2 — Change Streams and Collision Intelligence

See [`planning/ROADMAP_V2_2_CHANGE_STREAM_COLLISION_INTELLIGENCE.md`](../ROADMAP_V2_2_CHANGE_STREAM_COLLISION_INTELLIGENCE.md).

The v2.2 amendment establishes:

- `ChangeStream` as the durable unit of mutable Git isolation and integration, distinct from Task, Chat, Session, Branch, and Worktree;
- separate Change Streams/worktrees for independent mutating chats inside the same Project;
- explicit stacked/dependent Change Streams;
- continuous graph/Git/runtime-aware Collision Intelligence;
- collision diagnostics flowing through the same real-time Diagnostics Bus used for lint/type/test/runtime/CI failures;
- disposable speculative merge/rebase/restack integration workspaces rather than conflict discovery on canonical checkouts;
- safe rebase/restack synchronization points and stale-evidence invalidation;
- a provider-neutral Integration Queue with current-target revalidation;
- multi-owner collision routing and intelligent remediation guidance;
- identical collision/integration guarantees across multiple Symbiote Hosts.

Live roadmap issue `GIT-05` (#442) and the v2.2-updated `GIT-01`, `DIAG-01`, `FND-03`, and `WORK-02` contracts implement this amendment. The master roadmap, scheduler, distributed execution, continuous verification, Git/GitHub delivery, and client health surfaces must preserve these semantics.

> **Importer maintenance rule:** before the bootstrap workflow is used again as an authoritative synchronizer, its encoded issue data must be regenerated to include every accepted amendment listed here. Until then, the live v2.2 issue bodies plus amendment documents outrank older bootstrap wording where they conflict.
