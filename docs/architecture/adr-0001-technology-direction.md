# ADR-0001: Rust Host, strict TypeScript workbench, and no Electron

Decision status: accepted client constraints; shell/storage selections provisional.

Authority: the client's execution takeover instruction received 2026-09-07 US/Eastern. Recorded 2026-09-08 UTC. Canonical owners: #170 constitution, #173 decision governance, #38 architecture proof. This record interprets older technology wording through the latest explicit instruction; it does not claim #173's governance implementation or #38's proof complete.

## Evidence and precedence

Live #170, #173, #38 and #464 bodies and complete comment lists were retrieved through authenticated `gh issue view` on 2026-09-08 UTC; all four had no comments. Observed revisions were v2.4; updated timestamps were #170 2026-09-07T19:22:11Z, #173 2026-09-07T19:22:14Z, #38 2026-09-07T19:21:39Z and #464 2026-09-07T19:25:46Z. Timestamp values are snapshot metadata, not a license to overwrite later edits. Any GitHub amendment must compare bodies immediately before updating.

The live #38 baseline still listed Electron/Chromium among candidates. The new client instruction excludes Electron. This is an intentional correction, not an empirical framework benchmark. No compatibility versions, provider entitlements or cross-platform behavior have been certified by this ADR. Its product contract is [the constitution](product-constitution.md).

## Decision and alternatives

Use Rust for the Host/control plane, native runtime, CLI, supervision and first-party intelligence coordination. Use strict TypeScript for the workbench, with React and compiled static assets preferred. Do not add Go as a second core language. Permit bounded language-specific SDK/analyzer sidecars when justified and protocol-based extensions in other languages.

Electron is rejected by explicit client constraint. Tauri 2 is the preferred candidate pending #38. A non-Electron alternative remains eligible if measured Tauri failures violate a hard requirement. Removing required Preview features to retain Tauri is not an alternative. SurrealDB remains preferred for System Graph storage, pending proof of its actual version, behavior, deployment and resource profile; control-plane transactions require separate persistence evaluation.

Desktop remains a Host client and does not own canonical state or headless-work lifetime. CLI and desktop use the same native agent implementation. Isolate processes when trust, recovery, cancellation or resource limits require it; neither a giant process nor a service per concept follows from Rust reuse. Use versioned extension protocols, evaluating sandboxed WebAssembly only for appropriate bounded workloads. Do not default to trusted-daemon native plugin loading.

## Proof contract and stop conditions

Before shell selection, #38 must exercise at least four concurrent agent streams; three active bounded-scrollback terminal tabs; editor and diffs; live integrated application Preview; floating chat/voice controls over Preview; Project switch/restoration; cancellation and process-tree cleanup; crash recovery and client reconnect. Preserve the existing #38 scope for file watching, menus, updates/signing, packaging, simultaneous clients, disconnect/replay, headless execution and controller handoff.

Measure WebView composition, focus, stacking, clipping, resize, keyboard/input methods, scaling, accessibility, DOM/source mapping and security boundaries. Cover Linux Wayland and X11, Windows and macOS with explicit applicability. Record platform/version, hardware/workload, method, raw artifacts, predeclared thresholds, result, unknowns, cleanup and reproducibility. An untested platform remains pending. Stop dependent shell implementation on an unresolved hard requirement; failed evidence triggers a recorded non-Electron alternative evaluation.

Threat proof must treat every Preview origin, including localhost, as untrusted. Demonstrate no inherited Host/workbench authority, authenticated narrow design bridge operations, validation and isolated permissions. An iframe alone is insufficient evidence. Preview source edits must honor shared tokens, localization and data origins; DOM-only changes cannot pass implementation acceptance.

Benchmark whole process trees with attribution for shell/renderers, Preview, language servers, database, analyzers, SDK bridges and agents; include cold start, idle/active memory, CPU, installer size, build time and failure modes. Verify bounded caches/queues/scrollback, backpressure, lazy loading, virtualization, cancellation and Project-service suspension. Blank-window results cannot satisfy representative-workload gates.

