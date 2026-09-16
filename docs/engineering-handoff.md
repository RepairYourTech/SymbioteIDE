# Engineering handoff

## The shell contract's bar is the accepted record's — 2026-09-15 UTC

Change Stream `issue-173-accepted-bar` starts from merged #599 at `c0a536fe`. #38's workload obligations were stated four times — in ADR-0001, in the client amendment it interprets, in `docs/proofs/linux-shell.md` and in the contract JSON — and only the JSON was read, so the audit's probe (delete three workload obligations from the contract, run the suite) passed: the contract could shrink its own bar, and the bar it shrank was the accepted one. This pass gives that fact one owner and makes the contract answer it.

`SpikeContract` now carries `obligations`: the section of the decision's *accepted record* — the text the ledger pins by SHA-256 — that states what a proof of that choice must do, and one answer per clause of it. An answer is an obligation the contract declares (and a run must therefore exercise), `method` or `measurements` where the clause is about how the proof is run, or `elsewhere` naming the issue that owns a clause belonging to the decision's wider acceptance. The clauses are the record's own text, split at its sentence and semicolon boundaries; nothing is quoted into the contract, so the ADR stays the one readable copy. Two rules close the hole in both directions: a clause answered by nothing is refused where the record is read, and an obligation that answers no clause is refused where the contract is read — so neither a smaller nor a larger bar than the accepted one loads.

The audit's probe now fails by rule. Deleting the same three workload obligations leaves three answers naming obligations the contract does not declare, and the published map exits non-zero saying so (`clause 2 is answered by "#38: three active terminal tabs with bounded scrollback in real PTYs", which this contract does not declare`); deleting the answers instead leaves three clauses of the ADR's section unanswered, refused by the join. Both mutations were run and undone against the real tree.

The reconciliation is visible in the data rather than argued in prose. The committed contract answers the accepted section's 24 clauses with 32 answers: 24 naming an obligation of its own (a run has to exercise it), 2 naming its `method` or its predeclared `measurements`, and 6 naming #38 as the owner of a clause this workload does not carry — clauses 9 (the #38 scope items: file watching, menus, updates and signing, packaging, disconnect and replay), 10 (the geometry, input-method, accessibility and DOM-to-source checks), 18 (that an iframe is not evidence on its own), 19 and 20 (Preview source-edit acceptance), and 23 (bounded caches, queues and scrollback, backpressure, lazy loading, virtualization and Project-service suspension). Seven clauses are answered more than once, where a clause is both exercised and refused-on. Nothing was added to the bar: the six are named as owned elsewhere rather than quietly widened into the workload, and every clause that *is* this contract's is now tied to an accepted clause instead of to the contract's own preference.

`docs/proofs/linux-shell.md` — the copy that is a record of a run rather than accepted text — now says where the bar comes from, and the ADR itself is untouched, because accepted text is not this pass's to rewrite. What the rule proves and what it does not: it proves every clause the accepted record states is answered by something the contract holds, and that every obligation the contract declares answers a clause; it does not read the answer and judge whether an obligation's wording really covers its clause, and a pass could still move a clause to `elsewhere` or re-point it at another obligation it declares — as a change to the contract's own claims, visible in the diff, rather than as a silent shrink. #173 carries the remaining work: the contract is not the only place the shell proof's obligations are stated in prose.

## One concern per module — 2026-09-15 UTC

Change Stream `issue-173-crate-layout` starts from merged #598 at `af7e282f`. Three audits had named the same structure: `checks.rs` held seven refusal families in 448 lines, `SpikeContract`/`Measurement` lived in `lib.rs` while the code that reads them lived in `spike.rs`, and the crate root owned both the engine and the crate's shared vocabulary. This is a shape pass — same rules, same refusals, same order, same output — and the layout table in [the contract doc](../contracts/architecture.md) is the record later passes should build with.

