# Symbiote Repository Architecture (Plan‑Only, Iterative)

Status: Draft v0.1 (for iteration)
Scope: Repository layout, layering, capabilities, and rules to guide dispersion of the unified spec into code.
Non‑Goals: Implementation details, language‑specific micro‑choices, dependency versions.

## Objectives

- Single Rust monorepo with clear layering and strict boundaries
- Hybrid context engine: recommended, not mandatory; embedded local by default; workspace/project overrides
- Personal Assistant as first‑class orchestrator (global chat) across all systems
- Connectors as first‑class modules with Quick Connect flows, emulator parity, contract tests
- Observability, budget/cost guardrails, permissions, and approvals enforced at the edges
- Keep plan artifacts in newplan/; do not clutter code folders

## Principles

- Hexagonal layering: apps → domain crates → engines/adapters → foundations
- Capability flags: optional engines/connectors via Cargo features; runtime health gating
- Testability first: emulator/localstack parity, golden tests, e2e Playwright flows
- Safety first: vault/egress centralized; zero raw secrets in prompts/agents
- Scale first: large‑repo performance and retrieval quality are core acceptance criteria

## Top‑Level Layout (Proposed)

- apps/
  - desktop/  (Tauri shell)
  - ui/       (Leptos frontend; panels for Assistant/IDE/Workflow/Trading/MCP)
  - cli/      (developer utilities: scaffold/migrate/admin)
- crates/
  - core/         (types, ids, time, errors, feature flags, DI)
  - agents-sdk/   (common SDK for agents/tools: traits, bus/scheduler, hooks, slash, budgets/approvals, telemetry bindings)
  - context/      (hybrid context engine: SQLite required; Qdrant/Neo4j optional)
  - memory/       (working/episodic/semantic; Mem0‑compatible traits)
  - ai/           (provider adapters + router: OpenAI/Anthropic/Gemini/OpenRouter/local)
  - agents/       (agent framework primitives, policies, quality rules)
  - assistant/    (Personal Assistant orchestrator: router, planner, sessions, approvals, contextpacks)
  - trader/       (Ultimate Trader Agent Team: strategy, backtest, risk, execution, portfolio, exchange/DeFi, compliance, reporting)
  - tools/        (typed tool SDK: AST transforms, git ops, fs, http, shell)
  - workflow/     (visual workflow runtime, state, retries, saga/compensation)
  - nodes/        (category sub‑crates: ai/data/dev/control/triggers/outputs)
  - ide/          (LSP, diagnostics, debugger helpers, editor integrations)
  - diff/         (surgical diff engine: AST‑guided edits, invariants, patch journal)
  - mcp/          (MCP manager, settings, lifecycle, import adapters)
  - catalog/      (model catalog client + routing policies; Supabase → local cache)
  - settings/     (hierarchical: Global → Workspace → Project; profiles)
  - storage/      (SQLite abstraction, migrations, caches, patch/trace journal)
  - security/     (permission profiles, approvals, audit)
  - vault/        (OS keychain bridges, SecretHandles, crypto ops)
  - egress/       (policy‑enforcing HTTP client, allowlists, TLS pinning)
  - telemetry/    (OTEL exporters, budgets/cost metrics, anomaly detection)
  - connectors/   (first‑class connectors; see below)
  - browser/      (controlled automation contracts for tests/computer‑use)
- tests/
  - e2e/         (Playwright projects: Assistant, Compare Panel, Quick Connectors, MCP)
  - integration/  (connectors vs emulators; node contract tests)
  - performance/  (large‑repo corpus; retrieval recall@k, indexing lag, edit success)
  - golden/       (adapter parity, node I/O)
- infra/
  - docker/       (qdrant, neo4j, postgres; firebase/localstack)
  - devcontainers/ (repro dev envs)
  - ci/           (lint/build/test/release/SBOM/signing helpers)
  - configs/      (profiles dev/test/prod; egress allowlists; connector templates)
- examples/       (minimal apps, workflows, connector demos)
- docs/           (code docs, ADRs; plan remains in newplan/)
- newplan/        (PRDs/specs only; single source of truth)

## Source Control & GitHub Integration (Plan-Only)

Crates
- vcs/: source control engine (gix primary, git2 optional) — branching, partial staging, rebase/cherry-pick, stash, blame/history, LFS
- github/: GitHub integration — auth, PRs, issues, reviews, checks, labels, statuses, releases

