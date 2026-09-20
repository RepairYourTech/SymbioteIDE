#!/usr/bin/env python3
"""The parity and gap matrices: what #171's evidence means for our own axes, and every reading of it.

The registry owns what a competitor claims and how old the claim is. This module owns the *other*
half of the comparison, and nothing here restates a competitor fact: a row classifies one of the
registry's capability axes, and every competitor datum it carries is a pointer to a claim the
registry already holds — `(product, capability)` — so a matrix can never assert a product fact the
evidence discipline never admitted.

A row's `class` is one of `TABLE_STAKES`, `PARITY_GAP`, `DIFFERENTIATOR`, `INFRASTRUCTURE`,
`EXPERIMENT`, `REJECTED_PATTERN`, `ADJACENT_OPPORTUNITY`, `UNKNOWN`: table stakes are the price of
entry, a parity gap is work we have not closed, a differentiator is ours and needs current evidence
about *them*, infrastructure is judged by what it enables, an experiment is not a commitment, a
rejected pattern is a durable decision against copying, an adjacent opportunity is derived from more
than one product's behaviour, and an unknown is a stated research question rather than a blank.

A `pattern` records the decision on a competitor pattern — `BORROW`, `ADAPT`, `REJECT`, `RESEARCH` —
with its rationale, the requirements or issues it touches, its risk, and whether carrying it out
would move a locked decision. A `revision` records a claim of ours a competitor turned out to match,
so "we deliberately rejected this" stays distinguishable from "we never noticed it".

The readings:

    python3 planning/parity/parity.py --check
    python3 planning/parity/parity.py --matrices runtime
    python3 planning/parity/parity.py --movement 2026-08-15 registry

`--check` refuses the committed file on the first rule it breaks, naming its subject; `--matrices`
prints a matrix's rows; `--movement` maps a registry change onto the rows it touches and reports
locked rows separately, writing nothing.
"""
from __future__ import annotations

import argparse
import json
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parents[2]
MATRICES = ROOT / "planning/parity/matrices.json"
PROGRAM = ROOT / "planning/integrity/generated/registry.json"
LEDGER = ROOT / "docs/architecture/decisions.json"
RESEARCH = ROOT / "planning/research"
sys.path.insert(0, str(RESEARCH))

import registry as research  # noqa: E402  (the registry owns what a claim is)

SCHEMA_VERSION = 1
# What a row's axis can be relative to us. Declared here, once: the file states rows, not vocabulary.
CLASSES = (
    "TABLE_STAKES",
    "PARITY_GAP",
    "DIFFERENTIATOR",
    "INFRASTRUCTURE",
    "EXPERIMENT",
    "REJECTED_PATTERN",
    "ADJACENT_OPPORTUNITY",
    "UNKNOWN",
)
# What we may decide about a competitor pattern.
DECISIONS = ("BORROW", "ADAPT", "REJECT", "RESEARCH")
# A handoff out of the product, or an unresolved boundary inside our own plan.
HANDOFF_KINDS = ("cross_product", "internal")
# The class a row may not carry without evidence behind it: claiming we are ahead of a competitor who
# is ahead of us is worse than claiming nothing.
EVIDENCED = ("DIFFERENTIATOR", "PARITY_GAP")
# The class a row is allowed to be when the registry has nothing: a question, not a stance.
OPPORTUNITY = "ADJACENT_OPPORTUNITY"


class Refused(Exception):
    """A reading that cannot be made: the file, the program or the ledger says something it cannot."""


def read(path: pathlib.Path) -> dict:
    try:
        return json.loads(path.read_text())
    except FileNotFoundError as error:
        raise Refused(f"no file at {path}") from error
    except json.JSONDecodeError as error:
        raise Refused(f"{path} is not readable JSON: {error}") from error


