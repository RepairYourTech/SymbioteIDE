# Symbiote Agents ↔ Features Mapping (Plan-Only)

Status: Draft v0.1
Scope: For every major Symbiote feature, define the responsible agents, interactions, hooks, slash commands, safety, observability, and acceptance.
Non-Goals: Low-level API details; full schemas; provider versions.

## Conventions
- Agents: capitalized (e.g., "GitHub Agent").
- Hooks: snake_case lifecycle/domain/context events.
- Slash: user-facing commands starting with '/'.
- Safety: approvals, budgets, scopes, isolation.
- Obs: traces, metrics, artifacts.

## 1) Personal Assistant (Global Orchestrator)
Agents
- Assistant Orchestrator (primary)
- Planner Agent, Context Agent, Security Agent, Telemetry Agent (support)
Interactions
- Routes intents to domain agents; composes ContextPacks; enforces profiles/budgets
Hooks
- before_route, after_route, error, finally
Slash
- /vibe set interactive|auto; /scope global|workspace; /profile zerotrust|hil|auto
Safety
- Profile gate on tool calls; approvals on high-risk
Obs
- Turn cost/tokens/latency; route attribution; anomalies
Acceptance
- Intent→route < 500ms median; correct agent selection ≥95% in eval set

## 2) IDE & Code Intelligence (Editor, LSP, Diff)
Agents
- Refactor/Editor Agent (primary), Context Agent, Debug Agent, VCS Agent
Interactions
- AST-guided edits; compile/type checks; diff invariants; blame/history integrations
Hooks
- before_patch, after_patch, invariants_failed, build_started, build_completed
Slash
- /edit rename fn x→y; /search symbol Foo; /apply patch <id>
Safety
- Protected files/owners; approvals if crossing boundaries; rollback auto
Obs
- Edit success rate, compile errors, patch ids
Acceptance
- AST-safe patches; idempotent; compile success ≥98% within supported langs

## 3) Source Control & PRs (VS Code-level SCM)
Agents
- VCS Agent (primary), GitHub Agent, Refactor/Editor Agent, Security Agent
Interactions
- Worktrees per task; branch/commit/stage/partial hunks; PR lifecycle; CI status
Hooks
- before_commit, after_commit, before_pr_open, after_pr_merge
Slash
- /worktree new feature-x; /commit "feat: ..."; /pr open; /review 123
Safety
- Protected branches; signed commits; CODEOWNERS-aware approvals
Obs
- PR-linked traces; CI status; worktree cleanup
Acceptance
- VS Code parity for SCM basics; N concurrent worktrees; no orphaned processes

## 4) Visual Workflow Builder
Agents
- Workflow Agent (runtime), Workflow Builder Agent (design), Workflow Debugger Agent (diagnose), Tools (SDK), Planner Agent, Context Agent
Interactions
- Design: propose nodes/edges, select models/tools, generate JSON Schemas, insert tests; Execute: retries, backoff, saga/compensation; Debug: step traces, input/output diffs, reproducer flows
Hooks
- workflow_designed, schema_validated, node_started, node_failed, compensation_started, workflow_completed, workflow_debug_snapshot
Slash
- /flow design goal:"sync GH issues to Slack"; /flow validate name:ETL-1; /flow debug name:ETL-1 step:node-3; /flow run name:ETL-1; /flow export; /flow test name:ETL-1
Safety
- Tool scopes per node; approvals for high-risk nodes (shell/cloud); debug runs in sandboxed env; budgets per run
Obs
- Design quality score, schema errors, node timings, failure reasons, retries, costs per node and run, debug artifacts
Acceptance
- Deterministic replays; compensations reach stable state; debug produces minimal reproducer and root-cause classification

## 5) Model Catalog & Provider Routing
Agents
- Catalog Agent (inside catalog/), Provider Agents (OpenAI, Anthropic, Gemini, OpenRouter, Local), Assistant Orchestrator
Interactions
- Route selection by cost/latency/quality policy; live health and budget guard
Hooks
- before_provider_call, after_provider_call, budget_exceeded
Slash
- /route best_value; /provider set openai:gpt-4.1 for code
Safety
- Provider scopes; budget caps; zero-secrets traces
Obs
- Per-call cost/tokens/latency; provider attribution
Acceptance
- ±10% cost estimate; graceful failover; policy compliance

## 6) Context Engine & Memory
Agents
- Context Agent (primary), Assistant Orchestrator, Telemetry Agent
Interactions
- Graph narrow → vector refine → range assembly; memory TTL/PII policies
Hooks
- before_retrieval, after_retrieval, compaction_run, reembed_started
Slash
- /context profile local|hybrid; /index refresh; /memory purge workspace
Safety
- Workspace isolation; PII redaction; air-gapped support
Obs
- recall@k, latency, cache hit, token budgets
Acceptance
- Recall targets per corpus; budgets respected; no cross-workspace leakage

## 7) Connectors & Quick Connect
Agents
- Connectors Agent (manager), Security Agent, Vault, Egress
Interactions
- Healthchecks; SecretHandles; allowlists; emulator parity
Hooks
- connector_added, connector_failed, grant_expired
Slash
- /connect supabase; /connect github; /connector health vercel
Safety
- Scopes & time-boxed grants; MFA; approvals for sensitive scopes
Obs
- Connector health, grant usage, failure rates
Acceptance
- Setup ≤ 60s; healthcheck ≤ 2s; rollback on failure

