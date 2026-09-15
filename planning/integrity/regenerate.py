#!/usr/bin/env python3
"""Safe roadmap regeneration: plan intended issue bodies, then mutate and read back.

`validate.py` proves a snapshot is a well-formed canonical registry. This module
owns what happens next: regenerating issue bodies from the current registry and
accepted amendments without losing human edits, without overwriting newer
authority with a stale payload, and without ever creating a duplicate. The rules
are refused by name rather than left to an operator's care.

- **Nothing is planned that would not validate.** The intended snapshot is built
  and validated with `validate.validate_snapshot` before a plan exists; a
  duplicate key, a dangling dependency or a cycle therefore fails with no
  mutation call made, and `apply` refuses any plan carrying a refusal. A planned
  create is validated under the next free issue number, since that is the graph
  the operator would end up with.
- **A stale payload never overwrites newer authority.** An amendment must declare
  the revision marker and the body hash it was authored against. Where the live
  body carries a different revision marker, the plan refuses and names both: an
  old bootstrap payload may only regenerate from the current registry and
  accepted amendments.
- **A concurrent human edit is merged or aborted, never overwritten.** Where the
  live body still carries the declared revision but its bytes moved, the
  amendment is merged three-way against the human edit (base = the capture the
  registry was generated from, ours = the amendment, theirs = the live body).
  Disjoint changes merge and are recorded; overlapping changes refuse and name
  the conflicting region. A regenerated bootstrap payload that would drop a
  section the live body has refuses and names the lost sections, which is what
  stops an old importer restoring pre-native-agent, pre-learning or
  pre-Change-Stream wording in place of accepted text.
- **A create is reconciled, never duplicated.** An ambiguous create — a lost
  response, or an issue a human made first — is resolved by reading the key back
  and adopting what is there; a second ambiguity fails visibly instead of trying
  again blind.
- **A key resolves to the work it addresses, or it is refused.** `key_holders`
  answers "who carries this key" once, by role, and both the plan and `verify`
  ask it: an amendment naming a key canonical work carries amends that work, even
  where a superseded historical entry shares the key — which is how this roadmap
  keys most of its references — and an amendment naming a key *only* a reference
  entry carries is refused, naming the canonical issue that entry resolves to.
  A reference entry's bytes are never regenerated: its whole point is that its
  history is preserved.
- **What is written is validated, not only what was computed.** `apply` builds the
  registry its mutations would leave behind from the capture it is given, validates
  it with the same `validate_snapshot`, and refuses before the first mutation; the
  CLI always supplies that capture, so a plan file an operator edited is re-checked
  rather than trusted. `apply` then re-reads each target immediately before
  mutating and refuses if it moved since planning, compares the body hash it reads
  back with the intended one, and `verify` proves that every issue the plan did not
  name is byte-identical and in the same state.

The module holds no network path of its own. `apply` takes a runner: `CliRunner`
shells out to `gh api` for an operator — through `apply --dry-run`, which prints
what each body would become — and the suite passes a fake that records calls, so
what the tests prove is the rule rather than a transport.
"""

import argparse
import difflib
import hashlib
import json
import re
import subprocess
import sys
from difflib import SequenceMatcher
from pathlib import Path

from validate import (
    IntegrityError,
    authoritative_text,
    canonical_owner,
    marker,
    validate_snapshot,
)


def body_hash(body):
    return hashlib.sha256((body or "").encode()).hexdigest()


def headings(body):
    return re.findall(r"(?m)^##[ \t]+(.+?)[ \t]*$", body or "")


def label_names(raw):
    return [x.get("name") if isinstance(x, dict) else x for x in raw.get("labels") or []]


def canonical_key_of(raw):
    """The plan key that identifies this issue as canonical work, or None.

    A reference entry's key is deliberately *not* this: the two roles share a
    namespace in the issue text, and answering "who carries this key" from one
    string is what used to make a historical entry look like the work it points
    at. `key_holders` is the one place that question is answered.
    """
    text = authoritative_text(raw.get("body") or "", label_names(raw))
    return marker(text, "plan-key")


