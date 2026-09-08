# Engineering handoff

## Runtime SDK foundation — 2026-09-08 UTC

Change Stream `issue-184-runtime-sdk` starts from merged #475 at `c1b7f2f6d7adcbe7c50377d1ff9e990b11019ebe`. Owners #184/#464 consume the reviewed #181/#36 contracts. `symbiote-runtime-sdk` separates agent-loop adapters from inference providers, qualifies immutable dispatches against current identity-bound capability/control evidence, and defines bounded session events and native/external auth/billing checks.

Independent review covers adapter capability/activation logic separately from provider/event logic. New failure fixtures cover proof substitution, stale controls, context bounds, native/external ownership, absent CLI/local endpoint metadata, false completion, replay gaps, cancellation uncertainty and impossible/over-budget usage. Conformance fixtures are not real runtimes; the Host has no SDK activation endpoint or implementation of its ActivationJournal callback yet.

Next: implement dependency-ready runtime profile/configuration and trust/enforcement foundations, then actual adapter transport and dispatch integration under the canonical queue. #184/#464 remain open for real reference transports, compatibility dossiers, process isolation and full native/Codex operation. See `docs/contracts/runtime-sdk.md`, `runtime-events.md` and `providers.md`. Overall first usable release remains incomplete; do not mark the persistent build goal achieved.

## Durable metadata Host — 2026-09-08 UTC

Current Change Stream: `issue-43-durable-host`, isolated from merged #474 at `42eda51a7e4955cb5f44aa85833e8a313a6ae3a7`. Owners #43/#181/#180 consume the reviewed #36/#176 foundational contracts. New Rust workspace crates implement SQLite current-state/journal transactions, typed transport-neutral requests, and an actual Linux daemon/CLI using private same-UID IPC. The GUI has no canonical state ownership. No agent subprocess or model billing is enabled.

The daemon can register a Project with its Roots/Roles, create ready Tasks/initial Change Streams, read records and replay committed events. Real-process tests force daemon death and transaction-boundary death, reconnect independent clients, replay dropped replies and check concurrency. Separate review covers storage/protocol and Host transport; current-head CI belongs to the containing PR. See `docs/contracts/{storage,protocol,host}.md` for the exact implemented boundaries and reproduction commands.

Next prerequisite work includes execution permissions/credentials and durable side-effect disposition, runtime/workforce contracts, actual process/worktree supervision and the remaining #38 architecture proofs. Native/Codex execution, desktop integration, backup/export recovery and full protocol/service packaging remain pending. Do not close broad #43/#181/#180 on metadata storage alone or advertise the shell/security/runtime as release-ready.

## Architecture proof batch — 2026-09-08 UTC

Foundations PR #473 merged at `a15368c1744fdff3c186dda11f24714d738c7efe` after separate review and passing stable/MSRV/roadmap checks. The #36/#170/#176/#179/#173 issues remain open for their broader acceptance. Current proof work is isolated on `issue-38-linux-proof` from that revision.

Two runnable experiments now exist: `spikes/linux-shell` (Tauri/React/Monaco, three native PTYs, separate Preview and overlapping Lead WebViews) and `spikes/host-lifecycle` (independent Rust daemon, fixed process tree, disconnect/replay/cancel/restart fixtures). See `docs/proofs/linux-shell.md` and `docs/proofs/host-lifecycle.md` for reproduction, measurements, raw evidence and limitations. The initial GTK composition failure is retained alongside corrected interaction screenshots. Preview denial reports are untrusted observations, not authenticated security proof.

These experiments do not select the desktop shell or transactional storage. Native Wayland/real X11, Windows/macOS, accessibility/IME/scaling, arbitrary descendant containment, representative resource budgets and full Preview authority isolation remain unpassed. The injected guardian-loss test exposes a recovery boundary instead of claiming unconditional orphan cleanup. No native API/Codex integration, production Host or usable release is complete. Continue the live #38 acceptance and separate #43 storage decision before dependent production integration; preserve the canonical queue and open broad issues.

## Executable foundation batch — 2026-09-08 UTC

PR #472 was reviewed and merged as `d2c5d300652e3733ec0172c4b0ef0690fa168e37`. Current worktree `/mnt/data/projects/Symbiote-worktrees/issue-36-176-foundations`, branch `issue-36-176-foundations`, starts from that merged revision. Owners are #36/#170 domain/constitutional contracts, #176/#179 configuration/portable manifests, and #173 architecture decision/proof contracts. The first usable release must prove both native Rust/OpenAI API and external Codex execution under shared canonical controls; none of this batch calls paid inference or claims those integrations.

Three pure Rust crates replace the absence of application contracts: typed identities/dispatch/lifecycle evidence; scoped configuration/portable manifest and conflict previews; immutable accepted architecture decisions/version pins and selective invalidation. See `docs/contracts/` for acceptance coverage and remaining integration obligations. `README.md` documents reproducible build/test/schema commands. A Cargo lockfile pins dependencies; CI adds stable and Rust 1.85 contract verification. No unsafe code is permitted in these crates.

The baseline planning suites passed before changes. Independent review identified forged deserialization/dispatch-context/freshness defects in the domain boundary, invalid portable paths, and incomplete affected-artifact reporting in decision acceptance. All were fixed with regression tests and independently re-reviewed. Final local checks: 42 Rust tests pass (18 domain, 12 configuration, 12 architecture), strict all-target Clippy passes, formatting/diff checks pass and all three schema generators run. The domain reviewer also verified six independent adversarial probes including history tampering. CI/MSRV and merge evidence belong to the containing PR; no GitHub approval is inferred from separate-agent review.

The #38 environment probe found GTK3/WebKitGTK4.1, Tauri CLI and Xvfb. Native display is KDE Wayland; its `:1` X11 connection is XWayland, not standalone X11 certification. The Linux shell spike is prepared separately under `issue-38-linux-proof`; it must not be represented as a chosen desktop architecture. Windows/macOS, minimum-target resource budgets, Preview authority isolation, signed updates and full representative workload remain proof gates. Prototype preparation is independent; execution follows the reviewed #173 foundational contract. Graph storage and transactional control-plane storage remain separate candidates.

Next: integrate reviewed/current-head foundation checks, execute the bounded #38 Linux spike and publish raw evidence/failures, then advance #43/#181/#180 only after their explicit schema/persistence/proof requirements are supported. Keep broad issues open for later Host authentication, durable recovery, runtime, graph, full ontology and cross-platform acceptance. Never mark code-level enforcement authenticated solely because a test supplies an `Actor::Host` or an evidence record.

## Initial takeover record

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
