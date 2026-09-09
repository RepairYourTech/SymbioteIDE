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

- `thread/start` with the Host-authorized worktree as `cwd`, `sandbox`
  pinned to `read-only` and `approvalPolicy` pinned to `never`. The thread
  id is minted by the harness; the driver only records it.
- `turn/start` with the task prompt as one text input on the recorded
  thread. Turn identity is minted by the harness.

It never sends login/logout/account/config-write/fs/plugin/feedback/exec or
fuzzyFileSearch requests. Notifications are correlated by `threadId` +
`turnId`; foreign-turn notifications are absorbed but never stop the turn.

## Approval refusal is total

Every server-initiated escalation request the harness surfaces —
`execCommandApproval`, `applyPatchApproval`,
`item/permissions/requestApproval`, `item/commandExecution/requestApproval`,
`item/fileChange/requestApproval`, `item/tool/requestUserInput`,
`mcpServer/elicitation/request`, `item/tool/call`, and any unknown method —
is refused and recorded as a bounded Diagnostic naming only the method, plus
a count in the run summary. The driver never grants an approval: the
dispatch contract's access snapshot is the only permission authority, and
model text cannot widen it through the harness. Refusal answers follow the
pinned response shapes (`decline` decisions, decline/cancel elicitation
actions, empty answer sets); the transport owns the exact reply encoding.

With `approvalPolicy: "never"` the pinned harness is expected not to
escalate; the refusal path exists because the driver does not trust that
policy to hold.

## Accounting and bounds

- Token usage from `thread/tokenUsage/updated` is recorded as `Measured`
  aggregate input/output with billing and cached-input explicitly
  `Unknown/NotReported` and `coverage: AggregateOnly`. A notification
  missing its numeric fields counts as an unreported turn — never free.
- Notification budgets are bounded (`MAX_NOTIFICATIONS_PER_CALL`, ×16 per
  observed turn); exceeding them halts the run as `TransportLost`. Event
  text truncates at 16 KiB on char boundaries with a visible marker.
- Stop states are explicit (`Completed`, `Failed`, `Interrupted`,
  `TransportLost`) and map to `Exit(0)`, `Exit(1)`,
  `CancelAcknowledged`, and a Diagnostic + error respectively. A lost
  transport never reads as a completed turn.

## Honest non-claims

This slice is offline-only: the 14 tests run against deterministic fixture
transports and prove the driver's state machine, refusal behavior, event
mapping and bounds. No live `codex` process was launched by these tests, no
real App Server session was created, no model turn ran, no credential was
read and no spending occurred. The concrete production transport (sandboxed
`JsonlTransport` spawn of the pinned binary, #211/#218) is not implemented
here; the discovery example (#483) remains the only real-binary proof, and
it made no model turn. Durable session resume/reconnect (the SDK's
Disconnected/Reconnected events), steering, interruption requests and
multi-turn tool-result round trips are not implemented. #464/#465 remain
open.