def reference_of(raw):
    """The reference entry this issue is — its key and the canonical issue it
    resolves to — or None where it is not one."""
    text = authoritative_text(raw.get("body") or "", label_names(raw))
    program_entry = marker(text, "program-entry")
    key = marker(text, "reference-key") or program_entry
    if not key:
        return None
    return {
        "number": raw["number"],
        "key": key,
        "canonical_issue": canonical_owner(text, bool(program_entry)),
    }


def key_holders(snapshot):
    """Who carries every key a snapshot holds, by role.

    ``canonical`` lists the issues a key addresses as work; ``references`` lists
    the historical entries that resolve to it. A key in both is ordinary here:
    the reconciled roadmap keys a superseded entry exactly as the task that
    supersedes it, and the task is what an amendment addresses.
    """
    index = {}
    for raw in snapshot:
        if "pull_request" in raw:
            continue
        key = canonical_key_of(raw)
        if key:
            index.setdefault(key, {"canonical": [], "references": []})["canonical"].append(raw["number"])
            continue
        reference = reference_of(raw)
        if reference:
            index.setdefault(reference["key"], {"canonical": [], "references": []})["references"].append(reference)
    return index


def holders_of(index, key):
    return index.get(key, {"canonical": [], "references": []})


def revision_of(body):
    try:
        return marker(body or "", "plan-revision")
    except IntegrityError:
        return None


def edits(base, side):
    """The change a side makes to the base, keyed by the base range it replaces."""
    return {
        (start, end): side[after_start:after_end]
        for tag, start, end, after_start, after_end in SequenceMatcher(
            a=base, b=side, autojunk=False
        ).get_opcodes()
        if tag != "equal"
    }


def three_way_merge(base_text, ours_text, theirs_text):
    """Merge two edits of one body. Returns (merged_text_or_None, conflicts).

    A conflict is a base region both sides changed, or two different insertions at
    the same point; the conflicting lines from each side are reported so a refusal
    can name them rather than saying only that a merge failed.
    """
    base, ours, theirs = base_text.splitlines(), ours_text.splitlines(), theirs_text.splitlines()
    our_edits, their_edits = edits(base, ours), edits(base, theirs)
    conflicts = []
    for (start, end), our_lines in sorted(our_edits.items()):
        for (other_start, other_end), their_lines in sorted(their_edits.items()):
            overlaps = max(start, other_start) < min(end, other_end) or (
                start == end == other_start == other_end
            )
            if overlaps and our_lines != their_lines:
                conflicts.append({"ours": our_lines, "theirs": their_lines})
    if conflicts:
        return None, conflicts
    merged, cursor = [], 0
    for (start, end) in sorted(set(our_edits) | set(their_edits)):
        if start < cursor:
            continue
        merged.extend(base[cursor:start])
        merged.extend(our_edits.get((start, end)) or their_edits.get((start, end), []))
        cursor = end
    merged.extend(base[cursor:])
    return "\n".join(merged), []


def refusal(reason, **detail):
    entry = {"reason": reason}
    entry.update(detail)
    return entry


