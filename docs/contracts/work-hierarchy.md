# Canonical work hierarchy

Owner #189; consumes #36/#43/#181 foundations. `symbiote-domain::WorkItem`
represents Requests, Objectives, Capabilities, Plans, Milestones, Checkpoints and
Follow-ups with distinct typed IDs. Existing Tasks and Dispatches remain their
own aggregates. Runtime-local references are bounded observational metadata;
they neither create Tasks nor change canonical state.

`WorkSpec` carries Project and Role ownership, title/description, request
utterance or objective classification, requirements, constraints, risks,
acceptance criteria, priority, optional token/time budgets, parent, dependency
references and external references. A natural-language request can be recorded
before decomposition. An approval classification in request text is metadata;
it does not execute an approval command.

## Lifecycle and authority

The lifecycle distinguishes Draft, Clarifying, AwaitingApproval, Approved,
Running, CompletionRequested, Completed and Cancelled. Revisioned commands
preserve actor/time provenance and exact retries. Revising scope invalidates
approval; reopening returns to Draft and retains history. Illegal transitions,
stale revisions and command-ID reuse for different intent fail atomically.

Workers cannot mutate canonical WorkItems. Host protocol v1.5 exposes creation,
reading and explicit user edits, but no completion operation and no caller-set
actor, creation time or history. The Host derives identity from the authenticated
principal and reuses persisted command timestamps on retries. Older protocol
versions are rejected explicitly.

Pure completion checks require current-revision acceptance evidence, all
constitutional gates and an independent reviewer distinct from the owning Role.
Evidence references must come from an authenticated Host verifier; constructing
Rust evidence structs is not authentication. Closing child Tasks never closes a
Capability. Live verifier integration and full Capability Closure remain pending.
The Running workflow state does not launch an agent.

## Relationships, privacy and bounds

Storage validates referenced records, Project/Role lineage, parent types and
cycles transactionally over the current graph, including cross-Project links.
The Host requires `ManageWork` on the changed Project and read access to linked
Projects. Work reads and journal output check historical references too, so
removing a reference does not bypass a later permission revocation. These reads
fail closed if linked access is unavailable; they do not silently redact history.

Work specifications are bounded to 32 KiB, aggregates to 256 KiB, histories to
128 commands and graph validation to 4,096 items with bounded links. Capacity
errors are explicit. Durable paged aggregate history and larger graph operation
are future work; no history is silently discarded. Server responses retain the
existing 1 MiB bound, and clients can reduce journal page size when necessary.

## Task dependency graph

Tasks carry typed dependency edges as separate first-class records (not Task
fields), owned by one Task and replaced as a whole set per task. Eight kinds
exist: `requires`, `consumes_contract_from`, `blocks`, `reviews`, `verifies`,
`supersedes`, `conflicts_with` and `follow_up_to`. Direction: every kind reads
"this task waits on the target" except `blocks`, which is stored on the
blocking task and read as an incoming dependency of the blocked target.

Completion gating is enforced by the store for exactly the blocking kinds —
`requires`, `consumes_contract_from` and incoming `blocks` — and only when
every such target Task is `Completed`; a blocked completion returns a typed
conflict and the journal is untouched. The remaining kinds are recorded with
provenance but not yet enforced, because their policy depends on review/merge
and supersession states that do not exist yet; this is a documented gap, not a
silent one.

Graph validation is global (one graph across Projects, bounded to 16,384
tasks) and rejects self-edges, dangling targets and cycles through Kahn's
algorithm at every write and during journal replay. Per-task edge sets are
bounded to 64. Protocol v1.7 adds `set_task_dependencies` (`ManageWork` on the
owning Project plus `Read` on every target Project), `get_task_dependencies`
(read) and the `task_dependencies_set` journal payload. SQLite schema v7 adds
the `task_dependencies` table whose indexed columns are tamper-evident
projections of the edge bodies.The pre-dispatch `Queued`, `Assigned` and `Blocked` Task states now have Host-only commands, exact Role binding and recorded block/unblock guards. The startable pre-dispatch pair is exactly `Ready` or `Assigned`: dispatch compilation, the domain `Start` transition, the [dispatch preparation](dispatch-preparation.md) and the [scheduling projection](scheduling-leases.md) all accept that one pair, and both refuse a `Queued` or `Blocked` Task by the same `not_schedulable` name, so an assigned Task is startable end to end and a Task the Host has not presented for scheduling is no candidate anywhere. The rest of the #94 state machine (review/merge, waiting, pause, remediation and supersession states) remains pending. Durable lease and
heartbeat/stale-worker integration remains owned by the scheduler work. The
scheduler's dependency/stream projection still decides readiness after an
explicit unblock.

