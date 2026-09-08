#!/usr/bin/env python3
"""Offline roadmap inventory and structural validation; never calls GitHub.

Only explicit Dependencies sections define edges. A topological order is an
execution ordering, not evidence of readiness or implementation completion.
"""

import argparse
import hashlib
import heapq
import json
import re
from pathlib import Path


class IntegrityError(ValueError):
    """The snapshot cannot safely serve as a canonical execution registry."""


def section(body, heading):
    matches = re.findall(r"(?m)^## " + re.escape(heading) + r"\s*\n([\s\S]*?)(?=^## |\Z)", body)
    if len(matches) > 1:
        raise IntegrityError(f"duplicate {heading} section")
    return matches[0] if matches else ""


def refs(body):
    return [int(n) for n in re.findall(r"(?<![\w/])#(\d+)\b", body)]


def dependencies(body):
    if not re.search(r"(?m)^## Dependencies[ \t]*$", body):
        raise IntegrityError("canonical task missing explicit Dependencies section")
    lines = [line.strip() for line in section(body, "Dependencies").splitlines() if line.strip()]
    if not lines:
        raise IntegrityError("empty Dependencies section; explicitly declare - None")
    if lines in (["- None; governed by the constitution."], ["- None"]):
        return []
    result = []
    for line in lines:
        match = re.fullmatch(r"- (?:#|https://github\.com/RepairYourTech/SymbioteIDE/issues/)([1-9]\d*)", line)
        if not match:
            raise IntegrityError(f"unparsed dependency syntax: {line!r}")
        result.append(int(match[1]))
    return result


def marker(body, name):
    values = re.findall(r"<!--\s*symbiote-" + name + r":\s*([^\s]+)\s*-->", body)
    if len(values) > 1:
        raise IntegrityError(f"duplicate {name} marker")
    return values[0] if values else None


