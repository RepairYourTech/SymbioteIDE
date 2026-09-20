#!/usr/bin/env python3
"""The competitor registry: its schema, its evidence discipline, and every reading of it.

One owner of the vocabulary and one owner of each refusal. What a claim may say about a product
is a *state* from a closed list, an *observation* in the claimant's own words, and *evidence* —
a source URL or repository path, the date it was retrieved, an excerpt of what it actually says,
and a confidence. Marketing in the product's own voice is `ANNOUNCED`; two secondary write-ups
are `INFERRED`; nothing retrieved is `UNKNOWN` with a precise research gap rather than a blank.
`VERIFIED_CURRENT` is the one state that needs authoritative evidence behind it, and the only one
a stale date can turn into `STALE`.

The readings that follow are the contract:

* `problems` — every refusal this registry must survive, each naming its subject;
* `delta` — what changed between two snapshots, which capabilities a `watch` row binds to a
  Symbiote requirement or decision, and which of those decisions are locked;
* `refresh` — the claims a focused research pass would touch, refusing a plan that leaves the
  dimension it names or a change to a locked row.

    python3 planning/research/registry.py --check
    python3 planning/research/registry.py --delta 2026-08-15 2026-09-20
    python3 planning/research/registry.py --refresh runtime --plan changes.json

`--check` reads the committed registry and every snapshot in `snapshots/` and fails on the first
refusal, naming it. It writes nothing.
"""
from __future__ import annotations

import argparse
import datetime
import json
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parents[2]
REGISTRY = ROOT / "planning/research/registry.json"
SNAPSHOTS = ROOT / "planning/research/snapshots"

SCHEMA_VERSION = 1

# What a claim may be. `VERIFIED_CURRENT` is the only state that claims to be current fact.
STATES = ("VERIFIED_CURRENT", "ANNOUNCED", "EXPERIMENTAL", "INFERRED", "STALE", "UNKNOWN")
# What kind of thing stands behind a claim. Only the first four are authoritative.
AUTHORITATIVE = ("official_site", "official_docs", "official_repo", "official_changelog", "official_blog")
COMMUNITY = "community"
SOURCE_KINDS = AUTHORITATIVE + (COMMUNITY,)
CONFIDENCE = ("high", "medium", "low")
# How deep a harness integration is. The issue's own ladder, normalized by semantics, not wording.
GRADES = ("terminal_compatible", "detected", "managed", "structured", "orchestrator_grade", "certified")
# How the product actually talks to the agent. `none` is an honest answer for a claim with no
# structured interface; `pty` is a terminal, and a terminal is not a structured transport.
TRANSPORTS = ("pty", "sdk", "app_server", "rpc", "jsonl", "acp", "none")
STRUCTURED = ("sdk", "app_server", "rpc", "jsonl", "acp")
# A grade above `managed` claims the product is not merely wrapping a terminal.
DEEP_GRADES = ("structured", "orchestrator_grade", "certified")
CLASSES = (
    "direct_ade",
    "coding_agent_client",
    "orchestration_tool",
    "spec_artifact_product",
    "codebase_knowledge_product",
    "provider_native_app",
    "remote_mobile_client",
    "adjacent_substitute",
)


class Refused(Exception):
    """A reading that cannot be made: the registry, a snapshot or a plan says something it cannot."""


def read(path: pathlib.Path) -> dict:
    try:
        return json.loads(path.read_text())
    except FileNotFoundError as error:
        raise Refused(f"no registry at {path}") from error
    except json.JSONDecodeError as error:
        raise Refused(f"{path} is not readable JSON: {error}") from error


def days_between(earlier: str, later: str) -> int:
    first = datetime.date.fromisoformat(earlier)
    second = datetime.date.fromisoformat(later)
    return (second - first).days