`lib.rs` is now only the crate root: the shared vocabulary, `workspace_root`, the module map and the re-exports consumers name. `policy.rs` is the engine — states, authority, evidence, pins, facts, `DecisionRegistry` — and now owns the printed name of every state, so the duplicate that lived in `checks.rs` and in the example is gone. `spike.rs` absorbed the contract's shape, so a contract's type and the rules that read it are one file. `checks/` splits the join by subject: `mod.rs` (`Problem` and the ordered entry point), `records.rs` (a record's text and evidence, its owner, a lapsed fact), `artifacts.rs` (gate status, membership both ways, pins, provisional reach) and `contracts.rs` (the contract/decision link, obligation ownership, the run set). `Run::unmet` is now five named rule families — identity, workload, published, outcome, observations — in the order it always refused in, instead of one 141-line body.

Proof that nothing observable moved: both examples' output is byte-identical to the pre-pass binaries' — `--example decisions` and `--example architecture_schema` were run from a worktree at this pass's own base `af7e282f` and from this tree, and both diffs are empty (`IDENTICAL`, 8269-byte schema) — and all 38 refusal rules were reverted one at a time in place and still fail their own named tests (`38/38 rules bit`), with the driver resolving each anchor's file by search rather than by a path that no longer exists. `tests/governance.rs` gained `a_state_is_named_the_same_way_wherever_it_is_printed`, which holds the engine's `Display` to the serialized name for every variant of `DecisionState` and `GateStatus`, so the vocabulary cannot be printed one way and stored another.

Ruled out, and why: `Outcome` and `stop_condition` still describe "did it stop" twice, and folding them would delete two refusals rather than move code — a behaviour change this pass does not make, so it is named here instead; and the two test names the constitution's catalog binds in `tests/governance.rs` stay where they are, which the contract doc now states so a later pass does not move them.

Verified: 82 crate tests (26 ledger, 13 policy, 43 contract), 751 workspace tests on the default toolchain and on pinned 1.85.0, the integrity (160), housekeeping (11) and legacy-importer (1) suites, `cargo fmt --all --check`, strict Clippy, and the source-record check on freshly built binaries at 115 named / 0 unnamed and 42 / 0 — unchanged, because no driven binary's closure includes this crate.

## Contract-owned platform coverage — 2026-09-15 UTC

Change Stream `issue-173-contract-coverage` starts from merged #597 at `87b3b36f`. #597's own section claims that *"a single run on a single platform could never settle this contract, whose applicability is four platforms, so the flat shape would have made the path unreachable"*. That was false, and the audit proved it by building the settlement: one run, `untested_platforms: []`, the record flipped to `accepted` citing it, `blocking_issue` nulled — the map reported **0 refusals**. Coverage was enforced in neither direction, and `untested_platforms` was free text that nothing tied to the contract.

The coverage fact now has one owner: the contract declares the platforms it applies to as data (`applicable_platforms`), and its `platform` sentence points at that data rather than restating it as the authority. A settlement is refused by name when a platform the contract applies to is neither measured by one of the result's runs nor declared untested — `the result leaves "linux x11" neither measured nor declared untested, so nothing says whether the contract's applicability was covered` — so the sentence above is true now, for the reason it gave: a single-platform run cannot settle a four-platform contract. A declared untested platform keeps the choice `investigating`, as it did; declaring a platform the contract does not apply to is refused; and a contract that names no applicable platform, or names one twice, is refused when it is read.

Reproduced on the merged tree: the audit's settlement now reports three refusals, one per unaccounted platform, while the same settlement with one run per applicable platform reports 0 refusals and exits 0, so the path is walkable in the direction that measures. Every settlement in the contract suite now carries a run per applicable platform, which is also the first dossier in that suite with more than one run — the shape #597 added had never been exercised with two.

Ruled out, and why: `checks.rs`'s seven refusal families and the `SpikeContract`/`Measurement` split across `lib.rs` and `spike.rs` are the structural pass two audits have now flagged, and growing `checks.rs` here would have widened it, so the new rules live with the contract concern in `spike.rs`; the obligation list's three copies stay with #173, because the ADR is accepted text the ledger pins by hash and the proof record is history. What this does not close: a run's platform strings are self-reported, and `applicable_platforms` is this repository's own data, so a pass could shrink what a settlement has to cover by editing the contract — the workload's own platform obligation is the second copy that would notice, and #173 owns tying the copies together.

Verified locally: 81 crate tests (26 ledger, 12 policy, 43 contract; each rule reverted alone in place, failing its own named test, then restored byte-identically), 750 workspace tests on the default toolchain and on pinned 1.85.0, the integrity (160), housekeeping (11) and legacy-importer (1) suites, `cargo fmt --all --check`, strict Clippy, and the source-record check on freshly built binaries at 115 named / 0 unnamed and 42 / 0, unchanged because no compiled file moved. CI, current-head checks and independent review remain separate gates recorded on the pull request; this section is not an approval.

## Attested spike runs — 2026-09-15 UTC

Change Stream `issue-173-spike-run-attestation` starts from merged #596 at `cd204823`. #596 gave the shell choice a path to settled, and an audit of it showed the path could be walked without anything being measured: a result with an empty artifact list plus a record published `accepted` citing it passed every contract rule, deleting three workload obligations from the contract left the suite green, and the contract's `method` promised a run would record commit, platform and version, hardware, raw artifacts and failures while `Results` had no field for four of those. This slice makes the run the proof.

`Results` is now a dossier of `Run`s, one record per platform measured, and each run carries the commit it was built from, the platform and version it exercised, the hardware it ran on, the obligations it exercised named exactly as the contract names them, its outcome and stop condition, the failures it saw, one observation per predeclared measurement, and the raw artifacts it published with their SHA-256. A settlement is refused by name for a result that records no run, for a run that names no platform, version, hardware or revision, for a run that attests part of the workload or an obligation the contract never declared, for a run that published nothing, for a run that stopped on a condition the contract never declared or without recording it among its failures, and for a run that calls the platform it exercised untested. The hash path over cited artifacts is load-bearing now, because a settlement that published nothing is refused before it is reached. This section's closing claim — that one run per platform is not decoration, because a single-platform run could never settle this four-platform contract — was asserted here before it was true; the next section is the pass that made it true by giving the contract's applicability an owner in data.

Version 1 of the result schema absorbs the change rather than pretending to a history: no result artifact exists in this repository or any other, so there is nothing to migrate, and the first run ever written is written to this shape.

Ruled out, and why: deriving the contract's applicable platforms from its own `platform` prose would be prose parsing, so the platforms are declared as data instead, one section up; and repairing the obligation list's three copies (ADR-0001's proof contract, `docs/proofs/linux-shell.md`, the contract) is not honest here, because the ADR is accepted text the ledger pins by hash and the proof record is history — #173 keeps that check. The audit's other probe is re-run rather than papered over: deleting three workload obligations from the contract still leaves the suite green, because the obligations' content is #38's bar and no readable copy of it exists in the tree, so the contract doc now says that in place and names #173 as the owner of tying the copies together. What the run side closes is the half that is checkable: a run must attest every obligation the contract declares, so a narrowed contract cannot be measured against a subset.

Verified locally: 76 crate tests (26 ledger, 12 policy, 38 contract; 34 rules each reverted alone in place, each failing its own named test, then restored byte-identically), 745 workspace tests on the default toolchain and on pinned 1.85.0, the integrity (160), housekeeping (11) and legacy-importer (1) suites, `cargo fmt --all --check`, strict Clippy, and the source-record check on freshly built binaries at 115 named / 0 unnamed and 42 / 0, unchanged because no compiled file moved. CI, current-head checks and independent review remain separate gates recorded on the pull request; this section is not an approval.

## Desktop shell proof contract — 2026-09-15 UTC

Change Stream `issue-173-spike-contract` starts from merged #595 at `c9a5e320`. #173's proof mechanism had the gap the decision lifecycle had: `SpikeContract` existed in `symbiote-architecture` and no contract this repository owns was recorded, so #38's obligations lived in `docs/proofs` prose and nothing could say what the shell choice would be settled *by*. This slice records the contract as data and gives the crate a reader for it, which is a step in #173's plan rather than its closure: #173 still owes authenticated Host enforcement, durable accepted history and a transition history, among the obligations its own contract lists.

`docs/architecture/spike-contracts.json` holds one contract, `#38/desktop-shell-representative-workload`, settling `ADR-0001/DESKTOP-SHELL`: the hypothesis, the representative workload (#38's four agent streams, three bounded-scrollback terminals, editor and diffs, live Preview with floating controls, Project switch and restoration, cancellation of the whole tree, crash and reconnect, a second client over a headless Host, applicability across Wayland/X11/Windows/macOS, and whole-process-tree attribution), the reference hardware, the measurement plan, eleven predeclared ceilings (cold start to first frame and to ready, idle and under-load process-tree PSS, the unattributed share, worst input starvation, orphaned processes and ports after cancellation, journal events lost across a crash, installer size, clean locked build time), eight stop conditions and the cleanup — with every obligation tagged by the issue that owns it. The ledger record names the contract back (`proof_contract`), so the choice and its proof cannot drift apart silently.

A decision can now publish `accepted` only while a complete, in-threshold run of its contract stands, cited as its own evidence: `spike.rs` reads the result (contract identity, the contract's fingerprint when the runs happened, the platforms left untested, and one run per platform carrying its commit, platform and version, hardware, exercised obligations, outcome, failures, observations and raw artifacts with their SHA-256) and `checks.rs` joins it to the tree and the ledger. The shape of that run is extended in the next section. The fingerprint is what makes the ceilings predeclared rather than chosen after the fact: a threshold cannot move under a result measured against it. With no run committed, the shell choice stays `investigating`, which is the honest state — nothing has been measured.

Ruled out, and why: writing #38's *verdict* would invent the outcome of a spike nobody has run, so the contract records obligations and ceilings and never a result; authoring a contract for `ADR-0001/GRAPH-STORAGE` would mean writing #233's storage proof, so the field is per decision and #233 keeps its own; requiring every open decision to name a contract would do the same by force; and a results reader beyond the settlement rules (history, provenance fetching, an artifact store) belongs to the integrations #173 still lists.

Verified locally: 63 crate tests (26 ledger, 12 policy, 25 contract — 21 of them proved by reverting one rule at a time in place and watching its own named test fail, then restoring the file byte-identically, and one walking the settled path end to end), 732 workspace tests on the default toolchain and on pinned 1.85.0, the integrity (160), housekeeping (11) and legacy-importer (1) suites, `cargo fmt --all --check`, strict Clippy, and the source-record check on freshly built binaries at 115 named / 0 unnamed and 42 / 0, unchanged because no compiled file moved. CI, current-head checks and independent review remain separate gates recorded on the pull request; this section is not an approval.

## Repository decision ledger — 2026-09-15 UTC

Change Stream `issue-173-decision-ledger` starts from merged #594 at `1dc6fb96`. #173's gap was not a missing mechanism but a missing reader: `symbiote-architecture` implemented the decision lifecycle and nothing in this repository consulted it, so every pin and every state was a fixture and ADR-0001 was prose. This slice makes the repository's own decisions data and gives the crate a caller.

`docs/architecture/decisions.json` records ADR-0001 as three decisions — `ADR-0001/HOST-STACK` accepted on explicit client authority, `ADR-0001/DESKTOP-SHELL` and `ADR-0001/GRAPH-STORAGE` investigating under #38 and #233 — with the accepted text and the client instruction that authorized it content-addressed by SHA-256, and one pin record per workspace member cargo reports. `ledger.rs` replays the file through the crate's own transitions (propose, accept, draft revision, fact revisions, pins) and requires each record's published state and revision to be the ones the replay reaches; `repository.rs` asks cargo for the member list and the dependency names rather than parsing manifests by hand; `checks.rs` joins them into the refusals: a record whose text or evidence changed, an open decision with no owning issue, a decided one that still names one, a published status the gate does not reach, a member with no record, a record for a path cargo does not report, a member that declares a provisional choice's dependency without pinning it, a pin that member does not declare, and a compatibility fact whose validity lapsed. `cargo run -p symbiote-architecture --example decisions` prints the same map and exits non-zero while a refusal stands.

The published result is the state that was previously only prose: 24 members settled on the accepted stack constraint, and `crates/symbiote-desktop` — the one artifact reaching a choice still under proof, through `tauri` and `tauri-build` — published `provisional`, owned by #38. No compatibility fact is recorded, because ADR-0001 certifies no version; #38 and #233 own the measurements that would create one.

Ruled out, and why: recording a compatibility fact would mean inventing a certified version; a runtime `require_ready` caller belongs to the Host (#180/#181); and the remaining #173 obligations (durable accepted history, a recorded transition history, graph queryability) are named in [architecture governance](contracts/architecture.md) rather than half-built here. A spike contract was deferred rather than dropped, because committing one means either predeclaring #38's ceilings from outside #38 or inventing its verdict; the slice recorded above takes the first road and rules out the second.

Verified locally: 26 ledger tests and 12 policy tests in the crate, 705 workspace tests on the default toolchain and on pinned 1.85.0, the integrity (160), housekeeping (11) and legacy-importer (1) suites, `cargo fmt --all --check`, strict Clippy, and the source-record check on freshly built binaries at 115 named / 0 unnamed and 42 / 0. Twenty rules were reverted one at a time, each failing its own named test and then restored, and two on the real tree: one byte appended to ADR-0001 refuses all three records by hash, and removing the desktop record refuses the crate that declares `tauri`. CI, current-head checks and independent review remain separate gates recorded on the pull request; this section is not an approval.

## Constitution conformance structure — 2026-09-15 UTC

A structure-only pass on `issue-170-constitution-structure` splits `symbiote-constitution` so each concern has one owner: `catalog.rs` (the inventory, its `Fact`/`Invariant` types and the three normative sentences two invariants each depend on, now held in one constant each), `document.rs` (the document channel and the embedded constitution), `repository.rs` (the repository channel, its tree walk and manifest key heuristics), `harness.rs` (the test channel, unchanged in rules), `report.rs` (the verdict vocabulary, `evaluate`, and the committed encoding) and a 58-line `lib.rs` that is documentation, the module map and re-exports. The textual `include!("catalog.rs")` splice is gone, so the catalog is a real module with navigation and a module boundary. Channels depend on `report` for `Outcome` and never on one another.

No behaviour moved: the committed `docs/contracts/constitution-report.json` is byte-identical (sha256 `1b0fd21d…` before and after, `--write` idempotent, `--check` matching), so the invariants, channels, verdicts, failures and harness discovery results are the same by construction. The only source changes outside this crate are the two documentation files. The dead `identifiers()` accessor, which had no caller anywhere, is removed with the rewrite.

Verified locally: 9 conformance tests, `cargo test --workspace --locked`, `cargo fmt --all --check`, strict Clippy, the legacy/integrity/housekeeping Python suites, and the source-record check on freshly built binaries at 113 named / 0 unnamed and 40 / 0. CI, current-head checks and independent review remain separate gates recorded on the pull request.

## Constitution conformance hardening — 2026-09-15 UTC

A second slice on `issue-170-conformance-hardening` closes three defects a read-only audit found in the just-merged ledger, all of them ways the recorded evidence could be untrue rather than prose that needed tidying. The `test` channel no longer infers runnability from a file's shape: `symbiote-constitution::Harness` asks cargo for every workspace target and accepts a Rust binding only when the file is one of them or sits in a member's source tree behind a real `mod`/`#[path]` declaration, and a Python binding only when one of the three maintenance suites discovers it. That mattered: `tests/fixtures/not_evidence.rs` holds a plain `#[test]` with no `#[ignore]`, and the previous rule accepted it although cargo never compiles the file. #170's two unmapped criteria now have invariants (`CN-23` routes the named implementation owners, checked by `the_routed_owners_are_named_by_the_record`; `CN-24` requires implementations to obey the constitution and their Project/Role/Host policies, checked against the Host's policy authorization tests), and `EXPLANATIONS` makes an invariant with no executable check state why, in both directions. The report is now a guarded generated artifact: committed at `docs/contracts/constitution-report.json`, regenerated or checked by `cargo run -p symbiote-constitution --example constitution_report -- --write|--check`, and diffed by `the_committed_report_is_what_the_tree_emits`.

Ledger result: 24 invariants, 118 channels, 0 failures. Verified locally with 9 conformance tests, `cargo test --workspace --locked`, `cargo fmt --all --check`, strict Clippy, the legacy/integrity/housekeeping Python suites, and the source-record check on freshly built binaries at 113 named / 0 unnamed and 40 / 0. The design restructure the audit also named — the `include!` splice, `lib.rs` owning five concerns, and normative sentences owned by two entries — is deliberately left for a separate pass. CI, current-head checks and independent review remain separate gates recorded on the pull request.

## Constitution conformance — 2026-09-15 UTC

Change Stream `issue-170-invariant-conformance`. Owner #170 records the constitution's non-negotiable invariants as a machine-checked ledger: `symbiote-constitution` carries `CN-01`–`CN-22`, each bound to an exact document clause, a repository fact or a named workspace test, and emits the per-criterion report published against the issue. See [constitution conformance](contracts/constitution.md).

Two real gaps were fixed while making the ledger pass rather than by weakening the ledger. The constitution did not carry the methodology-boundary sentence #170 requires ("Core methodology and control cannot depend on a giant skill/prompt bundle; skills carry specialist expertise."), and three crates (`symbiote-client-sdk`, `symbiote-external-agent`, `symbiote-repo`) did not inherit the workspace's `unsafe_code = "forbid"` lint, so the workspace-wide claim was not enforced everywhere. The constitution's open-acceptance paragraph now maps every coverage item it named to the check that runs it or the issue that owns it; the production capability and the executable schemas remain owned by #36/#173/#38/#29-#34/#460/#449, and #170's own deliverable is the recorded coverage.

Verification at the pull request's head: `cargo test -p symbiote-constitution` (7 tests, 22 invariants, 112 channels), `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings` and `cargo test --workspace --locked`. CI, current-head checks, current-head evidence and independent review remain separate gates recorded on the pull request; this section is not an approval.

## Durable workforce bindings — 2026-09-08 UTC

Change Stream `issue-203-workforce-bindings` starts from merged #485 at `7dadbefe`.
The Host persists staffing intent around canonical WorkforceBinding identity and
reports incomplete readiness without authorizing execution. See
[workforce bindings](contracts/workforce-bindings.md) for schema v5, protocol v1.5,
historical Team validation and the remaining activation gates. #203 and the full
build goal remain open.

## Live Host inventory — 2026-09-08 UTC

Change Stream `issue-193-host-inventory` starts from merged #484 at `0d6c65a`.
The local Host exposes bounded passive resource observations with private stable
identity, expiry and explicit telemetry disablement. See [Host inventory](contracts/host-inventory.md)
for protocol v1.4, actual probe scope and remaining scheduling acceptance.
Effective capacity and reservations remain unproven, so they cannot authorize
worker launch. #193 and the full build goal remain open.

## Durable Project Team — 2026-09-08 UTC

Change Stream `issue-200-project-team` starts from merged #483 at `8383a6b`.
The Host stores Team configuration with revision checks, journal replay and
separate management authority. See [Project Team](contracts/project-team.md)
for schema v4 migration, protocol v1.3 and the executable CLI demonstration.
This establishes staffing intent; qualified native/external bindings and worker
execution remain pending. #200 and the full build goal remain open.

## Runtime discovery — 2026-09-08 UTC

Change Stream `issue-186-runtime-discovery` starts from merged #482 at `1735a48`.
The versioned inventory and read-only Codex probe distinguish observations from
activation authority. See [runtime discovery](contracts/runtime-discovery.md)
for the real offline sandbox proof, pinned compatibility and pending acceptance.
The application is not ready to ship: discovery is not worker execution, and both
native and external coding workflows still need end-to-end integration.

## Canonical work hierarchy — 2026-09-08 UTC

Change Stream `issue-189-work-hierarchy` starts from merged #481 at `bb95e49`.
Owner #189 adds replay-validated work aggregates, transactional graph/origin
storage and Host protocol v1.2. See [the hierarchy contract](contracts/work-hierarchy.md)
for schema v3 migration, authority, bounds and remaining acceptance. Existing
unclassified Tasks cannot start until explicitly assigned an origin.

Domain and storage authors independently cross-review each other's modules and
the Host integration. A separate review task was stopped by an automatic
security filter and is not counted as completed review. Current-head checks,
merge and issue evidence remain separate gates. #189 and the persistent build
goal remain open for the full product integration.

## Linux process sandbox — 2026-09-08 UTC

Change Stream `issue-218-linux-sandbox` starts from merged #480 at `06c81f9`.
Owner #218 consumes the runtime and resource-consent foundations. The new
launcher is a bounded internal Linux execution boundary, with read-only and
worktree-write profiles, isolated networking and a trusted helper for closing
inherited descriptors. It is not an authenticated Host worker endpoint.

See [the sandbox contract](security/linux-sandbox.md) and
[the live alias probes](proofs/linux-sandbox-aliases.md). Review identified
pathname socket, hardlink and inherited-descriptor hazards; these require
preventive checks in addition to namespace flags. One separate review agent's
test-writing turn was stopped by an automatic security filter citing possible
cybersecurity risk. That turn is not counted as completed review. Another
independent reviewer completed the full crate, helper, test and documentation
review with no blocking findings under the documented trust boundary. Heartbeat
assertions were tightened and fixtures bounded to ten seconds; the example now
asserts its report and exit.

Current-head verification and integration evidence must be recorded before
claiming this slice delivered. Full #218, production worktree provisioning,
authenticated dispatch, both live runtimes and the first usable release remain
open. The persistent build goal is not complete.

## Durable resource consent — 2026-09-08 UTC

Change Stream `issue-174-191-trust` starts from merged #479 at `615e483`. Owners #174/#191 establish the trust/threat baseline and a durable exact-resource consent boundary. New `symbiote-trust` checks snapshots, expiry/revocation and current policy; Host protocol v1.1 records/reads/revokes consent using server-derived authority, and store schema v2 journals those decisions transactionally.

See `docs/contracts/resource-consent.md` and `docs/security/trust-boundaries.md` for migration and remaining acceptance. Same-UID local-owner bootstrap is not a worker sandbox: #218 must prove preventive isolation before native/external workers launch. Licensing, publisher signatures, active revocation, complete trust tests and the first usable release remain pending. Broad #174/#191 and the persistent build goal stay open.

## Inactive profile projection — 2026-09-08 UTC

Change Stream `issue-187-projection` starts from merged #478 at `76ab590`. Owner #187 consumes #184/#179/#214 foundations. `symbiote-projection` reconciles explicit managed TOML primitives, binds pack mappings to exact eligible environment resources and publishes fresh private profile generations with readiness/content verification. Existing generations and native homes are never overwritten.

See `docs/contracts/projection.md` for tests, demo and remaining acceptance. This is inactive profile preparation, not native/Codex compatibility or activation. Next execution prerequisites still include #174/#191/#218 trust/threat/preventive enforcement, worktree/context/dispatch integration and real runtime packs. Broad #187 and the persistent build goal remain incomplete.

## Agent environment contracts — 2026-09-08 UTC

Change Stream `issue-214-environment` starts from merged #477 at `18fd58c`. Owner #214 consumes the #179/#184 foundations. Versioned desired resource declarations enter the portable Project manifest, with read-only environment resolution and explicit scope, trust, ownership and compatibility boundaries. Native files remain projections; this batch performs no resource installation or worker activation.

See `docs/contracts/agent-environment.md` for parsing, merge and remaining integration acceptance. #187 can consume these contracts for read-only projection planning before native file reconciliation. #174/#191/#218 trust and preventive enforcement, #189 hierarchy, actual native/Codex packs and full first-release verification remain open. The persistent build goal is not achieved.

## Local structured runtime transport — 2026-09-08 UTC

Change Stream `issue-185-transport` starts from merged #476 at `d7b1391b0e03e861a168e915c0602be681e3b83d`. Owner #185 consumes the #180/#184 foundational contracts. This batch adds a process-backed local JSONL substrate and separate strict JSON-RPC 2.0 response correlation. It does not expose a Host worker activation endpoint or certify Codex/ACP compatibility.

Independent review separates process supervision/framing from RPC correlation. See `docs/contracts/runtime-transport.md` for the actual guarantees, fixture commands and remaining acceptance. Broad #185 remains open for the other substrates, real packs, complete containment and recovery. The full native/Codex first release and persistent build goal remain incomplete.

Next dependency-ready work: #214 versioned agent environment/resource desired-state contracts before #187 native configuration projection; #189 request/objective/capability/plan hierarchy can consume existing #36/#43/#181 foundations independently. Activation still requires the #174 → #191 → #218 trust/threat/enforcement chain. A successful process fixture supplies no sandbox or credential authority.

## Runtime SDK foundation — 2026-09-08 UTC

Change Stream `issue-184-runtime-sdk` starts from merged #475 at `c1b7f2f6d7adcbe7c50377d1ff9e990b11019ebe`. Owners #184/#464 consume the reviewed #181/#36 contracts. `symbiote-runtime-sdk` separates agent-loop adapters from inference providers, qualifies immutable dispatches against current identity-bound capability/control evidence, and defines bounded session events and native/external auth/billing checks.

Independent review covers adapter capability/activation logic separately from provider/event logic. New failure fixtures cover proof substitution, stale controls, context bounds, native/external ownership, absent CLI/local endpoint metadata, false completion, replay gaps, cancellation uncertainty and impossible/over-budget usage. Conformance fixtures are not real runtimes; the Host has no SDK activation endpoint or implementation of its ActivationJournal callback yet.

Next: implement dependency-ready runtime profile/configuration and trust/enforcement foundations, then actual adapter transport and dispatch integration under the canonical queue. #184/#464 remain open for real reference transports, compatibility dossiers, process isolation and full native/Codex operation. See `docs/contracts/runtime-sdk.md`, `runtime-events.md` and `providers.md`. Overall first usable release remains incomplete; do not mark the persistent build goal achieved.

## Durable metadata Host — 2026-09-08 UTC

Current Change Stream: `issue-43-durable-host`, isolated from merged #474 at `42eda51a7e4955cb5f44aa85833e8a313a6ae3a7`. Owners #43/#181/#180 consume the reviewed #36/#176 foundational contracts. New Rust workspace crates implement SQLite current-state/journal transactions, typed transport-neutral requests, and an actual Linux daemon/CLI using private same-UID IPC. The GUI has no canonical state ownership. No agent subprocess or model billing is enabled.

The daemon can register a Project with its Roots/Roles, create ready Tasks/initial Change Streams, read records and replay committed events. Real-process tests force daemon death and transaction-boundary death, reconnect independent clients, replay dropped replies and check concurrency. Separate review covers storage/protocol and Host transport; current-head CI belongs to the containing PR. See `docs/contracts/{storage,protocol,host}.md` for the exact implemented boundaries and reproduction commands.

Next prerequisite work includes execution permissions/credentials and durable side-effect disposition, runtime/workforce contracts, actual process/worktree supervision and the remaining #38 architecture proofs. Native/Codex execution, desktop integration, backup/export recovery and full protocol/service packaging remain pending. Do not close broad #43/#181/#180 on metadata storage alone or advertise the shell/security/runtime as release-ready.

## Architecture proof batch — 2026-09-08 UTC

Foundations PR #473 merged at `a15368c1744fdff3c186dda11f24714d738c7efe` after separate review and passing stable/MSRV/roadmap checks. The #36/#170/#176/#179/#173 issues remain open for their broader acceptance. Current proof work is isolated on `issue-38-linux-proof` from that revision.

Two runnable experiments now exist: `spikes/linux-shell` (Tauri/React/Monaco, three native PTYs, separate Preview and overlapping Lead WebViews) and `spikes/host-lifecycle` (independent Rust daemon, fixed process tree, disconnect/replay/cancel/restart fixtures). See `docs/proofs/linux-shell.md` and `docs/proofs/host-lifecycle.md` for reproduction, measurements, raw evidence and limitations. The initial GTK composition failure is retained alongside corrected interaction screenshots. Preview denial reports are untrusted observations, not authenticated security proof.

These experiments do not select the desktop shell or transactional storage. Native Wayland/real X11, Windows/macOS, accessibility/IME/scaling, arbitrary descendant containment, representative resource budgets and full Preview authority isolation remain unpassed. The injected guardian-loss test exposes a recovery boundary instead of claiming unconditional orphan cleanup. No native API/Codex integration, production Host or usable release is complete. Continue the live #38 acceptance and separate #43 storage decision before dependent production integration; preserve the canonical queue and open broad issues.

## Executable foundation batch — 2026-09-08 UTC

PR #472 was reviewed and merged as `d2c5d300652e3733ec0172c4b0ef0690fa168e37`. Current worktree `/mnt/data/projects/Symbiote-worktrees/issue-36-176-foundations`, branch `issue-36-176-foundations`, starts from that merged revision. Owners are #36/#170 domain/constitutional contracts, #176/#179 configuration/portable manifests, and #173 architecture decision/proof contracts. The first usable release must prove both native Rust/OpenAI API and external Codex execution under shared canonical controls; none of this batch calls paid inference or claims those integrations.

Three pure Rust crates replace the absence of application contracts: typed identities/dispatch/lifecycle evidence; scoped configuration/portable manifest and conflict previews; immutable accepted architecture decisions/version pins and selective invalidation. See `docs/contracts/` for acceptance coverage and remaining integration obligations. `README.md` documents reproducible build/test/schema commands. A Cargo lockfile pins dependencies; CI adds stable and Rust 1.85 contract verification. No unsafe code is permitted in these crates.

The baseline planning suites passed before changes. Independent review identified forged deserialization/dispatch-context/freshness defects in the domain boundary, invalid portable paths, and incomplete affected-artifact reporting in decision acceptance. All were fixed with regression tests and independently re-reviewed. Final local checks: 42 Rust tests pass (18 domain, 12 configuration, 12 architecture), strict all-target Clippy passes, formatting/diff checks pass and all three schema generators run. The domain reviewer also verified six independent adversarial probes including history tampering. CI/MSRV and merge evidence belong to the containing PR; no GitHub approval is inferred from separate-agent review.

The #38 environment probe found GTK3/WebKitGTK4.1, Tauri CLI and Xvfb. Native display is KDE Wayland; its `:1` X11 connection is XWayland, not standalone X11 certification. The Linux shell spike is prepared separately under `issue-38-linux-proof`; it must not be represented as a chosen desktop architecture. Windows/macOS, minimum-target resource budgets, Preview authority isolation, signed updates and full representative workload remain proof gates. Prototype preparation is independent; execution follows the reviewed #173 foundational contract. Graph storage and transactional control-plane storage remain separate candidates.

Next: integrate reviewed/current-head foundation checks, execute the bounded #38 Linux spike and publish raw evidence/failures, then advance #43/#181/#180 only after their explicit schema/persistence/proof requirements are supported. Keep broad issues open for later Host authentication, durable recovery, runtime, graph, full ontology and cross-platform acceptance. Never mark code-level enforcement authenticated solely because a test supplies an `Actor::Host` or an evidence record.

## Initial takeover record

Recorded 2026-09-08 UTC. Canonical owners: #170 (constitution), #470 (roadmap integrity); #173/#38 own remaining governance/proof. Branch `issue-170-470-takeover-integrity`, worktree `/mnt/data/projects/Symbiote` (also `/home/birdman/Projects/Symbiote`). Base and inspected target: `772fe8446f4ba42d9ad037de76acc3d792fc1d07`. The containing PR's head is the exact implementation revision; this record does not certify a later head.

## Verified starting state

The supplied directory was empty. Cloned the authenticated GitHub repository without deleting existing content. Default `main` contained only planning/import scripts, encoded historical payloads and issue templates: no application, Cargo/npm manifest, tests, accepted ADRs or AGENTS.md. No open PRs or other worktrees were present in this checkout. Remote audit branch `planning/audit-2026-09-07` is preserved and not merged as executable bootstrap. No branch protection/rulesets were configured when inspected; the user's independent-review/current-head gates still apply.

Rust 1.97.1, Cargo 1.97.1, Node 22.22.1 and Python 3.14.7 are available locally. Previous CI certified planning imports/audit only; the latest historical verification workflow had failed. No application/platform baseline exists to certify. Python remains existing repository-maintenance tooling, not a second core service implementation language.

## Delivered scope

- [Constitution](architecture/product-constitution.md) and [ADR-0001](architecture/adr-0001-technology-direction.md) record the accepted Rust/strict-TypeScript/no-Electron direction. Tauri 2 and SurrealDB remain candidates awaiting evidence. #170 remains open for executable schema/conformance and later integration acceptance.
- Amended #36, #38, #54, #154, #165, #170, #180, #191, #233, #336, #353, #375, #431, #446, #464, #465 and #467. Exact read-back verified each body, and full before/after inventories confirmed unchanged titles, states, labels, assignees and milestones. Removed the positive Electron candidate wording from #38 and #336. Baseline v2.4 identity/revision markers remain; a separate dated architecture-amendment marker records precedence.
- Historical Python/Node importer entrypoints now refuse before loading payloads or credentials. Historical workflow jobs are disabled and issue-write permissions removed. Payload/history is retained. These protections take effect on the default branch only after reviewed integration; old git revisions and the audit branch remain historical executable code and must not be run as synchronizers.
- The offline #470 validator generates a compact canonical registry and topological index from current issue evidence. It validates structure, not implementation completion. A safe remote regeneration writer is deliberately absent.

## Evidence and limitations

Captured before/after inventories with `python planning/capture_roadmap.py OUTPUT.json`: 466 issues. Captures are local evidence in `/tmp/symbiote-takeover-before.json` and `/tmp/symbiote-takeover-after.json`; recapture for future work, never treat temporary paths as durable authority. Amendment hash/read-back results: `/tmp/symbiote-amendment-results.json`.

The bounded manual issue amendment performed a batch preflight, immediate per-issue body/timestamp check, body-only PATCH and exact metadata/body read-back. GitHub atomic compare-and-swap was not assumed; this is not certification of the concurrent-edit/three-way-merge writer required by #470. No ambiguous mutation was retried. No issue was closed or marked implemented.

Local regression commands:

```sh
python -m unittest discover -s planning -p 'test_legacy_importers.py' -v
python -m unittest discover -s planning/integrity -p 'test_*.py' -v
git diff --check
```

Results: 23 offline integrity tests pass, including the persistent reduced audit fixture; the legacy test passes for all four entrypoints. The full after-capture validates 241 tasks, 19 epics, 198 references, one historical program entry and 702 prerequisite edges. A separate review agent found a fail-open dependency parser; missing/empty/unsupported declarations now refuse and details sections no longer hide canonical edges. The reviewer verified the fixes, all 23 tests, exact generated registry equality with the full capture and exact index rendering, with no remaining bounded-slice findings. This is separate-agent review, not a GitHub approval or platform certification. PR CI and merge remain separate gates.

No Rust runtime, shell spike, storage benchmark, cross-platform test, preview security property, provider entitlement, merge, deployment or release is claimed. #470 still requires safe three-way regeneration/dry-run, stale authority handling, ambiguous create/lost-response recovery, retained-scope coverage and mutation read-back integration. The offline validator and unconditional legacy refusal cover only the first protective slice.

## Dependency-correct next work

Use the generated index and retrieve full issue acceptance/comments just in time. Topological order alone is not readiness; open prerequisites need explicit verified foundational evidence, not checked boxes. #170's contract is documented but executable schema/conformance is pending. Establish that foundational evidence, then #173's versioned ADR/spike governance and #36 domain/schema contracts. #38 remains gated by #170/#173 and must precede dependent Host/desktop implementation. Advance #470's remaining mutation-safety contract after confirming its #170 prerequisite; independent read-only safeguards do not lock an unproven product architecture. #171 is the other dependency-root investigation owner and can advance independently with bounded current-source research.

Before integration, inspect the separate review findings and PR current-head CI, refresh `origin/main`, and revalidate if the target changes. Leave this Change Stream as a review-ready PR unless all required gates and authority are satisfied. Continue through the same canonical owners rather than creating a parallel backlog.
