# Takeover and first Change Stream

Recorded 2026-09-08 UTC. Canonical owners: #170 (constitution), #470 (roadmap integrity); #173/#38 own remaining governance/proof. Branch `issue-170-470-takeover-integrity`, worktree `/mnt/data/projects/Symbiote` (also `/home/birdman/Projects/Symbiote`). Base and inspected target: `772fe8446f4ba42d9ad037de76acc3d792fc1d07`. The containing PR's head is the exact implementation revision; this record does not certify a later head.

## Verified starting state

The supplied directory was empty. Cloned the authenticated GitHub repository without deleting existing content. Default `main` contained only planning/import scripts, encoded historical payloads and issue templates: no application, Cargo/npm manifest, tests, accepted ADRs or AGENTS.md. No open PRs or other worktrees were present in this checkout. Remote audit branch `planning/audit-2026-09-07` is preserved and not merged as executable bootstrap. No branch protection/rulesets were configured when inspected; the user's independent-review/current-head gates still apply.

Rust 1.97.1, Cargo 1.97.1, Node 22.22.1 and Python 3.14.7 are available locally. Previous CI certified planning imports/audit only; the latest historical verification workflow had failed. No application/platform baseline exists to certify. Python remains existing repository-maintenance tooling, not a second core service implementation language.

## Delivered scope

- [Constitution](architecture/product-constitution.md) and [ADR-0001](architecture/adr-0001-technology-direction.md) record the accepted Rust/strict-TypeScript/no-Electron direction. Tauri 2 and SurrealDB remain candidates awaiting evidence. #170 remains open for executable schema/conformance and later integration acceptance.
- Amended #36, #38, #54, #154, #165, #170, #180, #191, #233, #336, #353, #375, #431, #446, #464, #465 and #467. Exact read-back verified each body, and full before/after inventories confirmed unchanged titles, states, labels, assignees and milestones. Removed the positive Electron candidate wording from #38 and #336. Baseline v2.4 identity/revision markers remain; a separate dated architecture-amendment marker records precedence.
- Historical Python/Node importer entrypoints now refuse before loading payloads or credentials. Historical workflow jobs are disabled and issue-write permissions removed. Payload/history is retained. These protections take effect on the default branch only after reviewed integration; old git revisions and the audit branch remain historical executable code and must not be run as synchronizers.
- The offline #470 validator generates a compact canonical registry and topological index from current issue evidence. It validates structure, not implementation completion. A safe remote regeneration writer is deliberately absent.

## Evidence and limitations

Captured before/after inventories with `python planning/capture_roadmap.py OUTPUT.json`: 466 issues. Captures are local evidence in `/tmp/symbiote-takeover-before.json` and `/tmp/symbiote-takeover-after.json`; recapture for future work, never treat temporary paths as durable authority. Amendment hash/read-back results: `/tmp/symbiote-amendment-results.json`.

The bounded manual issue amendment performed a batch preflight, immediate per-issue body/timestamp check, body-only PATCH and exact metadata/body read-back. GitHub atomic compare-and-swap was not assumed; this is not certification of the concurrent-edit/three-way-merge writer required by #470. No ambiguous mutation was retried. No issue was closed or marked implemented.

Local regression commands:

```sh
python -m unittest discover -s planning -p 'test_legacy_importers.py' -v
python -m unittest discover -s planning/integrity -p 'test_*.py' -v
git diff --check
```

Results: 23 offline integrity tests pass, including the persistent reduced audit fixture; the legacy test passes for all four entrypoints. The full after-capture validates 241 tasks, 19 epics, 198 references, one historical program entry and 702 prerequisite edges. A separate review agent found a fail-open dependency parser; missing/empty/unsupported declarations now refuse and details sections no longer hide canonical edges. The reviewer verified the fixes, all 23 tests, exact generated registry equality with the full capture and exact index rendering, with no remaining bounded-slice findings. This is separate-agent review, not a GitHub approval or platform certification. PR CI and merge remain separate gates.

No Rust runtime, shell spike, storage benchmark, cross-platform test, preview security property, provider entitlement, merge, deployment or release is claimed. #470 still requires safe three-way regeneration/dry-run, stale authority handling, ambiguous create/lost-response recovery, retained-scope coverage and mutation read-back integration. The offline validator and unconditional legacy refusal cover only the first protective slice.

## Dependency-correct next work

Use the generated index and retrieve full issue acceptance/comments just in time. Topological order alone is not readiness; open prerequisites need explicit verified foundational evidence, not checked boxes. #170's contract is documented but executable schema/conformance is pending. Establish that foundational evidence, then #173's versioned ADR/spike governance and #36 domain/schema contracts. #38 remains gated by #170/#173 and must precede dependent Host/desktop implementation. Advance #470's remaining mutation-safety contract after confirming its #170 prerequisite; independent read-only safeguards do not lock an unproven product architecture. #171 is the other dependency-root investigation owner and can advance independently with bounded current-source research.

Before integration, inspect the separate review findings and PR current-head CI, refresh `origin/main`, and revalidate if the target changes. Leave this Change Stream as a review-ready PR unless all required gates and authority are satisfied. Continue through the same canonical owners rather than creating a parallel backlog.