def plan(base_snapshot, live_snapshot, amendments):
    """Return {"mutations": [...], "refusals": [...], "intended_checked": bool}.

    ``base_snapshot`` is the REST capture the registry was generated from,
    ``live_snapshot`` what the issues look like now, and each amendment carries
    ``key``, ``body``, ``base_revision``, ``base_body_sha256`` and ``kind``
    (``bootstrap`` or ``amendment``). Mutations are all-or-nothing: a plan with
    any refusal has no mutations to apply.
    """
    base = {raw["number"]: raw for raw in base_snapshot if "pull_request" not in raw}
    live = {raw["number"]: raw for raw in live_snapshot if "pull_request" not in raw}
    holders = key_holders(live.values())

    intended, mutations, refusals = dict(live), [], []
    provisional = max(live, default=0)
    for amendment in amendments:
        missing = [f for f in ("key", "body", "base_revision", "base_body_sha256") if not amendment.get(f)]
        if missing:
            refusals.append(
                refusal(
                    "amendment declares no base revision to check itself against",
                    key=amendment.get("key"),
                    missing=missing,
                )
            )
            continue
        key = amendment["key"]
        held = holders_of(holders, key)
        candidates = held["canonical"]
        if not candidates and held["references"]:
            alias = held["references"][0]
            refusals.append(
                refusal(
                    "a key only a reference entry carries is never regenerated: "
                    "amend the canonical issue it resolves to",
                    key=key,
                    number=alias["number"],
                    canonical_issue=alias["canonical_issue"],
                )
            )
            continue
        if len(candidates) > 1:
            refusals.append(refusal("duplicate live key", key=key, numbers=sorted(candidates)))
            continue
        target = candidates[0] if candidates else None

        if target is None:
            provisional += 1
            intended[provisional] = {
                "number": provisional,
                "title": amendment.get("title", key),
                "state": "open",
                "state_reason": None,
                "updated_at": amendment.get("updated_at", ""),
                "labels": [{"name": "planning:canonical"}],
                "body": amendment["body"],
            }
            mutations.append(
                {
                    "action": "create",
                    "key": key,
                    "body": amendment["body"],
                    "body_sha256": body_hash(amendment["body"]),
                    "title": amendment.get("title", key),
                    "validated_as": provisional,
                    "base_live_sha256": None,
                }
            )
            continue

        current = live[target].get("body") or ""
        declared, expected = amendment["base_revision"], amendment["base_body_sha256"]
        live_revision = revision_of(current)
        if live_revision is not None and live_revision != declared:
            refusals.append(
                refusal(
                    "stale revision refuses to overwrite newer authority",
                    key=key,
                    number=target,
                    amendment_revision=declared,
                    live_revision=live_revision,
                )
            )
            continue

        if body_hash(current) == expected:
            merged, merge_kind = amendment["body"], "clean"
        else:
            if target not in base:
                refusals.append(
                    refusal("no base body to merge against", key=key, number=target)
                )
                continue
            merged, conflicts = three_way_merge(base[target].get("body") or "", amendment["body"], current)
            if merged is None:
                refusals.append(
                    refusal(
                        "concurrent human edit conflicts; abort rather than last-write-wins",
                        key=key,
                        number=target,
                        conflicts=conflicts,
                    )
                )
                continue
            merge_kind = "merged"

        if amendment.get("kind") == "bootstrap":
            lost = sorted(set(headings(current)) - set(headings(merged)))
            if lost:
                refusals.append(
                    refusal(
                        "a regenerated bootstrap may not drop sections the live body has",
                        key=key,
                        number=target,
                        lost_sections=lost,
                    )
                )
                continue

        intended[target] = dict(intended[target], body=merged)
        mutations.append(
            {
                "action": "update",
                "key": key,
                "number": target,
                "body": merged,
                "body_sha256": body_hash(merged),
                "merge": merge_kind,
                "base_live_sha256": body_hash(current),
            }
        )

    if refusals:
        return {"mutations": [], "refusals": refusals, "intended_checked": False}
    try:
        validate_snapshot([intended[number] for number in sorted(intended)])
    except IntegrityError as exc:
        return {
            "mutations": [],
            "refusals": [refusal("intended registry is invalid", detail=str(exc))],
            "intended_checked": True,
        }
    return {"mutations": mutations, "refusals": [], "intended_checked": True}


class AmbiguousCreate(Exception):
    """The transport cannot say whether the create happened."""


def created_reading(runner, mutation):
    """A create's number, or None where the transport cannot say whether it happened."""
    try:
        return runner.create(mutation["title"], mutation["body"])
    except AmbiguousCreate:
        return None


