# Architecture decision contracts (#173)

`symbiote-architecture` provides executable in-memory decision lifecycle, immutable version pins, selective invalidation and predeclared spike contracts. It consumes the #170 product constraints accepted in ADR-0001. It does not select Tauri, SurrealDB, control-plane storage or a provider entitlement.

## Public contract

Proposals start at revision 1. Draft edits require the exact current revision. Draft revision cannot accept a decision. Acceptance is a separate operation: Worker authority is rejected, high reversal cost requires measured-evidence metadata or an explicit Client exception, and every compatibility pin must resolve to fresh verified metadata. Accepted/terminal content cannot be edited; replacements create a new identity and superseding lineage. Supersession validates before changing either record, including concurrent replacement conflicts. Caller retries with stale revisions receive conflict errors without another mutation.

Implementation artifacts pin exact accepted decision revisions and compatibility fact revisions. `require_ready` refuses provisional, unknown, rejected, superseded and stale dependencies. A changed fact or superseding decision returns the sorted affected artifact identities. Unrelated artifacts stay valid. Evidence expiry is evaluated on each gate call; acceptance alone cannot permanently bless an expiring vendor fact. Preserve the caller's approval identity and exception rationale for audit.

`SpikeContract` requires hypothesis, workload, platform/hardware, method, finite numeric thresholds, stop conditions, result artifact and cleanup. Schema v1 rejects unsupported versions and unknown fixed fields. Generate structural JSON schemas with `cargo run -p symbiote-architecture --example architecture_schema`. The schemas describe serialization; runtime validation additionally checks cross-field policy. No previous persisted schema exists: v1 is the initial version, future versions are rejected rather than guessed or downgraded.

## The repository's own decision ledger

ADR-0001's decisions are also committed as data in [`decisions.json`](../architecture/decisions.json), because a record that exists only as prose cannot refuse anything: before it existed, this crate's registry had no caller in this repository and every pin was a fixture. The ledger holds one record per decision, the compatibility facts they depend on, and one pin record per workspace member.

A decision record holds the `draft` it published from, the `published` state and revision the repository claims, the accepted text it interprets (`record`, with its `record_sha256`), the `acceptance` that made it accepted, the `blocking_issue` that owns whatever decides a choice that is still open, and the `proposed_dependencies` a dependent implementation would express the choice with. An artifact record holds its `pins` and the gate `status` this repository publishes for it. `Ledger::registry` replays the file through the crate's own transitions — propose, accept, draft revision, fact revisions, pins — and then requires every record's published state and revision to be the ones the replay reaches, so the ledger cannot record a lifecycle this crate would refuse.

Refusals, each naming its subject:

| Refusal | Why it exists |
| --- | --- |
| a schema version this loader does not replay, an unknown field, or no decision at all | a ledger that cannot be read is not a passing ledger |
| a record's text is not in the tree, or has changed since the record cited it | an accepted decision is the text it was accepted from |
| the same for evidence a decision cites | observed evidence is content-addressed; an `https://` reference is recorded, not fetched |
| a record that is still open — `proposed` or `investigating` — with no `blocking_issue`, or a decided one that still names one | an unresolved choice must have an owner |
| a `published` state or revision the replay never reaches | Worker authority, missing measured evidence on a high-reversal-cost choice and invented revisions fail here |
| an artifact `status` that is not the one `gate` reaches, a settled artifact naming a blocking issue, or an unsettled one naming none | the readiness this repository publishes is the one the registry computes |
| a workspace member cargo reports with no record, or a record for a path cargo does not report | a new member must declare what it depends on, and the ledger cannot pin a crate that does not exist |
| a member that declares one of a decision's `proposed_dependencies` without pinning it, or pins it while declaring none of them | an unresolved choice may not reach implementation unrecorded |
| a compatibility fact whose validity has lapsed | vendor facts expire independently |

`cargo test -p symbiote-architecture` runs the ledger suite, and `cargo run -p symbiote-architecture --example decisions` prints the published map — every decision with its state, revision and owners, every artifact with its status, pins, reasons and blocking issue — and exits non-zero while a refusal stands.

What this does not claim: the member list and the dependency names are what cargo reports, so a dependency reached transitively through another member, or under a package name a record does not list, is not seen; a record's `draft` is the machine-readable summary of the text it pins, and the check compares that text's hash rather than the summary against it; an issue reference names the canonical owner without resolving its live state offline; `reviewer` records who stands behind an observation, and no independent review of these records exists in this repository; and the Host does not consult this ledger at run time yet.

## Trust and applicability

These APIs enforce transitions inside this library, not authentication or filesystem security. The future Host must authenticate the caller before supplying Authority, independently verify evidence contents/hashes/reviewer provenance, validate source authority, persist the audit journal transactionally, and call `require_ready` before dependent execution. Merely presenting a HTTPS source or SHA-256 string is not evidence verification. JSON records and fixtures do not prove framework behavior. Registry state is deliberately not exposed for arbitrary mutation/deserialization into trusted accepted state.

Pure contract tests cover normal transitions, rejected worker acceptance, high-cost exceptions, stale optimistic edits/retries, immutable accepted records, atomic competing supersession, selective fact expiry/revocation, round trips and invalid thresholds. These are platform-independent policy tests; operating-system process interruption/recovery, external evidence fetching, access control, graph query integration, UI accessibility, durable migration/rollback and live runtime compatibility remain with their canonical integrations. No interrupted process or lost database write is simulated and called production recovery.

## Acceptance accounting

Implemented foundational #173 scope: decision/authority/alternative/constraint/evidence/reversal/supersession schemas; proof contract; fresh version pins; refusal gate; machine-readable affected artifacts; draft and accepted lineage behavior. Source/issue/requirement/graph/deployment links are representable.

Now enforced in this repository: ADR-0001's decisions are recorded as data — `ADR-0001/HOST-STACK` accepted on explicit client authority, `ADR-0001/DESKTOP-SHELL` and `ADR-0001/GRAPH-STORAGE` investigating under #38 and #233 — the accepted text and the client instruction that authorized it are content-addressed, every workspace member is recorded and pins the accepted stack constraint, and the one artifact that reaches a choice still under proof (`crates/symbiote-desktop`, through `tauri` and `tauri-build`) publishes `provisional` rather than `settled`. A provisional choice cannot reach a member unrecorded, and a new workspace member cannot be added without a record. No compatibility fact is recorded, because ADR-0001 certifies no version: #38 and #233 own the measurements that would create one, and a fact must be fresh and verified wherever it is pinned.

Still pending: authenticated Host enforcement (`require_ready` has no product caller yet; #180 and #181 own it), durable accepted-history/audit persistence and recovery, a recorded transition history rather than each decision's published state and one replayed draft revision, measured #38 workload, automatic research refresh and authoritative source verification, graph/issue-generation integration and an immutable content-addressed artifact store. Existing ADR-0001 remains accepted client direction; its provisional choices cannot pass a fabricated compatibility fact. #173 remains open until these obligations are accounted for through real integrations.