## 8) Security, Approvals, Audit
Agents
- Security Agent (primary), Assistant Orchestrator, Telemetry Agent
Interactions
- Profiles (ZeroTrust/HiL/Auto); approvals escrow; audit log
Hooks
- approval_requested, approval_granted, policy_violation
Slash
- /approve <request-id>; /profile set zerotrust
Safety
- Required; mobile approvals supported; immutable audit
Obs
- Approval times, violation counts
Acceptance
- No high-risk actions without grant; full audit present

## 9) Vault & Egress (Secrets, Network Policy)
Agents
- Vault (daemon), Egress Proxy (policy), all agents via handles
Interactions
- SecretHandles only; signing and HTTP via broker; TLS pinning; allowlists
Hooks
- secret_used, redaction_applied, egress_denied
Slash
- /grant github:repo.write 15m; /egress allow api.github.com
Safety
- Never expose raw secrets; DLP/redaction; rate limits
Obs
- Secret usage counts; denied attempts
Acceptance
- Tests verify zero secret exposure; denied on disallowed domains

## 10) Telemetry & Observability
Agents
- Telemetry Agent (primary), all agents emit
Interactions
- OTEL spans; cost/tokens/latency; anomaly detection; budgets
Hooks
- budget_soft_limit, budget_hard_limit, anomaly_detected
Slash
- /budget set pr:10usd; /trace show last-run
Safety
- "No secrets" traces by default; deep trace only on approval
Obs
- Dashboards per workspace/PR/flow; cost attribution
Acceptance
- <1% events dropped; budget alerts timely

## 11) Debugging
Agents
- Debug Agent (primary), Refactor/Editor Agent, VCS Agent
Interactions
- DAP attach/launch; breakpoints; test triage; snapshot captures
Hooks
- debug_session_started, debug_exception, snapshot_captured
Slash
- /debug attach pid:1234; /debug run test my_suite::case
Safety
- Sandboxed helpers; controlled env; approvals for privileged ops
Obs
- Step latency, attach time, exception rates
Acceptance
- Attach <2s; step <100ms; stable across OSs

## 12) E2E Testing
Agents
- E2E Test Agent (primary), Workflow Agent, Security Agent
Interactions
- Playwright orchestration; seed/setup/teardown; artifact uploads; CI links
Hooks
- test_started, test_failed, test_flaked, artifacts_uploaded
Slash
- /test e2e panel:compare --browser all; /test rerun failed
Safety
- Sandboxed browsers; data resets; budgets on CI
Obs
- Pass/fail, flakes, durations, artifacts
Acceptance
- Core suite <5 min; <2% flake; artifacts preserved

## 13) Crypto Trading (Ultimate Trader Team)
Agents
- Strategy Research/Composer, Backtest/Simulation, Risk Manager, Execution, Portfolio/Rebalance, Exchange/Wallet, DeFi Ops, Compliance/Policy, Telemetry/Reporting
Interactions
- Strategy→sim→paper→live; approvals on risky ops; wallets via Vault; venue connectors
Hooks
- order_submitted, order_filled, risk_breach, circuit_breaker, promotion_ready
Slash
- /trade plan "BTC momentum" budget:100; /trade go live strategy:alpha-1; /trade kill
Safety
- Backtest gates; budgets; kill switch; KYC/region rules; hardware wallets
Obs
- P&L, VaR, max DD, slippage, fill quality, incidents
Acceptance
- No live before passing gates; incident response <30s; audit complete

## 14) MCP (Model Context Protocol)
Agents
- MCP Agent, Assistant Orchestrator, Tools
Interactions
- Server discovery/registration; hot-reload; on/off toggles; JSON import to settings
Hooks
- mcp_server_added, mcp_method_called, mcp_error
Slash
- /mcp add server.json; /mcp enable myserver; /mcp call tool:xyz
Safety
- Sandboxed; timeouts; quotas; approvals for dangerous methods
Obs
- Call rates, error rates
Acceptance
- Hot-reload works; isolation maintained

## 15) Browser Automation (Controlled)
Agents
- Browser Agent (contracts), E2E Test Agent (driver), Assistant Orchestrator
Interactions
- Playwright-based automation for allowed scenarios; strong guardrails
Hooks
- browser_start, nav_blocked, action_denied
Slash
- /browser open url:… scope:workspace; /browser click selector:…
Safety
- Allowlist only; no credential exfiltration; time-boxed sessions
Obs
- Actions, denials, latencies
Acceptance
- No policy breaches; reproducible scripts

## 16) Settings & Profiles
Agents
- Settings Agent (within settings/), Security Agent, Assistant Orchestrator
Interactions
- Global→Workspace→Project profiles; feature flags; isolation mode
Hooks
- profile_changed, feature_flag_toggled
Slash
- /settings show; /profile set workspace:hil; /isolation hard on
Safety
- Audited changes; rollback
Obs
- Change history
Acceptance
- Deterministic profile application; isolation switch safe

## 17) Examples & Tutorials (Assistive)
Agents
- Planner Agent, Assistant Orchestrator, Workflow Agent
Interactions
- Scaffold example projects; run sample flows; guided tours
Hooks
- example_created, tutorial_step_completed
Slash
- /example create qdrant-search; /tutorial start compare-panel
Safety
- Example keys isolated; teardown
Obs
- Completion rates
Acceptance
- Working, self-cleaning examples

## Cross-Cutting Integration (Providers)
- Provider Agents (OpenAI/Anthropic/Gemini/OpenRouter/Local/Groq/Qwen) are accessible from all features via Assistant routing and slash commands.
- Normalized streaming/tool/JSON behaviors ensure consistent downstream handling by agents.

## Evaluation & Coverage
- Feature-by-feature acceptance criteria above define done-ness.
- Test suites: golden, integration, E2E, performance; PR must pass relevant suites before merge.

