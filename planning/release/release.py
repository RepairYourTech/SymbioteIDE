#!/usr/bin/env python3
"""What a release promises, what it withholds and how its claims are measured.

One record (`release.json`), one reading. The record holds the issue's own metric
list, the four release gates, the explicit non-goals by release, the golden-path
requirement with the nearest committed evidence, the two registers regressions are
tracked against and the concerns reviewed; every fact another owner holds stays
there and is read, not restated:

* an issue number is checked against the program (`generated/registry.json`);
* a case is a name a file in this tree declares — `def` in Python, `fn` in Rust —
  and a measured metric or register whose evidence names nothing this tree
  declares is refused;
* the constitutional invariants and competitor-parity rows are counted from the
  registers' own committed artifacts, never from numbers written here;
* the applicability marks are the licensing and trust policy's, read from
  `planning/policy/policy.json`, and a concern this record reviews that the policy
  record does not carry is refused.

    python3 planning/release/release.py --check | --metrics | --gates | --registers
"""
from __future__ import annotations

import argparse
import json
import pathlib
import re
import sys

HERE = pathlib.Path(__file__).resolve().parent
ROOT = HERE.parents[1]
RECORD = HERE / "release.json"
PROGRAM = ROOT / "planning/integrity/generated/registry.json"
POLICY = ROOT / "planning/policy/policy.json"
GOLDEN = "golden-path"

# The metrics the issue's own criterion lists, once each: a record that simply
# stops mentioning one is the deletion this list exists to refuse.
METRICS: tuple[str, ...] = ("quality", "task-success", "context-efficiency",
                            "graph-precision-recall", "recovery", "latency",
                            "mobile-control", "delivery")
# The releases the issue names, in order. Every gate is one of these and every
# release in the record names its own non-goals.
GATES: tuple[str, ...] = ("technical-preview", "alpha", "beta", "stable")
# How a metric's value is accounted for. `measured` claims this tree produces it;
# `delegated` names the issue that owns it instead.
SOURCES: tuple[str, ...] = ("measured", "delegated")
# The registers regressions are tracked against. Each one is read from its own
# committed artifact; the ids here are the only ones a gate may track.
REGISTERS: tuple[str, ...] = ("constitution", "parity")
# The applicability marks the issue's criterion asks for, and the three marks a
# concern may carry. The marks themselves belong to the licensing and trust
# policy; this record only states which concerns it reviewed.
CONCERNS: tuple[str, ...] = ("security", "privacy", "accessibility", "performance",
                             "cross_platform")
MARKS: tuple[str, ...] = ("required", "not_applicable", "separately_tracked")

# A case is a name a file declares, in this repository's two test languages.
CASE = re.compile(r"^\s*(?:async\s+def|def|fn)\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)\s*\(", re.M)
SKIP = {".git", "target", "node_modules", "__pycache__", ".freebuff"}


class Tree:
    """The tree this record is read against: which paths are files and which case names exist."""

    def __init__(self, root: pathlib.Path) -> None:
        self.root = root
        self.files = [path for path in root.rglob("*")
                      if path.is_file() and not (SKIP & set(path.parts))]
        self._cases: dict[str, list[str]] | None = None

    def has(self, name: str) -> bool:
        return (self.root / name).is_file()

    @property
    def cases(self) -> dict[str, list[str]]:
        """Every case name a file declares, mapped to the files that declare it."""
        if self._cases is None:
            found: dict[str, list[str]] = {}
            for path in self.files:
                if path.suffix not in (".py", ".rs"):
                    continue
                for match in CASE.finditer(path.read_text(errors="replace")):
                    found.setdefault(match.group("name"), []).append(str(path.relative_to(self.root)))
            self._cases = found
        return self._cases


def read(path: pathlib.Path) -> dict:
    return json.loads(path.read_text())


def program_issues() -> set[int]:
    """The issue numbers the program carries, read from the generated registry."""
    document = read(PROGRAM)
    return {entry.get("number") for entry in document.get("entries", [])}