Worktrees Orchestrator
- Per-task ephemeral worktrees bound to Assistant/VCS/GitHub agents
- N concurrent worktrees; auto-clean on PR merge/close; resource guard prevents duplicate dev servers
- Policy: protected branches; signed commits/tags via vault; CODEOWNERS-aware approvals

UI Panels
- panels/sourcecontrol: Changes, Staged, History, Branch, Remote
- panels/pull_requests: PR dashboard with checks, artifacts, comments, merge queue

Safety & Observability
- Tokens/keys in vault; egress allowlist and TLS pinning for github.com
- PR-linked traces and costs; CI status surfaced; violations highlighted

Acceptance
- VS Code parity for SCM basics
- Worktrees: reliable create/switch/teardown; no orphaned dirs
- GitHub: create/update/merge PRs; labels/reviewers; checks; artifacts; comments


## Connectors (First‑Class)

- crates/connectors/
  - supabase/, firebase/, vercel/, stripe/, lemon/, qdrant/, pinecone/, astradb/, neo4j/, aws/, gcp/, azure/, s3/, gcs/, redis/, postgres/
- Contract
  - Connector trait: auth methods, scopes, healthcheck, rate limits, emulator support
  - Config schema (serde + JSON Schema), profile awareness (dev/test/prod)
  - Quick Connect: auto‑create SecretHandles + egress allowlists; validate with live healthcheck
  - Node registration hooks; typed inputs/outputs; golden tests; rollback on failure

## Context Engine (Recommended, Not Mandatory)

- SQLite (required): metadata, symbol index, patch journal, run/trace logs
- Qdrant (optional): embeddings search; HNSW configs; PQ compression; payload filtering
- Neo4j (optional): code graph (File, Symbol, Module, Import, Call, Test, Owner, etc.)
- Memory substrate (Mem0‑compatible) for working/episodic/semantic memory
- Profiles: App‑Core (embedded defaults) → Workspace/Project can switch to local self‑hosted or cloud
- Retrieval: graph narrow → vector refine → range assembly; context budget planner

## Assistant (First‑Class)

- Orchestrator of agents, workflows, and tools; global awareness with workspace lock
- Subsystems: router, planner, sessions, approvals, contextpacks, memory bindings, multimodal, computer_use
- Governance: budgets, cost guard, provider reliability/failover; audit and approvals integrated

## Workflow Nodes (Delivery Phases)

- v0.1: Supabase, Postgres, Qdrant, Neo4j (optional), Stripe, Vercel, S3/GCS/Blob
- v0.2: Firebase, Pinecone, AstraDB, Lemon Squeezy, Cloudflare, Render, Netlify
- v0.3: Redis Streams/Kafka, Bedrock/Vertex/Azure services, Datadog/Sentry, Weaviate
- Every node: JSON Schema I/O, emulator support, golden tests, live healthcheck, scope/profile pickers

## Workflow Builder & Debugger (Plan-Only)

Agents and runtime
- Workflow Builder Agent: composes nodes/edges, validates schemas, selects models/tools, inserts tests
- Workflow Debugger Agent: step traces, node I/O diffs, retries/backoff tuning, compensation plans, minimal reproducers
- workflow/ crate: runtime that executes nodes, manages retries/backoff, saga/compensation, backpressure

UI panels
- panels/workflow/designer: AI-assisted graph design, schema validation, quick test generation
- panels/workflow/debugger: timeline, per-node traces, input/output diffs, reproduce run, artifact viewer

Hooks
- workflow_designed, schema_validated, node_started, node_failed, compensation_started, workflow_completed, workflow_debug_snapshot

Safety & isolation
- Tool scopes per node; approvals for shell/cloud nodes; debug runs in sandboxed env; per-run budgets and timeouts

Acceptance
- Design: valid schemas, runnable flow, tests inserted; Debug: deterministic reproducer, root-cause classification; Runtime: compensation reaches stable state


## Layering & Dependencies

