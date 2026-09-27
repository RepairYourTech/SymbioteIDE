"""The census of the documents under `docs/` that publish a contract or a figure.

Seven passes measured the documents under `docs/contracts/` and held the figures a crate in this
repository owns at the crate that owns them. The record they left said the census was complete —
and nothing read that sentence, so a new document, a new bound inside one, or a deleted holding
case would have left it true and the surface unheld. This module is the owner of the census itself:
one table family naming every document it classifies, whether a case holds it or not, and the
figure count the reading derives from the Markdown ones, so an entry nothing classifies, a document
whose figures moved, and a classification naming a case that no longer reads the document each fail
by name in `test_contracts.py` rather than in prose.

**One key space, because the directory is not what makes a document held.** Every path here is
repo-relative, so a contract and a security document are classified by the same three tables and
read by the same `figures` and `readers`. A bound published in `docs/security/linux-sandbox.md` is
as unheld as one published in a contract, and the first version of this census treated them
differently — bare filenames for one directory, repo-relative paths for the rest, four reader
functions where two would do — so the question "which case holds this?" had four answer sites and
two spelling conventions to look up. `docs/contracts/` remains a *discovery* rule, not a naming one:
every entry directly under it must be classified whatever it is, and outside it a Markdown document
must be classified only when it states a figure. Those are different rules about which documents
the census owes a decision about, and `documents` states both.

**`HELD` owns "which case holds this document", and nowhere else states it.** Each entry is every
case that holds it, not one: `docs/contracts/work-hierarchy.md` is held by the domain's bound check
and by the host's measured-answer case, and a census that named only the first made the second
row in `mutation_probe.py` a claim of its own rather than a proof of this. A document one case
holds is a one-element tuple. `mutation_probe.py` names the case it proves and reads the crate from
here, so a row crediting a case this table does not record for that document is not a state the
tables can be written into.

What this cannot decide, and states rather than pretends: which *new* sentence in a document is a
figure a crate owns. `BOUND` reads the shapes below and `FIGURES` records what it derived when the
census was written, so a statement in one of those shapes appears or disappears with the record
moving in the same change — a reviewer sees the number move. A figure written in another shape, a
line whose value changes inside a shape the reading sees, and whether a held figure is the right one
to hold, all stay with the case that holds the document and with the reader of the change.
"""
from __future__ import annotations

import pathlib
import re

import prose

ROOT = pathlib.Path(__file__).resolve().parents[2]
DOCS = ROOT / "docs"
CONTRACTS = DOCS / "contracts"
CRATES = ROOT / "crates"

# What this reading calls a figure statement: a bound a sentence states over a surface, or a number
# carrying the unit or the count noun it counts. Each shape is driven by `test_contracts.py`, which
# proves the reading on the shapes it names — a spelling outside them is a figure this census does
# not see, which is the boundary the module docstring states rather than a gap in the count.
BOUND = re.compile(
    r"\b(?:at\s+(?:most|least)|no\s+(?:more|fewer)\s+than|bounded\s+(?:to|at|by)|limited\s+to"
    r"|limits?\s+(?:its\s+input\s+)?to|defaults?\s+to|up\s+to|exactly|maximum\s+of|minimum\s+of"
    r"|fewer\s+than|more\s+than)\s+\d[\d,_]*"
    r"|\d[\d,_]*(?:\.\d+)?\s*(?:KiB|MiB|GiB|KB|MB|bytes|bits?|hex|characters|chars|files|entries"
    r"|items|records|rows|columns|tables|messages|turns|tools|nodes|steps|seconds|minutes|hours"
    r"|days|ms|%)\b",
    re.IGNORECASE,
)