def case_problems(tree: Tree, where: str, evidence: dict) -> list[str]:
    """Why the evidence entry does not name a case this tree declares, or nothing."""
    path, case = evidence.get("path"), evidence.get("case")
    if not path or not tree.has(path):
        return [f"{where} names {path!r} as its evidence and this tree does not carry it"]
    if not case or case not in tree.cases:
        return [f"{where} names the case {case!r}, which no file in this tree declares, so "
                f"nothing exercises it"]
    if path not in tree.cases.get(case, []):
        return [f"{where} names the case {case!r} in {path}, and the files declaring it are "
                f"{tree.cases[case]}"]
    return []


def metric_problems(record: dict, tree: Tree, program: set[int]) -> list[str]:
    """Every metric the issue lists is defined once and its source says where the value comes
    from: a measured metric names evidence this tree declares, a delegated one names the issue
    that owns it and why the definition is not restated here."""
    found, problems = [], []
    rows = record.get("metrics")
    if not isinstance(rows, list) or not rows:
        return ["the record defines no metric, so the issue's own criterion is stated nowhere"]
    for row in rows:
        name = row.get("id")
        found.append(name)
        if not row.get("definition") or not row.get("unit"):
            problems.append(f"metric {name!r} states no definition or no unit, so what it "
                            f"measures is not evidence")
        source = row.get("source")
        if source not in SOURCES:
            problems.append(f"metric {name!r} reads {source!r} where this reader states "
                            f"{list(SOURCES)}")
        elif source == "measured":
            artifacts = row.get("artifact")
            if artifacts and not tree.has(artifacts):
                problems.append(f"metric {name!r} names {artifacts!r} as what it measures and "
                                f"this tree does not carry it")
            problems += case_problems(tree, f"metric {name!r}", row.get("evidence", {}))
        else:
            owner = row.get("owner")
            if owner not in program:
                problems.append(f"metric {name!r} is delegated to #{owner}, which this program "
                                f"does not carry")
            if not row.get("reason"):
                problems.append(f"metric {name!r} is delegated with no reason, so the definition "
                                f"is restated rather than owned")
        narrowing = row.get("narrowing")
        if narrowing is not None:
            if not narrowing.get("scope"):
                problems.append(f"metric {name!r} is narrowed with no scope stated")
            for owner in narrowing.get("owners", []):
                if owner not in program:
                    problems.append(f"metric {name!r} is narrowed by #{owner}, which this program "
                                    f"does not carry")
    if sorted(found) != sorted(METRICS):
        problems.append(f"the metrics defined are {sorted(found)}, where the issue's own list is "
                        f"{sorted(METRICS)}: a metric dropped or added here is a claim the issue "
                        f"makes that no row states")
    return problems


def gate_problems(record: dict) -> list[str]:
    """Every release gate is one of the four, requires the golden path and at least one metric
    or register it does not invent, and names the non-goals that bound it."""
    problems = []
    rows = record.get("gates")
    if not isinstance(rows, list) or not rows:
        return ["the record states no release gate, so nothing bounds what a release may claim"]
    metrics = {row.get("id") for row in record.get("metrics", [])}
    registers = {row.get("id") for row in record.get("registers", [])}
    goals = {row.get("id") for row in record.get("non_goals", [])}
    required: dict[str, list[str]] = {}
    for row in rows:
        name = row.get("id")
        requires = row.get("requires") or []
        tracks = row.get("tracks") or []
        required[name] = requires
        if not requires:
            problems.append(f"gate {name!r} requires nothing, so what it permits is unbounded")
        if GOLDEN not in requires:
            problems.append(f"gate {name!r} does not require {GOLDEN!r}, so a candidate could "
                            f"pass it without evidence a client request reaches production")
        for one in requires:
            if one != GOLDEN and one not in metrics:
                problems.append(f"gate {name!r} requires {one!r}, which is not a metric this "
                                f"record defines")
        for one in tracks:
            if one not in registers:
                problems.append(f"gate {name!r} tracks {one!r}, which is not a register this "
                                f"record states")
        if not tracks:
            problems.append(f"gate {name!r} tracks no register, so no regression reading bounds it")
        stated = row.get("non_goals") or []
        if not stated:
            problems.append(f"gate {name!r} states no non-goal, so what the release intentionally "
                            f"does not support is unrecorded")
        for one in stated:
            if one not in goals:
                problems.append(f"gate {name!r} names non-goal {one!r}, which the record does not "
                                f"state")
    ids = [row.get("id") for row in rows]
    if sorted(ids) != sorted(GATES):
        problems.append(f"the gates stated are {sorted(ids)}, where the issue's own releases are "
                        f"{sorted(GATES)}")
    tracked = {one for row in rows for one in (row.get("tracks") or [])}
    for one in sorted(registers - tracked):
        problems.append(f"register {one!r} is tracked by no gate, so a regression it carries "
                        f"bounds nothing")
    for metric in sorted(metrics):
        if not any(metric in row for row in required.values()):
            problems.append(f"metric {metric!r} is required by no gate, so it measures scope no "
                            f"release promises")
    return problems


