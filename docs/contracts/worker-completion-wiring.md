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
longer `Running` is refused before any loop runs. Tool execution is wired
end to end: a scripted transport proposing the declared `shell` tool runs
through an attached executor (`run_native_boxed(..., Some(ToolExecution))`),
the result enters the conversation, and completion still files; a consent
refusal (`ToolExecError::Refused`) feeds back as ToolFailed and the turn
still completes; a shell-executor factory that refuses to build is the
typed `RunnerError::ShellExecutorBuild` before any loop runs.

## Honest non-claims

The transports are deterministic fixtures; no live OpenAI Responses API
call, no live Codex turn, no credential read, no spending. The live native
transport and the live external turn each require explicit user
authorization for credentials and billing. The daemon wires the activation
endpoint (`run_started_dispatch`): it provisions the worktree (#211), builds
the transport from the operator's factory, and attaches the shell executor
when one is configured. The REAL sandboxed shell tool path (bubblewrap +
trusted launcher + exact-command consent) is proven at the executor layer in
host-crate tests; the daemon-level path remains scripted because a live
model turn is still gated on user authorization for credentials and billing.
Nothing schedules runs automatically. Durable session resume/reconnect and
multi-turn tool-result round trips remain pending (#465/#464 stay open).

## A worker's report as a first-class command

The same transition the runner files (`TaskAction::RequestCompletion` under
`Actor::Worker(dispatch)`) is reachable as the CLI command
`request-task-completion <task_id> <dispatch_id> <report>` — operation
`request_task_completion` (protocol v1.12), classified `Risk::Mutation`.
It is the worker-side proxy: the Host constructs the actor from the caller's
dispatch id and the domain binds that actor to the task's actual current
dispatch, so a report never bypasses verification, independent review, or
any Host gate past `CompletionRequested`.

Three properties are pinned on the wire (daemon test
`a_workers_report_replays_is_named_by_state_and_never_completes`):

- **A lost response replays.** The arm stamps its own clock and reads
  `expected_revision` off the live record, so it cannot rebuild the bytes a
  first attempt journaled. The store's `journaled_task_command` hands the
  recorded command back and the arm re-applies it byte-identically: the same
  command id answers `replayed: true` on the original receipt's sequence, a
  different report under a used id is `idempotency_conflict`, and a second
  filing under a new id is refused by the state the first attempt moved —
  by name. `cancel-task`, `interrupt-task`, `begin-task-verification`,
  `recover-task`, `block-task` and `unblock-task` share the same
  journaled-replay answer, because they share the same rebuilt-bytes shape.
- **A state refusal names the state and the move that answers it.** Ready
  and its pre-dispatch family point at `start-prepared-task`;
  `completion_requested` points at `begin-task-verification` with the
  report already on the record; `failed`/`interrupted` point at
  `recover-task`; a Running task answers on identity (a foreign dispatch
  id never speaks for the task); a closed task says so. A report that says
  nothing is refused as the shape it is.
- **The report is evidence, never completion.** The task reads
  `completion_requested` after the report and `verifying` only through the
  Host's own `begin-task-verification`.

## A lost activation response

A disconnected client does not roll back a committed command, so the
daemon's contract is that retrying its command ID recovers the durable
receipt. For `run_started_dispatch` the durable evidence of the run is the
task itself: a task at or past `CompletionRequested` under the requested
dispatch had its run complete (a run that fails leaves the task `Running`
and returns an error). So a replayed activation of that dispatch answers
with the recorded outcome — the same `worker_run` body with
`completed: true` — instead of re-executing the run or refusing it. A
foreign dispatch id is still refused, and any other state still refuses:
the replay path reads only a state this very dispatch's completed run could
have produced.