class Matrices:
    """The committed matrices, with the registry's claims indexed once."""

    def __init__(self, document: dict, origin: str, registry: research.Registry):
        self.document = document
        self.origin = origin
        self.registry = registry
        self.schema_version = document.get("schema_version")
        self.observed_at = document.get("observed_at", "")
        self.matrices = [name for name in document.get("matrices", []) if isinstance(name, str)]
        self.rows = document.get("rows", [])
        self.workflows = document.get("workflows", [])
        self.patterns = document.get("patterns", [])
        self.revisions = document.get("revisions", [])
        # The claims the registry holds, by subject: what a matrix row may point at.
        self.claims = {(product, claim.get("capability")): claim
                       for product, claim in registry.claims()}
        self.locked = {row.get("capability") for row in registry.watch if row.get("locked")}

    def holds(self, product: str, capability: str) -> bool:
        return (product, capability) in self.claims

    def claim(self, product: str, capability: str) -> dict | None:
        return self.claims.get((product, capability))

    def class_of(self, capability: str) -> str | None:
        for row in self.rows:
            if row.get("capability") == capability:
                return row.get("class")
        return None


def program_entries() -> dict:
    document = read(PROGRAM)
    return {
        "keys": {entry.get("key") for entry in document.get("entries", [])
                 if entry.get("kind") != "reference"},
        "issues": {entry.get("number") for entry in document.get("entries", [])},
    }


def ledger_ids() -> set[str]:
    document = read(LEDGER)
    return {decision.get("draft", {}).get("id") for decision in document.get("decisions", [])
            if decision.get("draft", {}).get("id")}


def problems(matrices: Matrices, program: dict, ledger: set[str]) -> list[str]:
    """Every refusal the committed matrices must survive, each naming its subject."""
    found: list[str] = []
    rows = matrices.rows
    by_capability: dict[str, dict] = {}

    if matrices.schema_version != SCHEMA_VERSION:
        found.append(f"the matrices declare schema {matrices.schema_version!r}, and this reader "
                     f"replays {SCHEMA_VERSION}")
    if not matrices.matrices:
        found.append("the file names no matrix, so no row can be read against one")

    for row in rows:
        subject = row.get("capability") or "a row naming no capability"
        if subject in by_capability:
            found.append(f"{subject} is classified twice")
            continue
        by_capability[subject] = row
        if subject not in matrices.registry.capabilities:
            found.append(f"{subject} is not a capability the registry carries")
        if row.get("matrix") not in matrices.matrices:
            found.append(f"{subject} is filed under {row.get('matrix')!r}, which is not a matrix "
                         f"this file names")
        if row.get("class") not in CLASSES:
            found.append(f"{subject} is classified {row.get('class')!r}, which is not a class of "
                         f"comparison this reader replays")
        if not str(row.get("stance", "")).strip():
            found.append(f"{subject} states no stance: a classified axis with nothing said about it "
                         f"is an omission")
        found.extend(evidence_problems(matrices, row, subject))
        found.extend(mapping_problems(row, subject, program, ledger))

    for capability in sorted(set(matrices.registry.capabilities) - set(by_capability)):
        found.append(f"{capability} is carried by the registry and classified by no row, so losing "
                     f"it would be silent")
    for name in sorted({row.get("class") for row in rows if row.get("class")}):
        if name not in {row.get("class") for row in rows}:
            found.append(f"{name} is carried by no row")

    found.extend(workflow_problems(matrices))
    found.extend(pattern_problems(matrices, program))
    found.extend(revision_problems(matrices))
    return found


def evidence_problems(matrices: Matrices, row: dict, subject: str) -> list[str]:
    """What a row's evidence may be: a claim the registry holds about *this* axis, current when the
    class requires it, and always present for the two classes that claim something about us."""
    found: list[str] = []
    evidence = row.get("evidence", [])
    for one in evidence:
        product, capability = one.get("product"), one.get("capability")
        if not matrices.holds(product, capability):
            found.append(f"{subject} points at the claim {product}/{capability}, which the registry "
                         f"does not hold")
        elif capability != subject:
            found.append(f"{subject} cites a claim about {capability}, which is not the axis this "
                         f"row classifies")
    if row.get("class") in EVIDENCED:
        if not evidence:
            found.append(f"{subject} is classified {row.get('class')} with no competitor evidence, "
                         f"and a claim about us without current evidence about them is not a claim")
        stale = [one for one in evidence
                 if (matrices.claim(one.get("product"), one.get("capability")) or {}).get("state")
                 in ("UNKNOWN", "STALE")]
        if stale:
            found.append(f"{subject} rests on evidence reading "
                         f"{(matrices.claim(stale[0].get('product'), stale[0].get('capability')) or {}).get('state')}, "
                         f"which cannot carry a {row.get('class')}")
    if row.get("class") == OPPORTUNITY:
        products = {one.get("product") for one in evidence}
        if len(products) < 2:
            found.append(f"{subject} is filed as {OPPORTUNITY} on {len(products)} product(s), and "
                         f"an opportunity read off one product is that product's feature, not ours")
    if row.get("class") == "UNKNOWN" and not str(row.get("research", "")).strip():
        found.append(f"{subject} is UNKNOWN and states no research question, so the unknown is a "
                     f"blank rather than a plan to settle it")
    return found


