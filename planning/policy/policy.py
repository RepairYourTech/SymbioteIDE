#!/usr/bin/env python3
"""The licensing, trust and data-ownership record: what this repository promises, and what holds it.

#174 owns the legal and trust model an open-source tool that controls repositories, credentials,
terminals and third-party agents needs. Most of that model is a *statement* — which licence covers
which artifact class, what a publisher must prove, what a contributor grants — and a statement is
only worth the reading that keeps it true of the tree. This module is that reading:

* `classes` — the five licensing boundaries (the core product, official packs, third-party
  extensions, catalog metadata, and hosted services), each with what it covers, what it does *not*
  cover, and whether a licence has been selected for it. A class that selects a licence the tree
  does not carry, or that states no boundary, is refused.
* `obligations` — one row per obligation the issue names, each at one of four statuses: `built`
  (with the path that shows it and the case that exercises it), `delegated` or `pending` (with the
  issue that owns it), or `not_applicable` (with the rationale). A row claiming `built` with no
  evidence path in the tree, or with a case no file declares, is refused — the fault the issue's
  own review clause names: a claim where the tree has nothing.
* `stability` — every public contract document with the level it is held at, the levels' own rules,
  and the level a workspace version below 1.0 cannot support.
* `supply_chain.dependencies` — every third-party dependency this workspace links, with the licence
  it is used under and the class it lands in, read back from the manifests rather than restated, and
  refused when a licence the distribution boundary cannot honour reaches a class that ships.
* `supply_chain.generated` — every committed artifact this repository generates, with the program
  that wrote it, the command that re-derives it and the case that holds it.
* `applicability` — security, privacy, accessibility, performance and cross-platform applicability,
  each marked required, not applicable with a rationale, or separately tracked against an owner.
* `runtime_assumptions` — the licence, authentication and telemetry assumption this project makes
  about every product in #171's universe, stated rather than left implicit, because the issue's own
  verification clause is that no issue depends on an *unstated* assumption.

Nothing here restates a fact another owner holds: the products come from `planning/research`, the
issue numbers from the generated program, who depends on what from the manifests, and the case names
from the files that declare them.

The readings:

    python3 planning/policy/policy.py --check
    python3 planning/policy/policy.py --classes
    python3 planning/policy/policy.py --audit
    python3 planning/policy/policy.py --generated

`--check` refuses the committed record on the first rule it breaks, naming its subject, then prints
what it read. `--classes` prints the five boundaries. `--audit` prints the runtime assumptions with
the fields still unknown, and the obligations this tree has not built. `--generated` prints the
provenance table. None of them writes anything.
"""
from __future__ import annotations

import argparse
import datetime
import json
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parents[2]
POLICY = ROOT / "planning/policy/policy.json"
PROGRAM = ROOT / "planning/integrity/generated/registry.json"
RESEARCH = ROOT / "planning/research"
sys.path.insert(0, str(RESEARCH))

import registry as research  # noqa: E402  (the registry owns what a product is)

SCHEMA_VERSION = 1

