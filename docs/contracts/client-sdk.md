# Client SDK — shared connection/state foundation (#182 foundation)

`symbiote-client-sdk` gives every controller — CLI, desktop, tests — one
implementation of the client-side facts instead of each recreating
synchronization logic: request construction against the supported protocol
version, command-id idempotency, response classification, per-Project
journal cursor tracking for resume, and a bounded recovery loop. It owns no
I/O: the transport is injected as a `FrameExchange` trait, so the Host's
authenticated Unix socket, in-process test harnesses, and future remote
transports share identical state logic.

## What it owns

- **Request construction.** `build_request` pins `CURRENT_VERSION` (a
  session against any other version is a construction error), mints a
  unique correlation id, and uses the caller's `command_id` verbatim as the
  durable idempotency key. Oversized requests (64 KiB bound) are refused
  client-side before any transport is touched, after the same parse the
  daemon will apply — offline misfires never become transport noise.
- **Response classification.** Exactly four terminal outcomes exist:
  `Ok(body)` with the typed `ResponseBody`; `Refused(protocol error)` with
  the daemon's typed error preserved; `Transport` (the command's
  disposition is unknown); and `Unparseable` (including a response version
  that no longer matches — the daemon was replaced under us and the caller
  must re-handshake, never guess). A disconnect is a transport fact, never
  a task outcome: the SDK never invents worker success.
- **Journal cursor resume.** `Journal` responses advance the per-Project
  durable cursor over events actually returned; positions survive process
  restart via `with_positions`. Unknown Projects bootstrap at cursor 0 —
  the caller decides whether that means full bootstrap or tail read.
- **Bounded recovery.** `recover` resends the same command id and
  byte-identical operation after transport failures (the daemon's journal
  replays the durable receipt), surfaces refusals immediately (the daemon
  already answered authoritatively — retrying would be meaningless), and
  gives up after the caller's attempt budget with the transport error.

## Deterministic chaos coverage

The chaos test drives a fixed LCG-scheduled hostile sequence — transport
failures, unparseable frames, wrong-version responses, refusals, durable
receipts — and asserts every terminal classification is honest (no
fabricated state) and that durable successes observe the exact journal
cursor. The schedule is computed, not sampled, so the sequence is
reproducible on every run and platform.

## Honest non-claims

This foundation performs no I/O and ships no live transport: wiring it to
the Host's socket (or a future remote transport) is the embedding client's
job, and the CLI currently uses its own direct exchange. There are no
optimistic-command semantics yet (the one mutation conflict behavior that
exists — revision CAS — is surfaced as `StaleRevision` refusals, not
speculatively applied), no streaming events, no offline cache, and no
resynchronization telemetry. #182 remains open: bootstrap snapshotting,
duplicate/delayed/out-of-order event handling beyond the journal page
contract, and the deterministic protocol chaos suite against a live Host
are all still pending.