# The cases that read each document as a contract, each with the crate that holds it: a document
# named here whose case is renamed or deleted fails `test_contracts.py` by name, and a document no
# case in this table names is one no row in `mutation_probe.py` can prove.
#
# Naming the right *file* is not naming the right *case*, and only one check here can tell them
# apart: `readers` answers whether any source in the crate reads the document, so a case that reads
# a different document out of the same file satisfies it while the named case compares nothing.
# `cli.md` was held that way for a long time — by
# `the_contract_document_names_the_commands_and_kinds_the_table_defines`, which reads `host.md` —
# and the case that really holds its policy bound, in another file of the same crate, read it too,
# so the file-level check was satisfied by an accident of layout. The entry below names the cases
# that compare, and `mutation_probe.py` is what keeps them apart from here: a row that changes the
# document and requires one of *these* cases to go red is a hold measured rather than assumed.
HELD: dict[str, tuple[tuple[str, str], ...]] = {
    "docs/contracts/agent-environment.md": (
        ("symbiote-config", "the_contract_states_the_manifest_bound_this_crate_enforces"),),
    "docs/contracts/architecture.md": (
        ("symbiote-architecture", "the_document_writes_a_row_for_every_module_the_crate_declares"),),
    "docs/contracts/cli.md": (
        ("symbiote-host", "the_contract_states_the_policy_byte_bound_this_parser_enforces"),),
    "docs/contracts/client-sdk.md": (
        ("symbiote-client-sdk",
         "the_contract_names_every_outcome_this_crate_classifies_and_none_it_does_not"),),
    "docs/contracts/configuration.md": (
        ("symbiote-config", "the_contract_states_the_specificity_and_storage_of_every_scope"),),
    "docs/contracts/constitution.md": (
        ("symbiote-constitution", "every_channel_the_contract_doc_claims_is_one"),),
    "docs/contracts/context-credential-resolution.md": (
        ("symbiote-context", "the_contract_states_the_bounds_this_crate_enforces"),),
    "docs/contracts/external-agent-loop.md": (
        ("symbiote-external-agent", "the_contract_states_the_bounds_this_driver_enforces"),),
    "docs/contracts/host-inventory.md": (
        ("symbiote-host-inventory", "the_contract_states_the_sample_lifetime_this_crate_stamps"),),
    "docs/contracts/host.md": (
        ("symbiote-host", "the_codes_the_documents_state_are_the_ones_this_binary_uses"),),
    "docs/contracts/native-agent-loop.md": (
        ("symbiote-native-agent", "the_contract_states_the_bounds_this_loop_enforces"),),
    "docs/contracts/projection.md": (
        ("symbiote-projection", "the_contract_states_the_bounds_this_module_enforces"),),
    "docs/contracts/protocol.md": (
        ("symbiote-protocol", "the_documents_state_the_version_and_bounds_this_crate_enforces"),),
    "docs/contracts/providers.md": (
        ("symbiote-runtime-sdk", "the_contract_states_the_envelope_bounds_this_module_enforces"),),
    "docs/contracts/repository.md": (
        ("symbiote-repo", "the_contract_states_the_bounds_this_crate_enforces"),),
    "docs/contracts/runtime-discovery.md": (
        ("symbiote-runtime-discovery", "the_contract_states_the_release_this_crate_pins"),),
    "docs/contracts/runtime-events.md": (
        ("symbiote-runtime-sdk", "the_contract_states_the_bytes_and_limits_this_module_enforces"),),
    "docs/contracts/runtime-sdk.md": (
        ("symbiote-runtime-sdk", "the_document_names_every_rule_the_suite_runs"),),
    "docs/contracts/runtime-transport.md": (
        ("symbiote-runtime-transport", "the_contract_states_the_limits_this_transport_defaults_to"),),
    "docs/contracts/scheduling-leases.md": (
        ("symbiote-domain", "the_contracts_state_the_work_and_lease_bounds_this_crate_enforces"),),
    "docs/contracts/storage.md": (
        ("symbiote-store", "the_contract_states_the_writer_timeout_this_crate_sets"),),
    "docs/contracts/work-hierarchy.md": (
        # Two cases, two crates, and two different claims: the domain's case compares the bounds
        # this document states and the host's measures the widest answer they allow. A census that
        # named only the first made the host's row a claim of its own rather than a proof of this.
        ("symbiote-domain", "the_contracts_state_the_work_and_lease_bounds_this_crate_enforces"),
        ("symbiote-host", "the_widest_answer_the_published_bounds_allow_is_measured_on_the_wire")),
    "docs/contracts/workforce-bindings.md": (
        ("symbiote-workforce", "the_contract_states_the_binding_bound_this_crate_enforces"),),
    "docs/contracts/worktrees.md": (
        ("symbiote-worktrees", "the_contract_states_the_identities_this_module_derives"),),
    "docs/security/linux-sandbox.md": (
        ("symbiote-sandbox",
         "the_document_states_the_traversal_and_diagnostic_bounds_this_crate_enforces"),),
}