def mapping_problems(row: dict, subject: str, program: dict, ledger: set[str]) -> list[str]:
    """Every accepted row maps to a requirement, an issue or a deferral, and every id it names is
    one the tree carries: a gap that maps to nothing is a thought, not a decision."""
    found: list[str] = []
    for key in row.get("requirements", []):
        if key not in program["keys"]:
            found.append(f"{subject} maps to {key!r}, which is not a requirement key this program "
                         f"carries")
    for issue in row.get("issues", []):
        if issue not in program["issues"]:
            found.append(f"{subject} maps to issue {issue}, which this program does not carry")
    for decision in row.get("decisions", []):
        if decision not in ledger:
            found.append(f"{subject} names decision {decision!r}, which the architecture ledger "
                         f"does not hold")
    if not (row.get("requirements") or row.get("issues") or str(row.get("deferral", "")).strip()):
        found.append(f"{subject} maps to no requirement, no issue and no deferral, so nothing owns "
                     f"what the row found")
    return found


def workflow_problems(matrices: Matrices) -> list[str]:
    """Every workflow is a chain with owners and at least one handoff, and at least one handoff in
    the file leaves the product: a comparison that only lists features is the checkbox matrix the
    issue refuses."""
    found: list[str] = []
    owners = set(matrices.registry.universe) | {"symbiote"}
    seen: set[str] = set()
    kinds: set[str] = set()
    for workflow in matrices.workflows:
        name = workflow.get("id") or "a workflow naming no id"
        if name in seen:
            found.append(f"{name} is written twice")
        seen.add(name)
        steps = workflow.get("steps", [])
        if len(steps) < 3:
            found.append(f"{name} is written as {len(steps)} step(s), and a handoff cannot be "
                         f"located in a workflow shorter than an end-to-end pass")
        for at, step in enumerate(steps):
            if step.get("owner") not in owners:
                found.append(f"{name} step {at} is owned by {step.get('owner')!r}, which is neither "
                             f"a product in the universe nor this repository")
            if not str(step.get("action", "")).strip():
                found.append(f"{name} step {at} states no action")
        handoffs = workflow.get("handoffs", [])
        if not handoffs:
            found.append(f"{name} states no handoff, so it is a feature list rather than a workflow")
        for handoff in handoffs:
            at, kind = handoff.get("at"), handoff.get("kind")
            if kind not in HANDOFF_KINDS:
                found.append(f"{name} states the handoff kind {kind!r}, which this reader does not "
                             f"know")
            if not isinstance(at, int) or not 0 <= at < len(steps):
                found.append(f"{name} places a handoff at step {at!r}, which the workflow does not "
                             f"have")
            if not str(handoff.get("gap", "")).strip():
                found.append(f"{name} states a handoff with no gap named")
            if not str(handoff.get("consequence", "")).strip():
                found.append(f"{name} states a gap with no consequence, so nothing says what it "
                             f"costs")
            kinds.add(kind)
    if matrices.workflows and "cross_product" not in kinds:
        found.append("no workflow names a cross-product handoff, so the analysis closed inside our "
                     "own product and never looked at where a competitor pushes the user out")
    return found


