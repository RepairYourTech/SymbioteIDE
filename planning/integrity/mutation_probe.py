#!/usr/bin/env python3
"""Change each published claim in this tree, one at a time, and watch the case that holds it
fail — then put the claim back.

`contracts.py` proves that a document a census calls held really is read by the case the census
names, and that the figure count it recorded still matches the document. Both are structural, and
both can hold a document whose numbers are fiction: a case can read a document, compare a figure
that never moved, and pass. PR #761 published a transport ceiling of 447,935 bytes exactly that
way — a case held `work-hierarchy.md`, the document's number had never been measured by anything,
and a substring check on the prose was the only thing standing behind it. So this driver takes the
other half: it mutates the claim and requires the suite to go red. A row that stays green is a
claim no witness holds, and the driver fails naming it.

**A row names a document and one of the cases the census says holds it; it does not name the
crate.** `CLAIMS` holds one row per claim — the document, the exact text of the claim, the text to
change it to, and which of that document's holding cases must notice. The crate is read from
`contracts.HELD` and the cargo selector that reaches the case from `SELECTORS` below, so "which case
holds this document" is stated once in this tree rather than thirty times here. A row crediting a
case the census does not record for that document is a state these two tables cannot be written
into, which is the mistake the first version of them made: the census named a case that read
`host.md` while the row named the one that compares `cli.md`, and every check in the directory
stayed green over both. Most claims are figures, and the replacement is deliberately one no code
can enforce, one more or one less than the true bound, so a red run is the case comparing the
number rather than the build rejecting the document. A hold does not have to be over a number: a
contract may publish a module row, a scope's specificity, a rule's identifier, a terminal outcome's
name, a report channel it claims, or an exit code, and the case holding it compares that name
rather than a bound. Those rows change a name to one the crate does not carry, which is the same
proof with a different text — so the table is about what a document *claims*, and a row whose
replacement is a figure would be a row that proved nothing.

`SELECTORS` is the one thing here the census cannot state, because it is about cargo rather than
about documents: a case is reached three ways in this tree and a row that guessed which would be a
row that ran nothing — `--test <name>` for a crate's own `tests/`, `--lib` for a case in the
library's `src`, and `--bin <name>` for one in a binary's, which is where the CLI's exit codes are
held. It is keyed by crate *and* case because two crates declare a case of the same name.

`SELF_PROOF` is what tells a row's red from a suite that was already broken: it changes prose the
witness does not read, in a document a row does hold, and requires the same suite to stay green.
Without it a row could pass by failing for a reason its mutation did not cause, which is the shape
of a proof that proves nothing. A mutation that does not compile is reported as invalid rather than
as proof, because a document edited into a form the build rejects has not been shown to change what
the case compares.

Every document a census calls held has a row, and `test_mutation_probe.py` holds that: the census
proves a hold is *possible* and this proves one exists, so a new entry in either table is a claim
this tree has not measured until a row names it.

**Why this mutates the live tree when `revert_rules.py` mutates a copy.** Three crates carry a
`build.rs` that runs `symbiote-source-stamp`, whose gate refuses to build a package outside this
workspace: cargo resolves the package's dependencies in the tree it was invoked in, the manifest
walk in the copy does not reach them, and the build stops rather than record a source list that
would let a proof pass against a binary built from other sources. That is a good guard and it makes
a copy unbuildable for exactly the crates worth probing — `symbiote-sandbox` and `symbiote-host` —
which is why `revert_rules.py`'s own rows touch only `symbiote-architecture`, the one crate in its
set with no build script. So the guards move to the driver instead:

* it refuses to start unless the tracked tree is clean, so there is nothing of yours to lose;
* every mutation is restored in a `finally`, including on a signal, from bytes read beforehand;
* a backup of each document is kept beside the driver for the run and removed only at the end;
* every watched document's bytes are digested before and after, and a byte that moved fails.

What a driver cannot hold is a run killed between the write and the `finally`. The backup exists so
that run is recoverable rather than silent, and a document left carrying `.figure-probe.bak` beside
it is one the next start refuses.
"""
from __future__ import annotations

import hashlib
import os
import pathlib
import re
import shutil
import subprocess
import sys
from typing import NamedTuple