class Registry:
    """The committed registry, with the taxonomy's capabilities flattened once."""

    def __init__(self, document: dict, origin: str):
        self.document = document
        self.origin = origin
        self.schema_version = document.get("schema_version")
        self.observed_at = document.get("observed_at", "")
        self.dimensions = document.get("taxonomy", {}).get("dimensions", [])
        self.capabilities = {}  # capability id -> (dimension id, capability)
        for dimension in self.dimensions:
            for capability in dimension.get("capabilities", []):
                self.capabilities[capability.get("id")] = (dimension.get("id"), capability)
        self.universe = {entry.get("id"): entry for entry in document.get("universe", [])}
        self.products = {entry.get("id"): entry for entry in document.get("products", [])}
        self.watch = document.get("watch", [])

    def dimension_of(self, capability: str) -> str | None:
        found = self.capabilities.get(capability)
        return found[0] if found else None

    def claims(self) -> list[tuple[str, dict]]:
        return [(product.get("id"), claim)
                for product in self.products.values()
                for claim in product.get("claims", [])]


def newest(evidence: list[dict]) -> str | None:
    """The most recent retrieval date among the authoritative evidence, or None."""
    dates = [entry.get("retrieved_at", "") for entry in evidence
             if entry.get("kind") in AUTHORITATIVE]
    return max(dates) if dates else None


def problems(registry: Registry, history_ids: set[str] | None = None) -> list[str]:
    """Every refusal the committed registry must survive, each naming its subject.

    `history_ids` is every product id a snapshot carries, when the caller has read them: it is what
    lets a rename be held to a name this registry's history really used.
    """
    found: list[str] = []

    if registry.schema_version != SCHEMA_VERSION:
        found.append(f"the registry declares schema {registry.schema_version!r}, and this reader "
                     f"replays {SCHEMA_VERSION}")
    try:
        datetime.date.fromisoformat(registry.observed_at)
    except ValueError:
        found.append(f"the registry's observed date {registry.observed_at!r} is not a date")

    for capability, (_, entry) in registry.capabilities.items():
        if entry.get("window_days") is None or not isinstance(entry.get("window_days"), int):
            found.append(f"taxonomy capability {capability} states no staleness window in days")
    for dimension in registry.dimensions:
        if not dimension.get("capabilities"):
            found.append(f"taxonomy dimension {dimension.get('id')} declares no capability")

    # A dossier for everything the universe names, and no dossier for something it does not.
    for product in registry.universe:
        if product not in registry.products:
            found.append(f"product {product} is in the universe and carries no dossier")
    for product in registry.products:
        if product not in registry.universe:
            found.append(f"product {product} carries a dossier and is not in the universe")

    for product, entry in registry.products.items():
        if entry.get("class") not in CLASSES:
            found.append(f"product {product} is classified {entry.get('class')!r}, which is not a "
                         f"word this registry tracks")
        claims = entry.get("claims", [])
        if not claims and not (entry.get("gap") or "").strip():
            found.append(f"product {product} states no claim and no research gap")
        seen: set[str] = set()
        for claim in claims:
            capability = claim.get("capability")
            if capability not in registry.capabilities:
                found.append(f"product {product} claims {capability!r}, which the taxonomy does "
                             f"not carry")
                continue
            if capability in seen:
                found.append(f"product {product} claims {capability} twice")
            seen.add(capability)
            found.extend(claim_problems(registry, product, claim))

    found.extend(rename_problems(registry, history_ids))
    return found