## DAG answers: progress, closure, blockers, critical path, overlap

`get_task_graph(project_id)` is the rest of the DAG surface. Readiness — which
Task may start now, and the `stream_unsafe` / `stream_leased` /
`dependency_unresolved` refusals — stays with the [scheduling
projection](scheduling-leases.md); this read does not restate it. It answers,
for one Project, from recorded task rows and dependency edges only:

* **Progress** — a count per canonical Task state, in lifecycle order, plus the
  whole `total`, the `considered` count, the `gates` read, and how many Tasks
  are `closed` (`Completed` or `Cancelled`) against `open`. There is no
  percentage field and no way to express one: a caller that wants a fraction
  divides these counts itself, from canonical state, rather than reporting a
  number an agent or a client handed it. `Failed` and `Interrupted` are not
  closed, because the Host recovers them to `Ready`.
* **Remaining closure** — every considered Task that is not closed, in canonical
  id order.
* **Blockers** — per open Task, the gates that still hold it: the edge kind, the
  canonical task on the other side (in any Project, named with that Project), and
  the state that task is in now. A gate is satisfied by completion alone, so a
  cancelled prerequisite keeps its dependent on this list rather than reading as
  delivered work. The five kinds the store does not enforce yet (`reviews`,
  `verifies`, `supersedes`, `conflicts_with`, `follow_up_to`) are deliberately
  absent: reporting them as blockers would enforce a policy that has not been
  written. `blocks` appears as an incoming gate, which is where it is stored.
* **Critical path** — the longest recorded chain of gating edges into the
  Project's open work, with `length` in Tasks (no effort or duration is recorded,
  so this is not a duration), how many of its members are open, and the  chain itself capped at 64 members with `truncated` saying whether it is the
  whole chain. Ties are resolved by canonical id order, so the same state always
  names the same chain. A Task with no recorded gate is a chain of one.
* **Overlap** — startable Tasks (`Ready` or `Assigned`) that share one Change
  Stream, because same-stream work is serialized by policy. A held lease is the
  dynamic half of that fact and stays the projection's `stream_leased` refusal.
  Every Change Stream currently holds exactly one Task, so this list is empty
  against today's canonical state: it is computed, not unimplemented, and the
  multi-task stream slice is what gives it content.

Cycles are rejected at every write, and a read that finds one in the rows
refuses by name (`cycle`) rather than walking it.
DAG answers are bounded to 256 tasks and 2,048 gates: the store takes the
Project's Tasks in canonical id order until either bound would be exceeded, so
the answer is exact over what it read and `progress.total` keeps the whole count
beside `progress.considered` — a partial answer says so instead of presenting a
whole one it did not read. A Project the store does not hold is `not_found`
rather than an empty graph. The read is a Project read (Project `Read` on the
subject, and on every Project the answer names) and it writes nothing: no
journal event, no derived table, no cache to drift from the state it describes.

## Foreign runtime session and task references

A runtime keeps its own todo lists: its own session identifiers, its own task or
subtask identifiers inside them, and its own status words. Those are recorded
against a canonical Task so a reader can follow the work a dispatch actually
did — as a bounded set of observations, never as Project truth. Each link names
the harness whose identifier it is, whether it names that harness's session or
its task, the identifier verbatim, the foreign session it sits in when the
harness reports one, the status that harness reported, and when the Host
recorded the observation. The request names no time: the claim a caller sends
carries the identifiers and the foreign status only, and the Host stamps the
instant it records the set, so no client can say when it saw a foreign claim.

The references grant nothing. A link never creates a Task, never advances or
completes one, never satisfies a completion gate and never becomes a dependency
edge: `complete` is that harness's word, and the only canonical completion is
the `TaskState::Completed` that verified Host evidence produces. A link cannot
be smuggled into canonical state through the aggregate either, because the Task
record carries no link field at all — the link set is a separate first-class
record, exactly as a dependency edge is. Writing one is a Project edit
(`ManageWork` on the owning Project) and reading one is a Project read, through
`set_task_foreign_links` and `get_task_foreign_links` (protocol v1.26). A
foreign system is not a principal: a harness's report reaches this record only
through an authenticated caller, and every recorded link is stamped with the
authority's own instant whether or not the caller sent one. Recording links does
not classify a Task either — the origin requirement below still applies to
every new Task.

