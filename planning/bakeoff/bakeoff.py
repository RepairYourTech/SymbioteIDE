#!/usr/bin/env python3
"""What #38's own acceptance criteria answer to, and what the tree shows for them.

The bake-off issue states its acceptance criteria as checkboxes; the way this repository
proves the shell choice is the committed spike contract
(`docs/architecture/spike-contracts.json`), the ADR-0001 section the choice's ledger record
names as the bar, and the dossier a run writes. Nothing joined them: the contract answers the
ADR's proof section clause by clause, and the issue's own criteria were answered by nothing a
reader could resolve. This record is that join, and it holds what it can be held to:

* the number of criteria is the program's own count for the issue
  (`acceptance_items` in `planning/integrity/generated/registry.json`), so a criterion
  dropped or added here is refused rather than silently states one fewer claim;
* a carrier is the contract's own three kinds, one level up from its `answers`: an
  `obligation` the contract declares and a run must exercise, a `measurement` it
  predeclares, or an `elsewhere` naming the issue that owns a criterion the contract does
  not carry — and each is read back against its owner, so a carrier that resolves to
  nothing is refused;
* the contract, the ledger record that binds it to this issue, and the dossier the runs
  wrote must agree with each other and with this record, so the join cannot point at
  another choice's contract or another contract's runs;
* what the committed runs show for each carrier is **derived** from the dossier rather
  than stated here, so this file cannot claim a measurement no run observed.

    python3 planning/bakeoff/bakeoff.py --check | --criteria | --dossier
"""
from __future__ import annotations

import argparse
import json
import pathlib
import sys

HERE = pathlib.Path(__file__).resolve().parent
ROOT = HERE.parents[1]
RECORD = HERE / "bakeoff.json"
PROGRAM = "planning/integrity/generated/registry.json"

# The carriers a criterion can answer to, the three kinds the contract's own answers use: an
# obligation it declares, a measurement it predeclares, or the issue that owns what neither
# carries. A criterion naming two kinds at once is refused, the way an answer is.
CARRIERS: tuple[str, ...] = ("obligation", "measurement", "elsewhere")


class Sources:
    """Where every fact is read from: the tree, the issue program and the two artifacts.

    Paths come from the record, so a test can point this at a tree of its own; only the
    program's location is fixed, because the record names the issue rather than the file.
    """

    def __init__(self, root: pathlib.Path = ROOT, program: pathlib.Path | None = None) -> None:
        self.root = pathlib.Path(root)
        self.program_path = pathlib.Path(program) if program else self.root / PROGRAM
        self._program: dict | None = None

    def has(self, name: str) -> bool:
        return bool(name) and (self.root / name).is_file()

    def read(self, name: str | pathlib.Path) -> dict:
        return json.loads((self.root / name).read_text())

    def entry(self, issue: int | None) -> dict | None:
        """The program's own entry for the issue, or nothing where it carries none.

        The program is read once and kept, so a record routing several criteria to owners does
        not parse the whole registry for each one.
        """
        if not self.program_path.is_file():
            return None
        if self._program is None:
            self._program = self.read(self.program_path)
        return next((one for one in self._program.get("entries", [])
                     if one.get("number") == issue), None)


def kind_of(carrier: dict) -> list[str]:
    """Which carrier kinds an entry names, in `CARRIERS` order."""
    return [kind for kind in CARRIERS if kind in carrier]


def contract_block(record: dict, sources: Sources) -> tuple[dict | None, list[str]]:
    """The contract this record routes through, and every way the join fails to resolve.

    The five paths are read, the contract document must hold the identity the record names
    and that contract must settle the decision the record names, the ledger must hold that
    decision with this issue as the one blocking it and this contract as its proof, and the
    dossier must be a result measured against that contract. A record pointing at another
    choice's contract, another contract's runs or a decision that is not this issue's is
    refused by name rather than compared against nothing.
    """
    block = record.get("contract")
    if not isinstance(block, dict):
        return None, ["the record names no contract, so the criteria answer to nothing"]
    problems: list[str] = []
    for what in ("document", "id", "decision", "ledger", "result", "record"):
        if not block.get(what):
            problems.append(f"the record's contract names no {what}")
    for what in ("document", "ledger", "result", "record"):
        if block.get(what) and not sources.has(block[what]):
            problems.append(f"the record names {block[what]!r} as the {what} and this tree "
                            f"does not carry it")
    if problems:
        return None, problems
    document = sources.read(block["document"])
    contract = next((one for one in document.get("contracts", [])
                     if one.get("id") == block["id"]), None)
    if contract is None:
        return None, [f"the contract document holds no contract {block['id']!r}, which the "
                      f"record names"]
    if contract.get("decision") != block["decision"]:
        problems.append(f"the contract {block['id']!r} settles {contract.get('decision')!r}, "
                        f"and the record names {block['decision']!r}")
    decisions = [one for one in sources.read(block["ledger"]).get("decisions", [])
                 if one["draft"].get("id") == block["decision"]]
    if not decisions:
        problems.append(f"the ledger holds no decision {block['decision']!r}, which the record "
                        f"names")
    else:
        held = decisions[0]
        if held.get("blocking_issue") != record.get("issue"):
            problems.append(f"the ledger's {block['decision']} is blocked by "
                            f"#{held.get('blocking_issue')}, not by the issue this record states")
        if held.get("proof_contract") != block["id"]:
            problems.append(f"the ledger's {block['decision']} is proved by "
                            f"{held.get('proof_contract')!r}, and this record names "
                            f"{block['id']!r}")
    results = sources.read(block["result"])
    if results.get("contract") != block["id"]:
        problems.append(f"the dossier measures {results.get('contract')!r}, and this record "
                        f"names {block['id']!r}")
    if not results.get("runs"):
        problems.append(f"the dossier records no run, so nothing has been measured against the "
                        f"bar this record routes")
    return contract, problems


