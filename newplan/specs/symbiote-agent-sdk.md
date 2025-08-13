# Symbiote Agent SDK (Rust) — Plan Only

Status: Draft v0.1
Scope: Core SDK to build agents/tools in Symbiote; traits, runtime, bus, integration, safety, testing, roadmap.
Non‑Goals: Provider-specific adapter code, UI details.

## Goals
- Single, ergonomic SDK to author agents/tools with strict safety and observability
- Deterministic, typed protocols: requests/responses, events, hooks, slash
- First-class integration with vault/egress/security/telemetry/context/memory
- Pluggable transports (in-proc bus initially), budgets and backpressure

## Crate layout (proposed)
- crates/agents-sdk/
  - src/lib.rs (prelude)
  - src/agent.rs (Agent trait, lifecycle)
  - src/tool.rs (Tool trait, schema I/O)
  - src/capability.rs (capability registry, discovery)
  - src/bus.rs (typed event bus, topics, backpressure)
  - src/scheduler.rs (queues, priorities, budgets, cancellation)
  - src/hooks.rs (lifecycle/domain/context hooks)
  - src/slash.rs (command schema, parsing helpers)
  - src/memory.rs (bindings to memory/ crate; working/task/episodic)
  - src/context.rs (ContextPacks API)
  - src/security.rs (profiles/approvals bindings)
  - src/vault.rs (SecretHandles use patterns)
  - src/egress.rs (policy client; allowlist, TLS pinning)
  - src/telemetry.rs (spans, metrics, costs/budgets)
  - src/errors.rs (error types; retryable vs fatal; user-facing messages)
  - src/testing.rs (golden I/O harness, record/replay, fixtures)

## Core traits
- Agent
  - fn name() -> &'static str
  - async fn plan(&self, req: PlanReq) -> Result<Plan>
  - async fn execute(&self, req: ExecReq) -> Result<ExecOutcome>
  - async fn observe(&self, req: ObserveReq) -> Result<Obs>
  - async fn cancel(&self, id: TaskId) -> Result<()>
- Tool
  - fn name() -> &'static str
  - fn input_schema() -> JsonSchema
  - fn output_schema() -> JsonSchema
  - async fn call(&self, input: Value, ctx: ToolCtx) -> Result<Value>
- Capability
  - descriptor (name, version, scopes)
  - healthcheck(), cost_estimate(), constraints()

## Runtime & Bus
- In-proc typed event bus with topics: agent.*, tool.*, hook.*, slash.*
- Backpressure and budgets per topic; bounded queues
- Scheduler: per-agent queues; priorities; cooperative cancellation; timeouts
- Idempotent task IDs; retries with jitter; dedupe of identical work items

## Hooks & Slash
- Hooks: before_route, after_route, before_execute, after_execute, error, finally; domain hooks (before_commit, node_failed, etc.)
- Slash: derive macro to map `/verb args` → typed commands; integrates with permissions and approvals

## Memory & Context
- Memory bindings: working/task/episodic/semantic with TTL, PII flags
- ContextPacks: graph narrow → vector refine → range assembly; budgets enforced

## Security & Safety
- Profiles: ZeroTrust/HiL/Full Auto; risk scoring preflight; approvals escrow
- Vault/Egress: SecretHandles only; egress allowlist/TLS pinning; signed requests
- DLP/redaction helpers for prompts and logs

## Telemetry & Budgets
- tracing spans; metrics; per-call costs/tokens/latency; budget guard (soft/hard)
- “No secrets” traces default; deep traces require approval

## Error model
- Retryable vs fatal errors; semantic categories (Quota, PolicyDenied, NotFound, Conflict, InvariantFailed)
- User-facing remediation suggestions attached to errors

## Testing & Evaluation
- Golden I/O tests (per agent/tool); schema validation; parity tests for provider agents
- Integration tests using emulators/connectors; deterministic record/replay fixtures
- E2E flows: agent pipelines (e.g., plan→worktree→PR) with approvals and budgets

## Versioning & Stability
- Stability tiers per trait/module: experimental → beta → stable (badges)
- Semver for public API; deprecation policy with migration notes

## Roadmap
- v0.1: In-proc bus, Agent/Tool/Capability traits, hooks/slash, memory/context/vault/egress/telemetry bindings, testing harness
- v0.2: External plugin boundary (FFI) design doc; remote agent process bridge (optional)
- v0.3: Distributed bus option; sandboxed worker pools; webhooks service for external hooks


## Normalized Provider Envelope (plan-only)

Frames (streaming)
- model_delta { role, text_delta, reasoning_delta?, tokens?, cost? }
- tool_call_start { call_id, name, args_schema }
- tool_call_delta { call_id, args_delta }
- tool_result { call_id, ok: bool, result_json?, error? }
- message_end { finish_reason, usage { prompt_tokens, completion_tokens, cost } }
- error { code, message, provider, retriable }

JSON/Structured outputs
- schema-bound decoding; recovery paths for non-conformant frames; emit validation_error frames and attempt repair when policy allows

Tool calling
- Parallel calls allowed; backpressure via scheduler; per-call budgets; args validated against JSON Schema before execution

## Minimal Agent/Tool example (signatures, plan-only)

