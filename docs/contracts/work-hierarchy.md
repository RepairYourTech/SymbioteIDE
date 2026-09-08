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

Workers cannot mutate canonical WorkItems. Host protocol v1.3 exposes creation,
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

For a working CLI demonstration, start the Host as described in [Host setup](host.md)
with a fresh private state directory, then submit `fixtures/work-hierarchy/project.json`,
`request.json`, `capability.json` and `plan.json` in that order:

```sh
target/debug/symbiote --state-dir /absolute/private/state-directory request < fixtures/work-hierarchy/project.json
```

Use the same command with each fixture filename. `read.json` returns the
Capability, its Request parent reference, acceptance contract and server-derived creator.
`shutdown.json` stops this demo Host. Restart with the same state directory;
`read.json` still returns the object and repeating `capability.json` returns the
original receipt with `replayed: true`. The initial local demonstration created
journal sequences 1–4 and returned the Capability in Draft; this is durable
intake/planning evidence, not completed coding work.

Tests cover evolving requests, stale revisions, approval invalidation,
cancel/reopen, completion evidence, cycles and cross-Project reads; SQLite tests
cover legacy migration, rollback, replay/tampering, immutable origins and
concurrency. Actual daemon tests exercise restart/retry and reject wire authority
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
