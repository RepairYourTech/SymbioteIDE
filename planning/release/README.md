# Release metrics, gates and non-goals

The in-repo half of [#175](https://github.com/RepairYourTech/SymbioteIDE/issues/175)
(A06, wave 0): what a release must be able to measure before it may claim anything, what each
release gate permits and withholds, what each release intentionally does not support, and how a
candidate's evidence reaches the tree. One record is the release facts (`release.json`), one module
owns every reading of it (`release.py`), and one suite drives every refusal (`test_release.py`).

As committed: **8** metrics (2 measured, 6 delegated), **4** gates, **7** non-goals, **2** registers
(24 invariants, 18 parity rows), **5** concerns.

## What the record holds

**The metrics the issue names, once each.** `quality`, `task-success`, `context-efficiency`,
`graph-precision-recall`, `recovery`, `latency`, `mobile-control` and `delivery` are each defined
with what they measure and their unit. A metric's `source` is where its value comes from: `measured`
names the case in this tree that produces it, `delegated` names the issue that owns it and why this
record does not restate the definition. Two are measured here — `recovery` by
`crates/symbiote-architecture/tests/audit.rs`'s
`the_committed_history_recovers_what_the_ledger_publishes`, and `delivery` by the chain case that
holds `.github/workflows/roadmap-integrity.yml` — and six are delegated: #456 (quality), #435 (task
success), #329 (context efficiency), #385 (graph precision/recall), #375 (latency) and #407 (mobile
control). Both measured metrics carry a stated narrowing with the issues that own the wider metric.

**Four release gates** — `technical-preview`, `alpha`, `beta`, `stable` — each with what it permits,
what it withholds, the metrics it requires, the non-goals that bound it and the registers it tracks.
Every gate requires `golden-path`: a candidate that cannot name the case and the committed trace
carrying a client request to the evidence it publishes does not pass. A gate is not a status: no
release is claimed met, and `--gates` prints what each one requires so readiness is readable rather
than inferred.

**Seven non-goals**, each naming its release, what it does not support, why, and the issue that owns
the gap where one does.

**The golden path**: the requirement above, the committed artifact
`docs/proofs/results/desktop-shell.json`, and the case that reads it
(`crates/symbiote-architecture/tests/spike_contracts.rs`'s
`the_committed_contract_passes_every_rule`). The full cross-platform laboratory is #435's.

**Two registers regressions are tracked against**, read rather than restated: the constitutional
invariants from `docs/contracts/constitution-report.json` and the competitor-parity rows from
`planning/parity/matrices.json`. Each register names the case that holds its own artifact, and every
figure this file writes beside a noun — here, in the figures above, and in the limits below — is held
against those files by one case, so a copy that moves without the others reds by name rather than
going unread.

**The concerns reviewed** are the applicability marks the licensing and trust policy already
carries, read from `planning/policy/policy.json`: five concerns, each `required`,
`not_applicable` or `separately_tracked` there. This record reviews them; it does not mark them.

## Readings

```
python3 planning/release/release.py --check      # refusals, exit 1 on any
python3 planning/release/release.py --metrics    # each metric and where its value comes from
python3 planning/release/release.py --gates      # each gate, its requirements and the marks
python3 planning/release/release.py --registers  # each register and what it carries
```

## Refusals

| Refusal | What it stops |
| --- | --- |
| a metric the issue lists is missing, repeated or invented | a claim the issue makes that no row states |
| a metric with no definition or no unit | a name standing where a measurement should be |
| a `measured` metric whose evidence path is not in the tree, whose case no file declares, or whose case lives in another file | a number attributed to nothing |
| a delegated metric with no owner in the program, or no reason | a definition restated instead of owned |
| a gate requiring an unknown metric, requiring nothing, or not requiring the golden path | an unbounded or unearned gate |
| a gate naming an unstated non-goal, tracking an unknown register, or tracking none | a boundary or regression reading that bounds nothing |
| a metric required by no gate, or a register tracked by no gate | scope that no release promises |
| a non-goal with an unknown release, no reason, or an owner outside the program | a boundary asserted rather than recorded |
| a golden path with a missing artifact, an undeclared case, or no owner | a candidate oweing evidence that does not exist |
| a register whose source is missing, carries nothing, or whose holding case no file declares | a regression register that cannot bound a change |
| a concern reviewed that the policy record does not mark, or one it marks that is not reviewed | applicability assumed rather than checked |
| a concern marked outside the three marks, or tracked by an issue the program lacks | a mark that cannot be read |

## Limits, with their measurement

- **Six of the eight metrics are delegated, and the two measured ones are narrowed.** This tree
  measures the architecture evidence chain's recovery and the delivery of its own checks
  (`--registers` reads 24 invariants and 18 parity rows); it measures no release candidate's
  quality, task success, context efficiency, graph precision/recall, latency or mobile control,
  and `--metrics` prints the owner of each. The narrowings are named on the metrics themselves.
- **No release candidate exists and no gate is claimed met.** The gates state what a candidate must
  present and a gate that omits the golden path is refused; they are not evidence that anything has
  passed one.
- **The golden path's full laboratory is #435's**, and its committed artifact predates any release
  candidate: the desktop-shell dossier is the nearest evidence this tree carries, not a
  production path for a shipped release.
- **The registers are read from their owners, never restated.** A regression the constitution
  report or the parity matrices do not carry cannot be tracked here, and a register whose artifact
  carries nothing is refused.
- **Applicability belongs to the policy record.** This record reviews the five marks
  `planning/policy/policy.json` carries and refuses a reviewed concern the policy record does not
  mark, or one it marks that is not reviewed; it does not select a licence, a level or a mark.
- **The figures above are held.** `test_release.py` reads this file and refuses a figure that
  disagrees with the record, the report or the matrices, so a count that nothing carries cannot go
  stale silently.
- **The standing boundaries stand**: the comparison-vacuity class the guard's own comparisons
  carry, the coordinated multi-file removal as the external root of trust, and no claim here about
  a release that has not shipped.