import contracts

ROOT = pathlib.Path(__file__).resolve().parents[2]
BACKUP_SUFFIX = ".figure-probe.bak"


class Claim(NamedTuple):
    """One published claim: the label a failure names it by, the document publishing it, which of
    that document's holding cases must go red, the exact text of the claim, and the text that
    replaces it. The crate the case lives in is `contracts.HELD`'s to answer, not this table's."""
    label: str
    document: str
    case: str
    claim: str
    replacement: str


# One row per published claim over the 25 documents a census calls held. The claim is the exact
# text the document writes once, and the replacement a text it does not write at all — the
# self-proof's shape for the wrong edit is the row that proves nothing.
CLAIMS: tuple[Claim, ...] = (
    Claim("linux-sandbox.md states 100,000 traversal entries", "docs/security/linux-sandbox.md",
          "the_document_states_the_traversal_and_diagnostic_bounds_this_crate_enforces",
          "100,000 entries", "100,001 entries"),
    Claim("linux-sandbox.md states 64 traversal levels", "docs/security/linux-sandbox.md",
          "the_document_states_the_traversal_and_diagnostic_bounds_this_crate_enforces",
          "64 levels", "65 levels"),
    Claim("linux-sandbox.md retains eight diagnostic lines", "docs/security/linux-sandbox.md",
          "the_document_states_the_traversal_and_diagnostic_bounds_this_crate_enforces",
          "eight diagnostic lines", "nine diagnostic lines"),
    Claim("linux-sandbox.md retains 1,024 bytes per diagnostic line", "docs/security/linux-sandbox.md",
          "the_document_states_the_traversal_and_diagnostic_bounds_this_crate_enforces",
          "1,024 UTF-8 bytes", "1,025 UTF-8 bytes"),
    Claim("work-hierarchy.md states the measured 280,402-byte answer",
          "docs/contracts/work-hierarchy.md",
          "the_widest_answer_the_published_bounds_allow_is_measured_on_the_wire",
          "280,402 bytes", "280,403 bytes"),
    Claim("work-hierarchy.md states the 26.7% share of the 1 MiB bound",
          "docs/contracts/work-hierarchy.md",
          "the_widest_answer_the_published_bounds_allow_is_measured_on_the_wire",
          "26.7% of the protocol", "26.8% of the protocol"),
    Claim("work-hierarchy.md states the 4,096-item graph bound",
          "docs/contracts/work-hierarchy.md",
          "the_contracts_state_the_work_and_lease_bounds_this_crate_enforces",
          "graph validation to 4,096 items", "graph validation to 4,097 items"),
    # --- The documents under `docs/contracts/`, one row per held document ------------------------
    #
    # The census proves each of these is read; these rows are what prove the reading compares. A
    # held document nothing here changes is a hold measured by nothing, so `test_mutation_probe.py`
    # refuses the table in that state.
    Claim("agent-environment.md states the 1 MiB manifest bound",
          "docs/contracts/agent-environment.md",
          "the_contract_states_the_manifest_bound_this_crate_enforces",
          "limits input to 1 MiB", "limits input to 2 MiB"),
    Claim("architecture.md writes a row for a module the crate does not declare",
          "docs/contracts/architecture.md",
          "the_document_writes_a_row_for_every_module_the_crate_declares",
          "| `spike.rs` |", "| `spike2.rs` |"),
    Claim("cli.md states the 64 KiB policy bound", "docs/contracts/cli.md",
          "the_contract_states_the_policy_byte_bound_this_parser_enforces",
          "bounded at 64 KiB", "bounded at 65 KiB"),
    Claim("client-sdk.md names a terminal outcome the crate does not classify",
          "docs/contracts/client-sdk.md",
          "the_contract_names_every_outcome_this_crate_classifies_and_none_it_does_not",
          "and `Unparseable` (", "and `Unparseabl` ("),
    Claim("configuration.md states a scope's specificity out of the resolver's order",
          "docs/contracts/configuration.md",
          "the_contract_states_the_specificity_and_storage_of_every_scope",
          "| `user` — user preferences | 2 |", "| `user` — user preferences | 9 |"),
    Claim("constitution.md claims a report channel the committed report does not carry",
          "docs/contracts/constitution.md", "every_channel_the_contract_doc_claims_is_one",
          "so `every_coverage_row_names_a_check_or_an_owner` and",
          "so `every_coverage_row_names_a_check_or_an_ownerZ` and"),
    Claim("context-credential-resolution.md states the 8 KiB per-item bound",
          "docs/contracts/context-credential-resolution.md",
          "the_contract_states_the_bounds_this_crate_enforces",
          "bounded per item (8 KiB", "bounded per item (9 KiB"),
    Claim("external-agent-loop.md states the 128-byte identifier bound",
          "docs/contracts/external-agent-loop.md",
          "the_contract_states_the_bounds_this_driver_enforces",
          "charset and 128-byte", "charset and 129-byte"),
    Claim("host-inventory.md states the five-second sample lifetime",
          "docs/contracts/host-inventory.md",
          "the_contract_states_the_sample_lifetime_this_crate_stamps",
          "Samples expire after five seconds", "Samples expire after six seconds"),
    Claim("host.md states an exit code the binary does not use", "docs/contracts/host.md",
          "the_codes_the_documents_state_are_the_ones_this_binary_uses",
          "1 usage/connection failure", "4 usage/connection failure"),
    Claim("native-agent-loop.md states the 256 KiB total tool-result bound",
          "docs/contracts/native-agent-loop.md",
          "the_contract_states_the_bounds_this_loop_enforces",
          "total tool-result text is bounded (256 KiB)",
          "total tool-result text is bounded (257 KiB)"),
    Claim("projection.md states the 4 MiB generation total", "docs/contracts/projection.md",
          "the_contract_states_the_bounds_this_module_enforces",
          "1 MiB per file and 4 MiB total", "1 MiB per file and 5 MiB total"),
    Claim("protocol.md states the 65,536-byte request bound", "docs/contracts/protocol.md",
          "the_documents_state_the_version_and_bounds_this_crate_enforces",
          "Requests are limited to 65,536 bytes", "Requests are limited to 65,537 bytes"),
    Claim("providers.md states the 128-message envelope bound", "docs/contracts/providers.md",
          "the_contract_states_the_envelope_bounds_this_module_enforces",
          "at most 128 messages", "at most 129 messages"),
    Claim("repository.md states the 16 KiB content-head bound", "docs/contracts/repository.md",
          "the_contract_states_the_bounds_this_crate_enforces",
          "at most 16 KiB each", "at most 17 KiB each"),
    Claim("runtime-discovery.md states a release other than the one pinned",
          "docs/contracts/runtime-discovery.md",
          "the_contract_states_the_release_this_crate_pins",
          "The tested release is `codex-cli 0.118.0`", "The tested release is `codex-cli 0.118.1`"),
    Claim("runtime-events.md states the 16,384-byte event-text bound",
          "docs/contracts/runtime-events.md",
          "the_contract_states_the_bytes_and_limits_this_module_enforces",
          "to 16,384 UTF-8", "to 16,385 UTF-8"),
    Claim("runtime-sdk.md omits a rule the suite runs", "docs/contracts/runtime-sdk.md",
          "the_document_names_every_rule_the_suite_runs",
          "descriptor_identity_is_valid", "zzz_descriptor_identity_valid"),
    Claim("runtime-transport.md states a frame bound other than the default",
          "docs/contracts/runtime-transport.md",
          "the_contract_states_the_limits_this_transport_defaults_to",
          "defaults to 64 KiB per frame", "defaults to 65 KiB per frame"),
    Claim("scheduling-leases.md states a lease window the crate does not declare",
          "docs/contracts/scheduling-leases.md",
          "the_contracts_state_the_work_and_lease_bounds_this_crate_enforces",
          "the declared 1s–1h window", "the declared 1s–2h window"),
    Claim("storage.md states a busy timeout the writer does not set",
          "docs/contracts/storage.md", "the_contract_states_the_writer_timeout_this_crate_sets",
          "foreign-key constraints, a five-second busy timeout",
          "foreign-key constraints, a six-second busy timeout"),
    Claim("workforce-bindings.md states a configuration bound the crate does not read",
          "docs/contracts/workforce-bindings.md",
          "the_contract_states_the_binding_bound_this_crate_enforces",
          "Configuration is bounded to 32 KiB", "Configuration is bounded to 33 KiB"),
    Claim("worktrees.md states a digest width the module does not truncate to",
          "docs/contracts/worktrees.md",
          "the_contract_states_the_identities_this_module_derives",
          "carries a 96-bit digest truncation", "carries a 97-bit digest truncation"),
)