# The documents no case reads as a contract, each with why. The first group states a surface this
# repository compares behaviourally — the crate's own cases, the store's migration tests — rather
# than a figure a case compares to the document; the second is generated output a `--check`
# invocation compares byte for byte, so its content is held without a case reading its prose; the
# third is proof a recorded run captured, where every figure is a measurement from that run rather
# than a bound the tree still enforces. An entry here that a case starts reading is a classification
# this census moves, and one that disappears fails too.
NOT_HELD: dict[str, str] = {
    "docs/agent-takeover.md": "a dated record of the tree at the client's takeover instruction, which names the protocol version and transport bounds that were current at that revision rather than bounds this tree publishes; the live figures are the contract's, held by the crates that enforce them",
    "docs/architecture/product-constitution.md": "a reconciliation record whose numerals are canonical requirement identifiers and the date the requirements were retrieved, not bounds over any surface",
    "docs/contracts/constitution-report.json": "generated by the constitution report example and compared with `--check`",
    "docs/contracts/dispatch-preparation.md": "the dispatch composition's refusal vocabulary and issue routing, which the domain and host cases drive rather than this document's prose",
    "docs/contracts/domain-ontology.v1.schema.json": "generated by the ontology schema example and compared with `--check`",
    "docs/contracts/domain.md": "the domain model's shape, which the domain crate's cases and the ontology check read rather than this document's prose",
    "docs/contracts/project-team.md": "the team configuration's schema narration: what schema v4 added, which is history the store's migration cases drive",
    "docs/contracts/protocol.v1.schema.json": "generated by the protocol schema example from the wire types' own derives, and compared byte for byte with the case beside the crate",
    "docs/contracts/provider-registry.md": "the three provider records' schema narration and their operation list, which the store's cases and the schema check drive",
    "docs/contracts/resource-consent.md": "the consent table's schema narration and its upgrade refusal, which the store's migration cases drive",
    "docs/contracts/role-resolution.md": "the route decision's shape and its replay rules, which the domain and store cases drive",
    "docs/contracts/schemas": "generated by `symbiote schema --write` and compared byte for byte",
    "docs/contracts/worker-completion-wiring.md": "the completion wiring's refusal vocabulary, which the host and domain cases drive",
    "docs/proofs/host-lifecycle.md": "captured evidence from one recorded run: the POSIX modes and page size the daemon's own filesystem exhibits, which the cases read from the filesystem rather than from this prose",
    "docs/proofs/linux-sandbox-aliases.md": "captured probe evidence from one recorded run of the alias probe, where each figure is a measurement the probe took rather than a bound the launcher enforces",
    "docs/proofs/linux-shell.md": "a spike's recorded measurements — sampled RSS, run durations, artifact timestamps, a package size — each a figure from a run that happened, held by the spike's own rule suite rather than by a crate in this workspace; the sandbox bounds it restates are the contract's",
}

