# Normalized runtime events and bounded session observation

Owner: [#184](https://github.com/RepairYourTech/SymbioteIDE/issues/184). `symbiote_runtime_sdk::events` implements a provider-neutral observation boundary and deterministic in-memory session tracker. It uses domain Session, Dispatch and Host identities and `RuntimeKind`; it imports no canonical Task mutator. A worker completion request remains an observation, and a zero exit code produces only an `Exited` session state. No event or tracker state means canonical work is complete.

## Wire contract

`SessionBinding` fixes the Session ID, Dispatch ID, Host ID and native/external runtime kind. `RuntimeEvent::new` binds a validated domain `CommandId`, positive per-session sequence and normalized payload to that immutable ownership tuple. Tool calls, approval requests, questions and optional vendor references use validated domain `RequestId` values. The tracker rejects any ownership mismatch, even on a duplicate event. Event fields have no setters; deserialization validates the positive sequence and bounded text before constructing the envelope.

The normalized payload variants cover ready, message, tool proposed/started/completed/failed, approval request, worker question, filesystem/process/Git/network/MCP/secret-access/post-edit observation, diagnostic, usage, completion request, exit, disconnect, reconnect, crash, cancellation request and cancellation acknowledgment. Observations do not themselves enforce filesystem, tool or credential restrictions. Structured adapters must emit only events they can actually observe; a generic terminal adapter must not fabricate structured tool lifecycles from guessed semantics.

`EventText` bounds each message, argument, result, error and diagnostic to 16,384 UTF-8 **bytes**. The generated JSON Schema advertises a maximum character length, with a description of the stricter runtime byte check. Unicode and JSON escaping mean the transport must impose its own frame-byte limit before deserializing; this type does not bound allocation while an unbounded transport is decoding a string. There are no unbounded vectors or raw vendor payloads in an event. `VendorEventReference` retains a bounded, nonempty namespace and typed reference to separately governed vendor evidence. Callers must redact message/tool content and manage raw evidence access; this module does not claim automatic secret detection or redaction.

Usage has explicit `Measured { value }` and `Unknown { reason }` representations for input/output/cache tokens and billed micro-units. Missing fields do not default to zero. Coverage separately identifies root-only, root-and-children, aggregate-only or unknown reporting. These values are reported observations, not verified costs, token accounting, entitlement decisions or complete child visibility. Currency/unit interpretation and quota/rate-limit interfaces remain outside this bounded event implementation.

Known cached input cannot exceed known input, and a known input/output sum must fit `u64`. The same validator runs for direct event construction and wire deserialization. Unknown values remain unknown; a measured cached value is not rejected merely because total input is unreported.

## Lifecycle and replay

Create a `SessionTracker` from a binding and `TrackerLimits`, then call `apply(event)`. Its `EventReceipt` contains the original event sequence, resulting session state, replay marker and residual-effect assessment. A retained exact duplicate returns its original result with `replayed=true`; changed content under the same ID conflicts. A reused sequence conflicts. A missing sequence reports the expected/received gap without advancing the cursor. Event receipts are not durable commits or external-effect acknowledgments.

The initial state awaits a ready event. A reconnect restores pre-disconnect readiness rather than inventing a completed handshake. Disconnect and crash make residual effects unknown. Tool events follow proposed → started → completed/failed; unknown, reused or invalidly ordered tool IDs are rejected. Approval/question events remain requests for another subsystem to resolve. A completion report leaves the session running. Exit/crash end runtime observation without touching Task or Capability state; trailing usage and diagnostics can still be retained.

Cancellation intent survives disconnect/reconnect. After cancellation is requested, new tool or mutation observations are rejected with `ResidualEffectUnknown`, leave the sequence unadvanced and keep the residual-effect assessment unknown. Acknowledgment and exit do not certify cleanup or undo earlier effects. Rejected events cannot be silently skipped to close a sequence gap: the producer/Host must reconcile its authoritative stream or terminate observation visibly. `Unassessed` is the default residual-effect value, never a claim that no effects occurred.

## Resource and recovery limits

Defaults retain 128 replay entries, at most 256 distinct tool calls and at most 4,096 event identities. Configured hard maxima are 1,024 retained entries, 256 tool identities and 65,536 event identities. Completed tool IDs remain counted so they cannot silently be reused. A bounded deque evicts old payloads, while the bounded identity set prevents an old event ID being recycled at a newer sequence. Exhausting a limit fails closed; it never silently drops an event and reports success.

`replay_floor()` exposes the oldest retained sequence. Events below it return `ReplayUnavailable`; expired history is not guessed or treated as a successful duplicate. A session reaching an identity/resource bound needs an authoritative continuation/checkpoint strategy owned by later integration work. Runtime cancellation/control must remain available independently of telemetry capacity; the tracker is not the mechanism that kills a process or sends cancellation to a harness.

The tracker implements `Serialize` for inspection but deliberately does **not** implement `Deserialize`. An arbitrary serialized snapshot cannot restore trusted history or ownership. Durable replay/checkpoint integration, authenticity and cross-process recovery are not supplied here. Accepted envelopes can be serialized/deserialized, then applied in order to a fresh tracker within its limits.

## Verification and pending acceptance

Run `cargo test -p symbiote-runtime-sdk --test events` and `cargo clippy -p symbiote-runtime-sdk --test events -- -D warnings`. Twelve event tests cover:

- The complete observed tool lifecycle, advisory completion report and false-green exit-zero boundary.
- Exact duplicates, changed IDs/content, reused sequences and gaps without cursor advancement.
- Cross-Session/Dispatch/Host/runtime rejection.
- Cancellation, residual mutation reports and cleanup uncertainty.
- Unknown versus measured-zero usage and explicit child visibility.
- Invalid measured token subsets and overflowing totals rejected at construction/deserialization without tracker advancement.
- Disconnect/reconnect before readiness and while cancellation is pending.
- Unknown, failed, reused and out-of-order tools.
- Replay-floor eviction, identity reuse detection and bounded-capacity failure.
- UTF-8 byte bounds, invalid wire fields and schema/reference round trips.

These are SDK contract tests, not native-agent, Codex, PTY, process cleanup or production adapter evidence. #184 remains open for real adapter integration, full lifecycle methods and capability conformance, qualified tool enforcement, authenticated/redacted raw-event handling, continuation/checkpoint persistence, rate-limit/quota reporting, advanced child lineage/cost coverage and platform-specific compatibility dossiers. No canonical Task mutation or actual runtime execution is enabled by this module.
