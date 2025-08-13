# Symbiote Agents Architecture (Plan-Only)

Status: Draft v0.1 (iterative)
Scope: Agent taxonomy, runtime, hooks, slash commands, safety, and acceptance.

## Goals

- Personal Assistant orchestrates; domain agents execute with typed, auditable, reversible actions
- Strong defaults: budgets, approvals, least-privilege, reproducibility, observability
- Hybrid context packs (graph→vector→range) for accuracy and surgical diffs


## Agent Taxonomy (Initial Set)

- Assistant Orchestrator
  - Intent→route→plan→monitor; HiL approvals; budgets
- GitHub Agent (PR & Repo Automation)
  - Branch/worktree/PR lifecycle, reviews, checks, merge queue, backports, releases
- VCS Agent (Low-level Git)
  - gix-based ops, worktrees orchestrator, stash/rebase/cherry-pick, LFS awareness
- Planner Agent
  - PRD/ADR/task scaffolding; traceability; duplication avoidance
- Context Agent
  - Retrieval plans & ContextPacks; invariants for edits; budget-aware packing
- Refactor/Editor Agent
  - AST-guided surgical diffs; idempotent patches; rollback; language guardrails
- Workflow Agent
- Workflow Builder Agent (NEW)
  - Designs AI agent workflows in the visual builder: drafts nodes/edges, validates schemas, selects models/tools, inserts tests
- Workflow Debugger Agent (NEW)
  - Diagnoses failing workflows: step traces, node input/output diffs, retries/backoff tuning, compensation plans, reproducer creation

  - Visual workflow execution; retries/backoff; saga/compensation; backpressure
- Security Agent
  - Permission Profiles (ZeroTrust/HiL/Full Auto); approvals; audit
- Telemetry Agent
  - Cost/tokens/latency; anomaly detection; budgets; policy viol. alerts
- Debug Agent (NEW)
  - DAP sessions; breakpoints; step/run; variable/watch; attaching; snapshotting
  - Triage failing tests; bisect; repro harness; log/trace correlation
- E2E Test Agent (NEW)
  - Playwright orchestration; cross-browser matrix; flake detection & retries
  - Test planning by feature; seed/setup; screenshots/video/artifacts; CI hooks

## Provider-Specific Agents (SDK + CLI)

Pattern
- Transport modes: SDK/HTTP (preferred), CLI (where SDK lacking), Local Runner (Ollama/vLLM/LM Studio)
- Contract: same Agent/Tool traits; normalized request/response (stream frames, tool calls, JSON/structured outputs, vision/audio)
- Safety: vault-backed API keys; egress allowlists; redaction; budget guard; PII/data-policy tags
- Observability: per-call cost/tokens/latency; provider/model attribution; traces with “no secrets” by default
- Testing: golden parity tests vs live provider for streaming/tool frames; record/replay fixtures; no mocks for core behavior

Initial Agents (v0.1)
- OpenAI SDK Agent
  - Chat/Responses, Tools/Function Calling, JSON/Structured Outputs, Vision/Images, Batch Embeddings
  - Slash: /openai chat|tools|embed …; Profiles: dev/test/prod keys
- Anthropic SDK Agent (Claude + Claude Code)
  - Tool Use, Reasoning, Code-focused workflows; “Claude Code” mode integrates with Refactor/Editor Agent
  - Slash: /claude chat|code …; Hooks: before_patch/after_patch for code suggestions
- Google AI SDK Agent (Gemini) + Gemini CLI Agent
  - Multimodal (vision/audio), Tools, JSON mode; CLI wrapper when SDK is unavailable or for admin ops
  - Slash: /gemini chat|tools|cli …
- OpenRouter Agent (Pass-through)
  - Raw model id support for experiments; enables any available model without pre-list; respects provider policy flags
  - Slash: /openrouter call --model <id> …
- Groq Agent (Optional)
  - Fast inference routing; parity with tools/JSON/streaming where supported
  - Slash: /groq chat|tools …
- Qwen CLI Agent (Optional)
  - Wraps Qwen/OpEncoder CLIs when present; standardizes streaming/tool frames; local/offline best-effort
  - Slash: /qwen cli --model …; Healthcheck: binary presence, version, model availability
- Local Runners Agent
  - Ollama/vLLM/LM Studio bridges; quant/VRAM awareness; local privacy mode; cost≈0 accounting with device metrics
  - Slash: /local chat|tools --runner ollama --model …

Capability Matrix (normalization goals)
- Chat/Streaming: token deltas, reasoning deltas (if available), tool-call start/stop frames
- Tools/Function Calling: schema-validated tool args; parallel calls; backpressure and budget limits
- JSON/Structured Outputs: schema-constrained decoding; non-conformant frames rejected with recovery paths
- Vision/Audio: images, OCR hints, bounding boxes; ASR/TTS when supported
- Reasoning: tokens budgeted; cost/latency estimators aware of provider semantics

CLI Agent Wrapper rules
- Discovery: PATH + Settings → explicit binary path; version check; `--help` probe
- Security: sandboxed spawn; timeouts; stdout/stderr parsing; redact secrets from logs
- Concurrency: worker pool; backpressure; per-workspace execution sandboxes
- Fallbacks: prefer SDK/HTTP when available; CLI used for missing features or admin tools

Slash Commands (Providers)
- /openai chat "…" --tools schema.json --json --model gpt-4.1
- /claude code refactor src/file.rs:12-48 "rename function and update callers"
- /gemini cli models list --project …
- /openrouter call --model anthropic/claude-3.7-sonnet --prompt "…"
- /local chat --runner ollama --model qwen2.5:7b-instruct --tools …