def claim_problems(registry: Registry, product: str, claim: dict) -> list[str]:
    """One claim: its state, the evidence behind it, its age, and a harness grade if it carries one."""
    found: list[str] = []
    capability = claim.get("capability")
    state = claim.get("state")
    evidence = claim.get("evidence", [])
    if state not in STATES:
        found.append(f"{product}/{capability} states {state!r}, which is not a capability state")
        return found

    for entry in evidence:
        if entry.get("kind") not in SOURCE_KINDS:
            found.append(f"{product}/{capability} cites a source of kind {entry.get('kind')!r}, "
                         f"which this registry does not read")
        if entry.get("confidence") not in CONFIDENCE:
            found.append(f"{product}/{capability} records confidence {entry.get('confidence')!r}")
        for field in ("source", "retrieved_at", "excerpt"):
            if not (entry.get(field) or "").strip():
                found.append(f"{product}/{capability} cites evidence with no {field}")
        try:
            datetime.date.fromisoformat(entry.get("retrieved_at", ""))
        except ValueError:
            found.append(f"{product}/{capability} cites evidence retrieved {entry.get('retrieved_at')!r}")

    authoritative = [entry for entry in evidence if entry.get("kind") in AUTHORITATIVE]
    if state == "VERIFIED_CURRENT":
        if not authoritative:
            behind = ", ".join(sorted({entry.get("kind", "?") for entry in evidence})) or "nothing"
            found.append(f"{product}/{capability} reads VERIFIED_CURRENT and {behind} stands "
                         f"behind it, which is not authoritative evidence")
        elif not authoritative[0].get("excerpt"):
            found.append(f"{product}/{capability} reads VERIFIED_CURRENT and its authoritative "
                         f"evidence carries no excerpt")
    if not authoritative and not (claim.get("gap") or "").strip():
        found.append(f"{product}/{capability} has no authoritative evidence and states no research "
                     f"gap")
    if state in ("INFERRED", "UNKNOWN") and authoritative:
        found.append(f"{product}/{capability} reads {state} with an authoritative source behind "
                     f"it, so it is at least ANNOUNCED rather than a guess")

    # Freshness: the window is the capability's, measured from the registry's own observation.
    window = registry.capabilities.get(capability, (None, {}))[1].get("window_days")
    latest = newest(evidence)
    if window and latest and registry.observed_at:
        age = days_between(latest, registry.observed_at)
        if age > window and state == "VERIFIED_CURRENT":
            found.append(f"{product}/{capability} reads VERIFIED_CURRENT on evidence {age} days "
                         f"old, past its {window}-day window; a stale claim reads STALE")
        if age < 0:
            found.append(f"{product}/{capability} cites evidence retrieved after the registry was "
                         f"observed")

    graded = registry.capabilities.get(capability, (None, {}))[1].get("grade")
    if graded and not (state == "UNKNOWN" and claim.get("grade") is None):
        # A claim that read nothing states no grade: there is no interface to grade, and the gap
        # carries what a research pass must retrieve instead.
        found.extend(grade_problems(product, capability, claim, authoritative))
    elif not graded and ("grade" in claim or "transport" in claim):
        found.append(f"{product}/{capability} states a support grade for a capability the "
                     f"taxonomy does not grade")
    return found


def grade_problems(product: str, capability: str, claim: dict, authoritative: list[dict]) -> list[str]:
    """A harness-support claim: which grade, over which transport, on whose evidence."""
    found: list[str] = []
    grade = claim.get("grade")
    transport = claim.get("transport")
    if grade not in GRADES:
        found.append(f"{product}/{capability} states grade {grade!r}, which is not a support grade")
    if transport not in TRANSPORTS:
        found.append(f"{product}/{capability} states transport {transport!r}, which is not a transport")
    if grade in DEEP_GRADES:
        if transport not in STRUCTURED:
            found.append(f"{product}/{capability} is graded {grade} over {transport!r}: a terminal "
                         f"or an absent interface is not evidence of structured integration")
        if not authoritative:
            found.append(f"{product}/{capability} is graded {grade} with no authoritative evidence "
                         f"of the interface")
    if grade == "terminal_compatible" and transport not in ("pty", "none"):
        found.append(f"{product}/{capability} is graded terminal_compatible over {transport!r}, "
                     f"which is a structured transport rather than a terminal")
    return found