def pattern_problems(matrices: Matrices, program: dict) -> list[str]:
    """Every pattern is a decision, on a real axis, about a claim the registry holds, with a
    rationale and a risk — and one that would move a locked decision says so."""
    found: list[str] = []
    for pattern in matrices.patterns:
        name = pattern.get("pattern") or "a pattern naming nothing"
        if pattern.get("decision") not in DECISIONS:
            found.append(f"{name!r} is decided {pattern.get('decision')!r}, which is not one of "
                         f"{', '.join(DECISIONS)}")
        capability = pattern.get("capability")
        if capability not in matrices.registry.capabilities:
            found.append(f"{name!r} decides about {capability!r}, which the registry's taxonomy does "
                         f"not carry")
        source = pattern.get("source") or {}
        if not matrices.holds(source.get("product"), source.get("capability")):
            found.append(f"{name!r} rests on {source.get('product')}/{source.get('capability')}, "
                         f"which the registry does not hold as a claim")
        if not str(pattern.get("rationale", "")).strip():
            found.append(f"{name!r} states no rationale, and a decision without one is "
                         f"rediscovered rather than remembered")
        if not str(pattern.get("risk", "")).strip():
            found.append(f"{name!r} states no risk")
        if pattern.get("changes_locked") not in (True, False):
            found.append(f"{name!r} does not say whether it would move a locked decision")
        if capability in matrices.locked and pattern.get("changes_locked") is not True:
            found.append(f"{name!r} decides about {capability}, which a locked row binds to a "
                         f"settled decision, and claims to move nothing")
        for issue in pattern.get("issues", []):
            if issue not in program["issues"]:
                found.append(f"{name!r} maps to issue {issue}, which this program does not carry")
    rejected = {row.get("capability") for row in matrices.rows
                if row.get("class") == "REJECTED_PATTERN"}
    decided = {pattern.get("capability") for pattern in matrices.patterns
               if pattern.get("decision") == "REJECT"}
    for capability in sorted(rejected - decided):
        found.append(f"{capability} is filed as REJECTED_PATTERN with no REJECT decision on it, so "
                     f"the rejection would be rediscovered")
    return found


def revision_problems(matrices: Matrices) -> list[str]:
    """A revision withdraws a claim of ours a competitor turned out to match, so it has to name what
    was claimed and the current claim that matched it."""
    found: list[str] = []
    if not matrices.revisions:
        found.append("no revision is recorded, so no claim of ours was ever tested against a "
                     "competitor that turned out to match it")
    for revision in matrices.revisions:
        subject = revision.get("capability") or "a revision naming no capability"
        if subject not in matrices.registry.capabilities:
            found.append(f"{subject} is revised against an axis the registry does not carry")
        matched = revision.get("matched_by") or {}
        claim = matrices.claim(matched.get("product"), matched.get("capability"))
        if claim is None:
            found.append(f"{subject} is revised against {matched.get('product')}/"
                         f"{matched.get('capability')}, which the registry does not hold as a claim")
        elif claim.get("state") in ("UNKNOWN", "STALE"):
            found.append(f"{subject} is revised against a claim reading {claim.get('state')}, which "
                         f"is not evidence that a competitor matches anything")
        for field in ("was", "revised"):
            if not str(revision.get(field, "")).strip():
                found.append(f"{subject} does not state what {field} says")
    return found


def matrix_report(matrices: Matrices, name: str) -> dict:
    """One matrix's rows, with the claims each rests on — a reading, not a refusal."""
    if name not in matrices.matrices:
        raise Refused(f"{name!r} is not a matrix this file names: "
                      f"{', '.join(matrices.matrices) or 'it names none'}")
    rows = [{"capability": row.get("capability"), "class": row.get("class"),
             "stance": row.get("stance"),
             "evidence": [f"{one.get('product')}/{one.get('capability')}"
                          for one in row.get("evidence", [])],
             "maps_to": sorted(row.get("requirements", []) + [f"#{one}" for one in row.get("issues", [])])
                         or [row.get("deferral", "")]}
            for row in matrices.rows if row.get("matrix") == name]
    return {"matrix": name, "rows": rows, "count": len(rows)}