def non_goal_problems(record: dict, program: set[int]) -> list[str]:
    """Every non-goal names its release, what it does not support, why, and the issue that owns
    the gap when one does."""
    problems = []
    rows = record.get("non_goals")
    if not isinstance(rows, list) or not rows:
        return ["the record states no non-goal, so what a release does not support is nowhere"]
    seen = set()
    for row in rows:
        name = row.get("id")
        if name in seen:
            problems.append(f"non-goal {name!r} is stated twice")
        seen.add(name)
        if row.get("release") not in GATES:
            problems.append(f"non-goal {name!r} names release {row.get('release')!r}, which is "
                            f"not one of {list(GATES)}")
        for field in ("does_not_support", "why"):
            if not row.get(field):
                problems.append(f"non-goal {name!r} states no {field.replace('_', ' ')}, so the "
                                f"boundary is asserted rather than recorded")
        owner = row.get("owner")
        if owner is not None and owner not in program:
            problems.append(f"non-goal {name!r} is owned by #{owner}, which this program does not "
                            f"carry")
    return problems


def golden_problems(record: dict, tree: Tree, program: set[int]) -> list[str]:
    """The golden-path requirement names a committed artifact and the case that reads it."""
    problems = []
    golden = record.get("golden_path")
    if not isinstance(golden, dict):
        return ["the record states no golden path, so a release candidate owes no evidence"]
    if not golden.get("requires"):
        problems.append("the golden path states no requirement")
    artifact = golden.get("artifact")
    if not artifact or not tree.has(artifact):
        problems.append(f"the golden path names {artifact!r} as its artifact and this tree does "
                        f"not carry it")
    problems += case_problems(tree, "the golden path", golden.get("evidence", {}))
    if golden.get("owner") not in program:
        problems.append(f"the golden path is owned by #{golden.get('owner')}, which this program "
                        f"does not carry")
    return problems


def register_problems(record: dict, tree: Tree) -> list[str]:
    """Each register is a committed artifact this tree carries with a declared case, and it
    carries something: an empty register bounds no regression."""
    problems = []
    rows = record.get("registers")
    if not isinstance(rows, list) or not rows:
        return ["the record states no register, so no regression is tracked"]
    found = [row.get("id") for row in rows]
    if sorted(found) != sorted(REGISTERS):
        problems.append(f"the registers stated are {sorted(found)}, where the issue names "
                        f"{sorted(REGISTERS)}")
    for row in rows:
        name = row.get("id")
        source, carries = row.get("source"), row.get("carries")
        if not source or not tree.has(source):
            problems.append(f"register {name!r} names {source!r} as its source and this tree does "
                            f"not carry it")
            continue
        document = read(tree.root / source)
        entries = document.get(carries) if carries else None
        if not isinstance(entries, list) or not entries:
            problems.append(f"register {name!r} reads {carries!r} out of {source}, which holds "
                            f"{type(entries).__name__ if entries is not None else 'nothing'}: an "
                            f"empty register bounds no regression")
        problems += case_problems(tree, f"register {name!r}", row.get("holds", {}))
    return problems