# The artifact classes the issue names, once each: the product, the packs this project publishes,
# third-party extensions, the catalog metadata about them, and any hosted service.
CLASSES = ("core", "official_pack", "extension", "catalog_metadata", "hosted_service")
# What an obligation is about, taken from the areas the issue's own scope is written in.
AREAS = ("data", "extension_trust", "publishing", "disclosure", "supply_chain")
# How an obligation is accounted for. `built` is the only one that claims the tree does something.
STATUSES = ("built", "delegated", "pending", "not_applicable")
# How an obligation the tree has not built is accounted for: an issue that owns it, or a rationale.
ACCOUNTED = ("delegated", "pending")
# The applicability marks the issue's own criterion asks for.
MARKS = ("required", "not_applicable", "separately_tracked")
CONCERNS = ("security", "privacy", "accessibility", "performance", "cross_platform")
# A contract's stability level, and the two that cannot be claimed below 1.0: a promise of
# stability or of a deprecation window is a promise about a version this workspace has not shipped.
LEVELS = ("experimental", "stable", "deprecated")
LEVELS_NEEDING_1_0 = ("stable", "deprecated")
# The manifest tables a third-party dependency can be declared in, and the class each implies.
# A crate under `dev-dependencies` never reaches a consumer, one under `build-dependencies` reaches
# the build, and one under `dependencies` reaches both the build and the shipped artifact — so the
# class a dependency lands in is the widest of the tables it is declared in.
KINDS = {"dependencies": "runtime", "build-dependencies": "build", "dev-dependencies": "test"}
WIDEST = ("runtime", "build", "test")
# The licences this reader knows, so an id outside them is a typo rather than a policy.
LICENSES = (
    "MIT", "Apache-2.0", "BSD-2-Clause", "BSD-3-Clause", "ISC", "Zlib", "Unicode-3.0",
    "CC0-1.0", "Unlicense", "MPL-2.0", "LGPL-3.0-only", "GPL-2.0-only", "GPL-3.0-only",
    "GPL-2.0-or-later", "GPL-3.0-or-later", "AGPL-3.0-only", "AGPL-3.0-or-later", "SSPL-1.0",
    "BUSL-1.1", "Elastic-2.0", "Commons-Clause",
)
# The obligations the issue's own criteria are made of. Each has to be accounted for; a record that
# simply stops mentioning one is the deletion this list exists to refuse.
REQUIRED_OBLIGATIONS = (
    "data.local-first-ownership",
    "data.consent-before-activation",
    "data.no-hidden-exfiltration",
    "data.telemetry-opt-in",
    "data.optional-hosted-services",
    "extension.publisher-identity",
    "extension.signing",
    "extension.permission-review",
    "extension.revocation",
    "extension.vulnerable-package-response",
    "publishing.contribution",
    "publishing.trademark",
    "publishing.fork",
    "publishing.commercial-hosting",
    "disclosure.coordinated-disclosure",
    "supply_chain.dependency-license",
    "supply_chain.artifact-provenance",
)
# The assumption fields the runtime audit states for every product, and the one value that means
# "not retrieved": an unknown with a named gap is a stated assumption, a blank is not.
ASSUMPTIONS = ("license", "authentication", "telemetry")
UNKNOWN = "UNKNOWN"
# A case is a name a file declares, in this repository's two test languages: `def name(` in a Python
# suite, `fn name(` in a Rust one. What the case *asserts* is the case's own business — this reader
# holds that the name exists, which is the same boundary the architecture contract's case states.
DECLARATION = re.compile(r"^\s*(?:pub\s+)?(?:async\s+)?(?:def|fn)\s+(?P<name>[A-Za-z_]\w*)\s*[(<]",
                         re.MULTILINE)
# A token in a re-derivation command that names a path, and the placeholder form that does not.
PATHLIKE = re.compile(r"[/.]")
PLACEHOLDER = re.compile(r"^<.*>$")
OR_ARMS = re.compile(r"\s+OR\s+")
AND_TOKENS = re.compile(r"\s+AND\s+")


class Refused(Exception):
    """A reading that cannot be made: the record, the program or the tree says something it cannot."""


def read(path: pathlib.Path) -> dict:
    try:
        return json.loads(path.read_text())
    except FileNotFoundError as error:
        raise Refused(f"no file at {path}") from error
    except json.JSONDecodeError as error:
        raise Refused(f"{path} is not readable JSON: {error}") from error


