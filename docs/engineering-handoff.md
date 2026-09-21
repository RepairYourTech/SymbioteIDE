# Engineering handoff

Where this work stands, how a claim in this repository is held, and what remains
open. Mission, authority and the critical path are in
[the takeover prompt](agent-takeover.md); what is implemented is in the
[README](../README.md) and in each subsystem's contract doc.

This file succeeds a per-pass chronicle that recorded every Change Stream's own
test counts. Those counts are what the repository's claims are *about*, and in
that form they went stale: twice a pass left a figure the tree contradicted, and
finding it meant reconstructing which run the sentence had meant. So this document
states no figure of its own. A number lives in the artifact that produces it — a
committed report, a suite's own output, a workflow — and is pointed at from here.
It cites no case name either: where a check matters it names the file or the
document that owns it, and the crate doc holds its own citations against its suite.
Its own citations are held the same way, by
[the integrity suite's handoff case](../planning/integrity/test_handoff.py), whose own
docstring states what it holds and what nothing there can.

## How a claim is held here

Five habits, each of them checked rather than asserted:

- **One owner per fact.** A rule about a record lives with the records, a rule
  about the tree with the artifacts, a rule about proof with the contracts, and a
  rule about a document with the component that owns the document.
- **A refusal rule is proved by reverting it.**
  [`planning/integrity/revert_rules.py`](../planning/integrity/revert_rules.py)
  removes each rule of the architecture governance crate alone, in a throwaway copy
  of the tree, and watches the case that holds it fail; its table, what that table
  does and does not claim, and the test that holds it to the tree are described in
  [the integrity README](../planning/integrity/README.md).
- **A generated artifact is guarded, not trusted.** Each committed artifact has a
  `--check` path plus a case that diffs the emitted bytes, so the artifact cannot
  drift from the tree silently.
- **A citation is a claim.** A document that names a check is held to a name a
  suite runs, or it names the file that owns the assertion instead.
- **A moved shape is reconciled where the feature is documented**, saying what
  moved and why the version absorbs it, so a reader of an old artifact gets an
  actionable refusal rather than a missing-field error.

## What is checked where

| Claim | Owner | Check |
| --- | --- | --- |
| The constitution's non-negotiable invariants, per criterion | [constitution](architecture/product-constitution.md), [conformance](contracts/constitution.md) | `cargo run -p symbiote-constitution --example constitution_report -- --check` |
| The domain's canonical entities and the noun that owns each | [domain](contracts/domain.md) | `cargo run -p symbiote-domain --example ontology_schema -- --check` |
| Which decisions are settled, provisional or under proof, and the pins each member declares | [architecture governance](contracts/architecture.md), [decisions](architecture/decisions.json) | `cargo run -p symbiote-architecture --example decisions` |
| What would settle the desktop-shell choice, and whether a run stands | [spike contract](architecture/spike-contracts.json) | the same map, plus the crate's own suite |
| The roadmap registry, its index and the accepted-decision coverage ledger | [integrity tooling](../planning/integrity/README.md) | `python3 planning/integrity/test_validate.py`, `python3 planning/integrity/coverage_ledger.py --check` |
| The CLI's published schema fixtures | [schemas](contracts/schemas) | `symbiote schema --check docs/contracts/schemas` |
| A driven binary's record covering every input its build read | [source records](../planning/integrity/README.md) | `planning/integrity/source_record.py --binary …` (see `.github/workflows/rust-contracts.yml`) |
| What a competitor claim rests on, how old it is, and what changed between snapshots | [competitor registry](../planning/research/README.md) | `python3 planning/research/registry.py --check`, `python3 planning/research/test_registry.py` |
| What that evidence means for our own axes: the class, the map to a requirement, and the decision on each pattern | [parity matrices](../planning/parity/README.md) | `python3 planning/parity/parity.py --check`, `python3 planning/parity/test_parity.py` |
| Which licence covers which artifact class, what each promise costs, and what every dependency and generated artifact is | [policy record](../planning/policy/README.md) | `python3 planning/policy/policy.py --check`, `python3 planning/policy/test_policy.py` |
| What a release must measure before it claims anything, what each gate permits and withholds, and what each release intentionally does not support | [release record](../planning/release/README.md) | `python3 planning/release/release.py --check`, `python3 planning/release/test_release.py` |

Each of those is held by a case in the component that owns it, and the driver
above is how this repository re-proves the architecture crate's own rules.

## The battery

```sh
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo fmt --all --check
python -m unittest discover -s planning -p 'test_legacy_importers.py' -v
python -m unittest discover -s planning/integrity -p 'test_*.py' -v
python -m unittest discover -s planning/housekeeping -p 'test_*.py' -v
python -m unittest discover -s planning/research -p 'test_*.py' -v
python -m unittest discover -s planning/parity -p 'test_*.py' -v
python -m unittest discover -s planning/policy -p 'test_*.py' -v
python -m unittest discover -s planning/release -p 'test_*.py' -v
python3 planning/integrity/revert_rules.py
```

The workspace runs on the declared minimum (Rust 1.85) and on stable; CI does
both. Every suite prints the counts it passed, and this document does not restate
them.

## What remains open

- **#173 — architecture governance.** The shell decision has a contract and rules
  that refuse a manufactured settlement, and one real run is now committed against
  it ([`desktop-shell.json`](proofs/results/desktop-shell.json)): a Wayland run on
  one applicable platform, in a partial run set that leaves the rest declared
  untested, so the choice is still `investigating`. What that run measured, what
  its instrument could not, and the artifacts behind every figure are the
  artifact's own — this file does not restate them; the run and how it was built
  are described in [the Linux shell record](proofs/linux-shell.md). The bar's content is
  written in three prose copies (ADR-0001's proof contract, the technology
  amendment, the Linux shell proof record); the copy that states its numbers — the
  ceilings the record writes beside its figures — is now compared with the
  contract's predeclared maximums by [the crate's `document`
  target](../crates/symbiote-architecture/tests/document.rs), which also requires
  every measurement the committed run observed to have a ceiling stated there. The
  other two copies restate the obligations rather than a threshold, and nothing
  compares their wording — with the one measured exception that the clause setting
  applicability is compared family for family: the answer carrying it, the
  obligation answering it and the declared platform data must name one set of
  platforms, so a contract applying to a platform the bar never named is refused
  naming both sides ([`architecture.md`](contracts/architecture.md)).
  A contract document carrying both old shapes at once is refused naming both moves:
  the field the loader cannot place is one of the moves it names, so the halfway
  document a partial rewrite leaves — `answers` arrived, `obligations` not yet gone —
  no longer reaches serde's field list. The message the shape before the bar moved
  printed — the removed field named first, the section's replacement buried in the
  list of expected fields — is held by a case in
  [the contract suite](../crates/symbiote-architecture/tests/spike_contracts.rs),
  which asserts the derive's own words for it rather than restating them here.
  **What #173 itself owed is now recorded in the tree:** every transition a
  decision went through is an entry in a hash-chained
  [history](architecture/journal.json) that replays back to what the ledger
  publishes, what each record rests on has an immutable digest-named copy in
  [the store](architecture/store), and every issue and requirement link is
  resolved against the issue program the roadmap side generates, with the index
  an issue-generation consumer asks for. What is left on this front is owned
  elsewhere and named rather than left pending: authenticated gate enforcement and
  transactional persistence and recovery (#180, #181), automatic research refresh
  and authoritative source verification of pinned facts (fetched at run time by
  #180 and #181, with the dossier format a fact is recorded in #188's and the
  refreshed registry that marks evidence stale #171's), the measured
  candidate run that would settle the shell choice (#38), and the System Graph query
  API that would serve these links to a graph consumer (#327). The refusals and the
  cases that drive them are in [`architecture.md`](contracts/architecture.md).
- **#38 — the shell proof.** Platform coverage, representative budgets, Preview
  authority isolation, signed updates, installer size and the rest of the
  contract's ceilings stay unmeasured, each blocked by a capability the fixture does
  not have; what each would take is stated once, in
  [the Linux shell record](proofs/linux-shell.md). The scope is
  [ADR-0001](architecture/adr-0001-technology-direction.md).
- **#170 and #36 — broader acceptance.** The machinery is in place and checked;
  the production capabilities the criteria name are owned by the issues they route.
- **#470 and #443 — roadmap mutations.** The registry, the index and the coverage
  ledger are validated read-only, and the one mutation path
  (`regenerate.py plan`/`apply`) refuses by name rather than leaving a rule to an
  operator's care — but it mutates only through a runner it is handed, whose every
  test is a recording fake, so no live regeneration has run and ambiguous
  create/lost-response recovery is not built.
- **Activation and release.** Worker execution needs the trust → consent →
  preventive-isolation chain (#174/#191/#218) and the runtime/environment
  integrations; the desktop workbench and both native and external agent execution
  are not implemented. Nothing here is release-ready.

## Provenance of this work

This repository was taken over on 2026-09-08 from `772fe844`: its checkout then
held planning/import scripts, encoded historical payloads and issue templates, and
no application, manifests, tests or accepted ADRs. The batch that started from it
recorded the accepted direction in the
[constitution](architecture/product-constitution.md) and
[ADR-0001](architecture/adr-0001-technology-direction.md); amended the issue bodies
whose stated candidates or scope that direction had moved — the Electron candidate
wording among them, removed from #38 and #336 — reading each body back after the
write; made the historical roadmap importers refuse before loading a payload,
disabled their workflow jobs and removed their issue-write permissions; and left a
dated amendment marker in
[the technology amendment](../planning/technology-amendment.md) recording its
precedence over the earlier baseline. Canonical owners from that point: #170 for
the constitution, #470 for roadmap integrity, #173 and #38 for governance and
proof. Captures taken then were local evidence and are not authority — recapture
rather than trusting one. The per-pass record this file replaces is in git history.

## Working facts worth carrying

- `/tmp` is never durable authority. A capture or a probe written there has to be
  recaptured when it matters; only committed artifacts and issue threads carry
  evidence.
- A probe run in a git worktree needs its own `CARGO_TARGET_DIR`. A shared one has
  made a worktree probe read the main checkout's artifacts and pass for the wrong
  reason.
- Separate-agent review, CI and the merge are separate gates. A recorded approval
  on a pull request is not an approval of the code, and a test that supplies an
  `Actor::Host` or an evidence record does not make enforcement authenticated.
- The historical roadmap importers are retired: they refuse before loading
  payloads, and old revisions or the audit branch must not be run as
  synchronizers.
