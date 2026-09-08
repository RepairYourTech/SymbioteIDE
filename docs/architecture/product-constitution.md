# Symbiote product constitution

Status: accepted product direction; implementation and conformance pending. Owner: [#170](https://github.com/RepairYourTech/SymbioteIDE/issues/170). Decision record: [ADR-0001](adr-0001-technology-direction.md).

This record reconciles the client's takeover instruction with live canonical v2.4 requirements retrieved on 2026-09-08 UTC. The instruction supersedes incompatible older technology candidates, not unrelated requirements. [#154](https://github.com/RepairYourTech/SymbioteIDE/issues/154) is the executable roadmap; #36 owns domain/security/event/lifecycle contracts; #443 records consolidation; #470 owns safe roadmap regeneration. Reference-only issues are history, not assignments. Source proves current behavior; approved specifications govern intended behavior. Contradictions require recorded remediation, never automatic rewriting of intent to match faulty code.

## Product and authority

Symbiote is an open-source AI software-development firm. The human is its client and supplies goals, constraints, feedback and approvals. Repeated model selection and manual terminal orchestration are not the product model. The selected conversational Lead may be native or external; it does not own canonical state, scheduling, policy, context, documentation, evidence or completion.

Symbiote owns Tasks, dependencies, decisions, artifacts, permissions and completion gates. The worker requests completion; Symbiote verifies completion. Native todos, subagent state, session history, successful exits and “done” reports are advisory foreign execution evidence. No metric, prompt, native completion claim, goal or learned method weakens security, impact, documentation, independent review, delivery or Capability Closure gates.

The process is code; expertise is pluggable. Deep Guidance is native stateful methodology: breadth-before-depth discovery, competitor and adjacent gap analysis, cross-cuts, decisions, implementation simulation, ambiguity audits, upstream remediation and independent re-audit. Skills and instructions provide expertise and guidance. Only demonstrated native or Host mechanisms enforce policy; observation is never prevention. Hosting, distribution, CI/CD, operations and rollback are researched from workload requirements during planning.

## Projects and workforce

Each dedicated multi-root Project owns its settings, Team, Lead, sessions, tasks, artifacts, knowledge, indexing, environment, Git state, terminals and layouts. Project switching restores an operating context, not a filter over global sessions. Project-local intelligence does not authorize maintainer telemetry or private knowledge transfer between Projects.

Keep these identities separate: stable Role; Workforce Binding; Workforce Runtime Contract; harness/runtime profile; inference provider/model; authentication method; billing entitlement; Host; dispatch/session. A Role survives changes to runtime, account, model or Host. A Workforce Binding selects eligible execution resources and policies for a Role. The versioned Workforce Runtime Contract is the deterministic, provenance-bearing compilation of resolved inputs for a dispatch, rather than the Role or runtime's mutable session state.

Resolution follows `Task → Role → Workforce Binding → Harness/runtime Profile → Eligible Host`. Compilation records input identities/versions, Workforce Protocol, Role Operating Contract, canonical Task Contract, Context Bundle policy and budget, permissions, skills/MCP/tool requirements, verification/documentation gates, artifacts and escalation policy. Identical pinned inputs must produce an equivalent contract; unsupported or ambiguous mappings fail explicitly. Executable schemas and compiler conformance remain with canonical owners #36, #184, #203 and #205.

`NATIVE_SYMBIOTE` and `EXTERNAL_HARNESS` are peer runtime kinds. External-only, hybrid and fully native configurations preserve the same canonical authority. Symbiote Agent works without third-party coding CLIs; embedding an external full agent loop remains an external integration. AgentRuntimeAdapter and InferenceProviderAdapter are separate; ProviderConnection, CredentialReference and BillingEntitlement are distinct. Capable local endpoints permit native operation without an inherently required internet service. Authentication, subscription and entitlement arrangements require verified support; subscription work cannot silently become billable API work.

Every Harness Pack declares control strength as native, wrapped/Host-enforced, externally observed, emulated or unsupported, using the strongest verified supported mechanism. `.claude/`, `.codex/`, `.pi/`, `.opencode/` and equivalents are projected/runtime state, with canonical desired intent in Symbiote. Preserve unknown user-owned content and independent upstream use where supported.

## Architecture and trust

Rust owns `symbioted`, the canonical control plane, native Agent Runtime, CLI, process/worktree supervision and first-party intelligence coordination. The workbench uses strictly checked TypeScript and prefers React compiled to static assets. Go is not a second core language. Justified SDK/analyzer sidecars may use another language; external extensions use versioned protocols.

Electron is ineligible for the desktop. Tauri 2 is the preferred, unproven shell candidate. SurrealDB is the preferred, unproven System Graph candidate: verify actual version, behavior, resource use and deployment model; evaluate transactional control-plane persistence separately. No dependent implementation may silently turn these candidates into final choices. #38 owns representative proof; failure requires documented evidence and a non-Electron alternative without dropping Preview requirements.

The Linux-first Host owns canonical state; desktop, CLI and cross-platform/remote clients share native execution and Host semantics. Closing or crashing desktop must leave authorized headless work alive. Shared Rust libraries do not require a single giant process. Isolate processes for trust, recovery, cancellation or resource limits; do not create a service per concept. Native workers cannot directly write privileged canonical state. Protocol extensions are the default; evaluate sandboxed WebAssembly for suitable bounded workloads, not arbitrary native libraries loaded into the trusted daemon.

Preview applications, including localhost content, are untrusted and receive no workbench/Host authority. An iframe is not an assumed security boundary. Use an authenticated narrow design bridge, validated operations and isolated permissions. Worktrees isolate Git mutation, not network access, secrets, production services or the filesystem. The threat model and enforcement evidence remain required.

## Knowledge, context and completeness

The System Graph models source/symbols, calls, supported data/control flow, contracts, schemas, events, configuration, infrastructure, tests, runtime evidence, history, requirements, decisions, documentation, tasks and releases. Provenance, uncertainty, coverage and unresolved dynamic boundaries are visible. Universal complete blast-radius knowledge is not promised.

The Context Broker supplies the smallest sufficient task/Role/model/permission/Host-aware evidence package, accounting for native instructions, tools, history and reserved capacity. Use progressive expansion and structured handoffs rather than transcript dumping.

Living Documentation is structured Developer Knowledge with stable identity, evidence, rationale uncertainty, confidence, freshness, audience, ownership and code links. Human documentation and compact agent context are projections of the same knowledge. Reconstruction distinguishes CONFIRMED, DERIVED, INFERRED and UNRESOLVED knowledge. Retain local invariant, safety, algorithm and external-constraint comments; externalize long architecture/history prose and remove obvious narration. Preserve necessary local warnings. Support code↔docs navigation, documentation blast radius, stale/contradicted/debt states, Context Broker use, Documentation Closure and release/capability integration. Generated documentation cannot certify itself or replace approved/executable primary evidence.

Tasks are execution units. Capability Closure covers applicable functionality, states, data, security, UX/accessibility, integrations, documentation, operations, delivery, recovery and support. Thin tests do not prove a capability complete. The target is no known unresolved release-blocking defect within verified scope; approved debt, unknowns and unmeasured behavior remain visible.

## Workbench and performance

Preserve floating voice/chat/hybrid Lead controls, structured sessions/files navigation, a responsive Agent Floor, collapsible tabbed terminals, Project Control/tasks sidebar, editor, Artifacts, Graph and Intelligence views. Preview is an interactive design surface separate from agent web research. Element feedback and visual edits must map to source and honor shared tokens, localization and data origins. Temporary DOM edits are not implemented features.

Measure the entire process tree, separately attributing shell/renderers, Preview, language servers, database, analyzers, SDK bridges and agents. Require bounded caches/queues/scrollback, backpressure, lazy loading, virtualization, real cancellation and Project-service suspension. Do not instantiate full editors/terminals/transcripts per agent card or replicate the whole graph in every process. Blank-window benchmarks do not establish product resource cost.

## Delivery, delegation and learning

Material mutation belongs to an issue-linked Change Stream and appropriately isolated branch/worktree. Preserve contributors' work. Detect textual and semantic collisions, and verify the exact current-target integration candidate; stale green checks do not establish merge safety. Required independent review, current-head checks and release gates remain separate evidence.

Goal execution and delegation are bounded, resumable, permissioned and observable, with root/child lineage and aggregate budgets. Children cannot bypass staffing or acquire undelegated authority. Opt-in scoped learning improves approved methods without changing security, staffing, billing, entitlements or locked decisions silently. Promotion and rollback remain controlled.

Global/Project/task analytics account for roots, children, reviews, failed attempts, remediation, infrastructure and unknowns. Improvement claims require comparable outcomes, unchanged quality gates, denominators, sample sizes and missingness. Unknown model attention, hypothetical defects prevented and counterfactual savings are not measurements.

## Change control and open acceptance

Accepted constitutional changes require a superseding ADR with authority, constraints, alternatives, dated/versioned evidence, reversibility, consequences and downstream impact propagation. Runtime/provider/platform facts expire independently and require refreshed official evidence. No proprietary required model, forced inference reseller, editor-fork dependency, transcript authority, generated-doc circular authority or provider-local completion authority is permitted.

This document establishes the prerequisite contract, not a production capability. #170 remains open until its schemas, deterministic compilation/provenance, external/native/hybrid conformance, enforcement and denied-completion evidence, documentation model, threat implications, contradiction audit, applicability review, exact implementation/PR/current-head evidence and required review are accounted for. Runtime failure, interruption, recovery, concurrency and permission checks remain pending until real mechanisms exist; mocks cannot pass later integration acceptance.

Hard prerequisites are only the live issues' **Dependencies** sections. Later canonical integrations do not become circular foundation prerequisites. #173 depends on #170; #38 depends on #170 and #173. Draft proof/governance artifacts may be prepared concurrently, but dependent implementation readiness must be established explicitly. The [ADR impact map](adr-0001-technology-direction.md#canonical-impact-map) preserves canonical ownership.
