#!/usr/bin/env python3
"""The audit's accepted-decision coverage, held as data and checked against the registry.

#443 reconciled the live issue inventory against the accepted discussion and recorded
one coverage matrix: a section per accepted capability family, naming the canonical
issues that own it. That matrix lived only in a build artifact, which expires, so the
reconciliation it certifies could not be read in this tree and nothing could tell
whether it still held.

This module owns it now. `fixtures/audit-2026-09-07-coverage.md` is the matrix exactly
as that artifact recorded it, byte for byte, and `generated/coverage.json` is the
ledger derived from it and from the committed registry: every family's owners resolved
against registered work, the totals the audit recorded checked against the registry's
own, and the parts of the registry no family claims named rather than left to be
inferred.

Refused by name:

* a family with no name or no owner, which is not coverage;
* the same family named twice;
* an owner the registry does not hold, or holds as a reference entry — a reference
  entry resolves to work and is not work to be covered;
* one owner named twice by one family;
* a registry whose declared counts disagree with the entries it holds, class by class
  through `validate.py`'s own counting, or whose entries and unkeyed issues do not
  account for the snapshot it names;
* totals that are not taken from the registry's entries, or not the revision the audit
  reconciled;
* a canonical task that declares no acceptance item, since a task with no acceptance
  and verification criteria is not actionable work;
* a ledger that no longer records the atoms these families leave unclaimed;
* a committed ledger that is not what this tree emits (`--check`, which also names the
  claim inside it that is untrue), or one whose recorded matrix bytes are not this
  matrix's.

Neither mode writes or reports a clean ledger before refusing: `--write` refuses the
same ledger `--check` would.

What it cannot check, stated here rather than implied: the families are the audit's
reading of the accepted discussion, and this repository holds no conversation corpus.
This module proves the matrix is internally sound, that its owners are work the
registry holds, and that it still agrees with the registry. It cannot prove that the
28 families are the right ones, and it does not claim to.

The provenance records the artifact's expiry as well as its hashes, because after
2026-10-07 the archive hash cannot be recomputed by anyone. Nothing here reaches the
network — this suite is offline — so the recorded hashes are a claim a reader re-tests
against the run, not something this tool re-fetches; the expiry says how long that is
possible, and it is data rather than a rule precisely so a calendar date cannot fail a
build.
"""

import argparse
import hashlib
import json
import re
import sys
from pathlib import Path

from validate import counted_classes

ROOT = Path(__file__).parent
MATRIX = ROOT / "fixtures/audit-2026-09-07-coverage.md"
REGISTRY = ROOT / "generated/registry.json"
OUTPUT = ROOT / "generated/coverage.json"

# The artifact this matrix was taken from, so the ledger names its own origin, and the
# inventory totals the audit reconciled. The archive's sha256 and the matrix's were read
# from the run below and hashed again on 2026-09-15, and the expiry is the run's own:
# after it, the archive hash can no longer be recomputed from the artifact, so the
# ledger states the window instead of leaving it implied. The totals and the matrix's
# hash are the audit's own revision counts and bytes, held here so the ledger can refuse
# them the moment the registry or the matrix moves past them.
PROVENANCE = {
    "revision": "2026-09-07-v2.4",
    "audit_issue": 443,
    "artifact": "roadmap-audit-result",
    "artifact_run": "https://github.com/RepairYourTech/SymbioteIDE/actions/runs/34155229015",
    "artifact_sha256": "a7612e6c529952bf0bb5e87c90af233b60c8ce4665ec0f049608fa9f780cbf36",
    "artifact_expires_at": "2026-10-07T19:29:47Z",
    "matrix": "fixtures/audit-2026-09-07-coverage.md",
    "matrix_sha256": "1e86040cf4d0142c10eb10aadaa66b3daffcc370c451075e27fc2d9699b7b116",
    "audited_totals": {"atoms": 241, "epics": 19, "references": 198, "edges": 702},
}


def parse_matrix(text):
    """The matrix as data: each family and the owner numbers its section names."""
    families = []
    for block in text.split("\n## ")[1:]:
        name, _, body = block.partition("\n")
        families.append({"family": name.strip(), "owners": [int(n) for n in re.findall(r"#(\d+)", body)]})
    return families


def facts(registry):
    """What the registry holds, counted from its entries by the classes validate.py owns.

    Every figure here is read out of the entries rather than out of the registry's own
    declaration of them, so a registry that miscounts itself cannot make the ledger
    repeat the mistake: the audited totals are compared against this, not against the
    registry's description of itself.
    """
    held = counted_classes(registry["entries"])
    return {
        "atoms": held["tasks"],
        "epics": held["epics"],
        "references": held["references"],
        "edges": sum(len(entry.get("dependencies") or []) for entry in registry["entries"]),
    }


def counts_disagree(registry):
    """Every way the registry's declared counts can be untrue of the entries it holds.

    The classes come from `validate.counted_classes`, the definition the generator
    itself uses, so this asks what agreement means instead of keeping a second opinion.
    A class the declaration omits is a disagreement too, since a registry that cannot
    say how many issues fall in a class has not accounted for them.
    """
    entries = registry["entries"]
    counts = registry["counts"]
    held = counted_classes(entries)
    found = [
        f"the registry declares {counts.get(field)} {field} and holds {held[field]}"
        for field in held
        if counts.get(field) != held[field]
    ]
    # Every issue is either an entry or one no key names, so the two must account for
    # the snapshot: a declaration moved between the classes without moving the entry
    # it describes shows up here.
    if len(entries) + counts.get("unkeyed", 0) != counts.get("snapshot_issues"):
        found.append(
            f"the registry holds {len(entries)} entries and declares {counts.get('unkeyed')} unkeyed, "
            f"which do not account for its {counts.get('snapshot_issues')} snapshot issues"
        )
    return found


