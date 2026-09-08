# Architecture decision contracts (#173)

`symbiote-architecture` provides executable in-memory decision lifecycle, immutable version pins, selective invalidation and predeclared spike contracts. It consumes the #170 product constraints accepted in ADR-0001. It does not select Tauri, SurrealDB, control-plane storage or a provider entitlement.

## Public contract

Proposals start at revision 1. Draft edits require the exact current revision. Draft revision cannot accept a decision. Acceptance is a separate operation: Worker authority is rejected, high reversal cost requires measured-evidence metadata or an explicit Client exception, and every compatibility pin must resolve to fresh verified metadata. Accepted/terminal content cannot be edited; replacements create a new identity and superseding lineage. Supersession validates before changing either record, including concurrent replacement conflicts. Caller retries with stale revisions receive conflict errors without another mutation.

Implementation artifacts pin exact accepted decision revisions and compatibility fact revisions. `require_ready` refuses provisional, unknown, rejected, superseded and stale dependencies. A changed fact or superseding decision returns the sorted affected artifact identities. Unrelated artifacts stay valid. Evidence expiry is evaluated on each gate call; acceptance alone cannot permanently bless an expiring vendor fact. Preserve the caller's approval identity and exception rationale for audit.

`SpikeContract` requires hypothesis, workload, platform/hardware, method, finite numeric thresholds, stop conditions, result artifact and cleanup. Schema v1 rejects unsupported versions and unknown fixed fields. Generate structural JSON schemas with `cargo run -p symbiote-architecture --example architecture_schema`. The schemas describe serialization; runtime validation additionally checks cross-field policy. No previous persisted schema exists: v1 is the initial version, future versions are rejected rather than guessed or downgraded.

## Trust and applicability

These APIs enforce transitions inside this library, not authentication or filesystem security. The future Host must authenticate the caller before supplying Authority, independently verify evidence contents/hashes/reviewer provenance, validate source authority, persist the audit journal transactionally, and call `require_ready` before dependent execution. Merely presenting a HTTPS source or SHA-256 string is not evidence verification. JSON records and fixtures do not prove framework behavior. Registry state is deliberately not exposed for arbitrary mutation/deserialization into trusted accepted state.

Pure contract tests cover normal transitions, rejected worker acceptance, high-cost exceptions, stale optimistic edits/retries, immutable accepted records, atomic competing supersession, selective fact expiry/revocation, round trips and invalid thresholds. These are platform-independent policy tests; operating-system process interruption/recovery, external evidence fetching, access control, graph query integration, UI accessibility, durable migration/rollback and live runtime compatibility remain with their canonical integrations. No interrupted process or lost database write is simulated and called production recovery.

## Acceptance accounting

Implemented foundational #173 scope: decision/authority/alternative/constraint/evidence/reversal/supersession schemas; proof contract; fresh version pins; refusal gate; machine-readable affected artifacts; draft and accepted lineage behavior. Source/issue/requirement/graph/deployment links are representable.

Still pending: authenticated Host enforcement, durable accepted-history/audit persistence and recovery, measured #38 workload, automatic research refresh and authoritative source verification, graph/issue-generation integration, immutable content-addressed artifact store and full implementation-time pin enforcement. Existing ADR-0001 remains accepted client direction; its provisional choices cannot pass a fabricated compatibility fact. #173 remains open until these obligations are accounted for through real integrations.