# How cargo reaches each holding case these rows name, keyed by crate and case because two crates
# declare a case of the same name. A case in a crate's `tests/` is `--test <file>`, one in the
# library's own `src` is `--lib`, and one in a binary's is `--bin <name>` — the CLI's exit codes
# are held by a case in `symbiote-host/src/bin/symbiote/tests.rs`, which is neither of the others.
SELECTORS: dict[tuple[str, str], str] = {
    ("symbiote-architecture", "the_document_writes_a_row_for_every_module_the_crate_declares"):
        "--test document",
    ("symbiote-client-sdk", "the_contract_names_every_outcome_this_crate_classifies_and_none_it_does_not"):
        "--test contracts",
    ("symbiote-config", "the_contract_states_the_manifest_bound_this_crate_enforces"): "--lib",
    ("symbiote-config", "the_contract_states_the_specificity_and_storage_of_every_scope"):
        "--test scopes",
    ("symbiote-constitution", "every_channel_the_contract_doc_claims_is_one"): "--test conformance",
    ("symbiote-context", "the_contract_states_the_bounds_this_crate_enforces"): "--test contracts",
    ("symbiote-domain", "the_contracts_state_the_work_and_lease_bounds_this_crate_enforces"): "--lib",
    ("symbiote-external-agent", "the_contract_states_the_bounds_this_driver_enforces"): "--lib",
    ("symbiote-host", "the_codes_the_documents_state_are_the_ones_this_binary_uses"):
        "--bin symbiote",
    ("symbiote-host", "the_contract_states_the_policy_byte_bound_this_parser_enforces"): "--lib",
    ("symbiote-host", "the_widest_answer_the_published_bounds_allow_is_measured_on_the_wire"):
        "--test daemon",
    ("symbiote-host-inventory", "the_contract_states_the_sample_lifetime_this_crate_stamps"):
        "--test contracts",
    ("symbiote-native-agent", "the_contract_states_the_bounds_this_loop_enforces"): "--lib",
    ("symbiote-projection", "the_contract_states_the_bounds_this_module_enforces"): "--lib",
    ("symbiote-protocol", "the_documents_state_the_version_and_bounds_this_crate_enforces"):
        "--test contracts",
    ("symbiote-repo", "the_contract_states_the_bounds_this_crate_enforces"): "--test contracts",
    ("symbiote-runtime-discovery", "the_contract_states_the_release_this_crate_pins"):
        "--test contracts",
    ("symbiote-runtime-sdk", "the_contract_states_the_bytes_and_limits_this_module_enforces"):
        "--test events",
    ("symbiote-runtime-sdk", "the_contract_states_the_envelope_bounds_this_module_enforces"):
        "--test provider",
    ("symbiote-runtime-sdk", "the_document_names_every_rule_the_suite_runs"): "--test conformance",
    ("symbiote-runtime-transport", "the_contract_states_the_limits_this_transport_defaults_to"):
        "--test process",
    ("symbiote-sandbox", "the_document_states_the_traversal_and_diagnostic_bounds_this_crate_enforces"):
        "--test document",
    ("symbiote-store", "the_contract_states_the_writer_timeout_this_crate_sets"): "--lib",
    ("symbiote-workforce", "the_contract_states_the_binding_bound_this_crate_enforces"):
        "--test contracts",
    ("symbiote-worktrees", "the_contract_states_the_identities_this_module_derives"): "--lib",
}

