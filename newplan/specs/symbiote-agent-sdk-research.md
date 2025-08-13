# Symbiote Agent SDK — Research & Decisions (Plan-Only)

Status: Draft v0.1
Scope: Research tracks and evaluation criteria for the Rust Agent SDK and adjacent ecosystems; outcomes feed into SDK spec and repo architecture.

## Research goals
- Select durable primitives for agents/tools: traits, typed bus, scheduler, hooks/slash, safety, observability
- Validate ecosystem choices for 2025+: performance, maintenance, security posture, cross-platform support
- Define spike/benchmark methodology and acceptance thresholds

## Decision areas & candidates

1) Concurrency/runtime primitives
- tokio + mpsc + broadcast + async-stream (baseline)
- actor-style libs: xtra, Heph, Actix (subset), bastion (status?)
- channels: flume vs tokio mpsc (latency/throughput, backpressure behavior)
- decision: typed event bus built on tokio; evaluate xtra for agent actors

2) Typed Event Bus & Scheduler
- Options: custom on tokio; messagebus crates; tower + services
- Criteria: typed topics, backpressure, budgets, cancellation, tracing hooks
- Plan: POC two designs (custom vs messagebus) and benchmark under load (N agents, M events/sec)

3) Observability
- OpenTelemetry (otlp) exporters + tracing-subscriber; metrics (prometheus/opentelemetry-metrics)
- Criteria: minimal overhead, span links across agents, redaction hooks
- Plan: span propagation through bus; verify “no secrets” traces by default

4) Storage & Context engines
- SQLite via SQLx (required)
- Qdrant client (qdrant-client) — check 2025 API surface, HNSW/PQ, payload filtering
- Neo4j driver (neo4rs) — maintenance status, async perf
- Criteria: Windows/macOS/Linux parity, active maintenance

5) Git & VCS
- gix (gitoxide) as primary vs git2 fallback
- Criteria: large repo perf, LFS awareness, worktrees support, Windows perf
- Plan: perf benchmark on 100k+ files repo; worktrees lifecycle API test

6) Desktop/UI
- Tauri v2 — IPC, permissions, updater; Leptos desktop integration
- Criteria: security (CSP, protocol handlers), plugin ecosystem, updater stability

7) Provider agents & streaming
- OpenAI/Anthropic/Gemini/OpenRouter/Local/Groq/Qwen
- Criteria: normalized streaming frames, tool-call envelopes, JSON/structured output
- Plan: define common envelope; parity tests per provider; record/replay fixtures

8) Testing strategy
- Golden I/O, integration with emulators/connectors, E2E (Playwright), performance harness (latency/throughput)
- Criteria: reproducible; no mocks for core behaviors; flake budgets

9) Security & Approvals
- Profiles, approvals escrow, vault/egress enforcement, DLP
- Criteria: zero raw secrets in context/logs; policy-as-code for approvals

## Benchmark plans
- Bus throughput: N agents producing/consuming on topics; p50/p95 latency; drop rate < 0.5%
- Scheduler fairness: starvation-free; priority correctness under load
- Context retrieval: recall@k vs latency on corpora sizes; budget enforcement correctness
- VCS operations: status/diff on large repos; worktree creation/teardown time

## Risks & mitigations
- Library churn: choose well-maintained crates; wrap behind traits; version pins
- Cross-platform quirks: Windows file path and process model; CI matrix includes all OSes
- Provider API changes: normalization layer and robust feature detection/healthchecks

## Research workflow
- Use Context7 for current docs/versions; complement with curated web research
- Capture findings and decision logs in this file; update SDK spec accordingly
- Small spikes with minimal code where needed; keep artifacts under tests/bench/

## Timeline (suggested)
- Week 1–2: bus/scheduler POCs, observability plumbing, initial benchmarks
- Week 3–4: provider normalization envelope + parity tests; VCS perf spike
- Week 5–6: context engine perf checks (qdrant/neo4j); desktop IPC/security review; draft decisions
- Week 7: finalize SDK v0.1 plan; lock in crate APIs and traits



## Candidates snapshot (to validate with Context7)

Actor/Bus
- Kameo (actors, fault-tolerant, async) — candidate for agent actors; verify maintenance and ergonomics
- Actix (mature actor framework) — powerful but heavier; consider only for specific cases
- Ractor (pure-Rust actors) — evaluate supervision tree features
- Custom typed bus on tokio (mpsc/broadcast) — likely baseline; benchmark vs messagebus crates

Git/VCS
- gix (gitoxide) — primary; large-repo perf, worktrees, LFS awareness
- git2 (libgit2) — fallback for edge operations

Context & Storage
- SQLx (SQLite) — compile-time query checking; required
- qdrant-client — HNSW, PQ, payload filters; check 2025 API/compat
- neo4rs — async Neo4j; verify maintenance cadence

Observability
- tracing + tracing-subscriber; opentelemetry + otlp exporters; metrics via OTEL

Desktop/UI
- Tauri v2 — IPC, security, updater; Leptos desktop integration

Providers
- Direct HTTP adapters (OpenAI/Anthropic/Gemini/OpenRouter) with our normalization layer
- Optional: rust-genai as reference for provider shapes; we still implement our own

## Immediate spikes (POCs)
- Typed Bus POC: custom tokio-based bus with typed topics, budgets, backpressure; benchmark N producers/consumers
- Provider Envelope: define normalized streaming/tool-call/JSON frames; implement shim for one provider to validate
- VCS Perf: gix status/diff on 100k-file repo; worktrees lifecycle; compare against git2 for edge cases
- Context Recall: minimal SQLite+Qdrant retrieval loop; measure recall@k vs latency targets
- OTEL Plumbing: span propagation across bus and agents; verify “no secrets” traces

## Open questions to resolve
- Do we need actor supervision trees in v1 or is typed bus + scheduler enough?
- FFI/dynamic plugin boundary in v0.2: which interface stable layer and sandbox model?
- Which provider becomes the baseline for normalized tool-call framing (OpenAI vs Anthropic)?
