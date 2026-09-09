# Worker completion wiring — Host-side runner (#465/#205 integration slice)

`symbiote-host::runner` is the composition point where a started dispatch is
executed by one of the two worker loops and where the loop's completion
report is filed as canonical evidence. It closes the "completion wiring"
pending item that `dispatch-preparation.md` tracked for both runtimes.

## What it does

- **Runtime selection is the contract's, not the caller's.** `run_native`
  refuses a dispatch whose profile is not `NativeSymbiote`;
  `run_external` refuses one that is not `ExternalHarness`. There is no
  fallback loop: native work never uses the harness, external work never
  uses the native inference path, and neither loop can be pointed at the
  other's billing route. (The native model envelope is read from the #464
  provider registry keyed by the contract's profile model, never from loop
  input.)
- **Preconditions are re-checked, not trusted.** The task must be `Running`
  under this exact dispatch (per the store's journaled state) before the
  loop runs; the loop session re-validates the dispatch contract at run
  time, refusing expired enforcement windows or foreign identities.
- **Completion is evidence.** On a genuinely finished turn — native: the
  loop's halt reason is `Stop` AND the stream's terminal event is `Exit(0)`
  with no `CancelAcknowledged` (the native loop maps a provider
  `Cancelled` finish to the same `HaltReason::Stop` as a normal finish, so
  the runner checks the event, not just the reason); external:
  `StopKind::Completed` — the final agent message is filed through the
  store's journaled `apply_task` as `Actor::Worker(dispatch)` with
  `TaskAction::RequestCompletion` — the same transition the protocol's
  `request_task_completion` uses. The domain law holds structurally: a task
  can only reach `CompletionRequested` this way. Host verification and
  independent review remain the completion gates; the runner has no code
  path to `Complete`, `BeginVerification`, or any other transition.
- **The filing command id is per-run** (`worker-completion-{dispatch}-{at}`):
  a Host retry of the same observed run replays byte-identically, but a new
  run's filing can never be silently swallowed by an older receipt.
- **Failure files nothing, with named causes.** A halted, failed,
  interrupted, or transport-lost loop leaves the task `Running` and returns
  `RunnerError::LoopFailed(reason)` carrying the loop's own halt/error
  identity — caller-sequencing bugs (`invalid_input`, `already_complete`)
  are never misread as retryable transient failures
  (`provider_failed`). Store refusals return `RunnerError::Store(cause)`
  with the store's error preserved, so an idempotency collision is
  distinguishable from a state move. A finished turn with no reportable
  message returns `NoReport` rather than filing an empty report the domain
  would reject. Retry policy is the Host's, decided outside the loop.

## Verification

Tests run against an in-process `Store::memory()` with the full composition
fixture (project, Team, provider registry entry, model descriptor, workforce
binding, classified Objective origin, task, route, preparation, start) —
the same composition the store's own preparation tests prove — with scripted
native and external transports. Covered: both runtimes file evidence and the
task reaches `CompletionRequested` (never `Completed`); foreign-runtime
refusals leave the task `Running`; a failed loop files nothing; a task no
longer `Running` is refused before any loop runs.

## Honest non-claims

The transports are deterministic fixtures; no live OpenAI Responses API
call, no live Codex turn, no credential read, no spending. The live native
transport and the live external turn each require explicit user
authorization for credentials and billing. The runner is a library
composition point; the daemon does not yet expose a worker-activation
endpoint and nothing schedules runs automatically. Durable session
resume/reconnect, tool execution, and multi-turn tool-result round trips
remain pending (#465/#464 stay open).