# Prose the witness does not read, in a document a row above does hold.
SELF_PROOF: Claim = Claim(
    "linux-sandbox.md prose the witness does not read", "docs/security/linux-sandbox.md",
    "the_document_states_the_traversal_and_diagnostic_bounds_this_crate_enforces",
    "Trees with unsupported entries", "Trees holding odd entries")


def digest(paths: list[pathlib.Path]) -> dict[str, str]:
    """The bytes of each path, so a document that was not put back is caught even though the
    driver restores in a `finally` rather than trusting itself to."""
    return {str(one): hashlib.sha256(one.read_bytes()).hexdigest()
            for one in paths if one.is_file()}


def dirty() -> list[str]:
    """Tracked files the tree has modified or staged: the driver refuses to start over someone's
    work, because a probe that restores bytes has to restore *these* bytes and would not."""
    run = subprocess.run(["git", "status", "--porcelain", "--untracked-files=no"],
                         cwd=ROOT, capture_output=True, text=True)
    return [line[3:] for line in run.stdout.splitlines() if line.strip()]


def leftover() -> list[str]:
    """Backups a previous run left behind, which means a previous run was killed between its write
    and its restore."""
    return sorted(str(one.relative_to(ROOT)) for one in ROOT.rglob("*" + BACKUP_SUFFIX)
                  if "target" not in one.parts and "node_modules" not in one.parts)


