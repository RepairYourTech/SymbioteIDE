# Constitution conformance

Canonical owner: [#170](https://github.com/RepairYourTech/SymbioteIDE/issues/170). `symbiote-constitution` is the ledger that turns the [product constitution](../architecture/product-constitution.md)'s non-negotiable invariants into checks that fail a build. It replaces prose that only a reader could enforce: each invariant is a record with a stable identifier, the requirement it satisfies, the exact normative clauses it depends on, the repository facts it asserts, and the tests that execute it.

The invariant catalog is `INVARIANTS` in `crates/symbiote-constitution/src/catalog.rs`: `CN-01`–`CN-24`, each naming its `#170` requirement. An entry is a compile-time constant of `&'static` data, so no runtime input can weaken, add or skip an invariant.

## Structure

Each concern has one owner, and a channel never depends on another channel:

- `catalog.rs` — the inventory and nothing else: `Fact`, `Invariant`, `INVARIANTS` and `EXPLANATIONS`, plus the normative sentences the constitution states once and more than one invariant depends on, held in one constant each so no clause has two owners.
- `document.rs` — the document channel, the embedded constitution it reads, and the coverage map parsed out of it.
- `repository.rs` — the repository channel: the tree walk, the manifest key heuristics, the four facts, and the checks this tree provides, which a coverage row's check names resolve against.
- `harness.rs` — the test channel: what cargo and the maintenance suites actually compile and run.
- `report.rs` — the verdict vocabulary (`Outcome`, `Coverage`, `Report`), the assembly of the three channels in `evaluate`, and the committed encoding.
- `claims.rs` — this contract document's own claims about the report, read from the document itself and resolved against the committed artifact.
- `lib.rs` — the crate documentation, the module map, the re-exports, and the workspace root the tests and example locate from.

The channels depend on `report` for `Outcome` and never on one another, so a change to one channel's rules lands in that channel's file and surfaces as drift in the committed artifact rather than as an unremarked difference. `--write` regenerates that artifact and is idempotent.

## Three channels, and why each can fail

`evaluate(root)` checks every invariant through three channels and reports one verdict per channel. A channel that finds nothing to check is a failure rather than a pass, so an invariant cannot be satisfied by silence.

- **document** — the required clause must appear verbatim in the constitution, and each forbidden authorization must be absent. The document is embedded with `include_str!`, so the check reads the text compiled into this binary and never a different file a later run might find. Required clauses are quoted exactly: editing a normative sentence without editing the ledger fails `the_constitution_conforms_and_every_binding_resolves`, which is how the ledger and the document cannot drift apart. The `forbidden` list is how the non-goals (`CN-10`) stop being merely an absence of lines: a later edit that writes `Electron is eligible` into the document is an authorization the build can see.
- **repository** — a fact of the tree. `no_electron` reads every `Cargo.toml` and `package.json` outside build output and dependency caches and refuses an Electron dependency in a key position. `no_go_core` refuses Go source or a `go.mod`. `forbid_unsafe_rust` requires the workspace's `unsafe_code = "forbid"` and that every crate manifest inherits it with `[lints] workspace = true`. `strict_typescript` requires the workbench's `"strict": true` and a `typecheck` script. The manifest walks fail rather than pass when they find nothing to read.
- **test** — a `relative/path::name` binding to a check the harness actually runs. [`Harness`] is the arbiter, and it does not infer runnability from a file's shape. It asks cargo (`cargo metadata --no-deps`) for the targets of every workspace member and accepts a Rust binding only when the file is one of those targets, or sits inside a member's source tree and a `mod` declaration or `#[path]` attribute actually reaches it. A Python binding must resolve to a file one of the three maintenance suites the `offline-validation` job discovers. On top of that the named item must exist, carry `#[test]` (or `def`), and not be `#[ignore]`d or `@unittest.skip`ped. Discovery failure fails every test channel with the reason rather than reporting a channel that was never checked — an appraiser with nothing to compare against has not passed.

The rule this enforces is narrow and deliberate: `tests/fixtures/not_evidence.rs` holds a plain `#[test]` that carries no `#[ignore]`, and a binding to it is refused because cargo never compiles it. Under the earlier rule, which checked only for the attribute, that binding passed.

## The coverage map accounts for itself

The constitution's coverage map — the table saying what accounts for each item #170 named — is checked rather than trusted. `document::coverage_rows` parses its rows out of the embedded text, and `document::accounts_for(row, is_known_check)` requires **every** check a row names to resolve: citing one real check beside an invented one is not accounting for the row, so a name merely shaped like a check is refused.

The resolver is `repository::Checks`, which reads the tree rather than inventing a second reading of it — the job names the workflows define (a key indented one level under `jobs:`), the invariant ids the ledger defines, and the paths and test names of the bindings `harness` has already proved are compiled and run. As everywhere else in this crate, a read that finds nothing is a failure rather than a pass: a resolver that finds no workflow job fails instead of resolving nothing.

An owner is an issue reference other than `RECORDED_ISSUE`, because naming the issue being accounted for is the claim itself rather than an owner of it. **What this does not verify**: an issue number cannot be resolved offline, so the owner half proves only that a row names some issue other than #170. Whether that issue owns the work is a human judgement recorded in the map, not something this ledger checks. The row for the reviewer gate says exactly that and routes it to #387 and #394 rather than asserting evidence that does not exist.

The rule is enforced as a test channel of `CN-22`, so `every_coverage_row_names_a_check_or_an_owner` and `a_row_citing_a_check_this_repository_does_not_run_is_refused` appear in the report as evidence and fail a build like any other channel.

## The documented contract cannot claim what the artifact does not show

`claims.rs` takes the names this document presents as its report channels from the document itself and resolves every one against the committed artifact, so the doc cannot assert a channel the report lacks while every check stays green — the failure mode behind this record's earlier untrue claims. The limit, stated once: a claim is found by the phrasing this document uses to make it, so a claim written another way is not seen, and a document making no such claim fails rather than passing vacuously.

## Evidence and reproduction

```sh
cargo test -p symbiote-constitution
cargo run -p symbiote-constitution --example constitution_report             # print
cargo run -p symbiote-constitution --example constitution_report -- --write  # regenerate
cargo run -p symbiote-constitution --example constitution_report -- --check  # report drift
```

The per-invariant report is a generated artifact guarded the way this repository guards its other generated artifacts: it is committed at [`docs/contracts/constitution-report.json`](constitution-report.json), the example regenerates or checks it, and `the_committed_report_is_what_the_tree_emits` diffs the committed encoding against a fresh run and names the first differing line. Any failed channel — or drift under `--check` — exits non-zero, so the example gates a build as well as producing the evidence published against the issue.

The suite's own tests are the evidence that the channels are not vacuous. `a_weakened_constitution_is_refused` removes one normative sentence and requires the invariant that carries it to fail. `a_forbidden_authorization_is_refused` appends an authorization and requires the non-goals to fail. `a_binding_the_harness_does_not_run_is_refused` hands the harness a test in a file cargo never compiles, a skipped test, a helper that is not a test, an unknown name, a missing file, a non-test script under a discovered suite, and malformed syntax, and requires all seven to be refused while a target test, a `mod`-reached unit test and a discovered suite test are accepted. `an_invariant_with_no_executable_check_states_why` requires every invariant to have a document clause or an executable check, and every invariant without one to appear in `EXPLANATIONS` with a stated reason — in both directions, so a reason cannot become a crutch for an entry that does have a check. `the_routed_owners_are_named_by_the_record` requires the owners `#170` routes (`CN-23`) to appear in the ledger's routing. `every_coverage_row_names_a_check_or_an_owner` requires every row of the coverage map to cite a check this repository actually runs or an issue other than the one being accounted for, and `a_row_citing_a_check_this_repository_does_not_run_is_refused` proves the resolution bites: renaming a job in the map fails the first test by name. `every_channel_the_contract_doc_claims_is_one` takes the channels this document claims from the document itself and requires the committed report to have each of them.

## Boundary and remaining acceptance

This crate checks the constitution's own claims and points at the tests that exercise them. It does not implement the invariant: a `test` binding records that a compiled, running test exercises the invariant, and the test's own body is the behavior. `owners` names the canonical issues that own an integration this contract deliberately does not implement — the executable schemas (#36, #184, #203, #205), decision governance (#173), Living Documentation (#29–#34), goal/delegation and learning identities (#460, #449), platform proofs (#38) and roadmap integrity (#470). An owner list is a routing of remaining work, not evidence that it is done.

The report is a snapshot at the head it was generated from, and the committed artifact is the drift guard rather than an authenticated claim. It is not authenticated: like every pure contract crate here, it reads files rather than authenticating their author, and it is not a substitute for the Host-side authenticated evidence #170's downstream integrations require. `Harness` runs `cargo metadata`, so the suite needs a cargo that can be started; where none can, every test channel fails by design and the reason is reported.

Security and privacy applicability is a required behavior at the authenticated Host boundary and is checked here only as constitution text (`CN-15`) plus the trust and inventory tests the entries name; the ledger reads the tree and launches nothing beyond cargo metadata, makes no network request and touches no credential. Accessibility and resource/performance applicability are not applicable to this headless check and remain required in the workbench issues. The checks are portable but only the Linux Rust toolchain was exercised; an unexercised platform is recorded as unproven rather than passed.
