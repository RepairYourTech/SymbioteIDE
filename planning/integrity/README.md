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

A description fails when a closing keyword is negated by its own sentence, or when the keyword is buried in prose instead of stated as its own clause: `Closes #54` is a closure both GitHub and a reader can see, while "the defect that closed #54 is fixed" is a remark that closes the issue anyway. The rule is precise rather than trigger-happy: of the 85 descriptions merged up to `40a43b2d`, exactly one trips it — PR #530, whose first line disclaimed the closure it performed one second after its merge.

A commit message (`--commits`) and a pull-request title (`--title`) fail on any closing keyword, because GitHub reads both on merge and no reviewer reads either as a closure statement. The title matters because this repository squash-merges with `squash_title=COMMIT_OR_PR_TITLE` and `squash_message=COMMIT_MESSAGES`: a pull request whose branch carries more than one commit merges with its title at the head of the merged message. That is not hypothetical here: of the 85 pull requests merged up to `40a43b2d`, 49 carried more than one commit, so their titles head the merged messages, and 44 have a subject that is exactly that pull request's own title with its number appended. The title rule refuses none of the 85 merged titles, including the house style that cites issue numbers in parentheses (`(#229, #464)`). PR #551 landed this guard and was itself caught by the commit channel: its final sentence, "the defect that closed #54 is reproduced rather than described", closed #54 when the pull request merged. The tests pin that message, the two real descriptions and two real titles, and the documented syntax.

Every count in this section is a snapshot of `main` at `40a43b2d`, not a property of the history: it grows with each merge, so a re-measurement should read the then-current `main`.

Keywords count only against the default branch, so a non-default base is reported as closing nothing rather than failing.

## Driven-binary source records

`source_record.py` checks that a driven binary's embedded record covers what its own build read. The end-to-end proofs run workspace binaries, and each binary carries a record of the inputs its build compiled (`symbiote-source-stamp`); the proofs compare that record against the tree, so a proof cannot run against a binary built from other sources. What no proof checks is whether the record *covers every input the compiler read*: if a new way to compile a file appears and the walk does not follow it, the record is quietly short, every proof stays green, and only an audit finds it. That happened here — the walk did not follow `#[path = "…"]` module attributes, and `crates/symbiote-store/src/tests.rs` was compiling six files the record named only for packages that happened to be walked anyway.

The oracle is cargo's own per-unit dep-info, written by rustc for each unit that produces the binary. The record's implementation never reads it — its module documentation says why: it is a private, versioned binary format written *after* the build script that must write the record, so it can neither populate a record on a first build nor be an input the documented rebuild could clear. A check that runs after the build has neither problem. Cargo's *merged* dep-info for a binary is **not** usable for this: it also carries the build script's `rerun-if-changed` declarations, so it echoes the record back and agrees with it by construction. A build-script unit's generated files under `target/` and registry sources outside the workspace are excluded, as the record documents, and so are the test, example and bench directories, which the record excludes on purpose and whose own units read them.

A configuration file that exists and is not named, or a named one whose content moved or that has gone, and a record that names no `env:CARGO`, are failures too — the build configuration is what the record cannot watch without making cargo relink the stamped crates on every build.

```sh
cargo build --locked -p symbiote-host --bin symbioted -p symbiote-sandbox --bin symbiote-sandbox-launch
python3 planning/integrity/source_record.py \
    --binary symbiote-host:symbioted \
    --binary symbiote-sandbox:symbiote-sandbox-launch
python3 planning/integrity/test_source_record.py
```

It reads and writes nothing; exit status is 0 when every input the compiler read is named. The `source-records` job in `Rust contracts` runs it on every pull request after building those two binaries, so a newly unrecorded input class fails the build instead of waiting for an audit; its own tests run with the rest of this directory's in `Roadmap integrity`.

This implements a bounded portion of #470. It does **not** certify conversation-to-issue coverage, acceptance completion, or exact-body mutation safety. `acceptance_items` is only a checkbox inventory. Safe importer regeneration, three-way merge, stale-revision refusal, concurrent-edit/ambiguous-create reconciliation, and post-mutation readback remain separate work. The original bootstrap importer must not be used to overwrite this registry's newer issue authority.