def run_case(crate: str, selector: str, case: str) -> tuple[int, str]:
    """The suite run, and everything it printed. Not `-q`: a quiet run prints a passing case as a
    bare dot, so a green run cannot be told from a case that was never selected at all."""
    run = subprocess.run(
        ["cargo", "test", "--locked", "-p", crate, *selector.split(), case],
        cwd=ROOT, capture_output=True, text=True,
        env={**os.environ, "PYTHONDONTWRITEBYTECODE": "1"},
    )
    return run.returncode, run.stdout + run.stderr


def verdicts(output: str, case: str) -> list[bool]:
    """What the harness said about each line it printed for `case`: whether that case failed.

    The filter is the case's own name, which is a *substring* of what the harness prints, so
    `--exact` cannot be used: a case inside a module is printed as `tests::name`, and an exact
    filter for `name` matches nothing at all — which is how six rows of this table first read as
    witnesses that did not run. Matching the name at the end of whatever module path precedes it
    is what the harness prints instead, and reading the verdict off that same line is what makes
    the run a proof: a row is proved by *its* case failing, not by the process exiting non-zero,
    which a neighbouring test in the same binary could do on its own.
    """
    return [line.rstrip().endswith("FAILED")
            for line in re.findall(rf"^test (?:\w+::)*{re.escape(case)} \.\.\. \w+.*$", output,
                                   re.M)]


def compile_error(output: str) -> bool:
    return "error[E" in output or "could not compile" in output


def first_error(output: str) -> str:
    for line in output.splitlines():
        if "error[E" in line or "panicked at" in line:
            return line.strip()
    return next((line.strip() for line in reversed(output.splitlines()) if line.strip()),
                "no output")


def swap(document: str, before: str, after: str, run) -> str | None:
    """Run `run` with `document` publishing `after` in place of `before`, restoring the bytes
    whatever happens, and return why the case failed to notice — or None when it did."""
    path = ROOT / document
    backup = path.with_name(path.name + BACKUP_SUFFIX)
    text = path.read_text()
    if text.count(before) != 1:
        return f"{document} writes {before!r} {text.count(before)} times, where this row needs once"
    path.write_bytes(path.read_bytes())
    shutil.copy2(path, backup)
    try:
        path.write_text(text.replace(before, after))
        return run()
    finally:
        path.write_bytes(backup.read_bytes())
        backup.unlink()


def unreachable(claim: Claim) -> str | None:
    """Why this claim names no case the driver can run — a case the census does not record as
    holding the document, or one holding it with no selector here — or None when it names both."""
    crate = contracts.holder(claim.document, claim.case)
    if crate is None:
        return (f"the census does not record {claim.case} as a case that holds {claim.document}, "
                f"so this row would be proving a claim of its own rather than the census's")
    if (crate, claim.case) not in SELECTORS:
        return (f"no selector states how cargo reaches {claim.case} in {crate}, so this row names "
                f"no run to make")
    return None


