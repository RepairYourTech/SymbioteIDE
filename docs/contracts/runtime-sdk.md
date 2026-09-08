# Runtime SDK foundations — #184 / #464

`symbiote-runtime-sdk` separates `AgentRuntimeAdapter` (agent/session loop) from `InferenceProviderAdapter` (model requests). It consumes canonical Dispatch, Role, Runtime Profile and enforcement records. No adapter method completes a canonical Task.

Qualification distinguishes native ownership from external full harnesses, transport from integration tier, granular capabilities from enforcement strength, and observed facts from unknown/unsupported features. Native profiles need no external installation. An embedded external harness cannot validate as native ownership. A tier label never supplies a missing capability or proves certification.

Qualification checks adapter/installation/profile revision/model, Host eligibility, version/platform proof bindings and expiry, mandatory compiled controls, named tools/skills and context bounds. Observed or emulated controls cannot satisfy preventive enforcement. The Host must authenticate referenced proof artifacts; SDK metadata cannot prove its own truth.

`PreparedLaunch` is data-only. Authorization rechecks freshness, calls a trusted `ActivationJournal` to persist the exact Dispatch and descriptor, and checks the receipt before producing a nonserializable `LaunchPermit`. Adapters must revalidate the permit immediately before activation. Rust visibility is not a sandbox against code in the trusted process. The callback is an SDK interface, not the SQLite adapter's production activation transaction. Durable fencing/outbox recovery and actual supervision remain downstream obligations. The metadata daemon exposes no launch operation in this batch.

[Runtime events](runtime-events.md) supply bounded immutable identity, gap/replay handling, tool lifecycle and cancellation uncertainty. Worker completion and exit code zero never grant canonical completion. [Provider contracts](providers.md) separate native API and harness authentication/billing and require explicit model capability support. Unknown usage remains unknown; local unauthenticated endpoints require exact Host-approved policy.

```sh
cargo test -p symbiote-runtime-sdk --locked
cargo clippy -p symbiote-runtime-sdk --all-targets --locked -- -D warnings
cargo run -p symbiote-runtime-sdk --example runtime_schema --locked
```

Reference adapters/journal in tests are in-memory conformance fixtures. They prove qualification/ordering, not real PTY execution, durable activation or OpenAI/Codex integration. The generic PTY fixture lacks required controls and is rejected; the structured fixture receives a permit only with matching fixture evidence. Real transports, installation/authentication/probe lifecycles, profile reconciliation, upstream dossiers, process boundaries, native model/tool loop, Codex App Server and cross-runtime end-to-end proof remain open under #184/#464 and integration owners. Unsupported behavior must never be replaced with simulated success.