class Tree:
    """What this record is read against: the repository, or a copy of it a case mutates.

    Every fact comes from the tree rather than from the record — the manifests of who depends on
    what, the root's licence files, the contract documents, the workspace version, and the case
    names the suites declare — so the same reading holds a synthetic tree in a test and the real one
    in CI.
    """

    def __init__(self, root: pathlib.Path = ROOT):
        self.root = root
        self._cases: set[str] | None = None

    def at(self, relative: str) -> pathlib.Path:
        return self.root / relative

    def exists(self, relative: str) -> bool:
        return self.at(relative).exists()

    def files(self, pattern: str) -> list[str]:
        return sorted(str(one.relative_to(self.root).as_posix())
                      for one in self.root.glob(pattern) if one.is_file())

    def manifests(self) -> dict[str, dict]:
        """Each workspace crate with the third-party dependencies its own manifest declares, and the
        widest table each is declared in — path dependencies are this workspace's own and are not
        third-party."""
        import tomllib

        found: dict[str, dict] = {}
        for manifest in sorted(self.root.glob("crates/*/Cargo.toml")):
            document = tomllib.loads(manifest.read_text())
            name = document.get("package", {}).get("name")
            if not name:
                continue
            declared: dict[str, str] = {}
            for table, kind in KINDS.items():
                for dependency, spec in (document.get(table) or {}).items():
                    if isinstance(spec, dict) and "path" in spec:
                        continue
                    if kind == "runtime" or dependency not in declared:
                        if dependency not in declared or WIDEST.index(kind) < WIDEST.index(declared[dependency]):
                            declared[dependency] = kind
            found[name] = {"manifest": str(manifest.relative_to(self.root).as_posix()),
                           "declared": declared}
        return found

    def workspace_version(self) -> str:
        import tomllib

        document = tomllib.loads((self.root / "Cargo.toml").read_text())
        return document.get("workspace", {}).get("package", {}).get("version", "")

    def license_files(self) -> list[str]:
        return sorted(one.name for one in self.root.glob("LICENSE*")) \
            + sorted(one.name for one in self.root.glob("COPYING*"))

    def cases(self) -> set[str]:
        """Every name this tree declares as a case, in its Python suites and its Rust tests: the
        maintenance suites under `planning/`, the runner's under `spikes/`, and every Rust test and
        in-module test in `crates/`."""
        if self._cases is None:
            found: set[str] = set()
            for pattern in ("planning/**/*.py", "spikes/**/*.py", "crates/**/*.rs"):
                for path in self.root.glob(pattern):
                    if "__pycache__" in path.parts or "target" in path.parts:
                        continue
                    if path.is_file():
                        found.update(match.group("name")
                                     for match in DECLARATION.finditer(path.read_text(errors="replace")))
            self._cases = found
        return self._cases


def expressions(licence: str) -> list[list[str]]:
    """A licence expression read as its alternatives: `MIT OR Apache-2.0` is two arms of one token
    each, `MIT AND Apache-2.0` is one arm of two, so a dependency offering a permitted alternative
    is permitted even where its other arm is not."""
    return [[token.strip().strip("()") for token in AND_TOKENS.split(arm)]
            for arm in OR_ARMS.split(licence)]


def class_problems(policy: dict, tree: Tree, program: set[int]) -> list[str]:
    """The five boundaries, and whether each license the record names is one this tree carries."""
    found: list[str] = []
    classes = policy.get("classes", [])
    seen = {}
    for entry in classes:
        name = entry.get("id") or "a class naming no id"
        if name in seen:
            found.append(f"artifact class {name} is recorded twice")
            continue
        seen[name] = entry
        if name not in CLASSES:
            found.append(f"{name!r} is not an artifact class the issue names "
                         f"({', '.join(CLASSES)})")
            continue
        for field in ("title", "covers", "boundary"):
            if not str(entry.get(field, "")).strip():
                found.append(f"artifact class {name} states no {field}, so it has no boundary")
        if entry.get("status") not in ("selected", "pending"):
            found.append(f"artifact class {name} is {entry.get('status')!r}, which is neither a "
                         f"selected licence nor a pending one")
            continue
        licence, status = entry.get("license"), entry.get("status")
        if status == "selected":
            if licence not in LICENSES:
                found.append(f"artifact class {name} selects {licence!r}, which is not a licence "
                             f"this reader knows")
            elif not tree.license_files():
                found.append(f"artifact class {name} selects {licence} and this tree carries no "
                             f"LICENSE file stating it, so the record claims a licence the tree "
                             f"does not grant")
        elif licence is not None:
            found.append(f"artifact class {name} is pending and names the licence {licence!r}: a "
                         f"boundary that has not selected one states none")
        owners = entry.get("owners", [])
        if status == "pending":
            if not owners:
                found.append(f"artifact class {name} is pending and names no owner, so nothing "
                             f"carries the selection")
            for owner in owners:
                if owner not in program:
                    found.append(f"artifact class {name} is owned by #{owner}, which this program "
                                 f"does not carry")
            if not str(entry.get("why", "")).strip():
                found.append(f"artifact class {name} is pending and states no reason, so the "
                             f"deferral is a silence")
    for name in CLASSES:
        if name not in seen:
            found.append(f"{name} is an artifact class the issue names and this record does not "
                         f"bound, so its licence would be decided by default")
    if tree.license_files() and not any(entry.get("status") == "selected" for entry in classes):
        found.append(f"this tree carries {', '.join(tree.license_files())} and no artifact class "
                     f"selects a licence, so a licence is granted by a file the record does not "
                     f"account for")
    return found