def intended_snapshot(plan_dict, live):
    """The registry a plan's mutations would leave behind, built from a capture.

    This is what makes the write path check the same thing the plan path did: an
    operator's plan file is instructions, and they are validated against the
    capture they will be applied to before any of them is written.
    """
    intended = {raw["number"]: dict(raw) for raw in live if "pull_request" not in raw}
    for mutation in plan_dict["mutations"]:
        number = mutation.get("number") or mutation.get("validated_as")
        if number is None:
            continue
        entry = intended.get(number)
        if entry is None:
            entry = {
                "number": number,
                "title": mutation.get("title", mutation["key"]),
                "state": "open",
                "state_reason": None,
                "updated_at": "",
                "labels": [{"name": "planning:canonical"}],
            }
        intended[number] = dict(entry, body=mutation["body"])
    return [intended[number] for number in sorted(intended)]


def apply(plan_dict, runner, live, dry_run=False):
    """Mutate through the runner, reading back and reporting what landed.

    The runner provides ``read(number)``, ``find(key)`` — the *canonical* issue
    carrying that key, or None — ``create(title, body)`` and ``update(number,
    body)``. A plan carrying refusals is never applied, and neither is one whose
    mutations would leave the registry invalid: what is written is validated
    against the capture it is written to, not only what was computed.
    """
    if plan_dict.get("refusals"):
        raise IntegrityError(
            f"plan carries refusals and cannot be applied: {plan_dict['refusals']}"
        )
    try:
        validate_snapshot(intended_snapshot(plan_dict, live))
    except IntegrityError as exc:
        raise IntegrityError(
            f"applying this plan would leave the registry invalid, so nothing is written: {exc}"
        )
    applied = []
    for mutation in plan_dict["mutations"]:
        if dry_run:
            applied.append({"action": mutation["action"], "key": mutation["key"], "dry_run": True})
            continue
        if mutation["action"] == "create":
            number = runner.find(mutation["key"])
            if number is None:
                number = created_reading(runner, mutation)
                if number is None:
                    number = runner.find(mutation["key"])
                    if number is None:
                        number = created_reading(runner, mutation)
            if number is None:
                raise IntegrityError(
                    f"ambiguous create for {mutation['key']}: the key is still not readable, "
                    "so nothing is retried blind"
                )
        else:
            number = mutation["number"]
            if body_hash(runner.read(number).get("body") or "") != mutation["base_live_sha256"]:
                raise IntegrityError(
                    f"#{number} moved since the plan was made: concurrent edits are replanned, never overwritten"
                )
            runner.update(number, mutation["body"])
        if body_hash(runner.read(number).get("body") or "") != mutation["body_sha256"]:
            raise IntegrityError(
                f"read-back of #{number} does not match the intended body: the mutation did not land as planned"
            )
        applied.append({"action": mutation["action"], "key": mutation["key"], "number": number})
    return {"applied": applied, "dry_run": dry_run}


def verify(plan_dict, before, live):
    """Prove what the plan named landed exactly and that nothing else moved.

    A key is matched against the canonical work that carries it, never against a
    reference entry: the historical entry shares the key by design and is not the
    issue the plan named.
    """
    problems = []
    named = set()
    holders = key_holders(live)
    by_number = {raw["number"]: raw for raw in live}
    for mutation in plan_dict["mutations"]:
        matching = holders_of(holders, mutation["key"])["canonical"]
        if len(matching) != 1:
            problems.append(
                f"{mutation['key']} is carried by {len(matching)} canonical issues after mutation"
            )
            planned = mutation.get("number") or mutation.get("validated_as")
            if planned is not None:
                named.add(planned)
            continue
        raw = by_number[matching[0]]
        named.add(raw["number"])
        if body_hash(raw.get("body") or "") != mutation["body_sha256"]:
            problems.append(f"#{raw['number']} does not carry the intended body")
    for raw in before:
        if raw["number"] in named:
            continue
        after = next((candidate for candidate in live if candidate["number"] == raw["number"]), None)
        if after is None:
            problems.append(f"#{raw['number']} disappeared from the inventory")
        elif body_hash(after.get("body") or "") != body_hash(raw.get("body") or ""):
            problems.append(f"#{raw['number']} was modified without being planned")
        elif after.get("state") != raw.get("state"):
            problems.append(f"#{raw['number']} changed state without being planned")
    return problems


