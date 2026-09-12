# Offline roadmap integrity

This Python standard-library maintenance tool validates an immutable GitHub REST issues snapshot and generates a structural registry and topological index. It has no network or issue-mutation path. Python is used for existing planning maintenance; this does not select the product runtime.

```sh
python3 planning/capture_roadmap.py --help
python3 planning/integrity/validate.py /path/to/issues.json --output /path/to/generated
python3 planning/integrity/test_validate.py
python3 planning/integrity/test_validate.py --snapshot /path/to/issues.json
```

Input is a JSON array with REST `number`, `body`, `labels`, `state`, `state_reason`, and `updated_at` fields. Pull requests are ignored. Canonical issues require `planning:canonical`, one `symbiote-plan-key`, and one `symbiote-plan-revision` marker. Reference entries require `planning:reference`, `symbiote-reference-key`, and a direct `Canonical owner: #N`; the historical program entry uses `symbiote-program-entry` and its explicit canonical roadmap link. Unclassified keyed issues remain outside the executable graph.

Atomic tasks require one parent epic, one wave, and one explicit `## Dependencies` section. Each dependency line must be `- #N` or a same-repository GitHub issue URL. Empty/unknown syntax fails; no prerequisites must be explicitly written as `- None` (the existing constitution sentence is also supported). Related/review/integration sections never create edges. Historical details in references cannot create metadata or edges; canonical bodies are parsed in full.

Checks include duplicate canonical keys, missing revisions, issue existence, canonical dependency targets, archived/not-planned prerequisites, cycles, later-wave prerequisites, body/label wave consistency, reciprocal epic membership, master epic membership, and any declared master/child counts. Closed completed issues remain represented; ordering does not establish readiness. Generated records preserve original body SHA-256, update timestamp, labels, and state. The snapshot SHA-256 identifies the exact source capture.

The committed reduced audit fixture reproduces the structural inputs from the after-audit capture; its `source_body_sha256` fields refer to original full bodies, not the deliberately reduced fixture bodies. Default tests compare its full graph, mapping, waves and counts to the generated original-snapshot registry. `--snapshot` additionally checks the full 466-issue audit capture and injects a duplicate-key regression. This optional check is pinned to that audit, not an arbitrary future inventory.

## Pull-request closing keywords

`closing_keywords.py` reports the issues GitHub's parser will close from a pull request's description and from the messages of its commits, which GitHub scans equally and with no reading of negation. Two rules follow.

A description fails when a closing keyword is negated by its own sentence, or when the keyword is buried in prose instead of stated as its own clause: `Closes #54` is a closure both GitHub and a reader can see, while "the defect that closed #54 is fixed" is a remark that closes the issue anyway. Of the last sixty descriptions merged here, exactly one trips this rule — PR #530, whose first line disclaimed the closure it performed one second after its merge.

A commit message (`--commits`) fails on any closing keyword, because nothing in review reads a commit message and the description is where an intended closure belongs. PR #551 landed this guard and was itself caught by that channel: its final sentence, "the defect that closed #54 is reproduced rather than described", closed #54 when the pull request merged. Its tests pin that message, the two real descriptions, and the documented syntax.

Keywords count only against the default branch, so a non-default base is reported as closing nothing rather than failing.

This implements a bounded portion of #470. It does **not** certify conversation-to-issue coverage, acceptance completion, or exact-body mutation safety. `acceptance_items` is only a checkbox inventory. Safe importer regeneration, three-way merge, stale-revision refusal, concurrent-edit/ambiguous-create reconciliation, and post-mutation readback remain separate work. The original bootstrap importer must not be used to overwrite this registry's newer issue authority.