- apps/* depend on high‑level domain crates; never on low‑level connectors directly
- connectors/ never depend on apps; only on core/vault/egress/settings/telemetry
- diff/ depends on ide/context/tools/storage; enforces CODEOWNERS & boundaries
- ai/ exposes unified provider contracts; agents/assistant build atop it

## Cargo Features & Build Flavors

- Features: context‑qdrant, context‑neo4j, connector‑*, provider‑*, mcp‑*, ops‑sbom, ops‑signing
- Flavors: local‑only (embedded engines), cloud‑extended (connectors on), air‑gapped (strict egress)
- Runtime health: features enable code paths; runtime health gates UI with clear banners


## Workspace Isolation Model

Dimensions
- Data: separate SQLite files per workspace; Qdrant collections namespaced (e.g., vec_code_{workspace_id}); Neo4j labels per workspace; cross‑workspace queries disabled by default
- Secrets: Vault SecretHandles scoped Global → Workspace → Project; least‑privilege access; no raw secrets in prompts/logs
- Network/Egress: per‑workspace allowlists + TLS pinning; “air‑gapped” flavor to fully block external egress
- Policy/Permissions: independent Permission Profiles (ZeroTrust/HiL/Full Auto) and approvals queues per workspace; separate budgets (daily/weekly hard/soft caps)
- Compute: dedicated task queues/executors per workspace; risky ops (shell, browser, code apply) in sandbox helpers; optional Hard Isolation spawns per‑workspace services
- Context: hybrid retrieval restricted to workspace scope; memory (working/episodic/semantic) partitioned per workspace; global memory is opt‑in and redacted
- UI/Session: Assistant sessions are global or workspace‑locked; header toggle; banners when capabilities limited (e.g., graph disabled, air‑gapped)

Modes
- Soft Isolation (default): shared embedded engines (SQLite always; Qdrant local; Neo4j optional) with strict namespacing and policies; low resource cost
- Hard Isolation (opt‑in): per‑workspace processes/services via docker‑compose or dedicated OS processes; workspace‑scoped temp dirs and env; higher resource cost

Controls
- Settings → Workspace → Isolation: select Soft/Hard; toggle air‑gapped; manage allowlists; choose engine endpoints
- Runtime health gates UI features; switching modes triggers safe rebinds and cache invalidation

Acceptance Criteria
- No cross‑workspace data access (DB, vectors, graph); verified by tests
- Separate budgets and approvals per workspace; audit trails include workspace id
- Sandbox risky ops; teardown leaves no residual processes or temp files; cleanup verified

## Testing Strategy

- Large‑repo regression: nightly on ≥1M LOC; recall@k, indexing lag, edit success rate
- Provider/OS matrix: Win/macOS/Linux × OpenAI/Anthropic/Gemini/OpenRouter/local
- Connectors: emulator‑first golden tests; healthcheck latency ≤ 2s; setup ≤ 60s; rollback clean
- Surgical edits: idempotent patches; language guardrails (TS Server, rust‑analyzer, ruff/rope, go/ast, JDT, Roslyn)

## Infra & Ops

- Docker compose for Qdrant/Neo4j/Postgres + emulators; one‑command dev up
- CI: feature matrix (qdrant on/off, neo4j on/off), SBOM generation, signing hooks
- Release channels: stable/beta/canary; code signing; auto‑update service (future)

## Dispersion Plan (Plan‑Only, No Code Yet)

- Phase 1: Create README skeletons in select crates (assistant, context, ai, workflow, connectors)
  - Each links to spec sections and acceptance criteria; no duplicated prose
- Phase 2: Add architecture/repository‑structure.md (this file) to the main spec ToC
- Phase 3: Track gaps as checklists; update READMEs as architecture evolves

## Acceptance Criteria (Architecture)

- Optional hybrid context supported by features and runtime checks; SQLite always present
- Assistant crate exists at top level and references orchestrator responsibilities
- Connectors are isolated, testable, and Quick Connect integrates with vault/egress
- Tests directories exist conceptually for e2e/integration/performance/golden
- Infra stubs defined for local engines/emulators; CI includes feature matrix

## Open Questions

- External plugin model (FFI/dylib) vs static crates for third‑party nodes/connectors in v1?
- Which connectors comprise the minimal viable set for v0.1 shipping?
- Auto‑update system scope for desktop: which channels and signing infra?
- Data residency enforcement: connector wrappers vs centralized policy engine?

## TODO (Iteration Checklist)

- [ ] Confirm Option A layout and naming
- [ ] Confirm v0.1 connector/node set
- [ ] Define Cargo feature matrix and defaults
- [ ] Document Quick Connect UX contracts and rollback semantics
- [ ] Specify large‑repo corpus composition and target metrics
- [ ] Define first‑run wizard scope (context profile + Quick Connects)
- [ ] Decide on plugin strategy (if any) for v1 vs later