Refusals are named and whole. A reference to a system that is not a harness, a
blank or control-character identifier, and a session link that names another
session are `InvalidForeignReference`; an identifier over 256 bytes is
`ResourceLimit`, as is a set over 64 links; one item named twice with two
observations is `ForeignObservationConflict`; a Task the Project does not hold
is `NotFound`; a row or journal event that disagrees with the recorded set is
an integrity failure. Nothing is trimmed, truncated or repaired, and a refused
set writes no rows and no journal event. Store schema v13 adds the
`task_foreign_links` table, whose indexed columns are a tamper-evident
projection of each link's body; older databases migrate by that statement
alone, preserving every existing record and journal event.

## Task traceability and migration

Every new stored Task requires a `TaskOrigin`: an existing Capability or an
explicitly Maintenance/Operational Objective in the same Project. Storage
validates the target; a runtime-local todo cannot supply that authority.

Store schema v3 adds work records and immutable Task-origin records. v1/v2
upgrades preserve existing events and materializations, with integrity checks
inside the migration transaction. Existing Tasks without origins remain visibly
unclassified; `Start` is blocked until an explicit, journaled origin assignment.
Migration does not invent maintenance work or retroactively rewrite provenance.
Existing-origin reassignment is rejected.

Back up the stopped Host database before changing application versions. Older
binaries reject schema v3; rollback uses a compatible binary or a verified backup,
not an in-place destructive downgrade. Corrupt input fails migration without
committing the schema change.

## Verification and remaining scope

For a working demonstration, start the Host as described in [Host setup](host.md)
with a fresh private state directory, then send `fixtures/work-hierarchy/project.json`,
`request.json`, `capability.json` and `plan.json` in that order. These files are
full request envelopes, and the daemon integration suite sends them exactly as
they are (`crates/symbiote-host/tests/daemon.rs`); the CLI's `raw` command takes
an *operation object* rather than an envelope, which is why the foreign-link
fixtures below are shaped that way instead. `read.json` returns the
Capability, its Request parent reference, acceptance contract and server-derived creator.
`shutdown.json` stops this demo Host. Restart with the same state directory;
`read.json` still returns the object and repeating `capability.json` returns the
original receipt with `replayed: true`. The initial local demonstration created
journal sequences 1–4 and returned the Capability in Draft; this is durable
intake/planning evidence, not completed coding work.

The foreign runtime link operations are `raw` operations, like the dependency
ones, because the typed CLI surface is the work hierarchy's own intake. For a
Task already created,

```sh
target/debug/symbiote --state-dir /absolute/private/state-directory raw fixtures/foreign-task-links/set.json
```

records one session link and one task link under that Task's Project, and the
same command with `fixtures/foreign-task-links/read.json` returns them exactly
as recorded — including the foreign `complete` status, which the Task's own
state does not inherit: `get-task` for the same Task still answers `ready` at
revision 0. The CLI mints a fresh command id per invocation, so a second
`set` is a supersession the journal keeps rather than a replay; the idempotent
retry belongs to a caller that reuses one command id, and the daemon suite
drives that over a real socket.

Tests cover evolving requests, stale revisions, approval invalidation,
cancel/reopen, completion evidence, cycles and cross-Project reads; SQLite tests
cover legacy migration, rollback, replay/tampering, immutable origins and
concurrency. The foreign reference cases cover every named refusal, the bounds,
persistence across a reopen, the journal audit in both directions, and the
property the whole slice exists for: a foreign `complete` leaves canonical Task
state, the completion gate and the dependency graph exactly as they were. Actual daemon tests exercise restart/retry and reject wire authority
and completion claims. Run:

```sh
cargo test -p symbiote-domain -p symbiote-store -p symbiote-protocol -p symbiote-host --locked
```

Independent review separates domain, storage and Host/protocol ownership. Broad
#189 remains open for runtime-local todo promotion workflows, richer planning
and dependency scheduling, voice/mobile intake clients and full cross-system
acceptance. No native/Codex execution, desktop workbench or first release is
claimed by this metadata integration. Cross-platform pure contracts are portable;
the current Host transport and CI remain Linux-specific.