def rename_problems(registry: Registry, history_ids: set[str] | None = None) -> list[str]:
    """`renamed_from` names the identity an entry replaces, and replaces something that is gone.

    An identity that survives a rename is the id the history uses, so the name it replaced must be
    one the history carried — a rename of something no snapshot ever held is a claim about the past
    this registry cannot support.
    """
    found: list[str] = []
    for product, entry in registry.products.items():
        replaced = entry.get("renamed_from")
        if replaced and replaced in registry.products:
            found.append(f"product {product} declares itself the rename of {replaced}, and {replaced} "
                         f"is still in the registry: a rename replaces what it names")
        if replaced and history_ids is not None and replaced not in history_ids:
            found.append(f"product {product} declares itself the rename of {replaced}, which no "
                         f"snapshot carries")
    for product, claim in registry.claims():
        replaced = claim.get("renamed_from")
        if replaced and replaced in registry.capabilities:
            found.append(f"{product}/{claim.get('capability')} declares itself the rename of "
                         f"{replaced}, and the taxonomy still carries {replaced}")
    return found


def load_snapshot(root: pathlib.Path, name: str) -> Registry:
    """One historical snapshot, named by its own date."""
    path = root / "planning/research/snapshots" / f"{name}.json"
    registry = Registry(read(path), str(path))
    if registry.observed_at != name:
        raise Refused(f"snapshot {path.name} was observed at {registry.observed_at!r}, and its own "
                      f"name says {name}")
    found = problems(registry)
    if found:
        raise Refused(f"snapshot {path.name} does not read: {found[0]}")
    return registry


def delta(before: Registry, after: Registry) -> dict:
    """What changed between two snapshots, and which Symbiote rows a `watch` binds to it.

    A rename is reported as a rename rather than as a removal beside an addition: the identity the
    history uses is the id, so an entry declaring `renamed_from` an id the earlier snapshot carried
    is the same subject under a new name, and a delta that reported it as two changes would make a
    stable identity look like a new product or a lost capability.
    """
    changes = []
    # A renamed product keeps its claims: the identity is the id, and the id it was is mapped onto the
    # one it became before anything is compared, so a rename does not read as a loss beside a gain.
    owners = {}
    for product, entry in after.products.items():
        replaced = entry.get("renamed_from")
        if replaced and replaced in before.products:
            owners[replaced] = product
            changes.append({"product": product, "capability": None, "change": "renamed",
                            "from": replaced, "to": product})
    before_claims = dict(((owners.get(owner, owner), one.get("capability")), one)
                         for owner, one in before.claims())
    renamed = {}
    for product, claim in after.claims():
        replaced = claim.get("renamed_from")
        if replaced and (product, replaced) in before_claims:
            renamed[(product, claim.get("capability"))] = replaced
            changes.append({"product": product, "capability": claim.get("capability"),
                            "change": "renamed", "from": replaced, "to": claim.get("capability")})
    for product, claim in after.claims():
        capability = claim.get("capability")
        if (product, capability) in renamed:
            continue
        earlier = before_claims.get((product, capability))
        if earlier is None:
            changes.append({"product": product, "capability": capability, "change": "added",
                            "from": None, "to": claim.get("state")})
        elif earlier.get("state") != claim.get("state"):
            changes.append({"product": product, "capability": capability, "change": "changed",
                            "from": earlier.get("state"), "to": claim.get("state")})
        elif earlier.get("observation") != claim.get("observation"):
            changes.append({"product": product, "capability": capability, "change": "revised",
                            "from": earlier.get("state"), "to": claim.get("state")})
    replaced_capabilities = {(owner, old) for (owner, _), old in renamed.items()}
    for product, claim in before.claims():
        capability = claim.get("capability")
        owner = owners.get(product, product)
        if (owner, capability) in replaced_capabilities:
            continue
        if (owner, capability) not in after_claims(after):
            changes.append({"product": owner, "capability": capability, "change": "removed",
                            "from": claim.get("state"), "to": None})

    touched = {change["capability"] for change in changes if change["capability"]}
    affected, locked = [], []
    for row in after.watch:
        if row.get("capability") not in touched:
            continue
        entry = {"capability": row.get("capability"), "requirements": row.get("requirements", []),
                 "decisions": row.get("decisions", []), "locked": bool(row.get("locked"))}
        (locked if entry["locked"] else affected).append(entry)
    return {"from": before.observed_at, "to": after.observed_at, "changes": changes,
            "affected": affected, "locked": locked,
            "unwatched": sorted(touched - {row.get("capability") for row in after.watch})}


