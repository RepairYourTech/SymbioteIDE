# Constitution conformance

Canonical owner: [#170](https://github.com/RepairYourTech/SymbioteIDE/issues/170). `symbiote-constitution` is the ledger that turns the [product constitution](../architecture/product-constitution.md)'s non-negotiable invariants into checks that fail a build. It replaces prose that only a reader could enforce: each invariant is a record with a stable identifier, the requirement it satisfies, the exact normative clauses it depends on, the repository facts it asserts, and the workspace tests that execute it.

The invariant catalog is `INVARIANTS` in `crates/symbiote-constitution/src/catalog.rs`: `CN-01`–`CN-22`, each naming its `#170` requirement. An entry is a compile-time constant of `&'static` data, so no runtime input can weaken, add or skip an invariant.

## Three channels, and why each can fail

`evaluate(root)` checks every invariant through three channels and reports one verdict per channel. A channel that finds nothing to check is a failure rather than a pass, so an invariant cannot be satisfied by silence.

- **document** — the required clause must appear verbatim in the constitution, and each forbidden authorization must be absent. The document is embedded with `include_str!`, so the check reads the text compiled into this binary and never a different file a later run might find. Required clauses are quoted exactly: editing a normative sentence without editing the ledger fails `the_constitution_conforms_and_every_binding_resolves`, which is how the ledger and the document cannot drift apart. The `forbidden` list is how the non-goals (`CN-10`) stop being merely an absence of lines: a later edit that writes `Electron is eligible` into the document is an authorization the build can see.
- **repository** — a fact of the tree. `no_electron` reads every `Cargo.toml` and `package.json` outside build output and dependency caches and refuses an Electron dependency in a key position. `no_go_core` refuses Go source or a `go.mod`. `forbid_unsafe_rust` requires the workspace's `unsafe_code = "forbid"` and that every crate manifest inherits it with `[lints] workspace = true`. `strict_typescript` requires the workbench's `"strict": true` and a `typecheck` script. The manifest walks fail rather than pass when they find nothing to read.
- **test** — a `relative/path::name` binding to a workspace test. The checker reads the named file, requires the item to exist, requires a `#[test]` attribute on it (or `def` for the Python maintenance suites), and refuses an item marked `#[ignore]` or `@unittest.skip`. The workspace test run executes the bound test; the binding is what keeps the ledger from citing a check that has been renamed, moved or skipped.

## Evidence and reproduction

```sh
cargo test -p symbiote-constitution
cargo run -p symbiote-constitution --example constitution_report
```

The example emits the per-invariant report as JSON — identifier, requirement, statement, owners and one verdict per channel — and exits non-zero when any channel failed, so it is usable as a gate as well as a published artifact. The committed report quoted against #170 is the output of that command at the pull request's head.

The suite's own tests are the evidence that the channels are not vacuous. `a_weakened_constitution_is_refused` removes one normative sentence and requires the invariant that carries it to fail. `a_forbidden_authorization_is_refused` appends an authorization and requires the non-goals to fail. `a_missing_skipped_or_non_test_binding_is_refused` hands the binding checker a skipped test, a helper that is not a test, an unknown test name, a missing file and malformed syntax, and requires all five to be refused. `the_repository_keeps_the_locked_technology_choices` asserts each fact directly.

## Boundary and remaining acceptance

This crate checks the constitution's own claims and points at the workspace tests that exercise them. It does not implement the invariant: a `test` binding records that a named test exists and runs, and the test's own body is the behavior. `owners` names the canonical issues that own an integration this contract deliberately does not implement — the executable schemas (#36, #184, #203, #205), decision governance (#173), Living Documentation (#29–#34), goal/delegation and learning identities (#460, #449), platform proofs (#38) and roadmap integrity (#470). An owner list is a routing of remaining work, not evidence that it is done.

The report is a snapshot at the head it was generated from; a later edit to the constitution, a manifest or a named test invalidates it until the suite runs again. It is not authenticated: like every pure contract crate here, it reads files rather than authenticating their author, and it is not a substitute for the Host-side authenticated evidence #170's downstream integrations require.

Security and privacy applicability is a required behavior at the authenticated Host boundary and is checked here only as constitution text (`CN-15`) plus the trust and inventory tests the entries name; the ledger reads the tree and launches nothing, makes no network request and touches no credential. Accessibility and resource/performance applicability are not applicable to this headless check and remain required in the workbench issues. The checks are portable but only the Linux Rust toolchain was exercised; an unexercised platform is recorded as unproven rather than passed.