class CliRunner:
    """The operator's transport: `gh api`, and nothing else.

    The suite never constructs this, so no test of this module reaches the network.
    """

    def __init__(self, repository):
        self.repository = repository

    def _api(self, *arguments):
        output = subprocess.check_output(["gh", "api", *arguments], text=True)
        return json.loads(output) if output.strip() else {}

    def read(self, number):
        return self._api(f"repos/{self.repository}/issues/{number}")

    def find(self, key):
        """The canonical issue carrying `key`, or None: a reference entry shares the
        key by design and is not the issue a create is reconciled against."""
        search = self._api("search/issues", "-f", f"q=repo:{self.repository} {key} in:body")
        for item in search.get("items", []):
            raw = self._api(f"repos/{self.repository}/issues/{item['number']}")
            if canonical_key_of(raw) == key:
                return item["number"]
        return None

    def create(self, title, body):
        created = self._api(
            f"repos/{self.repository}/issues", "-f", f"title={title}", "-f", f"body={body}"
        )
        return created.get("number")

    def update(self, number, body):
        return self._api(
            f"repos/{self.repository}/issues/{number}", "-X", "PATCH", "-f", f"body={body}"
        )


def diff_of(key, before, after):
    """The body change one mutation would make, as a unified diff."""
    return "".join(
        difflib.unified_diff(
            (before or "").splitlines(keepends=True),
            (after or "").splitlines(keepends=True),
            fromfile=f"{key} (live)",
            tofile=f"{key} (planned)",
        )
    )


def plan_command(parser, arguments):
    planned = plan(
        json.loads(arguments.base.read_text()),
        json.loads(arguments.live.read_text()),
        json.loads(arguments.amendments.read_text()),
    )
    arguments.output.write_text(json.dumps(planned, indent=2, sort_keys=True) + "\n")
    for mutation in planned["mutations"]:
        print(json.dumps({k: v for k, v in mutation.items() if k != "body"}, sort_keys=True))
    for entry in planned["refusals"]:
        print(json.dumps(entry, sort_keys=True), file=sys.stderr)
    if planned["refusals"]:
        parser.exit(1, f"{len(planned['refusals'])} refusal(s); nothing is planned for application\n")
    print(f"planned {len(planned['mutations'])} mutation(s)", file=sys.stderr)
    return 0


def apply_command(arguments):
    """Apply a plan file against the capture it will be written to.

    The plan file is instructions an operator may edit; `--live` is the registry
    those instructions would leave behind, and it is validated before a mutation is
    attempted.
    """
    planned = json.loads(arguments.plan.read_text())
    live = json.loads(arguments.live.read_text())
    runner = CliRunner(arguments.repository)
    if arguments.dry_run:
        for mutation in planned["mutations"]:
            number = mutation.get("number")
            before = runner.read(number).get("body") if number else ""
            print(diff_of(mutation["key"], before, mutation["body"]), end="")
        report = apply(planned, runner, live, dry_run=True)
        print(json.dumps(report, sort_keys=True), file=sys.stderr)
        return 0
    report = apply(planned, runner, live)
    print(json.dumps(report, sort_keys=True))
    return 0


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    planning = commands.add_parser("plan", help="plan intended bodies from a base, a live capture and amendments")
    planning.add_argument("--base", required=True, type=Path, help="REST capture the registry came from")
    planning.add_argument("--live", required=True, type=Path, help="current REST capture")
    planning.add_argument("--amendments", required=True, type=Path)
    planning.add_argument("--output", required=True, type=Path)
    applying = commands.add_parser("apply", help="apply a plan through gh api, or --dry-run it")
    applying.add_argument("--plan", required=True, type=Path)
    applying.add_argument("--repository", required=True, help="owner/name the plan's numbers belong to")
    applying.add_argument("--live", required=True, type=Path,
                          help="the capture these instructions are applied to, validated before any mutation")
    applying.add_argument("--dry-run", action="store_true", help="print the diff and mutate nothing")
    arguments = parser.parse_args(argv)
    return plan_command(parser, arguments) if arguments.command == "plan" else apply_command(arguments)
if __name__ == "__main__":
    main()
