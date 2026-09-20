# The parity and gap matrices (#172)

What [#171](../research/README.md)'s evidence means for Symbiote's own axes: one classified row per
capability the registry carries, the end-to-end workflows those capabilities sit in, the
borrow/adapt/reject/research decision on each pattern worth deciding about, and the claims of ours a
competitor turned out to match. One file is the matrices (`matrices.json`), one module owns every
reading of them (`parity.py`), and one suite drives every refusal (`test_parity.py`).

This is the in-repo half of [#172](https://github.com/RepairYourTech/SymbioteIDE/issues/172). The
matrices judge; the registry proves. Nothing here restates a competitor fact — every competitor datum
carries is a pointer at a claim the registry already holds, so a matrix cannot assert something the
registry's evidence discipline never admitted. This suite runs in the `offline-validation` job, and
the chain reading in `planning/integrity/chain.py` — held by `test_python_floor.py` — refuses a job
that runs this repository's checks without it.

## The record

```
row       capability (a registry taxonomy id), matrix, class, stance, evidence and the map to us
evidence  product + capability: a claim the registry holds, about the axis the row classifies
workflow  id, name, steps (each an owner and an action), and the handoffs with their gaps
pattern   the pattern, its capability axis, BORROW|ADAPT|REJECT|RESEARCH, its source claim, its
          rationale, the requirements or issues it touches, its risk, and whether it would move a
          locked decision
revision  the capability, what we claimed, the claim that matched it, and the revised stance
```

A row's `class` is one of `TABLE_STAKES`, `PARITY_GAP`, `DIFFERENTIATOR`, `INFRASTRUCTURE`,
`EXPERIMENT`, `REJECTED_PATTERN`, `ADJACENT_OPPORTUNITY` or `UNKNOWN` — the price of entry, work we
have not closed, something of ours a competitor does not have, an enabler judged by what it enables,
an experiment that is not a commitment, a durable decision against copying, an opportunity derived
from more than one product's behaviour, and a stated research question. A `pattern` records what we
decided about a competitor practice; a `revision` keeps "we deliberately rejected this" distinguishable
from "we never noticed it".

As committed: **18** rows over **7** matrices, **6** workflows, **10** patterns and **2** revisions,
with one pattern recorded as touching a locked decision.

## The readings

```
python3 planning/parity/parity.py --check
python3 planning/parity/parity.py --matrices runtime
python3 planning/parity/parity.py --movement 2026-08-15 registry
```

`--check` refuses the committed file on the first rule it breaks, naming the row, pattern, workflow or
revision it is about, and then prints what it read; a pattern that would move a locked decision is
printed for review rather than applied. `--matrices` prints one matrix's rows, or all of them with
their counts. `--movement` reads the registry's own `--delta` between two states and reports which
rows the change touches, which of those rows are locked, which were recomputed on a changed claim,
and which changed capabilities no `watch` row binds. It writes nothing.

## Refusals, each naming its subject

| Refusal | Why it exists |
| --- | --- |
| a row whose capability the registry does not carry, or a capability classified twice | the matrices cover the registry's axes, once each |
| a capability the registry carries and no row classifies | an axis nothing looked at is the silent loss this file exists to prevent |
| a class, decision, handoff kind or matrix outside the vocabulary | a comparison that cannot be read is not a comparison |
| a row with no stance, a pattern with no rationale or no risk, a revision that states neither `was` nor `revised` | a judgement with nothing said about it is an omission |
| evidence pointing at a claim the registry does not hold, or at another axis | a matrix may not assert a competitor fact the evidence discipline never admitted |
| `DIFFERENTIATOR` or `PARITY_GAP` with no evidence, or resting on a claim reading `UNKNOWN`/`STALE` | a claim about us without current evidence about them is not a claim |
| `ADJACENT_OPPORTUNITY` read off one product | that is the product's feature, not an opportunity of ours |
| `UNKNOWN` with no research question | an unknown with no plan to settle it is a blank |
| `REJECTED_PATTERN` with no `REJECT` decision on the same axis | a rejection nothing records is rediscovered |
| a pattern that decides about a locked capability and claims to move nothing | settled scope is not changed silently, and a decision that would change it has to say so |
| a pattern whose source is not a claim, or whose issue the program does not carry | the decision rests on evidence and lands on work that exists |
| a row, pattern or gap mapping to no requirement, issue or deferral | a finding nothing owns is a thought |
| a workflow shorter than an end-to-end pass, with an owner outside the universe, with no handoff, or with a handoff at a step it does not have | a workflow is a chain, not a feature list |
| no cross-product handoff anywhere, or no revision at all | the analysis has to leave the product, and a claim of ours has to have been tested |
| a movement naming a state the registry does not carry | a comparison against a state that does not exist |

## The cases each refusal rests on

`test_parity.py` drives every one of them alone, over a copy of the committed file mutated in memory,
and in both directions wherever a rule can be read both ways: the refusal above, and an honest edit of
the same shape that stays green (a reworded stance, a renamed workflow, a reordered file, a second
deferral). The readings have their own cases: a registry change touches the rows resting on it, a
locked row is reported locked rather than recomputed, a changed capability no `watch` row binds is
reported rather than dropped, an unknown state is refused, and the README's own figures are the ones
the file measures.

## What this does not claim

- **The classes and stances are judgement, held to structure.** The rules hold that every axis is
  classified once, that a claim about us rests on current evidence about them, that a rejection keeps
  its decision and that a workflow names where it leaves the product. They cannot hold whether a
  stance is *right*: nothing here runs a competitor or measures Symbiote's own behaviour beyond the
  requirements and decisions it cites.
- **The evidence is #171's, and thin.** Only four products carry claims in the registry, so most of
  the eighteen axes are `UNKNOWN` with a research question rather than a market level — including the
  whole knowledge matrix, which no codebase-knowledge product has a claim for. Criterion 8 of #172
  (Living Developer Knowledge table stakes versus differentiators) cannot be met from this registry.
- **No product fact is repeated here.** Where a row needs to say what a competitor does, it points at
  the registry claim; the reading resolves it. A registry claim turning `STALE` turns the
  differentiation claim resting on it into a refusal.
- **Nothing is published automatically.** An accepted gap becomes a requirement or an issue through
  the normal decision and impact path; this file never edits locked scope, and the one pattern that
  would move it is recorded for review. Which requirement an accepted gap should become is a
  maintainer's decision, not a rule's.
- **The workflow handoffs are ours to see, not theirs to confirm.** A handoff is recorded where this
  product's plan leaves the user, from the registry's claims about what the other product does; the
  competitor's own workflow is only as good as the claim cited for it.