def criteria_problems(record: dict, contract: dict, entry: dict | None,
                      sources: Sources) -> list[str]:
    """Every criterion the issue states is stated once here, and every carrier resolves.

    The count is the program's — how many acceptance items the issue's own body states — so
    this record cannot drop or add a criterion; a carrier must be one of the contract's own
    obligations, one of the measurements it predeclares, or an issue the program carries with
    a reason for routing part of the criterion away.
    """
    problems: list[str] = []
    if entry is None:
        return [f"the program carries no issue #{record.get('issue')}, so the criteria this "
                f"record states are held to nothing"]
    expected = entry.get("acceptance_items")
    if not isinstance(expected, int):
        return [f"the program records no acceptance-item count for #{record.get('issue')}, so "
                f"how many criteria the issue states is unknown"]
    rows = record.get("criteria")
    if not isinstance(rows, list) or not rows:
        return [f"the record states no criterion, so #{record.get('issue')}'s acceptance "
                f"criteria answer to nothing"]
    stated = [row.get("n") for row in rows]
    numbered = [one for one in stated if isinstance(one, int) and not isinstance(one, bool)]
    if len(numbered) != len(stated):
        problems.append(f"the criteria are numbered {stated}, and one of those is not a number: "
                        f"every row's `n` is the number the issue states its criterion under")
    elif sorted(stated) != list(range(1, expected + 1)):
        problems.append(f"the criteria stated are {sorted(stated)}, where the program records "
                        f"{expected} acceptance items for #{record.get('issue')}: a criterion "
                        f"dropped, added or renumbered here is a claim the issue makes that no "
                        f"row answers")
    obligations = set(contract.get("workload", []))
    measurements = {one.get("name") for one in contract.get("measurements", [])}
    for row in rows:
        number = row.get("n")
        if not row.get("criterion"):
            problems.append(f"criterion {number} states no criterion words, so its row is a "
                            f"number standing where the issue's own claim should be")
        carriers = row.get("carried_by")
        if not isinstance(carriers, list) or not carriers:
            problems.append(f"criterion {number} is carried by nothing, so the issue's "
                            f"criterion answers to no obligation, measurement or owner")
            continue
        seen: list[str] = []
        for carrier in carriers:
            if not isinstance(carrier, dict):
                problems.append(f"criterion {number} is carried by {carrier!r}, which names no "
                                f"obligation, measurement or owner")
                continue
            kinds = kind_of(carrier)
            if len(kinds) != 1:
                problems.append(f"criterion {number} is carried by {kinds or 'no kind'} at "
                                f"once: a carrier is one obligation, one measurement or one "
                                f"issue, not {len(kinds)} of them")
                continue
            kind, value = kinds[0], carrier[kinds[0]]
            if value in seen:
                problems.append(f"criterion {number} names the {kind} {value!r} twice, so one "
                                f"carrier stands where two facts are owed")
            seen.append(value)
            if kind == "obligation" and value not in obligations:
                problems.append(f"criterion {number} names the obligation {value!r}, which the "
                                f"contract does not declare")
            if kind == "measurement" and value not in measurements:
                problems.append(f"criterion {number} names the measurement {value!r}, which the "
                                f"contract does not predeclare")
            if kind == "elsewhere":
                if not isinstance(value, dict) or not value.get("why"):
                    problems.append(f"criterion {number} routes part of itself away with no "
                                    f"reason, so nothing says what the owner carries")
                issue = value.get("issue") if isinstance(value, dict) else None
                if sources.entry(issue) is None:
                    problems.append(f"criterion {number} names #{issue} as an owner, and this "
                                    f"program does not carry it")
    return problems