def obligation_problems(policy: dict, tree: Tree, program: set[int]) -> list[str]:
    """One row per obligation the issue names, at a status the tree can stand behind."""
    found: list[str] = []
    obligations = policy.get("obligations", [])
    seen = {}
    for entry in obligations:
        name = entry.get("id") or "an obligation naming no id"
        if name in seen:
            found.append(f"obligation {name} is recorded twice")
            continue
        seen[name] = entry
        if entry.get("area") not in AREAS:
            found.append(f"obligation {name} is filed under {entry.get('area')!r}, which is not an "
                         f"area of this issue")
        if not str(entry.get("requirement", "")).strip():
            found.append(f"obligation {name} states no requirement, so nothing is promised")
        status = entry.get("status")
        if status not in STATUSES:
            found.append(f"obligation {name} is {status!r}, which is not a status this reader "
                         f"knows ({', '.join(STATUSES)})")
            continue
        found.extend(one for one in obligation_status_problems(entry, name, status, tree, program))
    for name in REQUIRED_OBLIGATIONS:
        if name not in seen:
            found.append(f"{name} is an obligation this issue's criteria are made of and no row "
                         f"accounts for it, so it would be dropped without being answered")
    return found


def obligation_status_problems(entry: dict, name: str, status: str, tree: Tree,
                               program: set[int]) -> list[str]:
    """What each status has to carry: `built` an evidence path in the tree and a case, the two
    deferred statuses an owner, `not_applicable` a rationale — and never both at once."""
    found: list[str] = []
    evidence, case = entry.get("evidence"), entry.get("case")
    owners = entry.get("owners", [])
    if status == "built":
        if not evidence or not tree.exists(evidence):
            found.append(f"obligation {name} is built with the evidence {evidence!r}, which this "
                         f"tree does not carry, so the record claims what nothing shows")
        if not case or case not in tree.cases():
            found.append(f"obligation {name} is built naming the case {case!r}, which no file in "
                         f"this tree declares, so nothing exercises it")
        if owners:
            found.append(f"obligation {name} is built and names owners, so it is deferred and "
                         f"done at once")
    if status in ACCOUNTED:
        if not owners:
            found.append(f"obligation {name} is {status} and names no owner, so nothing carries it")
        for owner in owners:
            if owner not in program:
                found.append(f"obligation {name} is owned by #{owner}, which this program does "
                             f"not carry")
        if not str(entry.get("why", "")).strip():
            found.append(f"obligation {name} is {status} and states no reason, so the deferral is "
                         f"a silence")
        if evidence or case:
            found.append(f"obligation {name} is {status} and cites evidence, so it claims to be "
                         f"built while deferring it")
    if status == "not_applicable":
        if not str(entry.get("rationale", "")).strip():
            found.append(f"obligation {name} is not applicable and states no rationale, which is "
                         f"the mark the issue asks to be given rather than assumed")
        if owners or evidence or case:
            found.append(f"obligation {name} is not applicable and names an owner or evidence, so "
                         f"it is not a decision about applicability")
    return found