```rust
pub struct ExampleTool;
impl Tool for ExampleTool {
    fn name() -> &'static str { "example.tool" }
    fn input_schema() -> JsonSchema { /* ... */ }
    fn output_schema() -> JsonSchema { /* ... */ }
    async fn call(&self, input: Value, ctx: ToolCtx) -> Result<Value> {
        // use ctx.egress, ctx.vault, ctx.telemetry
        Ok(json!({"ok": true}))
    }
}

pub struct ExampleAgent;
#[async_trait]
impl Agent for ExampleAgent {
    async fn plan(&self, req: PlanReq) -> Result<Plan> { /* ... */ }
    async fn execute(&self, req: ExecReq) -> Result<ExecOutcome> { /* route to tools, emit frames */ }
    async fn observe(&self, req: ObserveReq) -> Result<Obs> { /* traces, budgets */ }
    async fn cancel(&self, id: TaskId) -> Result<()> { Ok(()) }
}
```

## Typed Bus (direction)
- Build on tokio mpsc/broadcast with typed topics; attach span context; enforce budgets and backpressure
- Evaluate actor supervision (Kameo/Actix) for long-lived agents; keep bus generic and lightweight in v0.1



## Hook Payload Schemas (plan-only)

- workflow_designed { workflow_id, name, nodes: [NodeSpec], edges: [EdgeSpec], author, created_at }
- schema_validated { workflow_id, errors: [SchemaError], warnings: [SchemaWarning] }
- node_failed { workflow_id, node_id, attempt, error_code, error_msg, input_excerpt?, output_excerpt?, span_id }
- workflow_debug_snapshot { workflow_id, run_id, node_id?, inputs, outputs?, logs_ref?, artifacts_ref?, captured_at }
- before_commit { workspace_id, repo, branch, staged_files: [Path], cc_scope }
- after_pr_merge { repo, pr_number, merge_sha, merged_by, checks: [CheckStatus] }

## Slash Grammar & Permissions (plan-only)

- Grammar: /<verb> <args> [--flags]; args: key:value or quoted strings; supports @mentions and #refs
- Permissions: each verb has scopes; evaluated against Security profiles; high-risk verbs auto-raise approvals
- Examples
  - /worktree new name:feature-x base:main
  - /pr open title:"feat: add X" reviewers:@alice,@bob labels:#feature
  - /flow debug name:ETL-1 step:node-3 --retries 0

## Budgets & Cost Guard API

- Budget types: per-task, per-agent, per-workspace; soft_limit, hard_limit, window
- Pre-exec estimate: cost_estimate(req) → { est_tokens, est_cost, confidence }
- Guard: enforce(request) → Allow | Deny(reason) | RequireApproval
- Events: budget_soft_limit, budget_hard_limit (span-linked)

## Approvals & Profiles API

- Profiles: ZeroTrust | HiL | FullAuto; per-workspace default; per-command overrides
- Request: request_approval(action, risk_score, context) → Pending(id)
- Grant: approve(id, actor) | reject(id, reason); audit persisted; hooks fired
- Risk scoring inputs: provider, scopes, data sensitivity, diff size, tool class (shell/cloud)

## Settings & Isolation Binding

- Hierarchy: Global → Workspace → Project; merge with precedence
- Isolation: Soft (shared engines, namespaces) | Hard (per-workspace processes); air_gapped: bool
- Binding helpers: with_workspace_ctx(workspace_id) → BoundCtx { vault, egress, storage, telemetry }

## Provider Envelope JSON (example)

```json
{"type":"model_delta","role":"assistant","text_delta":"Fix applied to parser.","tokens":28}
{"type":"tool_call_start","call_id":"t1","name":"git.commit","args_schema":{"type":"object","properties":{"message":{"type":"string"}}}}
{"type":"tool_call_delta","call_id":"t1","args_delta":"{\"message\":\"feat: parser fix\"}"}
{"type":"tool_result","call_id":"t1","ok":true,"result_json":{"commit":"abc123"}}
{"type":"message_end","finish_reason":"stop","usage":{"prompt_tokens":512,"completion_tokens":128,"cost":0.0123}}
```

## VCS/Worktrees Hooks & Guards

- Resource guard: ensure_single_dev_server(worktree_id) → Reuse(existing_pid) | Start(new_pid)
- Hooks: before_commit/after_commit, before_pr_open/after_pr_merge; link spans to PR numbers and SHAs
- Cleanup: on PR merge/close → teardown_worktree(worktree_id); verify no orphan processes/dirs

## Debug & E2E Integration Points

- Debug Agent API: debug.attach(pid) → session_id; debug.snapshot(session_id) → artifacts_ref
- E2E Agent API: e2e.run(project, browsers[], shard?, retries) → run_id; artifacts_uploaded(run_id, urls[])
- Both emit OTEL spans; artifacts stored with workspace-scoped references

## Testing Matrix & Acceptance (SDK)

- Buses: throughput/latency under N agents × M events/sec; drop rate < 0.5%; p95 latency targets
- Provider parity: streaming/tool/JSON envelope across OpenAI/Anthropic/Gemini/OpenRouter/Local; golden fixtures; record/replay
- VCS: large-repo status/diff; worktree lifecycle; SCM parity tests (stage/hunks/rebase)
- Workflow: design→validate→run→debug; deterministic reproducer creation; compensation to stable state
- Security: zero raw secrets in logs; approvals enforced; egress allowlist honored; DLP redaction verified
- Observability: span linkage across agents; budget alerts timely; <1% event drop
