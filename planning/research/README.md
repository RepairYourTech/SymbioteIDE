# The competitor registry (#171)

A dated, source-backed registry of the products Symbiote is compared against, so the architecture is
grounded in what the market actually supports rather than in memory or marketing. One file is the
registry (`registry.json`), one directory holds the historical states it can be diffed against
(`snapshots/`), and one module owns every reading of both (`registry.py`).

This is the in-repo half of [#171](https://github.com/RepairYourTech/SymbioteIDE/issues/171): the
schema, the evidence discipline, the states, staleness, snapshots and deltas, and the focused
refresh. The research itself — the dossiers, and keeping them current — is owned elsewhere and named
under *What this does not claim*. This suite runs in the `offline-validation` job, and the chain
reading in `planning/integrity/chain.py` — held by `test_python_floor.py` — refuses a job that runs
this repository's checks without it: dropping the step, moving it to another job, putting it behind a
condition that cannot hold, or letting another command own its exit status reds there by name.

## The record

```
product   id, name, class, an optional renamed_from, claims, and a gap where it has none
claim     capability (a taxonomy id), state, observation, and either evidence or a research gap;
          a capability the taxonomy grades also carries grade and transport
evidence  kind, source, retrieved_at, confidence, excerpt
watch     capability → the requirement keys and decision ids it informs, and whether locked
```

An `observation` is what the product does in the claimant's own words; it is not a paraphrase of the
evidence and not a score. Evidence is a URL or repository path, the date it was retrieved, what it
actually says, and how much weight it carries. `renamed_from` is how a product or a capability that
changed its name keeps the identity the history uses.

## The states

| State | What it claims |
| --- | --- |
| `VERIFIED_CURRENT` | current behaviour, on authoritative evidence inside the capability's window |
| `ANNOUNCED` | the product says so; nothing else does |
| `EXPERIMENTAL` | shipped behind a flag, a beta or an early-access programme |
| `INFERRED` | secondary reporting only, with the gap that would settle it |
| `STALE` | evidence outside the capability's window; the state a stale claim must move to |
| `UNKNOWN` | nothing was retrieved, with a precise research gap rather than a blank |

Sources are `official_site`, `official_docs`, `official_repo`, `official_changelog`, `official_blog`
— the authoritative kinds — and `community`, which may supplement a claim but can never carry one on
its own. A harness integration is graded `terminal_compatible`, `detected`, `managed`, `structured`,
`orchestrator_grade` or `certified`, over a transport that is `pty`, `sdk`, `app_server`, `rpc`,
`jsonl`, `acp` or `none`: a terminal is a transport, and a terminal is not a structured integration.

## The readings

```
python3 planning/research/registry.py --check
python3 planning/research/registry.py --delta 2026-08-15 registry
python3 planning/research/registry.py --refresh runtime --plan plan.json
```

`--check` reads the committed registry and every snapshot and refuses on the first rule broken,
naming its subject; it writes nothing. `--delta` prints what changed between two states, which
capabilities a `watch` row binds to a requirement or a decision, which of those decisions are locked,
and which changed capabilities no row watches. `--refresh` reads a research plan — capability ids,
with the product they belong to — and refuses one that leaves the dimension it names or re-observes a
capability a locked row binds to a settled decision.

## Refusals, each naming its subject

| Refusal | Why it exists |
| --- | --- |
| a claim state outside the list, or an unknown source kind, confidence, grade, transport or product class | a registry that cannot be read is not a registry |
| a `VERIFIED_CURRENT` claim with no authoritative evidence, or with only community evidence | marketing and forums are not current fact |
| `VERIFIED_CURRENT` on evidence older than the capability's window | a claim that cannot be current must read `STALE` rather than keep masquerading |
| a claim with no authoritative evidence and no research gap | an unknown with no plan to settle it is an omission |
| `INFERRED` or `UNKNOWN` with authoritative evidence behind it | which state a claim is in has to follow its evidence |
| evidence missing a source, a retrieval date, an excerpt or a confidence, or retrieved after the registry was observed | a claim is only as good as what stands behind it |
| a capability outside the taxonomy, or one a product claims twice | similar names are normalized by semantics, once |
| a grade for a capability the taxonomy does not grade, an unknown grade, an unknown transport | the ladder is the taxonomy's, not a claim's |
| `structured` or deeper over `pty`, or a deep grade with no authoritative evidence of the interface | a terminal launch is not evidence of structured integration |
| `terminal_compatible` over a structured transport | the grade and the interface have to agree |
| a product in the universe with no dossier, or a dossier for a product outside it | the universe is the list, both ways |
| a dossier with no claim and no research gap | an empty dossier is not a researched one |
| a rename that replaces something still present | a rename replaces what it names |
| a rename of a name no snapshot carries | the history is what makes an identity stable |
| a taxonomy capability with no staleness window | freshness needs a window to be measured against |
| a snapshot whose observed date is not its own name, or a snapshot that breaks a rule | history is read under today's rules, not trusted |
| a refresh plan that leaves the dimension it names, or re-observes a locked row | a focused pass is focused, and a settled decision is not re-opened by research |

`--check` also refuses a registry or snapshot whose schema version this reader does not replay, and
will not read a snapshot that is not there rather than treating it as empty.

## The cases each refusal rests on

`test_registry.py` drives every one of them alone and in both directions, over the committed
registry and in a temporary copy of `snapshots/`: the committed registry and every snapshot pass
(`test_the_committed_registry_and_every_snapshot_pass_every_rule`), and each refusal above has a case
that breaks exactly it and an honest edit of the same shape that stays green
(`HonestEdits`). The readings the issue asks for have their own cases: a rename is one change rather
than a loss beside a gain, a state that moved is reported with both ends, a change a `watch` row
binds appears against the row, a locked row is reported locked rather than reported changed, a change
no row watches stays visible, a snapshot reconstructs the claims it recorded, and a refresh leaves
every other dimension and every identity alone.

## What this does not claim

- **The dossiers are a seed, not the market.** Orca, Traycer, BridgeMind One and T3 Code carry
  atomic claims, and only where an official source was retrieved on 2026-09-20; the other universe
  entries declare a research gap and nothing else. The launch-baseline universe is not researched,
  and #171's dossiers for it are owed. No case reads this file: the registry's own records are what
  the suite checks, so a count written here is a copy nothing refuses, and the counts that stood
  beside these names are gone rather than left to drift.
- **Excerpts are what was retrieved, not a full reading.** Each excerpt is the text the source
  returned for the query that found it, recorded on the date it was retrieved. Nothing here fetched
  a page in full or judged whether the product delivers what it says.
- **Nothing is discovered or refreshed automatically.** `--refresh` reads a plan; no scheduled job,
  no crawler and no source verification exists in this repository. Continuous discovery and
  capability extraction belong to [#249](https://github.com/RepairYourTech/SymbioteIDE/issues/249),
  and the parity and gap matrices that consume this registry are
  [#172](https://github.com/RepairYourTech/SymbioteIDE/issues/172)'s.
- **Two authoritative sources that disagree are recorded, not arbitrated.** The registry holds a
  kind and a confidence per source; nothing compares two official sources with each other, so a
  conflicting-source case is a case about a claim's own evidence rather than a reconciliation.
- **A `watch` row is a claim, not a link this repository resolves.** It names the requirement keys
  and decision ids a capability would inform; whether a change really touches them is a reading the
  ledger side owns, and a locked decision's own record lives in `docs/architecture/decisions.json`.