def after_claims(registry: Registry) -> set[tuple[str, str]]:
    """The product/capability pairs a snapshot carries, as one set."""
    return {(product, claim.get("capability")) for product, claim in registry.claims()}


def refresh(registry: Registry, dimension: str, plan: list[dict]) -> list[str]:
    """The claims a focused refresh may update: only the dimension it names, and no locked row.

    A plan is what a research pass intends to re-observe: capability ids, optionally with the
    product they belong to. What it may not do is leave the dimension it names, or change a
    capability a locked `watch` row binds to a decision this repository has already settled.
    """
    found: list[str] = []
    if dimension not in {entry.get("id") for entry in registry.dimensions}:
        found.append(f"no taxonomy dimension is named {dimension!r}")
        return found
    for item in plan:
        capability = item.get("capability")
        where = registry.dimension_of(capability)
        if where is None:
            found.append(f"the plan names {capability!r}, which the taxonomy does not carry")
        elif where != dimension:
            found.append(f"a refresh of {dimension} names {capability}, which belongs to {where}")
    for row in registry.watch:
        if row.get("locked") and row.get("capability") in {item.get("capability") for item in plan}:
            found.append(f"the plan re-observes {row.get('capability')}, which a locked row binds to "
                         f"{', '.join(row.get('decisions', [])) or 'a settled decision'}")
    return found


def snapshots() -> list[str]:
    return sorted(path.stem for path in SNAPSHOTS.glob("*.json"))


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--check", action="store_true",
                        help="refuse the committed registry and every snapshot, or pass them")
    parser.add_argument("--delta", nargs=2, metavar=("BEFORE", "AFTER"),
                        help="print what changed between two snapshots")
    parser.add_argument("--refresh", metavar="DIMENSION",
                        help="read a focused-refresh plan (--plan) against one dimension")
    parser.add_argument("--plan", help="a JSON list of {capability, product} a refresh intends")
    arguments = parser.parse_args(argv)

    if arguments.delta:
        before, after = arguments.delta
        try:
            report = delta(load_snapshot(ROOT, before), load_snapshot(ROOT, after)
                           if after != "registry" else registry())
        except Refused as error:
            print(f"refused: {error}", file=sys.stderr)
            return 1
        print(json.dumps(report, indent=2))
        return 0

    if arguments.refresh:
        try:
            plan = json.loads(pathlib.Path(arguments.plan).read_text()) if arguments.plan else []
        except (OSError, json.JSONDecodeError) as error:
            print(f"refused: the plan is not readable: {error}", file=sys.stderr)
            return 1
        found = refresh(registry(), arguments.refresh, plan)
        if found:
            for refusal in found:
                print(f"refused: {refusal}", file=sys.stderr)
            return 1
        print(f"{arguments.refresh}: the plan stays inside the dimension it names")
        return 0

    found: list[str] = []
    committed = registry()
    history: set[str] = set(committed.products)
    loaded = []
    for name in snapshots():
        try:
            loaded.append(load_snapshot(ROOT, name))
            history.update(loaded[-1].products)
        except Refused as error:
            found.append(str(error))
    found.extend(problems(committed, history))
    if found:
        for refusal in found:
            print(f"refused: {refusal}", file=sys.stderr)
        return 1
    print(f"registry {registry().observed_at}: {len(registry().products)} products, "
          f"{len(registry().claims())} claims, {len(registry().capabilities)} taxonomy capabilities, "
          f"{len(snapshots())} snapshots, 0 refusals")
    return 0


def registry() -> Registry:
    return Registry(read(REGISTRY), str(REGISTRY))


if __name__ == "__main__":
    sys.exit(main())