def readings(record: dict, contract: dict | None, artifact: dict | None) -> dict:
    """What the record routes and what the committed runs show, counted rather than declared.

    The criteria, their carriers and the contract's own sizes come from the record and the
    contract; what stands behind them comes from the dossier — an obligation a run attests,
    a measurement a run observed, or nothing yet.
    """
    rows = record.get("criteria", [])
    carriers = [carrier for row in rows for carrier in row.get("carried_by", [])]
    runs = (artifact or {}).get("runs", [])
    attested = {one for run in runs for one in run.get("exercised", [])}
    observed = {one.get("measurement") for run in runs for one in run.get("observations", [])}
    applicable = (contract or {}).get("applicable_platforms", [])
    measured = {run.get("platform") for run in runs}
    untested = {one for one in (artifact or {}).get("untested_platforms", [])}
    return {
        "criteria": len(rows),
        "carriers": len(carriers),
        "kinds": {kind: sum(1 for carrier in carriers if kind in carrier) for kind in CARRIERS},
        "routed": sum(1 for row in rows
                      if any("elsewhere" in carrier for carrier in row.get("carried_by", []))),
        "obligations": len((contract or {}).get("workload", [])),
        "obligations_named": len({carrier["obligation"] for carrier in carriers
                                  if "obligation" in carrier}),
        "measurements": len((contract or {}).get("measurements", [])),
        "measurements_named": len({carrier["measurement"] for carrier in carriers
                                   if "measurement" in carrier}),
        "obligations_attested": len({carrier["obligation"] for carrier in carriers
                                     if "obligation" in carrier} & attested),
        "measurements_observed": len({carrier["measurement"] for carrier in carriers
                                      if "measurement" in carrier} & observed),
        "platforms": len(applicable),
        "platforms_measured": len(measured & set(applicable)),
        "platforms_untested": len(untested & set(applicable)),
        "runs": len(runs),
    }


def measured_for(record_row: dict, artifact: dict | None) -> list[str]:
    """What the dossier shows for one criterion's carriers, in the record's own order."""
    runs = (artifact or {}).get("runs", [])
    attested = {one for run in runs for one in run.get("exercised", [])}
    observed: dict[str, list[float]] = {}
    for run in runs:
        for one in run.get("observations", []):
            observed.setdefault(one.get("measurement"), []).append(one.get("observed"))
    found = []
    for carrier in record_row.get("carried_by", []):
        if "obligation" in carrier:
            state = ("attested by a committed run" if carrier["obligation"] in attested
                     else "attested by no committed run")
            found.append(f"obligation {carrier['obligation']!r}: {state}")
        elif "measurement" in carrier:
            values = observed.get(carrier["measurement"])
            state = (f"observed at {', '.join(f'{one:g}' for one in values)}"
                     if values else "observed by no committed run")
            found.append(f"measurement {carrier['measurement']}: {state}")
        else:
            found.append(f"owner #{carrier['elsewhere'].get('issue')}: "
                         f"{carrier['elsewhere'].get('why')}")
    return found


def problems(record: dict, sources: Sources) -> list[str]:
    contract, found = contract_block(record, sources)
    entry = sources.entry(record.get("issue"))
    if contract is not None:
        found += criteria_problems(record, contract, entry, sources)
    return found


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--check", action="store_true", help="refuse every rule this record breaks")
    mode.add_argument("--criteria", action="store_true", help="each criterion and what carries it")
    mode.add_argument("--dossier", action="store_true", help="what the committed runs show")
    args = parser.parse_args(argv)

    record, sources = json.loads(RECORD.read_text()), Sources()
    contract, _found = contract_block(record, sources)
    artifact = None
    block = record.get("contract", {})
    if block.get("result") and sources.has(block["result"]):
        artifact = sources.read(block["result"])
    if args.criteria:
        for row in record.get("criteria", []):
            print(f"#{record.get('issue')} criterion {row.get('n')}: {row.get('criterion')}")
            for line in measured_for(row, artifact):
                print(f"    {line}")
        return 0
    if args.dossier:
        counts = readings(record, contract, artifact)
        for name in ("runs", "platforms", "platforms_measured", "platforms_untested",
                     "obligations_attested", "measurements_observed"):
            print(f"{name.replace('_', ' ')}: {counts[name]}")
        return 0

    found = problems(record, sources)
    counts = readings(record, contract, artifact)
    for one in found:
        print(f"refused: {one}")
    kinds = ", ".join(f"{amount} {kind}"
                      for kind, amount in counts["kinds"].items())
    print(f"bake-off #{record.get('issue')} ({record.get('observed_at')}): "
          f"{counts['criteria']} criteria over {counts['carriers']} carriers ({kinds}), "
          f"{counts['obligations_named']} of {counts['obligations']} obligations and "
          f"{counts['measurements_named']} of {counts['measurements']} measurements named, "
          f"{counts['routed']} criteria routed to an owner, "
          f"{counts['platforms_measured']} of {counts['platforms']} applicable platforms "
          f"measured ({counts['platforms_untested']} declared untested, {counts['runs']} runs), "
          f"{len(found)} refusals")
    return 1 if found else 0


if __name__ == "__main__":
    sys.exit(main())
