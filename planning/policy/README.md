# The licensing, trust and data-ownership record (#174)

The in-repo half of [#174](https://github.com/RepairYourTech/SymbioteIDE/issues/174) (A05): the
licensing boundaries an open-source tool that controls repositories, credentials, terminals and
third-party agents needs, the obligations each one carries, the stability level of every public
contract, the licence and provenance of everything this workspace links or generates, and the
assumption this project makes about every product it plans to integrate. One file is the record
(`policy.json`), one module owns every reading of it (`policy.py`), and one suite drives every
refusal (`test_policy.py`).

This suite runs in the `offline-validation` job, and the chain reading in
`planning/integrity/chain.py` — held by `test_python_floor.py` — refuses a job that runs this
repository's checks without it.

## The record

```
class        an artifact class (id, title), what it covers, what it does *not* cover, whether a
             licence is selected, and the owners still owing the selection
obligation   id, area, the requirement in the issue's own terms, and its status — built (with the
             path that shows it and the case that exercises it), delegated or pending (with the
             issue that owns it), or not applicable (with the rationale)
stability    the levels and what each promises, and one row per public contract document
dependency   a third-party dependency, the licence it is used under, and the class it lands in
generated    a committed generated artifact, the program that wrote it, the command that re-derives
             it, and the case that holds it
applicability  security, privacy, accessibility, performance and cross-platform, each marked
assumption   the licence, authentication and telemetry assumption about a product in #171's universe
```

As committed: **5** artifact classes, **17** obligations, **31** contract documents, **11**
dependencies, **7** generated artifacts and **10** universe products.

## The readings

```
python3 planning/policy/policy.py --check
python3 planning/policy/policy.py --classes
python3 planning/policy/policy.py --audit
python3 planning/policy/policy.py --generated
```

`--check` refuses the committed record on the first rule it breaks, naming its subject, then prints
what it read; it also prints that no class selects a licence, so the selection is visibly owed rather
than quietly absent. `--classes` prints the boundaries. `--audit` prints each product's
assumptions with the fields still unknown, the count of unknown fields, and the obligations this tree
has not built — the debt the issue's verification clause is about, readable rather than inferred.
`--generated` prints the provenance table. None of them writes anything, and none reaches the
network.

The record is read against the tree, not against prose: the dependency set and the class each
dependency lands in come from `crates/*/Cargo.toml`, the contract documents from
`docs/contracts/*.md`, the case names from the files that declare them, the products from
`planning/research/registry.json`, and the issue numbers from the generated program. A file this
record names that the tree does not carry is a refusal, in both directions.

## Refusals, each naming its subject

| Refusal | Why it exists |
| --- | --- |
| an artifact class the issue names and the record does not bound, or one bound twice | a licence decided by default is a licence nobody chose |
| a class stating no `covers` or no `boundary` | a boundary with no exclusion is a permission |
| a class selecting a licence while the tree carries no `LICENSE` stating it | the record would claim a licence the tree does not grant |
| a tree carrying a `LICENSE` (or `COPYING`) file no class selects | a licence granted by a file the record does not account for |
| a pending class with no owner or no reason | a deferral with no owner and no reason is a silence |
| a class, obligation, concern or product owned by an issue the program does not carry | a delegation to nothing |
| an obligation this issue's criteria are made of that no row accounts for | the obligation the issue names, dropped without being answered |
| a `built` obligation with no evidence path in the tree, or with a case no file declares | a claim where the tree has nothing — the fault the issue's own review clause names |
| an obligation both built and deferred, or built with an owner | a deferral and a delivery cannot be the same row |
| a `not_applicable` obligation with no rationale | the mark the issue asks to be given rather than assumed |
| a public contract document no stability row holds, or a row for a document the tree lacks | a contract whose level is assumed, or a level for a contract that does not exist |
| a level with no rule, or a level this reader does not know | a level that means whatever a reader assumes |
| `stable` or `deprecated` below a 1.0 workspace version | a promise of stability is a promise about a released version |
| a dependency the manifests declare with no row, or a row no manifest declares | the licence of what this workspace links, in both directions |
| a dependency filed under a class the manifests do not imply | the class is what its licence has to survive |
| a licence id this reader does not know, or one both permitted and denied | a boundary that cannot be read is not a boundary |
| a licence no class permits, or a denied-only licence | the distribution boundary refusing what it cannot honour |
| a dependency class with no permitted licence | a class a dependency can land in with nothing permitting it |
| a generated artifact the tree lacks, with a generator that is not a file, with a re-derivation naming a path the tree lacks, or held by a case no file declares | provenance that names nothing real |
| a concern marked nowhere, marked outside the marks this record states, required against an obligation that does not exist, or tracked by no issue | applicability assumed rather than reviewed |
| a universe product with no assumption recorded, a field left blank, an unknown naming no gap, or a value the registry holds no source for | the unstated assumption the issue's verification clause exists to find |

## What this does not claim

- **No licence is selected.** No `LICENSE`, `COPYING`, `NOTICE`, `CONTRIBUTING` or `SECURITY` file
  is present and `[workspace.package] publish = false`, so every class is `pending` with
  [#174](https://github.com/RepairYourTech/SymbioteIDE/issues/174) as their owner. This record
  states the boundaries the selection must fit and refuses a licence the tree cannot back; choosing
  the licence, the contributor terms and the trademark posture is a maintainer's decision, not a
  rule's, and it is the part of A05 that stays open.
- **The dependency licences are pinned data, not re-derived.** The `license` field of each
  dependency row was read from the crate's own manifest in the Cargo registry cache on 2026-09-20;
  a re-verification needs a resolver and the network, which the `offline-validation` job does not
  have. What is held offline is the *set*: every dependency the manifests declare is recorded, none
  the tree does not declare is, and each is filed under the class the widest table it appears in
  implies. A crate relicensing upstream, or a new transitive licence, is not caught here — the
  transitive closure is pinned by `Cargo.lock` (exact versions and checksums, 434 packages) and its
  licences are not enumerated, because this tree carries no per-package licence record and
  inventing one is what the issue forbids. That half is
  [#434](https://github.com/RepairYourTech/SymbioteIDE/issues/434)'s (signed artifacts, SBOM and
  supply-chain provenance).
- **Provenance holds what is declared, not the absence of undeclared writes.** A generated file
  placed in a root no row names is not caught: the record holds each declared artifact's writer, its
  re-derivation and its case, and this tree carries no marker inside a generated artifact (and
  `docs/proofs/results/` is not to be re-recorded), so a writer that must register does not exist.
- **A `built` status is held by a name, not by a passing test.** The case is required to be
  *declared* by a file in this tree; whether it asserts the right thing is the case's own business,
  the same boundary the architecture contract's document case states.
- **The runtime audit states the gap rather than filling it.** Every one of #171's universe
  products is recorded `UNKNOWN` for licence, authentication and telemetry with the gap named per
  field,
  because the registry holds no such claim for any of them and asserting one here would restate a
  product fact the registry owns. The rule that a stated value needs a source the registry holds is
  implemented and driven in this suite; the retrieval itself is
  [#171](https://github.com/RepairYourTech/SymbioteIDE/issues/171)'s registry refresh. The audit's
  finding is therefore precise: integrating any of them currently depends on an assumption this
  tree states as unknown rather than one it can stand behind.
- **Applicability is marked, not proven.** Security and privacy are `required` against the consent
  and no-exfiltration obligations this tree does build; accessibility, performance and cross-platform
  are `separately_tracked` to [#358](https://github.com/RepairYourTech/SymbioteIDE/issues/358),
  [#375](https://github.com/RepairYourTech/SymbioteIDE/issues/375) and
  [#435](https://github.com/RepairYourTech/SymbioteIDE/issues/435). Those issues' own obligations are
  theirs; nothing here measures them.
- **Neighbouring owners.** The threat model and adversarial evidence are
  [#191](https://github.com/RepairYourTech/SymbioteIDE/issues/191)'s; the extension SDK, permission
  model, publisher identity, signing and revocation are
  [#431](https://github.com/RepairYourTech/SymbioteIDE/issues/431)'s; telemetry storage, retention,
  privacy and optional upload are [#459](https://github.com/RepairYourTech/SymbioteIDE/issues/459)'s
  with the opt-in surface in [#374](https://github.com/RepairYourTech/SymbioteIDE/issues/374);
  release signing, installers and update channels are
  [#434](https://github.com/RepairYourTech/SymbioteIDE/issues/434)'s. Each was read before it was
  named.
