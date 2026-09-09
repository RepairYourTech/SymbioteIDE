# Native agent worker loop

Canonical owner: [#465](https://github.com/RepairYourTech/SymbioteIDE/issues/465) (native side), preserving the #464 AgentRuntimeAdapter-versus-InferenceProviderAdapter separation. `symbiote-native-agent` implements the model/tool turn engine driven by a started dispatch against its resolved provider context. This is the first worker-loop slice: the loop's logic, budgets, events and completion handoff are fully exercised **offline**; the live OpenAI Responses API transport and sandbox-executed tools are the named next slices and require user-granted credentials and billing authorization.

## The turn engine

`NativeSession::new(dispatch, model, at)` re-validates the dispatch contract (enforcement windows, host eligibility) at session creation — authorization is never trusted from preparation time. `run(prompt, transport)` loops:

1. Builds a fully validated `ProviderRequest` whose context budget is **the dispatch contract's** context (tightened by what the loop already consumed), never the model's full window; the contract's declared tools are the only tool definitions offered.
2. Calls the [`InferenceTransport`] trait — the exact inference-call boundary mirroring the SDK's `InferenceProviderAdapter` separation. The production implementation is the OpenAI Responses API client; `ScriptedTransport` and ad-hoc test transports provide deterministic offline envelopes.
3. Validates the response envelope through the SDK's own `ProviderResponse::validate` (schema version, tool-call coherence, finish/text consistency, duplicate call ids, declared-tool membership, usage-vs-window) — the loop is the enforcement point. Usage accounting is conservative: `Known` usage splits input/output exactly; `Unknown` usage is never free — the turn's full request budgets are charged, the turn is counted as unreported in the durable summary, and a 512 KiB accumulated-output byte backstop bounds runaway loops that report nothing. Events (`Ready` first, then `Message`, `ToolProposed`, `Diagnostic`, `Exit`, `CancelAcknowledged`) follow the SDK session tracker's ingestible order; event text truncates on char boundaries with a visible marker rather than panicking.
4. Halts on `Stop`/`Length`/`Cancelled`, on budget exhaustion **before** the provider call, on a 64-turn cap, on provider failure, or when the model proposes a tool absent from the dispatch contract — an undeclared tool is a contract violation, recorded and refused, never papered over.

## Completion is evidence

`request_completion(report)` records a `CompletionRequested` runtime event and the report. Protocol v1.12 adds `request_task_completion` (local-owner proxy; the Host maps the actor to the dispatch's `Worker` identity) which routes the report through the domain lifecycle: `Running → CompletionRequested` only. Verification (`BeginVerification`), evidence checks, gates and independent review remain Host-owned beyond that point — a worker's report can never mark a task `Completed`.

## Honest non-claims

No HTTPS transport yet: every test runs against offline envelopes; **live Responses-API verification requires user authorization for credentials and billing, and none was performed**. Tools are validated and recorded but not executed in the sandbox (next slice: sandbox invocation through #218's launcher with the #211 reserved worktree). Tool proposals are not yet echoed into the conversation (message parts are text-only), so a provider has no memory of a proposal between turns — documented tool-turn amnesia. No live process containment, no streaming, no resume-after-crash of an in-flight turn sequence. External (Codex) execution shares none of this code path and cannot bill native credentials.

## Evidence

Seven offline tests: single-turn stop within contract budget (request honors contract context, not model window; tools exactly the contract's; Ready-first event order; real task/provider identities in the summary), tool-calls continue until stop + undeclared-tool halt attributed as `unknown_tool` with a tracker-compatible Diagnostic, budget exhaustion refusing before the provider call, unreported usage charging full budgets and halting the follow-up turn, completion-report-as-evidence with double-filing and oversized-report typed refusals, expired-contract session refusal, and invalid envelopes rejected by SDK validation. A daemon test proves the completion request against an unstarted task fails closed and the task stays Ready.