def stability_problems(policy: dict, tree: Tree) -> list[str]:
    """Every contract document at a level, every level stating its own rule, and no level claimed
    that the workspace version cannot stand behind."""
    found: list[str] = []
    stability = policy.get("stability", {})
    levels = {entry.get("id"): entry for entry in stability.get("levels", [])}
    for name in LEVELS:
        if name not in levels:
            found.append(f"stability level {name} is one the issue names and this record states no "
                         f"rule for it")
        elif not str(levels[name].get("rule", "")).strip():
            found.append(f"stability level {name} states no rule, so the level means whatever a "
                         f"reader assumes")
    for name in levels:
        if name not in LEVELS:
            found.append(f"{name!r} is not a stability level this reader knows "
                         f"({', '.join(LEVELS)})")
    default = stability.get("default_level")
    if default not in LEVELS:
        found.append(f"the default stability level is {default!r}, which is not a level this "
                     f"reader knows")
    seen: dict[str, int] = {}
    for entry in stability.get("contracts", []):
        doc = entry.get("doc")
        seen[doc] = seen.get(doc, 0) + 1
        if not doc or not tree.exists(doc):
            found.append(f"stability names {doc!r}, which this tree does not carry")
        if not str(entry.get("surface", "")).strip():
            found.append(f"stability names {doc!r} with no surface, so the contract has no family")
        level = entry.get("level", default)
        if level not in LEVELS:
            found.append(f"{doc!r} is held at {level!r}, which is not a stability level")
        elif level in LEVELS_NEEDING_1_0 and not supports(tree.workspace_version(), level):
            found.append(f"{doc!r} is held at {level}, which this workspace's version "
                         f"{tree.workspace_version()} cannot support: a promise of stability or of "
                         f"a deprecation window is a promise about a released version")
    for doc in tree.files("docs/contracts/*.md"):
        if doc not in seen:
            found.append(f"{doc} is a public contract document and no stability row holds it, so "
                         f"its level would be assumed")
    for doc, times in seen.items():
        if times > 1:
            found.append(f"{doc} is held at a stability level {times} times")
    return found


def supports(version: str, level: str) -> bool:
    """Whether a workspace version can carry a level: a promise about a released contract needs the
    release, so anything below 1.0 supports only `experimental`."""
    if level not in LEVELS_NEEDING_1_0:
        return True
    if not version:
        return False
    try:
        major = int(str(version).split(".", 1)[0])
    except ValueError:
        return False
    return major >= 1


def supply_problems(policy: dict, tree: Tree) -> list[str]:
    """The distribution boundary, and the artifacts this repository generates."""
    found: list[str] = []
    chain = policy.get("supply_chain", {})
    permitted, denied = chain.get("permitted", {}), chain.get("denied", [])
    for kind in WIDEST:
        if kind not in permitted:
            found.append(f"no licence is permitted for the {kind} class, so a dependency landing "
                         f"there is decided by nothing")
    for kind, ids in permitted.items():
        if kind not in WIDEST:
            found.append(f"{kind!r} is not a dependency class ({', '.join(WIDEST)})")
        for one in ids:
            if one not in LICENSES:
                found.append(f"{kind} permits {one!r}, which is not a licence this reader knows")
    for one in denied:
        if one not in LICENSES:
            found.append(f"{one!r} is denied and is not a licence this reader knows")
    for one in sorted(set(denied) & {idea for ids in permitted.values() for idea in ids}):
        found.append(f"{one} is both permitted and denied, so the boundary says nothing about it")
    found.extend(dependency_problems(chain.get("dependencies", []), permitted, denied, tree))
    found.extend(generated_problems(chain.get("generated", []), tree))
    return found