def holds(claim: Claim) -> str | None:
    """Why the case does not fail when the claim moves, or None when it does."""
    crate = contracts.holder(claim.document, claim.case)

    def run() -> str | None:
        # The exit status is deliberately not read here: a row is proved by its own case going red,
        # which a neighbouring test in the same binary cannot do for it. The self-proof below is
        # where the status still means something, because it asks that a run stay green.
        _code, output = run_case(crate, SELECTORS[(crate, claim.case)], claim.case)
        said = verdicts(output, claim.case)
        if not said:
            return f"{claim.case} is not a case the suite runs: {first_error(output)}"
        if compile_error(output):
            return (f"the mutation did not compile, so it proved nothing: {first_error(output)}")
        if not all(said):
            return (f"{claim.case} passes with {claim.document} publishing {claim.replacement!r} "
                    f"rather than {claim.claim!r}")
        return None

    return swap(claim.document, claim.claim, claim.replacement, run)


def self_proof() -> str | None:
    """Why the driver cannot tell a row's red from an already-broken run, or None when it can."""
    crate = contracts.holder(SELF_PROOF.document, SELF_PROOF.case)

    def run() -> str | None:
        code, output = run_case(crate, SELECTORS[(crate, SELF_PROOF.case)], SELF_PROOF.case)
        said = verdicts(output, SELF_PROOF.case)
        if not said:
            return f"{SELF_PROOF.case} is not a case the suite runs: {first_error(output)}"
        if compile_error(output):
            return f"the self-proof's own edit did not compile: {first_error(output)}"
        if any(said) or code != 0:
            return (f"{SELF_PROOF.case} fails when {SELF_PROOF.document}'s prose changes, so a "
                    f"row's red could be a suite that was already broken: {first_error(output)}")
        return None

    return swap(SELF_PROOF.document, SELF_PROOF.claim, SELF_PROOF.replacement, run)


def main() -> int:
    stale = leftover()
    if stale:
        print(f"A previous probe left {', '.join(stale)} behind, so it was killed between its "
              f"write and its restore. Restore from the backup, or delete it if the document is "
              f"already right, before probing again.")
        return 1
    changed = dirty()
    if changed:
        print(f"This probe restores the bytes it changes, so it will not start over a tree with "
              f"work in it. Commit, stash or revert first: {', '.join(changed)}")
        return 1

    # A row naming no case the driver can run is refused before anything is written: the run below
    # reads `SELECTORS` for every row it reaches, so a table with a hole in it is a crash halfway
    # through a mutation rather than a refusal.
    unreachable_rows = [(claim.label, why) for claim in (*CLAIMS, SELF_PROOF)
                        if (why := unreachable(claim))]
    if unreachable_rows:
        print("rows naming no case this driver can run:")
        for label, why in unreachable_rows:
            print(f"  {label}\n      {why}")
        return 1

    watched = sorted({ROOT / claim.document for claim in (*CLAIMS, SELF_PROOF)})
    before = digest(watched)
    unheld, invalid, bit = [], [], 0
    print(f"Reverting {len(CLAIMS)} published claims, one at a time, and putting each back.")
    for claim in CLAIMS:
        why = holds(claim)
        if why is None:
            bit += 1
            print(f"  BIT         {claim.label}")
        elif "did not compile" in why:
            invalid.append((claim.label, why))
            print(f"  INVALID     {claim.label}: {why}")
        else:
            unheld.append((claim.label, why))
            print(f"  NO WITNESS  {claim.label}\n              {why}")
    print("\nProving the driver tells a row's red from a suite that was already broken.")
    why = self_proof()
    if why is None:
        print("  BIT         prose the witness does not read changes, and the suite holds")
    else:
        invalid.append(("the self-proof", why))
        print(f"  FAILED      {why}")

    after = digest(watched)
    moved = [one for one, was in before.items() if after.get(one) != was]
    if moved:
        print(f"\nA document was not put back: {', '.join(moved)}")
        return 1
    print(f"\nclaims held: {bit}/{len(CLAIMS)}; every document restored byte for byte")
    for title, rows in (("claims no case holds", unheld), ("rows that proved nothing", invalid)):
        if rows:
            print(f"{title} ({len(rows)}):")
            for label, why in rows:
                print(f"  {label}\n      {why}")
    return 1 if (unheld or invalid) else 0


if __name__ == "__main__":
    sys.exit(main())
