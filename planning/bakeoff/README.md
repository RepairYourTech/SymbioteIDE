# The shell bake-off's acceptance record (#38)

The in-repo half of [#38](https://github.com/RepairYourTech/SymbioteIDE/issues/38)
(FND-05, wave 0): its own acceptance criteria, each routed to the obligation, the
predeclared measurement or the owner issue that carries it, and what the committed runs
currently show for it. One record is the routing (`bakeoff.json`), one module owns every
reading of it (`bakeoff.py`), and one suite drives every refusal (`test_bakeoff.py`).

As committed: **8** criteria over **18** carriers: 10 obligation carriers, 5 measurement
carriers and 3 routings to 2 owner issues; 10 of the contract's 14 obligations and 5 of its
11 measurements named; 1 of 4 platforms measured, 3 declared untested, over 2 runs.

## What the record holds

**The issue's own acceptance criteria, once each.** The count is not this file's: the issue
program records how many acceptance items #38's body states
(`acceptance_items` in `planning/integrity/generated/registry.json`), and the record's rows
are held to that count in both directions — a criterion dropped, added or renumbered here is
refused by name. A row carries the issue's own number and what carries it, and nothing else:
the words are the issue's, read at the issue, because no committed artifact in this tree holds
them (see the limits below).

**What carries each criterion**, in the kinds its contract's `answers` use, one level up from
the clauses they answer, plus a routing for what the workload does not carry:

- an `obligation` the contract declares, which a run of it must exercise;
- a `measurement` it predeclares, which a run must observe inside its ceiling;
- an `elsewhere` naming the issue that owns a part of the criterion the contract's workload
  does not carry, with the reason it is routed rather than restated.

Every carrier is read back against its owner: an obligation or measurement the contract does
not carry, or an issue the program does not carry, is refused. So a quotation in this file
cannot drift from the contract, and a routing cannot name an owner that does not exist.

**The join between the artifacts that settle the choice.** The record names the
contract document and its identity, the ledger record that binds that contract to this issue,
the dossier the runs wrote and the shell record those runs are read against. The contract must
settle the decision the record names, that decision must be blocked by #38 and proved by that
contract, the dossier must be a result measured against it with a run behind it, and the shell
record must name every measurement the contract predeclares — so this record cannot point at
another choice's contract, another contract's runs, a decision somebody else is blocked on, or
a record that leaves a ceiling on the bar unaccounted for.

## Readings

```
python3 planning/bakeoff/bakeoff.py --check      # refusals, exit 1 on any
python3 planning/bakeoff/bakeoff.py --criteria   # each criterion, its carriers, and what the runs show
python3 planning/bakeoff/bakeoff.py --dossier     # runs, platforms and how far each carrier is backed
```

`--criteria` prints what stands behind each carrier from the dossier rather than from this
file: an obligation a committed run attests, a measurement a run observed and at what value,
or the owner a part of the criterion is routed to. Nothing here reports a criterion met.

## Refusals

| Refusal | What it stops |
| --- | --- |
| a criterion dropped, added or renumbered against the program's own count | a claim the issue makes that no row answers |
| a row carrying a field beside its number and carriers | a claim beside the routing that no rule in this record reads |
| a criterion carried by nothing | a number standing where the issue's own routing should be |
| a carrier naming two kinds at once, or one carrier stated twice in a row | one fact standing where two are owed, or a routing padded with a repeat |
| an obligation the contract does not declare, or a measurement it does not predeclare | a carrier that resolves to nothing |
| a routing to an issue the program does not carry, or with no reason | an owner that cannot own it, or a boundary asserted rather than recorded |
| a routed path this tree does not carry | a join to an artifact that does not exist |
| a contract settling another decision, or a ledger record for another decision | a routing through another choice's contract |
| a choice blocked by another issue, or proved by another contract | a join that is not this issue's |
| a dossier measured against another contract, or recording no run | evidence that belongs to something else, or to nothing |
| a shell record naming no measurement the contract predeclares | a figure on the bar that no reader of that record can find |

## Limits, with their measurement

- **The criteria's words are not carried here, because nothing in this tree holds them.**
  The program records how many acceptance items #38's body stated and pins the body's hash
  (`body_sha256`), and the only committed body capture reduces bodies — its own scope says it is
  not an exact-body provenance claim, and #38's captured body is 194 bytes with zero checkbox
  lines — so the words are not in this tree to be read back. The rows therefore state the
  number and the carriers, a field beside them is refused, and a reader reads each criterion's
  words at the issue; what is held is the count, the routing and that nothing unheld is carried.
- **A carrier is a claim about coverage, not a proof of it.** That an obligation's wording
  covers the criterion it is routed to is a reading, exactly as the contract's own `answers`
  say of their clauses; what is held is that the carrier exists, is declared once, and
  resolves. A routing carries part or all of a criterion to the issue that owns it, a whole
  claim may belong to an owner this repository does not implement, and `--criteria` prints
  them.
- **The dossier is partial, and this record claims nothing settled by it.** Of the carriers
  this record names, the committed runs attest 2 of the 14 obligations and observe 3 of the
  11 measurements the contract predeclares, so most carriers read as backed by no committed
  run yet; #38 stays open, and what a run would need to observe each unmeasured figure is
  stated in [the Linux shell record](../../docs/proofs/linux-shell.md).
- **The ceilings and the obligation wording belong to the contract.** This file quotes the
  obligations by their contract text and reads them back, so a reworded obligation reds here;
  it does not restate a threshold, a platform list or a stop condition, and the bar itself —
  which section of ADR-0001 the choice is settled against — is the ledger's and the crate's.
- **This suite's cases are held by presence**, as the registry's, parity's, policy's and
  release's are: the chain reading holds that the job runs the suite, not what is inside it.
  The rule driver's case accounting walks the files its own rows state their refusals in, and
  none of them is this suite — so deleting a case here is caught only by the diff.
- **The standing boundaries stand**: the comparison-vacuity class the guard's own comparisons
  carry, the coordinated multi-file removal as the external root of trust, and no claim here
  about a shell that has not been selected.