def dependency_problems(rows: list[dict], permitted: dict, denied: list[str],
                        tree: Tree) -> list[str]:
    """Every third-party dependency the manifests declare, recorded once with the licence it is
    used under: the record may not lose one, keep one nothing declares, or file one under the
    wrong class."""
    found: list[str] = []
    declared: dict[str, str] = {}
    for crate, manifest in tree.manifests().items():
        for dependency, kind in manifest["declared"].items():
            if dependency in declared and WIDEST.index(declared[dependency]) >= WIDEST.index(kind):
                continue
            declared[dependency] = kind
    recorded: dict[str, dict] = {}
    for row in rows:
        name = row.get("name") or "a dependency naming nothing"
        if name in recorded:
            found.append(f"dependency {name} is recorded twice")
            continue
        recorded[name] = row
        if name not in declared:
            found.append(f"dependency {name} is recorded and no crate in this workspace declares "
                         f"it, so the record carries a dependency the tree does not")
            continue
        if row.get("class") != declared[name]:
            found.append(f"dependency {name} is filed under {row.get('class')!r} and the manifests "
                         f"declare it as {declared[name]}, which is the class its licence has to "
                         f"survive")
        licence = row.get("license", "")
        arms = expressions(licence)
        for arm in arms:
            for token in arm:
                if token not in LICENSES:
                    found.append(f"dependency {name} states the licence {token!r}, which is not a "
                                 f"licence this reader knows")
        if arms and all(any(token in denied for token in arm) for arm in arms):
            found.append(f"dependency {name} offers only denied licences ({licence}), which this "
                         f"distribution boundary cannot honour")
        if not any(all(token in permitted.get(row.get("class", ""), []) for token in arm)
                   for arm in arms):
            found.append(f"dependency {name} is used under {licence!r} and reaches the "
                         f"{row.get('class')} class, which does not permit it")
    for name in sorted(set(declared) - set(recorded)):
        found.append(f"dependency {name} is declared by a crate in this workspace and no row "
                     f"records the licence it is used under")
    return found


def generated_problems(rows: list[dict], tree: Tree) -> list[str]:
    """Every generated artifact names the program that wrote it, the command that re-derives it and
    the case that holds it — and each of those is a file or a name this tree carries."""
    found: list[str] = []
    if not rows:
        found.append("no generated artifact is declared, so nothing this repository writes is "
                     "accounted for")
    for row in rows:
        paths = row.get("paths", [])
        if not paths:
            found.append("a generated artifact row names no path")
        for path in paths:
            if not tree.exists(path):
                found.append(f"generated artifact {path} is declared and this tree does not carry "
                             f"it")
        generator = row.get("generator")
        if not generator or not tree.at(generator).is_file():
            found.append(f"the generated artifact {paths[0] if paths else '?'} names the generator "
                         f"{generator!r}, which is not a file this tree carries")
        check = row.get("check")
        if not check or check not in tree.cases():
            found.append(f"the generated artifact {paths[0] if paths else '?'} is held by the case "
                         f"{check!r}, which no file in this tree declares")
        rerun = str(row.get("rerun", "")).strip()
        if not rerun:
            found.append(f"the generated artifact {paths[0] if paths else '?'} states no command "
                         f"that re-derives it, so it cannot be checked")
            continue
        for token in rerun.split():
            if PLACEHOLDER.match(token) or not PATHLIKE.search(token):
                continue
            if not tree.exists(token):
                found.append(f"the generated artifact {paths[0] if paths else '?'} is re-derived "
                             f"by a command naming {token!r}, which this tree does not carry")
    return found


def applicability_problems(policy: dict, program: set[int], obligations: set[str]) -> list[str]:
    """Security, privacy, accessibility, performance and cross-platform applicability, each marked
    once — required against an obligation, not applicable with a rationale, or tracked elsewhere
    against an owner."""
    found: list[str] = []
    seen: dict[str, int] = {}
    for entry in policy.get("applicability", []):
        concern = entry.get("concern")
        seen[concern] = seen.get(concern, 0) + 1
        mark = entry.get("mark")
        if concern not in CONCERNS:
            found.append(f"{concern!r} is not a concern this issue asks to be reviewed")
            continue
        if mark not in MARKS:
            found.append(f"{concern} is marked {mark!r}, which is not one of {', '.join(MARKS)}")
            continue
        if mark == "required" and entry.get("obligation") not in obligations:
            found.append(f"{concern} is marked required against the obligation "
                         f"{entry.get('obligation')!r}, which no row in this record carries")
        if mark == "not_applicable" and not str(entry.get("rationale", "")).strip():
            found.append(f"{concern} is marked not applicable with no rationale, which is the "
                         f"assumption the issue asks to be given rather than made")
        if mark == "separately_tracked":
            owners = entry.get("owners", [])
            if not owners:
                found.append(f"{concern} is marked separately tracked and names no issue, so "
                             f"nobody tracks it")
            for owner in owners:
                if owner not in program:
                    found.append(f"{concern} is tracked by #{owner}, which this program does not "
                                 f"carry")
    for concern in CONCERNS:
        if concern not in seen:
            found.append(f"{concern} is a concern the issue asks to be reviewed and this record "
                         f"marks it nowhere, so its applicability is assumed")
        elif seen[concern] > 1:
            found.append(f"{concern} is marked {seen[concern]} times")
    return found