def validate_snapshot(snapshot):
    """Validate a complete REST issue array and return a deterministic registry.

    Closed completed canonical work remains in the graph; not_planned or
    archived work cannot be an executable prerequisite. Historical details are
    preserved by body hash, but cannot contribute metadata or dependencies.
    """
    if not isinstance(snapshot, list):
        raise IntegrityError("snapshot must be a JSON array of REST issues")
    issues, canonical, aliases, entries = {}, {}, {}, {}
    for raw in snapshot:
        if not isinstance(raw, dict):
            raise IntegrityError("each issue must be an object")
        if "pull_request" in raw:
            continue
        required = {"number", "body", "labels", "state", "state_reason", "updated_at"}
        if not required <= raw.keys():
            raise IntegrityError(f"issue missing REST fields: {sorted(required - raw.keys())}")
        number = raw["number"]
        if type(number) is not int or number <= 0 or number in issues:
            raise IntegrityError(f"invalid or duplicate issue number: {number}")
        if raw["state"] not in ("open", "closed") or not isinstance(raw["updated_at"], str):
            raise IntegrityError(f"#{number}: invalid REST state or updated_at")
        if raw["body"] is not None and not isinstance(raw["body"], str):
            raise IntegrityError(f"#{number}: invalid body")
        if not isinstance(raw["labels"], list):
            raise IntegrityError(f"#{number}: labels must be an array")
        labels = [x.get("name") if isinstance(x, dict) else x for x in raw["labels"]]
        if any(not isinstance(x, str) for x in labels):
            raise IntegrityError(f"#{number}: invalid labels")
        body = raw["body"] or ""
        controlling = body if "planning:canonical" in labels else re.split(r"<details\b", body, maxsplit=1, flags=re.I)[0]
        key = marker(controlling, "plan-key")
        program_entry = marker(controlling, "program-entry")
        reference_key = marker(controlling, "reference-key") or program_entry
        revision = marker(controlling, "plan-revision")
        record = dict(number=number, title=raw.get("title", ""), state=raw["state"],
                      state_reason=raw["state_reason"], labels=sorted(labels),
                      updated_at=raw["updated_at"], body_sha256=hashlib.sha256(body.encode()).hexdigest())
        issues[number] = record
        if not key and not reference_key:
            if "planning:reference" in labels or "planning:canonical" in labels:
                raise IntegrityError(f"#{number}: planning label without stable key")
            continue
        if key and "planning:canonical" not in labels:
            record.update(key=key, revision=revision, kind="unclassified")
            entries[number] = record
            continue
        if not revision:
            raise IntegrityError(f"#{number}: missing revision authority")
        if key and reference_key:
            raise IntegrityError(f"#{number}: both canonical and reference keys")
        record.update(key=key or reference_key, revision=revision)
        if key and raw["state"] == "closed" and raw["state_reason"] in ("not_planned", "duplicate"):
            record.update(kind="retired")
            entries[number] = record
            continue
        plain = controlling.replace("**", "")
        if reference_key:
            targets = re.findall(r"Execute the canonical roadmap at\s*#(\d+)" if program_entry else r"Canonical owner:\s*#(\d+)", plain)
            if len(targets) != 1 or "planning:reference" not in labels:
                raise IntegrityError(f"#{number}: invalid alias mapping or missing reference label")
            record.update(kind="reference", canonical_issue=int(targets[0]), program_entry=bool(program_entry))
            aliases[number] = record
        else:
            if "planning:reference" in labels:
                raise IntegrityError(f"#{number}: canonical issue labeled reference")
            if key in canonical:
                raise IntegrityError(f"duplicate active key {key}: #{canonical[key]} and #{number}")
            canonical[key] = number
            parent = re.findall(r"Parent epic:\s*#(\d+)", plain, re.I)
            wave = re.findall(r"\bWave:\s*(\d+)\b", plain)
            child_body = section(controlling, "Child issues")
            kind = "master" if key == "ROADMAP" else "epic" if child_body else "task"
            if kind == "task" and (len(parent) != 1 or len(wave) != 1):
                raise IntegrityError(f"#{number}: task needs exactly one parent epic and wave")
            prerequisites = dependencies(controlling) if kind == "task" else []
            if len(prerequisites) != len(set(prerequisites)):
                raise IntegrityError(f"#{number}: duplicate dependencies")
            record.update(kind=kind, epic=int(parent[0]) if parent else None,
                          wave=int(wave[0]) if wave else None, dependencies=sorted(prerequisites),
                          children=refs(child_body), acceptance_items=len(re.findall(r"(?m)^\s*- \[[ xX]\]", controlling)))
            if wave:
                wave_labels = [x for x in labels if x.startswith("wave:")]
                if wave_labels and wave_labels != [f"wave:{int(wave[0])}"]:
                    raise IntegrityError(f"#{number}: body and label wave disagree")
        entries[number] = record

    masters = [r for r in entries.values() if r["kind"] == "master"]
    if len(masters) != 1:
        raise IntegrityError("exactly one ROADMAP master is required")
    tasks = {n: r for n, r in entries.items() if r["kind"] == "task"}
    epics = {n: r for n, r in entries.items() if r["kind"] == "epic"}
    for number, alias in aliases.items():
        target = entries.get(alias["canonical_issue"])
        if target is None or target["kind"] not in ("master", "epic", "task"):
            raise IntegrityError(f"#{number}: alias must resolve directly to canonical work")
    for number, epic in epics.items():
        children = epic["children"]
        expected = sorted(n for n, task in tasks.items() if task["epic"] == number)
        if len(children) != len(set(children)) or sorted(children) != expected:
            raise IntegrityError(f"#{number}: epic membership differs from canonical task parents")
    indegree = {n: 0 for n in tasks}
    successors = {n: [] for n in tasks}
    for number, task in tasks.items():
        if task["epic"] not in epics:
            raise IntegrityError(f"#{number}: dangling parent epic #{task['epic']}")
        for dep in task["dependencies"]:
            if dep not in issues:
                raise IntegrityError(f"#{number}: dangling dependency #{dep}")
            if dep not in tasks:
                raise IntegrityError(f"#{number}: dependency #{dep} is not canonical atomic work")
            prerequisite = tasks[dep]
            if prerequisite["state_reason"] == "not_planned" or any("archived" in label for label in prerequisite["labels"]):
                raise IntegrityError(f"#{number}: dependency #{dep} is archived or not planned")
            if prerequisite["wave"] > task["wave"]:
                raise IntegrityError(f"#{number}: dependency #{dep} is in a later wave")
            indegree[number] += 1
            successors[dep].append(number)
    ready = [(tasks[n]["wave"], n) for n, degree in indegree.items() if degree == 0]
    heapq.heapify(ready)
    order = []
    while ready:
        _, number = heapq.heappop(ready)
        order.append(number)
        for successor in sorted(successors[number]):
            indegree[successor] -= 1
            if not indegree[successor]:
                heapq.heappush(ready, (tasks[successor]["wave"], successor))
    if len(order) != len(tasks):
        raise IntegrityError(f"dependency cycle among: {sorted(n for n, degree in indegree.items() if degree)}")
    master_body = (next(i for i in snapshot if i.get("number") == masters[0]["number"])["body"] or "").split("<details", 1)[0]
    if sorted(refs(section(master_body, "Canonical epics"))) != sorted(epics):
        raise IntegrityError("master epic membership differs from canonical registry")
    counts = {"tasks": len(tasks), "epics": len(epics), "references": sum(not r["program_entry"] for r in aliases.values()), "program_entries": sum(r["program_entry"] for r in aliases.values()), "snapshot_issues": len(issues)}
    summary = re.search(r"(\d+) canonical atomic issues across (\d+) epics", master_body)
    if summary and (int(summary[1]), int(summary[2])) != (len(tasks), len(epics)):
        raise IntegrityError("master declared counts differ from generated counts")
    for n, count in re.findall(r"#(\d+)[^\n]*\((\d+) children\)", section(master_body, "Canonical epics")):
        if len(epics[int(n)]["children"]) != int(count):
            raise IntegrityError(f"master declared child count differs for epic #{n}")
    return {"schema_version": 1, "scope": "offline structural validation only; no completion or coverage certification",
            "counts": counts, "topological_order": order,
            "waves": {str(w): [n for n in order if tasks[n]["wave"] == w] for w in sorted({t["wave"] for t in tasks.values()})},
            "entries": [entries[n] for n in sorted(entries)]}


