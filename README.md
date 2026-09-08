# Symbiote

Symbiote is an open-source AI software-development firm with a human client. The canonical executable roadmap is [#154](https://github.com/RepairYourTech/SymbioteIDE/issues/154); accepted product direction is in [the constitution](docs/architecture/product-constitution.md).

## Current implementation

The Rust workspace implements foundational domain/lifecycle, configuration/portable-manifest, and architecture-decision contracts. These are usable pure libraries with executable validation and failure tests, not yet a Host daemon or desktop application. Native Rust/OpenAI API and external Codex execution are both required for the first usable release. No paid inference is performed by these tests.

```sh
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo fmt --all --check
cargo run -p symbiote-domain --example domain_schema
cargo run -p symbiote-config --example config_schema
cargo run -p symbiote-architecture --example architecture_schema
```

Schema examples emit JSON to stdout. The generated structural schemas do not replace runtime cross-field checks. Rust 1.85 is the declared minimum; Cargo.lock pins dependencies. CI tests the minimum and current stable toolchain. Workspace code forbids unsafe Rust.

Contract documentation: [domain](docs/contracts/domain.md), [configuration](docs/contracts/configuration.md), [architecture governance](docs/contracts/architecture.md). Each records implemented behavior and pending integration acceptance. [Engineering handoff](docs/engineering-handoff.md) tracks the current Change Stream and next gates.

## Architecture

Rust owns `symbioted`, native agent, CLI and canonical control; strict TypeScript/React is the preferred static workbench. Desktop is a Host client. No Electron. Tauri 2 and SurrealDB remain preferred candidates pending [#38](https://github.com/RepairYourTech/SymbioteIDE/issues/38) proof; control-plane persistence is evaluated separately.

The historical roadmap importers are retired and refuse execution. Use the [read-only integrity tooling](planning/integrity/README.md) for inventory/validation, never historical encoded payloads as authoritative synchronizers.
