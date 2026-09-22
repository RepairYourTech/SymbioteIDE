"""The census of `docs/contracts/`: every entry there, and who holds what it states.

Seven passes measured the documents under `docs/contracts/` and held the figures a crate in this
repository owns at the crate that owns them. The record they left said the census was complete — and
nothing read that sentence, so a sixteenth document, a new bound inside one, or a deleted holding
case would have left it true and the surface unheld. This module is the owner of the census itself:
one table naming every entry under the directory, whether a case reads it, and the figure count the
reading derives from it, so an entry nothing classifies, a document whose figures moved, and a
classification naming a case that no longer reads the document each fail by name in
`test_contracts.py` rather than in prose.

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

ROOT = pathlib.Path(__file__).resolve().parents[2]
CONTRACTS = ROOT / "docs" / "contracts"
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

# The entries a case reads as a contract, each with the crate that holds it and one case that reads
# the document to compare what it states against the declaration that enforces it. A document named
# here whose case is renamed or deleted fails `test_contracts.py` by name.
HELD = {
    "agent-environment.md": ("symbiote-config", "the_contract_states_the_manifest_bound_this_crate_enforces"),
    "architecture.md": ("symbiote-architecture", "the_document_writes_a_row_for_every_module_the_crate_declares"),
    "cli.md": ("symbiote-host", "the_contract_document_names_the_commands_and_kinds_the_table_defines"),
    "client-sdk.md": ("symbiote-client-sdk", "the_contract_names_every_outcome_this_crate_classifies_and_none_it_does_not"),
    "constitution.md": ("symbiote-constitution", "every_channel_the_contract_doc_claims_is_one"),
    "context-credential-resolution.md": ("symbiote-context", "the_contract_states_the_bounds_this_crate_enforces"),
    "external-agent-loop.md": ("symbiote-external-agent", "the_contract_states_the_bounds_this_driver_enforces"),
    "host-inventory.md": ("symbiote-host-inventory", "the_contract_states_the_sample_lifetime_this_crate_stamps"),
    "host.md": ("symbiote-host", "the_codes_the_documents_state_are_the_ones_this_binary_uses"),
    "native-agent-loop.md": ("symbiote-native-agent", "the_contract_states_the_bounds_this_loop_enforces"),
    "projection.md": ("symbiote-projection", "the_contract_states_the_bounds_this_module_enforces"),
    "protocol.md": ("symbiote-protocol", "the_documents_state_the_version_and_bounds_this_crate_enforces"),
    "providers.md": ("symbiote-runtime-sdk", "the_contract_states_the_envelope_bounds_this_module_enforces"),
    "repository.md": ("symbiote-repo", "the_contract_states_the_bounds_this_crate_enforces"),
    "runtime-discovery.md": ("symbiote-runtime-discovery", "the_contract_states_the_release_this_crate_pins"),
    "runtime-events.md": ("symbiote-runtime-sdk", "the_contract_states_the_bytes_and_limits_this_module_enforces"),
    "runtime-transport.md": ("symbiote-runtime-transport", "the_contract_states_the_limits_this_transport_defaults_to"),
    "scheduling-leases.md": ("symbiote-domain", "the_contracts_state_the_work_and_lease_bounds_this_crate_enforces"),
    "storage.md": ("symbiote-store", "the_contract_states_the_writer_timeout_this_crate_sets"),
    "work-hierarchy.md": ("symbiote-domain", "the_contracts_state_the_work_and_lease_bounds_this_crate_enforces"),
    "workforce-bindings.md": ("symbiote-workforce", "the_contract_states_the_binding_bound_this_crate_enforces"),
    "worktrees.md": ("symbiote-worktrees", "the_contract_states_the_identities_this_module_derives"),
}

# The entries no case reads as a contract, each with why. The first group states a surface this
# repository compares behaviourally — the crate's own cases, the store's migration tests — rather
# than a figure a case compares to the document; the second is generated output a `--check`
# invocation compares byte for byte, so its content is held without a case reading its prose. An
# entry here that a case starts reading is a classification this census moves, and one that
# disappears fails too.
NOT_HELD = {
    "configuration.md": "the layered configuration model: the config crate's own cases drive precedence, so its key order and layering are that crate's bounds and no figure here is read",
    "dispatch-preparation.md": "the dispatch composition's refusal vocabulary and issue routing, which the domain and host cases drive rather than this document's prose",
    "domain.md": "the domain model's shape, which the domain crate's cases and the ontology check read rather than this document's prose",
    "project-team.md": "the team configuration's schema narration: what schema v4 added, which is history the store's migration cases drive",
    "provider-registry.md": "the three provider records' schema narration and their operation list, which the store's cases and the schema check drive",
    "resource-consent.md": "the consent table's schema narration and its upgrade refusal, which the store's migration cases drive",
    "role-resolution.md": "the route decision's shape and its replay rules, which the domain and store cases drive",
    "runtime-sdk.md": "the declared-runtime input form, which the runtime SDK's own cases drive",
    "worker-completion-wiring.md": "the completion wiring's refusal vocabulary, which the host and domain cases drive",
    "constitution-report.json": "generated by the constitution report example and compared with `--check`",
    "domain-ontology.v1.schema.json": "generated by the ontology schema example and compared with `--check`",
    "schemas": "generated by `symbiote schema --write` and compared byte for byte",
}

# What `BOUND` derived from every Markdown document when the census was written — read off the
# documents rather than restated from a pass's prose, which is where the first draft's numbers came
# from and why they were wrong. A document that gains or loses a statement in one of those shapes
# moves its figure in the same change, and the case refuses until it does, so a bound cannot enter a
# contract without the census moving with it.
FIGURES = {
    "agent-environment.md": 2,
    "architecture.md": 0,
    "cli.md": 1,
    "client-sdk.md": 1,
    "configuration.md": 0,
    "constitution.md": 0,
    "context-credential-resolution.md": 2,
    "dispatch-preparation.md": 0,
    "domain.md": 0,
    "external-agent-loop.md": 2,
    "host-inventory.md": 0,
    "host.md": 3,
    "native-agent-loop.md": 2,
    "projection.md": 1,
    "project-team.md": 0,
    "protocol.md": 2,
    "provider-registry.md": 0,
    "providers.md": 1,
    "repository.md": 4,
    "resource-consent.md": 0,
    "role-resolution.md": 1,
    "runtime-discovery.md": 0,
    "runtime-events.md": 1,
    "runtime-sdk.md": 0,
    "runtime-transport.md": 1,
    "scheduling-leases.md": 0,
    "storage.md": 0,
    "work-hierarchy.md": 5,
    "worker-completion-wiring.md": 0,
    "workforce-bindings.md": 1,
    "worktrees.md": 3,
}


def entries(root: pathlib.Path = CONTRACTS) -> list[str]:
    """Every entry directly under the contracts directory: files and directories alike."""
    return sorted(one.name for one in root.iterdir())


def figures(name: str, root: pathlib.Path = CONTRACTS) -> int:
    """How many lines of `name` state a figure: one per line, whatever it states on it."""
    return sum(1 for line in (root / name).read_text().splitlines() if BOUND.search(line))


def readers(name: str, crates: pathlib.Path = CRATES) -> list[str]:
    """Every crate source that reads `name`: a file naming its path under `contracts/`."""
    return [str(source.relative_to(crates.parent)) for source in sorted(crates.rglob("*.rs"))
            if f"contracts/{name}" in source.read_text(errors="replace")]