Acceptance (Provider Agents)
- Parity: streaming frames/tool calls/JSON mode normalized across providers
- Budgets: pre-exec cost estimate ±10%; enforce soft/hard limits; route to “Best Value” when set
- Health: live healthchecks (SDK auth, CLI presence, rate limits) surfaced in Assistant UI
- Safety: egress/PII policies enforced; signed audit for high-risk operations

## Runtime (crates/agents)
- Traits: Agent, Tool, Capability; typed requests/responses (serde)
- Bus: in-process typed event bus (publish/subscribe) with topics and backpressure
- Scheduling: per-agent queues, priorities, budgets; cooperative cancel/pause
- Memory: working/task/episodic/semantic via memory/ with TTL + PII policy
- Safety: profiles, preflight risk scoring, approvals escrow; vault SecretHandles only
- Observability: per-step spans; "no secrets" traces by default; deep traces by approval

## Hooks System (Deep Integration)
- Hook types
  - Lifecycle: before_route, after_route, before_execute, after_execute, error, finally
  - Domain: before_commit, after_commit, before_pr_open, after_pr_merge, test_started, test_flaked
  - Context: before_retrieval, after_retrieval, before_patch, after_patch
- Hook targets
  - Built-in (Rust functions), Workflow nodes, External connectors (webhooks), Slash Commands
- Registration
  - Declarative in settings (Global→Workspace→Project) with allowlists; code-registry in crates/agents
- Execution
  - Typed payloads; timeouts; budgets; retries with jitter; circuit breakers
- Safety
  - Hook isolation; egress allowlists; rate limits; audit logs; redaction

## Slash Commands (Deep Integration)
- Parser: crates/assistant parses /verb args → typed command → routes to agent/tool
- Security: per-command scopes; profiles; approvals on high-risk commands
- Examples
  - /plan "auth + db" — Planner Agent; outputs PRD+tasks
  - /worktree new feature-x — VCS Agent; creates isolated worktree
  - /pr open --title "X" — GitHub Agent; opens PR; links spec/tasks
  - /review 123 — GitHub Agent; context-aware review; suggested patches
  - /test e2e compare-panel --browser all — E2E Test Agent; Playwright matrix
  - /debug attach pid:1234 — Debug Agent; start DAP; collect snapshot
- Acceptance
  - Parse < 10ms; authorization enforced; audit with trace_id; no duplicate dev servers

## Debug Agent (Details)
- DAP integration (Rust DAP clients for Node/Rust/Python/Go as supported)
- Features: breakpoints, step in/over/out, watch, evaluate, exception break
- Attach/launch: map to workspace config; env/profiles; symbol/source maps
- Test triage: run failing tests focused; capture traces; propose surgical fixes
- Acceptance: attach < 2s; step latency < 100ms; stable across OS matrix

## E2E Test Agent (Details)
- Playwright control: select projects/browsers; shard; retries; collect artifacts
- Seed/setup: DB seeding, emulators, fixtures; teardown; idempotent runs
- Flake analysis: retry budget, failure clustering, quarantine and alerting
- CI Integration: annotate PRs with results; attach screenshots/videos; rerun failed
- Acceptance: green path < 5 min for core suite; < 2% flake budget; artifacts preserved

## GitHub Agent (Details)
- Branch strategy; protected main; Conventional Commits; signed commits/tags
- PR lifecycle: draft→ready; labels/reviewers; checks; artifacts; merge queue
- Reviews: hybrid-context risk analysis; minimal patch suggestions; compile guard
- Backports: cherry-pick with conflict helper; labels; changelog updates
- Acceptance: SCM parity; N concurrent worktrees; no orphan dirs; single dev server per worktree

## VCS Agent (Details)
- gix primary; git2 fallback (feature-gated)
- Worktrees orchestrator: allocate/teardown; resource guard; auto-clean
- Partial staging; stash; rebase/cherry-pick; blame/history; LFS aware

## Planner & Context Agents
- Planner: spec-first; dedupe; traceability; acceptance propagation
- Context: retrieval plans; budget-aware range assembly; invariants verification

## Safety & Governance
- Profiles & approvals baked-in; high-risk actions prompt the mobile Remote Control app
- Vault SecretHandles only; Egress allowlists; redaction policies; DLP hooks optional

## Observability & Budgets
- Per-agent costs/tokens/latency; budgets (soft/hard) per task/PR/workspace
- PR-linked traces with provider/model/tool attribution; anomalies flagged

- Provider Agent Test Matrix (parity targets)
  - OpenAI: chat/stream/tool/JSON/vision; function-calling; batch embeddings
  - Anthropic (Claude/Claude Code): tool use; reasoning; code suggestions integration
  - Google AI (Gemini): multimodal (vision/audio); tools; JSON mode; Gemini CLI coverage
  - OpenRouter: arbitrary model IDs; streaming/tool parity passthrough
  - Local Runners (Ollama/vLLM/LM Studio): chat/tool parity; device/VRAM awareness
  - Optional: Groq; Qwen CLI
- Acceptance: all tested providers meet normalized envelope compliance and parity thresholds; record/replay fixtures maintained

## Testing Strategy
- Golden tests: agent request/response; tool I/O schemas; hook payloads
- Integration: connectors/emulators; GitHub sandbox; DAP adapters; Playwright smoke
- E2E: Assistant chat flows to agents; approvals; PR creation→merge; debug attach→fix

## Roadmap
- v0.1: GitHub, VCS, Refactor, Context, Security, E2E Test, Debug
- v0.2: Merge queue/backports/releases; GH App webhooks; advanced DAP; flake triage UI
- v0.3: GitLab/Bitbucket variants; release automation; mobile approvals expansions

