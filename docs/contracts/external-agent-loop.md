# External agent loop — Codex App Server driver (#465 external side)

`symbiote-external-agent` drives the pinned Codex App Server (`codex-cli
0.118.0`, #483) as an external harness over versionless JSONL RPC. The
harness is untrusted execution: it never writes canonical state, and its
completion report is RequestCompletion evidence, never completion (#465,
Host verification and independent review remain the completion gates).

## Boundary and separation

- The crate preserves the #464 architecture: this driver is the external
  counterpart of the native turn engine (`symbiote-native-agent`); neither
  shares a code path with the other, and a `Dispatch` compiled for one
  runtime kind is refused by the other's driver. A native dispatch passed to
  the external driver is an explicit `NotExternalHarness` error, and vice
  versa at the native driver's contract validation.
- Credential separation is structural: the driver sends no credentials, no
  API keys and no native billing route. The harness authenticates with its
  own configured account. `account/read` (already proven by the #483
  discovery probe) is never called here, `refreshToken` is never sent, and a
  native provider credential reference is never copied into an external
  session. This implements the no-silent-fallback rule between native API
  billing and external harness execution.
- The driver performs no network requests, credential reads, canonical
  writes or tool execution of its own. It maps harness observations onto the
  SDK's normalized event stream (Ready-first, per the session-tracker
  contract) with `RuntimeKind::ExternalHarness` session bindings.

## Pinned protocol subset

The tested release is `codex-cli 0.118.0`; shapes were checked against the
installed executable's `app-server generate-json-schema` output (v2
thread/turn APIs) without experimental methods. The driver sends exactly:

- `thread/start` with the worktree path **as visible to the harness
  process** as `cwd` — through the sandboxed launcher the Host path is
  mounted at `/workspace` and invisible by its host name, so composed
  callers pass `/workspace` — `sandbox` pinned to `read-only` and
  `approvalPolicy` pinned to `never`. The thread id is minted by the
  harness; the driver only records it.
- `turn/start` with the task prompt as one text input on the recorded
  thread. Turn identity is minted by the harness.

The handshake precedes everything: `initialize` → pinned
`symbiote/0.118.0` check → `initialized`. The driver never sends
login/logout/account/config-write/fs/plugin/feedback/exec or
fuzzyFileSearch requests. Notifications are correlated by `threadId` +
`turnId`; foreign-turn or unattributable notifications are ignored with a
bounded diagnostic and never stop the turn or touch its accounting.

## Approval refusal is total

Every server-initiated escalation request the harness surfaces is refused
and recorded as a bounded Diagnostic naming only the method, plus a count in
the run summary. Known pinned approval shapes — `execCommandApproval`,
`applyPatchApproval`, `item/permissions/requestApproval`,
`item/tool/requestUserInput`, `mcpServer/elicitation/request` — receive a
shaped denial the harness understands. Methods outside the pinned approval
responses (including `item/commandExecution/requestApproval`,
`item/fileChange/requestApproval`, `item/tool/call` and anything unknown)
receive **no reply at all**: the driver does not invent a refusal body for a
shape it has not verified, it merely refuses to grant and records. The
driver never grants an approval: the dispatch contract's access snapshot is
the only permission authority, and model text cannot widen it through the
harness.

With `approvalPolicy: "never"` the pinned harness is expected not to
escalate; the refusal path exists because the driver does not trust that
policy to hold.

## Accounting and bounds

- Token usage from `thread/tokenUsage/updated` is recorded as `Measured`
  aggregate input/output with billing and cached-input explicitly
  `Unknown/NotReported` and `coverage: AggregateOnly`. A notification
  missing its numeric fields counts as an unreported turn — never free.
  Frames that do not name this run's thread are never accounted here.
- Frames are correlated by exact `threadId`/`turnId` equality before they
  can drive the run's state or accounting; unattributable frames (no ids,
  foreign thread) leave a bounded diagnostic and nothing else. Thread/turn
  ids are validated to the pinned protocol's identifier charset and 128-byte
  bound on receipt.
- One observed turn may absorb at most
  `MAX_NOTIFICATIONS_PER_CALL * MAX_TURNS_NOTIFICATION_ROUNDS` frames;
  exceeding that halts the run as `TransportLost`. Event text truncates at
  16 KiB on char boundaries with a visible marker.
- Stop states are explicit (`Completed`, `Failed`, `Interrupted`,
  `TransportLost`) and map to `Exit(0)`, `Exit(1)`, `Exit(None)`, and a
  Diagnostic + error respectively. `Exit(None)` for interruptions, not
  `CancelAcknowledged`: the driver never sends a cancellation request, so an
  acknowledgement would be rejected by the SDK session tracker. A lost
  transport never reads as a completed turn.
- `request_completion` is refused after a terminal stop: its
  `CompletionRequested` event must remain tracker-replayable, and it can
  never follow a terminal `Exit`. Every completed turn's full event stream
  replays cleanly through the SDK `SessionTracker` (asserted in tests).

## Production transport

The `process` module owns the wire: versionless request envelopes (no
`jsonrpc` field, transport-assigned numeric ids), strict frame
classification (response / notification / server request; unknown fields,
null ids and stray responses are violations, never guesses), bounded
buffering, and the per-method denial bodies from the pinned response
schemas. `launch_sandboxed` composes the Host sandbox (#218) with the
pinned binary (`/usr/bin/codex app-server --listen stdio://`): trusted
helper, consent whose fingerprint covers the exact invocation, disposable
HOME, isolated network, reserved worktree (#211). The driver completes the
real handshake (`initialize` → pinned `symbiote/0.118.0` check →
`initialized`) before any thread or turn interaction; a server that does
not report the pinned version is refused.

The `codex_thread_smoke` example is the real-binary proof, mirroring the
#483 discovery proof: sandboxed launch, the driver's own `begin_thread`
against the production framing, then cancellation — no turn, no model
call, no credentials, no network. It runs locally and in CI
(`Rust contracts`, stable toolchain). Sizing note: the shared transport's
default frame cap is 64 KiB; a real turn whose frames exceed that is
rejected (`FrameTooLarge`) and reads as a lost transport, so production
callers must size `TransportLimits` explicitly — event-text truncation
only applies after framing.

## Honest non-claims

Unit tests run against deterministic fixture transports; two tests drive
the full driver over a real framed subprocess (scripted frames, no codex
binary, no network, no credentials) to pin EOF, deadline and
silent-harness behavior. The only live-binary evidence is the smoke
proof's handshake + thread start: **no model turn ran** — with
authentication `required` and no credentials in the sandbox, a turn would
fail anyway, so execution proof still requires explicit user authorization
for credentials and billing. No tool-result round trips, no
steering/interrupt requests, no durable resume/reconnect. #464/#465 remain
open.
