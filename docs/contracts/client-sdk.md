# Client SDK — shared connection/state foundation (#182)

`symbiote-client-sdk` gives every controller — CLI, desktop, tests — one
implementation of the client-side facts instead of each recreating
synchronization logic: request construction against the supported protocol
version, command-id idempotency, response classification, snapshot bootstrap,
per-Project journal cursor tracking and resume, a deterministic reduction of
journal events into client state, and a bounded recovery loop. It owns no
I/O: the transport is injected as a `FrameExchange` trait, so the Host's
authenticated Unix socket, in-process test harnesses, and future remote
transports share identical state logic.

## What it owns

- **Request construction.** `build_request` pins `CURRENT_VERSION`
  structurally (the session has no other version), mints a unique
  correlation id, and uses the caller's `command_id` verbatim as the
  durable idempotency key. Build failures have their own identities —
  `InvalidCommandId` (charset/length), `InvalidOperation` (wrong kind,
  unknown fields, or a semantic validation the Host would apply, including a
  journal page limit outside 1–256), and `RequestTooLarge` for the 64 KiB
  bound — refused client-side before any transport is touched, after the
  same parse the daemon will apply, so offline misfires never become
  transport noise.
- **Response classification.** Terminal outcomes exist:
  `Ok(body)` with the typed `ResponseBody`; `Refused(protocol error)` with
  the daemon's typed error preserved; `Transport` (the command's
  disposition is unknown); and `Unparseable` (including a response version
  that no longer matches — the daemon was replaced under us and the caller
  must re-handshake, never guess — a page that is not an answer to the
  request it was returned for, and a snapshot naming another Project than
  the one asked about). A disconnect is a transport fact, never
  a task outcome: the SDK never invents worker success.
- **Snapshot bootstrap.** `bootstrap` asks for one Project's snapshot and
  returns the reduced `ClientState` seeded from it: the Project, its Roots,
  its Roles and its Tasks exactly as the daemon's own reads answer them, with
  nothing carried over from an earlier state. The session's durable cursor
  for that Project becomes the snapshot's own cursor — the Project's journal
  head at the moment the records were read — so the next resume continues
  from the point the records were read at instead of replaying the log. A
  refused or mismatched snapshot returns no state and leaves the cursor where
  it was.
- **Journal resume and event reduction.** `resume` reads pages after the
  durable cursor and applies them to the caller's `ClientState`. The rules
  are named and each is held by a test: only events after the cursor are
  applied, so an event at or below it is counted as a duplicate and never
  applied twice; a page that breaks the daemon's own page contract is
  refused as `Unparseable` with nothing applied, so neither the state nor the
  cursor moves on a page that was not an answer to this request — duplicate
  events are the one tolerated exception, filtered out, after which the
  remainder must obey the same contract; a page holding nothing new is
  counted as stale (an observable degraded answer, not a silent one) and
  stops the resume instead of being re-asked forever; a transport failure
  stops the resume with the durable cursor at the last applied event, so the
  next resume continues from exactly there; and a refusal is surfaced
  immediately and unapplied.
- **Resynchronization telemetry.** `ResumeOutcome` reports pages read,
  events applied, durable events this state carries no record for
  (`not_modeled`), events belonging to another Project than the state holds
  (`foreign`, counted and never mixed in), duplicates, stale pages, the final
  cursor, and `caught_up` — which is true only when the daemon reported no
  further events, never when the caller's page bound stopped the loop.
- **Bounded recovery.** `recover` resends the same command id and
  byte-identical operation after transport failures (the daemon's journal
  replays the durable receipt), surfaces refusals immediately (the daemon
  already answered authoritatively — retrying would be meaningless), and
  gives up after the caller's attempt budget with the transport error.

## What the reduced state models

`ClientState` holds the record kinds a snapshot carries — Project, Roots,
Roles, Tasks — keyed by identity, and classifies every journal event as
`Applied`, `ForeignProject` or `NotModeled`. A durable event with no record
in this state is named rather than dropped silently: the cursor still
advances over it, so it is never re-read forever, and the state never claims
a record no read or event provided. The reduction is deterministic, so
replaying a recorded log rebuilds the same state, and a bootstrap at a
cursor followed by the events after it reaches exactly the state a replay
from the beginning reaches.

## Deterministic chaos coverage

The chaos test drives a fixed xorshift64-scheduled hostile sequence —
transport failures, unparseable frames, wrong-version responses,
refusals, durable receipts (the fixed schedule is asserted to include
every class at least once) — consumes every frame, and asserts the frame
count equals the sum of honest classifications: durable successes observe
the exact last Journal cursor, refusals and unparseable frames claim
nothing, and no fabricated state exists. The schedule is computed, not
sampled, so the sequence is reproducible on every run and platform. The
snapshot and resume paths are driven the same way with scripted frames
(delayed pages, out-of-order pages, mid-resume transport failures), and one
case in the Host's daemon suite runs them against a real `symbioted` over its
socket: bootstrap, resume after further work, and a cold replay agree record
for record, across a SIGKILL and restart.

## Honest non-claims

This crate performs no I/O and ships no live transport: wiring it to
the Host's socket (or a future remote transport) is the embedding client's
job, and the CLI currently uses its own direct exchange. The reduced state
is in memory only — no offline cache or on-disk state is claimed; the
durable facts a shell persists are the journal positions, which
`with_positions` restores. There are no optimistic-command semantics yet
(the one mutation conflict behavior that exists — revision CAS — is surfaced
as `StaleRevision` refusals, not speculatively applied), no rollback, no
streaming events, and no snapshot paging: a snapshot that cannot fit the
response bound is refused rather than served in parts. #182 remains open:
bounded bootstrap for very large Projects, the remaining snapshot kinds
(work items, consents, teams, bindings, leases, preparations, elevations),
multi-client convergence and controller-disconnect rules, simultaneous
approval resolution, client-local UI state ownership, and the deterministic
protocol chaos suite against a live Host are all still pending.