def concern_problems(record: dict, program: set[int]) -> list[str]:
    """The concerns this record reviews are the marks the policy record carries, and each mark
    is one of the three the issue states."""
    problems = []
    concerns = record.get("concerns")
    if not isinstance(concerns, dict):
        return ["the record reviews no concern, so applicability is unmarked"]
    source = concerns.get("source")
    if not source or not (ROOT / source).is_file():
        return [f"the concerns read {source!r} as their source and this tree does not carry it"]
    marks = read(ROOT / source).get("applicability")
    if not isinstance(marks, list) or not marks:
        return [f"{source} carries no applicability mark, so nothing is reviewed there"]
    stated = [row.get("concern") for row in marks]
    reviewed = concerns.get("reviewed")
    if not isinstance(reviewed, list) or sorted(reviewed) != sorted(stated):
        problems.append(f"the concerns reviewed are {reviewed}, where {source} marks {stated}: a "
                        f"concern reviewed here that the policy record does not mark, or one it "
                        f"marks that is not reviewed, is applicability assumed rather than "
                        f"checked")
    for row in marks:
        if row.get("mark") not in MARKS:
            problems.append(f"concern {row.get('concern')!r} carries mark {row.get('mark')!r}, "
                            f"where the issue states {list(MARKS)}")
        for owner in row.get("owners", []):
            if owner not in program:
                problems.append(f"concern {row.get('concern')!r} is tracked by #{owner}, which "
                                f"this program does not carry")
    return problems


def problems(record: dict, tree: Tree, program: set[int]) -> list[str]:
    return (metric_problems(record, tree, program) + gate_problems(record)
            + non_goal_problems(record, program) + golden_problems(record, tree, program)
            + register_problems(record, tree) + concern_problems(record, program))


def readings(record: dict) -> tuple[int, dict[str, int], int, dict[str, int]]:
    """What the record and its registers carry, counted rather than declared."""
    metrics = record.get("metrics", [])
    sources = {row.get("source"): 0 for row in metrics}
    for row in metrics:
        sources[row.get("source")] = sources.get(row.get("source"), 0) + 1
    counts = {}
    for row in record.get("registers", []):
        source, carries = row.get("source"), row.get("carries")
        if source and carries and (ROOT / source).is_file():
            counts[row.get("id")] = len(read(ROOT / source).get(carries, []))
    golden = len(record.get("non_goals", []))
    return len(metrics), sources, golden, counts


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--check", action="store_true", help="refuse every rule this record breaks")
    mode.add_argument("--metrics", action="store_true", help="each metric and where its value comes from")
    mode.add_argument("--gates", action="store_true", help="each gate, its requirements and the concerns' marks")
    mode.add_argument("--registers", action="store_true", help="each register and what it carries")
    args = parser.parse_args(argv)

    record, tree = read(RECORD), Tree(ROOT)
    program = program_issues()
    if args.metrics:
        for row in record.get("metrics", []):
            where = (f"measured by {row['evidence']['path']}::{row['evidence']['case']}"
                     + (f" over {row['artifact']}" if row.get("artifact") else "")
                     if row.get("source") == "measured"
                     else f"delegated to #{row.get('owner')}")
            print(f"{row.get('id'):24} {row.get('source'):10} {where}")
        return 0
    if args.gates:
        marks = {row.get("concern"): row.get("mark")
                 for row in read(POLICY).get("applicability", [])}
        for row in record.get("gates", []):
            print(f"{row.get('id')}: requires {', '.join(row.get('requires', []))}")
            print(f"    permits: {row.get('permits')}")
            print(f"    withholds: {row.get('withholds')}")
            print(f"    concerns: {', '.join(f'{name}={mark}' for name, mark in marks.items())}")
        return 0
    if args.registers:
        for row in record.get("registers", []):
            source, carries = row.get("source"), row.get("carries")
            count = len(read(ROOT / source).get(carries, [])) if tree.has(source) else "missing"
            print(f"{row.get('id'):14} {source} carries {carries}: {count}")
        return 0

    found = problems(record, tree, program)
    count, sources, goals, registers = readings(record)
    for one in found:
        print(f"refused: {one}")
    carried = ", ".join(f"{name} {amount}" for name, amount in registers.items())
    print(f"release {record.get('observed_at')}: {count} metrics "
          f"({sources.get('measured', 0)} measured, {sources.get('delegated', 0)} delegated), "
          f"{len(record.get('gates', []))} gates, {goals} non-goals, "
          f"{len(record.get('registers', []))} registers ({carried}), "
          f"{len(record.get('concerns', {}).get('reviewed', []))} concerns, "
          f"{len(found)} refusals")
    return 1 if found else 0


if __name__ == "__main__":
    sys.exit(main())