# What `BOUND` derived from every Markdown document in the census when it was written — read off the
# documents rather than restated from a pass's prose, which is where the first draft's numbers came
# from and why they were wrong. A document that gains or loses a statement in one of those shapes
# moves its figure in the same change, and the case refuses until it does, so a bound cannot enter a
# document without the census moving with it.
FIGURES: dict[str, int] = {
    "docs/agent-takeover.md": 1,
    "docs/architecture/product-constitution.md": 1,
    "docs/contracts/agent-environment.md": 2,
    "docs/contracts/architecture.md": 0,
    "docs/contracts/cli.md": 2,
    "docs/contracts/client-sdk.md": 1,
    "docs/contracts/configuration.md": 0,
    "docs/contracts/constitution.md": 0,
    "docs/contracts/context-credential-resolution.md": 2,
    "docs/contracts/dispatch-preparation.md": 0,
    "docs/contracts/domain.md": 0,
    "docs/contracts/external-agent-loop.md": 2,
    "docs/contracts/host-inventory.md": 0,
    "docs/contracts/host.md": 3,
    "docs/contracts/native-agent-loop.md": 2,
    "docs/contracts/project-team.md": 0,
    "docs/contracts/projection.md": 1,
    "docs/contracts/protocol.md": 2,
    "docs/contracts/provider-registry.md": 0,
    "docs/contracts/providers.md": 1,
    "docs/contracts/repository.md": 4,
    "docs/contracts/resource-consent.md": 0,
    "docs/contracts/role-resolution.md": 1,
    "docs/contracts/runtime-discovery.md": 2,
    "docs/contracts/runtime-events.md": 1,
    "docs/contracts/runtime-sdk.md": 0,
    "docs/contracts/runtime-transport.md": 1,
    "docs/contracts/scheduling-leases.md": 0,
    "docs/contracts/storage.md": 0,
    "docs/contracts/work-hierarchy.md": 8,
    "docs/contracts/worker-completion-wiring.md": 0,
    "docs/contracts/workforce-bindings.md": 1,
    "docs/contracts/worktrees.md": 3,
    "docs/proofs/host-lifecycle.md": 1,
    "docs/proofs/linux-sandbox-aliases.md": 1,
    "docs/proofs/linux-shell.md": 12,
    "docs/security/linux-sandbox.md": 2,
}


def states_a_figure(document: pathlib.Path) -> bool:
    """Whether any line of `document` states a figure in a shape `BOUND` reads."""
    return any(BOUND.search(line) for line in prose.statements(document))


def documents(docs: pathlib.Path = DOCS) -> list[str]:
    """Every repo-relative path the census owes a decision about, under two rules: every entry
    directly under the contracts directory, whatever it is, because a contract is a surface whether
    or not it states a figure; and every Markdown document outside it that does state one, because
    a bound published in a security or proof document is as unheld as one published in a contract
    and there is no directory to say so."""
    here = sorted(str(one.relative_to(docs.parent)) for one in CONTRACTS.iterdir())
    outside = (str(one.relative_to(docs.parent)) for one in sorted(docs.rglob("*.md"))
               if CONTRACTS not in one.parents and states_a_figure(one))
    return sorted([*here, *outside])


def markdown(docs: pathlib.Path = DOCS) -> list[str]:
    """The subset of `documents` that `FIGURES` covers: the Markdown ones, since a figure is a
    sentence and a generated schema or a directory of them states none."""
    return [name for name in documents(docs) if name.endswith(".md")]


def figures(name: str, root: pathlib.Path = CONTRACTS) -> int:
    """How many lines of the document `name` names state a figure: one per line, whatever it states
    on it. `FIGURES` has been spelled two ways in this tree's life — a bare filename under
    `docs/contracts/` and a repo-relative path — and one default root is right for one spelling and
    `docs/contracts/docs/contracts/cli.md` for the other, so the name decides: a name carrying a
    directory is read from the repository root and a bare one from the contracts directory, and the
    default is only where a name is read from when it does not say. `root` reads a document written
    elsewhere, which is what a fixture in a temporary directory is.
    """
    base = root if root != CONTRACTS else (ROOT if "/" in name else CONTRACTS)
    return sum(1 for line in prose.statements(base / name) if BOUND.search(line))


def readers(name: str, crates: pathlib.Path = CRATES) -> list[str]:
    """Every crate source that reads the repo-relative document `name`: a file naming the path it
    is read by. Matched on the path below `docs/`, which is the form a source spells it in —
    `contracts/cli.md` under an `include_str!` and `security/linux-sandbox.md` inside a wider
    relative path — so one reading covers contracts and everything published outside them."""
    reading = name[len("docs/"):] if name.startswith("docs/") else name
    return [str(source.relative_to(crates.parent)) for source in sorted(crates.rglob("*.rs"))
            if reading in source.read_text(errors="replace")]


def holder(document: str, case: str) -> str | None:
    """The crate whose case named `case` holds `document`, or None when the census does not record
    that case as a holder of it. The lookup a `mutation_probe.py` row makes, so the crate a claim
    is proved against is read from here rather than restated in the row."""
    return next((crate for crate, held in HELD.get(document, ()) if held == case), None)
