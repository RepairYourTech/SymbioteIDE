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

## How a claim is held here

Five habits, each of them checked rather than asserted:

- **One owner per fact.** A rule about a record lives with the records, a rule
  about the tree with the artifacts, a rule about proof with the contracts, and a
  rule about a document with the component that owns the document.
- **A refusal rule is proved by reverting it.**
  [`planning/integrity/revert_rules.py`](../planning/integrity/revert_rules.py)
  holds one row per rule of the architecture governance crate and the contract
  document: it removes the rule alone, watches the case that holds it fail, and
  restores the file byte-identically.
  `planning/integrity/test_revert_rules.py` holds that table to the tree — every
  anchor is held once, every named case is one a suite runs — so a moved anchor or
  a renamed case fails in a second instead of becoming a row the driver skips.
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
python3 planning/integrity/revert_rules.py
```

The workspace runs on the declared minimum (Rust 1.85) and on stable; CI does
both. Every suite prints the counts it passed, and this document does not restate
them.

## What remains open

- **#173 — architecture governance.** The shell decision has a contract and rules
  that refuse a manufactured settlement, and no candidate run exists yet, so the
  choice is `investigating` and nothing has been measured. The bar's content is
  still written in three prose copies (ADR-0001's proof contract, the technology
  amendment, the Linux shell proof record) against the one the contract answers.
  A contract document carrying both old shapes at once is refused by serde's field
  list rather than by name. The message the pre-#602 wrapper printed is not
  reproducible from this tree, which the case that quotes it says in place.
- **#38 — the shell proof.** Platform coverage, representative budgets, Preview
  authority isolation, signed updates, installer size and the rest of the
  contract's ceilings stay unmeasured; see [the Linux shell record](proofs/linux-shell.md)
  and [ADR-0001](architecture/adr-0001-technology-direction.md).
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