def render_index(registry):
    by_number = {r["number"]: r for r in registry["entries"]}
    lines = ["# Generated roadmap execution index", "", "Generated from a validated snapshot. Ordering does not establish readiness, acceptance coverage, or implementation completion.", "", f"Counts: {json.dumps(registry['counts'], sort_keys=True)}", ""]
    for wave, numbers in registry["waves"].items():
        lines.extend([f"## Wave {wave}", "", "| Order | Issue | Key | Epic | Prerequisites | State |", "| --- | --- | --- | --- | --- | --- |"])
        for n in numbers:
            r = by_number[n]
            lines.append(f"| {registry['topological_order'].index(n) + 1} | #{n} | {r['key']} | #{r['epic']} | {', '.join('#' + str(d) for d in r['dependencies']) or 'None'} | {r['state']} |")
        lines.append("")
    return "\n".join(lines)


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("snapshot", type=Path)
    parser.add_argument("--output", required=True, type=Path, help="directory for registry.json and execution-index.md")
    args = parser.parse_args(argv)
    try:
        source = args.snapshot.read_bytes()
        registry = validate_snapshot(json.loads(source))
        registry["snapshot_sha256"] = hashlib.sha256(source).hexdigest()
    except (IntegrityError, ValueError, OSError) as exc:
        parser.exit(1, f"roadmap integrity failed: {exc}\n")
    args.output.mkdir(parents=True, exist_ok=True)
    (args.output / "registry.json").write_text(json.dumps(registry, indent=2, sort_keys=True) + "\n")
    (args.output / "execution-index.md").write_text(render_index(registry))
    print(json.dumps(registry["counts"], sort_keys=True))


if __name__ == "__main__":
    main()
