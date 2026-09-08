# Transactional control-plane adapter

The current database schema is v8. [Work hierarchy](work-hierarchy.md) documents
work records, Task origins and transactional v1/v2 upgrades; [Project Team](project-team.md) documents v4 team state; [workforce bindings](workforce-bindings.md) documents v5 binding state; [Role routing](role-resolution.md) documents the v6 `work_routes` table and `work_routed` journal events; [work hierarchy](work-hierarchy.md) documents the v7 `task_dependencies` table, its journal replay audit and the completion gate; [scheduling and leases](scheduling-leases.md) documents the v8 `task_leases` table with fencing tokens. Legacy unclassified Tasks remain readable but cannot start until explicitly classified. The original API and schema notes below describe the earlier foundation where not superseded here.

Owner: [#43](https://github.com/RepairYourTech/SymbioteIDE/issues/43). `symbiote-store` is a bounded internal SQLite adapter for canonical Project/Root/Role registration, initial Task/Change Stream creation, trusted task transitions and event replay. It consumes `symbiote-domain` records and transitions rather than introducing a second ontology. It does not complete #43 or select the System Graph database.

The adapter pins `rusqlite = 0.40.2` with its bundled SQLite feature; the workspace lockfile pins `libsqlite3-sys = 0.38.2`, whose bundled SQLite is 3.53.2. `Store::sqlite_version()` exposes the linked runtime version. The initial 3.50.2 bundle was rejected because SQLite documents a [WAL-reset corruption bug](https://www.sqlite.org/wal.html#walresetbug) fixed in 3.51.3 and selected backports. Initialization requires SQLite 3.51.3 or newer. The package declares Rust 1.85; local tests used the installed stable toolchain, while the workspace CI MSRV job must verify 1.85 compatibility. API reference: [rusqlite 0.40.2](https://docs.rs/rusqlite/0.40.2/rusqlite/). SQLite transaction durability still depends on its documented filesystem/locking assumptions; a process-kill test is not a power-loss test. See [SQLite atomic commit](https://www.sqlite.org/atomiccommit.html) and [WAL](https://www.sqlite.org/wal.html).

## Service boundary and supported operations

`Store::open(path)` and `Store::memory()` provide disk-backed and temporary modes. The production caller must supply a trusted private directory and establish actor authorization before invoking mutation APIs. The adapter refuses an existing non-regular database path, including a symlink. This check is not a substitute for directory ownership and isolation: the parent directory must not be concurrently mutable by an untrusted process.

| API | Behavior |
| --- | --- |
| `register_project(command_id, project, roots, roles)` | Atomically registers an active revision-zero Project, its exact nonempty Root set and Role set including its Lead; rejects duplicate IDs, foreign ownership and noninitial revisions. |
| `registration_timestamp(command_id)` | Returns the original registration time so the trusted Host can reconstruct an identical draft retry without generating a different timestamp; another command kind conflicts. |
| `create_task(command_id, task, stream)` | Atomically stores a Ready revision-zero Task and Active revision-zero stream with exactly that Task; Project/Root/Role/stream lineage must agree. Worktree IDs and per-Root branch names cannot be reused. |
| `apply_task(task_id, command)` | Loads canonical current state inside a writer transaction, invokes `Task::apply`, updates the indexed snapshot and appends its journal record in the same commit. |
| `project(id)`, `task(id)` | Return typed canonical records; malformed snapshots fail rather than becoming defaults. |
| `events(project_id, after, limit)` | Returns only that Project's events, exclusively after a global cursor, with a 1–256 record limit, `next_cursor` and `has_more`. Future global cursors fail. |
| `record_route(command_id, decision, actor, at)` | Persists the latest deterministic routing decision for a canonical work item; the journal keeps the full history. Requires the work item and, for resolutions, a current Team member Role. |
| `get_route(project, work_id)` | Returns the latest stored decision; malformed snapshots fail rather than becoming defaults. |
| `set_task_dependencies(command_id, project, task, edges, actor, at)` | Replaces one Task's typed dependency edge set and re-validates the whole global DAG in the same transaction; cycles, dangling targets and self-edges fail closed. |
| `dependencies_satisfied_for_completion(project, task)` | Typed completion gate over blocking dependency kinds; `apply_task` enforces it transactionally. |
| `integrity_check()` | Checks SQLite integrity and foreign keys, reconstructs supported state from the journal in one read snapshot, and compares it with indexed current records. |

No method accepts arbitrary SQL or generic JSON mutations. `TaskCommand` is an **internal trusted service value**, not an authenticated wire command. Domain Host/Worker identity checks still require an authenticated caller upstream. A completion command's supplied stream must equal the stored stream; a caller cannot invent a newly validated stream snapshot. There is no stream mutation API in this slice, so full durable completion is not yet enabled by this adapter. Multi-task stream membership, stream integration, qualifications, authenticated verifier provenance and other entity lifecycles remain pending.

## Atomicity, concurrency and replay

Writer transactions use SQLite `BEGIN IMMEDIATE`, foreign-key constraints, a five-second busy timeout, WAL and `synchronous=FULL`. Reads/reconnects can coexist with the writer. Two connections racing against the same Task revision cannot both commit incompatible outcomes: the losing domain operation returns a revision conflict. SQL constraints remain a backstop after typed relationship validation. A failed current-state write, journal insertion or commit returns an error and rolls back the whole transaction.

Each consequential operation stores one immutable journal row with Project ID, globally unique command ID, resulting revision, exact serialized request and a typed event payload. Repeating the same command and payload returns its original receipt with `replayed=true`; changing the payload or operation under the same command ID conflicts. This remains true after subsequent transitions or reconnects. Vectors preserve their serialized order, so reordering registration inputs is a different exact payload. The Host must preserve the original normalized draft, actor and creation time for retries.

Global journal sequence numbers begin at one and are positive and contiguous; no pruning operation exists. A Project's visible sequence may contain gaps caused by events in other Projects. Replay validates its cursor against the current **global** journal head in a read snapshot, but never returns another Project's payload. Update/delete triggers make journal rows append-only through this adapter's schema. On reopen, semantic audit reconstructs Project, Root, Role, Task and stream state from typed events, validates Task transition replay, and rejects mismatch with current snapshots or their indexed identities/relationships. Unknown/damaged state is not silently repaired.

This journal is an audit/query boundary, **not an external-effect executor**. Reading/replaying events never launches tools, creates child processes, sends issues or records billing charges. Transactional outbox/inbox effects, effect acknowledgments, budget reservations and conservative reconciliation of unknown external outcomes are unimplemented. No exactly-once external-effect claim is made. Runtime/provider/entitlement snapshots already carried by a domain Dispatch persist when a trusted Task Start commits, but qualification/authentication and actual execution are not supplied here.

## Migration, recovery and security limits

The v1 migration creates the original tables and journal. Schema v2 adds durable resource consents and their journal reconstruction. Fresh initialization and v1-to-v2 upgrades validate physical, foreign-key and semantic integrity inside the migration transaction before commit. A corrupt v1 store is rejected without persisting its upgrade. A nonempty unversioned database, foreign application ID, future version, invalid snapshot or journal gap refuses open; no incompatible database is reset.

Transaction rollback is supported. There is **no promised schema downgrade**: do not edit the version pragma to bypass an older binary's refusal. Preserve a consistent stopped-Host backup before upgrading valuable state; rollback requires the complete prior database and matching binary. The v1-to-v2 fixture verifies retained records and journal. Restore/export, managed backup, corruption repair, compaction and retention remain pending #43 scope. Do not copy an open WAL database's main file alone as a backup; its WAL may contain committed data. See [resource consent](resource-consent.md) for the new operations.

The DB is not encrypted by this adapter. The owning Host must enforce private filesystem access; deployment/device encryption and managed encryption-at-rest policy remain pending. Actual secret values belong in a vault. This API stores supplied Project text and trusted Task reports, so upstream callers must avoid embedding secrets in them; it does not redact arbitrary prose or claim encrypted storage. It makes no vault calls and stores no network credentials on its own.

Canonical records and accepted transition evidence live here. Rebuildable graph indexes, analytics projections and sampled traces are not added as canonical rows. Accepted knowledge, artifacts, decisions, integrations, client state, GoalRun checkpoints and budget reservations are not yet persisted. Full-history Task snapshots and an in-memory semantic audit are deliberately bounded-foundation choices; production retention, incremental verification and measured large-project behavior remain open. Event count limits are not a response-byte or aggregate-memory bound.

## Verification and remaining acceptance

Run `cargo test -p symbiote-store` and `cargo clippy -p symbiote-store --all-targets -- -D warnings` from the workspace. The test suite includes real temporary SQLite files, two independent connections, late transaction failure, semantic corruption and actual child-process termination. Its subprocess entrypoint is a test helper, not an additional acceptance scenario.

- Registration enforces exact Root/Role ownership, preserves stable identity and returns durable identical-retry receipts.
- Task creation rejects foreign lineage and reused workspaces without leaving a partial stream/task or journal row.
- Reconnect preserves current state; bounded replay stays within one Project and rejects future cursors.
- Trusted Task transitions and their journal rows commit together; a worker cannot bypass domain authority.
- Two writer connections racing terminal updates produce one winner and one revision conflict.
- Injected journal insertion failure rolls back previously inserted current-state records; update/delete of journal rows fails.
- Invalid files, future versions, forged Task snapshots, journal cursor zero/gaps and existing symlinks are rejected without reset.
- The parent crash test starts a real test subprocess, waits for a specific transaction boundary, kills only that owned child, and reopens the DB. It covers migration-before-commit, registered-current-state-before-journal, journal-before-commit and after-commit. The first three leave no partial registration; the committed case survives and deduplicates its retry.

These tests prove the specified local boundaries; they do not satisfy “crash at every mutation boundary” for future entity types, restore, backups, filesystem failures or external effects. #43 remains open for those obligations, encryption-at-rest decisions, export/restore identity preservation, production retention, migration compatibility beyond the initial schema, and broader canonical state coverage. The independent System Graph and production persistence architecture decision remain outside this bounded adapter.