def assumption_problems(policy: dict, universe: dict[str, list[str]]) -> list[str]:
    """The licence, authentication and telemetry assumption about every product in #171's universe,
    stated once each: a blank is the unstated assumption the issue's verification refuses, and an
    `UNKNOWN` names the gap that would settle it rather than passing for an answer."""
    found: list[str] = []
    seen: dict[str, int] = {}
    for entry in policy.get("runtime_assumptions", []):
        product = entry.get("product")
        seen[product] = seen.get(product, 0) + 1
        if product not in universe:
            found.append(f"a licensing assumption is recorded for {product!r}, which #171's "
                         f"universe does not carry")
            continue
        gaps = entry.get("gaps", [])
        for field in ASSUMPTIONS:
            value = str(entry.get(field, "")).strip()
            if not value:
                found.append(f"the {field} assumption about {product} is unstated, which is the "
                             f"assumption this audit refuses to leave implicit")
            elif value == UNKNOWN:
                if field not in gaps:
                    found.append(f"the {field} assumption about {product} is unknown and names no "
                                 f"gap, so nobody can settle it")
            elif not universe[product]:
                found.append(f"the {field} assumption about {product} is stated as {value} and "
                             f"#171's registry holds no source for it, so nothing stands behind "
                             f"the claim")
        for gap in gaps:
            if gap not in ASSUMPTIONS:
                found.append(f"the assumption for {product} names the gap {gap!r}, which is not "
                             f"an assumption this audit states")
    for product in sorted(set(universe) - set(seen)):
        found.append(f"{product} is in #171's universe and no licensing, authentication or "
                     f"telemetry assumption is recorded for it, so integrating it would depend on "
                     f"an unstated assumption")
    for product, times in seen.items():
        if times > 1:
            found.append(f"the assumptions about {product} are recorded {times} times")
    return found


def problems(policy: dict, tree: Tree, program: set[int],
             universe: dict[str, list[str]]) -> list[str]:
    """Every refusal the committed record must survive, each naming its subject."""
    found: list[str] = []
    if policy.get("schema_version") != SCHEMA_VERSION:
        found.append(f"the record declares schema {policy.get('schema_version')!r}, and this reader "
                     f"replays {SCHEMA_VERSION}")
    try:
        datetime.date.fromisoformat(policy.get("observed_at", ""))
    except ValueError:
        found.append(f"the record's observed date {policy.get('observed_at')!r} is not a date")
    if policy.get("issue") not in program:
        found.append(f"the record is held by issue {policy.get('issue')!r}, which this program does "
                     f"not carry")
    found.extend(class_problems(policy, tree, program))
    found.extend(obligation_problems(policy, tree, program))
    found.extend(stability_problems(policy, tree))
    found.extend(supply_problems(policy, tree))
    found.extend(applicability_problems(
        policy, program, {entry.get("id") for entry in policy.get("obligations", [])}))
    found.extend(assumption_problems(policy, universe))
    return found


def program_issues() -> set[int]:
    document = read(PROGRAM)
    return {entry.get("number") for entry in document.get("entries", [])}


def universe_sources() -> dict[str, list[str]]:
    """What #171 holds about each product in its universe: the sources its own evidence names. The
    runtime audit reads this rather than restating a product fact, so a product the registry drops
    is an assumption the audit stops being able to state."""
    registry = research.registry()
    found: dict[str, list[str]] = {}
    for product in registry.universe:
        sources = [evidence.get("source", "") for claim in
                   (registry.products.get(product, {}) or {}).get("claims", [])
                   for evidence in claim.get("evidence", []) if evidence.get("source")]
        found[product] = sorted(set(sources))
    return found