## Consequences, reversibility and invalidation

These constraints remove Electron and a second Go core from downstream design freedom. Tauri/SurrealDB proofs may invalidate candidates without changing required product behavior. Keep clients and storage behind explicit protocols/contracts to limit migration cost; do not promise zero reversal cost. No state migration is performed by these documents. Later stateful implementations must specify migration/rollback before their own acceptance.

Accepted decision content is immutable after acceptance into reviewed history: changes create a superseding ADR and explicit lineage, rather than silently editing the settled decision. Draft corrections before merge must preserve review history. Changed compatibility facts invalidate affected proofs and dependent artifacts selectively. #173 owns machine-readable invalidation, graph queryability, immutable lineage, compatibility dossiers and fixtures; this Markdown map does not implement those behaviors.

## Canonical impact map

Owners below were checked against the live `planning:canonical` issue inventory on 2026-09-08 UTC. These are impact/integration references, not new hard dependencies or duplicate assignments.

| Boundary or changed requirement | Existing canonical owners | Required propagation/evidence |
| --- | --- | --- |
| Constitution, schemas, ADRs and architecture proof | #170, #36, #173, #38 | Accepted constraints, identity/schema contracts, contradiction audit; shell/storage remain unproven |
| Host, shared RPC/client and administrative CLI | #180, #181, #182, #54 | Rust ownership, headless lifetime, reconnect and common protocol semantics |
| Native/external runtime, provider and standalone CLI | #464, #465, #471, #447, #467, #469 | Shared native implementation, runtime/provider/auth/billing separation, real mixed-runtime and missing-CLI conformance |
| Workforce enforcement and dispatch | #184, #203, #205, #222 | Versioned contracts, truthful control strength, Host authorization and canonical completion |
| Desktop and dedicated Project context | #336, #337, #177, #178, #179 | No Electron; proof-gated shell, strict TypeScript/React preference, restoration and independent Project state |
| Preview, source edits and diagnostics | #353, #388, #390, #354 | Isolated untrusted design surface, narrow bridge, composition/source mapping and real source edit evidence |
| Security and recovery | #191, #218, #372 | Trust boundaries, permission/process isolation, cancellation/crash/reconnect proof |
| Graph and persistence | #232, #233, #220, #239 | Version/deployment proof, provenance/uncertainty, separately evaluated control persistence and service suspension |
| Context and Deep Guidance | #241, #242, #244, #245, #258, #260, #261 | Minimal evidence, native stateful methodology and early delivery/rollback planning |
| Living Documentation | #29, #30, #31, #32, #33, #34 | Shared knowledge projections, uncertainty, freshness, documentation closure and audience safety |
| Workbench surfaces | #346, #347, #349, #392, #457 | Preserve task/file/editor/Graph/Intelligence behavior; bounded resource consumption |
| Goal/delegation, learning and measurement | #460, #461, #462, #449, #451, #453, #454, #457 | Aggregate permissions/budgets, opt-in governance and comparable full-cost outcomes |
| Review, integration and completion | #387, #394, #424, #383, #335 | Independent review, current-target verification and Capability/Impact Closure |
| Platform and operational validation | #417, #432, #433, #374 | Linux/Windows/macOS applicable evidence, process-tree observability and opt-in telemetry separation |
| Roadmap authority and importer | #470 | Preserve stable identities, revision checks and human amendments; never replay unsafe bootstrap as authority |

## Remaining acceptance

No runtime, shell, database, enforcement mechanism or benchmark is implemented or certified here. #170's prerequisite schema/conformance and review evidence remains pending; #173's machine-readable governance and #38's representative proofs remain pending. This decision may guide independent contract drafting, but does not satisfy the hard prerequisites for dependent implementation. GitHub reconciliation, CI and independent review must be recorded by the integrating Change Stream before claiming those gates.