def movement(matrices: Matrices, before_name: str, after_name: str) -> dict:
    """Which rows a registry change touches, and which of them are locked — read from the registry's
    own delta rather than a second comparison of our own."""
    for name in (before_name, after_name):
        if name != "registry" and name not in research.snapshots():
            raise Refused(f"{name!r} is not a state the registry carries: registry, "
                          f"{', '.join(research.snapshots()) or 'no snapshot'}")
    before = research.load_snapshot(ROOT, before_name)
    after = research.registry() if after_name == "registry" else research.load_snapshot(ROOT, after_name)
    report = research.delta(before, after)

    moved = [(change.get("product"), change.get("capability")) for change in report["changes"]]
    by_capability: dict[str, list[dict]] = {}
    for product, capability in moved:
        if capability:
            by_capability.setdefault(capability, []).append({"product": product})
    affected, locked, recomputed, cited = [], [], [], set()
    for row in matrices.rows:
        subject = row.get("capability")
        points = {(one.get("product"), one.get("capability")) for one in row.get("evidence", [])}
        touched = ([{"product": one["product"], "capability": subject}
                    for one in by_capability.get(subject, []) if one["product"]]
                   + [{"product": product, "capability": capability}
                      for product, capability in points if (product, capability) in set(moved)])
        if not touched:
            continue
        cited.add(subject)
        entry = {"capability": subject, "class": row.get("class"), "touched": touched,
                 "locked": subject in matrices.locked}
        if entry["locked"]:
            locked.append(entry)
        else:
            affected.append(entry)
        if any(change.get("change") in ("changed", "revised")
               for change in report["changes"]
               if (change.get("product"), change.get("capability")) in set(moved) & set(points)):
            recomputed.append(subject)
    # A capability no `watch` row binds to a requirement or a decision is one where a market change
    # has nothing on our side to review — reported rather than silently recomputed, because that is
    # the case the registry's own watch table leaves open.
    changed_capabilities = {capability for _product, capability in moved if capability}
    unwatched = sorted(capability for capability in changed_capabilities
                       if capability not in cited
                       or not any(row.get("capability") == capability
                                  for row in matrices.registry.watch))
    return {
        "from": before_name,
        "to": after_name,
        "affected": affected,
        "locked": locked,
        "recomputed": sorted(set(recomputed)),
        "unwatched": unwatched,
        "changes": report["changes"],
    }


def matrices() -> Matrices:
    return Matrices(read(MATRICES), str(MATRICES), research.registry())


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--check", action="store_true",
                        help="refuse the committed matrices, or pass them")
    parser.add_argument("--matrices", metavar="MATRIX", nargs="?", const="",
                        help="print one matrix's rows (default: every matrix)")
    parser.add_argument("--movement", nargs=2, metavar=("BEFORE", "AFTER"),
                        help="map a registry change onto the rows it touches")
    arguments = parser.parse_args(argv)
    try:
        if arguments.movement:
            print(json.dumps(movement(matrices(), *arguments.movement), indent=2))
            return 0
        if arguments.matrices is not None:
            committed = matrices()
            names = [arguments.matrices] if arguments.matrices else committed.matrices
            report = [matrix_report(committed, name) for name in names]
            print(json.dumps(report if arguments.matrices else
                             {"matrices": report,
                              "counts": {one["matrix"]: one["count"] for one in report}}, indent=2))
            return 0
        program, ledger = program_entries(), ledger_ids()
        committed = matrices()
        found = problems(committed, program, ledger)
    except Refused as error:
        print(f"refused: {error}", file=sys.stderr)
        return 1
    if found:
        for refusal in found:
            print(f"refused: {refusal}", file=sys.stderr)
        return 1
    review = [pattern.get("pattern") for pattern in committed.patterns
              if pattern.get("changes_locked") is True]
    print(f"matrices {committed.observed_at}: {len(committed.rows)} rows over "
          f"{len(committed.matrices)} matrices, {len(committed.workflows)} workflows, "
          f"{len(committed.patterns)} patterns, {len(committed.revisions)} revisions, "
          f"{len(CLASSES)} classes, 0 refusals")
    if review:
        print(f"{len(review)} pattern(s) would move a locked decision and are recorded for review "
              f"rather than applied: {', '.join(review)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