def record() -> dict:
    return read(POLICY)


def class_report(policy: dict) -> list[dict]:
    """The five boundaries as the record holds them, with whether a licence is selected."""
    return [{"class": entry.get("id"), "title": entry.get("title"),
             "license": entry.get("license"), "status": entry.get("status"),
             "owners": entry.get("owners", []), "covers": entry.get("covers"),
             "boundary": entry.get("boundary"), "why": entry.get("why", "")}
            for entry in policy.get("classes", [])]


def audit(policy: dict, universe: dict[str, list[str]]) -> dict:
    """The assumptions this project makes about the products it plans to integrate, and the
    obligations this tree has not built — the two readings the issue's own verification clause and
    applicability criterion are about."""
    assumptions = []
    for entry in policy.get("runtime_assumptions", []):
        unknown = [field for field in ASSUMPTIONS
                   if str(entry.get(field, "")).strip() == UNKNOWN]
        assumptions.append({"product": entry.get("product"), "unknown": unknown,
                            "stated": {field: entry.get(field) for field in ASSUMPTIONS
                                       if field not in unknown},
                            "gaps": entry.get("gaps", []),
                            "sources": universe.get(entry.get("product"), [])})
    unbuilt = [{"obligation": entry.get("id"), "area": entry.get("area"),
                "status": entry.get("status"), "owners": entry.get("owners", []),
                "why": entry.get("why", "")}
               for entry in policy.get("obligations", []) if entry.get("status") != "built"]
    return {"assumptions": assumptions,
            "unknown_fields": sum(len(one["unknown"]) for one in assumptions),
            "unbuilt": unbuilt}


def generated_report(policy: dict) -> list[dict]:
    return [{"paths": row.get("paths", []), "generator": row.get("generator"),
             "rerun": row.get("rerun"), "check": row.get("check")}
            for row in policy.get("supply_chain", {}).get("generated", [])]


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--check", action="store_true",
                        help="refuse the committed record, or pass it")
    parser.add_argument("--classes", action="store_true",
                        help="print the five licensing boundaries")
    parser.add_argument("--audit", action="store_true",
                        help="print the runtime assumptions and the obligations not built")
    parser.add_argument("--generated", action="store_true",
                        help="print the generated-artifact provenance table")
    arguments = parser.parse_args(argv)
    try:
        policy = record()
        if arguments.classes:
            print(json.dumps(class_report(policy), indent=2))
            return 0
        if arguments.audit:
            print(json.dumps(audit(policy, universe_sources()), indent=2))
            return 0
        if arguments.generated:
            print(json.dumps(generated_report(policy), indent=2))
            return 0
        found = problems(policy, Tree(), program_issues(), universe_sources())
    except Refused as error:
        print(f"refused: {error}", file=sys.stderr)
        return 1
    if found:
        for refusal in found:
            print(f"refused: {refusal}", file=sys.stderr)
        return 1
    unbuilt = [entry for entry in policy.get("obligations", []) if entry.get("status") != "built"]
    selected = [entry.get("id") for entry in policy.get("classes", [])
                if entry.get("status") == "selected"]
    print(f"policy {policy.get('observed_at')}: {len(policy.get('classes', []))} artifact classes "
          f"({len(selected)} licensed), {len(policy.get('obligations', []))} obligations "
          f"({len(unbuilt)} not built), "
          f"{len(policy.get('stability', {}).get('contracts', []))} contracts at "
          f"{len(LEVELS)} levels, "
          f"{len(policy.get('supply_chain', {}).get('dependencies', []))} dependencies, "
          f"{len(policy.get('supply_chain', {}).get('generated', []))} generated artifacts, "
          f"{len(policy.get('applicability', []))} applicability marks, 0 refusals")
    if not selected:
        print("no artifact class selects a licence: every boundary is stated and every selection "
              "is #174's own remaining work")
    return 0


if __name__ == "__main__":
    sys.exit(main())