def ledger(matrix_text, registry, provenance=None):
    """The ledger: the matrix resolved against a registry, with what it leaves out."""
    families = parse_matrix(matrix_text)
    entries = {entry["number"]: entry for entry in registry["entries"]}
    claimed = {number for family in families for number in family["owners"]}
    atoms = {number for number, entry in entries.items() if entry["kind"] == "task"}
    return {
        "schema_version": 1,
        "scope": "accepted-decision coverage of the canonical roadmap; not implementation conformance",
        "provenance": dict(PROVENANCE if provenance is None else provenance,
                           matrix_sha256=hashlib.sha256(matrix_text.encode()).hexdigest()),
        "families": families,
        "families_count": len(families),
        "owners_named": len(claimed),
        "totals": facts(registry),
        "unclaimed_atoms": sorted(atoms - claimed),
        "atoms_without_acceptance_items": sorted(
            number for number, entry in entries.items()
            if entry["kind"] == "task" and not entry.get("acceptance_items")
        ),
    }


def problems(built, registry, matrix_text):
    """Every way the ledger can be untrue of the registry, named."""
    found = list(counts_disagree(registry))
    entries = {entry["number"]: entry for entry in registry["entries"]}
    seen = set()
    for family in built["families"]:
        name = family["family"]
        if not name:
            found.append("a coverage family has no name")
        if name in seen:
            found.append(f"the family {name!r} is named twice")
        seen.add(name)
        if not family["owners"]:
            found.append(f"the family {name!r} names no owner, which is not coverage")
        for number in sorted({number for number in family["owners"] if family["owners"].count(number) > 1}):
            found.append(f"the family {name!r} names #{number} more than once")
        for number in family["owners"]:
            entry = entries.get(number)
            if entry is None:
                found.append(f"the family {name!r} names #{number}, which the registry does not hold")
            elif entry["kind"] == "reference":
                found.append(
                    f"the family {name!r} names #{number}, a reference entry rather than the work it resolves to"
                )
    audited = built["provenance"]["audited_totals"]
    for field, held in facts(registry).items():
        if built["totals"].get(field) != held:
            found.append(f"the ledger's {field} is {built['totals'].get(field)} where its entries hold {held}")
        if audited[field] != held:
            found.append(
                f"the audit reconciled {audited[field]} {field} at {built['provenance']['revision']} "
                f"and the registry holds {held}: the coverage needs re-deriving against the current inventory"
            )
    claimed = {number for family in built["families"] for number in family["owners"]}
    atoms = {number for number, entry in entries.items() if entry["kind"] == "task"}
    if sorted(built["unclaimed_atoms"]) != sorted(atoms - claimed):
        found.append("the ledger's unclaimed atoms are not the atoms these families leave unclaimed")
    for number in built["atoms_without_acceptance_items"]:
        found.append(f"#{number} is canonical work that declares no acceptance item")
    recorded = built["provenance"].get("matrix_sha256")
    actual = hashlib.sha256(matrix_text.encode()).hexdigest()
    if recorded != actual:
        found.append(
            "the ledger records the matrix's bytes as "
            f"{recorded} where they are {actual}: it is not this matrix's ledger"
        )
    return found


REQUIRED = ("schema_version", "scope", "provenance", "families", "families_count", "owners_named", "totals",
            "unclaimed_atoms", "atoms_without_acceptance_items")


def render(matrix_text, registry):
    return json.dumps(ledger(matrix_text, registry), indent=2, sort_keys=True) + "\n"


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--matrix", type=Path, default=MATRIX)
    parser.add_argument("--registry", type=Path, default=REGISTRY)
    parser.add_argument("--output", type=Path, default=OUTPUT)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--write", action="store_true", help="write the ledger this tree emits")
    mode.add_argument("--check", action="store_true", help="fail if the committed ledger is not what this tree emits")
    arguments = parser.parse_args(argv)
    matrix_text = arguments.matrix.read_text()
    registry = json.loads(arguments.registry.read_text())
    built = ledger(matrix_text, registry)
    summary = (
        f"{built['families_count']} families, {built['owners_named']} owners, "
        f"{built['totals']['atoms']} atoms, {len(built['unclaimed_atoms'])} unclaimed"
    )
    refusal = problems(built, registry, matrix_text)
    if arguments.write:
        for entry in refusal:
            print(entry, file=sys.stderr)
        if refusal:
            print("refusing to write a ledger that is not true of this registry", file=sys.stderr)
            return 1
        arguments.output.write_text(render(matrix_text, registry))
        print(summary)
        return 0
    found = []
    if not arguments.output.exists():
        print(f"the committed ledger {arguments.output} does not exist", file=sys.stderr)
        return 1
    try:
        committed = json.loads(arguments.output.read_text())
    except json.JSONDecodeError as error:
        print(f"the committed ledger {arguments.output} is not JSON: {error}", file=sys.stderr)
        return 1
    missing = [key for key in REQUIRED if key not in committed]
    if missing:
        found.append(f"the committed ledger is missing {missing}")
    else:
        # The committed ledger's own claims are checked against the registry, not only
        # its bytes against this tree's emission.
        found.extend(problems(committed, registry, matrix_text))
    if arguments.output.read_text() != render(matrix_text, registry):
        found.append(f"the committed ledger {arguments.output} is not what this tree emits")
    for entry in found:
        print(entry, file=sys.stderr)
    if found:
        return 1
    print(f"{summary}: matches the tree")
    return 0


if __name__ == "__main__":
    sys.exit(main())
