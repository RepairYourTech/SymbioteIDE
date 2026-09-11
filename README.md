# Symbiote

Symbiote is an open-source AI software-development firm with a human client. The canonical executable roadmap is [#154](https://github.com/RepairYourTech/SymbioteIDE/issues/154); accepted product direction is in [the constitution](docs/architecture/product-constitution.md).

## Current implementation

The [canonical work hierarchy](docs/contracts/work-hierarchy.md) now records
requests, objectives, capabilities and plans through the Host. Tasks require an
explicit origin, and user edits preserve history across restarts; worker
execution and verified capability closure remain separate gates.

The Rust workspace now includes a local `symbioted` service and `symbiote` CLI backed by a transactional SQLite store. Clients register Projects and ready Tasks, read canonical records and replay the journal across daemon restarts. Domain/lifecycle, configuration/portable-manifest and architecture-decision contracts remain shared libraries. Agent execution and the complete desktop application are not implemented yet; native Rust/OpenAI API and external Codex execution are both required for the first usable release. No paid inference is performed by these tests.

Start the local service using the [Host run instructions](docs/contracts/host.md). The Host currently targets Linux and authenticates local clients using OS peer credentials. The separate [Linux shell experiment](docs/proofs/linux-shell.md) remains a proof workload, not the production workbench.

The [runtime SDK](docs/contracts/runtime-sdk.md) now defines capability-qualified agent adapters, separate inference providers and bounded session events. These contracts are prerequisites for live native/Codex integration; they do not enable agent execution in the Host yet.

The [local runtime transport](docs/contracts/runtime-transport.md) adds bounded subprocess JSONL I/O and explicit JSON-RPC response correlation. Its deterministic harness fixtures exercise process failures; real runtime packs and sandbox enforcement remain pending.

[Agent environments](docs/contracts/agent-environment.md) describe portable resource intent and resolve Role/profile settings before native configuration is projected. Host permission and credential gates remain separate from read-only configuration resolution.

[Configuration projection](docs/contracts/projection.md) now reconciles managed TOML keys and publishes verifiable inactive profile generations while preserving existing profiles. Actual harness activation and reload verification remain pending.

The Host records [resource consent and revocation](docs/contracts/resource-consent.md) against exact fingerprints and scope. The [trust boundary baseline](docs/security/trust-boundaries.md) keeps OS worker isolation and other release requirements explicit; consent metadata does not activate a resource.

The [Linux sandbox](docs/security/linux-sandbox.md) develops preventive process
isolation for read-only and worktree-write execution. Real hostile-process tests
cover its boundary; authenticated worker launch and full runtime integration
remain separate acceptance gates. Linux test runs require an installed,
operational `bwrap` and enabled unprivileged namespaces.

```sh
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo fmt --all --check
cargo run -p symbiote-domain --example domain_schema
cargo run -p symbiote-config --example config_schema
cargo run -p symbiote-architecture --example architecture_schema
cargo run -p symbiote-protocol --example protocol_schema
cargo run -p symbiote-runtime-sdk --example runtime_schema
```

Schema examples emit JSON to stdout. The generated structural schemas do not replace runtime cross-field checks. The `symbiote` CLI's `--json` envelope and its authorization policy are published as committed JSON Schema fixtures in [docs/contracts/schemas](docs/contracts/schemas); `symbiote schema` emits them from the binary (`schema envelope|policy` prints one document, `--write DIR` regenerates the committed fixtures as generated artifacts, `--check DIR` reports drift without writing), and a test diffs the emitted documents against them. Rust 1.85 is the declared minimum; Cargo.lock pins dependencies. CI tests the minimum and current stable toolchain. Workspace code forbids unsafe Rust.

Contract documentation: [domain](docs/contracts/domain.md), [configuration](docs/contracts/configuration.md), [architecture governance](docs/contracts/architecture.md), [storage](docs/contracts/storage.md), [protocol](docs/contracts/protocol.md), [Host](docs/contracts/host.md), [CLI](docs/contracts/cli.md). Each records implemented behavior and pending integration acceptance. [Engineering handoff](docs/engineering-handoff.md) tracks the current Change Stream and next gates.

## Architecture

Rust owns `symbioted`, native agent, CLI and canonical control; strict TypeScript/React is the preferred static workbench. Desktop is a Host client. No Electron. Tauri 2 and SurrealDB remain preferred candidates pending [#38](https://github.com/RepairYourTech/SymbioteIDE/issues/38) proof; control-plane persistence is evaluated separately.

The historical roadmap importers are retired and refuse execution. Use the [read-only integrity tooling](planning/integrity/README.md) for inventory/validation, never historical encoded payloads as authoritative synchronizers.
